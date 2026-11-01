#[cfg(test)]
use crate::Document;
use crate::Editor;
use crate::design::{ACCENT, Icon, MUTED, button, draw};
#[cfg(test)]
use eframe::egui::Pos2;
use eframe::egui::{self, Color32, Rect, Sense, Vec2};
impl Editor {
    pub fn workspace(&mut self, ui: &mut egui::Ui) {
        self.animation.thumbnail_cache.begin_frame();
        let ctx = ui.ctx().clone();
        egui::Panel::top("application")
            .exact_size(42.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(28.), Sense::hover());
                    crate::design::logo(ui.painter(), logo_rect);
                    ui.label(egui::RichText::new("LibreToon").strong().size(14.));
                    ui.add_space(8.);
                    ui.menu_button(crate::i18n::tr("Archivo"), |ui| {
                        if button(ui, Icon::New, "Nuevo", "Nuevo documento", false, true).clicked()
                        {
                            self.new_document();
                            ui.close();
                        }
                        if button(ui, Icon::Open, "Abrir…", "Abrir proyecto", false, true).clicked()
                        {
                            self.open();
                            ui.close();
                        }
                        if button(ui, Icon::Save, "Guardar", "Ctrl+S", false, true).clicked() {
                            self.save_project(false);
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::SaveAs,
                            "Guardar como…",
                            "Guardar una copia",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.save_project(true);
                            ui.close();
                        }
                        ui.separator();
                        if button(
                            ui,
                            Icon::Settings,
                            "Ajustes de documento…",
                            "Ajustes de documento…",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.open_document_settings();
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Settings,
                            "Preferencias del programa…",
                            "Preferencias del programa…",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.studio.preferences_open = true;
                            ui.close();
                        }
                        ui.separator();
                        if button(
                            ui,
                            Icon::Export,
                            "Exportar vídeo…",
                            "Exportar vídeo…",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.open_export(false);
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Film,
                            "Exportar GIF…",
                            "Exportar GIF…",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.open_export(true);
                            ui.close();
                        }
                    });
                    ui.menu_button(crate::i18n::tr("Edición"), |ui| {
                        if button(
                            ui,
                            Icon::Copy,
                            "Copiar",
                            "Copiar",
                            false,
                            !self.canvas.selected.is_empty(),
                        )
                        .clicked()
                        {
                            self.copy_selection(false);
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Cut,
                            "Cortar",
                            "Cortar",
                            false,
                            !self.canvas.selected.is_empty(),
                        )
                        .clicked()
                        {
                            self.copy_selection(true);
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Paste,
                            "Pegar",
                            "Pegar",
                            false,
                            !self.studio.clipboard.is_empty(),
                        )
                        .clicked()
                        {
                            self.paste_selection();
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Symbol,
                            "Convertir a símbolo · F8",
                            "Convertir a símbolo",
                            false,
                            !self.canvas.selected.is_empty(),
                        )
                        .clicked()
                        {
                            self.convert_symbol();
                            ui.close();
                        }
                        ui.separator();

                        if button(
                            ui,
                            Icon::Undo,
                            "Deshacer",
                            "Ctrl+Z",
                            false,
                            !self.undo.is_empty(),
                        )
                        .clicked()
                        {
                            self.history(false);
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Redo,
                            "Rehacer",
                            "Ctrl+Y",
                            false,
                            !self.redo.is_empty(),
                        )
                        .clicked()
                        {
                            self.history(true);
                            ui.close();
                        }
                    });
                    ui.add_space(12.);
                    ui.separator();
                    if button(ui, Icon::Open, "", "Abrir proyecto", false, true).clicked() {
                        self.open();
                    }
                    if button(ui, Icon::Save, "", "Guardar · Ctrl+S", false, true).clicked() {
                        self.save_project(false);
                    }
                    ui.separator();
                    if button(
                        ui,
                        Icon::Undo,
                        "",
                        "Deshacer · Ctrl+Z",
                        false,
                        !self.undo.is_empty(),
                    )
                    .clicked()
                    {
                        self.history(false);
                    }
                    if button(
                        ui,
                        Icon::Redo,
                        "",
                        "Rehacer · Ctrl+Y",
                        false,
                        !self.redo.is_empty(),
                    )
                    .clicked()
                    {
                        self.history(true);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.window_controls(ui);
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 32.),
                            Sense::click_and_drag(),
                        );
                        let _ = rect;
                        if response.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        if response.double_clicked() {
                            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                    });
                });
            });
        self.document_tabs(ui);
        if self.hub.home {
            self.home_ui(ui);
            self.notice_ui(&ctx);
            return;
        }
        egui::Panel::left("tools")
            .exact_size(88.)
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    use crate::canvas::Tool::*;
                    for (label, rows) in [
                        (
                            "SELECCIÓN",
                            vec![(Select, Object), (Lasso, PolygonSelect), (Wand, Wand)],
                        ),
                        ("EDICIÓN", vec![(Transform, Mirror)]),
                        (
                            "PINTURA",
                            vec![
                                (Brush, Eraser),
                                (Bucket, Eyedropper),
                                (Airbrush, SketchBrush),
                            ],
                        ),
                        ("Rigging", vec![(Bones, Puppet)]),
                        ("TEXTO", vec![(Text, Text)]),
                        ("VECTORIAL", vec![(VectorBrush, Pen), (Node, Width)]),
                        ("FORMAS", vec![(Line, Rectangle), (Ellipse, Polygon)]),
                        (
                            "LIENZO",
                            vec![
                                (Pan, Zoom),
                                (Ruler, Symmetry),
                                (Perspective, Camera),
                                (Mannequin, Mannequin),
                            ],
                        ),
                    ] {
                        ui.add_space(7.);
                        ui.label(
                            egui::RichText::new(crate::i18n::tr(label))
                                .size(8.)
                                .color(MUTED),
                        );
                        ui.separator();
                        for (left, right) in rows {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.;
                                for (index, tool) in [left, right].into_iter().enumerate() {
                                    if index == 1 && left == right {
                                        continue;
                                    }
                                    self.tool_button(ui, tool);
                                }
                            });
                        }
                    }
                    ui.add_space(7.);
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("COLOR"))
                            .size(8.)
                            .color(MUTED),
                    );
                    ui.separator();
                    self.color_swatches(ui);
                });
            });
        egui::Panel::right("properties")
            .exact_size(305.)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        Icon::Settings,
                        "Propiedades",
                        "Propiedades",
                        !self.feature.library,
                        true,
                    )
                    .clicked()
                    {
                        self.feature.library = false;
                    }
                    if button(
                        ui,
                        Icon::Library,
                        "Biblioteca",
                        "Biblioteca",
                        self.feature.library,
                        true,
                    )
                    .clicked()
                    {
                        self.feature.library = true;
                    }
                });
                ui.separator();
                if self.feature.library {
                    self.library_ui(ui);
                } else {
                    self.properties_ui(ui);
                }
            });
        self.sequencer(ui);
        egui::Panel::top("document_tab")
            .exact_size(34.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    draw(
                        ui.painter(),
                        Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                        Icon::Film,
                        ACCENT,
                    );
                    ui.add_space(26.);
                    ui.label(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or("Sin título.fla".into()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if button(
                            ui,
                            Icon::Stage,
                            "",
                            "Solo escenario",
                            self.studio.stage_only,
                            true,
                        )
                        .clicked()
                        {
                            self.studio.stage_only = !self.studio.stage_only;
                        }
                        if button(
                            ui,
                            Icon::Fit,
                            "",
                            "Encajar escenario y centrar vista",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.canvas.fit();
                        }
                        if button(
                            ui,
                            Icon::Rotate,
                            "",
                            "Rotar lienzo: arrastra sobre el escenario",
                            self.tool == crate::canvas::Tool::RotateView,
                            true,
                        )
                        .clicked()
                        {
                            self.tool = crate::canvas::Tool::RotateView;
                        }
                        let mut degrees = self.canvas.angle.to_degrees();
                        if ui
                            .add(egui::DragValue::new(&mut degrees).speed(1.).suffix("°"))
                            .on_hover_text(
                                "Ángulo de la vista. Doble clic para escribir 0 y restablecer.",
                            )
                            .changed()
                        {
                            self.canvas.angle = degrees.rem_euclid(360.).to_radians();
                        }
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", self.canvas.actual_scale * 100.))
                                .size(11.)
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::Panel::top("scene_bar")
            .exact_size(28.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if !self.feature.symbol_stack.is_empty() {
                        if button(
                            ui,
                            Icon::Previous,
                            "Volver a escena",
                            "Volver a escena",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.exit_symbol();
                        }
                        if let Some((_, _, _, id)) = self.feature.symbol_stack.last() {
                            if let Some(symbol) =
                                self.doc.extras.symbols.iter().find(|s| s.id == *id)
                            {
                                ui.label(
                                    egui::RichText::new(format!("/ {}", symbol.name)).color(ACCENT),
                                );
                            }
                        }
                    } else {
                        let label = self.scene_label();
                        if button(ui, Icon::Film, &label, "Scene manager", false, true).clicked() {
                            self.animation.scenes_open = true;
                        }
                        ui.label(
                            egui::RichText::new(crate::i18n::tr(" /  Escenario"))
                                .size(11.)
                                .color(MUTED),
                        );
                    }
                });
            });
        if !ctx.egui_wants_keyboard_input()
            && !self.animation.scenes_open
            && self.animation.picker.is_none()
            && !self.studio.preferences_open
            && self.studio.export.is_none()
        {
            if self.studio.prefs.bindings[17].key == "W"
                && !self.studio.prefs.bindings[17].ctrl
                && !self.studio.prefs.bindings[17].shift
                && !self.studio.prefs.bindings[17].alt
                && !self.animation.scenes_open
                && self.animation.picker.is_none()
                && !self.studio.preferences_open
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::W))
            {
                self.activate_tool(crate::canvas::Tool::Width);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::X)) {
                self.swap_colors();
                self.change_selected_style(true, true, false, false);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::D)) {
                self.color = Color32::BLACK;
                self.fill_color = Color32::WHITE;
                self.stroke_enabled = true;
                self.fill_enabled = true;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F8)) {
                self.convert_symbol();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Comma)) {
                self.frame = self.frame.saturating_sub(1);
                self.playing = false;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Period)) {
                self.frame = (self.frame + 1).min(self.doc.layers[0].frames.len() - 1);
                self.playing = false;
            }
            self.tool_shortcuts(&ctx);
        }
        if !ctx.input(|i| i.pointer.primary_down()) {
            self.style_transaction = false;
        }
        self.document_settings_ui(&ctx);
        self.preferences_ui(&ctx);
        self.scenes_ui(&ctx);
        self.frame_picker_ui(&ctx);
        self.export_ui(&ctx);
        self.notice_ui(&ctx);
    }
    fn new_document(&mut self) {
        self.add_document(crate::fresh_document(), None);
    }
}

