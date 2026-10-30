use crate::{
    Editor, animation,
    design::{Icon, button},
    i18n::tr,
};
use eframe::egui;
impl Editor {
    pub fn scene_label(&self) -> String {
        let i = self.doc.extras.active_scene;
        self.doc.extras.scenes.get(i).map_or_else(
            || tr("Escena 1").to_string(),
            |s| format!("{}. {}", i + 1, s.name),
        )
    }
    fn ensure_scenes(&mut self) {
        if self.doc.extras.scenes.is_empty() {
            let s = animation::scene_from(&self.doc, crate::features::id(), "Scene".into());
            self.doc.extras.scenes.push(s);
        }
    }
    pub fn switch_scene(&mut self, index: usize) {
        if index >= self.doc.extras.scenes.len() {
            return;
        }
        self.finish_pending_pen();
        animation::store_scene(&mut self.doc);
        let s = self.doc.extras.scenes[index].clone();
        self.doc.layers = s.layers;
        self.doc.extras.folders = s.folders;
        self.doc.extras.cameras = s.cameras;
        self.doc.extras.perspective = s.perspective;
        self.doc.extras.active_scene = index;
        self.frame = 0;
        self.layer = 0;
        self.canvas.selected.clear();
        self.style_selection.clear();
        self.feature.selected_folder = None;
        self.animation.picker = None;
        self.feature.selected_camera = None;
    }
    pub fn scenes_ui(&mut self, ctx: &egui::Context) {
        if !self.animation.scenes_open {
            return;
        }
        let mut open = true;
        egui::Window::new(tr("Gestor de escenas"))
            .open(&mut open)
            .default_width(470.)
            .default_pos(ctx.content_rect().center() - egui::vec2(235., 160.))
            .show(ctx, |ui| {
                self.ensure_scenes();
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        Icon::Plus,
                        "+ Scene",
                        "Add scene",
                        false,
                        self.doc.extras.scenes.len() < 100,
                    )
                    .clicked()
                    {
                        self.leave_symbol_context_for_document_change();
                        self.checkpoint();
                        animation::store_scene(&mut self.doc);
                        let mut fresh = crate::fresh_document();
                        fresh.width = self.doc.width;
                        fresh.height = self.doc.height;
                        let s =
                            animation::scene_from(&fresh, crate::features::id(), "Scene".into());
                        self.doc.extras.scenes.push(s);
                        self.switch_scene(self.doc.extras.scenes.len() - 1);
                        self.recover();
                    }
                    if button(
                        ui,
                        Icon::Play,
                        tr("Reproducir todas las escenas"),
                        "Preview movie",
                        self.animation.play_all && self.playing,
                        true,
                    )
                    .clicked()
                    {
                        self.leave_symbol_context_for_document_change();
                        self.switch_scene(0);
                        self.animation.play_all = true;
                        self.playing = true;
                        self.tick = std::time::Instant::now();
                    }
                });
                ui.separator();
                let mut reorder = None;
                egui::ScrollArea::vertical()
                    .max_height(400.)
                    .show(ui, |ui| {
                        for i in 0..self.doc.extras.scenes.len() {
                            let s = self.doc.extras.scenes[i].clone();
                            ui.horizontal(|ui| {
                                let (rect, drag) = ui
                                    .allocate_exact_size(egui::vec2(24., 28.), egui::Sense::drag());
                                crate::design::draw(
                                    ui.painter(),
                                    rect,
                                    Icon::Pan,
                                    crate::design::MUTED,
                                );
                                if drag.drag_started() {
                                    self.animation.scene_drag = Some(i);
                                }
                                ui.label(format!("{}.", i + 1));
                                if self.animation.scene_rename == Some(s.id) {
                                    let r = ui
                                        .text_edit_singleline(&mut self.doc.extras.scenes[i].name);
                                    if r.changed() {
                                        self.recover();
                                    }
                                    if r.lost_focus() {
                                        self.animation.scene_rename = None;
                                        self.recover();
                                    }
                                } else {
                                    let r = ui.add_sized(
                                        [300., 28.],
                                        egui::Button::selectable(
                                            i == self.doc.extras.active_scene,
                                            &s.name,
                                        ),
                                    );
                                    if r.double_clicked() {
                                        self.checkpoint();
                                        self.animation.scene_rename = Some(s.id);
                                    } else if r.clicked() {
                                        self.leave_symbol_context_for_document_change();
                                        self.switch_scene(i);
                                    }
                                    if ui.input(|x| {
                                        x.pointer.any_released()
                                            && x.pointer
                                                .interact_pos()
                                                .is_some_and(|p| r.rect.contains(p))
                                    }) {
                                        if let Some(from) = self.animation.scene_drag.take() {
                                            reorder = Some((from, i));
                                        }
                                    }
                                }
                                if button(
                                    ui,
                                    Icon::Trash,
                                    "",
                                    "Delete scene",
                                    false,
                                    self.doc.extras.scenes.len() > 1,
                                )
                                .clicked()
                                {
                                    self.animation.scene_delete = Some(s.id);
                                }
                            });
                        }
                    });
                if let Some((a, b)) = reorder {
                    if a != b {
                        self.checkpoint();
                        animation::store_scene(&mut self.doc);
                        let active = self.doc.extras.scenes[self.doc.extras.active_scene].id;
                        let s = self.doc.extras.scenes.remove(a);
                        self.doc.extras.scenes.insert(b, s);
                        self.doc.extras.active_scene = self
                            .doc
                            .extras
                            .scenes
                            .iter()
                            .position(|s| s.id == active)
                            .unwrap();
                        self.recover();
                    }
                }
                if ui.input(|i| i.pointer.any_released()) {
                    self.animation.scene_drag = None;
                }
            });
        self.animation.scenes_open = open;
        if let Some(id) = self.animation.scene_delete {
            egui::Modal::new(egui::Id::unique("confirm-delete-scene")).show(ctx, |ui| {
                ui.heading(tr("Eliminar escena"));
                ui.label(tr(
                    "Se eliminará todo el contenido de esta escena. ¿Continuar?",
                ));
                ui.horizontal(|ui| {
                    if button(ui, Icon::Close, tr("Cancelar"), "Cancel", false, true).clicked() {
                        self.animation.scene_delete = None;
                    }
                    if button(
                        ui,
                        Icon::Trash,
                        tr("Eliminar"),
                        "Delete scene and contents",
                        false,
                        true,
                    )
                    .clicked()
                    {
                        self.checkpoint();
                        animation::store_scene(&mut self.doc);
                        if let Some(i) = self.doc.extras.scenes.iter().position(|s| s.id == id) {
                            let active = self.doc.extras.scenes[self.doc.extras.active_scene].id;
                            self.doc.extras.scenes.remove(i);
                            let next = self
                                .doc
                                .extras
                                .scenes
                                .iter()
                                .position(|s| s.id == active)
                                .unwrap_or(0);
                            let s = self.doc.extras.scenes[next].clone();
                            self.doc.layers = s.layers;
                            self.doc.extras.folders = s.folders;
                            self.doc.extras.cameras = s.cameras;
                            self.doc.extras.perspective = s.perspective;
                            self.doc.extras.active_scene = next;
                            self.layer = 0;
                            self.frame = 0;
                            self.canvas.selected.clear();
                        }
                        self.animation.scene_delete = None;
                        self.recover();
                    }
                });
            });
        }
    }
}
