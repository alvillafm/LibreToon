//! Lighting for the composited artwork underneath a timeline layer.
use crate::{
    Editor,
    canvas::View,
    design::{self, Icon},
    i18n::tr,
    shared::SharedVec,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    Point,
    Directional,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Point => "Point light",
            Self::Directional => "Directional light",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    position: [f32; 2],
    radius: f32,
    intensity: f32,
    relief: f32,
    color: [u8; 3],
    angle: f32,
    elevation: f32,
    falloff: f32,
    bevel: f32,
    texture_relief: f32,
    shadow: f32,
    specular: f32,
    gloss: f32,
    steps: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            position: [0., 0.],
            radius: 240.,
            intensity: 0.75,
            relief: 0.55,
            color: [255, 228, 181],
            angle: -135.,
            elevation: 40.,
            falloff: 1.4,
            bevel: 28.,
            texture_relief: 0.,
            shadow: 0.28,
            specular: 0.16,
            gloss: 24.,
            steps: 0,
        }
    }
}

impl Settings {
    fn valid(self) -> bool {
        self.position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 100_000.)
            && (1. ..=20_000.).contains(&self.radius)
            && (0. ..=2.).contains(&self.intensity)
            && (0. ..=1.).contains(&self.relief)
            && (-180. ..=180.).contains(&self.angle)
            && (5. ..=85.).contains(&self.elevation)
            && (0.25..=5.).contains(&self.falloff)
            && (1. ..=240.).contains(&self.bevel)
            && (0. ..=1.).contains(&self.texture_relief)
            && (0. ..=1.).contains(&self.shadow)
            && (0. ..=1.).contains(&self.specular)
            && (2. ..=96.).contains(&self.gloss)
            && self.steps <= 8
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    #[serde(default)]
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gizmo_position: Option<[f32; 2]>,
    base: Settings,
    #[serde(default)]
    keys: SharedVec<(usize, Settings)>,
}

impl Light {
    pub fn new(w: u32, h: u32, kind: Kind) -> Self {
        Self {
            kind,
            gizmo_position: None,
            base: Settings {
                position: [w as f32 * 0.5, h as f32 * 0.4],
                radius: (w.min(h) as f32 * 0.42).max(1.),
                ..Settings::default()
            },
            keys: Default::default(),
        }
    }

    pub fn valid(&self) -> bool {
        self.base.valid()
            && self
                .gizmo_position
                .is_none_or(|point| point.iter().all(|v| v.is_finite() && v.abs() <= 100_000.))
            && self.keys.len() <= 10_000
            && self
                .keys
                .iter()
                .all(|(frame, settings)| *frame < 10_000 && settings.valid())
            && self.keys.windows(2).all(|pair| pair[0].0 < pair[1].0)
    }

    pub fn keyed(&self, frame: usize) -> bool {
        self.keys.binary_search_by_key(&frame, |k| k.0).is_ok()
    }

    fn set(&mut self, frame: usize, value: Settings) {
        match self.keys.binary_search_by_key(&frame, |k| k.0) {
            Ok(i) => self.keys[i].1 = value,
            Err(i) => self.keys.insert(i, (frame, value)),
        }
    }

    fn at(&self, frame: usize) -> Settings {
        let after = self.keys.partition_point(|key| key.0 <= frame);
        let (a, start) = after
            .checked_sub(1)
            .map_or((0, self.base), |i| self.keys[i]);
        let Some(&(b, end)) = self.keys.get(after) else {
            return start;
        };
        let t = (frame - a) as f32 / b.saturating_sub(a).max(1) as f32;
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        Settings {
            position: std::array::from_fn(|i| lerp(start.position[i], end.position[i])),
            radius: lerp(start.radius, end.radius),
            intensity: lerp(start.intensity, end.intensity),
            relief: lerp(start.relief, end.relief),
            angle: {
                let delta = (end.angle - start.angle + 180.).rem_euclid(360.) - 180.;
                (start.angle + delta * t + 180.).rem_euclid(360.) - 180.
            },
            elevation: lerp(start.elevation, end.elevation),
            falloff: lerp(start.falloff, end.falloff),
            bevel: lerp(start.bevel, end.bevel),
            texture_relief: lerp(start.texture_relief, end.texture_relief),
            shadow: lerp(start.shadow, end.shadow),
            specular: lerp(start.specular, end.specular),
            gloss: lerp(start.gloss, end.gloss),
            steps: start.steps,
            color: std::array::from_fn(|i| {
                lerp(start.color[i].into(), end.color[i].into()).round() as u8
            }),
        }
    }

