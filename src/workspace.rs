#[cfg(test)]
use crate::Document;
use crate::Editor;
use crate::design::{Icon, button, draw};
#[cfg(test)]
use eframe::egui::Pos2;
use eframe::egui::{self, Color32, Rect, Sense, Vec2};
fn top_menu(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    let label = egui::RichText::new(crate::i18n::tr(label)).size(13.);
    let trigger = egui::Button::new(label)
        .frame_when_inactive(false)
        .corner_radius(6.)
        .min_size(egui::vec2(46., 28.));
    egui::containers::menu::MenuButton::from_button(trigger).ui(ui, content);
}

impl Editor {
    pub fn workspace(&mut self, ui: &mut egui::Ui) {
        self.animation.thumbnail_cache.begin_frame();
        let ctx = ui.ctx().clone();
        egui::Panel::top("application")
            .exact_size(42.)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(28.), Sense::hover());
                    crate::design::logo(ui.painter(), logo_rect);
                    ui.label(egui::RichText::new("LibreToon").strong().size(14.));
                    ui.add_space(8.);
                    top_menu(ui, "Archivo", |ui| {
                        if button(ui, Icon::New, "Nuevo", "Nuevo documento", false, true).clicked()
                        {
                            self.new_document();
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Open,
                            "Abrir proyecto…",
                            "Abrir proyecto",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.open();
                            ui.close();
                        }
                        ui.menu_button(crate::i18n::tr("Importar"), |ui| {
                            if button(
                                ui,
                                Icon::Open,
                                "Adobe Animate (.fla)…",
                                "Import Adobe Animate project",
                                false,
                                true,
                            )
                            .clicked()
                            {
                                self.import_fla();
                                ui.close();
                            }
                        });
                        if button(
                            ui,
                            Icon::Open,
                            "Abrir copia guardada…",
                            "Abrir el archivo del disco en otra pestaña sin descartar cambios",
                            false,
                            self.path.is_some() && !self.hub.home,
                        )
                        .clicked()
                        {
                            self.open_saved_document_copy();
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
                        if let Some(path) = &self.path {
                            let backups: Vec<_> = (1..=3)
                                .filter_map(|version| {
                                    let file = crate::project_save::backup_path(path, version);
                                    file.is_file().then_some((version, file))
                                })
                                .collect();
                            if !backups.is_empty() {
                                ui.menu_button(crate::i18n::tr("Open previous version"), |ui| {
                                    for (version, file) in backups {
                                        if ui
                                            .button(format!(
                                                "{} {version}",
                                                crate::i18n::tr("Backup")
                                            ))
                                            .clicked()
                                        {
                                            self.loader.request_saved_copy(file);
                                            ui.close();
                                        }
                                    }
                                });
                            }
                        }
                        if !self.hub.home
                            && (self.doc.extras.imported
                                || !self.doc.extras.import_warnings.is_empty())
                            && button(
                                ui,
                                Icon::DocumentSettings,
                                "Informe de importación FLA…",
                                "Informe de importación FLA…",
                                false,
                                true,
                            )
                            .clicked()
                        {
                            self.import_report_open = true;
                            ui.close();
                        }
                        ui.separator();
                        if button(
                            ui,
                            Icon::DocumentSettings,
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
                            Icon::Preferences,
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
                            "Export Adobe Animate (.fla)…",
                            "Export Adobe Animate (.fla)…",
                            false,
                            !self.hub.home,
                        )
                        .clicked()
                        {
                            self.export_fla_copy();
                            ui.close();
                        }
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
                    top_menu(ui, "Edición", |ui| {
                        if button(
                            ui,
                            Icon::Copy,
                            "Copy frames",
                            "Copy frames",
                            false,
                            !self.hub.home,
                        )
                        .clicked()
                        {
                            self.copy_timeline_frames();
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Paste,
                            "Paste frames",
                            "Paste frames",
                            false,
                            !self.hub.home && self.can_paste_timeline_frames(),
                        )
                        .clicked()
                        {
                            self.paste_timeline_frames();
                            ui.close();
                        }
                        ui.separator();

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
                            !self.hub.home && !self.doc.layers[self.layer].options.locked,
                        )
                        .clicked()
                        {
                            if !self.paste_media_image() {
                                self.paste_selection();
                            }
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
                            &self.action_hint(crate::shortcuts::Action::Undo),
                            false,
                            self.can_undo(),
                        )
                        .clicked()
                        {
                            self.perform_action(crate::shortcuts::Action::Undo, ui.ctx());
                            ui.close();
                        }
                        if button(
                            ui,
                            Icon::Redo,
                            "Rehacer",
                            &self.action_hint(crate::shortcuts::Action::Redo),
                            false,
                            self.can_redo(),
                        )
                        .clicked()
                        {
                            self.perform_action(crate::shortcuts::Action::Redo, ui.ctx());
                            ui.close();
                        }
                    });
                    top_menu(ui, "Vista", |ui| self.view_menu(ui));
                    top_menu(ui, "Herramientas", |ui| {
                        if button(ui, Icon::Reference, "RefTool", "RefTool", false, true).clicked()
                        {
                            self.new_reference_window();
                            ui.close();
                        }
                    });
                    top_menu(ui, "Ayuda", |ui| {
                        if button(ui, Icon::Rotate, "Buscar actualizaciones", "", false, true)
                            .clicked()
                        {
                            self.updater.check(ui.ctx(), true);
                            ui.close();
                        }
                        if button(ui, Icon::Info, "Credits", "Credits", false, true).clicked() {
                            self.studio.credits_open = true;
                            ui.close();
                        }
                        crate::design::category(ui, "VERSIÓN ACTUAL:");
                        ui.label(env!("CARGO_PKG_VERSION"));
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
                        &self.action_tip(crate::shortcuts::Action::Undo),
                        false,
                        self.can_undo(),
                    )
                    .clicked()
                    {
                        self.perform_action(crate::shortcuts::Action::Undo, ui.ctx());
                    }
                    if button(
                        ui,
                        Icon::Redo,
                        "",
                        &self.action_tip(crate::shortcuts::Action::Redo),
                        false,
                        self.can_redo(),
                    )
                    .clicked()
                    {
                        self.perform_action(crate::shortcuts::Action::Redo, ui.ctx());
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
            self.document_settings_ui(&ctx);
            self.preferences_ui(&ctx);
            self.notice_ui(&ctx);
            return;
        }
        egui::Panel::left("tools")
            .exact_size(76.)
            .resizable(false)
            .show(ui, |ui| {
                let tool_height = ((ui.available_height() - 300.) / 16.).clamp(18., 24.);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.;
                    ui.spacing_mut().interact_size.y = tool_height;
                    use crate::canvas::Tool::*;
                    for (label, rows) in [
                        (
                            "SELECCIÓN",
                            vec![(Select, Object), (Lasso, PolygonSelect), (Wand, Wand)],
                        ),
                        ("EDICIÓN", vec![(Transform, Mirror)]),
                        ("PINTURA", vec![(Brush, Eraser), (Bucket, Eyedropper)]),
                        ("VECTORIAL", vec![(Pen, Node), (Width, Trim)]),
                        ("FORMAS", vec![(Line, Rectangle), (Ellipse, Polygon)]),
                        ("TEXTO", vec![(Text, Text)]),
                        (
                            "LIENZO",
                            vec![
                                (Pan, Zoom),
                                (Ruler, Symmetry),
                                (Perspective, Camera),
                                (Mannequin, Mannequin),
                            ],
                        ),
                        ("RIGGING", vec![(Bones, Puppet)]),
                    ] {
                        ui.add_space(3.);
                        ui.label(
                            egui::RichText::new(crate::i18n::tr(label))
                                .size(8.)
                                .color(crate::themes::muted()),
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
                            .color(crate::themes::muted()),
                    );
                    ui.separator();
                    self.color_swatches(ui);
                });
            });
        egui::Panel::right("properties")
            .default_size(370.)
            .min_size(245.)
            .max_size(650.)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if button(
                        ui,
                        Icon::Settings,
                        "Propiedades",
                        "Propiedades",
                        !self.feature.library && !self.clips.open,
                        true,
                    )
                    .clicked()
                    {
                        self.feature.library = false;
                        self.clips.open = false;
                    }
                    if button(
                        ui,
                        Icon::Library,
                        "Biblioteca",
                        "Biblioteca",
                        self.feature.library && !self.clips.open,
                        true,
                    )
                    .clicked()
                    {
                        self.feature.library = true;
                        self.clips.open = false;
                    }
                    if button(
                        ui,
                        Icon::Film,
                        "Animation",
                        "Animation Manager",
                        self.clips.open,
                        true,
                    )
                    .clicked()
                    {
                        self.clips.open = true;
                    }
                });
                ui.separator();
                if self.clips.open {
                    self.animation_manager_ui(ui);
                } else if self.feature.library {
                    self.library_ui(ui);
                } else {
                    self.properties_ui(ui);
                }
            });
        self.sequencer(ui);
        self.finish_animation_clip_ui(&ctx);
        self.animation_clip_dialog(&ctx);
        self.grid_settings_ui(&ctx);
        egui::Panel::top("document_tab")
            .exact_size(34.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    draw(
                        ui.painter(),
                        Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                        Icon::Film,
                        crate::themes::accent(),
                    );
                    ui.add_space(26.);
                    ui.label(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or("Sin título.ltoon".into()),
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
                            ui.ctx().request_repaint();
                        }
                        if crate::post::present(&self.doc)
                            && button(
                                ui,
                                Icon::Fx,
                                "",
                                "Show / Hide Post-Processing",
                                !self.post.hidden,
                                true,
                            )
                            .clicked()
                        {
                            self.post.hidden = !self.post.hidden;
                            ui.ctx().request_repaint();
                        }
                        if crate::depth::camera(&self.doc, self.frame).is_some_and(|c| {
                            crate::animation::camera_optics(c, self.frame).strength > 0.
                        }) {
                            if button(
                                ui,
                                Icon::Blur,
                                "",
                                "Show / Hide Camera Blur",
                                !self.depth.hide_blur,
                                true,
                            )
                            .clicked()
                            {
                                self.depth.hide_blur = !self.depth.hide_blur;
                                ui.ctx().request_repaint();
                            }
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
                            .add(
                                egui::DragValue::new(&mut degrees)
                                    .speed(1.)
                                    .suffix(crate::i18n::tr("°")),
                            )
                            .on_hover_text(crate::i18n::tr(
                                "Ángulo de la vista. Doble clic para escribir 0 y restablecer.",
                            ))
                            .changed()
                        {
                            self.canvas.angle = degrees.rem_euclid(360.).to_radians();
                        }
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", self.canvas.actual_scale * 100.))
                                .size(11.)
                                .color(crate::themes::muted()),
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
                                    egui::RichText::new(format!("/ {}", symbol.name))
                                        .color(crate::themes::accent()),
                                );
                            }
                        }
                    } else {
                        self.scene_dropdown(ui);
                        ui.label(
                            egui::RichText::new(crate::i18n::tr(" /  Escenario"))
                                .size(11.)
                                .color(crate::themes::muted()),
                        );
                    }
                });
            });
        if !ctx.egui_wants_keyboard_input()
            && !self.animation.scenes_open
            && self.animation.picker.is_none()
            && !self.studio.preferences_open
            && self.document_settings.is_none()
            && self.studio.export.is_none()
        {
            if self.studio.prefs.bindings[17].key == "W"
                && !self.studio.prefs.bindings[17].ctrl
                && !self.studio.prefs.bindings[17].shift
                && !self.studio.prefs.bindings[17].alt
                && !self.animation.scenes_open
                && self.animation.picker.is_none()
                && !self.studio.preferences_open
                && self.document_settings.is_none()
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
    pub(crate) fn new_document(&mut self) {
        self.playing = false;
        self.document_settings = Some(crate::inspector::DocumentSettings {
            creating: true,
            width: 1920,
            height: 1080,
            fps: 24,
        });
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
