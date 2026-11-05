//! Camera-space layer planes and bounded, premultiplied layer compositing.
use crate::{
    Document, Editor, Mark,
    canvas::{Tool, View},
    design::{self, Icon},
    i18n::tr,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};
const REFERENCE: f32 = 1000.;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerDepth {
    pub distance: f32,
    pub keep_size: bool,
}
impl Default for LayerDepth {
    fn default() -> Self {
        Self {
            distance: REFERENCE,
            keep_size: true,
        }
    }
}
impl LayerDepth {
    pub fn valid(&self) -> bool {
        self.distance.is_finite() && (1. ..=1e6).contains(&self.distance)
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlurMethod {
    #[default]
    Gaussian,
    Lens,
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Optics {
    pub position: f32,
    pub method: BlurMethod,
    pub strength: f32,
    pub quality: Quality,
    pub focus: f32,
    pub range: f32,
}
impl Default for Optics {
    fn default() -> Self {
        Self {
            position: 0.,
            method: BlurMethod::Gaussian,
            strength: 0.,
            quality: Quality::Medium,
            focus: REFERENCE,
            range: 100.,
        }
    }
}
impl Optics {
    pub fn interpolate(&self, to: &Self, t: f32) -> Self {
        let t = t.clamp(0., 1.);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        // Discrete settings belong to the preceding key, even if easing reaches 1 early.
        let discrete = self;
        Self {
            position: lerp(self.position, to.position),
            focus: lerp(self.focus, to.focus),
            range: lerp(self.range, to.range),
            strength: lerp(self.strength, to.strength),
            method: discrete.method,
            quality: discrete.quality,
        }
    }
    pub fn valid(&self) -> bool {
        self.position.is_finite()
            && self.position.abs() <= 1e6
            && self.strength.is_finite()
            && (0. ..=100.).contains(&self.strength)
            && self.focus.is_finite()
            && (0. ..=2e6).contains(&self.focus)
            && self.range.is_finite()
            && (0. ..=2e6).contains(&self.range)
    }
    pub fn radius(&self, distance: f32) -> f32 {
        let t = ((distance - self.focus).abs() - self.range).max(0.) / self.range.max(100.);
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t) * self.strength * 0.32
    }
}
pub fn camera(doc: &Document, frame: usize) -> Option<&crate::features::Camera> {
    doc.extras
        .cameras
        .iter()
        .find(|c| frame >= c.start && frame < c.start + c.duration)
}
fn factors(layer: &LayerDepth, optics: &Optics) -> (f32, f32) {
    let distance = (layer.distance - optics.position).max(1.);
    (
        if layer.keep_size {
            layer.distance / distance
        } else {
            REFERENCE / distance
        },
        REFERENCE / distance,
    )
}
pub fn project_mark(
    doc: &Document,
    pose: &crate::animation::CameraKey,
    optics: &Optics,
    layer: &LayerDepth,
    mark: &Mark,
) -> Mark {
    let (size, motion) = factors(layer, optics);
    let center = Pos2::new(doc.width as f32 / 2., doc.height as f32 / 2.);
    let offset = (crate::vector::pos(pose.center) - center) * (1. - motion);
    if size == 1. && offset == Vec2::ZERO {
        return mark.clone();
    }
    let mut result = crate::vector::transform(mark, |p| center + (p - center) * size + offset);
    result.width *= size;
    result
}
pub fn layer_view(doc: &Document, frame: usize, layer: usize, base: View) -> View {
    let Some(layer) = doc.layers.get(layer) else {
        return base;
    };
    view_for_plane(doc, frame, &layer.options.depth, base)
}
pub fn view_for_plane(doc: &Document, frame: usize, plane: &LayerDepth, base: View) -> View {
    let Some(c) = camera(doc, frame) else {
        return base;
    };
    let pose = crate::animation::camera_at(c, frame);
    let (size, motion) = factors(plane, &pose.optics(c));
    let center = Pos2::new(doc.width as f32 / 2., doc.height as f32 / 2.);
    View {
        scale: base.scale * size,
        stage_center: center + (crate::vector::pos(pose.center) - center) * (motion / size),
        ..base
    }
}
pub fn active(editor: &Editor) -> bool {
    editor.playing || editor.studio.stage_only || editor.tool == Tool::Camera
}
fn label(ui: &mut egui::Ui, icon: Icon, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(18.), egui::Sense::hover());
        design::draw(ui.painter(), r, icon, crate::themes::accent());
        ui.label(tr(text));
    });
}
const HELP: &str = "Las distancias se miden desde la cámara. Foco 200 y amplitud 100 mantienen nítido de 100 a 300; fuera de ese intervalo el desenfoque aumenta suavemente. Mantener tamaño conserva el tamaño inicial de las capas al cambiar su profundidad. Las capas cercanas se desplazan más al mover la cámara. Ocultar el desenfoque solo afecta al editor, nunca a la exportación.";
fn help(ui: &mut egui::Ui) {
    let id = ui.scope_id().with("depth-help-open");
    let mut open = ui.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let response =
        design::button(ui, Icon::Info, "", "Ayuda de cámara", open, true).on_hover_text(tr(HELP));
    if response.clicked() {
        open = !open;
    }
    egui::Popup::from_response(&response)
        .width(340.)
        .open_bool(&mut open)
        .show(|ui| {
            ui.label(tr(HELP));
        });
    ui.data_mut(|data| data.insert_temp(id, open));
}
pub fn optics_ui(ui: &mut egui::Ui, optics: &mut Optics) -> bool {
    design::category(ui, "Profundidad de campo");
    ui.horizontal(|ui| {
        label(ui, Icon::Camera, "Ajustes de enfoque");
        help(ui);
    });
    let mut changed = false;
    label(ui, Icon::Blur, "Método de desenfoque");
    egui::ComboBox::from_id_salt("camera-blur-method")
        .selected_text(tr(if optics.method == BlurMethod::Gaussian {
            "Gaussiano"
        } else {
            "Lente"
        }))
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut optics.method, BlurMethod::Gaussian, tr("Gaussiano"))
                .changed();
            changed |= ui
                .selectable_value(&mut optics.method, BlurMethod::Lens, tr("Lente"))
                .changed();
        });
    label(ui, Icon::Blur, "Fuerza de desenfoque");
    changed |= ui
        .add(egui::Slider::new(&mut optics.strength, 0. ..=100.))
        .changed();
    label(ui, Icon::Settings, "Calidad del desenfoque");
    egui::ComboBox::from_id_salt("camera-blur-quality")
        .selected_text(tr(match optics.quality {
            Quality::Low => "Baja",
            Quality::Medium => "Media",
            Quality::High => "Alta",
        }))
        .show_ui(ui, |ui| {
            for (q, s) in [
                (Quality::Low, "Baja"),
                (Quality::Medium, "Media"),
                (Quality::High, "Alta"),
            ] {
                changed |= ui.selectable_value(&mut optics.quality, q, tr(s)).changed();
            }
        });
    for (icon, title, value) in [
        (Icon::Focus, "Punto de foco", &mut optics.focus),
        (Icon::Depth, "Amplitud de enfoque", &mut optics.range),
    ] {
        label(ui, icon, title);
        changed |= ui
            .add(egui::DragValue::new(value).range(0. ..=2e6).speed(1.))
            .changed();
    }
    ui.small(format!(
        "{}: {:.0} – {:.0}",
        tr("Zona nítida"),
        (optics.focus - optics.range).max(0.),
        optics.focus + optics.range
    ));
    changed
}
#[derive(Default)]
pub struct DepthEditor {
    pub open: bool,
    pub hide_blur: bool,
    pub preview: Option<egui::TextureHandle>,
    job: Option<std::sync::mpsc::Receiver<(Vec<usize>, Vec<u8>, [usize; 2])>>,
    requested: Vec<usize>,
    completed: Vec<usize>,
    owner: Option<Document>,
}
impl Editor {
    pub fn depth_window(&mut self, ctx: &egui::Context) {
        if !self.depth.open {
            return;
        }
        let mut open = true;
        egui::Window::new(tr("Profundidad de capas"))
            .id(egui::Id::unique("layer-depth"))
            .open(&mut open)
            .default_size([740., 600.])
            .vscroll(true)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    label(ui, Icon::Camera, "Cámara");
                    if self.doc.extras.cameras.is_empty() {
                        if design::button(
                            ui,
                            Icon::Plus,
                            "Añadir cámara",
                            "Añadir cámara",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.add_camera();
                        }
                    } else {
                        let selected = camera(&self.doc, self.frame)
                            .map(|c| c.id)
                            .or(self.feature.selected_camera);
                        egui::ComboBox::from_id_salt("depth-camera")
                            .selected_text(
                                self.doc
                                    .extras
                                    .cameras
                                    .iter()
                                    .find(|c| Some(c.id) == selected)
                                    .map(|c| c.name.as_str())
                                    .unwrap_or("Camera"),
                            )
                            .show_ui(ui, |ui| {
                                for c in &self.doc.extras.cameras {
                                    if ui
                                        .selectable_label(Some(c.id) == selected, &c.name)
                                        .clicked()
                                    {
                                        self.feature.selected_camera = Some(c.id);
                                        self.frame = c.start;
                                    }
                                }
                            });
                    }
                    help(ui);
                });
                let index = self
                    .feature
                    .selected_camera
                    .and_then(|id| self.doc.extras.cameras.iter().position(|c| c.id == id))
                    .or_else(|| {
                        self.doc.extras.cameras.iter().position(|c| {
                            self.frame >= c.start && self.frame < c.start + c.duration
                        })
                    });
                let mut position = index.map_or(0., |i| {
                    crate::animation::camera_optics(&self.doc.extras.cameras[i], self.frame)
                        .position
                });
                ui.horizontal(|ui| {
                    label(ui, Icon::Depth, "Posición de cámara");
                    if ui
                        .add_enabled(
                            index.is_some(),
                            egui::DragValue::new(&mut position).range(-1e6..=1e6),
                        )
                        .changed()
                    {
                        self.checkpoint();
                        let camera = &mut self.doc.extras.cameras[index.unwrap()];
                        let mut key = crate::animation::camera_at(camera, self.frame);
                        let mut optics = key.optics(camera);
                        optics.position = position;
                        key.optics = Some(optics);
                        key.frame = self
                            .frame
                            .saturating_sub(camera.start)
                            .min(camera.duration - 1);
                        crate::animation::camera_record(camera, key);
                        self.recover();
                    }
                });
                let mut keep = self.doc.layers.iter().all(|l| l.options.depth.keep_size)
                    && self.video_depth_rows().iter().all(|(_, _, p)| p.keep_size);
                if ui.checkbox(&mut keep, tr("Mantener tamaño")).changed() {
                    self.checkpoint();
                    for l in &mut self.doc.layers {
                        l.options.depth.keep_size = keep;
                    }
                    for track in &mut self.doc.extras.media.tracks {
                        track.depth.keep_size = keep;
                    }
                    self.recover();
                }
                ui.small(tr("Vista lateral · distancias desde la cámara"));
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 170.),
                    egui::Sense::hover(),
                );
                self.depth_diagram(ui, rect, position);
                egui::ScrollArea::vertical()
                    .max_height(250.)
                    .show(ui, |ui| {
                        let mut edit = None;
                        for (i, l) in self.doc.layers.iter().enumerate() {
                            ui.horizontal(|ui| {
                                label(ui, Icon::Layers, &l.name);
                                let mut value = l.options.depth.distance - position;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut value)
                                            .range(1. ..=1e6)
                                            .prefix("↔ "),
                                    )
                                    .on_hover_text(tr("Distancia a la cámara"))
                                    .changed()
                                {
                                    edit = Some((i, (value + position).clamp(1., 1e6)));
                                }
                                if !l.visible {
                                    label(ui, Icon::EyeOff, "Oculta");
                                }
                            });
                        }
                        for (index, name, mut plane) in self.video_depth_rows() {
                            ui.horizontal(|ui| {
                                label(ui, Icon::Film, &name);
                                let mut distance = plane.distance - position;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut distance)
                                            .range(1. ..=1e6)
                                            .prefix("↔ "),
                                    )
                                    .changed()
                                {
                                    plane.distance = (distance + position).clamp(1., 1e6);
                                    self.set_video_depth(index, plane);
                                }
                            });
                        }
                        if let Some((i, value)) = edit {
                            self.checkpoint();
                            self.doc.layers[i].options.depth.distance = value;
                            self.recover();
                        }
                    });
                ui.separator();
                label(ui, Icon::Eye, "Previsualización de cámara");
                if let Some(texture) = self
                    .depth
                    .preview
                    .as_ref()
                    .filter(|_| !self.depth.hide_blur)
                {
                    ui.add(
                        egui::Image::new(texture).max_size(Vec2::new(ui.available_width(), 230.)),
                    );
                } else {
                    ui.small(tr("Activa la cámara para previsualizar el resultado"));
                }
            });
        self.depth.open = open;
    }
    fn depth_diagram(&self, ui: &egui::Ui, rect: Rect, camera: f32) {
        let p = ui.painter();
        p.rect_filled(rect, 5., crate::themes::panel());
        let x = rect.left() + 38.;
        let mid = rect.center().y;
        design::draw(
            p,
            Rect::from_center_size(Pos2::new(x, mid), Vec2::splat(26.)),
            Icon::Camera,
            crate::themes::accent(),
        );
        let max = self
            .doc
            .layers
            .iter()
            .map(|l| (l.options.depth.distance - camera).max(1.))
            .fold(100., f32::max);
        p.line_segment(
            [Pos2::new(x + 18., mid), Pos2::new(rect.right() - 15., mid)],
            Stroke::new(1., crate::themes::muted()),
        );
        for (i, l) in self.doc.layers.iter().enumerate() {
            let distance = l.options.depth.distance - camera;
            let xx = x + 35. + distance.max(0.) / max * (rect.width() - 105.);
            let y = rect.top() + 25. + (i % 4) as f32 * 24.;
            let color = if i == self.layer {
                crate::themes::accent()
            } else {
                Color32::from_rgb(115, 153, 180)
            };
            p.line_segment(
                [Pos2::new(xx, y), Pos2::new(xx, rect.bottom() - 18.)],
                Stroke::new(2., color),
            );
            p.text(
                Pos2::new(xx + 4., y),
                egui::Align2::LEFT_TOP,
                format!("{} · {:.0}", l.name, distance),
                egui::FontId::proportional(11.),
                color,
            );
        }
    }
}
// All channels are premultiplied: transparent pixels never acquire dark halos.
pub fn blur(pixels: Vec<u8>, w: u32, h: u32, radius: f32, optics: &Optics) -> Vec<u8> {
    if radius < 0.1 {
        return pixels;
    }
    let factor = match optics.quality {
        Quality::Low => 0.25,
        Quality::Medium => 0.5,
        Quality::High => 1.,
    };
    let image = image::RgbaImage::from_raw(w, h, pixels).unwrap();
    let (sw, sh) = (
        ((w as f32 * factor).round() as u32).max(1),
        ((h as f32 * factor).round() as u32).max(1),
    );
    let small = if factor < 1. {
        image::imageops::resize(&image, sw, sh, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    let result = if optics.method == BlurMethod::Gaussian {
        image::imageops::blur(&small, (radius * factor * 0.5).max(0.1))
    } else {
        let samples = match optics.quality {
            Quality::Low => 8,
            Quality::Medium => 20,
            Quality::High => 48,
        };
        let offsets: Vec<_> = (0..samples)
            .map(|i| {
                let a = i as f32 * 2.399963;
                let r = ((i as f32 + 0.5) / samples as f32).sqrt() * radius * factor;
                ((a.cos() * r).round() as i32, (a.sin() * r).round() as i32)
            })
            .collect();
        image::RgbaImage::from_fn(sw, sh, |x, y| {
            let mut sum = [0u32; 4];
            for &(dx, dy) in &offsets {
                let xx = x as i32 + dx;
                let yy = y as i32 + dy;
                if xx >= 0 && yy >= 0 && xx < sw as i32 && yy < sh as i32 {
                    let c = small.get_pixel(xx as u32, yy as u32);
                    for k in 0..4 {
                        sum[k] += c[k] as u32;
                    }
                }
            }
            image::Rgba(std::array::from_fn(|k| (sum[k] / samples) as u8))
        })
    };
    if factor < 1. {
        image::imageops::resize(&result, w, h, image::imageops::FilterType::Triangle).into_raw()
    } else {
        result.into_raw()
    }
}
impl Editor {
    pub fn camera_depth_view(&self, base: View, layer: usize) -> View {
        let base = if active(self) {
            layer_view(&self.doc, self.frame, layer, base)
        } else {
            base
        };
        crate::folder_groups::view(
            base,
            self.doc
                .layers
                .get(layer)
                .map(|l| crate::folder_groups::layer_matrix(&self.doc, l))
                .unwrap_or(crate::rig::IDENTITY),
        )
    }
    pub fn camera_blur_preview(&mut self, painter: &egui::Painter, stage: View) -> bool {
        let enabled = self.depth.open || active(self);
        let optics =
            camera(&self.doc, self.frame).map(|c| crate::animation::camera_optics(c, self.frame));
        if optics.is_none() {
            self.depth.preview = None;
            self.depth.job = None;
            self.depth.requested.clear();
            self.depth.completed.clear();
            self.depth.owner = None;
            return false;
        }
        let Some(optics) = optics.filter(|_| enabled && !self.depth.hide_blur) else {
            return false;
        };
        if optics.strength == 0. && !self.depth.open {
            return false;
        }
        let max = match optics.quality {
            Quality::Low => 480.,
            Quality::Medium => 800.,
            Quality::High => 1200.,
        };
        let scale = (max / self.doc.width.max(self.doc.height) as f32).min(1.);
        let size = [
            (self.doc.width as f32 * scale).round().max(1.) as usize,
            (self.doc.height as f32 * scale).round().max(1.) as usize,
        ];
        let video = self.depth_video_frames();
        let mut key = vec![
            self.doc.layers.identity(),
            self.doc.extras.cameras.identity(),
            self.doc.extras.symbols.identity(),
            self.doc.extras.folders.identity(),
            self.doc.extras.rig.bones.identity(),
            self.doc.extras.rig.bindings.identity(),
            self.doc.extras.media.assets.identity(),
            self.doc.extras.media.tracks.identity(),
            self.doc.width as usize,
            self.doc.height as usize,
            self.frame,
            size[0],
            size[1],
        ];
        key.push(if self.playing {
            self.animation.clock
        } else {
            self.frame
        });
        for clip in &video {
            key.push(std::sync::Arc::as_ptr(&clip.pixels) as usize);
        }
        if let Some(job) = &self.depth.job {
            match job.try_recv() {
                Ok((done, pixels, size)) => {
                    self.depth.preview = Some(painter.ctx().load_texture(
                        "camera-depth-preview",
                        egui::ColorImage::from_rgb(size, &pixels),
                        egui::TextureOptions::LINEAR,
                    ));
                    self.depth.completed = done;
                    self.depth.job = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.depth.job = None;
                    self.depth.requested.clear();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.depth.job.is_none() && self.depth.requested != key {
            let doc = crate::animation::scene_doc(&self.doc, self.doc.extras.active_scene);
            self.depth.owner = Some(doc.clone());
            self.depth.requested = key.clone();
            let frame = self.frame;
            let clock = if self.playing {
                self.animation.clock
            } else {
                self.frame
            };
            let (tx, rx) = std::sync::mpsc::channel();
            self.depth.job = Some(rx);
            let ctx = painter.ctx().clone();
            std::thread::spawn(move || {
                let render = egui::Context::default();
                let mut cache = crate::paint::RenderCache::default();
                let pixels = crate::exporter::raster_preview(
                    &doc,
                    frame,
                    clock,
                    size[0] as u32,
                    size[1] as u32,
                    &render,
                    &mut cache,
                    &video,
                );
                let _ = tx.send((key, pixels, size));
                ctx.request_repaint();
            });
        }
        let ready = self.depth.completed == self.depth.requested;
        let compatible = self.depth.completed.get(..9) == self.depth.requested.get(..9)
            && self.depth.completed.get(10..12) == self.depth.requested.get(10..12);
        if !active(self) || optics.strength == 0. || (!ready && !(self.playing && compatible)) {
            return false;
        }
        let Some(texture) = &self.depth.preview else {
            return false;
        };
        let corners = [
            Pos2::ZERO,
            Pos2::new(self.doc.width as f32, 0.),
            Pos2::new(self.doc.width as f32, self.doc.height as f32),
            Pos2::new(0., self.doc.height as f32),
        ];
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for (p, uv) in corners.into_iter().zip([
            Pos2::new(0., 0.),
            Pos2::new(1., 0.),
            Pos2::new(1., 1.),
            Pos2::new(0., 1.),
        ]) {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: stage.screen(p),
                uv,
                color: Color32::WHITE,
            });
        }
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        painter.add(mesh);
        true
    }
}
impl Editor {
    pub(crate) fn depth_point(&self, point: Pos2, from: usize, to: usize) -> Pos2 {
        if from == to {
            return point;
        }
        let base = View {
            center: Pos2::ZERO,
            stage_center: Pos2::ZERO,
            scale: 1.,
            angle: 0.,
        };
        let a = self.camera_depth_view(base, from);
        let b = self.camera_depth_view(base, to);
        b.world(a.screen(point))
    }
    pub(crate) fn depth_mark(&self, mark: &Mark, from: usize, to: usize) -> Mark {
        if from == to {
            return mark.clone();
        }
        let mut result = crate::vector::transform(mark, |p| self.depth_point(p, from, to));
        let origin = self.depth_point(Pos2::ZERO, from, to);
        result.width *= self
            .depth_point(Pos2::new(1., 0.), from, to)
            .distance(origin);
        result
    }
    pub(crate) fn depth_hit(&self, layer: usize, mark: &Mark, point: Pos2, tolerance: f32) -> bool {
        crate::folder_groups::layer_active(&self.doc, &self.doc.layers[layer], self.frame)
            && self.selection_hit(mark, self.depth_point(point, self.layer, layer), tolerance)
    }
}

pub fn visible(plane: &LayerDepth, optics: &Optics) -> bool {
    plane.distance - optics.position >= 1.
}
#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> Document {
        let mut d = crate::fresh_document();
        d.width = 96;
        d.height = 64;
        d.extras.cameras.push(crate::features::Camera {
            optics: Optics::default(),
            animation: Default::default(),
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [48., 32.],
            zoom: 1.,
            angle: 0.,
        });
        d
    }
    fn square(x: f32, color: [u8; 4]) -> Mark {
        Mark {
            points: vec![[x, 20.], [x + 8., 44.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some(color),
            ..Default::default()
        }
    }
    #[test]
    fn default_depth_keeps_legacy_projection_and_maintain_size_preserves_art() {
        let mut d = doc();
        let mark = square(30., [0, 0, 0, 255]);
        d.layers[0].frames[0].push(mark.clone());
        let original = crate::features::resolve_export(&d, 0, true).remove(0).2;
        assert!(
            crate::vector::control_bounds(&original)
                .center()
                .distance(crate::vector::control_bounds(&mark).center())
                < 0.001
        );
        for distance in [200., 500., 2000.] {
            d.layers[0].options.depth.distance = distance;
            let m = crate::features::resolve_export(&d, 0, true).remove(0).2;
            assert!((crate::vector::control_bounds(&m).width() - 8.).abs() < 0.001);
        }
        d.layers[0].options.depth.keep_size = false;
        let m = crate::features::resolve_export(&d, 0, true).remove(0).2;
        assert!((crate::vector::control_bounds(&m).width() - 4.).abs() < 0.001);
    }
    #[test]
    fn nearby_layers_move_more_and_editor_projection_matches_export() {
        let mut d = doc();
        let mark = square(30., [0, 0, 0, 255]);
        d.extras.cameras[0].center[0] += 10.;
        let pose = crate::animation::camera_at(&d.extras.cameras[0], 0);
        let base = View {
            center: Pos2::new(48., 32.),
            stage_center: crate::vector::pos(pose.center),
            scale: pose.zoom,
            angle: -pose.angle,
        };
        let mut movement = vec![];
        for distance in [200., 1000., 2000.] {
            d.layers[0].options.depth.distance = distance;
            d.layers[0].frames[0] = vec![mark.clone()].into();
            let m = crate::features::resolve_export(&d, 0, true).remove(0).2;
            let projected = crate::vector::flatten(&m, 0.1)[0];
            let preview = layer_view(&d, 0, 0, base).screen(Pos2::new(30., 20.));
            assert!(preview.distance(projected) < 0.001);
            movement.push((preview.x - 30.).abs());
        }
        assert!(movement[0] > movement[1] && movement[1] > movement[2]);
        d.extras.cameras[0].optics.position = 3000.;
        assert!(crate::features::resolve_export(&d, 0, true).is_empty());
    }
    #[test]
    fn focus_interval_is_inclusive_and_transitions_continuously() {
        let o = Optics {
            focus: 200.,
            range: 100.,
            strength: 100.,
            ..Default::default()
        };
        for d in [100., 200., 300.] {
            assert_eq!(o.radius(d), 0.);
        }
        assert!(o.radius(301.) > 0. && o.radius(301.) < o.radius(310.));
        assert!(o.radius(310.) < o.radius(350.));
        assert_eq!(o.radius(0.), o.radius(400.));
    }
    #[test]
    fn both_blur_methods_preserve_premultiplied_color_and_output_size() {
        for method in [BlurMethod::Gaussian, BlurMethod::Lens] {
            for quality in [Quality::Low, Quality::Medium, Quality::High] {
                let mut pixels = vec![0; 32 * 32 * 4];
                for y in 12..20 {
                    for x in 12..20 {
                        pixels[(y * 32 + x) * 4] = 255;
                        pixels[(y * 32 + x) * 4 + 3] = 255;
                    }
                }
                let o = Optics {
                    method,
                    quality,
                    ..Default::default()
                };
                let blurred = blur(pixels, 32, 32, 6., &o);
                assert_eq!(blurred.len(), 32 * 32 * 4);
                assert!(
                    blurred
                        .chunks_exact(4)
                        .all(|p| p[0] == p[3] && p[1] == 0 && p[2] == 0)
                );
                assert!(blurred[(16 * 32 + 10) * 4 + 3] > 0);
            }
        }
    }
    #[test]
    fn depth_and_optics_roundtrip_and_old_projects_keep_defaults() {
        let mut d = doc();
        d.layers[0].options.depth.distance = 200.;
        d.extras.cameras[0].optics = Optics {
            focus: 200.,
            strength: 60.,
            method: BlurMethod::Lens,
            ..Default::default()
        };
        let json = serde_json::to_vec(&d).unwrap();
        let restored: Document = serde_json::from_slice(&json).unwrap();
        assert!(restored.validate().is_ok());
        assert!(restored.layers[0].options.depth == d.layers[0].options.depth);
        assert!(restored.extras.cameras[0].optics == d.extras.cameras[0].optics);
        let fla = crate::fla::encode(&d).unwrap();
        let restored = crate::fla::decode(&fla).unwrap();
        assert_eq!(restored.layers[0].options.depth.distance, 200.);
        assert_eq!(restored.extras.cameras[0].optics.focus, 200.);
        let mut legacy = serde_json::to_value(&d).unwrap();
        legacy["layers"][0]["options"]
            .as_object_mut()
            .unwrap()
            .remove("depth");
        legacy["extras"]["cameras"][0]
            .as_object_mut()
            .unwrap()
            .remove("optics");
        let restored: Document = serde_json::from_value(legacy).unwrap();
        assert!(restored.layers[0].options.depth.keep_size);
        assert_eq!(restored.extras.cameras[0].optics.strength, 0.);
    }
    #[test]
    fn export_blurs_only_unfocused_planes_and_svg_has_layer_filters() {
        let mut d = doc();
        d.layers[0].frames[0].push(square(20., [0, 0, 0, 255]));
        let mut layer = d.layers[0].clone();
        layer.name = "Focus".into();
        layer.options.depth.distance = 200.;
        layer.frames[0] = vec![square(70., [0, 0, 0, 255])].into();
        d.layers.push(layer);
        let ctx = egui::Context::default();
        let mut cache = crate::paint::RenderCache::default();
        let sharp = crate::exporter::raster(&d, 0, 96, 64, &ctx, &mut cache);
        for method in [BlurMethod::Gaussian, BlurMethod::Lens] {
            d.extras.cameras[0].optics = Optics {
                focus: 200.,
                range: 100.,
                strength: 50.,
                quality: Quality::High,
                method,
                ..Default::default()
            };
            let pixels = crate::exporter::raster(&d, 0, 96, 64, &ctx, &mut cache);
            assert_eq!(pixels.len(), 96 * 64 * 3);
            assert!(pixels[(32 * 96 + 17) * 3] < sharp[(32 * 96 + 17) * 3]);
            assert_eq!(pixels[(32 * 96 + 74) * 3], sharp[(32 * 96 + 74) * 3]);
            assert!(pixels[(32 * 96 + 69) * 3].abs_diff(sharp[(32 * 96 + 69) * 3]) <= 1);
            let svg = d.svg(0);
            assert!(svg.contains("dof-0"));
            assert!(!svg.contains("dof-1"));
        }
    }
    #[test]
    fn hidden_editor_blur_does_not_schedule_rendering_or_change_export() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut e = Editor::new(&cc);
        e.doc = doc();
        e.doc.layers[0].frames[0].push(square(20., [0, 0, 0, 255]));
        e.doc.extras.cameras[0].optics.strength = 30.;
        e.studio.stage_only = true;
        e.depth.hide_blur = true;
        let before = serde_json::to_vec(&e.doc).unwrap();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let view = View {
                center: Pos2::ZERO,
                scale: 1.,
                stage_center: Pos2::ZERO,
                angle: 0.,
            };
            assert!(!e.camera_blur_preview(ui.painter(), view));
        });
        out.textures_delta.clear();
        assert!(e.depth.job.is_none());
        assert!(e.depth.preview.is_none());
        assert_eq!(before, serde_json::to_vec(&e.doc).unwrap());
    }
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn offstage_art_can_blur_into_the_camera_frame() {
        let mut d = crate::fresh_document();
        d.width = 64;
        d.height = 64;
        d.layers[0].frames[0].push(Mark {
            points: vec![[-10., 20.], [-1., 44.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([0, 0, 0, 255]),
            ..Default::default()
        });
        let ctx = egui::Context::default();
        let mut cache = crate::paint::RenderCache::default();
        d.extras.cameras.push(crate::features::Camera {
            optics: Optics {
                strength: 50.,
                focus: 200.,
                quality: Quality::High,
                ..Default::default()
            },
            animation: Default::default(),
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [32., 32.],
            zoom: 1.,
            angle: 0.,
        });
        let pixels = crate::exporter::raster(&d, 0, 64, 64, &ctx, &mut cache);
        assert!(pixels[(32 * 64) * 3] < 250);
    }
    #[test]
    fn depth_window_and_camera_properties_render_on_a_laptop_sized_ui() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut e = Editor::new(&cc);
        e.doc = crate::fresh_document();
        e.add_camera();
        e.depth.open = true;
        let before = serde_json::to_vec(&e.doc).unwrap();
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1366., 768.))),
                    ..Default::default()
                },
                |ui| {
                    e.depth_window(ui.ctx());
                    egui::Panel::right("camera-properties-test").show(ui, |ui| {
                        e.camera_properties(ui);
                    });
                },
            );
            assert!(output.shapes.len() > 20);
            output.textures_delta.clear();
        }
        assert_eq!(before, serde_json::to_vec(&e.doc).unwrap());
        assert!(e.doc.layers[0].options.depth.keep_size);
    }
    #[test]
    #[ignore = "manual release benchmark; software compositing, not whole-app FPS"]
    fn benchmark_camera_depth_and_blur() {
        let mut d = crate::fresh_document();
        d.width = 640;
        d.height = 360;
        d.layers[0].frames[0].push(Mark {
            points: vec![[100., 80.], [450., 260.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([40, 120, 220, 255]),
            ..Default::default()
        });
        let mut second = d.layers[0].clone();
        second.options.depth.distance = 200.;
        d.layers.push(second);
        d.extras.cameras.push(crate::features::Camera {
            optics: Optics {
                strength: 60.,
                focus: 200.,
                ..Default::default()
            },
            animation: Default::default(),
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [320., 180.],
            zoom: 1.,
            angle: 0.,
        });
        for method in [BlurMethod::Gaussian, BlurMethod::Lens] {
            for quality in [Quality::Low, Quality::Medium, Quality::High] {
                d.extras.cameras[0].optics.method = method;
                d.extras.cameras[0].optics.quality = quality;
                let ctx = egui::Context::default();
                let mut cache = crate::paint::RenderCache::default();
                let start = std::time::Instant::now();
                for _ in 0..3 {
                    std::hint::black_box(crate::exporter::raster(
                        &d, 0, 640, 360, &ctx, &mut cache,
                    ));
                }
                eprintln!(
                    "640x360, 2 layers, {} / {}: {:?}/frame",
                    if method == BlurMethod::Gaussian {
                        "Gaussian"
                    } else {
                        "Lens"
                    },
                    match quality {
                        Quality::Low => "low",
                        Quality::Medium => "medium",
                        Quality::High => "high",
                    },
                    start.elapsed() / 3
                );
            }
        }
    }
}
#[cfg(test)]
mod focus_key_tests {
    use super::*;
    fn project() -> Document {
        let mut d = crate::fresh_document();
        d.width = 96;
        d.height = 64;
        let mut camera = crate::features::Camera {
            optics: Optics {
                focus: 200.,
                range: 100.,
                strength: 50.,
                ..Default::default()
            },
            animation: Default::default(),
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [48., 32.],
            zoom: 1.,
            angle: 0.,
        };
        let a = crate::animation::camera_at(&camera, 0);
        crate::animation::camera_record(&mut camera, a);
        let mut b = crate::animation::camera_at(&camera, 20);
        b.frame = 20;
        b.optics.as_mut().unwrap().focus = 1000.;
        crate::animation::camera_record(&mut camera, b);
        d.extras.cameras.push(camera);
        d
    }
    #[test]
    fn keys_interpolate_all_numeric_optics_and_step_method_and_quality() {
        let mut d = project();
        let c = &mut d.extras.cameras[0];
        let end = c.animation.keys[1].optics.as_mut().unwrap();
        end.position = 100.;
        end.range = 40.;
        end.strength = 90.;
        end.method = BlurMethod::Lens;
        end.quality = Quality::High;
        let middle = crate::animation::camera_optics(c, 10);
        assert_eq!(
            (middle.focus, middle.range, middle.position, middle.strength),
            (600., 70., 50., 70.)
        );
        assert!(middle.method == BlurMethod::Gaussian && middle.quality == Quality::Medium);
        let end = crate::animation::camera_optics(c, 20);
        assert!(end.method == BlurMethod::Lens && end.quality == Quality::High);
        let restored: Document = serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
        assert!(crate::animation::camera_optics(&restored.extras.cameras[0], 10) == middle);
        let restored = crate::fla::decode(&crate::fla::encode(&d).unwrap()).unwrap();
        assert!(crate::animation::camera_optics(&restored.extras.cameras[0], 10) == middle);
        let mut old = serde_json::to_value(&d).unwrap();
        for k in old["extras"]["cameras"][0]["animation"]["keys"]
            .as_array_mut()
            .unwrap()
        {
            k.as_object_mut().unwrap().remove("optics");
        }
        let old: Document = serde_json::from_value(old).unwrap();
        assert_eq!(
            crate::animation::camera_optics(&old.extras.cameras[0], 10).focus,
            200.
        );
    }
    #[test]
    fn focus_pull_changes_the_exported_sharp_plane() {
        let mut d = project();
        let rectangle = |x| Mark {
            points: vec![[x, 20.], [x + 8., 44.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([0, 0, 0, 255]),
            ..Default::default()
        };
        d.layers[0].frames[0] = vec![rectangle(20.)].into();
        d.layers[0].frames[20] = d.layers[0].frames[0].clone();
        let mut layer = d.layers[0].clone();
        layer.options.depth.distance = 200.;
        layer.frames[0] = vec![rectangle(70.)].into();
        layer.frames[20] = layer.frames[0].clone();
        d.layers.push(layer);
        let ctx = egui::Context::default();
        let mut cache = crate::paint::RenderCache::default();
        let a = crate::exporter::raster(&d, 0, 96, 64, &ctx, &mut cache);
        let b = crate::exporter::raster(&d, 20, 96, 64, &ctx, &mut cache);
        assert!(a[(32 * 96 + 17) * 3] < b[(32 * 96 + 17) * 3]);
        assert!(a[(32 * 96 + 67) * 3] > b[(32 * 96 + 67) * 3]);
    }
    #[test]
    fn splitting_and_trimming_a_camera_preserve_evaluated_focus_and_undo() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        e.doc = project();
        let original = e.doc.extras.cameras[0].clone();
        let expected = crate::animation::camera_optics(&original, 10);
        e.camera_timeline_action(1, 10, 4);
        assert_eq!(e.doc.extras.cameras.len(), 2);
        let right = &e.doc.extras.cameras[1];
        assert!(crate::animation::camera_optics(right, 10) == expected);
        assert_eq!(
            crate::animation::camera_optics(right, 15).focus,
            crate::animation::camera_optics(&original, 15).focus
        );
        e.history(false);
        assert_eq!(e.doc.extras.cameras.len(), 1);
        assert!(crate::animation::camera_optics(&e.doc.extras.cameras[0], 10) == expected);
    }
}