    pub fn apply(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        doc: &crate::Document,
        frame: usize,
        camera: bool,
        opacity: f32,
    ) {
        let settings = self.at(frame);
        if settings.intensity == 0. || opacity == 0. {
            return;
        }
        let center = Pos2::new(doc.width as f32 * 0.5, doc.height as f32 * 0.5);
        let mut point = crate::vector::pos(settings.position);
        let mut zoom = 1.;
        let mut angle = settings.angle.to_radians();
        if camera && let Some(camera) = crate::depth::camera(doc, frame) {
            let pose = crate::animation::camera_at(camera, frame);
            zoom = pose.zoom;
            angle -= pose.angle;
            point = center
                + egui::emath::Rot2::from_angle(-pose.angle)
                    * (point - crate::vector::pos(pose.center))
                    * zoom;
        }
        let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
        let x = (point.x - center.x) * scale + w as f32 * 0.5;
        let y = (point.y - center.y) * scale + h as f32 * 0.5;
        let radius = (settings.radius * zoom * scale).max(1.);
        let (left, top, right, bottom) = match self.kind {
            Kind::Point => (
                (x - radius).max(0.) as usize,
                (y - radius).max(0.) as usize,
                ((x + radius).ceil() as usize).min(w as usize),
                ((y + radius).ceil() as usize).min(h as usize),
            ),
            Kind::Directional => (0, 0, w as usize, h as usize),
        };
        if left >= right || top >= bottom {
            return;
        }
        let width = w as usize;
        let surface = (settings.relief > 0.).then(|| {
            Surface::new(
                pixels,
                width,
                h as usize,
                settings.bevel * scale * zoom,
                settings.texture_relief,
            )
        });
        let elevation = settings.elevation.to_radians();
        let dir = Vec2::angled(angle) * elevation.cos();
        let height = elevation.sin().max(0.08);
        let tint = settings.color.map(|c| f32::from(c) / 255.);
        for py in top..bottom {
            for px in left..right {
                let idx = (py * width + px) * 4;
                let alpha = f32::from(pixels[idx + 3]);
                if alpha == 0. {
                    continue;
                }
                let (lx, ly, lz, attenuation) = match self.kind {
                    Kind::Directional => (dir.x, dir.y, height, 1.),
                    Kind::Point => {
                        let dx = (x - px as f32) / radius;
                        let dy = (y - py as f32) / radius;
                        let d2 = dx * dx + dy * dy;
                        if d2 >= 1. {
                            continue;
                        }
                        let z = height * 0.85;
                        let inverse = (d2 + z * z).sqrt().recip();
                        (
                            dx * inverse,
                            dy * inverse,
                            z * inverse,
                            (1. - d2.sqrt()).powf(settings.falloff),
                        )
                    }
                };
                let n = surface
                    .as_ref()
                    .map_or(Vec2::ZERO, |surface| surface.slope(px, py))
                    * settings.relief
                    * 3.2;
                let nz = (1. + n.length_sq()).sqrt().recip();
                let mut diffuse = ((n.x * lx + n.y * ly + lz) * nz).clamp(0., 1.);
                if settings.steps >= 2 {
                    let steps = f32::from(settings.steps - 1);
                    diffuse = (diffuse * steps).round() / steps;
                }
                let halfway = (2. + 2. * lz).sqrt();
                let specular = if settings.specular > 0. {
                    (((n.x * lx + n.y * ly + lz + 1.) * nz) / halfway)
                        .clamp(0., 1.)
                        .powf(settings.gloss)
                        * settings.specular
                } else {
                    0.
                };
                let strength = settings.intensity * attenuation * opacity;
                let shade = (1. - diffuse) * settings.shadow * strength;
                for channel in 0..3 {
                    let value = f32::from(pixels[idx + channel]);
                    let lit = value * (1. - shade)
                        + ((alpha - value) * diffuse * 0.64 + alpha * specular)
                            * strength
                            * tint[channel];
                    pixels[idx + channel] = lit.clamp(0., alpha).round() as u8;
                }
            }
        }
    }

