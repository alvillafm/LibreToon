use crate::{
    Editor,
    canvas::Tool,
    design::{self, ACCENT, Icon, MUTED},
    shared::SharedVec,
    width::{self, WidthStop},
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

pub struct DocumentSettings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}
impl Editor {
    pub fn open_document_settings(&mut self) {
        self.document_settings = Some(DocumentSettings {
            width: self.doc.width,
            height: self.doc.height,
            fps: self.doc.fps,
        });
    }
    pub fn apply_document_settings(&mut self, width: u32, height: u32, fps: u32) {
        if !(1..=16384).contains(&width)
            || !(1..=16384).contains(&height)
            || !(1..=240).contains(&fps)
        {
            return;
        }
        if (width, height, fps) == (self.doc.width, self.doc.height, self.doc.fps) {
            return;
        }
        self.finish_pending_pen();
        self.checkpoint();
        self.doc.width = width;
        self.doc.height = height;
        self.doc.fps = fps;
        self.canvas.fit();
        self.playing = false;
        self.tick = std::time::Instant::now();
        self.recover();
    }
    pub fn set_fps(&mut self, fps: u32) {
        if fps == self.doc.fps || !(1..=240).contains(&fps) {
            return;
        }
        self.checkpoint();
        self.doc.fps = fps;
        self.tick = std::time::Instant::now();
        self.recover();
    }
    pub fn set_duration(&mut self, seconds: f32) {
        if !seconds.is_finite() {
            return;
        }
        let count = ((seconds * self.doc.fps as f32).round() as usize).clamp(1, 10000);
        if count == self.doc.layers[0].frames.len() {
            return;
        }
        self.finish_pending_pen();
        self.checkpoint();
        for layer in &mut self.doc.layers {
            layer.frames.resize(count, Default::default());
        }
        self.frame = self.frame.min(count - 1);
        self.playing = false;
        self.recover();
    }
    pub fn document_settings_ui(&mut self, ctx: &egui::Context) {
        let Some(mut settings) = self.document_settings.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("Ajustes de documento")
            .id(egui::Id::unique("document-settings"))
            .open(&mut open)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .resizable(false)
            .show(ctx, |ui| {
                let presets = [
                    ("HD · 1280 × 720", 1280, 720),
                    ("Full HD · 1920 × 1080", 1920, 1080),
                    ("4K UHD · 3840 × 2160", 3840, 2160),
                    ("4K DCI · 4096 × 2160", 4096, 2160),
                    ("Vertical · 1080 × 1920", 1080, 1920),
                    ("Cuadrado · 1080 × 1080", 1080, 1080),
                ];
                let label = presets
                    .iter()
                    .find(|(_, w, h)| *w == settings.width && *h == settings.height)
                    .map_or("Personalizada", |(label, _, _)| *label);
                egui::ComboBox::from_id_salt("resolution-preset")
                    .selected_text(label)
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for (label, w, h) in presets {
                            if ui
                                .selectable_label(
                                    settings.width == w && settings.height == h,
                                    label,
                                )
                                .clicked()
                            {
                                settings.width = w;
                                settings.height = h;
                            }
                        }
                    });
                ui.add_space(10.);
                egui::Grid::new("document-size")
                    .spacing([18., 10.])
                    .show(ui, |ui| {
                        ui.label("Ancho");
                        ui.add(
                            egui::DragValue::new(&mut settings.width)
                                .range(1..=16384)
                                .suffix(" px")
                                .update_while_editing(false),
                        );
                        ui.end_row();
                        ui.label("Alto");
                        ui.add(
                            egui::DragValue::new(&mut settings.height)
                                .range(1..=16384)
                                .suffix(" px")
                                .update_while_editing(false),
                        );
                        ui.end_row();
                        ui.label("Velocidad");
                        ui.add(
                            egui::DragValue::new(&mut settings.fps)
                                .range(1..=240)
                                .suffix(" fps")
                                .update_while_editing(false),
                        );
                        ui.end_row();
                    });
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    if ui.button("Aplicar").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        cancel = true;
                    }
                });
            });
        if apply {
            self.apply_document_settings(settings.width, settings.height, settings.fps);
        } else if open && !cancel {
            self.document_settings = Some(settings);
        }
    }
    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.color, &mut self.fill_color);
        std::mem::swap(&mut self.stroke_enabled, &mut self.fill_enabled);
    }
    pub fn color_swatches(&mut self, ui: &mut egui::Ui) {
        let old_fill = self.fill_color;
        let old_stroke = self.color;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Relleno").size(8.).color(MUTED));
                if ui.color_edit_button_srgba(&mut self.fill_color).clicked() {
                    self.active_fill = true;
                }
            });
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Contorno").size(8.).color(MUTED));
                // Hollow frame makes the stroke control distinct from the fill.
                let response = ui.color_edit_button_srgba(&mut self.color);
                ui.painter().rect_stroke(
                    response.rect.shrink(3.),
                    0.,
                    Stroke::new(2., Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
                if response.clicked() {
                    self.active_fill = false;
                }
            });
        });
        if old_fill != self.fill_color {
            self.fill_enabled = true;
            self.change_selected_style(true, false, false, false);
        }
        if old_stroke != self.color {
            self.stroke_enabled = true;
            self.change_selected_style(false, true, false, false);
        }
        ui.horizontal(|ui| {
            if ui
                .small_button("⇄")
                .on_hover_text("Intercambiar colores · X")
                .clicked()
            {
                self.swap_colors();
                self.change_selected_style(true, true, false, false);
            }
            if ui
                .small_button("◩")
                .on_hover_text("Blanco y negro · D")
                .clicked()
            {
                self.color = Color32::BLACK;
                self.fill_color = Color32::WHITE;
                self.stroke_enabled = true;
                self.fill_enabled = true;
                self.change_selected_style(true, true, false, false);
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fill_enabled, egui::RichText::new("R").size(9.))
                .on_hover_text("Dibujar relleno");
            ui.checkbox(&mut self.stroke_enabled, egui::RichText::new("C").size(9.))
                .on_hover_text("Dibujar contorno");
        });
    }
    pub fn properties_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            design::draw(
                ui.painter(),
                Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                Icon::Settings,
                MUTED,
            );
            ui.add_space(25.);
            ui.strong("Propiedades");
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.);
            self.tool_options_ui(ui);
            if !self.canvas.selected.is_empty()
                && matches!(
                    self.tool,
                    Tool::Select
                        | Tool::Transform
                        | Tool::Object
                        | Tool::Lasso
                        | Tool::PolygonSelect
                        | Tool::Wand
                        | Tool::Width
                        | Tool::Node
                )
            {
                self.selection_properties_ui(ui);
                return;
            }
            ui.horizontal(|ui| {
                design::draw(
                    ui.painter(),
                    Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                    self.tool.icon(),
                    ACCENT,
                );
                ui.add_space(27.);
                ui.heading(self.tool.name());
            });
            ui.add_space(14.);
            if self.tool == Tool::Width {
                if let Some(&(l, m)) = self.canvas.selected.first() {
                    let mark = &self.doc.layers[l].frames[self.frame][m];
                    if mark.stroke {
                        if self.width_profile != mark.width_profile {
                            self.width_profile = mark.width_profile.clone();
                            self.width_preset = width::Preset::Custom;
                        }
                        self.width = mark.width;
                    }
                }
            }
            if matches!(
                self.tool,
                Tool::Rectangle | Tool::Pen | Tool::Ellipse | Tool::Polygon
            ) {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.fill_enabled, "Relleno");
                    ui.color_edit_button_srgba(&mut self.fill_color);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.stroke_enabled, "Contorno");
                    ui.color_edit_button_srgba(&mut self.color);
                });
                ui.add_space(8.);
            }
            if matches!(
                self.tool,
                Tool::Brush
                    | Tool::VectorBrush
                    | Tool::Rectangle
                    | Tool::Pen
                    | Tool::Width
                    | Tool::Line
                    | Tool::Ellipse
                    | Tool::Polygon
                    | Tool::Eraser
            ) {
                ui.label("Grosor");
                let response = ui.add(egui::Slider::new(&mut self.width, 1.0..=80.).suffix(" px"));
                if response.changed() && self.tool == Tool::Width {
                    let ids = self.canvas.selected.clone();
                    self.checkpoint();
                    for &(l, m) in &ids {
                        self.doc.layers[l].frames[self.frame][m].width = self.width;
                    }
                    self.canvas.selected = ids;
                    self.recover();
                }
            }
            if matches!(self.tool, Tool::Brush | Tool::VectorBrush) {
                ui.add_space(10.);
                ui.label("Suavizado");
                ui.add(
                    egui::Slider::new(&mut self.smoothing, 0.0..=1.)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.)),
                );
            }
            if self.tool == Tool::Bucket {
                ui.label("Cierre de huecos");
                egui::ComboBox::from_id_salt("bucket-gap")
                    .selected_text(self.gap.label())
                    .show_ui(ui, |ui| {
                        for gap in crate::bucket::Gap::ALL {
                            ui.selectable_value(&mut self.gap, gap, gap.label());
                        }
                    });
            }
            if matches!(
                self.tool,
                Tool::VectorBrush
                    | Tool::Pen
                    | Tool::Width
                    | Tool::Line
                    | Tool::Ellipse
                    | Tool::Polygon
            ) {
                ui.add_space(12.);
                self.width_profile_ui(ui);
            }
            if !self.canvas.selected.is_empty() && self.tool != Tool::Bucket {
                ui.add_space(12.);
                if ui
                    .add_enabled(
                        self.fill_enabled || self.stroke_enabled,
                        egui::Button::new("Aplicar estilo a la selección"),
                    )
                    .clicked()
                {
                    self.apply_drawing_style();
                }
            }
        });
    }
    pub(crate) fn width_profile_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Perfil del contorno");
        let previous = self.width_preset;
        let mut changed = false;
        let mut dragging = false;
        egui::ComboBox::from_id_salt("width-preset")
            .selected_text(self.width_preset.label())
            .show_ui(ui, |ui| {
                for preset in width::Preset::ALL {
                    ui.selectable_value(&mut self.width_preset, preset, preset.label());
                }
            });
        if previous != self.width_preset {
            self.width_profile = self.width_preset.stops();
            self.profile_point = None;
            changed = true;
        }
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 112.),
            Sense::click_and_drag(),
        );
        let graph = rect.shrink2(Vec2::new(8., 10.));
        let painter = ui.painter();
        painter.rect_filled(rect, 3., Color32::from_rgb(25, 26, 30));
        for i in 0..=4 {
            let y = graph.bottom() - graph.height() * i as f32 / 4.;
            painter.line_segment(
                [Pos2::new(graph.left(), y), Pos2::new(graph.right(), y)],
                Stroke::new(0.5, Color32::from_gray(60)),
            );
        }
        let screen = |stop: WidthStop| {
            Pos2::new(
                graph.left() + stop.t * graph.width(),
                graph.bottom() - stop.width * graph.height() / 4.,
            )
        };
        let points: Vec<_> = (0..=96)
            .map(|i| {
                let t = i as f32 / 96.;
                screen(WidthStop {
                    t,
                    width: width::sample(&self.width_profile, t),
                })
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(2., ACCENT)));
        for (i, stop) in self.width_profile.iter().enumerate() {
            painter.circle_filled(
                screen(*stop),
                if self.profile_point == Some(i) {
                    4.5
                } else {
                    3.5
                },
                if self.profile_point == Some(i) {
                    Color32::WHITE
                } else {
                    ACCENT
                },
            );
        }
        if response.drag_started() || response.clicked() {
            let point = ui
                .input(|i| i.pointer.press_origin())
                .or(response.interact_pointer_pos());
            if let Some(point) = point {
                let closest = self
                    .width_profile
                    .iter()
                    .enumerate()
                    .find(|(_, s)| screen(**s).distance(point) < 10.)
                    .map(|(i, _)| i);
                if let Some(index) = closest {
                    self.profile_point = Some(index);
                } else {
                    let t = ((point.x - graph.left()) / graph.width()).clamp(0., 1.);
                    let w = ((graph.bottom() - point.y) / graph.height() * 4.).clamp(0., 4.);
                    self.profile_point = Some(width::edit_stop(&mut self.width_profile, t, w));
                    changed = true;
                    self.width_preset = width::Preset::Custom;
                }
            }
        }
        if response.dragged() {
            dragging = true;
            if let (Some(index), Some(point)) =
                (self.profile_point, response.interact_pointer_pos())
            {
                let n = self.width_profile.len();
                if index < n {
                    let t = ((point.x - graph.left()) / graph.width()).clamp(0., 1.);
                    let value = ((graph.bottom() - point.y) / graph.height() * 4.).clamp(0., 4.);
                    let min = if index > 0 {
                        self.width_profile[index - 1].t + 0.001
                    } else {
                        0.
                    };
                    let max = if index + 1 < n {
                        self.width_profile[index + 1].t - 0.001
                    } else {
                        1.
                    };
                    self.width_profile[index].t = if index == 0 {
                        0.
                    } else if index + 1 == n {
                        1.
                    } else {
                        t.clamp(min, max)
                    };
                    self.width_profile[index].width = value;
                    changed = true;
                    self.width_preset = width::Preset::Custom;
                }
            }
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Inicio").size(10.).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Final").size(10.).color(MUTED));
            });
        });
        if let Some(index) = self.profile_point.filter(|i| *i < self.width_profile.len()) {
            let mut value = self.width_profile[index].width * 100.;
            let r = ui.add(
                egui::DragValue::new(&mut value)
                    .range(0.0..=400.)
                    .suffix(" %")
                    .prefix("Grosor: ")
                    .update_while_editing(false),
            );
            dragging |= r.dragged();
            if r.changed() {
                self.width_profile[index].width = value / 100.;
                changed = true;
                self.width_preset = width::Preset::Custom;
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.width_profile.len() < 128, egui::Button::new("+ Punto"))
                .clicked()
            {
                if self.width_profile.is_empty() {
                    self.width_profile = width::Preset::Custom.stops();
                }
                let pair = self
                    .width_profile
                    .windows(2)
                    .max_by(|a, b| (a[1].t - a[0].t).total_cmp(&(b[1].t - b[0].t)))
                    .unwrap();
                let t = (pair[0].t + pair[1].t) * 0.5;
                let value = width::sample(&self.width_profile, t);
                self.profile_point = Some(width::edit_stop(&mut self.width_profile, t, value));
                changed = true;
                self.width_preset = width::Preset::Custom;
            }
            let removable = self
                .profile_point
                .is_some_and(|i| i > 0 && i + 1 < self.width_profile.len());
            if ui
                .add_enabled(removable, egui::Button::new("− Punto"))
                .clicked()
            {
                self.width_profile
                    .remove(self.profile_point.take().unwrap());
                changed = true;
                self.width_preset = width::Preset::Custom;
            }
        });
        if self.tool == Tool::Width || !self.canvas.selected.is_empty() {
            let ids = self.canvas.selected.clone();
            if changed && !ids.is_empty() {
                if !self.profile_transaction {
                    self.checkpoint();
                    self.profile_transaction = true;
                }
                for &(l, m) in &ids {
                    if self.doc.layers[l].frames[self.frame][m].stroke {
                        self.doc.layers[l].frames[self.frame][m].width_profile =
                            self.width_profile.clone();
                    }
                }
                self.canvas.selected = ids;
            }
            if self.profile_transaction && !dragging {
                self.profile_transaction = false;
                self.recover();
            }
        } else if ui
            .add_enabled(
                !self.canvas.selected.is_empty(),
                egui::Button::new("Aplicar perfil a la selección"),
            )
            .clicked()
        {
            self.apply_width_profile(self.width_profile.clone());
        }
    }
    pub fn apply_width_profile(&mut self, profile: SharedVec<WidthStop>) {
        let ids = self.canvas.selected.clone();
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for &(l, m) in &ids {
            if self.doc.layers[l].frames[self.frame][m].stroke {
                self.doc.layers[l].frames[self.frame][m].width_profile = profile.clone();
            }
        }
        self.canvas.selected = ids;
        self.recover();
    }
}

