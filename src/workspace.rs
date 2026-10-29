use crate::design::{ACCENT, Icon, MUTED, PANEL, button, draw, section};
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
            .exact_size(49.)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(7.);
                for (tool, tip) in [
                    (crate::canvas::Tool::Select, "Seleccionar · V"),
                    (crate::canvas::Tool::Transform, "Transformación libre · Q"),
                    (crate::canvas::Tool::Brush, "Pincel · B"),
                    (crate::canvas::Tool::Rectangle, "Rectángulo · R"),
                ] {
                    if button(ui, tool.icon(), "", tip, self.tool == tool, true).clicked() {
                        self.tool = tool;
                    }
                }
                ui.add_space(6.);
                ui.separator();
                ui.add_space(5.);
                ui.color_edit_button_srgba(&mut self.color)
                    .on_hover_text("Color del trazo");
                ui.separator();
                if button(
                    ui,
                    Icon::Trash,
                    "",
                    "Vaciar dibujo de esta capa y fotograma",
                    false,
                    true,
                )
                .clicked()
                {
                    self.checkpoint();
                    self.doc.layers[self.layer].frames[self.frame].clear();
                    self.recover();
                }
            });
        egui::Panel::right("properties").exact_size(248.).resizable(true).show(ui,|ui|{
            ui.horizontal(|ui|{draw(ui.painter(),Rect::from_min_size(ui.cursor().min,Vec2::splat(20.)),Icon::Settings,MUTED);ui.add_space(25.);ui.label(egui::RichText::new("Propiedades").strong());});
            ui.separator();
            egui::ScrollArea::vertical().show(ui,|ui|{
                section(ui,"HERRAMIENTA");
                ui.horizontal(|ui|{draw(ui.painter(),Rect::from_min_size(ui.cursor().min,Vec2::splat(20.)),self.tool.icon(),ACCENT);ui.add_space(27.);ui.heading(self.tool.name());});
                ui.add_space(10.);
                ui.horizontal(|ui|{ui.label("Trazo");ui.color_edit_button_srgba(&mut self.color);ui.label(egui::RichText::new(format!("#{:02X}{:02X}{:02X}",self.color.r(),self.color.g(),self.color.b())).monospace().color(MUTED));});
                ui.label("Grosor");ui.horizontal(|ui|{ui.add(egui::Slider::new(&mut self.width,1.0..=30.0).suffix(" px"));});
                if self.tool==crate::canvas::Tool::Brush {ui.add_space(8.);ui.label("Suavizado del pincel");ui.add(egui::Slider::new(&mut self.smoothing,0.0..=1.0).custom_formatter(|v,_|format!("{:.0}%",v*100.)));}
                ui.add_space(10.);ui.separator();section(ui,"DOCUMENTO");
                egui::Grid::new("dimensions").num_columns(2).spacing([18.,12.]).show(ui,|ui|{ui.label(egui::RichText::new("Escenario").color(MUTED));ui.label("960 × 540 px");ui.end_row();ui.label(egui::RichText::new("Duración").color(MUTED));ui.label(format!("{:.2} s",self.doc.layers[0].frames.len() as f32/self.doc.fps as f32));ui.end_row();ui.label(egui::RichText::new("Velocidad").color(MUTED));let mut fps=self.doc.fps;if ui.add(egui::DragValue::new(&mut fps).range(1..=60).suffix(" fps")).changed(){self.checkpoint();self.doc.fps=fps;self.recover();}ui.end_row();});
                ui.add_space(10.);ui.separator();section(ui,"CAPA ACTIVA");
                let mut name=self.doc.layers[self.layer].name.clone();if ui.text_edit_singleline(&mut name).changed(){self.checkpoint();self.doc.layers[self.layer].name=name;self.recover();}
                ui.horizontal(|ui|{
                    if button(ui,Icon::Up,"","Subir capa",false,self.layer+1<self.doc.layers.len()).clicked(){self.checkpoint();self.doc.layers.swap(self.layer,self.layer+1);self.layer+=1;self.recover();}
                    if button(ui,Icon::Down,"","Bajar capa",false,self.layer>0).clicked(){self.checkpoint();self.doc.layers.swap(self.layer,self.layer-1);self.layer-=1;self.recover();}
                    if button(ui,Icon::Trash,"","Eliminar capa",false,self.doc.layers.len()>1).clicked(){self.checkpoint();self.doc.layers.remove(self.layer);self.layer=self.layer.min(self.doc.layers.len()-1);self.recover();}
                });
                ui.add_space(10.);ui.separator();section(ui,"SELECCIÓN Y VISTA");
                ui.label(egui::RichText::new(format!("{} trazos seleccionados",self.canvas.selected.len())).color(ACCENT));
                ui.label(egui::RichText::new("V · Seleccionar   Q · Transformar\nMayús+clic · Añadir o quitar\nArrastra en vacío · Marco de selección\nTiradores · Escalar   Mayús · Proporcional\nTirador circular · Rotar   Cruz · Pivote\nRueda · Zoom al cursor\nBotón central / Espacio+arrastrar · Desplazar\nSupr · Borrar   Ctrl+D · Duplicar\nEsc · Cancelar transformación").size(11.).color(MUTED));                ui.add_space(10.);ui.separator();section(ui,"DIBUJO POR FOTOGRAMAS");
                ui.label(egui::RichText::new("Dibuja en el escenario. Duplica un fotograma para continuar la animación y activa el papel cebolla como referencia.").size(11.).color(MUTED));
            });
        });
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
                    ui.label(
                        egui::RichText::new(format!(
                            "/ {}    {} fps",
                            self.doc.layers[0].frames.len(),
                            self.doc.fps
                        ))
                        .color(MUTED),
                    );
                    ui.separator();
                    if button(ui, Icon::Onion, "", "Papel cebolla", self.onion, true).clicked() {
                        self.onion = !self.onion;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if button(ui, Icon::Layers, "", "Añadir capa", false, true).clicked() {
                        self.checkpoint();
                        let n = self.doc.layers.len();
                        self.doc.layers.push(Layer {
                            name: format!("Capa {}", n + 1),
                            visible: true,
                            frames: vec![vec![]; self.doc.layers[0].frames.len()],
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
                        let painter = ui.painter();
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
                        for f in 0..count {
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
                        for (row, l) in (0..self.doc.layers.len()).rev().enumerate() {
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
                                painter,
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
                            clipped.text(
                                Pos2::new(r.left() + 42., y + row_h / 2.),
                                egui::Align2::LEFT_CENTER,
                                &self.doc.layers[l].name,
                                egui::FontId::proportional(11.),
                                Color32::from_gray(218),
                            );
                            if ui
                                .interact(eye, ui.scope_id().with(("eye", l)), Sense::click())
                                .on_hover_text("Mostrar / ocultar capa")
                                .clicked()
                            {
                                self.checkpoint();
                                self.doc.layers[l].visible = !self.doc.layers[l].visible;
                                self.recover();
                            }
                            if ui
                                .interact(
                                    Rect::from_min_max(Pos2::new(r.left() + 30., y), name_rect.max),
                                    ui.scope_id().with(("name", l)),
                                    Sense::click(),
                                )
                                .clicked()
                            {
                                self.layer = l;
                            }
                            for f in 0..count {
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
                            egui::RichText::new(format!("{:.0}%", self.canvas.zoom * 100.))
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
        if !ctx.egui_wants_keyboard_input() && !ctx.input(|i| i.modifiers.command) {
            if ctx.input(|i| i.key_pressed(egui::Key::V)) {
                self.tool = crate::canvas::Tool::Select;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
                self.tool = crate::canvas::Tool::Transform;
            }
        }
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::B)) {
                self.tool = crate::canvas::Tool::Brush;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::R)) {
                self.tool = crate::canvas::Tool::Rectangle;
            }
        }
    }
    fn new_document(&mut self) {
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
                vec![]
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