    pub fn edit_key(&mut self, frame: usize, destination: Option<usize>) -> bool {
        match (self.keys.binary_search_by_key(&frame, |k| k.0), destination) {
            (Ok(index), Some(to)) if to != frame && !self.keyed(to) => {
                let value = self.keys.remove(index).1;
                self.set(to, value);
            }
            (Ok(index), None) => {
                self.keys.remove(index);
            }
            (Err(_), Some(to)) if to == frame => {
                self.set(frame, self.at(frame));
            }
            _ => return false,
        }
        true
    }
}

// A capped, two-pass distance field gives an editable bevel across the silhouette.
// Texture relief also works on opaque artwork, where coverage has no internal edges.
struct Surface {
    width: usize,
    height: usize,
    step: usize,
    slopes: Vec<Vec2>,
}
impl Surface {
    fn new(pixels: &[u8], width: usize, height: usize, bevel: f32, texture: f32) -> Self {
        let step = width.max(height).div_ceil(1024).max(1);
        let w = width.div_ceil(step);
        let h = height.div_ceil(step);
        let limit = (bevel / step as f32).max(1.) * 3.;
        let cap = limit.ceil().min(4096.) as u16;
        let mut distance = vec![cap; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if pixels[(y * step * width + x * step) * 4 + 3] < 128 {
                    distance[i] = 0;
                    continue;
                }
                if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                    distance[i] = 3;
                }
                if x > 0 {
                    distance[i] = distance[i].min(distance[i - 1] + 3);
                }
                if y > 0 {
                    distance[i] = distance[i].min(distance[i - w] + 3);
                    if x > 0 {
                        distance[i] = distance[i].min(distance[i - w - 1] + 4);
                    }
                    if x + 1 < w {
                        distance[i] = distance[i].min(distance[i - w + 1] + 4);
                    }
                }
            }
        }
        for y in (0..h).rev() {
            for x in (0..w).rev() {
                let i = y * w + x;
                if x + 1 < w {
                    distance[i] = distance[i].min(distance[i + 1] + 3);
                }
                if y + 1 < h {
                    distance[i] = distance[i].min(distance[i + w] + 3);
                    if x > 0 {
                        distance[i] = distance[i].min(distance[i + w - 1] + 4);
                    }
                    if x + 1 < w {
                        distance[i] = distance[i].min(distance[i + w + 1] + 4);
                    }
                }
            }
        }
        let mut values: Vec<_> = distance
            .into_iter()
            .enumerate()
            .map(|(i, distance)| {
                let p = ((i / w) * step * width + (i % w) * step) * 4;
                let alpha = f32::from(pixels[p + 3]).max(1.);
                let luma = (0.2126 * f32::from(pixels[p])
                    + 0.7152 * f32::from(pixels[p + 1])
                    + 0.0722 * f32::from(pixels[p + 2]))
                    / alpha;
                let t = (f32::from(distance) / limit).min(1.);
                t * (2. - t) * (limit / 3.).min(48.) + luma * texture * 8.
            })
            .collect();
        // Smooth both the distance-field steps and the optional luma detail before
        // deriving normals. The bounded grid keeps this independent of export size.
        let mut scratch = vec![0.; values.len()];
        for _ in 0..2 {
            smooth_surface(&values, &mut scratch, w, h, true);
            smooth_surface(&scratch, &mut values, w, h, false);
        }
        let slopes = (0..w * h)
            .map(|i| {
                let x = i % w;
                let y = i / w;
                Vec2::new(
                    values[y * w + x.saturating_sub(1)] - values[y * w + (x + 1).min(w - 1)],
                    values[y.saturating_sub(1) * w + x] - values[(y + 1).min(h - 1) * w + x],
                ) * 0.5
            })
            .collect();
        Self {
            width: w,
            height: h,
            step,
            slopes,
        }
    }

    fn slope(&self, px: usize, py: usize) -> Vec2 {
        let x = px / self.step;
        let y = py / self.step;
        if self.step == 1 {
            return self.slopes[y * self.width + x];
        }
        let x1 = (x + 1).min(self.width - 1);
        let y1 = (y + 1).min(self.height - 1);
        let fx = (px % self.step) as f32 / self.step as f32;
        let fy = (py % self.step) as f32 / self.step as f32;
        let at = |x, y| self.slopes[y * self.width + x];
        let lerp = |a: Vec2, b: Vec2, t| a + (b - a) * t;
        lerp(
            lerp(at(x, y), at(x1, y), fx),
            lerp(at(x, y1), at(x1, y1), fx),
            fy,
        )
    }
}

