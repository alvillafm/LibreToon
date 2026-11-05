//! Click-to-trim at vector crossings. Curves and width profiles stay editable.
use crate::{
    Editor, Mark,
    canvas::{Tool, View},
    features::id,
    shared::SharedVec,
};
use eframe::egui::{self, Pos2, Rect};
struct Parts {
    layer: usize,
    index: usize,
    original: Mark,
    pieces: Vec<Mark>,
}
#[derive(Default)]
pub struct State {
    key: Option<(
        [usize; 8],
        SharedVec<crate::document::Layer>,
        SharedVec<crate::features::Folder>,
        SharedVec<crate::features::Symbol>,
        crate::rig::Rig,
        SharedVec<crate::features::Camera>,
    )>,
    parts: Option<Parts>,
    hover: Option<usize>,
}
impl State {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
fn distance(mark: &Mark, p: Pos2) -> f32 {
    crate::vector::flatten(mark, 0.15)
        .windows(2)
        .map(|w| {
            let ab = w[1] - w[0];
            let t = ((p - w[0]).dot(ab) / ab.length_sq().max(1e-12)).clamp(0., 1.);
            p.distance(w[0] + ab * t)
        })
        .min_by(f32::total_cmp)
        .unwrap_or(f32::INFINITY)
}
fn key(e: &Editor) -> [usize; 8] {
    [
        e.doc.layers.identity(),
        e.doc.extras.folders.identity(),
        e.doc.extras.symbols.identity(),
        e.doc.extras.rig.bones.identity(),
        e.doc.extras.rig.bindings.identity(),
        e.doc.extras.cameras.identity(),
        e.frame,
        usize::from(crate::depth::active(e)),
    ]
}
impl Editor {
    fn trim_candidate(&mut self, pointer: Pos2, base: View) {
        self.vector_trim.hover = None;
        let k = key(self);
        if self
            .vector_trim
            .key
            .as_ref()
            .is_none_or(|(old, ..)| *old != k)
        {
            self.vector_trim.parts = None;
            self.vector_trim.key = Some((
                k,
                self.doc.layers.clone(),
                self.doc.extras.folders.clone(),
                self.doc.extras.symbols.clone(),
                self.doc.extras.rig.clone(),
                self.doc.extras.cameras.clone(),
            ));
        }
        let found = self
            .doc
            .layers
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| {
                l.visible
                    && !l.options.locked
                    && !l.options.animation_track
                    && crate::folder_groups::layer_active(&self.doc, l, self.frame)
            })
            .find_map(|(layer, data)| {
                let view = self.camera_depth_view(base, layer);
                let p = view.world(pointer);
                crate::animation::marks(data, self.frame)
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, m)| {
                        m.stroke
                            && m.style.instance.is_none()
                            && m.style.text.is_none()
                            && !m.style.guide
                            && crate::canvas::stroke_hit(m, p, 5. / view.scale.abs().max(0.001))
                    })
                    .map(|(index, m)| (layer, index, m.clone(), p))
            });
        let Some((layer, index, original, p)) = found else {
            return;
        };
        if self
            .vector_trim
            .parts
            .as_ref()
            .is_none_or(|c| c.layer != layer || c.index != index)
        {
            let others: Vec<_> = crate::features::resolve(&self.doc, self.frame, false)
                .into_iter()
                .filter(|(l, m, mark)| {
                    (*l != layer || *m != index) && mark.stroke && !mark.style.guide
                })
                .map(|(l, _, mark)| self.depth_mark(&mark, l, layer))
                .collect();
            let pieces = crate::editing::crossing_parts(&original, &others);
            self.vector_trim.parts = Some(Parts {
                layer,
                index,
                original,
                pieces,
            });
        }
        if let Some(parts) = &self.vector_trim.parts {
            self.vector_trim.hover = parts
                .pieces
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| distance(a, p).total_cmp(&distance(b, p)))
                .map(|(i, _)| i);
        }
    }
    fn commit_trim(&mut self) -> bool {
        let Some(chosen) = self.vector_trim.hover else {
            return false;
        };
        let Some(parts) = self.vector_trim.parts.take() else {
            return false;
        };
        if parts.layer >= self.doc.layers.len() || self.doc.layers[parts.layer].options.locked {
            return false;
        }
        self.finish_pending_pen();
        self.checkpoint();
        self.materialize_motion(&[(parts.layer, parts.index)]);
        let mut replacements = vec![];
        if parts.original.fill.is_some() {
            let mut fill = parts.original.clone();
            fill.stroke = false;
            fill.style.track = id();
            replacements.push(fill);
        }
        for (index, mut piece) in parts.pieces.into_iter().enumerate() {
            if index != chosen {
                piece.fill = None;
                piece.style.track = id();
                replacements.push(piece);
            }
        }
        let frame = &mut self.doc.layers[parts.layer].frames[self.frame];
        if parts.index >= frame.len() {
            return false;
        }
        frame.splice(parts.index..=parts.index, replacements);
        self.layer = parts.layer;
        self.vector_trim.clear();
        self.recover();
        true
    }
    pub(crate) fn vector_trim_input(&mut self, ui: &egui::Ui, area: Rect, base: View) -> bool {
        if self.tool != Tool::Trim
            || self.playing
            || self.drawing.pen.eraser()
            || ui.input(|i| i.key_down(egui::Key::Space) || i.pointer.middle_down())
        {
            self.vector_trim.hover = None;
            return false;
        }
        if self.folder_groups.dialog.is_some()
            || self.document_settings.is_some()
            || self.view_options.draft.is_some()
        {
            return false;
        }
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else {
            return false;
        };
        if !area.contains(pointer) || !ui.rect_contains_pointer(area) {
            self.vector_trim.hover = None;
            return false;
        }
        self.trim_candidate(pointer, base);
        ui.ctx()
            .set_cursor_icon(if self.vector_trim.hover.is_some() {
                egui::CursorIcon::Crosshair
            } else {
                egui::CursorIcon::Default
            });
        if ui.input(|i| i.pointer.primary_pressed()) {
            self.commit_trim();
        }
        true
    }
    pub(crate) fn paint_vector_trim(&self, painter: &egui::Painter, base: View) {
        if self.tool != Tool::Trim {
            return;
        }
        if let Some((parts, index)) = self.vector_trim.parts.as_ref().zip(self.vector_trim.hover) {
            if let Some(piece) = parts.pieces.get(index) {
                let view = self.camera_depth_view(base, parts.layer);
                let mut preview = piece.clone();
                preview.color = [255, 85, 90, 255];
                preview.fill = None;
                preview.style.gradient = None;
                preview.width += 2. / view.scale.abs().max(0.001);
                crate::canvas::paint_mark(painter, &preview, view, false, false, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Editor {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        e.doc = crate::fresh_document();
        e.recovery = std::env::temp_dir().join(format!(
            "trim-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        let line = |a, b| Mark {
            points: vec![a, b].into(),
            width: 3.,
            ..Default::default()
        };
        e.doc.layers[0].frames[0] = vec![
            line([0., 50.], [100., 50.]),
            line([30., 0.], [30., 100.]),
            line([70., 0.], [70., 100.]),
        ]
        .into();
        e.tool = Tool::Trim;
        e
    }
    #[test]
    fn trimming_one_crossing_segment_keeps_the_other_lines_and_undo() {
        let mut e = fixture();
        let before = e.doc.clone();
        let base = View {
            center: Pos2::ZERO,
            stage_center: Pos2::ZERO,
            scale: 1.,
            angle: 0.,
        };
        e.trim_candidate(egui::pos2(50., 50.), base);
        assert_eq!(e.vector_trim.parts.as_ref().unwrap().pieces.len(), 3);
        assert!(e.commit_trim());
        let frame = &e.doc.layers[0].frames[0];
        assert_eq!(frame.len(), 4);
        assert!(
            !frame
                .iter()
                .any(|m| crate::canvas::stroke_hit(m, egui::pos2(50., 50.), 0.))
        );
        assert!(
            frame
                .iter()
                .any(|m| crate::canvas::stroke_hit(m, egui::pos2(10., 50.), 0.))
        );
        assert!(e.doc.validate().is_ok());
        e.history(false);
        assert_eq!(
            serde_json::to_vec(&e.doc).unwrap(),
            serde_json::to_vec(&before).unwrap()
        );
    }
    #[test]
    fn filled_shape_retains_fill_when_only_its_stroke_is_trimmed() {
        let mut e = fixture();
        e.doc.layers[0].frames[0][0].fill = Some([10, 20, 30, 255]);
        let base = View {
            center: Pos2::ZERO,
            stage_center: Pos2::ZERO,
            scale: 1.,
            angle: 0.,
        };
        e.trim_candidate(egui::pos2(50., 50.), base);
        e.commit_trim();
        assert!(
            e.doc.layers[0].frames[0]
                .iter()
                .any(|m| !m.stroke && m.fill == Some([10, 20, 30, 255]))
        );
    }
    #[test]
    fn curved_trim_retains_bezier_handles_and_variable_width() {
        let mut e = fixture();
        let curve = crate::vector::Cubic([
            egui::pos2(0., 50.),
            egui::pos2(25., 10.),
            egui::pos2(75., 90.),
            egui::pos2(100., 50.),
        ]);
        let style = Mark {
            width: 4.,
            width_profile: crate::width::Preset::Tapered.stops(),
            ..Default::default()
        };
        e.doc.layers[0].frames[0][0] = crate::vector::from_cubics(&[curve], &style);
        let base = View {
            center: Pos2::ZERO,
            stage_center: Pos2::ZERO,
            scale: 1.,
            angle: 0.,
        };
        e.trim_candidate(curve.at(0.5), base);
        assert_eq!(e.vector_trim.parts.as_ref().unwrap().pieces.len(), 3);
        assert!(e.commit_trim());
        let frame = &e.doc.layers[0].frames[0];
        assert_eq!(frame.len(), 4);
        for m in &frame[..2] {
            assert!(!m.nodes.is_empty());
            assert!(!m.width_profile.is_empty());
            let c = crate::vector::cubics(m)[0];
            assert!(
                (c.0[1].y - c.0[0].y).abs() > 0.1,
                "curved handles must survive"
            );
        }
    }
    #[test]
    fn locked_target_cannot_be_trimmed() {
        let mut e = fixture();
        e.doc.layers[0].options.locked = true;
        let base = View {
            center: Pos2::ZERO,
            stage_center: Pos2::ZERO,
            scale: 1.,
            angle: 0.,
        };
        e.trim_candidate(egui::pos2(50., 50.), base);
        assert!(e.vector_trim.hover.is_none());
        assert!(!e.commit_trim());
        assert_eq!(e.doc.layers[0].frames[0].len(), 3);
    }
}