/// Include partially visible cells, clamp to content and skip offscreen work.
pub(crate) fn visible_range(
    origin: f32,
    size: f32,
    count: usize,
    min: f32,
    max: f32,
) -> std::ops::Range<usize> {
    let first = ((min - origin) / size).floor().max(0.) as usize;
    let end = ((max - origin) / size).ceil().max(0.) as usize;
    first.min(count)..end.min(count).max(first.min(count))
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn timeline_only_visits_visible_cells_at_any_scroll_offset() {
        assert_eq!(visible_range(180., 22., 10_000, 0., 1000.), 0..38);
        assert_eq!(visible_range(-10000., 22., 10_000, 0., 1000.), 454..500);
        assert_eq!(visible_range(180., 22., 10_000, 0., 100.), 0..0);
        assert_eq!(
            visible_range(-300000., 22., 10_000, 0., 1000.),
            10000..10000
        );
        assert_eq!(visible_range(24., 27., 100, 25., 100.), 0..3);
    }
}

#[cfg(test)]
mod timeline_ui_tests {
    use super::*;
    fn draw_workspace(editor: &mut Editor, ctx: &egui::Context) -> usize {
        editor.hub.home = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 860.))),
                ..Default::default()
            },
            |ui| editor.workspace(ui),
        );
        output.textures_delta.clear();
        output.shapes.len()
    }
    #[test]
    fn large_timeline_keeps_paint_work_bounded_by_viewport() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut editor = Editor::new(&cc);
        editor.doc = Document::default();
        editor.doc.layers[0].frames.resize(100, Default::default());
        let layer = editor.doc.layers[0].clone();
        editor.doc.layers.resize(10, layer);
        draw_workspace(&mut editor, &ctx);
        let small = draw_workspace(&mut editor, &ctx);
        editor.doc.layers[0]
            .frames
            .resize(10_000, Default::default());
        let layer = editor.doc.layers[0].clone();
        editor.doc.layers.resize(100, layer);
        let large = draw_workspace(&mut editor, &ctx);
        eprintln!("timeline shapes: 10x100={small}, 100x10000={large}");
        assert!(large <= small + 32, "small={small}, large={large}");
    }
}

#[cfg(test)]
mod shortcut_regressions {
    use super::*;
    #[test]
    fn shift_w_uses_key_event_modifiers_even_after_shift_was_released() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut editor = Editor::new(&cc);
        editor.doc = Document::default();
        editor.hub.home = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 860.))),
                events: vec![
                    egui::Event::Key {
                        key: egui::Key::W,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::SHIFT,
                    },
                    egui::Event::Key {
                        key: egui::Key::W,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| editor.workspace(ui),
        );
        output.textures_delta.clear();
        assert!(editor.tool == crate::canvas::Tool::Width);
    }
}