fn smooth_surface(
    source: &[f32],
    target: &mut [f32],
    width: usize,
    height: usize,
    horizontal: bool,
) {
    for (i, value) in target.iter_mut().enumerate() {
        let x = i % width;
        let y = i / width;
        let axis = if horizontal { x } else { y };
        let last = if horizontal { width - 1 } else { height - 1 };
        let sample = |offset: isize| {
            let p = axis.saturating_add_signed(offset).min(last);
            source[if horizontal {
                y * width + p
            } else {
                p * width + x
            }]
        };
        *value = (sample(-2) + 4. * sample(-1) + 6. * source[i] + 4. * sample(1) + sample(2)) / 16.;
    }
}

impl Editor {
    pub(crate) fn edit_light_key(
        &mut self,
        layer: usize,
        frame: usize,
        destination: Option<usize>,
    ) {
        if self.doc.layers[layer].options.locked {
            return;
        }
        let mut light = self.doc.layers[layer].options.light.clone().unwrap();
        if light.edit_key(frame, destination) {
            self.checkpoint();
            self.doc.layers[layer].options.light = Some(light);
            self.layer = layer;
            self.frame = destination.unwrap_or(frame);
            self.playing = false;
            self.recover();
        }
    }

    pub(crate) fn light_properties(&mut self, ui: &mut egui::Ui) {
        let kind = self.doc.layers[self.layer]
            .options
            .light
            .as_ref()
            .unwrap()
            .kind;
        design::category(ui, kind.name());
        ui.small(tr("Illuminates the visible artwork below this track."));
        let light = self.doc.layers[self.layer].options.light.as_ref().unwrap();
        let original = light.at(self.frame);
        let mut value = original;
        let mut remove = false;
        let mut insert = false;
        ui.add_enabled_ui(!self.doc.layers[self.layer].options.locked, |ui| {
            design::category(ui, "Light source");
            if kind == Kind::Point {
                control(
                    ui,
                    Icon::Focus,
                    "X",
                    &mut value.position[0],
                    -100_000. ..=100_000.,
                );
                control(
                    ui,
                    Icon::Focus,
                    "Y",
                    &mut value.position[1],
                    -100_000. ..=100_000.,
                );
                control(
                    ui,
                    Icon::Ellipse,
                    "Radius",
                    &mut value.radius,
                    1. ..=20_000.,
                );
                control(ui, Icon::Tween, "Falloff", &mut value.falloff, 0.25..=5.);
            } else {
                control(
                    ui,
                    Icon::Rotate,
                    "Direction",
                    &mut value.angle,
                    -180. ..=180.,
                );
            }
            for (icon, name, field, range) in [
                (Icon::Bulb, "Intensity", &mut value.intensity, 0. ..=2.),
                (Icon::Depth, "Elevation", &mut value.elevation, 5. ..=85.),
            ] {
                control(ui, icon, name, field, range);
            }
            ui.horizontal(|ui| {
                ui.label(tr("Color"));
                ui.color_edit_button_srgb(&mut value.color);
            });
            design::category(ui, "Surface shading");
            for (icon, name, field, range) in [
                (Icon::Depth, "Relief", &mut value.relief, 0. ..=1.),
                (Icon::Ellipse, "Bevel width", &mut value.bevel, 1. ..=240.),
                (
                    Icon::Grid,
                    "Texture relief",
                    &mut value.texture_relief,
                    0. ..=1.,
                ),
                (
                    Icon::ShadeBrush,
                    "Shadow strength",
                    &mut value.shadow,
                    0. ..=1.,
                ),
                (
                    Icon::LightBrush,
                    "Specular strength",
                    &mut value.specular,
                    0. ..=1.,
                ),
                (Icon::Focus, "Gloss", &mut value.gloss, 2. ..=96.),
            ] {
                control(ui, icon, name, field, range);
            }
            egui::ComboBox::from_id_salt("light-shading")
                .selected_text(if value.steps < 2 {
                    tr("Smooth shading").into()
                } else {
                    format!("{} · {}", tr("Cel shading"), value.steps)
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut value.steps, 0, tr("Smooth shading"));
                    for steps in 2..=5 {
                        ui.selectable_value(
                            &mut value.steps,
                            steps,
                            format!("{} · {steps}", tr("Cel shading")),
                        );
                    }
                });
            design::category(ui, "Light keyframes");
            ui.horizontal(|ui| {
                insert = design::button(
                    ui,
                    Icon::Plus,
                    "",
                    "Add light key",
                    false,
                    !light.keyed(self.frame),
                )
                .clicked();
                remove = design::button(
                    ui,
                    Icon::Minus,
                    "",
                    "Delete light key",
                    false,
                    light.keyed(self.frame),
                )
                .clicked();
            });
        });
        if value != original || insert || remove {
            self.checkpoint();
            let light = self.doc.layers[self.layer].options.light.as_mut().unwrap();
            if remove {
                light.keys.retain(|key| key.0 != self.frame);
            } else if light.keys.is_empty() && !insert && self.frame == 0 {
                light.base = value;
            } else {
                light.set(self.frame, value);
            }
            self.recover();
        }
    }

    pub(crate) fn light_gizmo(&mut self, ui: &mut egui::Ui, view: View, input: bool) -> bool {
        let layer = &self.doc.layers[self.layer];
        let Some(light) = layer
            .options
            .light
            .as_ref()
            .filter(|_| layer.visible && !self.playing)
        else {
            return false;
        };
        let settings = light.at(self.frame);
        let kind = light.kind;
        let anchor = crate::vector::pos(light.gizmo_position.unwrap_or(settings.position));
        let center = view.screen(anchor);
        let arrow =
            view.screen(anchor + Vec2::angled(settings.angle.to_radians()) * (55. / view.scale));
        let center_rect = Rect::from_center_size(center, Vec2::splat(27.));
        let arrow_rect = Rect::from_center_size(arrow, Vec2::splat(24.));
        if !input {
            let color = Color32::GOLD;
            if kind == Kind::Point {
                ui.painter().circle_stroke(
                    center,
                    settings.radius * view.scale,
                    Stroke::new(1., color.gamma_multiply(0.45)),
                );
            } else {
                ui.painter()
                    .arrow(center, arrow - center, Stroke::new(2., color));
                ui.painter()
                    .circle_stroke(arrow, 8., Stroke::new(1., color));
            }
            ui.painter()
                .circle_filled(center, 13., crate::themes::panel());
            design::draw(ui.painter(), center_rect.shrink(2.), Icon::Bulb, color);
            return false;
        }
        if layer.options.locked {
            return false;
        }
        let moved = ui
            .interact(
                center_rect,
                ui.scope_id().with(("move-light", self.layer)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text(tr("Move light handle"));
        let rotated = (kind == Kind::Directional).then(|| {
            ui.interact(
                arrow_rect,
                ui.scope_id().with(("rotate-light", self.layer)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text(tr("Rotate directional light"))
        });
        let response = rotated
            .as_ref()
            .filter(|r| r.dragged() || r.drag_stopped() || r.is_pointer_button_down_on());
        let (response, rotate) = response.map_or((&moved, false), |r| (r, true));
        if response.drag_started() {
            self.checkpoint();
        }
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let point = view.world(pointer);
            let position = [
                point.x.clamp(-100_000., 100_000.),
                point.y.clamp(-100_000., 100_000.),
            ];
            let light = self.doc.layers[self.layer].options.light.as_mut().unwrap();
            if rotate {
                let delta = point - anchor;
                if delta.length_sq() > 0.0001 {
                    let angle = delta.y.atan2(delta.x).to_degrees();
                    light.set(self.frame, Settings { angle, ..settings });
                }
            } else if kind == Kind::Directional {
                light.gizmo_position = Some(position);
            } else {
                light.set(
                    self.frame,
                    Settings {
                        position,
                        ..settings
                    },
                );
            }
        }
        if response.drag_stopped() {
            self.recover();
        }
        response.dragged() || response.is_pointer_button_down_on()
    }
}

fn control(
    ui: &mut egui::Ui,
    icon: Icon,
    name: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(17.), egui::Sense::hover());
        design::draw(ui.painter(), rect, icon, crate::themes::muted());
        ui.label(tr(name));
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(0.2)
                .update_while_editing(false),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bevel_normals_interpolate_between_surface_grid_samples() {
        let width = 2048;
        let height = 32;
        let mut pixels = vec![0; width * height * 4];
        for row in pixels.chunks_exact_mut(width * 4) {
            for p in row[12 * 4..64 * 4].chunks_exact_mut(4) {
                p.copy_from_slice(&[90, 90, 90, 255]);
            }
        }
        let surface = Surface::new(&pixels, width, height, 20., 0.);
        assert_eq!(surface.step, 2);
        let a = surface.slope(16, 16);
        let b = surface.slope(18, 16);
        assert_ne!(a, b);
        assert!((surface.slope(17, 16) - (a + b) * 0.5).length() < 0.0001);
    }

    #[test]
    fn light_keys_can_be_moved_and_removed_without_overwriting_another_pose() {
        let mut light = Light::new(64, 64, Kind::Directional);
        light.set(
            5,
            Settings {
                angle: 90.,
                ..light.base
            },
        );
        light.set(
            9,
            Settings {
                angle: 0.,
                ..light.base
            },
        );
        assert!(!light.edit_key(5, Some(9)));
        assert!(light.edit_key(5, Some(7)));
        assert!(!light.keyed(5));
        assert_eq!(light.at(7).angle, 90.);
        assert!(light.edit_key(7, None));
        assert!(!light.keyed(7));
        assert!(light.keyed(9));
    }

    #[test]
    fn directional_light_reverses_the_lit_edge_when_turned() {
        let doc = crate::Document {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let mut pixels = vec![0; 64 * 64 * 4];
        for y in 12..52 {
            for x in 12..52 {
                pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4].copy_from_slice(&[90, 90, 90, 255]);
            }
        }
        let mut left = pixels.clone();
        let mut light = Light::new(64, 64, Kind::Directional);
        light.base.angle = 180.;
        light.base.bevel = 12.;
        light.apply(&mut left, 64, 64, &doc, 0, false, 1.);
        let mut moved = pixels.clone();
        light.gizmo_position = Some([200., 200.]);
        light.apply(&mut moved, 64, 64, &doc, 0, false, 1.);
        assert_eq!(left, moved);
        light.base.angle = 0.;
        light.apply(&mut pixels, 64, 64, &doc, 0, false, 1.);
        let edge = (32 * 64 + 14) * 4;
        assert!(left[edge] > pixels[edge]);
        assert_eq!(left[edge + 3], 255);
        assert_eq!(left[3], 0);
    }

    #[test]
    fn keyed_point_light_moves_and_only_lights_artwork_under_its_track() {
        let mut doc = crate::Document {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let mark = |x: f32| crate::Mark {
            points: vec![[x, 16.], [x + 23., 32.]].into(),
            fill: Some([64, 64, 64, 255]),
            rectangle: true,
            stroke: false,
            closed: true,
            ..Default::default()
        };
        let mut upper = doc.layers[0].clone();
        let mut lighting = upper.clone();
        doc.layers[0].frames[0].push(mark(4.));
        upper.frames[0].push(mark(37.));
        let mut light = Light::new(64, 64, Kind::Point);
        light.base.position = [16., 24.];
        light.set(
            10,
            Settings {
                position: [48., 24.],
                ..light.base
            },
        );
        assert_eq!(light.at(5).position, [32., 24.]);
        lighting.options.light = Some(light);
        doc.layers.push(lighting);
        doc.layers.push(upper);
        let rgb = crate::exporter::raster(
            &doc,
            0,
            64,
            64,
            &egui::Context::default(),
            &mut Default::default(),
        );
        assert!(rgb[(24 * 64 + 16) * 3] > 64);
        assert_eq!(rgb[(24 * 64 + 48) * 3], 64);
        assert_eq!(rgb[0], 255);
    }
}
