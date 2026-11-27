//! Non-destructive vector shape masks share the normal cel/tween editor.
use crate::{
    Editor, Mark,
    canvas::{Tool, View},
    document::Layer,
    features::LayerOptions,
};
use eframe::egui::{self, Color32, Pos2, Rect};
use geo::BooleanOps;

pub(crate) fn clip(mark: &Mark, area: &geo::MultiPolygon<f64>, inverted: bool) -> Vec<Mark> {
    if !inverted {
        return crate::features::clip_to(mark, area);
    }
    let bounds = crate::vector::control_bounds(mark).expand(mark.width.max(1.) + 2.);
    if !bounds.is_finite() {
        return Vec::new();
    }
    let frame = geo::MultiPolygon(vec![
        geo::Rect::new(
            (f64::from(bounds.left()), f64::from(bounds.top())),
            (f64::from(bounds.right()), f64::from(bounds.bottom())),
        )
        .to_polygon(),
    ]);
    crate::features::clip_to(mark, &frame.difference(area))
}

impl Editor {
    pub(crate) fn can_add_shape_mask(&self) -> bool {
        let layer = &self.doc.layers[self.layer];
        !self.playing
            && self.doc.layers.len() < 100
            && !layer.options.locked
            && layer.options.mask_id.is_none()
            && layer.options.masked_by.is_none()
            && !crate::post::is_adjustment(layer)
            && layer.options.light.is_none()
            && !layer.options.light_mask
            && layer.options.mannequin.is_none()
            && !layer.options.animation_track
    }

    pub(crate) fn add_shape_mask(&mut self, ellipse: bool) {
        if !self.can_add_shape_mask() {
            return;
        }
        self.finish_raster_stroke();
        self.finish_pending_pen();
        let target = self.layer;
        let layer = &self.doc.layers[target];
        let count = layer.frames.len();
        let mut bounds = crate::animation::marks(layer, self.frame)
            .iter()
            .fold(Rect::NOTHING, |r, m| {
                r.union(crate::vector::control_bounds(m))
            });
        if !bounds.is_finite() || bounds.width() < 1. || bounds.height() < 1. {
            bounds = Rect::from_min_max(
                Pos2::ZERO,
                Pos2::new(self.doc.width as f32, self.doc.height as f32),
            );
        }
        bounds = bounds.expand(2.);
        let style = Mark {
            fill: Some([255; 4]),
            stroke: false,
            closed: true,
            ..Default::default()
        };
        let mark = if ellipse {
            crate::editing::shape(Tool::Ellipse, bounds.min, bounds.max, 4, false, 0.5, &style)
        } else {
            crate::rectangle::create(bounds.min, bounds.max, [0.; 4], &style)
        };
        let id = crate::features::id();
        let mut keys = vec![false; count];
        keys[0] = true;
        let folder = layer.options.folder;
        let depth = layer.options.depth.clone();
        let name = format!("{} · {}", crate::i18n::tr("Shape mask"), layer.name);
        self.checkpoint();
        self.doc.layers[target].options.masked_by = Some(id);
        let slot = target + 1;
        for track in &mut self.doc.extras.media.tracks {
            if track.slot >= slot {
                track.slot += 1;
            }
        }
        for folder in &mut self.doc.extras.folders {
            if let Some(position) = &mut folder.timeline_slot
                && *position >= slot
            {
                *position += 1;
            }
        }
        let cel: crate::shared::SharedVec<_> = vec![mark].into();
        self.doc.layers.insert(
            slot,
            Layer {
                name,
                visible: true,
                frames: vec![cel; count].into(),
                options: LayerOptions {
                    mask_id: Some(id),
                    depth,
                    folder,
                    keys: keys.into(),
                    ..Default::default()
                },
            },
        );
        self.layer = slot;
        self.feature.frame_selection = None;
        self.feature.frame_drag = None;
        self.canvas.selected = vec![(slot, 0)];
        self.activate_tool(Tool::Transform);
        self.recover();
    }

