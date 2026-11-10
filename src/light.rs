//! Point lighting for the composited artwork underneath a timeline layer.
use crate::{
    Editor,
    canvas::View,
    design::{self, Icon},
    i18n::tr,
    shared::SharedVec,
};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    position: [f32; 2],
    radius: f32,
    intensity: f32,
    relief: f32,
    color: [u8; 3],
}

impl Settings {
    fn valid(self) -> bool {
        self.position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 100_000.)
            && (1. ..=20_000.).contains(&self.radius)
            && (0. ..=2.).contains(&self.intensity)
            && (0. ..=1.).contains(&self.relief)
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    base: Settings,
    #[serde(default)]
    keys: SharedVec<(usize, Settings)>,
}

impl Light {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            base: Settings {
                position: [w as f32 * 0.5, h as f32 * 0.4],
                radius: (w.min(h) as f32 * 0.42).max(1.),
                intensity: 0.75,
                relief: 0.55,
                color: [255, 228, 181],
            },
            keys: Default::default(),
        }
    }

    pub fn valid(&self) -> bool {
        self.base.valid()
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
        let Settings {
            position,
            radius,
            intensity,
            relief,
            color,
        } = self.at(frame);
        if intensity == 0. || opacity == 0. {
            return;
        }
        let center = Pos2::new(doc.width as f32 * 0.5, doc.height as f32 * 0.5);
        let mut point = crate::vector::pos(position);
        let mut zoom = 1.;
        if camera && let Some(camera) = crate::depth::camera(doc, frame) {
            let pose = crate::animation::camera_at(camera, frame);
            zoom = pose.zoom;
            point = center
                + egui::emath::Rot2::from_angle(-pose.angle)
                    * (point - crate::vector::pos(pose.center))
                    * zoom;
        }
        let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
        let x = (point.x - center.x) * scale + w as f32 * 0.5;
        let y = (point.y - center.y) * scale + h as f32 * 0.5;
        let radius = (radius * zoom * scale).max(1.);
        let left = (x - radius).floor().max(0.) as usize;
        let right = ((x + radius).ceil() as usize).min(w as usize);
        let top = (y - radius).floor().max(0.) as usize;
        let bottom = ((y + radius).ceil() as usize).min(h as usize);
        let width = w as usize;
        let stride = (4. * scale).round().max(1.) as usize;
        // Alpha is preserved, so neighbouring coverage can be sampled in place.
        for py in top..bottom {
            for px in left..right {
                let idx = (py * width + px) * 4;
                let alpha = pixels[idx + 3] as f32;
                if alpha == 0. {
                    continue;
                }
                let dx = (x - px as f32) / radius;
                let dy = (y - py as f32) / radius;
                let dist = dx.hypot(dy);
                if dist >= 1. {
                    continue;
                }
                let coverage = |sx, sy| pixels[(sy * width + sx) * 4 + 3] as f32 / 255.;
                let nx = coverage(px.saturating_sub(stride), py)
                    - coverage((px + stride).min(width - 1), py);
                let ny = coverage(px, py.saturating_sub(stride))
                    - coverage(px, (py + stride).min(h as usize - 1));
                let shading = 1. - relief * 0.35 + relief * (nx * dx + ny * dy) * 1.4;
                let gain = (1. - dist).powi(2) * intensity * shading * opacity;
                for c in 0..3 {
                    let value = f32::from(pixels[idx + c]);
                    pixels[idx + c] = (value
                        + (alpha * 0.7 + value * 0.3) * gain * f32::from(color[c]) / 255.)
                        .clamp(0., alpha)
                        .round() as u8;
                }
            }
        }
    }
}

impl Editor {
    pub(crate) fn light_properties(&mut self, ui: &mut egui::Ui) {
        design::category(ui, "Point light");
        ui.small(tr("Illuminates the visible artwork below this track."));
        let light = self.doc.layers[self.layer].options.light.as_ref().unwrap();
        let original = light.at(self.frame);
        let mut value = original;
        let mut remove = false;
        let mut insert = false;
        ui.add_enabled_ui(!self.doc.layers[self.layer].options.locked, |ui| {
            for (icon, name, field, range) in [
                (
                    Icon::Focus,
                    "X",
                    &mut value.position[0],
                    -100_000. ..=100_000.,
                ),
                (Icon::Ellipse, "Radius", &mut value.radius, 1. ..=20_000.),
                (Icon::Bulb, "Intensity", &mut value.intensity, 0. ..=2.),
                (Icon::Depth, "Relief", &mut value.relief, 0. ..=1.),
            ] {
                control(ui, icon, name, field, range);
            }
            control(
                ui,
                Icon::Focus,
                "Y",
                &mut value.position[1],
                -100_000. ..=100_000.,
            );
            ui.horizontal(|ui| {
                ui.label(tr("Color"));
                ui.color_edit_button_srgb(&mut value.color);
            });
            ui.separator();
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
        let center = view.screen(crate::vector::pos(settings.position));
        let rect = Rect::from_center_size(center, Vec2::splat(27.));
        if !input {
            let color = crate::themes::accent();
            ui.painter().circle_stroke(
                center,
                settings.radius * view.scale,
                Stroke::new(1., color.gamma_multiply(0.45)),
            );
            ui.painter()
                .circle_filled(center, 13., crate::themes::panel());
            design::draw(ui.painter(), rect.shrink(2.), Icon::Bulb, color);
            return false;
        }
        if layer.options.locked {
            return false;
        }
        let response = ui
            .interact(
                rect,
                ui.scope_id().with("point-light-gizmo"),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::Grab);
        if response.drag_started() {
            self.checkpoint();
        }
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let point = view.world(pointer);
            let value = Settings {
                position: [
                    point.x.clamp(-100_000., 100_000.),
                    point.y.clamp(-100_000., 100_000.),
                ],
                ..settings
            };
            self.doc.layers[self.layer]
                .options
                .light
                .as_mut()
                .unwrap()
                .set(self.frame, value);
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
        let mut light = Light::new(64, 64);
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
