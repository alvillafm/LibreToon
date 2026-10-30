use crate::design::{ACCENT, Icon, MUTED, PANEL, button, draw};
use crate::{Document, Editor, Layer};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::time::Instant;
impl Editor {
    pub fn workspace(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("application")
            .exact_size(42.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(28.), Sense::hover());
                    crate::design::logo(ui.painter(), logo_rect);
                    ui.label(egui::RichText::new("LibreToon").strong().size(14.));
                    ui.add_space(8.);
                    ui.menu_button("Archivo", |ui| {
                        if button(
                            ui,
                            Icon::Settings,
                            "Ajustes de documento…",
                            "Tamaño del escenario y velocidad",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.open_document_settings();
                            ui.close();
                        }
                        ui.separator();
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
                            Icon::Export,
                            "Exportar SVG…",
                            "Exportar fotograma actual",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.export();
                            ui.close();
                        }
                    });
                    ui.menu_button("Edición", |ui| {
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
                        if button(
                            ui,
                            Icon::Export,
                            "Exportar",
                            "Exportar el fotograma como SVG",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.export();
                        }
                        ui.label(egui::RichText::new("ANIMACIÓN 2D").size(10.).color(MUTED));
                    });
                });
            });
        egui::Panel::bottom("status")
            .exact_size(25.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status).size(11.).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Rust nativo  •  LibreToon 0.2")
                                .size(10.)
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::Panel::left("tools")
            .exact_size(88.)
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    use crate::canvas::Tool::*;
                    for (label, rows) in [
                        (
                            "SELECCIÓN",
                            vec![(Select, Transform), (Lasso, PolygonSelect), (Wand, Object)],
                        ),
                        ("PINTURA", vec![(Brush, Eraser), (Bucket, Eyedropper)]),
                        ("VECTORIAL", vec![(VectorBrush, Pen), (Node, Width)]),
                        ("FORMAS", vec![(Line, Rectangle), (Ellipse, Polygon)]),
                        ("LIENZO", vec![(Pan, Zoom)]),
                    ] {
                        ui.add_space(7.);
                        ui.label(egui::RichText::new(label).size(8.).color(MUTED));
                        ui.separator();
                        for (left, right) in rows {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.;
                                for tool in [left, right] {
                                    if button(
                                        ui,
                                        tool.icon(),
                                        "",
                                        tool.name(),
                                        self.tool == tool,
                                        true,
                                    )
                                    .clicked()
                                    {
                                        self.activate_tool(tool);
                                    }
                                }
                            });
                        }
                    }
                    ui.add_space(7.);
                    ui.label(egui::RichText::new("COLOR").size(8.).color(MUTED));
                    ui.separator();
                    self.color_swatches(ui);
                });
            });
        egui::Panel::right("properties")
            .exact_size(248.)
            .resizable(true)
            .show(ui, |ui| self.properties_ui(ui));
        egui::Panel::bottom("timeline")
            .exact_size(224.)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Línea de tiempo").strong());
                    ui.separator();
                    if button(ui, Icon::First, "", "Primer fotograma", false, true).clicked() {
                        self.frame = 0;
                        self.playing = false;
                    }
                    if button(
                        ui,
                        Icon::Previous,
                        "",
                        "Fotograma anterior",
                        false,
                        self.frame > 0,
                    )
                    .clicked()
                    {
                        self.frame -= 1;
                        self.playing = false;
                    }
                    if button(
                        ui,
                        if self.playing {
                            Icon::Pause
                        } else {
                            Icon::Play
                        },
                        "",
                        if self.playing { "Pausar" } else { "Reproducir" },
                        self.playing,
                        true,
                    )
                    .clicked()
                    {
                        self.playing = !self.playing;
                        self.tick = Instant::now();
                    }
                    if button(
                        ui,
                        Icon::Next,
                        "",
                        "Fotograma siguiente",
                        false,
                        self.frame + 1 < self.doc.layers[0].frames.len(),
                    )
                    .clicked()
                    {
                        self.frame += 1;
                        self.playing = false;
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("{:03}", self.frame + 1))
                            .monospace()
                            .color(ACCENT),
                    );
                    ui.label(egui::RichText::new(format!("/ {}",self.doc.layers[0].frames.len())).color(MUTED));
                    let mut fps=self.doc.fps;
                    if ui.add(egui::DragValue::new(&mut fps).range(1..=240).suffix(" fps").update_while_editing(false)).on_hover_text("Velocidad del documento").changed(){self.set_fps(fps);}
                    ui.separator();
                    if !self.duration_editing{self.timeline_duration=self.doc.layers[0].frames.len() as f32/self.doc.fps as f32;}
                    let response=ui.add(egui::DragValue::new(&mut self.timeline_duration).range(1./self.doc.fps as f32..=10000./self.doc.fps as f32).speed(0.05).fixed_decimals(2).suffix(" s").update_while_editing(false)).on_hover_text("Duración total. Al acortar se recortan los fotogramas finales; Ctrl+Z permite recuperarlos.");
                    let editing=response.dragged()||response.has_focus();
                    if response.drag_stopped()||response.lost_focus()||(response.changed()&&!editing){self.set_duration(self.timeline_duration);}
                    self.duration_editing=editing;
                    ui.separator();
                    if button(ui, Icon::Onion, "", "Papel cebolla", self.onion, true).clicked() {
                        self.onion = !self.onion;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if button(ui, Icon::Layers, "", "Añadir capa", false, self.doc.layers.len()<100).clicked() {
                        self.checkpoint();
                        let n = self.doc.layers.len();
                        let frame_count = self.doc.layers[0].frames.len();
                        self.doc.layers.push(Layer {
                            name: format!("Capa {}", n + 1),
                            visible: true,
                            frames: vec![Default::default(); frame_count].into(),
                        });
                        self.layer = n;
                        self.recover();
                    }
                    if button(ui, Icon::Plus, "", "Insertar fotograma vacío", false, true).clicked()
                    {
                        self.insert_frame(false);
                    }
                    if button(ui, Icon::Copy, "", "Duplicar fotograma", false, true).clicked() {
                        self.insert_frame(true);
                    }
                    if button(
                        ui,
                        Icon::Trash,
                        "",
                        "Eliminar fotograma",
                        false,
                        self.doc.layers[0].frames.len() > 1,
                    )
                    .clicked()
                    {
                        self.checkpoint();
                        for l in &mut self.doc.layers {
                            l.frames.remove(self.frame);
                        }
                        self.frame = self.frame.min(self.doc.layers[0].frames.len() - 1);
                        self.recover();
                    }
                    ui.separator();
                    if button(ui,Icon::Up,"","Subir capa activa",false,self.layer+1<self.doc.layers.len()).clicked(){self.checkpoint();self.doc.layers.swap(self.layer,self.layer+1);self.layer+=1;self.recover();}
                    if button(ui,Icon::Down,"","Bajar capa activa",false,self.layer>0).clicked(){self.checkpoint();self.doc.layers.swap(self.layer,self.layer-1);self.layer-=1;self.recover();}
                    if button(ui,Icon::Trash,"","Eliminar capa activa",false,self.doc.layers.len()>1).clicked(){self.checkpoint();self.doc.layers.remove(self.layer);self.layer=self.layer.min(self.doc.layers.len()-1);self.renaming_layer=None;self.recover();}
                    ui.label(egui::RichText::new("CAPAS").size(10.).color(MUTED));
                });
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let count = self.doc.layers[0].frames.len();
                        let row_h = 27.;
                        let label_w = 180.;
                        let cell_w = 22.;
                        let (r, _) = ui.allocate_exact_size(
                            Vec2::new(
                                label_w + count as f32 * cell_w,
                                24. + self.doc.layers.len() as f32 * row_h,
                            ),
                            Sense::hover(),
                        );
                        let visible = ui.clip_rect();
                        let frames = visible_range(
                            r.left() + label_w,
                            cell_w,
                            count,
                            visible.left(),
                            visible.right(),
                        );
                        let rows = visible_range(
                            r.top() + 24.,
                            row_h,
                            self.doc.layers.len(),
                            visible.top(),
                            visible.bottom(),
                        );
                        let painter = ui.painter().clone();
                        painter.rect_filled(
                            Rect::from_min_size(r.min, Vec2::new(r.width(), 24.)),
                            0.,
                            Color32::from_rgb(32, 33, 37),
                        );
                        painter.text(
                            r.min + Vec2::new(12., 12.),
                            egui::Align2::LEFT_CENTER,
                            "Nombre de capa",
                            egui::FontId::proportional(10.),
                            MUTED,
                        );
                        for f in frames.clone() {
                            let x = r.left() + label_w + f as f32 * cell_w;
                            if f == 0 || (f + 1) % 5 == 0 {
                                painter.text(
                                    Pos2::new(x + cell_w / 2., r.top() + 12.),
                                    egui::Align2::CENTER_CENTER,
                                    (f + 1).to_string(),
                                    egui::FontId::monospace(10.),
                                    MUTED,
                                );
                            }
                        }
                        for row in rows {
                            let l = self.doc.layers.len() - 1 - row;
                            let y = r.top() + 24. + row as f32 * row_h;
                            let name_rect = Rect::from_min_size(
                                Pos2::new(r.left(), y),
                                Vec2::new(label_w, row_h),
                            );
                            painter.rect_filled(
                                name_rect,
                                0.,
                                if self.layer == l {
                                    Color32::from_rgb(46, 61, 80)
                                } else {
                                    PANEL
                                },
                            );
                            let eye = Rect::from_min_size(
                                name_rect.min + Vec2::new(5., 3.),
                                Vec2::splat(21.),
                            );
                            draw(
                                &painter,
                                eye,
                                if self.doc.layers[l].visible {
                                    Icon::Eye
                                } else {
                                    Icon::EyeOff
                                },
                                MUTED,
                            );
                            painter.line_segment(
                                [
                                    Pos2::new(r.left() + 33., y + 7.),
                                    Pos2::new(r.left() + 33., y + 20.),
                                ],
                                Stroke::new(3., ACCENT),
                            );
                            let clipped =
                                painter.with_clip_rect(name_rect.shrink2(Vec2::new(4., 0.)));
                            if self.renaming_layer!=Some(l){
                            clipped.text(
                                Pos2::new(r.left() + 42., y + row_h / 2.),
                                egui::Align2::LEFT_CENTER,
                                &self.doc.layers[l].name,
                                egui::FontId::proportional(11.),
                                Color32::from_gray(218),
                            );
                            }
                            if ui
                                .interact(eye, ui.scope_id().with(("eye", l)), Sense::click())
                                .on_hover_text("Mostrar / ocultar capa")
                                .clicked()
                            {
                                self.checkpoint();
                                self.doc.layers[l].visible = !self.doc.layers[l].visible;
                                self.recover();
                            }
                            if self.renaming_layer==Some(l){
                                let mut name=self.doc.layers[l].name.clone();let field=Rect::from_min_max(Pos2::new(r.left()+39.,y+2.),name_rect.max-Vec2::new(3.,2.));
                                let response=ui.put(field,egui::TextEdit::singleline(&mut name).id(egui::Id::unique(("layer-name",l))).frame(egui::Frame::NONE));
                                if self.rename_focus{response.request_focus();self.rename_focus=false;}
                                if response.changed(){self.checkpoint();self.doc.layers[l].name=name;self.recover();}
                                if response.lost_focus()||ui.input(|i|i.key_pressed(egui::Key::Enter)){self.renaming_layer=None;}
                            }else{
                                let response=ui.interact(Rect::from_min_max(Pos2::new(r.left()+30.,y),name_rect.max),ui.scope_id().with(("name",l)),Sense::click()).on_hover_text("Doble clic para cambiar el nombre");
                                if response.clicked(){self.layer=l;}
                                if response.double_clicked(){self.renaming_layer=Some(l);self.rename_focus=true;}
                            }
                            for f in frames.clone() {
                                let cell = Rect::from_min_size(
                                    Pos2::new(r.left() + label_w + f as f32 * cell_w, y),
                                    Vec2::new(cell_w, row_h),
                                );
                                let active = f == self.frame;
                                let bg = if active {
                                    Color32::from_rgb(47, 83, 124)
                                } else if (f / 5) % 2 == 0 {
                                    Color32::from_rgb(55, 57, 62)
                                } else {
                                    Color32::from_rgb(48, 50, 55)
                                };
                                painter.rect_filled(cell, 0., bg);
                                painter.rect_stroke(
                                    cell,
                                    0.,
                                    Stroke::new(0.5, Color32::from_rgb(30, 31, 35)),
                                    egui::StrokeKind::Inside,
                                );
                                if !self.doc.layers[l].frames[f].is_empty() {
                                    painter.circle_filled(
                                        cell.center(),
                                        3.3,
                                        if active {
                                            ACCENT
                                        } else {
                                            Color32::from_gray(204)
                                        },
                                    );
                                } else {
                                    painter.circle_stroke(
                                        cell.center(),
                                        2.6,
                                        Stroke::new(1., Color32::from_gray(102)),
                                    );
                                }
                                if ui
                                    .interact(
                                        cell,
                                        ui.scope_id().with(("frame", l, f)),
                                        Sense::click(),
                                    )
                                    .on_hover_text(format!(
                                        "{} · Fotograma {}",
                                        self.doc.layers[l].name,
                                        f + 1
                                    ))
                                    .clicked()
                                {
                                    self.frame = f;
                                    self.layer = l;
                                    self.playing = false;
                                }
                            }
                        }
                        let x = r.left() + label_w + (self.frame as f32 + 0.5) * cell_w;
                        painter.line_segment(
                            [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
                            Stroke::new(1., ACCENT),
                        );
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                Pos2::new(x - 5., r.top()),
                                Pos2::new(x + 5., r.top()),
                                Pos2::new(x + 5., r.top() + 8.),
                                Pos2::new(x, r.top() + 13.),
                                Pos2::new(x - 5., r.top() + 8.),
                            ],
                            ACCENT,
                            Stroke::NONE,
                        ));
                    });
            });
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
                    ui.label(egui::RichText::new("Escena 1").size(11.).color(ACCENT));
                    ui.label(egui::RichText::new(" /  Escenario").size(11.).color(MUTED));
                });
            });
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::W)) {
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
            for (key, tool) in [
                (egui::Key::E, crate::canvas::Tool::Eraser),
                (egui::Key::I, crate::canvas::Tool::Eyedropper),
                (egui::Key::L, crate::canvas::Tool::Lasso),
                (egui::Key::H, crate::canvas::Tool::Pan),
                (egui::Key::Z, crate::canvas::Tool::Zoom),
                (egui::Key::O, crate::canvas::Tool::Ellipse),
                (egui::Key::N, crate::canvas::Tool::Line),
                (egui::Key::V, crate::canvas::Tool::Select),
                (egui::Key::Q, crate::canvas::Tool::Transform),
                (egui::Key::B, crate::canvas::Tool::Brush),
                (egui::Key::Y, crate::canvas::Tool::VectorBrush),
                (egui::Key::P, crate::canvas::Tool::Pen),
                (egui::Key::A, crate::canvas::Tool::Node),
                (egui::Key::R, crate::canvas::Tool::Rectangle),
                (egui::Key::K, crate::canvas::Tool::Bucket),
                (egui::Key::W, crate::canvas::Tool::Width),
            ] {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    self.activate_tool(tool);
                }
            }
        }
        if !ctx.input(|i| i.pointer.primary_down()) {
            self.style_transaction = false;
        }
        self.document_settings_ui(&ctx);
    }
    fn new_document(&mut self) {
        self.renaming_layer = None;
        self.canvas.cancel_drafts();
        self.draft = None;
        self.checkpoint();
        self.doc = Document::default();
        self.frame = 0;
        self.layer = 0;
        self.path = None;
        self.playing = false;
        self.recover();
    }
    fn insert_frame(&mut self, duplicate: bool) {
        self.checkpoint();
        for l in &mut self.doc.layers {
            let content = if duplicate {
                l.frames[self.frame].clone()
            } else {
                Default::default()
            };
            l.frames.insert(self.frame + 1, content);
        }
        self.frame += 1;
        self.playing = false;
        self.recover();
    }
    fn export(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("fotograma.svg")
            .save_file()
        {
            self.status = match std::fs::write(p, self.doc.svg(self.frame)) {
                Ok(()) => "Fotograma exportado".into(),
                Err(e) => e.to_string(),
            };
        }
    }
}

/// Include partially visible cells, clamp to content and skip offscreen work.
fn visible_range(
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
