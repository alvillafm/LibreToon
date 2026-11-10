//! Non-destructive scene adjustment layers, applied to the accumulated image below each row.
use crate::{
    Document, Editor,
    design::{self, Icon, button},
    i18n::tr,
};
use eframe::egui::{self, Color32, Pos2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Hue,
    Levels,
    Curves,
    Glow,
    Vignette,
    Harmony,
    Gaussian,
    Lens,
    Sharpen,
    Exposure,
    Posterize,
}
impl Kind {
    const ALL: [Self; 11] = [
        Self::Hue,
        Self::Levels,
        Self::Curves,
        Self::Glow,
        Self::Vignette,
        Self::Harmony,
        Self::Gaussian,
        Self::Lens,
        Self::Sharpen,
        Self::Exposure,
        Self::Posterize,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Hue => "Hue / Saturation",
            Self::Levels => "Color levels",
            Self::Curves => "Color curves",
            Self::Glow => "Bloom / Glow",
            Self::Vignette => "Vignette",
            Self::Harmony => "Color integration",
            Self::Gaussian => "Gaussian blur",
            Self::Lens => "Lens blur",
            Self::Sharpen => "Sharpen",
            Self::Exposure => "Exposure / Contrast",
            Self::Posterize => "Posterize",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Hue => Icon::SwapColors,
            Self::Levels => Icon::Adjust,
            Self::Curves => Icon::Tween,
            Self::Glow => Icon::Wand,
            Self::Vignette => Icon::Ellipse,
            Self::Harmony => Icon::Eyedropper,
            Self::Gaussian => Icon::Blur,
            Self::Lens => Icon::Camera,
            Self::Sharpen => Icon::Focus,
            Self::Exposure => Icon::LightBrush,
            Self::Posterize => Icon::Layers,
        }
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    kind: Kind,
    enabled: bool,
    values: [f32; 3],
    color: [u8; 3],
    channels: [[f32; 5]; 4],
}
impl Effect {
    pub(crate) fn new(kind: Kind) -> Self {
        let values = match kind {
            Kind::Glow => [0.68, 18., 0.5],
            Kind::Vignette => [0.55, 0.58, 0.5],
            Kind::Harmony => [0.35, 0.75, 0.],
            Kind::Gaussian | Kind::Lens => [12., 0., 0.],
            Kind::Sharpen => [0.6, 3., 0.],
            Kind::Posterize => [8., 0., 0.],
            _ => [0.; 3],
        };
        let channels = if kind == Kind::Levels {
            [[0., 1., 1., 0., 1.]; 4]
        } else {
            [[0., 0.25, 0.5, 0.75, 1.]; 4]
        };
        Self {
            kind,
            enabled: true,
            values,
            channels,
            color: if kind == Kind::Harmony {
                [93, 148, 117]
            } else {
                [0; 3]
            },
        }
    }
}
pub fn valid(effects: &[Effect]) -> bool {
    effects.len() <= 16
        && effects.iter().all(|e| {
            e.values
                .iter()
                .chain(e.channels.iter().flatten())
                .all(|x| x.is_finite() && x.abs() <= 360.)
        })
}