    pub(crate) fn shape_mask_properties(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.doc.layers[self.layer]
            .options
            .mask_id
            .or(self.doc.layers[self.layer].options.masked_by)
        else {
            return;
        };
        let Some(source) = self
            .doc
            .layers
            .iter()
            .position(|l| l.options.mask_id == Some(id))
        else {
            return;
        };
        crate::design::category(ui, "Shape mask");
        let target = self
            .doc
            .layers
            .iter()
            .position(|l| l.options.masked_by == Some(id));
        let mut disabled = self.doc.layers[source].options.mask_disabled;
        let mut inverted = self.doc.layers[source].options.mask_inverted;
        let locked = self.doc.layers[source].options.locked || self.playing;
        let mut enabled = !disabled;
        let mut changed = false;
        ui.add_enabled_ui(!locked, |ui| {
            changed |= ui
                .checkbox(&mut enabled, crate::i18n::tr("Enable mask"))
                .changed();
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .selectable_value(&mut inverted, false, crate::i18n::tr("Reveal inside"))
                    .changed();
                changed |= ui
                    .selectable_value(&mut inverted, true, crate::i18n::tr("Hide inside"))
                    .changed();
            });
        });
        disabled = !enabled;
        if changed {
            self.checkpoint();
            self.doc.layers[source].options.mask_disabled = disabled;
            self.doc.layers[source].options.mask_inverted = inverted;
            self.recover();
        }
        let editing = source == self.layer;
        if !editing {
            if crate::design::button(ui, crate::design::Icon::Mask, "Edit mask", "", false, true)
                .clicked()
            {
                self.layer = source;
                self.canvas.clear_selection();
                self.activate_tool(Tool::Select);
            }
        } else {
            if let Some(target) = target
                && crate::design::button(
                    ui,
                    crate::design::Icon::Layers,
                    "Back to artwork",
                    "",
                    false,
                    true,
                )
                .clicked()
            {
                self.layer = target;
                self.canvas.clear_selection();
                self.activate_tool(Tool::Select);
            }
            ui.add_enabled_ui(!locked, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (tool, icon, label) in [
                        (Tool::Node, crate::design::Icon::Node, "Edit mask nodes"),
                        (
                            Tool::Rectangle,
                            crate::design::Icon::Rectangle,
                            "Add rectangle",
                        ),
                        (Tool::Ellipse, crate::design::Icon::Ellipse, "Add ellipse"),
                        (Tool::Pen, crate::design::Icon::Pen, "Add path"),
                    ] {
                        if crate::design::button(ui, icon, label, "", self.tool == tool, true)
                            .clicked()
                        {
                            self.layer = source;
                            self.fill_enabled = true;
                            self.stroke_enabled = false;
                            if self.fill_color.a() == 0 {
                                self.fill_color = Color32::WHITE;
                            }
                            self.canvas.selected = (0..self.doc.layers[source].frames[self.frame]
                                .len())
                                .map(|m| (source, m))
                                .collect();
                            self.activate_tool(tool);
                        }
                    }
                });
            });
            if crate::design::button(
                ui,
                crate::design::Icon::Pen,
                "Draw new mask",
                "Replace the current mask drawing with a new path",
                false,
                !locked,
            )
            .clicked()
            {
                self.checkpoint();
                self.doc.layers[source].frames[self.frame] = Default::default();
                self.fill_enabled = true;
                self.stroke_enabled = false;
                self.fill_color = Color32::WHITE;
                self.activate_tool(Tool::Pen);
                self.recover();
            }
            ui.small(crate::i18n::tr(
                "Animate the mask with keyframes and shape tweens on its timeline row.",
            ));
        }
        if crate::design::button(
            ui,
            crate::design::Icon::Trash,
            "Remove mask",
            "Keep the original artwork",
            false,
            !locked,
        )
        .clicked()
        {
            self.timeline_layer_action(source, 1);
            if let Some(target) = target {
                self.layer = target - usize::from(source < target);
            }
        }
    }

    pub(crate) fn shape_mask_overlay(&self, painter: &egui::Painter, view: View) {
        let layer = &self.doc.layers[self.layer];
        if self.playing || layer.options.mask_id.is_none() {
            return;
        }
        let color = Color32::from_rgb(42, 202, 168);
        for mark in crate::animation::marks(layer, self.frame).iter() {
            let points = crate::vector::flatten(mark, 0.25 / view.scale.max(0.01));
            painter.add(egui::Shape::Path(egui::epaint::PathShape {
                points: points.into_iter().map(|p| view.screen(p)).collect(),
                closed: mark.closed || mark.rectangle,
                fill: Color32::TRANSPARENT,
                stroke: egui::Stroke::new(1.5, color).into(),
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn rect(a: [f32; 2], b: [f32; 2]) -> Mark {
        Mark {
            points: vec![a, b].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([255, 0, 0, 255]),
            ..Default::default()
        }
    }
    pub(super) fn visible(doc: &crate::Document, frame: usize, p: Pos2) -> bool {
        crate::features::resolve_export(doc, frame, false)
            .iter()
            .any(|(_, _, m)| crate::vector::contains_fill(m, p))
    }
    pub(super) fn editor() -> Editor {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        e.doc = crate::Document::default();
        e.recovery = std::env::temp_dir().join(format!(
            "shape-mask-{}-{}.json",
            std::process::id(),
            crate::features::id()
        ));
        let cel: crate::shared::SharedVec<_> = vec![rect([10., 10.], [210., 210.])].into();
        e.doc.layers[0].frames = vec![cel; 24].into();
        e.frame = 0;
        e.layer = 0;
        e
    }
    #[test]
    fn masks_reveal_hide_disable_and_preserve_source_with_one_undo() {
        let mut e = editor();
        let original = e.doc.layers[0].frames[0].clone();
        e.add_shape_mask(false);
        assert_eq!(e.doc.layers.len(), 2);
        assert_eq!(e.layer, 1);
        let cel: crate::shared::SharedVec<_> = vec![rect([10., 10.], [110., 210.])].into();
        e.doc.layers[1].frames = vec![cel; 24].into();
        assert!(visible(&e.doc, 0, Pos2::new(50., 50.)));
        assert!(!visible(&e.doc, 0, Pos2::new(150., 50.)));
        e.doc.layers[1].options.mask_inverted = true;
        assert!(!visible(&e.doc, 0, Pos2::new(50., 50.)));
        assert!(visible(&e.doc, 0, Pos2::new(150., 50.)));
        e.doc.layers[1].options.mask_disabled = true;
        assert!(visible(&e.doc, 0, Pos2::new(50., 50.)));
        assert!(visible(&e.doc, 0, Pos2::new(150., 50.)));
        assert_eq!(e.doc.layers[0].frames[0].identity(), original.identity());
        e.history(false);
        assert_eq!(e.doc.layers.len(), 1);
        assert!(e.doc.layers[0].options.masked_by.is_none());
        e.history(true);
        assert_eq!(e.doc.layers.len(), 2);
        e.timeline_layer_action(1, 1);
        assert!(e.doc.layers[0].options.masked_by.is_none());
    }
    #[test]
    fn mask_keys_move_independently_and_native_metadata_roundtrips() {
        let mut e = editor();
        e.add_shape_mask(true);
        e.doc.layers[1].frames[0] = vec![rect([10., 10.], [100., 210.])].into();
        e.doc.layers[1].frames[1] = vec![rect([120., 10.], [210., 210.])].into();
        e.doc.layers[1].options.keys[1] = true;
        let saved = serde_json::to_vec(&e.doc).unwrap();
        let restored: crate::Document = serde_json::from_slice(&saved).unwrap();
        assert!(restored.validate().is_ok());
        assert!(visible(&restored, 0, Pos2::new(50., 50.)));
        assert!(!visible(&restored, 1, Pos2::new(50., 50.)));
        assert!(visible(&restored, 1, Pos2::new(150., 50.)));
        assert!(!visible(&restored, 0, Pos2::new(150., 50.)));
    }
    #[test]
    fn inverted_mask_clips_textures_without_destroying_their_basis() {
        let mut image = rect([10., 10.], [110., 110.]);
        image.style.raster_tile = true;
        image.style.media_image = Some(99);
        let area = crate::features::ink(&[rect([10., 10.], [60., 110.])]);
        let parts = clip(&image, &area, true);
        assert!(!parts.is_empty());
        assert!(
            parts
                .iter()
                .all(|m| m.style.media_image == Some(99) && m.style.texture_basis.is_some())
        );
        assert!(
            !parts
                .iter()
                .any(|m| crate::vector::contains_fill(m, Pos2::new(30., 50.)))
        );
        assert!(
            parts
                .iter()
                .any(|m| crate::vector::contains_fill(m, Pos2::new(90., 50.)))
        );
    }
}

#[cfg(test)]
#[test]
fn shape_mask_interpolates_and_does_not_modify_target_geometry() {
    let mut e = tests::editor();
    e.add_shape_mask(false);
    e.doc.layers[1].frames[0] = vec![tests::rect([10., 10.], [80., 210.])].into();
    e.doc.layers[1].frames[10] = vec![tests::rect([140., 10.], [210., 210.])].into();
    e.doc.layers[1].options.keys[10] = true;
    e.doc.layers[1].options.tweens = vec![crate::animation::Tween {
        start: 0,
        end: 10,
        curve: Default::default(),
        shape: true,
    }]
    .into();
    assert!(tests::visible(&e.doc, 5, Pos2::new(110., 100.)));
    assert!(!tests::visible(&e.doc, 5, Pos2::new(20., 100.)));
    assert!(!tests::visible(&e.doc, 5, Pos2::new(200., 100.)));
}

#[cfg(test)]
#[test]
#[ignore = "headless shape mask UI visual QA"]
fn shape_mask_visual_qa() {
    let mut e = tests::editor();
    e.add_shape_mask(true);
    crate::manual_visuals::screenshot_setup(
        e.doc.clone(),
        0,
        Tool::Transform,
        false,
        std::path::Path::new("target/qa/shape-mask.png"),
        |_, editor| {
            editor.layer = 1;
            editor.canvas.selected = vec![(1, 0)];
        },
    );
}
