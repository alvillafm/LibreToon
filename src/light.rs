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
    Ambient,
    Rim,
    RimShadow,
    Spot,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Point => "Point light",
            Self::Directional => "Directional light",
            Self::Ambient => "Ambient light",
            Self::Rim => "Rim light",
            Self::RimShadow => "Rim shadow",
            Self::Spot => "Spot light",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Gobo {
    None,
    Blinds,
    Window,
    Leaves,
    Library,
}

impl Gobo {
    fn name(self) -> &'static str {
        match self {
            Self::None => "No gobo",
            Self::Blinds => "Blinds",
            Self::Window => "Window",
            Self::Leaves => "Leaves",
            Self::Library => "Library symbol",
        }
    }

    fn transmission(self, along: f32, across: f32, offset: f32) -> f32 {
        let x = across + offset;
        match self {
            Self::None | Self::Library => 1.,
            Self::Blinds => {
                if (x * 9.).rem_euclid(1.) < 0.55 {
                    1.
                } else {
                    0.12
                }
            }
            Self::Window => {
                if x.abs() < 0.035 || (along - 0.52).abs() < 0.035 {
                    0.08
                } else {
                    1.
                }
            }
            Self::Leaves => {
                let pattern = (x * 67. + along * 33.).sin() * (along * 54. - x * 24.).sin();
                (0.65 + pattern * 0.43).clamp(0.12, 1.)
            }
        }
    }
}

const GOBO_SIZE: usize = 48;

fn make_gobo(doc: &crate::Document, id: u64) -> Option<SharedVec<u8>> {
    use geo::{BoundingRect, Contains};
    let symbol = doc.extras.symbols.iter().find(|symbol| symbol.id == id)?;
    let mut sample = doc.clone();
    sample.layers = symbol.layers.clone();
    sample.extras.rig = symbol.rig.clone();
    sample.extras.folders = symbol.folders.clone();
    sample.extras.scenes = Default::default();
    let marks: Vec<_> = crate::features::resolve_export(&sample, 0, false)
        .into_iter()
        .map(|(_, _, mark)| mark)
        .filter(|mark| {
            mark.style.media_image.is_none() && !mark.style.raster_tile && !mark.style.light_mask
        })
        .take(513)
        .collect();
    if marks.len() > 512 {
        return None;
    }
    let silhouette = crate::features::ink(&marks);
    let bounds = silhouette.bounding_rect()?;
    let extent = bounds.width().max(bounds.height()) * 1.1;
    if extent <= 0. {
        return None;
    }
    let center = bounds.center();
    Some(
        (0..GOBO_SIZE * GOBO_SIZE)
            .map(|i| {
                let x = ((i % GOBO_SIZE) as f64 + 0.5) / GOBO_SIZE as f64 - 0.5;
                let y = ((i / GOBO_SIZE) as f64 + 0.5) / GOBO_SIZE as f64 - 0.5;
                if silhouette.contains(&geo::Point::new(
                    center.x + x * extent,
                    center.y + y * extent,
                )) {
                    255
                } else {
                    0
                }
            })
            .collect(),
    )
}