pub fn is_adjustment(layer: &crate::document::Layer) -> bool {
    layer.options.adjustment.is_some() || layer.options.light.is_some()
}
pub fn present(doc: &Document) -> bool {
    doc.layers.iter().any(is_adjustment)
}
pub fn visible_effects(layer: &crate::document::Layer) -> bool {
    layer.visible
        && (layer.options.light.is_some()
            || layer
                .options
                .adjustment
                .as_ref()
                .is_some_and(|effects| effects.iter().any(|e| e.enabled)))
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}
fn hue_saturation(rgb: [f32; 3], hue: f32, saturation: f32, light: f32) -> [f32; 3] {
    let max = rgb.into_iter().fold(0., f32::max);
    let min = rgb.into_iter().fold(1., f32::min);
    let delta = max - min;
    let old_l = (max + min) * 0.5;
    let s = if delta > 0.00001 {
        delta / (1. - (2. * old_l - 1.).abs()).max(0.00001)
    } else {
        0.
    };
    let h = if delta <= 0.00001 {
        0.
    } else if max == rgb[0] {
        (rgb[1] - rgb[2]) / delta
    } else if max == rgb[1] {
        2. + (rgb[2] - rgb[0]) / delta
    } else {
        4. + (rgb[0] - rgb[1]) / delta
    };
    let h = (h + hue / 60.).rem_euclid(6.);
    let s = (s * (1. + saturation)).clamp(0., 1.);
    let l = (old_l + light).clamp(0., 1.);
    let chroma = (1. - (2. * l - 1.).abs()) * s;
    let x = chroma * (1. - (h.rem_euclid(2.) - 1.).abs());
    let rgb = match h as u32 {
        0 => [chroma, x, 0.],
        1 => [x, chroma, 0.],
        2 => [0., chroma, x],
        3 => [0., x, chroma],
        4 => [x, 0., chroma],
        _ => [chroma, 0., x],
    };
    rgb.map(|v| v + l - chroma / 2.)
}
fn channel_value(kind: Kind, v: f32, p: [f32; 5]) -> f32 {
    if kind == Kind::Levels {
        let t = ((v - p[0]) / (p[1] - p[0]).max(0.001))
            .clamp(0., 1.)
            .powf(1. / p[2].clamp(0.1, 5.));
        (p[3] + (p[4] - p[3]) * t).clamp(0., 1.)
    } else {
        let x = v.clamp(0., 1.) * 4.;
        let i = (x as usize).min(3);
        (p[i] + (p[i + 1] - p[i]) * (x - i as f32)).clamp(0., 1.)
    }
}
fn smooth(x: f32) -> f32 {
    let t = x.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub fn apply(pixels: &mut Vec<u8>, w: u32, h: u32, effects: &[Effect], scale: f32) {
    // Stored in application order. The inspector displays the top effect first.
    for effect in effects.iter().filter(|e| e.enabled) {
        let [a, b, c] = effect.values;
        let kind = effect.kind;
        let blur = |input: Vec<u8>, radius: f32, method| {
            crate::depth::blur(
                input,
                w,
                h,
                (radius * scale).clamp(0., 128.),
                &crate::depth::Optics {
                    method,
                    quality: crate::depth::Quality::High,
                    ..Default::default()
                },
            )
        };
        if matches!(kind, Kind::Gaussian | Kind::Lens) {
            *pixels = blur(
                std::mem::take(pixels),
                a,
                if kind == Kind::Lens {
                    crate::depth::BlurMethod::Lens
                } else {
                    crate::depth::BlurMethod::Gaussian
                },
            );
            continue;
        }
        if kind == Kind::Glow {
            let mut bright = pixels.clone();
            for p in bright.chunks_exact_mut(4) {
                let alpha = f32::from(p[3]).max(1.);
                let strength = smooth(
                    (luma([
                        f32::from(p[0]) / alpha,
                        f32::from(p[1]) / alpha,
                        f32::from(p[2]) / alpha,
                    ]) - a)
                        / (1. - a).max(0.01),
                );
                for v in p {
                    *v = (f32::from(*v) * strength).round() as u8;
                }
            }
            let glow = blur(bright, b, crate::depth::BlurMethod::Gaussian);
            for (p, g) in pixels.chunks_exact_mut(4).zip(glow.chunks_exact(4)) {
                let ga = f32::from(g[3]) * c.clamp(0., 2.);
                let alpha = (f32::from(p[3]) + ga * (1. - f32::from(p[3]) / 255.)).min(255.);
                for i in 0..3 {
                    p[i] = (f32::from(p[i]) + f32::from(g[i]) * c.clamp(0., 2.))
                        .min(alpha)
                        .round() as u8;
                }
                p[3] = alpha.round() as u8;
            }
            continue;
        }
        let blurred = (kind == Kind::Sharpen)
            .then(|| blur(pixels.clone(), b, crate::depth::BlurMethod::Gaussian));
        let tint = effect.color.map(|v| f32::from(v) / 255.);
        for (index, p) in pixels.chunks_exact_mut(4).enumerate() {
            let alpha = f32::from(p[3]);
            if alpha == 0. {
                continue;
            }
            let rgb = [
                f32::from(p[0]) / alpha,
                f32::from(p[1]) / alpha,
                f32::from(p[2]) / alpha,
            ];
            let out = match kind {
                Kind::Hue => hue_saturation(rgb, a, b, c),
                Kind::Levels | Kind::Curves => std::array::from_fn(|i| {
                    channel_value(
                        kind,
                        channel_value(kind, rgb[i], effect.channels[i + 1]),
                        effect.channels[0],
                    )
                }),
                Kind::Vignette => {
                    let x = 2. * (index as u32 % w) as f32 / w as f32 - 1.;
                    let y = 2. * (index as u32 / w) as f32 / h as f32 - 1.;
                    let edge = x.hypot(y) / std::f32::consts::SQRT_2;
                    let strength = smooth((edge - b) / (c.max(0.01))) * a;
                    std::array::from_fn(|i| rgb[i] + (tint[i] - rgb[i]) * strength)
                }
                Kind::Harmony => {
                    let y = luma(rgb);
                    let tint_luma = luma(tint);
                    std::array::from_fn(|i| {
                        let target = (tint[i] + (y - tint_luma) * b).clamp(0., 1.);
                        rgb[i] + (target - rgb[i]) * a
                    })
                }
                Kind::Sharpen => {
                    let low = &blurred.as_ref().unwrap()[index * 4..index * 4 + 4];
                    let low_alpha = f32::from(low[3]).max(1.);
                    std::array::from_fn(|i| rgb[i] + a * (rgb[i] - f32::from(low[i]) / low_alpha))
                }
                Kind::Exposure => rgb.map(|v| (v * 2f32.powf(a) - 0.5) * (1. + b) + 0.5),
                Kind::Posterize => {
                    let steps = a.round().clamp(2., 32.) - 1.;
                    rgb.map(|v| (v * steps).round() / steps)
                }
                _ => rgb,
            };
            for i in 0..3 {
                p[i] = (out[i].clamp(0., 1.) * alpha).round() as u8;
            }
        }
    }
}

fn slider(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.), egui::Sense::hover());
        design::draw(ui.painter(), rect, icon, crate::themes::muted());
        ui.add(egui::Slider::new(value, range).text(tr(label)));
    });
}
fn curve(ui: &mut egui::Ui, values: &mut [f32; 5], color: Color32) {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(220.), 112.),
        egui::Sense::click_and_drag(),
    );
    let graph = rect.shrink(7.);
    ui.painter()
        .rect_filled(rect, 4., crate::themes::palette().canvas);
    for i in 0..5 {
        let t = i as f32 / 4.;
        let x = egui::lerp(graph.x_range(), t);
        let y = egui::lerp(graph.y_range(), t);
        ui.painter().vline(
            x,
            graph.y_range(),
            egui::Stroke::new(1., crate::themes::line()),
        );
        ui.painter().hline(
            graph.x_range(),
            y,
            egui::Stroke::new(1., crate::themes::line()),
        );
    }
    let key = response.id.with("curve-control");
    if (response.drag_started() || response.clicked())
        && let Some(p) = response.interact_pointer_pos()
    {
        let index = (((p.x - graph.left()) / graph.width()) * 4.)
            .round()
            .clamp(0., 4.) as usize;
        ui.ctx().data_mut(|data| data.insert_temp(key, index));
    }
    if (response.dragged() || response.clicked())
        && let Some(p) = response.interact_pointer_pos()
    {
        let index = ui
            .ctx()
            .data(|data| data.get_temp::<usize>(key))
            .unwrap_or(0);
        values[index] = (1. - (p.y - graph.top()) / graph.height()).clamp(0., 1.);
    }
    let points: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Pos2::new(
                egui::lerp(graph.x_range(), i as f32 / 4.),
                egui::lerp(graph.y_range(), 1. - v),
            )
        })
        .collect();
    ui.painter().add(egui::Shape::line(
        points.clone(),
        egui::Stroke::new(2., color),
    ));
    for p in points {
        ui.painter().circle_filled(p, 3.5, color);
    }
}
fn settings(ui: &mut egui::Ui, effect: &mut Effect, channel: &mut usize) {
    let [a, b, c] = &mut effect.values;
    match effect.kind {
        Kind::Hue => {
            slider(ui, Icon::Rotate, "Hue", a, -180. ..=180.);
            slider(ui, Icon::SwapColors, "Saturation", b, -1. ..=1.);
            slider(ui, Icon::LightBrush, "Lightness", c, -1. ..=1.);
        }
        Kind::Levels | Kind::Curves => {
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["Master", "Red", "Green", "Blue"].into_iter().enumerate() {
                    ui.selectable_value(channel, i, tr(name));
                }
            });
            let p = &mut effect.channels[*channel];
            if effect.kind == Kind::Curves {
                curve(
                    ui,
                    p,
                    [
                        crate::themes::accent(),
                        Color32::LIGHT_RED,
                        Color32::LIGHT_GREEN,
                        Color32::LIGHT_BLUE,
                    ][*channel],
                );
            } else {
                for (value, name, range) in [
                    (0, "Input black", 0. ..=0.99),
                    (1, "Input white", 0.01..=1.),
                    (2, "Gamma", 0.1..=5.),
                    (3, "Output black", 0. ..=1.),
                    (4, "Output white", 0. ..=1.),
                ] {
                    slider(ui, Icon::Adjust, name, &mut p[value], range);
                }
                p[1] = p[1].max(p[0] + 0.01).min(1.);
            }
        }
        Kind::Glow => {
            slider(ui, Icon::LightBrush, "Brightness threshold", a, 0. ..=1.);
            slider(ui, Icon::Blur, "Radius", b, 1. ..=100.);
            slider(ui, Icon::Wand, "Intensity", c, 0. ..=2.);
        }
        Kind::Vignette => {
            slider(ui, Icon::Ellipse, "Intensity", a, 0. ..=1.);
            slider(ui, Icon::Focus, "Start radius", b, 0. ..=1.);
            slider(ui, Icon::Blur, "Softness", c, 0.01..=0.9);
            ui.color_edit_button_srgb(&mut effect.color)
                .on_hover_text(tr("Vignette color"));
        }
        Kind::Harmony => {
            ui.color_edit_button_srgb(&mut effect.color)
                .on_hover_text(tr("Dominant color"));
            slider(ui, Icon::Eyedropper, "Tint amount", a, 0. ..=1.);
            slider(ui, Icon::LightBrush, "Preserve lighting", b, 0. ..=1.);
        }
        Kind::Gaussian | Kind::Lens => slider(ui, effect.kind.icon(), "Radius", a, 0. ..=100.),
        Kind::Sharpen => {
            slider(ui, Icon::Focus, "Intensity", a, 0. ..=2.);
            slider(ui, Icon::Blur, "Radius", b, 0.5..=12.);
        }
        Kind::Exposure => {
            slider(ui, Icon::LightBrush, "Exposure", a, -3. ..=3.);
            slider(ui, Icon::Adjust, "Contrast", b, -1. ..=1.);
        }
        Kind::Posterize => slider(ui, Icon::Layers, "Color steps", a, 2. ..=32.),
    }
}
#[derive(Clone, Copy)]
struct EffectDrag(usize);

