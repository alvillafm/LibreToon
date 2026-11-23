//! One history entry per continuous inspector gesture, including text entry.
use crate::{Document, Editor};
use eframe::egui;

pub(crate) struct Edit {
    id: egui::Id,
    original: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    selection: Vec<(usize, usize)>,
}
impl Editor {
    pub(crate) fn property_reference(
        &self,
        id: impl std::hash::Hash + std::fmt::Debug,
    ) -> Option<&Document> {
        let id = egui::Id::unique(id);
        self.property_edit
            .as_ref()
            .filter(|edit| edit.id == id)
            .map(|edit| &edit.original)
    }
    pub(crate) fn begin_property_edit(&mut self, id: impl std::hash::Hash + std::fmt::Debug) {
        let id = egui::Id::unique(id);
        if self
            .property_edit
            .as_ref()
            .is_some_and(|edit| edit.id == id)
        {
            return;
        }
        let edit = Edit {
            id,
            original: self.doc.clone(),
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            selection: self.canvas.selected.clone(),
        };
        self.checkpoint();
        self.canvas.selected = edit.selection.clone();
        self.property_edit = Some(edit);
    }
    pub(crate) fn finish_property_edit(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Some(edit) = self.property_edit.take() {
                self.doc = edit.original;
                self.undo = edit.undo;
                self.redo = edit.redo;
                self.canvas.selected = edit.selection;
                self.recover();
            }
        } else if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.property_edit = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dragging_scale_on_an_untracked_first_frame_is_linear_and_undoes_once() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        e.doc = crate::fresh_document();
        e.doc.layers[0].frames[0] = vec![crate::Mark {
            points: vec![[100., 100.], [200., 200.]].into(),
            rectangle: true,
            closed: true,
            fill: Some([255, 0, 0, 255]),
            ..Default::default()
        }]
        .into();
        e.canvas.selected = vec![(0, 0)];
        let original = serde_json::to_value(&e.doc).unwrap();
        let tick = |e: &mut Editor, events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(400., 500.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    e.motion_transform_ui(ui);
                    e.finish_property_edit(&ctx);
                },
            );
            out.textures_delta.clear();
            out
        };
        tick(&mut e, vec![]);
        let out = tick(&mut e, vec![]);
        let point = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.job.text == "100.0" || t.galley.job.text == "100" =>
                {
                    Some(t.pos + egui::Vec2::new(8., 6.))
                }
                _ => None,
            })
            .expect("scale field");
        let button = |p, pressed| egui::Event::PointerButton {
            pos: p,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Default::default(),
        };
        tick(
            &mut e,
            vec![egui::Event::PointerMoved(point), button(point, true)],
        );
        for offset in [20., 40.] {
            tick(
                &mut e,
                vec![egui::Event::PointerMoved(point + egui::Vec2::X * offset)],
            );
        }
        tick(&mut e, vec![button(point + egui::Vec2::X * 40., false)]);
        let width = crate::vector::control_bounds(&e.doc.layers[0].frames[0][0]).width();
        assert!(
            (width - 120.).abs() < 1.5,
            "scale gesture produced width {width}"
        );
        assert_eq!(e.undo.len(), 1);
        e.history(false);
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), original);
    }
    #[test]
    fn repeated_property_updates_undo_once_and_escape_restores_history() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        e.doc = crate::fresh_document();
        let original = serde_json::to_value(&e.doc).unwrap();
        for value in 100..125 {
            e.begin_property_edit(("size", 0));
            e.doc.width = value;
        }
        assert_eq!(e.undo.len(), 1);
        e.history(false);
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), original);
        let undo = e.undo.len();
        let redo = e.redo.len();
        e.begin_property_edit("cancel");
        e.doc.width = 900;
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            },
            |_| e.finish_property_edit(&ctx),
        );
        out.textures_delta.clear();
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), original);
        assert_eq!((e.undo.len(), e.redo.len()), (undo, redo));
    }
}
