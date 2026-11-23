//! Atomic relocation of an entire drawing tween, including its keyed drawings.
use crate::{Editor, shared::SharedVec};

#[derive(Clone, Copy)]
pub(crate) struct Drag {
    pub layer: usize,
    pub start: usize,
    pub end: usize,
    pub target: usize,
}
impl Editor {
    pub(crate) fn tween_drop_valid(&self, drag: Drag) -> bool {
        let Some(layer) = self.doc.layers.get(drag.layer) else {
            return false;
        };
        let duration = drag.end.saturating_sub(drag.start);
        if layer.options.locked
            || duration == 0
            || drag.target == drag.start
            || drag.target.saturating_add(duration) >= 10000
            || drag.end >= layer.frames.len()
            || !layer
                .options
                .tweens
                .iter()
                .any(|s| s.start == drag.start && s.end == drag.end)
        {
            return false;
        }
        let destination = drag.target..=drag.target + duration;
        let source = drag.start..=drag.end;
        !layer.options.tweens.iter().any(|s| {
            (s.start != drag.start || s.end != drag.end)
                && ((s.start < drag.end && s.end > drag.start)
                    || !(s.end < drag.target || s.start > drag.target + duration))
        }) && !(0..layer.frames.len()).any(|f| {
            destination.contains(&f)
                && !source.contains(&f)
                && layer.key(f)
                && !layer.frames[f].is_empty()
        })
    }
    pub(crate) fn move_entire_tween(&mut self, drag: Drag) -> bool {
        if !self.tween_drop_valid(drag) {
            return false;
        }
        self.finish_pending_pen();
        self.checkpoint();
        let layer = &mut self.doc.layers[drag.layer];
        layer.init_keys();
        let source = drag.start..=drag.end;
        let shared: std::collections::BTreeSet<_> = layer
            .options
            .tweens
            .iter()
            .filter(|s| s.start != drag.start || s.end != drag.end)
            .flat_map(|s| [s.start, s.end])
            .filter(|f| source.contains(f))
            .collect();
        let delta = drag.target as isize - drag.start as isize;
        let remap = |frame: usize| {
            if source.contains(&frame) {
                frame.checked_add_signed(delta).unwrap()
            } else {
                frame
            }
        };
        let mut keys: std::collections::BTreeMap<usize, SharedVec<crate::Mark>> =
            (0..layer.frames.len())
                .filter(|&f| layer.key(f) && (!source.contains(&f) || shared.contains(&f)))
                .map(|f| (f, layer.frames[f].clone()))
                .collect();
        // Empty destination keys may be replaced, but occupied drawings are never overwritten.
        for f in (drag.start..=drag.end).filter(|&f| layer.key(f)) {
            keys.insert(remap(f), layer.frames[f].clone());
        }
        keys.entry(0).or_default();
        let count = layer
            .frames
            .len()
            .max(drag.target + drag.end - drag.start + 1);
        let mut held = SharedVec::default();
        layer.frames = (0..count)
            .map(|f| {
                if let Some(cel) = keys.get(&f) {
                    held = cel.clone();
                }
                held.clone()
            })
            .collect::<Vec<_>>()
            .into();
        layer.options.keys = (0..count)
            .map(|f| keys.contains_key(&f))
            .collect::<Vec<_>>()
            .into();
        for span in &mut layer.options.tweens {
            if span.start == drag.start && span.end == drag.end {
                span.start = remap(span.start);
                span.end = remap(span.end);
            }
        }
        layer.options.tweens.sort_by_key(|s| s.start);
        let mut classes = std::collections::BTreeMap::new();
        for (f, class) in std::mem::take(&mut layer.options.drawing_classes) {
            if shared.contains(&f) {
                classes.insert(f, class);
            }
            classes.insert(remap(f), class);
        }
        layer.options.drawing_classes = classes;
        for other in &mut self.doc.layers {
            if other.frames.len() < count {
                if crate::animation_clips::resize_track(other, count) {
                    continue;
                }
                other.init_keys();
                let held = other.frames.last().cloned().unwrap_or_default();
                other.frames.resize(count, held);
                other.options.keys.resize(count, false);
            }
        }
        self.layer = drag.layer;
        self.frame = drag.target;
        self.feature.frame_selection =
            Some((drag.layer, drag.target, drag.target + drag.end - drag.start));
        self.playing = false;
        self.fit_camera_duration();
        self.recover();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_adjacent_tween_retains_the_shared_boundary_drawing() {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            eframe::egui::Context::default(),
        ));
        e.doc = crate::fresh_document();
        let layer = &mut e.doc.layers[0];
        for f in 0..layer.frames.len() {
            layer.frames[f] = vec![crate::Mark {
                points: vec![[f as f32, 0.], [f as f32 + 10., 10.]].into(),
                ..Default::default()
            }]
            .into();
        }
        layer.options.keys = (0..layer.frames.len())
            .map(|f| [0, 5, 10].contains(&f))
            .collect::<Vec<_>>()
            .into();
        layer.options.tweens = [(0, 5), (5, 10)]
            .into_iter()
            .map(|(start, end)| crate::animation::Tween {
                start,
                end,
                shape: false,
                curve: Default::default(),
            })
            .collect::<Vec<_>>()
            .into();
        let boundary = layer.frames[5].clone();
        assert!(e.move_entire_tween(Drag {
            layer: 0,
            start: 5,
            end: 10,
            target: 15
        }));
        assert!(e.doc.layers[0].frames[5] == boundary);
        assert!(e.doc.layers[0].frames[15] == boundary);
        assert_eq!(
            (
                e.doc.layers[0].options.tweens[0].start,
                e.doc.layers[0].options.tweens[0].end
            ),
            (0, 5)
        );
        e.doc.validate().unwrap();
    }
    #[test]
    fn entire_motion_and_shape_tweens_move_from_frame_zero_and_undo_once() {
        for shape in [false, true] {
            let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
                eframe::egui::Context::default(),
            ));
            e.doc = crate::fresh_document();
            let layer = &mut e.doc.layers[0];
            let a = crate::Mark {
                points: vec![[10., 10.], [30., 30.]].into(),
                rectangle: true,
                closed: true,
                fill: Some([255, 0, 0, 255]),
                ..Default::default()
            };
            let b = crate::vector::transform(&a, |p| p + eframe::egui::Vec2::X * 100.);
            for frame in 0..layer.frames.len() {
                layer.frames[frame] = vec![if frame < 5 { a.clone() } else { b.clone() }].into();
            }
            layer.options.keys = (0..layer.frames.len())
                .map(|f| f == 0 || f == 5)
                .collect::<Vec<_>>()
                .into();
            layer.options.tweens = vec![crate::animation::Tween {
                start: 0,
                end: 5,
                shape,
                curve: Default::default(),
            }]
            .into();
            let before = serde_json::to_value(&e.doc).unwrap();
            assert!(e.move_entire_tween(Drag {
                layer: 0,
                start: 0,
                end: 5,
                target: 10
            }));
            assert!(e.doc.layers[0].frames[0].is_empty());
            assert_eq!(
                (
                    e.doc.layers[0].options.tweens[0].start,
                    e.doc.layers[0].options.tweens[0].end
                ),
                (10, 15)
            );
            assert!(e.doc.layers[0].key(10) && e.doc.layers[0].key(15));
            e.doc.validate().unwrap();
            assert_eq!(e.undo.len(), 1);
            e.history(false);
            assert_eq!(serde_json::to_value(&e.doc).unwrap(), before);
        }
    }
}