#[derive(Default)]
pub(crate) struct State {
    pub hidden: bool,
    selected: usize,
    channel: usize,
    transaction: bool,
}
impl Editor {
    pub(crate) fn adjustment_active(&self) -> bool {
        self.doc.layers.get(self.layer).is_some_and(is_adjustment)
    }

    pub(crate) fn adjustment_properties(&mut self, ui: &mut egui::Ui) {
        let Some(effects) = self.doc.layers[self.layer].options.adjustment.as_ref() else {
            return;
        };
        let before = effects.clone();
        let mut effects = before.clone();
        let locked = self.doc.layers[self.layer].options.locked;
        ui.heading(tr("Adjustment layer"));
        ui.small(tr("Effects apply to all visible layers below this layer."));
        ui.small(tr("Effects run from bottom to top. Drag to reorder."));
        ui.separator();
        ui.add_enabled_ui(!locked, |ui| {
            ui.horizontal(|ui| {
                let add = button(ui, Icon::Plus, "", "Add effect", false, effects.len() < 16);
                egui::Popup::menu(&add).show(|ui| {
                    for kind in Kind::ALL {
                        if button(ui, kind.icon(), kind.name(), "", false, effects.len() < 16)
                            .clicked()
                        {
                            effects.push(Effect::new(kind));
                            self.post.selected = effects.len() - 1;
                            self.post.channel = 0;
                            ui.close();
                        }
                    }
                });
                if button(
                    ui,
                    Icon::Minus,
                    "",
                    "Remove selected effect",
                    false,
                    !effects.is_empty(),
                )
                .clicked()
                {
                    let selected = self.post.selected.min(effects.len() - 1);
                    effects.remove(selected);
                    self.post.selected = self.post.selected.saturating_sub(1);
                }
            });
            let mut reorder = None;
            for (i, effect) in effects.iter_mut().enumerate().rev() {
                let row = ui
                    .horizontal(|ui| {
                        if button(
                            ui,
                            if effect.enabled {
                                Icon::Eye
                            } else {
                                Icon::EyeOff
                            },
                            "",
                            "Show / Hide effect",
                            effect.enabled,
                            true,
                        )
                        .clicked()
                        {
                            effect.enabled = !effect.enabled;
                        }
                        let selected = i == self.post.selected;
                        let response = button(
                            ui,
                            effect.kind.icon(),
                            effect.kind.name(),
                            "",
                            selected,
                            true,
                        );
                        let drag = response
                            .interact(egui::Sense::drag())
                            .on_hover_cursor(egui::CursorIcon::Grab);
                        if drag.clicked() || drag.drag_started() {
                            self.post.selected = i;
                            self.post.channel = 0;
                        }
                        if drag.drag_started() {
                            drag.dnd_set_drag_payload(EffectDrag(i));
                        }
                    })
                    .response;
                if !locked
                    && let Some(source) = row.dnd_hover_payload::<EffectDrag>()
                    && let Some(pos) = ui.ctx().pointer_interact_pos()
                {
                    let before = pos.y < row.rect.center().y;
                    let y = if before {
                        row.rect.top()
                    } else {
                        row.rect.bottom()
                    };
                    ui.painter().hline(
                        row.rect.x_range(),
                        y,
                        egui::Stroke::new(2., crate::themes::accent()),
                    );
                    if row.dnd_release_payload::<EffectDrag>().is_some() {
                        reorder = Some((source.0, i + usize::from(before)));
                    }
                }
            }
            if let Some((from, slot)) = reorder
                && from < effects.len()
            {
                let to = slot - usize::from(from < slot);
                if from != to {
                    let effect = effects.remove(from);
                    effects.insert(to, effect);
                }
                self.post.selected = to;
            }
            let len = effects.len();
            if let Some(effect) = effects.get_mut(self.post.selected.min(len.saturating_sub(1))) {
                ui.separator();
                settings(ui, effect, &mut self.post.channel);
                if button(ui, Icon::ResetColors, "Reset effect", "", false, true).clicked() {
                    *effect = Effect {
                        enabled: effect.enabled,
                        ..Effect::new(effect.kind)
                    };
                }
            }
        });
        if *effects != *before {
            if !self.post.transaction {
                self.checkpoint();
                self.post.transaction = true;
            }
            self.doc.layers[self.layer].options.adjustment = Some(effects);
            self.recover();
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.post.transaction = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjustment_roundtrips_and_only_affects_layers_below_it() {
        let mut doc = Document {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let mut upper = doc.layers[0].clone();
        let mut adjustment = upper.clone();
        let red_rectangle = |left, right| crate::Mark {
            points: vec![[left, 10.], [right, 10.], [right, 54.], [left, 54.]].into(),
            fill: Some([255, 0, 0, 255]),
            stroke: false,
            closed: true,
            ..Default::default()
        };
        doc.layers[0].frames[0].push(red_rectangle(4., 30.));
        upper.frames[0].push(red_rectangle(34., 60.));
        let mut hue = Effect::new(Kind::Hue);
        hue.values[0] = 120.;
        adjustment.options.adjustment = Some(vec![hue].into());
        doc.layers.push(adjustment);
        doc.layers.push(upper);

        let bytes = crate::native_project::encode(&doc).unwrap();
        let mut doc = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        let render = |doc: &Document| {
            crate::exporter::raster(
                doc,
                0,
                64,
                64,
                &egui::Context::default(),
                &mut Default::default(),
            )
        };
        let left = (32 * 64 + 16) * 3;
        let right = (32 * 64 + 48) * 3;
        let pixels = render(&doc);
        assert_eq!(&pixels[left..left + 3], &[0, 255, 0]);
        assert_eq!(&pixels[right..right + 3], &[255, 0, 0]);

        doc.layers[1].visible = false;
        let pixels = render(&doc);
        assert_eq!(&pixels[left..left + 3], &[255, 0, 0]);
    }
}
