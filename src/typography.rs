//! Editable text backed by portable vector outlines. Fonts are read only when
//! committing an edit; displaying and exporting a document never needs a font.
use crate::{Editor, Mark, document::Node, features::Gradient};
use eframe::egui::{self, Color32, Pos2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextSpec {
    pub content: String,
    pub box_size: Option<[f32; 2]>,
    pub font: String,
    pub size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub color: [u8; 4],
    pub outline: bool,
    pub outline_width: f32,
    pub outline_color: [u8; 4],
    pub align: usize,
    pub matrix: [f32; 6],
    pub gradient: Option<Gradient>,
}
impl Default for TextSpec {
    fn default() -> Self {
        Self {
            content: String::new(),
            box_size: None,
            font: String::new(),
            size: 48.,
            line_height: 1.2,
            letter_spacing: 0.,
            word_spacing: 0.,
            color: [35, 38, 48, 255],
            outline: false,
            outline_width: 2.,
            outline_color: [0, 0, 0, 255],
            align: 0,
            matrix: [1., 0., 0., 1., 100., 150.],
            gradient: None,
        }
    }
}
impl TextSpec {
    pub fn valid(&self) -> bool {
        self.box_size.is_none_or(|v| {
            v.iter()
                .all(|v| v.is_finite() && (1. ..=16384.).contains(v))
        }) && self.content.len() <= 65536
            && self.font.len() <= 4096
            && self.size.is_finite()
            && (1. ..=2048.).contains(&self.size)
            && self.line_height.is_finite()
            && (0.1..=10.).contains(&self.line_height)
            && [self.letter_spacing, self.word_spacing]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 2048.)
            && self.outline_width.is_finite()
            && (0.1..=100.).contains(&self.outline_width)
            && self.align <= 2
            && self.matrix.iter().all(|v| v.is_finite() && v.abs() <= 1e7)
            && self.gradient.as_ref().is_none_or(|g| g.valid())
    }
}
pub struct TextEditor {
    pub spec: TextSpec,
    pub editing: Option<(usize, usize, usize)>,
    pub fonts: Vec<(String, PathBuf)>,
    pub fonts_loaded: bool,
    pub box_drag: Option<Pos2>,
    pub canvas_edit: bool,
    pub focus: bool,
    pub transaction: bool,
}
impl Default for TextEditor {
    fn default() -> Self {
        Self {
            spec: Default::default(),
            editing: None,
            fonts: vec![],
            fonts_loaded: false,
            box_drag: None,
            canvas_edit: false,
            focus: false,
            transaction: false,
        }
    }
}
pub fn system_fonts() -> Vec<(String, PathBuf)> {
    let mut roots = vec![];
    if let Some(w) = std::env::var_os("WINDIR") {
        roots.push(PathBuf::from(w).join("Fonts"));
    }
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(p).join("Microsoft/Windows/Fonts"));
    }
    roots.extend([
        PathBuf::from("/usr/share/fonts/truetype/dejavu"),
        PathBuf::from("/usr/share/fonts/truetype/liberation2"),
    ]);
    let mut out = vec![];
    for root in roots {
        if let Ok(entries) = root.read_dir() {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
                {
                    let name = p
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    out.push((name, p));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}
struct Outline {
    contours: Vec<Vec<Node>>,
    current: Vec<Node>,
    x: f32,
    y: f32,
    scale: f32,
}
impl Outline {
    fn p(&self, x: f32, y: f32) -> [f32; 2] {
        [self.x + x * self.scale, self.y - y * self.scale]
    }
    fn finish(&mut self) {
        if self.current.len() > 1 {
            self.contours.push(std::mem::take(&mut self.current));
        } else {
            self.current.clear();
        }
    }
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish();
        self.current.push(Node::corner(self.p(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push(Node::corner(self.p(x, y)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let q = crate::vector::pos(self.p(x1, y1));
        let end = crate::vector::pos(self.p(x, y));
        if let Some(last) = self.current.last_mut() {
            let a = crate::vector::pos(last.position);
            last.out_handle = crate::vector::xy(a + (q - a) * 2. / 3.);
        }
        let mut n = Node::corner(crate::vector::xy(end));
        n.in_handle = crate::vector::xy(end + (q - end) * 2. / 3.);
        self.current.push(n);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let a = self.p(x1, y1);
        let b = self.p(x2, y2);
        let end = self.p(x, y);
        if let Some(last) = self.current.last_mut() {
            last.out_handle = a;
        }
        let mut n = Node::corner(end);
        n.in_handle = b;
        self.current.push(n);
    }
    fn close(&mut self) {
        if self.current.len() > 1
            && self.current.first().unwrap().position == self.current.last().unwrap().position
        {
            let last = self.current.pop().unwrap();
            self.current[0].in_handle = last.in_handle;
        }
        self.finish();
    }
}
pub fn geometry(spec: &TextSpec) -> Result<Mark, String> {
    if !spec.valid() {
        return Err("Propiedades de texto fuera de rango".into());
    }
    if spec.content.trim().is_empty() && spec.box_size.is_none() {
        return Err("Escribe un texto antes de insertarlo".into());
    }
    let bytes=std::fs::read(&spec.font).map_err(|_|"Selecciona una fuente instalada (TTF/OTF). El texto ya guardado conserva su dibujo sin la fuente.".to_string())?;
    let face =
        ttf_parser::Face::parse(&bytes, 0).map_err(|e| format!("Fuente no compatible: {e:?}"))?;
    let scale = spec.size / face.units_per_em() as f32;
    let advance = |c: char| -> Result<f32, String> {
        let id = face
            .glyph_index(c)
            .ok_or_else(|| format!("La fuente no contiene el carácter «{c}»"))?;
        Ok(face.glyph_hor_advance(id).unwrap_or(0) as f32 * scale
            + spec.letter_spacing
            + if c == ' ' { spec.word_spacing } else { 0. })
    };
    let mut builder = Outline {
        contours: vec![],
        current: vec![],
        x: 0.,
        y: 0.,
        scale,
    };
    let mut lines = vec![];
    for paragraph in spec.content.split('\n') {
        let mut line = String::new();
        let mut width = 0.;
        for word in paragraph.split_inclusive(char::is_whitespace) {
            let word_width = word
                .chars()
                .map(advance)
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .sum::<f32>();
            if !line.is_empty() && spec.box_size.is_some_and(|b| width + word_width > b[0]) {
                lines.push(std::mem::take(&mut line));
                width = 0.;
            }
            for c in word.chars() {
                let w = advance(c)?;
                if !line.is_empty() && spec.box_size.is_some_and(|b| width + w > b[0]) {
                    lines.push(std::mem::take(&mut line));
                    width = 0.;
                }
                line.push(c);
                width += w;
            }
        }
        lines.push(line);
    }
    for (line_index, line) in lines.iter().enumerate() {
        let width = line
            .chars()
            .map(advance)
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum::<f32>();
        builder.x = match spec.align {
            1 => spec.box_size.map_or(-width / 2., |b| (b[0] - width) / 2.),
            2 => spec.box_size.map_or(-width, |b| b[0] - width),
            _ => 0.,
        };
        builder.y = line_index as f32 * spec.size * spec.line_height;
        for c in line.chars() {
            let glyph = face
                .glyph_index(c)
                .ok_or_else(|| format!("Carácter no disponible: {c}"))?;
            face.outline_glyph(glyph, &mut builder);
            builder.finish();
            builder.x += advance(c)?;
        }
    }
    let mut contours = builder.contours.into_iter();
    let first = contours.next().unwrap_or_default();
    let holes = contours
        .map(|nodes| {
            let m = Mark {
                nodes: nodes.into(),
                closed: true,
                ..Default::default()
            };
            crate::vector::flatten(&m, 0.04)
                .into_iter()
                .map(crate::vector::xy)
                .collect()
        })
        .collect();
    let mut mark = Mark {
        points: if first.is_empty() {
            vec![[0., 0.], spec.box_size.unwrap_or([1., 1.])].into()
        } else {
            Default::default()
        },
        nodes: first.into(),
        holes,
        closed: true,
        fill: if spec.content.trim().is_empty() {
            None
        } else {
            Some(spec.color)
        },
        stroke: spec.outline,
        width: spec.outline_width,
        color: spec.outline_color,
        ..Default::default()
    };
    mark = crate::features::affine(&mark, spec.matrix);
    mark.style.gradient = spec.gradient.clone();
    let bounds = crate::vector::control_bounds(&mark);
    if let Some(g) = mark.style.gradient.as_mut() {
        g.fit(bounds);
    }
    mark.style.text = Some(Box::new(spec.clone()));
    Ok(mark)
}
impl Editor {
    pub fn sync_text_styles(&mut self) {
        for layer in &mut self.doc.layers {
            let Some(frame) = layer.frames.get(self.frame) else {
                continue;
            };
            let changed = frame.iter().any(|m| {
                m.style.text.as_ref().is_some_and(|s| {
                    s.color != m.fill.unwrap_or(s.color)
                        || s.outline != m.stroke
                        || s.outline_width != m.width
                        || s.outline_color != m.color
                        || s.gradient != m.style.gradient
                })
            });
            if !changed {
                continue;
            }
            for mark in &mut layer.frames[self.frame] {
                if let Some(text) = mark.style.text.as_mut() {
                    text.color = mark.fill.unwrap_or(text.color);
                    text.outline = mark.stroke;
                    text.outline_width = mark.width;
                    text.outline_color = mark.color;
                    text.gradient = mark.style.gradient.clone();
                }
            }
        }
    }
    pub fn text_at(&mut self, p: Pos2) {
        self.drawing.text.transaction = false;
        self.drawing.text.canvas_edit = true;
        self.drawing.text.focus = true;
        let selected = self
            .doc
            .layers
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| l.visible && !l.options.locked)
            .find_map(|(l, layer)| {
                layer.frames[self.frame]
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(m, mark)| {
                        mark.style
                            .text
                            .as_ref()
                            .filter(|s| {
                                text_box_bounds(s).is_some_and(|r| r.contains(p))
                                    || crate::vector::contains_fill(mark, p)
                            })
                            .map(|s| ((l, m, self.frame), (**s).clone()))
                    })
            });
        if let Some((id, spec)) = selected {
            self.drawing.text.spec = spec;
            self.drawing.text.editing = Some(id);
            self.canvas.selected = vec![(id.0, id.1)];
            self.layer = id.0;
        } else {
            self.drawing.text.editing = None;
            self.drawing.text.spec.content.clear();
            self.drawing.text.spec.box_size = None;
            self.drawing.text.spec.matrix = [1., 0., 0., 1., p.x, p.y];
            self.canvas.clear_selection();
        }
    }
    pub fn commit_text(&mut self) {
        let edit = self.drawing.text.editing.filter(|&(l, m, f)| {
            f == self.frame
                && self
                    .doc
                    .layers
                    .get(l)
                    .and_then(|l| l.frames.get(f))
                    .and_then(|f| f.get(m))
                    .is_some_and(|m| m.style.text.is_some())
        });
        if self.drawing.text.editing.is_some() && edit.is_none() {
            return;
        }
        let layer = edit.map_or(self.layer, |id| id.0);
        if self.doc.layers[layer].options.locked
            || !self.doc.layers[layer].visible
            || self.doc.layers[layer].options.mannequin.is_some()
        {
            return;
        }
        match geometry(&self.drawing.text.spec) {
            Ok(mark) => {
                if !self.drawing.text.transaction {
                    self.checkpoint();
                    self.drawing.text.transaction = true;
                }
                let (l, m) = if let Some((l, m, _)) = edit {
                    self.materialize_motion(&[(l, m)]);
                    self.doc.layers[l].frames[self.frame][m] = mark;
                    (l, m)
                } else {
                    let l = self.layer;
                    self.materialize_motion(&[(l, 0)]);
                    let m = self.doc.layers[l].frames[self.frame].len();
                    self.doc.layers[l].frames[self.frame].push(mark);
                    (l, m)
                };
                self.canvas.selected = vec![(l, m)];
                self.drawing.text.editing = Some((l, m, self.frame));
                self.recover();
            }
            Err(e) => self.status = e,
        }
    }
    pub fn ensure_text_fonts(&mut self) {
        if !self.drawing.text.fonts_loaded {
            self.drawing.text.fonts = system_fonts();
            self.drawing.text.fonts_loaded = true;
            if self.drawing.text.spec.font.is_empty() {
                if let Some((_, p)) = self
                    .drawing
                    .text
                    .fonts
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case("arial"))
                    .or(self.drawing.text.fonts.first())
                {
                    self.drawing.text.spec.font = p.to_string_lossy().into_owned();
                }
            }
        }
    }
    pub fn text_properties(&mut self, ui: &mut egui::Ui) {
        self.ensure_text_fonts();
        let original = self.drawing.text.spec.clone();
        crate::design::category(ui, "CONTENIDO");
        ui.add(
            egui::TextEdit::multiline(&mut self.drawing.text.spec.content)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .char_limit(16384),
        );
        if let Some(size) = self.drawing.text.spec.box_size.as_mut() {
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("Ancho"));
                ui.add(egui::DragValue::new(&mut size[0]).range(1. ..=16384.));
                ui.label(crate::i18n::tr("Alto"));
                ui.add(egui::DragValue::new(&mut size[1]).range(1. ..=16384.));
            });
        }
        crate::design::category(ui, "TIPOGRAFÍA");
        let spec = &mut self.drawing.text.spec;
        let name = PathBuf::from(&spec.font)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        egui::ComboBox::from_id_salt("text-font")
            .selected_text(name)
            .width(230.)
            .show_ui(ui, |ui| {
                for (name, p) in &self.drawing.text.fonts {
                    ui.selectable_value(&mut spec.font, p.to_string_lossy().into_owned(), name);
                }
            });
        if ui
            .small_button(crate::i18n::tr("＋ Cargar fuente TTF / OTF"))
            .clicked()
        {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Fuentes", &["ttf", "otf"])
                .pick_file()
            {
                spec.font = p.to_string_lossy().into_owned();
            }
        }
        egui::Grid::new("text-metrics")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(crate::i18n::tr("Tamaño"));
                ui.add(
                    egui::DragValue::new(&mut spec.size)
                        .range(1. ..=2048.)
                        .suffix(crate::i18n::tr(" px")),
                );
                ui.end_row();
                ui.label(crate::i18n::tr("Interlineado"));
                ui.add(
                    egui::DragValue::new(&mut spec.line_height)
                        .range(0.1..=10.)
                        .speed(0.05)
                        .suffix(crate::i18n::tr(" ×")),
                );
                ui.end_row();
                ui.label(crate::i18n::tr("Caracteres"));
                ui.add(
                    egui::DragValue::new(&mut spec.letter_spacing)
                        .range(-100. ..=500.)
                        .suffix(crate::i18n::tr(" px")),
                );
                ui.end_row();
                ui.label(crate::i18n::tr("Palabras"));
                ui.add(
                    egui::DragValue::new(&mut spec.word_spacing)
                        .range(-100. ..=500.)
                        .suffix(crate::i18n::tr(" px")),
                );
                ui.end_row();
            });
        ui.horizontal(|ui| {
            for (i, label) in ["Izquierda", "Centro", "Derecha"].into_iter().enumerate() {
                ui.selectable_value(&mut spec.align, i, crate::i18n::tr(label));
            }
        });
        crate::design::category(ui, "RELLENO");
        edit_color(ui, &mut spec.color);
        let mut gradient = spec.gradient.is_some();
        if ui
            .checkbox(&mut gradient, crate::i18n::tr("Degradado"))
            .changed()
        {
            spec.gradient = if gradient {
                Some(self.feature.gradient.clone())
            } else {
                None
            };
        }
        if let Some(g) = spec.gradient.clone() {
            self.feature.gradient = g;
            if self.visual_gradient_ui(ui) {
                self.drawing.text.spec.gradient = Some(self.feature.gradient.clone());
            }
        }
        let spec = &mut self.drawing.text.spec;
        crate::design::category(ui, "CONTORNO");
        ui.checkbox(&mut spec.outline, crate::i18n::tr("Outline"));
        if spec.outline {
            ui.add(
                egui::Slider::new(&mut spec.outline_width, 0.1..=100.)
                    .text(crate::i18n::tr("Grosor"))
                    .suffix(crate::i18n::tr(" px")),
            );
            edit_color(ui, &mut spec.outline_color);
        }
        if original != self.drawing.text.spec && self.drawing.text.editing.is_some() {
            self.commit_text();
        }
        ui.small(crate::i18n::tr("Arrastra para crear un cuadro. Escribe en el lienzo; las propiedades se aplican al instante."));
    }
}
fn edit_color(ui: &mut egui::Ui, rgba: &mut [u8; 4]) {
    let mut c = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    if ui.color_edit_button_srgba(&mut c).changed() {
        *rgba = c.to_srgba_unmultiplied();
    }
}