fn sample_gobo(matte: &[u8], along: f32, across: f32) -> f32 {
    let x = (across + 0.5) * (GOBO_SIZE - 1) as f32;
    let y = along * (GOBO_SIZE - 1) as f32;
    if !(0. ..=(GOBO_SIZE - 1) as f32).contains(&x) || !(0. ..=(GOBO_SIZE - 1) as f32).contains(&y)
    {
        return 0.;
    }
    let (ix, iy) = (
        (x as usize).min(GOBO_SIZE - 2),
        (y as usize).min(GOBO_SIZE - 2),
    );
    let tx = x - ix as f32;
    let ty = y - iy as f32;
    let row = |row| {
        let a = f32::from(matte[row * GOBO_SIZE + ix]);
        let b = f32::from(matte[row * GOBO_SIZE + ix + 1]);
        egui::lerp(a..=b, tx)
    };
    egui::lerp(row(iy)..=row(iy + 1), ty) / 255.
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
    cone: f32,
    softness: f32,
    preserve_lines: bool,
    gobo: Gobo,
    gobo_offset: f32,
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
            cone: 55.,
            softness: 0.3,
            preserve_lines: true,
            gobo: Gobo::None,
            gobo_offset: 0.,
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
            && (4. ..=170.).contains(&self.cone)
            && (0. ..=1.).contains(&self.softness)
            && (-2. ..=2.).contains(&self.gobo_offset)
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Receiver {
    Layer(u64),
    Folder(u64),
}

impl Receiver {
    pub(crate) fn matches(self, doc: &crate::Document, layer: &crate::document::Layer) -> bool {
        match self {
            Self::Layer(id) => layer.options.light_receiver == Some(id),
            Self::Folder(id) => {
                crate::folder_groups::belongs(&doc.extras.folders, layer.options.folder, id)
            }
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub floor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) caster: Option<u64>,
    offset: [f32; 2],
    softness: f32,
    strength: f32,
    #[serde(default)]
    follow_light: bool,
    #[serde(default = "shadow_height")]
    height: f32,
}

fn shadow_height() -> f32 {
    64.
}

impl Shadow {
    pub fn apply(&self, floor: &mut [u8], mask: Vec<u8>, w: u32, h: u32, scale: f32, opacity: f32) {
        self.apply_transformed(floor, mask, w, h, scale, opacity, ([0., 0.], 1.));
    }
    fn apply_transformed(
        &self,
        floor: &mut [u8],
        mask: Vec<u8>,
        w: u32,
        h: u32,
        scale: f32,
        opacity: f32,
        projection: ([f32; 2], f32),
    ) {
        let mask = if self.softness > 0. {
            crate::depth::blur(
                mask,
                w,
                h,
                (self.softness * scale).min(128.),
                &crate::depth::Optics::default(),
            )
        } else {
            mask
        };
        let dx = (self.offset[0] * scale).round() as isize;
        let dy = (self.offset[1] * scale).round() as isize;
        let mut config = [0u32; 48];
        config[..7].copy_from_slice(&[w, h, 0, 0, w, h, 6]);
        for (i, value) in [
            dx as f32,
            dy as f32,
            projection.1,
            projection.0[0],
            projection.0[1],
            self.strength * opacity,
        ]
        .into_iter()
        .enumerate()
        {
            config[16 + i] = value.to_bits();
        }
        if crate::gpu_lens::light_on_worker(
            floor,
            &crate::gpu_light::LightDispatch {
                config,
                surface: None,
                mask: Some(&mask),
                gobo: None,
            },
        ) {
            return;
        }
        let w = w as usize;
        let h = h as usize;
        for (i, pixel) in floor.chunks_exact_mut(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let sx =
                ((i % w) as f32 - dx as f32 - projection.0[0]) / projection.1 + projection.0[0];
            let sy =
                ((i / w) as f32 - dy as f32 - projection.0[1]) / projection.1 + projection.0[1];
            let Some(x) = (sx.round() as isize)
                .try_into()
                .ok()
                .filter(|x: &usize| *x < w)
            else {
                continue;
            };
            let Some(y) = (sy.round() as isize)
                .try_into()
                .ok()
                .filter(|y: &usize| *y < h)
            else {
                continue;
            };
            let darkness = f32::from(mask[(y * w + x) * 4 + 3]) / 255. * self.strength * opacity;
            for channel in &mut pixel[..3] {
                *channel = (f32::from(*channel) * (1. - darkness)).round() as u8;
            }
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
enum MaskMode {
    #[default]
    PaintLight,
    Protect,
    Ignore,
}

impl MaskMode {
    fn name(self) -> &'static str {
        match self {
            Self::PaintLight => "Light painted areas",
            Self::Protect => "Protect painted areas",
            Self::Ignore => "Ignore light masks",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Preset {
    name: String,
    size: [u32; 2],
    lights: Vec<PresetLight>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PresetLight {
    name: String,
    light: Light,
    opacity: f32,
    visible: bool,
}

impl Preset {
    pub(crate) fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.chars().count() <= 48
            && self
                .size
                .iter()
                .all(|dimension| (1..=16384).contains(dimension))
            && (1..=100).contains(&self.lights.len())
            && self.lights.iter().all(|item| {
                item.name.chars().count() <= 256
                    && (0. ..=1.).contains(&item.opacity)
                    && item.light.valid()
            })
    }
}

#[derive(Clone, Copy)]
enum PresetTarget {
    All,
    Layer(usize),
    Folder(u64),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    #[serde(default)]
    pub kind: Kind,
    #[serde(default)]
    mask_mode: MaskMode,
    #[serde(default)]
    mask_feather: f32,
    #[serde(default, skip_serializing_if = "SharedVec::is_empty")]
    pub receivers: SharedVec<Receiver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gobo_matte: Option<SharedVec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gizmo_position: Option<[f32; 2]>,
    base: Settings,
    #[serde(default)]
    keys: SharedVec<(usize, Settings)>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    key_easing: std::sync::Arc<std::collections::BTreeMap<usize, crate::animation::Curve>>,
}

#[derive(PartialEq)]
pub(crate) struct ShadingKey(Kind, Settings, MaskMode, f32, Option<SharedVec<u8>>);

pub(crate) struct NativeSettings {
    pub config: [u32; 48],
    pub bevel: f32,
    pub texture: f32,
    pub relief: bool,
    pub feather: f32,
    pub gobo: Option<SharedVec<u8>>,
}
pub(crate) struct NativeShadow {
    pub floor: u64,
    pub caster: Option<u64>,
    pub softness: f32,
    pub values: [f32; 6],
}

impl Light {
    pub(crate) fn native_shadow(
        &self,
        w: u32,
        h: u32,
        padding: u32,
        doc: &crate::Document,
        frame: usize,
        camera: bool,
        opacity: f32,
    ) -> Option<NativeShadow> {
        let shadow = self.shadow.as_ref()?;
        let scale = ((w - padding * 2) as f32 / doc.width as f32)
            .min((h - padding * 2) as f32 / doc.height as f32);
        let mut offset = shadow.offset;
        let mut center = Pos2::ZERO;
        let mut expansion = 1.;
        if shadow.follow_light {
            let settings = self.at(frame);
            let doc_center = Pos2::new(doc.width as f32 * 0.5, doc.height as f32 * 0.5);
            let mut point = crate::vector::pos(settings.position);
            let mut zoom = 1.;
            let mut angle = settings.angle.to_radians();
            if camera && let Some(track) = crate::depth::camera(doc, frame) {
                let pose = crate::animation::camera_at(track, frame);
                zoom = pose.zoom;
                point = doc_center
                    + egui::emath::Rot2::from_angle(-pose.angle)
                        * (point - crate::vector::pos(pose.center))
                        * zoom;
                angle -= pose.angle;
            }
            center = Pos2::new(w as f32 * 0.5, h as f32 * 0.5) + (point - doc_center) * scale;
            if matches!(self.kind, Kind::Point | Kind::Spot) {
                let height = (settings.radius * settings.elevation.to_radians().sin())
                    .max(shadow.height + 1.);
                expansion = (height / (height - shadow.height)).clamp(1., 8.);
            } else {
                let delta = -Vec2::angled(angle)
                    * (shadow.height / settings.elevation.to_radians().tan().max(0.05))
                    * zoom;
                offset[0] += delta.x;
                offset[1] += delta.y;
            }
        }
        Some(NativeShadow {
            floor: shadow.floor,
            caster: shadow.caster,
            softness: (shadow.softness * scale).min(128.),
            values: [
                (offset[0] * scale).round(),
                (offset[1] * scale).round(),
                expansion,
                center.x,
                center.y,
                shadow.strength * opacity,
            ],
        })
    }
    pub(crate) fn native_settings(
        &self,
        frame: usize,
        size: [u32; 2],
        view: crate::canvas::View,
        step: usize,
        opacity: f32,
        masked: bool,
    ) -> NativeSettings {
        let s = self.at(frame);
        let p = view.screen(crate::vector::pos(s.position));
        let radius = (s.radius * view.scale).max(1.);
        let angle = s.angle.to_radians();
        let aim = Vec2::angled(angle);
        let elevation = s.elevation.to_radians();
        let direction = aim * elevation.cos();
        let half = s.cone.to_radians() * 0.5;
        let outer = half.cos();
        let inner = (half * (1. - s.softness)).cos();
        let mut config = [0u32; 48];
        config[..16].copy_from_slice(&[
            size[0],
            size[1],
            0,
            0,
            size[0],
            size[1],
            self.kind as u32,
            if !masked || !self.uses_mask() {
                0
            } else if self.mask_mode == MaskMode::Protect {
                2
            } else {
                1
            },
            if s.relief > 0. && self.kind != Kind::Ambient {
                size[0].div_ceil(step as u32)
            } else {
                0
            },
            size[1].div_ceil(step as u32),
            step as u32,
            u32::from(s.steps),
            u32::from(s.preserve_lines),
            s.gobo as u32,
            u32::from(self.gobo_matte.is_some()),
            0,
        ]);
        for (i, v) in [
            p.x,
            p.y,
            radius,
            direction.x,
            direction.y,
            elevation.sin().max(0.08),
            s.relief,
            s.intensity,
            s.shadow,
            s.specular,
            s.gloss,
            s.falloff,
            f32::from(s.color[0]) / 255.,
            f32::from(s.color[1]) / 255.,
            f32::from(s.color[2]) / 255.,
            s.softness,
            aim.x,
            aim.y,
            outer,
            (inner - outer).max(0.0001),
            s.gobo_offset,
            opacity,
        ]
        .into_iter()
        .enumerate()
        {
            config[16 + i] = v.to_bits();
        }
        NativeSettings {
            config,
            bevel: s.bevel * view.scale,
            texture: s.texture_relief,
            relief: s.relief > 0. && self.kind != Kind::Ambient,
            feather: self.mask_feather * view.scale,
            gobo: self.gobo_matte.clone(),
        }
    }
    pub(crate) fn apply_shadow(
        &self,
        floor: &mut [u8],
        mask: Vec<u8>,
        w: u32,
        h: u32,
        padding: u32,
        doc: &crate::Document,
        frame: usize,
        camera: bool,
        opacity: f32,
    ) {
        let Some(shadow) = &self.shadow else {
            return;
        };
        let scale = ((w - padding * 2) as f32 / doc.width as f32)
            .min((h - padding * 2) as f32 / doc.height as f32);
        if !shadow.follow_light {
            shadow.apply(floor, mask, w, h, scale, opacity);
            return;
        }
        let settings = self.at(frame);
        let mut projected = shadow.clone();
        let mut center = Pos2::new(doc.width as f32 * 0.5, doc.height as f32 * 0.5);
        let mut point = crate::vector::pos(settings.position);
        let mut zoom = 1.;
        let mut angle = settings.angle.to_radians();
        if camera && let Some(track) = crate::depth::camera(doc, frame) {
            let pose = crate::animation::camera_at(track, frame);
            zoom = pose.zoom;
            point = center
                + egui::emath::Rot2::from_angle(-pose.angle)
                    * (point - crate::vector::pos(pose.center))
                    * zoom;
            angle -= pose.angle;
        }
        point = Pos2::new(w as f32 * 0.5, h as f32 * 0.5) + (point - center) * scale;
        center = point;
        let expansion = if matches!(self.kind, Kind::Point | Kind::Spot) {
            let lamp_height =
                (settings.radius * settings.elevation.to_radians().sin()).max(shadow.height + 1.);
            (lamp_height / (lamp_height - shadow.height)).clamp(1., 8.)
        } else {
            let delta = -Vec2::angled(angle)
                * (shadow.height / settings.elevation.to_radians().tan().max(0.05))
                * zoom;
            projected.offset[0] += delta.x;
            projected.offset[1] += delta.y;
            1.
        };
        projected.apply_transformed(
            floor,
            mask,
            w,
            h,
            scale,
            opacity,
            ([center.x, center.y], expansion),
        );
    }
    pub(crate) fn batchable_at(&self, frame: usize) -> bool {
        self.at(frame).texture_relief == 0.
    }
    pub(crate) fn shading_key(&self, frame: usize) -> ShadingKey {
        ShadingKey(
            self.kind,
            self.at(frame),
            self.mask_mode,
            self.mask_feather,
            self.gobo_matte.clone(),
        )
    }

    pub fn new(w: u32, h: u32, kind: Kind) -> Self {
        Self {
            kind,
            mask_mode: Default::default(),
            mask_feather: 0.,
            receivers: Default::default(),
            shadow: None,
            gobo_matte: None,
            gizmo_position: None,
            base: Settings {
                position: [w as f32 * 0.5, h as f32 * 0.4],
                radius: (w.min(h) as f32 * 0.42).max(1.),
                intensity: if kind == Kind::Ambient { 0.25 } else { 0.75 },
                color: if kind == Kind::RimShadow {
                    [65, 45, 112]
                } else {
                    Settings::default().color
                },
                ..Settings::default()
            },
            keys: Default::default(),
            key_easing: Default::default(),
        }
    }

    pub fn valid(&self) -> bool {
        self.base.valid()
            && (0. ..=128.).contains(&self.mask_feather)
            && self.receivers.len() <= 100
            && self
                .gobo_matte
                .as_ref()
                .is_none_or(|matte| matte.len() == GOBO_SIZE * GOBO_SIZE)
            && self.shadow.as_ref().is_none_or(|shadow| {
                (0. ..=1.).contains(&shadow.strength)
                    && (0. ..=1024.).contains(&shadow.height)
                    && (0. ..=128.).contains(&shadow.softness)
                    && shadow.offset.iter().all(|x| (-1024. ..=1024.).contains(x))
            })
            && self
                .gizmo_position
                .is_none_or(|point| point.iter().all(|v| v.is_finite() && v.abs() <= 100_000.))
            && self.keys.len() <= 10_000
            && self
                .keys
                .iter()
                .all(|(frame, settings)| *frame < 10_000 && settings.valid())
            && self.keys.windows(2).all(|pair| pair[0].0 < pair[1].0)
            && self.key_easing.len() <= self.keys.len() + 1
            && self
                .key_easing
                .iter()
                .all(|(&frame, curve)| (frame == 0 || self.keyed(frame)) && curve.valid())
    }

    pub(crate) fn timing_segment(
        &self,
        frame: usize,
    ) -> Option<(usize, usize, Option<&crate::animation::Curve>)> {
        let next = self.keys.partition_point(|key| key.0 <= frame);
        let end = self.keys.get(next)?.0;
        let start = next.checked_sub(1).map_or(0, |i| self.keys[i].0);
        Some((start, end, self.key_easing.get(&start)))
    }

    pub(crate) fn set_timing(&mut self, frame: usize, curve: crate::animation::Curve) {
        std::sync::Arc::make_mut(&mut self.key_easing).insert(frame, curve);
    }

    pub(crate) fn demo_orbit(w: u32, h: u32, color: [u8; 3], phase: f32) -> Self {
        let mut light = Self::new(w, h, Kind::Point);
        light.base.color = color;
        light.base.radius = 650.;
        light.base.intensity = 0.72;
        light.base.relief = 0.85;
        light.base.specular = 0.22;
        light.base.gloss = 32.;
        for frame in [0, 18, 36, 54, 71] {
            let angle = phase + frame as f32 / 71. * std::f32::consts::TAU;
            let settings = Settings {
                position: [
                    w as f32 * 0.5 + angle.cos() * 260.,
                    h as f32 * 0.5 + angle.sin() * 170.,
                ],
                ..light.base
            };
            if frame == 0 {
                light.base = settings;
            }
            light.set(frame, settings);
        }
        light
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
        let t = self.key_easing.get(&a).map_or(t, |curve| curve.time(t));
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
            cone: lerp(start.cone, end.cone),
            softness: lerp(start.softness, end.softness),
            preserve_lines: start.preserve_lines,
            gobo: start.gobo,
            gobo_offset: lerp(start.gobo_offset, end.gobo_offset),
            color: std::array::from_fn(|i| {
                lerp(start.color[i].into(), end.color[i].into()).round() as u8
            }),
        }
    }

    pub(crate) fn uses_mask(&self) -> bool {
        self.mask_mode != MaskMode::Ignore
    }

    pub fn receives(&self, doc: &crate::Document, layer: &crate::document::Layer) -> bool {
        self.receivers.is_empty()
            || self
                .receivers
                .iter()
                .any(|receiver| receiver.matches(doc, layer))
    }

    pub(crate) fn apply_padded(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        padding: u32,
        doc: &crate::Document,
        frame: usize,
        camera: bool,
        opacity: f32,
        height_map: Option<&[u8]>,
        mask: Option<&[u8]>,
        surfaces: Option<&mut SurfaceCache>,
    ) {
        let center = Pos2::new(doc.width as f32 * 0.5, doc.height as f32 * 0.5);
        let mut point = crate::vector::pos(self.at(frame).position);
        let (mut zoom, mut angle) = (1., 0.);
        if camera && let Some(camera) = crate::depth::camera(doc, frame) {
            let pose = crate::animation::camera_at(camera, frame);
            zoom = pose.zoom;
            angle = -pose.angle;
            point = center
                + egui::emath::Rot2::from_angle(angle)
                    * (point - crate::vector::pos(pose.center))
                    * zoom;
        }
        let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
        let (w, h) = (w + padding * 2, h + padding * 2);
        let point = Pos2::new(w as f32 * 0.5, h as f32 * 0.5) + (point - center) * scale;
        self.shade(
            pixels,
            w,
            h,
            frame,
            point,
            scale * zoom,
            angle,
            opacity,
            height_map,
            mask,
            None,
            surfaces,
        );
    }

    pub(crate) fn shade_region(
        &self,
        pixels: &mut [u8],
        size: [u32; 2],
        view: crate::canvas::View,
        surface_step: usize,
        frame: usize,
        opacity: f32,
        height_map: Option<&[u8]>,
        mask: Option<&[u8]>,
        surfaces: &mut SurfaceCache,
    ) {
        let point = view.screen(crate::vector::pos(self.at(frame).position));
        self.shade(
            pixels,
            size[0],
            size[1],
            frame,
            point,
            view.scale,
            0.,
            opacity,
            height_map,
            mask,
            Some(surface_step),
            Some(surfaces),
        );
    }

    pub(crate) fn region_padding(&self, frame: usize, scale: f32, step: usize) -> f32 {
        let settings = self.at(frame);
        let relief = if settings.relief > 0. && self.kind != Kind::Ambient {
            settings.bevel * scale + 27. * step as f32
        } else {
            0.
        };
        let feather = if self.uses_mask() {
            (self.mask_feather * scale).min(128.) * 3.
        } else {
            0.
        };
        relief.max(feather)
    }

    fn shade(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        frame: usize,
        point: Pos2,
        scale: f32,
        angle: f32,
        opacity: f32,
        height_map: Option<&[u8]>,
        mask: Option<&[u8]>,
        surface_step: Option<usize>,
        surfaces: Option<&mut SurfaceCache>,
    ) {
        let settings = self.at(frame);
        if settings.intensity == 0. || opacity == 0. {
            return;
        }
        let Pos2 { x, y } = point;
        let angle = settings.angle.to_radians() + angle;
        let radius = (settings.radius * scale).max(1.);
        let (left, top, right, bottom) = match self.kind {
            Kind::Point | Kind::Spot => (
                (x - radius).max(0.) as usize,
                (y - radius).max(0.) as usize,
                ((x + radius).ceil() as usize).min(w as usize),
                ((y + radius).ceil() as usize).min(h as usize),
            ),
            Kind::Directional | Kind::Ambient | Kind::Rim | Kind::RimShadow => {
                (0, 0, w as usize, h as usize)
            }
        };
        if left >= right || top >= bottom {
            return;
        }
        let width = w as usize;
        let mask = mask.filter(|_| self.uses_mask());
        let feathered = mask.filter(|_| self.mask_feather > 0.).map(|matte| {
            crate::depth::blur(
                matte.to_vec(),
                w,
                h,
                (self.mask_feather * scale).min(128.),
                &crate::depth::Optics::default(),
            )
        });
        let mask = feathered.as_deref().or(mask);
        let surface = (settings.relief > 0. && self.kind != Kind::Ambient).then(|| {
            let bevel = settings.bevel * scale;
            let step = surface_step.unwrap_or_else(|| width.max(h as usize).div_ceil(1024).max(1));
            if let Some(cache) = surfaces
                && height_map.is_none()
                && settings.texture_relief == 0.
            {
                return cache.get(pixels, width, h as usize, bevel, step);
            }
            std::sync::Arc::new(Surface::new(
                pixels,
                width,
                h as usize,
                bevel,
                settings.texture_relief,
                height_map,
                step,
            ))
        });
        let elevation = settings.elevation.to_radians();
        let aim = Vec2::angled(angle);
        let dir = aim * elevation.cos();
        let height = elevation.sin().max(0.08);
        let tint = settings.color.map(|c| f32::from(c) / 255.);
        let spot = (self.kind == Kind::Spot).then(|| {
            let half = settings.cone.to_radians() * 0.5;
            let outer = half.cos();
            let inner = (half * (1. - settings.softness)).cos();
            (Vec2::new(-aim.y, aim.x), outer, (inner - outer).max(0.0001))
        });
        let uses_specular = settings.specular > 0.
            && matches!(self.kind, Kind::Point | Kind::Directional | Kind::Spot);
        let stride = width * 4;
        if (right - left) * (bottom - top) >= 64 * 1024 || crate::gpu_lens::collecting_lights() {
            let mut config = [0u32; 48];
            config[..16].copy_from_slice(&[
                w,
                h,
                left as u32,
                top as u32,
                right as u32,
                bottom as u32,
                self.kind as u32,
                if mask.is_none() {
                    0
                } else if self.mask_mode == MaskMode::Protect {
                    2
                } else {
                    1
                },
                surface.as_ref().map_or(0, |s| s.width as u32),
                surface.as_ref().map_or(0, |s| s.height as u32),
                surface.as_ref().map_or(1, |s| s.step as u32),
                u32::from(settings.steps),
                u32::from(settings.preserve_lines),
                settings.gobo as u32,
                u32::from(self.gobo_matte.is_some()),
                0,
            ]);
            let half = settings.cone.to_radians() * 0.5;
            let outer = half.cos();
            let inner = (half * (1. - settings.softness)).cos();
            for (index, value) in [
                x,
                y,
                radius,
                dir.x,
                dir.y,
                height,
                settings.relief,
                settings.intensity,
                settings.shadow,
                settings.specular,
                settings.gloss,
                settings.falloff,
                tint[0],
                tint[1],
                tint[2],
                settings.softness,
                aim.x,
                aim.y,
                outer,
                (inner - outer).max(0.0001),
                settings.gobo_offset,
                opacity,
            ]
            .into_iter()
            .enumerate()
            {
                config[16 + index] = value.to_bits();
            }
            let dispatch = crate::gpu_light::LightDispatch {
                config,
                surface: surface.clone(),
                mask,
                gobo: self.gobo_matte.as_deref().map(|matte| matte.as_slice()),
            };
            if crate::gpu_lens::light_on_worker(pixels, &dispatch) {
                return;
            }
        }
        let shade_rows = |first_row: usize, rows: &mut [u8]| {
            for (row, pixels) in rows.chunks_exact_mut(stride).enumerate() {
                let py = first_row + row;
                for px in left..right {
                    let idx = px * 4;
                    let alpha = f32::from(pixels[idx + 3]);
                    if alpha == 0. {
                        continue;
                    }
                    let (lx, ly, lz, mut attenuation) = match self.kind {
                        Kind::Directional | Kind::Ambient | Kind::Rim | Kind::RimShadow => {
                            (dir.x, dir.y, height, 1.)
                        }
                        Kind::Point | Kind::Spot => {
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
                    if let Some((cross_axis, outer, transition)) = spot {
                        let ray = Vec2::new(px as f32 - x, py as f32 - y) / radius;
                        let along = ray.dot(aim);
                        let across = ray.dot(cross_axis);
                        let cos = along / ray.length().max(0.00001);
                        let edge = ((cos - outer) / transition).clamp(0., 1.);
                        attenuation *= edge
                            * edge
                            * (3. - 2. * edge)
                            * if settings.gobo == Gobo::Library {
                                self.gobo_matte.as_ref().map_or(1., |matte| {
                                    sample_gobo(matte, along, across + settings.gobo_offset)
                                })
                            } else {
                                settings
                                    .gobo
                                    .transmission(along, across, settings.gobo_offset)
                            };
                        if attenuation <= 0. {
                            continue;
                        }
                    }
                    let n = surface
                        .as_ref()
                        .map_or(Vec2::ZERO, |surface| surface.slope(px, py))
                        * settings.relief
                        * 3.2;
                    let nz = (1. + n.length_sq()).sqrt().recip();
                    let mut diffuse = match self.kind {
                        Kind::Ambient => 0.65,
                        Kind::Rim => ((n.x * lx + n.y * ly) * nz).max(0.) * (1. - nz).sqrt(),
                        Kind::RimShadow => {
                            let edge = (1. - nz).sqrt();
                            let strength = (-(n.x * lx + n.y * ly) * nz).max(0.) * edge * 3.;
                            if settings.softness == 0. {
                                f32::from(strength >= 0.23)
                            } else {
                                ((strength - 0.23) / settings.softness + 0.5).clamp(0., 1.)
                                    * (strength * 3.).min(1.)
                            }
                        }
                        _ => ((n.x * lx + n.y * ly + lz) * nz).clamp(0., 1.),
                    };
                    if settings.steps >= 2 {
                        let steps = f32::from(settings.steps - 1);
                        let level = diffuse * steps;
                        let blend = ((level.fract() - 0.40) / 0.20).clamp(0., 1.);
                        diffuse = (level.floor() + blend * blend * (3. - 2. * blend)) / steps;
                    }
                    let specular = if uses_specular {
                        let halfway = (2. + 2. * lz).sqrt();
                        (((n.x * lx + n.y * ly + lz + 1.) * nz) / halfway)
                            .clamp(0., 1.)
                            .powf(settings.gloss)
                            * settings.specular
                    } else {
                        0.
                    };
                    let influence = mask.map_or(1., |mask| {
                        let alpha = f32::from(mask[py * stride + idx + 3]) / 255.;
                        if self.mask_mode == MaskMode::Protect {
                            1. - alpha
                        } else {
                            alpha
                        }
                    });
                    let strength = settings.intensity * attenuation * opacity * influence;
                    let shade = if matches!(self.kind, Kind::Point | Kind::Directional | Kind::Spot)
                    {
                        (1. - diffuse) * settings.shadow * strength
                    } else {
                        0.
                    };
                    for channel in 0..3 {
                        let value = f32::from(pixels[idx + channel]);
                        if self.kind == Kind::RimShadow {
                            let ink = if settings.preserve_lines {
                                (value / alpha * 6.).min(1.)
                            } else {
                                1.
                            };
                            let weight = (diffuse * strength * ink).clamp(0., 1.);
                            pixels[idx + channel] =
                                (value * (1. - weight * (1. - tint[channel]))).round() as u8;
                            continue;
                        }
                        let lit = value * (1. - shade)
                            + ((alpha - value) * diffuse * 0.64 + alpha * specular)
                                * strength
                                * tint[channel];
                        pixels[idx + channel] = lit.clamp(0., alpha).round() as u8;
                    }
                }
            }
        };
        let rows = &mut pixels[top * stride..bottom * stride];
        static THREADS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        let available = *THREADS
            .get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get().min(4)));
        let worker_count = ((right - left) * (bottom - top) / (256 * 1024)).clamp(1, available);
        if worker_count == 1 {
            shade_rows(top, rows);
        } else {
            let chunk_height = (bottom - top).div_ceil(worker_count);
            std::thread::scope(|scope| {
                let shade_rows = &shade_rows;
                for (index, rows) in rows.chunks_mut(chunk_height * stride).enumerate() {
                    scope.spawn(move || shade_rows(top + index * chunk_height, rows));
                }
            });
        }
    }

    pub fn edit_key(&mut self, frame: usize, destination: Option<usize>) -> bool {
        match (self.keys.binary_search_by_key(&frame, |k| k.0), destination) {
            (Ok(index), Some(to)) if to != frame && !self.keyed(to) => {
                let value = self.keys.remove(index).1;
                self.set(to, value);
                if self.key_easing.contains_key(&frame) {
                    let easing = std::sync::Arc::make_mut(&mut self.key_easing);
                    let curve = easing.remove(&frame).unwrap();
                    easing.insert(to, curve);
                }
            }
            (Ok(index), None) => {
                self.keys.remove(index);
                if self.key_easing.contains_key(&frame) {
                    std::sync::Arc::make_mut(&mut self.key_easing).remove(&frame);
                }
            }
            (Err(_), Some(to)) if to == frame => {
                self.set(frame, self.at(frame));
            }
            _ => return false,
        }
        true
    }
}

// A capped Euclidean distance field gives an editable bevel across the silhouette.
// Texture relief also works on opaque artwork, where coverage has no internal edges.
#[derive(Default)]
pub(crate) struct SurfaceCache {
    entries: std::collections::VecDeque<SurfaceEntry>,
}

struct SurfaceEntry {
    width: usize,
    height: usize,
    bevel: f32,
    alpha: std::sync::Arc<Vec<u8>>,
    surface: std::sync::Arc<Surface>,
}

impl SurfaceEntry {
    fn bytes(&self) -> usize {
        self.surface.slopes.len() * std::mem::size_of::<Vec2>()
    }
}

impl SurfaceCache {
    fn get(
        &mut self,
        pixels: &[u8],
        width: usize,
        height: usize,
        bevel: f32,
        step: usize,
    ) -> std::sync::Arc<Surface> {
        let _span = crate::render_profile::span("surface normals");
        let alpha = || pixels[3..].iter().step_by(4).copied();
        let shared_alpha = self
            .entries
            .iter()
            .find(|entry| {
                entry.width == width
                    && entry.height == height
                    && entry.alpha.iter().copied().eq(alpha())
            })
            .map(|entry| entry.alpha.clone());
        if let Some(index) = shared_alpha.as_ref().and_then(|alpha| {
            self.entries.iter().position(|entry| {
                entry.width == width
                    && entry.height == height
                    && entry.bevel == bevel
                    && entry.surface.step == step
                    && std::sync::Arc::ptr_eq(alpha, &entry.alpha)
            })
        }) {
            let entry = self.entries.remove(index).unwrap();
            let surface = entry.surface.clone();
            self.entries.push_back(entry);
            return surface;
        }
        let surface =
            std::sync::Arc::new(Surface::new(pixels, width, height, bevel, 0., None, step));
        let alpha = shared_alpha.unwrap_or_else(|| std::sync::Arc::new(alpha().collect()));
        let size = alpha.len() + surface.slopes.len() * std::mem::size_of::<Vec2>();
        const LIMIT: usize = 64 * 1024 * 1024;
        if size <= LIMIT {
            let storage = |entries: &std::collections::VecDeque<SurfaceEntry>| {
                let mut alphas = std::collections::HashSet::new();
                alphas.insert(std::sync::Arc::as_ptr(&alpha));
                size + entries
                    .iter()
                    .map(|entry| {
                        entry.bytes()
                            + if alphas.insert(std::sync::Arc::as_ptr(&entry.alpha)) {
                                entry.alpha.len()
                            } else {
                                0
                            }
                    })
                    .sum::<usize>()
            };
            while storage(&self.entries) > LIMIT || self.entries.len() >= 32 {
                self.entries.pop_front().unwrap();
            }
            self.entries.push_back(SurfaceEntry {
                width,
                height,
                bevel,
                alpha,
                surface: surface.clone(),
            });
        }
        surface
    }
}

pub(crate) struct Surface {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) step: usize,
    pub(crate) slopes: Vec<Vec2>,
}
impl Surface {
    fn new(
        pixels: &[u8],
        width: usize,
        height: usize,
        bevel: f32,
        texture: f32,
        height_map: Option<&[u8]>,
        step: usize,
    ) -> Self {
        if let Some(surface) = crate::gpu_lens::surface_on_worker(
            pixels, width, height, bevel, texture, height_map, step,
        ) {
            return surface;
        }
        let w = width.div_ceil(step);
        let h = height.div_ceil(step);
        let limit = (bevel / step as f32).max(1.);
        let mut coverage = Vec::with_capacity(w * h);
        let mut details = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let mut sums = [0u32; 4];
                let mut height_sum = 0.;
                let mut count = 0.;
                for py in y * step..((y + 1) * step).min(height) {
                    for px in x * step..((x + 1) * step).min(width) {
                        let p = (py * width + px) * 4;
                        for (sum, &sample) in sums.iter_mut().zip(&pixels[p..p + 4]) {
                            *sum += u32::from(sample);
                        }
                        if let Some(map) = height_map {
                            height_sum += f32::from(map[p]) - f32::from(map[p + 2]);
                        }
                        count += 1.;
                    }
                }
                let [r, g, b, alpha] = sums.map(|sum| sum as f32);
                coverage.push(alpha / (255. * count));
                let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / alpha.max(1.);
                details.push(luma * texture * 8. + height_sum / (255. * count) * 22.);
            }
        }
        let distance = silhouette_distances(&coverage, w, h);
        let mut values: Vec<_> = details
            .into_iter()
            .enumerate()
            .map(|(i, detail)| {
                let (x, y) = (i % w, i / w);
                let edge = (distance[(y + 1) * (w + 2) + x + 1].sqrt() - 0.5).max(0.);
                let t = (edge / limit).min(1.);
                (t * (2. - t) * limit.min(48.) + detail) * coverage[i]
            })
            .collect();
        drop(distance);
        // Average on a scale relative to the bevel so normals stay smooth at any zoom.
        let radius = (limit * 0.22).round().clamp(2., 12.) as usize;
        let mut scratch = vec![0.; values.len()];
        for _ in 0..2 {
            smooth_surface(&values, &mut scratch, w, h, radius, true);
            smooth_surface(&scratch, &mut values, w, h, radius, false);
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

pub(crate) fn silhouette_distances(coverage: &[f32], w: usize, h: usize) -> Vec<f32> {
    // Keep fractional coverage in the distance transform. As an edge moves
    // through a cell its cost increases continuously instead of switching
    // between an exterior site and an opaque cell at 50% coverage.
    let (stride, rows) = (w + 2, h + 2);
    let far = (stride * stride + rows * rows) as f32;
    let mut field = vec![0.; stride * rows];
    for (y, samples) in coverage.chunks_exact(w).enumerate() {
        for (x, &alpha) in samples.iter().enumerate() {
            let edge = alpha / (1. - alpha).max(f32::EPSILON);
            field[(y + 1) * stride + x + 1] = edge.powi(2).min(far);
        }
    }
    // Extend the edge cells so the camera crop does not create relief.
    for row in field.chunks_exact_mut(stride) {
        row[0] = row[1];
        row[w + 1] = row[w];
    }
    field.copy_within(stride..stride * 2, 0);
    field.copy_within(h * stride..(h + 1) * stride, (h + 1) * stride);
    let size = stride.max(rows);
    let mut sites = vec![0; size];
    let mut boundaries = vec![0.; size + 1];
    let mut input = vec![0.; size];
    let mut output = vec![0.; size];
    for row in field.chunks_exact_mut(stride) {
        distance_line(row, &mut output[..stride], &mut sites, &mut boundaries);
        row.copy_from_slice(&output[..stride]);
    }
    for x in 0..stride {
        for (value, row) in input.iter_mut().zip(field.chunks_exact(stride)) {
            *value = row[x];
        }
        distance_line(
            &input[..rows],
            &mut output[..rows],
            &mut sites,
            &mut boundaries,
        );
        for (row, &distance) in field.chunks_exact_mut(stride).zip(&output) {
            row[x] = distance;
        }
    }
    field
}

fn distance_line(input: &[f32], output: &mut [f32], sites: &mut [usize], boundaries: &mut [f32]) {
    // Lower envelope of the squared-distance parabolas. Each site enters and exits once.
    let mut last = 0;
    sites[0] = 0;
    boundaries[0] = f32::NEG_INFINITY;
    boundaries[1] = f32::INFINITY;
    for q in 1..input.len() {
        let cross = |p: usize| {
            let gap = (q - p) as f32;
            (input[q] - input[p]) / (2. * gap) + (q + p) as f32 * 0.5
        };
        let mut at = cross(sites[last]);
        while at <= boundaries[last] {
            last -= 1;
            at = cross(sites[last]);
        }
        last += 1;
        sites[last] = q;
        boundaries[last] = at;
        boundaries[last + 1] = f32::INFINITY;
    }
    let mut site = 0;
    for (q, value) in output.iter_mut().enumerate() {
        while boundaries[site + 1] < q as f32 {
            site += 1;
        }
        let p = sites[site];
        let delta = q.abs_diff(p) as f32;
        *value = delta * delta + input[p];
    }
}

fn smooth_surface(
    source: &[f32],
    target: &mut [f32],
    width: usize,
    height: usize,
    radius: usize,
    horizontal: bool,
) {
    let (lines, length, line_step, sample_step) = if horizontal {
        (height, width, width, 1)
    } else {
        (width, height, 1, width)
    };
    let weight = 1. / (2 * radius + 1) as f32;
    for line in 0..lines {
        let first = line * line_step;
        let sample = |position: usize| source[first + position.min(length - 1) * sample_step];
        let mut sum = radius as f32 * sample(0) + (0..=radius).map(sample).sum::<f32>();
        for position in 0..length {
            target[first + position * sample_step] = sum * weight;
            sum += sample(position + radius + 1) - sample(position.saturating_sub(radius));
        }
    }
}

#[derive(Clone, Copy)]
enum MaskTarget {
    Layer(usize),
    Symbol(u64, usize),
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

    pub(crate) fn light_presets_ui(&mut self, ui: &mut egui::Ui) {
        design::category(ui, "Lighting presets");
        let has_lights = self
            .doc
            .layers
            .iter()
            .any(|layer| layer.options.light.is_some());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.post.preset_name)
                    .desired_width(130.)
                    .char_limit(48)
                    .hint_text(tr("Preset name")),
            );
            if design::button(
                ui,
                Icon::Save,
                "",
                "Save current lighting",
                false,
                has_lights
                    && !self.post.preset_name.trim().is_empty()
                    && self.studio.prefs.light_presets.len() < 64,
            )
            .clicked()
            {
                let lights = self
                    .doc
                    .layers
                    .iter()
                    .filter_map(|layer| {
                        let mut light = layer.options.light.clone()?;
                        light.base = light.at(self.frame);
                        light.keys.clear();
                        light.key_easing = Default::default();
                        light.receivers.clear();
                        light.shadow = None;
                        Some(PresetLight {
                            name: layer.name.clone(),
                            light,
                            opacity: layer.options.opacity,
                            visible: layer.visible,
                        })
                    })
                    .collect();
                let preset = Preset {
                    name: self.post.preset_name.trim().to_owned(),
                    size: [self.doc.width, self.doc.height],
                    lights,
                };
                self.studio.prefs.light_presets.push(preset);
                if let Err(error) = self.studio.save() {
                    self.studio.prefs.light_presets.pop();
                    self.status = error;
                } else {
                    self.post.preset_name.clear();
                }
            }
        });
        let mut apply = None;
        let mut remove = None;
        let can_layer = !crate::post::is_adjustment(&self.doc.layers[self.layer]);
        let folder = self.feature.selected_folder;
        egui::ScrollArea::vertical()
            .max_height(220.)
            .show(ui, |ui| {
                for (index, preset) in self.studio.prefs.light_presets.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            ui.menu_button(&preset.name, |ui| {
                                let enabled = self.doc.layers.len() + preset.lights.len() <= 100;
                                for (label, target, available) in [
                                    ("All artwork below", PresetTarget::All, true),
                                    ("Selected layer", PresetTarget::Layer(self.layer), can_layer),
                                    (
                                        "Selected folder",
                                        PresetTarget::Folder(folder.unwrap_or(0)),
                                        folder.is_some(),
                                    ),
                                ] {
                                    if design::button(
                                        ui,
                                        Icon::Bulb,
                                        label,
                                        "",
                                        false,
                                        enabled && available,
                                    )
                                    .clicked()
                                    {
                                        apply = Some((index, target));
                                        ui.close();
                                    }
                                }
                            });
                            if design::button(
                                ui,
                                Icon::Trash,
                                "",
                                "Delete lighting preset",
                                false,
                                true,
                            )
                            .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    });
                }
            });
        if crate::preset_file::exchange(
            ui,
            &mut self.studio.prefs.light_presets,
            "ltlights",
            Preset::valid,
            &mut self.status,
        ) && let Err(error) = self.studio.save()
        {
            self.status = error;
        }
        if let Some((index, target)) = apply {
            let preset = self.studio.prefs.light_presets[index].clone();
            self.apply_light_preset(&preset, target);
            ui.close();
        } else if let Some(index) = remove {
            let preset = self.studio.prefs.light_presets.remove(index);
            if let Err(error) = self.studio.save() {
                self.studio.prefs.light_presets.insert(index, preset);
                self.status = error;
            }
        }
    }

    fn apply_light_preset(&mut self, preset: &Preset, target: PresetTarget) {
        if preset.lights.is_empty()
            || self.doc.layers.len() + preset.lights.len() > 100
            || preset.size.contains(&0)
            || !self.feature.symbol_stack.is_empty()
            || !preset
                .lights
                .iter()
                .all(|p| p.light.valid() && (0. ..=1.).contains(&p.opacity))
        {
            return;
        }
        self.finish_raster_stroke();
        self.finish_pending_pen();
        self.checkpoint();
        let receiver = match target {
            PresetTarget::All => None,
            PresetTarget::Folder(id) => Some(Receiver::Folder(id)),
            PresetTarget::Layer(index) => Some(Receiver::Layer(
                *self.doc.layers[index]
                    .options
                    .light_receiver
                    .get_or_insert_with(crate::features::id),
            )),
        };
        let count = self.doc.layers[0].frames.len();
        let sx = self.doc.width as f32 / preset.size[0] as f32;
        let sy = self.doc.height as f32 / preset.size[1] as f32;
        for saved in &preset.lights {
            let mut light = saved.light.clone();
            light.receivers = receiver.into_iter().collect();
            light.keys.clear();
            light.key_easing = Default::default();
            light.shadow = None;
            let scale_point = |p: [f32; 2]| {
                [
                    (p[0] * sx).clamp(-100_000., 100_000.),
                    (p[1] * sy).clamp(-100_000., 100_000.),
                ]
            };
            light.base.position = scale_point(light.base.position);
            light.gizmo_position = light.gizmo_position.map(scale_point);
            light.base.radius = (light.base.radius * sx.min(sy)).clamp(1., 20_000.);
            self.doc.layers.push(crate::document::Layer {
                name: saved.name.clone(),
                visible: saved.visible,
                frames: vec![Default::default(); count].into(),
                options: crate::features::LayerOptions {
                    light: Some(light),
                    opacity: saved.opacity,
                    ..Default::default()
                },
            });
        }
        self.layer = self.doc.layers.len() - 1;
        self.recover();
    }

    pub(crate) fn height_guide_properties(&mut self, ui: &mut egui::Ui) {
        let layer = &self.doc.layers[self.layer];
        let Some(mut height) = layer.options.height_guide else {
            return;
        };
        design::category(ui, "Painted relief");
        ui.small(tr("Draw the raised or recessed area on this guide layer."));
        let original = height;
        ui.add_enabled_ui(!layer.options.locked, |ui| {
            control(ui, Icon::Depth, "Surface height", &mut height, -1. ..=1.)
        });
        if height != original {
            self.checkpoint();
            self.doc.layers[self.layer].options.height_guide = Some(height);
            self.recover();
        }
    }

    fn light_mask_targets(&self, light: &Light) -> Vec<(String, MaskTarget)> {
        let mut targets = Vec::new();
        for (index, layer) in self.doc.layers.iter().enumerate().take(self.layer).rev() {
            if !layer.visible
                || layer.options.light_mask
                || layer.options.height_guide.is_some()
                || crate::post::is_adjustment(layer)
                || !light.receives(&self.doc, layer)
            {
                continue;
            }
            let marks = &layer.frames[self.frame];
            let mut symbols = Vec::new();
            for mark in marks {
                let Some(instance) = &mark.style.instance else {
                    continue;
                };
                let Some(symbol) = self
                    .doc
                    .extras
                    .symbols
                    .iter()
                    .find(|s| s.id == instance.symbol)
                else {
                    continue;
                };
                if !symbols.iter().any(|&id| id == symbol.id) {
                    symbols.push(symbol.id);
                    let count = symbol.layers[0].frames.len();
                    let pose = crate::lip::pose_at(&mark.style.mouth_keys, self.frame)
                        .unwrap_or_else(|| {
                            crate::features::symbol_frame(instance, count, self.frame, self.frame)
                        })
                        .min(count - 1);
                    targets.push((
                        format!("{} › {}", layer.name, symbol.name),
                        MaskTarget::Symbol(symbol.id, pose),
                    ));
                }
            }
            if symbols.is_empty() || marks.iter().any(|mark| mark.style.instance.is_none()) {
                targets.push((layer.name.clone(), MaskTarget::Layer(index)));
            }
        }
        targets
    }

    fn start_light_mask(&mut self, target: MaskTarget) {
        match target {
            MaskTarget::Symbol(id, frame) => {
                let depth = self.feature.symbol_stack.len();
                self.enter_symbol(id);
                if self.feature.symbol_stack.len() == depth {
                    return;
                }
                self.frame = frame.min(self.doc.layers[0].frames.len() - 1);
                self.open_symbol_light_mask();
            }
            MaskTarget::Layer(index) => {
                if let Some(id) = self.doc.layers[index].options.light_receiver
                    && self.focus_light_mask(Some(id))
                {
                    return;
                }
                if self.doc.layers.len() >= 100 {
                    return;
                }
                self.finish_raster_stroke();
                self.finish_pending_pen();
                self.checkpoint();
                let id = *self.doc.layers[index]
                    .options
                    .light_receiver
                    .get_or_insert_with(crate::features::id);
                let folder = self.doc.layers[index].options.folder;
                self.create_light_mask(Some(id), folder);
            }
        }
    }

    pub(crate) fn open_symbol_light_mask(&mut self) {
        if self.focus_light_mask(None) || self.doc.layers.len() >= 100 {
            return;
        }
        self.finish_raster_stroke();
        self.finish_pending_pen();
        self.checkpoint();
        self.create_light_mask(None, self.feature.selected_folder);
    }

    fn focus_light_mask(&mut self, target: Option<u64>) -> bool {
        if let Some(index) =
            self.doc.layers.iter().position(|layer| {
                layer.options.light_mask && layer.options.light_mask_target == target
            })
        {
            self.finish_raster_stroke();
            self.layer = index;
            self.tool = crate::canvas::Tool::Brush;
            self.playing = false;
            return true;
        }
        false
    }

    fn create_light_mask(&mut self, target: Option<u64>, folder: Option<u64>) {
        let count = self.doc.layers[0].frames.len();
        let mut keys = vec![false; count];
        keys[0] = true;
        self.doc.layers.push(crate::document::Layer {
            name: tr("Light mask").into(),
            visible: true,
            frames: vec![Default::default(); count].into(),
            options: crate::features::LayerOptions {
                raster: true,
                light_mask: true,
                light_mask_target: target,
                folder,
                keys: keys.into(),
                ..Default::default()
            },
        });
        self.layer = self.doc.layers.len() - 1;
        self.tool = crate::canvas::Tool::Brush;
        self.playing = false;
        self.recover();
    }

    fn lighting_setup(&mut self, ui: &mut egui::Ui) {
        let light_index = self.layer;
        let source = &self.doc.layers[light_index];
        let light = source.options.light.as_ref().unwrap();
        let mut updated = light.clone();
        let mut new_receiver = None;
        let mut new_floor = None;
        let mut new_caster = None;
        let mut paint_target = None;
        design::category(ui, "Light receivers");
        if design::button(
            ui,
            Icon::Eye,
            "Solo in preview",
            "",
            self.post.solo_light,
            true,
        )
        .clicked()
        {
            self.post.solo_light = !self.post.solo_light;
        }
        ui.add_enabled_ui(!source.options.locked, |ui| {
            ui.menu_button(tr("Choose receivers"), |ui| {
                if ui.button(tr("All artwork below")).clicked() {
                    updated.receivers.clear();
                }
                for (index, layer) in self
                    .doc
                    .layers
                    .iter()
                    .enumerate()
                    .take(light_index)
                    .rev()
                    .filter(|(_, layer)| {
                        !crate::post::is_adjustment(layer)
                            && layer.options.height_guide.is_none()
                            && !layer.options.light_mask
                    })
                {
                    let receiver = layer.options.light_receiver.map(Receiver::Layer);
                    let selected = receiver.is_some_and(|id| updated.receivers.contains(&id));
                    if ui.selectable_label(selected, &layer.name).clicked() {
                        if selected {
                            updated.receivers.retain(|id| Some(*id) != receiver);
                        } else {
                            new_receiver = Some(index);
                        }
                    }
                }
                ui.separator();
                for folder in &self.doc.extras.folders {
                    let id = Receiver::Folder(folder.id);
                    let selected = updated.receivers.contains(&id);
                    if design::button(ui, Icon::Folder, &folder.name, "", selected, true).clicked()
                    {
                        if selected {
                            updated.receivers.retain(|receiver| *receiver != id);
                        } else if updated.receivers.len() < 100 {
                            updated.receivers.push(id);
                        }
                    }
                }
            });
            ui.small(tr(if updated.receivers.is_empty() {
                "All artwork below"
            } else {
                "Selected receivers are lit separately."
            }));
            design::category(ui, "Light mask");
            let targets = self.light_mask_targets(&updated);
            let brush = design::button(
                ui,
                Icon::Brush,
                "Light mask brush",
                "",
                false,
                !targets.is_empty(),
            );
            if targets.len() == 1 {
                if brush.clicked() {
                    paint_target = Some(targets[0].1);
                }
            } else {
                egui::Popup::menu(&brush).show(|ui| {
                    for (name, target) in &targets {
                        if design::button(ui, Icon::Mask, name, "", false, true).clicked() {
                            paint_target = Some(*target);
                            ui.close();
                        }
                    }
                });
            }
            egui::ComboBox::from_id_salt("light-mask-mode")
                .selected_text(tr(updated.mask_mode.name()))
                .show_ui(ui, |ui| {
                    for mode in [MaskMode::PaintLight, MaskMode::Protect, MaskMode::Ignore] {
                        ui.selectable_value(&mut updated.mask_mode, mode, tr(mode.name()));
                    }
                });
            if updated.uses_mask() {
                control(
                    ui,
                    Icon::Blur,
                    "Mask feather",
                    &mut updated.mask_feather,
                    0. ..=128.,
                );
            }
            design::category(ui, "Cast shadow");
            ui.menu_button(tr("Shadow floor"), |ui| {
                if ui.button(tr("No cast shadow")).clicked() {
                    updated.shadow = None;
                }
                for (index, layer) in self
                    .doc
                    .layers
                    .iter()
                    .enumerate()
                    .take(light_index)
                    .rev()
                    .filter(|(_, layer)| {
                        !crate::post::is_adjustment(layer)
                            && layer.options.height_guide.is_none()
                            && !layer.options.light_mask
                    })
                {
                    let selected = updated
                        .shadow
                        .as_ref()
                        .is_some_and(|shadow| Some(shadow.floor) == layer.options.light_receiver);
                    if ui.selectable_label(selected, &layer.name).clicked() {
                        new_floor = Some(index);
                    }
                }
            });
            if let Some(shadow) = &mut updated.shadow {
                ui.checkbox(&mut shadow.follow_light, tr("Follow light"));
                if shadow.follow_light {
                    control(
                        ui,
                        Icon::Depth,
                        "Occluder height",
                        &mut shadow.height,
                        0. ..=1024.,
                    );
                }
                ui.menu_button(tr("Shadow caster"), |ui| {
                    if ui
                        .selectable_label(shadow.caster.is_none(), tr("All artwork above receiver"))
                        .clicked()
                    {
                        shadow.caster = None;
                        ui.close();
                    }
                    let mut above_receiver = false;
                    for (index, candidate) in self.doc.layers.iter().enumerate().take(light_index) {
                        if candidate.options.light_receiver == Some(shadow.floor) {
                            above_receiver = true;
                            continue;
                        }
                        if above_receiver
                            && !crate::post::is_adjustment(candidate)
                            && candidate.options.height_guide.is_none()
                            && !candidate.options.light_mask
                            && ui
                                .selectable_label(
                                    shadow.caster.is_some()
                                        && shadow.caster == candidate.options.light_receiver,
                                    &candidate.name,
                                )
                                .clicked()
                        {
                            new_caster = Some(index);
                            ui.close();
                        }
                    }
                });
                ui.small(tr(
                    "Artwork above the floor and in the receiver list casts the shadow.",
                ));
                control(
                    ui,
                    Icon::Focus,
                    "Shadow X",
                    &mut shadow.offset[0],
                    -1024. ..=1024.,
                );
                control(
                    ui,
                    Icon::Focus,
                    "Shadow Y",
                    &mut shadow.offset[1],
                    -1024. ..=1024.,
                );
                control(
                    ui,
                    Icon::Blur,
                    "Shadow softness",
                    &mut shadow.softness,
                    0. ..=128.,
                );
                control(
                    ui,
                    Icon::ShadeBrush,
                    "Shadow opacity",
                    &mut shadow.strength,
                    0. ..=1.,
                );
            }
        });
        if updated != *light
            || new_receiver.is_some()
            || new_floor.is_some()
            || new_caster.is_some()
        {
            self.checkpoint();
            if let Some(index) = new_receiver {
                let id = self.doc.layers[index]
                    .options
                    .light_receiver
                    .get_or_insert_with(crate::features::id);
                if updated.receivers.len() < 100 {
                    updated.receivers.push(Receiver::Layer(*id));
                }
            }
            if let Some(index) = new_floor {
                let id = self.doc.layers[index]
                    .options
                    .light_receiver
                    .get_or_insert_with(crate::features::id);
                updated.shadow = Some(Shadow {
                    floor: *id,
                    caster: None,
                    offset: [24., 16.],
                    softness: 10.,
                    strength: 0.38,
                    follow_light: true,
                    height: shadow_height(),
                });
            }
            if let Some(index) = new_caster
                && let Some(shadow) = &mut updated.shadow
            {
                shadow.caster = Some(
                    *self.doc.layers[index]
                        .options
                        .light_receiver
                        .get_or_insert_with(crate::features::id),
                );
            }
            self.doc.layers[light_index].options.light = Some(updated);
            self.recover();
        }
        if let Some(target) = paint_target {
            self.start_light_mask(target);
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
        let mut gobo_symbol = None;
        ui.add_enabled_ui(!self.doc.layers[self.layer].options.locked, |ui| {
            design::category(ui, "Light source");
            if matches!(kind, Kind::Point | Kind::Spot) {
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
            }
            if kind != Kind::Point {
                control(
                    ui,
                    Icon::Rotate,
                    "Direction",
                    &mut value.angle,
                    -180. ..=180.,
                );
            }
            if kind == Kind::Spot {
                control(ui, Icon::Rotate, "Cone angle", &mut value.cone, 4. ..=170.);
                egui::ComboBox::from_id_salt("spot-gobo")
                    .selected_text(tr(value.gobo.name()))
                    .show_ui(ui, |ui| {
                        for pattern in [
                            Gobo::None,
                            Gobo::Blinds,
                            Gobo::Window,
                            Gobo::Leaves,
                            Gobo::Library,
                        ] {
                            if pattern == Gobo::Library && light.gobo_matte.is_none() {
                                continue;
                            }
                            ui.selectable_value(&mut value.gobo, pattern, tr(pattern.name()));
                        }
                    });
                ui.menu_button(tr("Use symbol as gobo"), |ui| {
                    for symbol in &self.doc.extras.symbols {
                        if ui.button(&symbol.name).clicked() {
                            gobo_symbol = Some(symbol.id);
                            ui.close();
                        }
                    }
                });
                if value.gobo != Gobo::None {
                    control(
                        ui,
                        Icon::Transform,
                        "Gobo offset",
                        &mut value.gobo_offset,
                        -2. ..=2.,
                    );
                }
            }
            if matches!(kind, Kind::RimShadow | Kind::Spot) {
                control(
                    ui,
                    Icon::Blur,
                    "Edge softness",
                    &mut value.softness,
                    0. ..=1.,
                );
            }
            if kind == Kind::RimShadow {
                ui.checkbox(&mut value.preserve_lines, tr("Preserve line art"));
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
        let matte = gobo_symbol.and_then(|id| make_gobo(&self.doc, id));
        if gobo_symbol.is_some() && matte.is_none() {
            self.status = tr("The symbol has no supported gobo artwork.").into();
        }
        if matte.is_some() {
            value.gobo = Gobo::Library;
        }
        if value != original || insert || remove || matte.is_some() {
            self.checkpoint();
            let light = self.doc.layers[self.layer].options.light.as_mut().unwrap();
            if let Some(matte) = matte {
                light.gobo_matte = Some(matte);
            }
            if remove {
                light.edit_key(self.frame, None);
            } else if light.keys.is_empty() && !insert && self.frame == 0 {
                light.base = value;
            } else {
                light.set(self.frame, value);
            }
            self.recover();
        }
        self.lighting_setup(ui);
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
        if kind == Kind::Ambient {
            return false;
        }
        let anchor = crate::vector::pos(light.gizmo_position.unwrap_or(settings.position));
        let center = view.screen(anchor);
        let arrow =
            view.screen(anchor + Vec2::angled(settings.angle.to_radians()) * (55. / view.scale));
        let center_rect = Rect::from_center_size(center, Vec2::splat(27.));
        let arrow_rect = Rect::from_center_size(arrow, Vec2::splat(24.));
        let cone_handle = center
            + Vec2::angled(settings.angle.to_radians() + settings.cone.to_radians() * 0.5)
                * (settings.radius * view.scale).clamp(75., 145.);
        if !input {
            let color = Color32::GOLD;
            if kind == Kind::Point {
                ui.painter().circle_stroke(
                    center,
                    settings.radius * view.scale,
                    Stroke::new(1., color.gamma_multiply(0.45)),
                );
            } else {
                if kind == Kind::Spot {
                    ui.painter()
                        .circle_stroke(cone_handle, 5., Stroke::new(1.5, color));
                    let reach = settings.radius * view.scale;
                    let angle = (arrow - center).angle();
                    let half = settings.cone.to_radians() * 0.5;
                    for side in [-1., 1.] {
                        let end = center + Vec2::angled(angle + side * half) * reach;
                        ui.painter().line_segment(
                            [center, end],
                            Stroke::new(1., color.gamma_multiply(0.5)),
                        );
                    }
                }
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
        let drag_id = ui.scope_id().with("light-gizmo-drag");
        let context = (self.doc.extras.active_scene, self.layer, self.frame, kind);
        let mut drag = ui
            .data(|data| data.get_temp::<GizmoDrag>(drag_id))
            .filter(|drag| drag.context == context);
        let (pointer, pressed, down, released) = ui.input(|input| {
            (
                input.pointer.interact_pos(),
                input.pointer.primary_pressed(),
                input.pointer.primary_down(),
                input.pointer.primary_released(),
            )
        });
        if pressed {
            drag = pointer.and_then(|pointer| {
                if !ui.clip_rect().contains(pointer)
                    || ui.ctx().layer_id_at(pointer) != Some(ui.layer_id())
                {
                    return None;
                }
                let action = if center_rect.contains(pointer) {
                    GizmoAction::Move
                } else if kind != Kind::Point && arrow_rect.contains(pointer) {
                    GizmoAction::Rotate
                } else if kind == Kind::Spot
                    && Rect::from_center_size(cone_handle, Vec2::splat(18.)).contains(pointer)
                {
                    GizmoAction::Aperture
                } else {
                    return None;
                };
                Some(GizmoDrag {
                    context,
                    action,
                    press: pointer,
                    offset: anchor - view.world(pointer),
                    changed: false,
                })
            });
        }
        if let Some(drag) = &mut drag
            && (down || released)
            && let Some(pointer) = pointer
            && (drag.changed || pointer.distance(drag.press) >= 2.)
        {
            if !drag.changed {
                self.checkpoint();
                drag.changed = true;
            }
            let light = self.doc.layers[self.layer].options.light.as_mut().unwrap();
            let point = view.world(pointer);
            match drag.action {
                GizmoAction::Move => {
                    let position = {
                        let position = point + drag.offset;
                        [position.x, position.y].map(|v| v.clamp(-100_000., 100_000.))
                    };
                    if matches!(kind, Kind::Point | Kind::Spot) {
                        light.gizmo_position = None;
                        light.set(
                            self.frame,
                            Settings {
                                position,
                                ..settings
                            },
                        );
                    } else {
                        light.gizmo_position = Some(position);
                    }
                }
                GizmoAction::Rotate => {
                    let delta = point - anchor;
                    if delta.length_sq() > 0.0001 {
                        light.set(
                            self.frame,
                            Settings {
                                angle: delta.angle().to_degrees(),
                                ..settings
                            },
                        );
                    }
                }
                GizmoAction::Aperture => {
                    let delta = (point - anchor).angle().to_degrees() - settings.angle;
                    let cone =
                        (((delta + 180.).rem_euclid(360.) - 180.).abs() * 2.).clamp(4., 170.);
                    light.set(self.frame, Settings { cone, ..settings });
                }
            }
        }
        let consumed = drag.is_some();
        if !down && !pressed {
            if drag.as_ref().is_some_and(|drag| drag.changed) {
                self.recover();
            }
            drag = None;
        }
        if consumed
            || pointer.is_some_and(|p| {
                ui.clip_rect().contains(p)
                    && ui.ctx().layer_id_at(p) == Some(ui.layer_id())
                    && (center_rect.contains(p)
                        || kind != Kind::Point && arrow_rect.contains(p)
                        || kind == Kind::Spot
                            && Rect::from_center_size(cone_handle, Vec2::splat(18.)).contains(p))
            })
        {
            ui.ctx().set_cursor_icon(if consumed {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        ui.data_mut(|data| {
            if let Some(drag) = drag {
                data.insert_temp(drag_id, drag);
            } else {
                data.remove::<GizmoDrag>(drag_id);
            }
        });
        consumed
    }
}

#[derive(Clone, Copy)]
enum GizmoAction {
    Move,
    Rotate,
    Aperture,
}

#[derive(Clone, Copy)]
struct GizmoDrag {
    context: (usize, usize, usize, Kind),
    action: GizmoAction,
    press: Pos2,
    offset: Vec2,
    changed: bool,
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
    #[ignore = "requires a GPU; native frame graph with camera, guide, masks and projected shadows"]
    fn gpu_scene_masks_guides_and_shadows_match_the_reference() {
        use crate::gpu_lens::{GpuLens, block_on, use_on_worker};
        let square = |x: f32, y: f32, size: f32, color| crate::Mark {
            points: vec![[x, y], [x + size, y + size]].into(),
            fill: Some(color),
            color,
            width: 1.,
            rectangle: true,
            closed: true,
            stroke: false,
            ..Default::default()
        };
        let instance = eframe::wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu = std::sync::Arc::new(GpuLens::new(device, queue));
        let mut doc = crate::Document::default();
        doc.width = 96;
        doc.height = 64;
        doc.layers[0].frames[0].push(square(-20., -20., 140., [100, 110, 120, 255]));
        doc.layers[0].options.light_receiver = Some(100);
        let mut art = doc.layers[0].clone();
        art.name = "Caster".into();
        art.options.light_receiver = Some(200);
        art.frames[0].clear();
        art.frames[0].push(square(28., 18., 25., [80, 110, 150, 220]));
        let mut guide = square(30., 20., 12., [255, 255, 255, 150]);
        guide.style.surface_height = Some(0.4);
        art.frames[0].push(guide);
        doc.layers.push(art);
        let mut mask = doc.layers[0].clone();
        mask.name = "Light mask".into();
        mask.options.light_receiver = None;
        mask.options.light_mask = true;
        mask.options.light_mask_target = Some(200);
        mask.frames[0].clear();
        mask.frames[0].push(square(32., 15., 25., [255, 255, 255, 160]));
        doc.layers.push(mask);
        let mut light = Light::new(96, 64, Kind::Point);
        light.base.position = [15., 8.];
        light.base.radius = 120.;
        light.base.texture_relief = 0.15;
        light.mask_feather = 3.;
        light.shadow = Some(Shadow {
            floor: 100,
            caster: Some(200),
            offset: [3., 4.],
            softness: 3.,
            strength: 0.45,
            follow_light: true,
            height: 12.,
        });
        let mut lamp = doc.layers[0].clone();
        lamp.frames[0].clear();
        lamp.options.light_receiver = None;
        lamp.options.light = Some(light);
        doc.layers.push(lamp);
        doc.extras.cameras.push(crate::features::Camera {
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [48., 32.],
            zoom: 1.75,
            angle: 0.15,
            optics: Default::default(),
            animation: Default::default(),
        });
        let ctx = eframe::egui::Context::default();
        for mode in [MaskMode::PaintLight, MaskMode::Protect, MaskMode::Ignore] {
            doc.layers[3].options.light.as_mut().unwrap().mask_mode = mode;
            use_on_worker(None);
            let expected = crate::exporter::raster(
                &doc,
                0,
                192,
                128,
                &ctx,
                &mut crate::paint::RenderCache::default(),
            );
            use_on_worker(Some(gpu.clone()));
            let mut cache = crate::paint::RenderCache::default();
            let actual = crate::exporter::raster(&doc, 0, 192, 128, &ctx, &mut cache);
            use_on_worker(None);
            assert_eq!(
                cache.native_scene.frames, 1,
                "native masks and shadows must execute: {}",
                cache.native_scene.reason
            );
            let max = expected
                .iter()
                .zip(&actual)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            let rms = (expected
                .iter()
                .zip(&actual)
                .map(|(a, b)| f64::from(a.abs_diff(*b)).powi(2))
                .sum::<f64>()
                / expected.len() as f64)
                .sqrt();
            eprintln!("native mask mode {}: max {max}, RMS {rms}", mode.name());
            assert!(max <= 5 && rms < 0.4);
        }
    }
    #[test]
    fn bevel_variants_share_coverage_and_retain_recent_surfaces() {
        let mut cache = SurfaceCache::default();
        let pixels = [60, 70, 80, 200].repeat(64 * 48);
        let first = cache.get(&pixels, 64, 48, 1., 1);
        for bevel in 2..=14 {
            cache.get(&pixels, 64, 48, bevel as f32, 1);
        }
        let again = cache.get(&pixels, 64, 48, 1., 1);
        assert!(std::sync::Arc::ptr_eq(&first, &again));
        let alpha = &cache.entries[0].alpha;
        assert!(
            cache
                .entries
                .iter()
                .all(|entry| std::sync::Arc::ptr_eq(alpha, &entry.alpha))
        );
    }
    #[test]
    #[ignore = "requires a GPU; distance-transform relief parity across alpha and bevel sizes"]
    fn gpu_surface_preserves_the_relief_field() {
        use crate::gpu_lens::block_on;
        let instance = eframe::wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu =
            crate::gpu_surface::GpuSurface::new(device, queue).expect("surface shaders compile");
        let (w, h) = (320, 240);
        let mut pixels = vec![0; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let distance = ((x as f32 - 160.).powi(2) + (y as f32 - 120.).powi(2)).sqrt();
                let alpha = ((100.5 - distance) * 0.5).clamp(0., 1.);
                let i = (y * w + x) * 4;
                pixels[i..i + 4]
                    .copy_from_slice(&[70., 110., 150., 255.].map(|v| (v * alpha).round() as u8));
            }
        }
        for step in [1, 2, 3] {
            for bevel in [8., 28., 72.] {
                let expected = Surface::new(&pixels, w, h, bevel, 0., None, step);
                let actual = gpu
                    .surface(&pixels, w, h, bevel, 0., None, step)
                    .expect("surface dispatch");
                let max = expected
                    .slopes
                    .iter()
                    .zip(&actual.slopes)
                    .map(|(a, b)| (*a - *b).abs().max_elem())
                    .fold(0., f32::max);
                eprintln!("surface step {step}, bevel {bevel}: max normal difference {max}");
                assert!(max < 0.002);
            }
        }
    }
    #[test]
    fn projected_occluders_follow_the_light_and_keep_old_shadows_compatible() {
        let mut doc = crate::Document::default();
        doc.width = 64;
        doc.height = 32;
        let mut light = Light::new(64, 32, Kind::Point);
        light.base.radius = 200.;
        light.base.elevation = 45.;
        light.shadow = Some(Shadow {
            floor: 1,
            caster: None,
            offset: [0., 0.],
            softness: 0.,
            strength: 0.8,
            follow_light: true,
            height: 32.,
        });
        let mut mask = vec![0; 64 * 32 * 4];
        for y in 10..22 {
            for x in 24..32 {
                mask[(y * 64 + x) * 4 + 3] = 255;
            }
        }
        let mut centroids = Vec::new();
        for x in [8., 56.] {
            light.base.position = [x, 16.];
            let mut pixels = [100, 100, 100, 255].repeat(64 * 32);
            light.apply_shadow(&mut pixels, mask.clone(), 64, 32, 0, &doc, 0, false, 1.);
            let mut sum = 0.;
            let mut weighted = 0.;
            for (i, p) in pixels.chunks_exact(4).enumerate() {
                let darkness = 100. - f32::from(p[0]);
                sum += darkness;
                weighted += darkness * (i % 64) as f32;
                assert_eq!(p[3], 255);
            }
            assert!(sum > 0.);
            centroids.push(weighted / sum);
        }
        assert!(
            centroids[0] > centroids[1] + 8.,
            "moving a point light must move its occluder's shadow in the opposite direction"
        );
        let mut legacy = serde_json::to_value(&light).unwrap();
        let shadow = legacy["shadow"].as_object_mut().unwrap();
        shadow.remove("follow_light");
        shadow.remove("height");
        let restored: Light = serde_json::from_value(legacy).unwrap();
        assert!(!restored.shadow.as_ref().unwrap().follow_light);
        assert!(restored.valid());
    }
    #[test]
    #[ignore = "requires a GPU; resident light chain and CPU fallback parity"]
    fn gpu_resident_light_chain_matches_cpu() {
        use crate::gpu_lens::{GpuLens, block_on, light_batch, use_on_worker};
        let instance = eframe::wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu = std::sync::Arc::new(GpuLens::new(device, queue));
        let (w, h) = (320, 240);
        let source: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                if i % 320 > 20 && i % 320 < 300 {
                    [65, 82, 93, 200]
                } else {
                    [0; 4]
                }
            })
            .collect();
        let lights: Vec<_> = [
            Kind::Point,
            Kind::Spot,
            Kind::Rim,
            Kind::Directional,
            Kind::Ambient,
        ]
        .into_iter()
        .map(|kind| {
            let mut light = Light::new(w, h, kind);
            light.base.radius = 400.;
            light.base.intensity = 0.22;
            light
        })
        .collect();
        let mut surfaces = SurfaceCache::default();
        let mut draw = |pixels: &mut [u8]| {
            for light in &lights {
                light.shade(
                    pixels,
                    w,
                    h,
                    0,
                    Pos2::new(160., 100.),
                    1.,
                    0.,
                    0.8,
                    None,
                    None,
                    Some(2),
                    Some(&mut surfaces),
                );
            }
        };
        let mut expected = source.clone();
        use_on_worker(None);
        draw(&mut expected);
        let mut actual = source.clone();
        use_on_worker(Some(gpu.clone()));
        light_batch(&mut actual, &mut draw);
        use_on_worker(None);
        assert_eq!(gpu.light_pass_count(), lights.len());
        let max = expected
            .iter()
            .zip(&actual)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(max <= 2, "resident light chain max {max}");
        // The bounded collector must replay the complete chain on CPU rather
        // than publish partial lighting when its command budget is exceeded.
        let many = |pixels: &mut [u8]| {
            for _ in 0..65 {
                lights[4].shade(
                    pixels,
                    w,
                    h,
                    0,
                    Pos2::new(160., 100.),
                    1.,
                    0.,
                    0.01,
                    None,
                    None,
                    Some(2),
                    None,
                );
            }
        };
        let mut expected = source.clone();
        many(&mut expected);
        let mut actual = source;
        use_on_worker(Some(gpu));
        light_batch(&mut actual, many);
        use_on_worker(None);
        assert_eq!(expected, actual);
    }

    #[test]
    #[ignore = "requires a compute GPU, compares all light kinds with CPU"]
    fn gpu_lights_match_cpu_with_relief_masks_and_gobos() {
        use crate::gpu_lens::{GpuLens, block_on, use_on_worker};
        use eframe::wgpu;

        let instance = wgpu::Instance::default();
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let gpu = std::sync::Arc::new(GpuLens::new(device, queue));
        let (w, h) = (640u32, 480u32);
        let mut source = vec![0; w as usize * h as usize * 4];
        let mut mask = source.clone();
        for y in 10..h - 10 {
            for x in 20..w - 20 {
                let index = (y as usize * w as usize + x as usize) * 4;
                let alpha =
                    (((x - 20).min(w - 21 - x).min(y - 10).min(h - 11 - y)) as f32 / 3.).min(1.);
                source[index..index + 4].copy_from_slice(
                    &[72, 105, 141, 200].map(|value| (value as f32 * alpha) as u8),
                );
                mask[index + 3] = ((x * 7 + y * 13) % 256) as u8;
            }
        }
        let mut cpu_total = std::time::Duration::ZERO;
        let mut gpu_total = std::time::Duration::ZERO;
        let mut largest_error = 0u8;
        let mut count = 0;
        let mut surfaces = SurfaceCache::default();
        for kind in [
            Kind::Point,
            Kind::Directional,
            Kind::Ambient,
            Kind::Rim,
            Kind::RimShadow,
            Kind::Spot,
        ] {
            for mode in [MaskMode::Ignore, MaskMode::PaintLight, MaskMode::Protect] {
                let gobos: &[Gobo] = if kind == Kind::Spot {
                    &[
                        Gobo::None,
                        Gobo::Blinds,
                        Gobo::Window,
                        Gobo::Leaves,
                        Gobo::Library,
                    ]
                } else {
                    &[Gobo::None]
                };
                for &gobo in gobos {
                    let mut light = Light::new(w, h, kind);
                    light.mask_mode = mode;
                    light.base.gobo = gobo;
                    light.base.position = [320., 240.];
                    light.base.radius = 560.;
                    light.base.steps = if mode == MaskMode::PaintLight { 4 } else { 0 };
                    light.base.specular = 0.55;
                    light.gobo_matte = (gobo == Gobo::Library)
                        .then(|| (0..48 * 48).map(|i| ((i * 17) % 256) as u8).collect());
                    let mut expected = source.clone();
                    let mut actual = source.clone();
                    use_on_worker(None);
                    let began = std::time::Instant::now();
                    light.shade(
                        &mut expected,
                        w,
                        h,
                        0,
                        Pos2::new(320., 240.),
                        1.,
                        0.2,
                        0.87,
                        None,
                        Some(&mask),
                        Some(2),
                        Some(&mut surfaces),
                    );
                    cpu_total += began.elapsed();
                    use_on_worker(Some(gpu.clone()));
                    let began = std::time::Instant::now();
                    light.shade(
                        &mut actual,
                        w,
                        h,
                        0,
                        Pos2::new(320., 240.),
                        1.,
                        0.2,
                        0.87,
                        None,
                        Some(&mask),
                        Some(2),
                        Some(&mut surfaces),
                    );
                    gpu_total += began.elapsed();
                    count += 1;
                    assert_eq!(gpu.light_pass_count(), count, "compute pass must execute");
                    for (&a, &b) in actual.iter().zip(&expected) {
                        largest_error = largest_error.max(a.abs_diff(b));
                    }
                    assert!(
                        actual
                            .iter()
                            .zip(&expected)
                            .all(|(&a, &b)| a.abs_diff(b) <= 1),
                        "{} GPU error {largest_error}",
                        kind.name()
                    );
                }
            }
        }
        use_on_worker(None);
        eprintln!(
            "{}: {count} lighting passes at {w}x{h}, CPU {cpu_total:?}, GPU {gpu_total:?}, maximum channel error {largest_error}/255",
            adapter.get_info().name
        );
    }

    #[test]
    fn keyed_light_easing_follows_moved_key_and_roundtrips() {
        let mut light = Light::new(640, 360, Kind::Point);
        light.set(
            2,
            Settings {
                intensity: 0.,
                ..light.base
            },
        );
        light.set(
            10,
            Settings {
                intensity: 1.,
                ..light.base
            },
        );
        assert_eq!(light.at(6).intensity, 0.5);
        light.set_timing(
            2,
            crate::animation::Curve {
                pace: [0., 0.6],
                ..Default::default()
            },
        );
        assert!(light.at(6).intensity < 0.4);
        assert!(light.edit_key(2, Some(4)));
        let curve = light.timing_segment(7).unwrap().2.unwrap();
        assert_eq!(curve.pace, [0., 0.6]);
        let reopened: Light = serde_json::from_slice(&serde_json::to_vec(&light).unwrap()).unwrap();
        assert_eq!(reopened.at(7).intensity, light.at(7).intensity);
    }

    #[test]
    fn large_masked_light_matches_independent_small_row_bands() {
        let (w, h) = (1024, 1024);
        let mut light = Light::new(w, h, Kind::Point);
        light.base.radius = 800.;
        light.base.relief = 0.;
        let center = Pos2::new(512., 512.);
        let row_bytes = w as usize * 4;
        let mut image = vec![0; row_bytes * h as usize];
        let mut mask = image.clone();
        for (index, pixel) in image.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[34, 56, (index % 97) as u8, 170]);
            mask[index * 4 + 3] = ((index * 13 + index / w as usize) % 256) as u8;
        }
        let mut reference = image.clone();
        light.shade(
            &mut image,
            w,
            h,
            0,
            center,
            1.,
            0.,
            0.9,
            None,
            Some(&mask),
            None,
            None,
        );
        for (band, (pixels, mask)) in reference
            .chunks_exact_mut(row_bytes * 128)
            .zip(mask.chunks_exact(row_bytes * 128))
            .enumerate()
        {
            let point = center - Vec2::new(0., (band * 128) as f32);
            light.shade(
                pixels,
                w,
                128,
                0,
                point,
                1.,
                0.,
                0.9,
                None,
                Some(mask),
                None,
                None,
            );
        }
        assert_eq!(image, reference);
    }

    #[test]
    fn scene_mask_brush_can_paint_and_subtract_on_a_vector_receiver() {
        let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        editor.doc.width = 96;
        editor.doc.height = 64;
        editor.doc.layers[0].frames[0].push(crate::Mark {
            points: vec![[1., 1.], [95., 63.]].into(),
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([60, 60, 60, 255]),
            ..Default::default()
        });
        let mut light = editor.doc.layers[0].clone();
        light.frames = vec![Default::default(); light.frames.len()].into();
        light.options.light = Some(Light::new(96, 64, Kind::Ambient));
        editor.doc.layers.push(light);
        editor.layer = 1;
        editor.start_light_mask(MaskTarget::Layer(0));
        assert!(editor.doc.layers[2].options.raster && editor.doc.layers[2].options.light_mask);
        assert!(!editor.doc.layers[0].options.raster);
        assert_eq!(
            editor.doc.layers[2].options.light_mask_target,
            editor.doc.layers[0].options.light_receiver
        );
        editor.start_light_mask(MaskTarget::Layer(0));
        assert_eq!(editor.doc.layers.len(), 3);
        let view = View {
            center: Pos2::ZERO,
            scale: 1.,
            stage_center: Pos2::ZERO,
            angle: 0.,
        };
        let mut canvas = crate::canvas::Canvas::default();
        let brush = crate::canvas::Tool::Brush;
        for (x, subtract) in [(20., false), (60., false), (20., true)] {
            editor.raster_input(
                &mut canvas,
                brush,
                Some(Pos2::new(x, 20.)),
                view,
                true,
                false,
                subtract,
            );
        }
        let rgb = crate::exporter::raster(
            &editor.doc,
            0,
            96,
            64,
            &egui::Context::default(),
            &mut Default::default(),
        );
        assert_eq!(rgb[(20 * 96 + 20) * 3], 60);
        assert!(rgb[(20 * 96 + 60) * 3] > 60);
        editor.raster_input(
            &mut canvas,
            brush,
            Some(Pos2::new(60., 20.)),
            view,
            true,
            false,
            true,
        );
        let empty = crate::exporter::raster(
            &editor.doc,
            0,
            96,
            64,
            &egui::Context::default(),
            &mut Default::default(),
        );
        assert_eq!(empty[(20 * 96 + 60) * 3], 60);
    }

    #[test]
    fn light_mask_moves_with_symbol_and_does_not_change_unmasked_artwork() {
        let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        editor.doc.width = 64;
        editor.doc.height = 64;
        let square = |x: f32, y: f32, size: f32| crate::Mark {
            points: vec![[x, y], [x + size, y + size]].into(),
            rectangle: true,
            closed: true,
            fill: Some([70, 70, 70, 255]),
            stroke: false,
            ..Default::default()
        };
        editor.doc.layers[0].frames[0].push(square(4., 8., 20.));
        editor.canvas.selected = vec![(0, 0)];
        editor.convert_symbol();
        let mut pixels = vec![0; 64 * 64 * 4];
        for y in 8..18 {
            for x in 4..14 {
                let i = (y * 64 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[255; 4]);
            }
        }
        let mut asset = crate::media::bitmap(pixels, 64, 64, "Mask tile".into());
        asset.raster_tile = true;
        let mut tile = square(0., 0., 64.);
        tile.fill = Some([0; 4]);
        tile.color = [255; 4];
        tile.style.raster_tile = true;
        tile.style.media_image = Some(asset.id);
        editor.doc.extras.media.assets.push(asset);
        let symbol = &mut editor.doc.extras.symbols[0];
        let mut guide = symbol.layers[0].clone();
        guide.frames[0] = vec![tile].into();
        guide.options.raster = true;
        guide.options.light_mask = true;
        symbol.layers.push(guide);
        let instance = editor.doc.layers[0].frames[0][0]
            .style
            .instance
            .as_mut()
            .unwrap();
        instance.matrix[4] = 6.;
        editor.doc.layers[0].frames[0].push(square(40., 8., 20.));
        let mut lamp = editor.doc.layers[0].clone();
        lamp.frames = vec![Default::default(); lamp.frames.len()].into();
        lamp.options.light = Some(Light::new(64, 64, Kind::Ambient));
        editor.doc.layers.push(lamp);
        let render = |doc: &crate::Document| {
            crate::exporter::raster(
                doc,
                0,
                64,
                64,
                &egui::Context::default(),
                &mut Default::default(),
            )
        };
        let bytes = crate::native_project::encode(&editor.doc).unwrap();
        let mut doc = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        let rgb = render(&doc);
        let index = |x: usize, y: usize| (y * 64 + x) * 3;
        assert!(rgb[index(14, 12)] > 70);
        assert_eq!(rgb[index(24, 12)], 70);
        assert!(rgb[index(48, 12)] > 70);
        doc.layers[1].options.light.as_mut().unwrap().mask_mode = MaskMode::Protect;
        let inverse = render(&doc);
        assert_eq!(inverse[index(14, 12)], 70);
        assert!(inverse[index(24, 12)] > 70);
        doc.layers[1].visible = false;
        let unlit = render(&doc);
        assert_eq!(unlit[index(14, 12)], 70);
    }

    #[test]
    fn lighting_preset_retargets_and_scales_for_another_document() {
        let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ));
        editor.doc.width = 128;
        editor.doc.height = 128;
        let mut light = Light::new(64, 64, Kind::Point);
        light.receivers.push(Receiver::Layer(123));
        light.set(3, light.base);
        let preset = Preset {
            name: "Night".into(),
            size: [64, 64],
            lights: vec![PresetLight {
                name: "Rim".into(),
                light,
                visible: true,
                opacity: 0.6,
            }],
        };
        editor.apply_light_preset(&preset, PresetTarget::Layer(0));
        let applied = editor.doc.layers[1].options.light.as_ref().unwrap();
        assert_eq!(applied.base.position[0], 64.);
        assert!(
            applied.receivers[0]
                == Receiver::Layer(editor.doc.layers[0].options.light_receiver.unwrap())
        );
        assert!(applied.keys.is_empty());
        assert_eq!(editor.doc.layers[1].options.opacity, 0.6);
    }

    #[test]
    fn selected_receiver_leaves_background_unlit_and_casts_on_named_floor() {
        let mut doc = crate::Document {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let square = |x: f32, y: f32, width: f32, color| crate::Mark {
            points: vec![[x, y], [x + width, y + width]].into(),
            rectangle: true,
            closed: true,
            fill: Some(color),
            stroke: false,
            ..Default::default()
        };
        let mut caster = doc.layers[0].clone();
        let mut lamp = caster.clone();
        doc.layers[0].options.light_receiver = Some(100);
        doc.layers[0].frames[0].push(square(0., 0., 64., [150, 150, 150, 255]));
        caster.options.light_receiver = Some(200);
        caster.frames[0].push(square(8., 8., 16., [90, 90, 90, 255]));
        let mut light = Light::new(64, 64, Kind::Ambient);
        light.receivers.push(Receiver::Layer(200));
        light.shadow = Some(Shadow {
            floor: 100,
            caster: None,
            offset: [24., 16.],
            softness: 0.,
            strength: 0.5,
            follow_light: false,
            height: shadow_height(),
        });
        lamp.options.light = Some(light);
        doc.layers.push(caster);
        doc.layers.push(lamp);
        let rgb = crate::exporter::raster(
            &doc,
            0,
            64,
            64,
            &egui::Context::default(),
            &mut Default::default(),
        );
        let pixel = |x: usize, y: usize| rgb[(y * 64 + x) * 3];
        assert_eq!(pixel(56, 56), 150);
        assert!(pixel(16, 16) > 90);
        assert_eq!(pixel(38, 30), 75);
    }

    #[test]
    fn round_rim_has_continuous_intensity_across_the_lit_arc() {
        let size = 512usize;
        let doc = crate::Document {
            width: size as u32,
            height: size as u32,
            ..Default::default()
        };
        let center = 256.;
        let radius = 146.;
        let mut pixels = vec![0; size * size * 4];
        for (i, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            let p = Vec2::new((i % size) as f32 - center, (i / size) as f32 - center);
            let alpha = (radius + 0.5 - p.length()).clamp(0., 1.);
            pixel.copy_from_slice(&[
                (80. * alpha) as u8,
                (65. * alpha) as u8,
                (210. * alpha) as u8,
                (255. * alpha) as u8,
            ]);
        }
        let mut light = Light::new(doc.width, doc.height, Kind::Rim);
        light.base.angle = -24.;
        light.base.intensity = 2.;
        light.base.relief = 0.9;
        light.base.color = [0, 255, 0];
        light.apply_padded(
            &mut pixels,
            doc.width,
            doc.height,
            0,
            &doc,
            0,
            false,
            1.,
            None,
            None,
            None,
        );
        for distance in [4., 9., 15.] {
            let intensity: Vec<_> = (-60..=60)
                .map(|offset| {
                    let angle = (light.base.angle + offset as f32).to_radians();
                    let p = Vec2::splat(center) + Vec2::angled(angle) * (radius - distance);
                    let x = p.x as usize;
                    let y = p.y as usize;
                    let sample = |x: usize, y: usize| f32::from(pixels[(y * size + x) * 4 + 1]);
                    let mix = |a, b, t| a + (b - a) * t;
                    mix(
                        mix(sample(x, y), sample(x + 1, y), p.x.fract()),
                        mix(sample(x, y + 1), sample(x + 1, y + 1), p.x.fract()),
                        p.y.fract(),
                    )
                })
                .collect();
            assert!(intensity[60] > 75.);
            let largest = intensity
                .windows(3)
                .map(|v| (v[0] - 2. * v[1] + v[2]).abs())
                .fold(0., f32::max);
            assert!(
                largest < 4.,
                "Abrupt rim-light changes {largest:.2} at {distance}px"
            );
        }
    }

    #[test]
    fn relief_cache_reuses_alpha_only_normals_and_rejects_changed_coverage() {
        let mut cache = SurfaceCache::default();
        let mut pixels = vec![255; 16 * 16 * 4];
        let first = cache.get(&pixels, 16, 16, 10., 1);
        pixels[0] = 100;
        let recolored = cache.get(&pixels, 16, 16, 10., 1);
        assert!(std::sync::Arc::ptr_eq(&first, &recolored));
        pixels[3] = 100;
        let edited = cache.get(&pixels, 16, 16, 10., 1);
        assert!(!std::sync::Arc::ptr_eq(&first, &edited));
        assert_eq!(
            edited.slopes,
            Surface::new(&pixels, 16, 16, 10., 0., None, 1).slopes
        );
    }

    #[test]
    fn crossing_half_coverage_does_not_make_the_bevel_jump() {
        let (width, height) = (96, 64);
        let surface = |coverage: u8| {
            let mut rgba = vec![0; width * height * 4];
            for y in 0..height {
                for x in 16..width {
                    rgba[(y * width + x) * 4 + 3] = if x == 16 { coverage } else { 255 };
                }
            }
            Surface::new(&rgba, width, height, 18., 0., None, 1)
        };
        for alpha in [31, 63, 95, 127, 159, 191, 223, 254] {
            let before = surface(alpha);
            let after = surface(alpha + 1);
            for x in 16..42 {
                let delta = before.slope(x, 32) - after.slope(x, 32);
                assert!(delta.length() < 0.035, "{alpha} at {x}: {delta:?}");
            }
        }
    }

    #[test]
    fn fully_opaque_artwork_has_no_bevel_at_camera_crop() {
        let (width, height) = (32, 32);
        let pixels = vec![255; width * height * 4];
        let surface = Surface::new(&pixels, width, height, 12., 0., None, 1);
        for y in 0..height {
            for x in 0..width {
                assert_eq!(surface.slope(x, y), Vec2::ZERO);
            }
        }
    }

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
        let surface = Surface::new(&pixels, width, height, 20., 0., None, 2);
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
    fn new_light_handles_capture_drag_over_the_canvas() {
        for kind in [Kind::RimShadow, Kind::Spot] {
            let ctx = egui::Context::default();
            let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            editor.doc.layers[0].options.light = Some(Light::new(200, 200, kind));
            let view = View {
                center: Pos2::ZERO,
                stage_center: Pos2::ZERO,
                scale: 1.,
                angle: 0.,
            };
            let tick = |editor: &mut Editor, events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., 260.))),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
                            editor.light_gizmo(ui, view, true);
                        });
                    },
                );
                output.textures_delta.clear();
            };
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            };
            let drag = |editor: &mut Editor, from, to| {
                tick(
                    editor,
                    vec![egui::Event::PointerMoved(from), button(from, true)],
                );
                tick(editor, vec![egui::Event::PointerMoved(to)]);
                tick(editor, vec![button(to, false)]);
            };
            tick(&mut editor, Vec::new());
            let center = Pos2::new(100., 80.);
            let destination = Pos2::new(135., 110.);
            drag(&mut editor, center, destination);
            let light = editor.doc.layers[0].options.light.as_ref().unwrap();
            assert_eq!(
                light.gizmo_position.unwrap_or(light.at(0).position),
                [135., 110.]
            );
            let arrow = destination + Vec2::angled(light.at(0).angle.to_radians()) * 55.;
            drag(&mut editor, arrow, destination + Vec2::new(0., 55.));
            let light = editor.doc.layers[0].options.light.as_ref().unwrap();
            assert!((light.at(0).angle - 90.).abs() < 0.01);
            assert_eq!(editor.undo.len(), 2);
        }
    }

    #[test]
    fn spot_limits_illumination_to_the_cone_and_rim_shadow_preserves_alpha() {
        let doc = crate::Document {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let mut pixels = vec![0; 64 * 64 * 4];
        for y in 8..56 {
            for x in 8..56 {
                pixels[(y * 64 + x) * 4..][..4].copy_from_slice(&[140, 140, 140, 255]);
            }
        }
        let mut spot = Light::new(64, 64, Kind::Spot);
        spot.base.position = [30., 32.];
        spot.base.angle = 0.;
        spot.base.radius = 30.;
        spot.base.relief = 0.;
        spot.base.shadow = 0.;
        let mut lit = pixels.clone();
        spot.apply_padded(&mut lit, 64, 64, 0, &doc, 0, false, 1., None, None, None);
        assert!(lit[(32 * 64 + 40) * 4] > 140);
        assert_eq!(lit[(32 * 64 + 20) * 4], 140);
        let mut shadow = Light::new(64, 64, Kind::RimShadow);
        shadow.base.angle = 0.;
        shadow.base.bevel = 8.;
        let mut shaded = pixels.clone();
        shadow.apply_padded(&mut shaded, 64, 64, 0, &doc, 0, false, 1., None, None, None);
        assert!(
            shaded
                .chunks_exact(4)
                .zip(pixels.chunks_exact(4))
                .any(|(after, before)| after[0] < before[0])
        );
        assert!(
            shaded
                .chunks_exact(4)
                .zip(pixels.chunks_exact(4))
                .all(|(after, before)| after[3] == before[3])
        );
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
        light.apply_padded(&mut left, 64, 64, 0, &doc, 0, false, 1., None, None, None);
        let mut moved = pixels.clone();
        light.gizmo_position = Some([200., 200.]);
        light.apply_padded(&mut moved, 64, 64, 0, &doc, 0, false, 1., None, None, None);
        assert_eq!(left, moved);
        light.base.angle = 0.;
        light.apply_padded(&mut pixels, 64, 64, 0, &doc, 0, false, 1., None, None, None);
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