#[cfg(test)]
mod ux_tests {
    use super::*;
    fn run(test: impl FnOnce(&mut Editor)) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut editor = Editor::new(&cc);
        editor.doc = crate::Document::default();
        let path = std::env::temp_dir().join(format!(
            "libretoon-ux-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        editor.recovery = path.clone();
        test(&mut editor);
        drop(editor);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn document_settings_preserve_art_and_undo_dimensions_and_fps() {
        run(|e| {
            e.doc.layers[0].frames[0].push(crate::document::Mark {
                points: vec![[22., 33.]].into(),
                ..Default::default()
            });
            e.apply_document_settings(3840, 2160, 120);
            assert_eq!((e.doc.width, e.doc.height, e.doc.fps), (3840, 2160, 120));
            assert_eq!(e.doc.layers[0].frames[0][0].points[0], [22., 33.]);
            e.history(false);
            assert_eq!((e.doc.width, e.doc.height, e.doc.fps), (960, 540, 12));
            assert!(e.doc.validate().is_ok());
            e.apply_document_settings(0, 2160, 120);
            assert_eq!(e.doc.width, 960);
        });
    }
    #[test]
    fn timeline_duration_resizes_all_layers_and_undo_recovers_trimmed_art() {
        run(|e| {
            e.doc.layers[0].frames[23].push(crate::document::Mark {
                points: vec![[2., 3.]].into(),
                ..Default::default()
            });
            let layer = e.doc.layers[0].clone();
            e.doc.layers.push(layer);
            e.set_duration(4.);
            assert!(e.doc.layers.iter().all(|l| l.frames.len() == 48));
            e.set_duration(1.);
            assert!(e.doc.layers.iter().all(|l| l.frames.len() == 12));
            e.history(false);
            assert_eq!(e.doc.layers[0].frames[23][0].points[0], [2., 3.]);
            assert!(e.doc.validate().is_ok());
        });
    }
    #[test]
    fn color_swap_exchanges_both_colors_and_none_states() {
        run(|e| {
            e.fill_color = Color32::RED;
            e.color = Color32::BLUE;
            e.fill_enabled = true;
            e.stroke_enabled = false;
            e.swap_colors();
            assert_eq!(e.fill_color, Color32::BLUE);
            assert_eq!(e.color, Color32::RED);
            assert!(!e.fill_enabled && e.stroke_enabled);
            e.swap_colors();
            assert_eq!(e.fill_color, Color32::RED);
        });
    }
}