pub fn text_box_bounds(spec: &TextSpec) -> Option<egui::Rect> {
    let size = spec.box_size?;
    let mut bounds = egui::Rect::NOTHING;
    for p in [
        Pos2::new(0., -spec.size),
        Pos2::new(size[0], -spec.size),
        Pos2::new(size[0], size[1] - spec.size),
        Pos2::new(0., size[1] - spec.size),
    ] {
        bounds.extend_with(crate::rig::point(spec.matrix, p));
    }
    Some(bounds)
}
impl Editor {
    pub fn text_canvas_ui(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        view: crate::canvas::View,
    ) {
        if self.tool != crate::canvas::Tool::Text || !self.drawing.text.canvas_edit {
            return;
        }
        if let Some(start) = self.drawing.text.box_drag {
            let end = ui
                .input(|i| i.pointer.interact_pos())
                .map(|p| view.world(p))
                .unwrap_or(start);
            let rect = egui::Rect::from_two_pos(start, end);
            painter.rect_stroke(
                egui::Rect::from_two_pos(view.screen(rect.min), view.screen(rect.max)),
                0.,
                egui::Stroke::new(1., crate::themes::accent()),
                egui::StrokeKind::Inside,
            );
            if ui.input(|i| i.pointer.primary_down()) {
                return;
            }
            self.drawing.text.box_drag = None;
            let size = if rect.width() > 12. && rect.height() > 12. {
                rect.size()
            } else {
                egui::vec2(320., 140.)
            };
            let top = if rect.width() > 12. { rect.min } else { start };
            self.drawing.text.spec.box_size = Some([size.x.max(20.), size.y.max(20.)]);
            self.drawing.text.spec.matrix =
                [1., 0., 0., 1., top.x, top.y + self.drawing.text.spec.size];
            self.ensure_text_fonts();
            self.commit_text();
            self.drawing.text.focus = true;
        }
        if self
            .drawing
            .text
            .editing
            .is_some_and(|(_, _, f)| f != self.frame)
        {
            self.drawing.text.canvas_edit = false;
            return;
        }
        let Some(bounds) = text_box_bounds(&self.drawing.text.spec).or_else(|| {
            self.drawing.text.editing.and_then(|(l, m, f)| {
                self.doc
                    .layers
                    .get(l)?
                    .frames
                    .get(f)?
                    .get(m)
                    .map(crate::vector::control_bounds)
            })
        }) else {
            return;
        };
        let rect = egui::Rect::from_two_pos(view.screen(bounds.min), view.screen(bounds.max));
        painter.rect_stroke(
            rect,
            0.,
            egui::Stroke::new(1., crate::themes::accent()),
            egui::StrokeKind::Inside,
        );
        let family = egui::FontFamily::Name("canvas-text-font".into());
        let mut definitions = ui.ctx().fonts(|f| f.definitions().clone());
        let font_path = self.drawing.text.spec.font.clone();
        let ready = definitions
            .families
            .get(&family)
            .is_some_and(|v| v.first() == Some(&font_path));
        if !ready {
            if let Ok(bytes) = std::fs::read(&font_path) {
                if let Some(old) = definitions.families.get(&family).and_then(|v| v.first()) {
                    definitions.font_data.remove(&old.clone());
                }
                definitions.font_data.insert(
                    font_path.clone(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                definitions.families.insert(family.clone(), vec![font_path]);
                ui.ctx().set_fonts(definitions);
            }
        }
        let family = if ready {
            family
        } else {
            egui::FontFamily::Proportional
        };
        let align = match self.drawing.text.spec.align {
            1 => egui::Align::Center,
            2 => egui::Align::Max,
            _ => egui::Align::Min,
        };
        let size = (self.drawing.text.spec.size * view.scale).clamp(5., 200.);
        let font_id = egui::FontId::new(size, family);
        let line_height = size * self.drawing.text.spec.line_height;
        let letter_spacing = self.drawing.text.spec.letter_spacing * view.scale;
        let word_spacing = self.drawing.text.spec.word_spacing * view.scale;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = width;
            job.halign = align;
            for c in text.as_str().chars() {
                job.append(
                    &c.to_string(),
                    0.,
                    egui::TextFormat {
                        font_id: font_id.clone(),
                        color: Color32::TRANSPARENT,
                        extra_letter_spacing: letter_spacing
                            + if c == ' ' { word_spacing } else { 0. },
                        line_height: Some(line_height),
                        ..Default::default()
                    },
                );
            }
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let response = ui.put(
            rect,
            egui::TextEdit::multiline(&mut self.drawing.text.spec.content)
                .id(egui::Id::unique(("canvas-text", self.drawing.text.editing)))
                .font(font_id.clone())
                .layouter(&mut layouter)
                .align(egui::Align2([align, egui::Align::Min]))
                .text_color(Color32::TRANSPARENT)
                .frame(egui::Frame::NONE)
                .margin(0.)
                .desired_width(rect.width())
                .char_limit(16384),
        );
        if self.drawing.text.focus {
            response.request_focus();
            self.drawing.text.focus = false;
        }
        if response.changed() {
            self.commit_text();
            ui.ctx().request_repaint();
        }
    }
}
