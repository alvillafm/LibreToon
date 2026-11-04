use crate::{
    document::{Document, Layer, Mark},
    shared::SharedVec,
};
use eframe::egui::{Pos2, Vec2};
use geo::BooleanOps;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cap {
    #[default]
    Round,
    Flat,
    Square,
    Triangle,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub t: f32,
    pub color: [u8; 4],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub radial: bool,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub stops: SharedVec<Stop>,
}
impl Default for Gradient {
    fn default() -> Self {
        Self {
            radial: false,
            start: [0., 0.],
            end: [100., 0.],
            stops: vec![
                Stop {
                    t: 0.,
                    color: [89, 72, 220, 255],
                },
                Stop {
                    t: 1.,
                    color: [255, 255, 255, 255],
                },
            ]
            .into(),
        }
    }
}
impl Gradient {
    pub fn color(&self, p: Pos2) -> [u8; 4] {
        let a = crate::vector::pos(self.start);
        let d = crate::vector::pos(self.end) - a;
        let t = if self.radial {
            p.distance(a) / d.length().max(0.001)
        } else {
            (p - a).dot(d) / d.length_sq().max(0.001)
        }
        .clamp(0., 1.);
        let first = &self.stops[0];
        if t <= first.t {
            return first.color;
        }
        for pair in self.stops.windows(2) {
            if t <= pair[1].t {
                let f = (t - pair[0].t) / (pair[1].t - pair[0].t).max(0.00001);
                return std::array::from_fn(|i| {
                    (pair[0].color[i] as f32 * (1. - f) + pair[1].color[i] as f32 * f).round() as u8
                });
            }
        }
        self.stops.last().unwrap().color
    }
    pub fn fit(&mut self, b: eframe::egui::Rect) {
        let direction =
            (crate::vector::pos(self.end) - crate::vector::pos(self.start)).normalized();
        let direction = if direction.length_sq() < 0.5 {
            Vec2::X
        } else {
            direction
        };
        let radius = (direction.x.abs() * b.width() + direction.y.abs() * b.height()) / 2.;
        self.start = crate::vector::xy(if self.radial {
            b.center()
        } else {
            b.center() - direction * radius
        });
        self.end = crate::vector::xy(b.center() + direction * radius.max(1.));
    }
    pub fn valid(&self) -> bool {
        self.stops.len() >= 2
            && self.stops.len() <= 32
            && self
                .start
                .iter()
                .chain(self.end.iter())
                .all(|v| v.is_finite() && v.abs() < 1e7)
            && self
                .stops
                .iter()
                .all(|s| s.t.is_finite() && (0. ..=1.).contains(&s.t))
            && self.stops.windows(2).all(|s| s[0].t <= s[1].t)
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Playback {
    #[default]
    Loop,
    Once,
    Single,
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolClock {
    #[default]
    Scene,
    Independent,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    #[serde(default)]
    pub clock: SymbolClock,
    pub symbol: u64,
    pub matrix: [f32; 6],
    pub mode: Playback,
    pub first: usize,
    pub born: usize,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_clip: Option<crate::animation_clips::Timing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_image: Option<u64>,
    pub instance_color: Option<[f32; 8]>,
    pub guide: bool,
    pub text: Option<Box<crate::typography::TextSpec>>,
    pub track: u64,
    pub gradient: Option<Gradient>,
    pub shade: u8,
    pub light: u8,
    pub start_cap: Cap,
    pub end_cap: Cap,
    pub instance: Option<Instance>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerOptions {
    pub animation_track: bool,
    pub depth: crate::depth::LayerDepth,
    pub mannequin: Option<crate::mannequin::Mannequin>,
    pub tweens: SharedVec<crate::animation::Tween>,
    pub locked: bool,
    pub opacity: f32,
    pub clip: bool,
    pub folder: Option<u64>,
    pub keys: SharedVec<bool>,
}
impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            animation_track: false,
            depth: Default::default(),
            mannequin: None,
            locked: false,
            tweens: Default::default(),
            opacity: 1.,
            clip: false,
            folder: None,
            keys: Default::default(),
        }
    }
}
pub fn default_style(style: &MarkStyle) -> bool {
    *style == MarkStyle::default()
}
pub fn default_layer_options(options: &LayerOptions) -> bool {
    *options == LayerOptions::default()
}
pub fn default_extras(extras: &ProjectExtras) -> bool {
    extras.animation_library.is_empty()
        && extras.animation_clips.is_empty()
        && !extras.imported
        && extras.import_warnings.is_empty()
        && extras.rig.is_empty()
        && extras.library.is_empty()
        && extras.media.assets.is_empty()
        && extras.media.tracks.is_empty()
        && extras.scenes.is_empty()
        && extras.active_scene == 0
        && extras.folders.is_empty()
        && extras.symbols.is_empty()
        && extras.cameras.is_empty()
        && extras.perspective == Perspective::default()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub collapsed: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Symbol {
    #[serde(default)]
    pub rig: crate::rig::Rig,
    #[serde(default)]
    pub library_folder: Option<u64>,
    #[serde(default)]
    pub folders: SharedVec<Folder>,
    #[serde(default)]
    pub perspective: Perspective,
    pub id: u64,
    pub name: String,
    pub layers: SharedVec<Layer>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Camera {
    #[serde(default)]
    pub optics: crate::depth::Optics,
    #[serde(default)]
    pub animation: crate::animation::CameraAnimation,
    pub id: u64,
    pub name: String,
    pub start: usize,
    pub duration: usize,
    pub center: [f32; 2],
    pub zoom: f32,
    pub angle: f32,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Perspective {
    pub enabled: bool,
    pub horizon: f32,
    pub points: SharedVec<[f32; 2]>,
    pub rays: u32,
}
impl Default for Perspective {
    fn default() -> Self {
        Self {
            enabled: false,
            horizon: 180.,
            points: vec![[480., 180.]].into(),
            rays: 12,
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectExtras {
    pub animation_library: crate::animation_clips::Collection,
    pub animation_clips: SharedVec<crate::animation_clips::Asset>,
    pub imported: bool,
    pub import_warnings: SharedVec<String>,
    pub rig: crate::rig::Rig,
    pub library: crate::library::Library,
    pub media: crate::media::MediaLibrary,
    pub scenes: SharedVec<crate::animation::Scene>,
    pub active_scene: usize,
    pub folders: SharedVec<Folder>,
    pub symbols: SharedVec<Symbol>,
    pub cameras: SharedVec<Camera>,
    pub perspective: Perspective,
}
#[derive(Clone)]
pub struct CameraTrim {
    pub index: usize,
    pub pointer_x: f32,
    pub left: Camera,
    pub right: Camera,
}
#[derive(Clone, Copy, Debug)]
pub struct FrameDrag {
    pub layer: usize,
    pub start: usize,
    pub end: usize,
    pub grab: usize,
    pub target: Option<usize>,
}
pub struct FeatureState {
    pub toon_levels: u8,
    pub gradient_on: bool,
    pub gradient: Gradient,
    pub start_cap: Cap,
    pub end_cap: Cap,
    pub library: bool,
    pub selected_symbol: Option<u64>,
    pub selected_folder: Option<u64>,
    pub selected_camera: Option<u64>,
    pub symbol_stack: Vec<(Document, usize, usize, u64)>,
    pub frame_selection: Option<(usize, usize, usize)>,
    pub frame_drag: Option<FrameDrag>,
    pub row_drag: Option<(bool, usize)>,
    pub camera_drag: Option<usize>,
    pub camera_trim: Option<CameraTrim>,
    pub guide_drag: Option<usize>,
    pub last_pointer: Option<Pos2>,
}
impl Default for FeatureState {
    fn default() -> Self {
        Self {
            toon_levels: 1,
            gradient_on: false,
            gradient: Default::default(),
            start_cap: Cap::Round,
            end_cap: Cap::Round,
            library: false,
            selected_symbol: None,
            selected_folder: None,
            selected_camera: None,
            symbol_stack: vec![],
            frame_selection: None,
            frame_drag: None,
            row_drag: None,
            camera_drag: None,
            camera_trim: None,
            guide_drag: None,
            last_pointer: None,
        }
    }
}
impl Layer {
    pub fn key(&self, f: usize) -> bool {
        self.options.keys.len() != self.frames.len() || self.options.keys[f]
    }
    pub fn exposure(&self, f: usize) -> std::ops::Range<usize> {
        let start = (0..=f).rev().find(|&i| self.key(i)).unwrap_or(0);
        let end = (f + 1..self.frames.len())
            .find(|&i| self.key(i))
            .unwrap_or(self.frames.len());
        start..end
    }
    pub fn init_keys(&mut self) {
        if self.options.keys.len() != self.frames.len() {
            self.options.keys = vec![true; self.frames.len()].into();
        }
    }
}
pub fn id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    time.wrapping_add(NEXT.fetch_add(1, Ordering::Relaxed))
}
pub fn styled(polys: geo::MultiPolygon<f64>, color: [u8; 4], style: &MarkStyle) -> Vec<Mark> {
    let mut marks = crate::vector::polygons_to_marks(polys, color);
    for m in &mut marks {
        m.style = style.clone();
        m.style.instance = None;
    }
    marks
}
pub fn clip_to(m: &Mark, area: &geo::MultiPolygon<f64>) -> Vec<Mark> {
    if m.style.text.is_some() {
        let mut style = m.style.clone();
        style.text = None;
        let mut out = vec![];
        if let Some(color) = m.fill {
            out.extend(styled(
                crate::vector::text_ink(m).intersection(area),
                color,
                &style,
            ));
        }
        if m.stroke {
            let mut line = m.clone();
            line.style.text = None;
            line.fill = None;
            for ring in std::iter::once(crate::vector::flatten(m, 0.04)).chain(
                m.holes
                    .iter()
                    .map(|r| r.iter().copied().map(crate::vector::pos).collect()),
            ) {
                line.nodes = Default::default();
                line.holes = Default::default();
                line.points = ring.into_iter().map(crate::vector::xy).collect();
                out.extend(styled(
                    crate::width::outline(&line).intersection(area),
                    m.color,
                    &style,
                ));
            }
        }
        return out;
    }
    let mut out = vec![];
    for polygon in &area.0 {
        let (_, parts) = crate::editing::split_area(m, polygon);
        out.extend(parts);
    }
    out
}

pub fn affine(m: &Mark, matrix: [f32; 6]) -> Mark {
    crate::vector::transform(m, |p| {
        Pos2::new(
            matrix[0] * p.x + matrix[2] * p.y + matrix[4],
            matrix[1] * p.x + matrix[3] * p.y + matrix[5],
        )
    })
}
fn expand(
    doc: &Document,
    mark: &Mark,
    frame: usize,
    depth: usize,
    out: &mut Vec<Mark>,
    guides: bool,
    clock: usize,
    poses: &std::collections::HashMap<u64, crate::rig::Pose>,
) {
    if mark.style.guide && !guides {
        return;
    }
    if depth > 16 {
        return;
    }
    if let Some(instance) = &mark.style.instance {
        let Some(symbol) = doc.extras.symbols.iter().find(|s| s.id == instance.symbol) else {
            return;
        };
        let count = symbol.layers.first().map_or(1, |l| l.frames.len()).max(1);
        let parent_frame = frame;
        let frame = mark
            .style
            .animation_clip
            .as_ref()
            .and_then(|timing| {
                crate::animation_clips::source_frame(doc, timing, instance.born, frame)
            })
            .unwrap_or_else(|| symbol_frame(instance, count, frame, clock));
        let mut sub = doc.clone();
        sub.layers = symbol.layers.clone();
        sub.extras.cameras = Default::default();
        sub.extras.rig = symbol.rig.clone();
        let binding = doc
            .extras
            .rig
            .bindings
            .iter()
            .find(|b| b.target == mark.style.track && b.symbol == instance.symbol);
        let deform = binding.map(|b| crate::rig::Deformer::new(b, poses, parent_frame));
        for (_, _, mut m) in resolve_internal(
            &sub,
            frame,
            depth + 1,
            guides,
            if mark.style.animation_clip.is_some() {
                frame
            } else {
                clock
                    .saturating_sub(instance.born)
                    .saturating_add(instance.first)
            },
        ) {
            if let Some(c) = mark.style.instance_color {
                apply_color(&mut m, c);
            }
            if let Some(d) = &deform {
                out.extend(d.mark(&m, instance.matrix));
            } else {
                out.push(affine(&m, instance.matrix));
            }
        }
    } else {
        out.push(mark.clone());
    }
}
pub fn ink(marks: &[Mark]) -> geo::MultiPolygon<f64> {
    let mut pieces = vec![];
    for mark in marks {
        if mark.fill.is_some_and(|c| c[3] > 0)
            || mark
                .style
                .gradient
                .as_ref()
                .is_some_and(|g| g.stops.iter().any(|s| s.color[3] > 0))
        {
            pieces.push(if mark.style.text.is_some() {
                crate::vector::text_ink(mark)
            } else {
                geo::MultiPolygon(vec![crate::vector::polygon(mark)])
            });
        }
        if mark.stroke && mark.color[3] > 0 {
            pieces.push(crate::width::outline(mark));
        }
    }
    geo::algorithm::bool_ops::unary_union(&pieces)
}
fn resolve_internal(
    doc: &Document,
    frame: usize,
    depth: usize,
    guides: bool,
    clock: usize,
) -> Vec<(usize, usize, Mark)> {
    let poses = doc.extras.rig.poses(frame);
    let mut output = vec![];
    let mut below = vec![];
    for (l, layer) in doc.layers.iter().enumerate() {
        if layer.options.mannequin.is_some() {
            continue;
        }
        if !layer.visible {
            below.clear();
            continue;
        }
        let mut current = vec![];
        let mask = if layer.options.clip {
            Some(ink(&below))
        } else {
            None
        };
        for (m, mark) in crate::animation::marks(layer, frame.min(layer.frames.len() - 1))
            .iter()
            .enumerate()
        {
            let mut expanded = vec![];
            expand(
                doc,
                mark,
                frame,
                depth,
                &mut expanded,
                guides,
                clock,
                &poses,
            );
            for original in expanded {
                let parts = mask
                    .as_ref()
                    .map_or_else(|| vec![original.clone()], |mask| clip_to(&original, mask));
                for mut part in parts {
                    let alpha = layer.options.opacity.clamp(0., 1.);
                    part.color[3] = (part.color[3] as f32 * alpha).round() as u8;
                    if let Some(c) = part.fill.as_mut() {
                        c[3] = (c[3] as f32 * alpha).round() as u8;
                    }
                    if let Some(g) = part.style.gradient.as_mut() {
                        for s in &mut g.stops {
                            s.color[3] = (s.color[3] as f32 * alpha).round() as u8;
                        }
                    }
                    current.push(part.clone());
                    output.push((l, m, part));
                }
            }
        }
        below = current;
    }
    output
}
pub fn resolve(doc: &Document, frame: usize, camera: bool) -> Vec<(usize, usize, Mark)> {
    resolve_mode(doc, frame, camera, true, frame)
}
pub fn resolve_export(doc: &Document, frame: usize, camera: bool) -> Vec<(usize, usize, Mark)> {
    resolve_mode(doc, frame, camera, false, frame)
}
fn resolve_mode(
    doc: &Document,
    frame: usize,
    camera: bool,
    guides: bool,
    clock: usize,
) -> Vec<(usize, usize, Mark)> {
    let mut marks = resolve_internal(doc, frame, 0, guides, clock);
    if camera {
        if let Some(c) = doc
            .extras
            .cameras
            .iter()
            .find(|c| frame >= c.start && frame < c.start + c.duration)
        {
            let optics = crate::animation::camera_optics(c, frame);
            let c = crate::animation::camera_at(c, frame);
            marks.retain(|(l, _, _)| crate::depth::visible(&doc.layers[*l].options.depth, &optics));
            for (l, _, m) in &mut marks {
                *m = crate::depth::project_mark(doc, &c, &optics, &doc.layers[*l].options.depth, m);
                *m = crate::vector::transform(m, |p| {
                    Pos2::new(doc.width as f32 / 2., doc.height as f32 / 2.)
                        + crate::canvas::rotate(p - crate::vector::pos(c.center), -c.angle) * c.zoom
                });
                m.width *= c.zoom;
            }
        }
    }
    marks
}
pub fn baked(doc: &Document) -> Document {
    if doc.extras.rig.is_empty()
        && doc.extras.scenes.is_empty()
        && doc.extras.symbols.is_empty()
        && doc.extras.cameras.is_empty()
        && doc.layers.iter().all(|l| {
            l.options.tweens.is_empty()
                && l.options.tweens.is_empty()
                && l.options.mannequin.is_none()
                && l.options.opacity == 1.
                && !l.options.clip
                && l.frames.iter().flatten().all(|m| {
                    !m.style.guide && m.style.gradient.is_none() && m.style.instance.is_none()
                })
        })
    {
        return doc.clone();
    }
    let count = crate::animation::movie_count(doc);
    let mut result = Document::default();
    result.width = doc.width;
    result.height = doc.height;
    result.fps = doc.fps;
    result.layers[0].frames = Vec::with_capacity(count).into();
    for f in 0..count {
        let mut frame = vec![];
        for (_, _, mark) in crate::animation::resolve_movie(doc, f, true) {
            if let Some(g) = &mark.style.gradient {
                let bounds = crate::vector::control_bounds(&mark);
                let source = if mark.style.text.is_some() {
                    crate::vector::text_ink(&mark)
                } else {
                    geo::MultiPolygon(vec![crate::vector::polygon(&mark)])
                };
                for y in 0..24 {
                    for x in 0..24 {
                        let a = bounds.min
                            + Vec2::new(
                                bounds.width() * x as f32 / 24.,
                                bounds.height() * y as f32 / 24.,
                            );
                        let b = a + bounds.size() / 24.;
                        let cutter =
                            crate::editing::area(&[a, Pos2::new(b.x, a.y), b, Pos2::new(a.x, b.y)]);
                        let mut style = mark.style.clone();
                        style.gradient = None;
                        frame.extend(styled(
                            source.intersection(&cutter),
                            g.color(a + (b - a) * 0.5),
                            &style,
                        ));
                    }
                }
                if mark.stroke && mark.color[3] > 0 {
                    let mut stroke = mark.clone();
                    stroke.fill = None;
                    stroke.style.gradient = None;
                    frame.push(stroke);
                }
            } else {
                frame.push(mark);
            }
        }
        result.layers[0].frames.push(frame.into());
    }
    result
}
pub fn validate(doc: &Document) -> Result<(), String> {
    doc.extras.rig.validate()?;
    doc.extras.library.validate(&doc.extras.symbols)?;
    for symbol in &doc.extras.symbols {
        symbol.rig.validate()?;
    }
    if doc.extras.folders.len() > 200
        || doc.extras.symbols.len() > 500
        || doc.extras.cameras.len() > 1000
    {
        return Err("Too many project resources".into());
    }
    if doc.layers.is_empty() || doc.layers.len() > 100 {
        return Err("Invalid layers".into());
    }
    if doc
        .layers
        .iter()
        .filter(|l| l.options.mannequin.is_some())
        .count()
        > 1
    {
        return Err("Only one mannequin per scene".into());
    }
    validate_resources(doc)?;
    validate_expansion(doc)?;
    crate::animation::validate(doc)?;
    crate::animation_clips::validate(doc)?;
    let mut checked = std::collections::HashSet::new();
    for layer in &doc.layers {
        if !layer.options.depth.valid()
            || !layer.options.opacity.is_finite()
            || !(0. ..=1.).contains(&layer.options.opacity)
        {
            return Err("Invalid layer opacity".into());
        }
        for m in layer
            .frames
            .iter()
            .filter(|f| checked.insert(f.identity()))
            .flatten()
        {
            if m.style.gradient.as_ref().is_some_and(|g| !g.valid()) {
                return Err("Invalid gradient".into());
            }
            if let Some(i) = &m.style.instance {
                if !i.matrix.iter().all(|v| v.is_finite() && v.abs() < 1e7)
                    || !doc.extras.symbols.iter().any(|s| s.id == i.symbol)
                {
                    return Err("Invalid symbol instance".into());
                }
            }
        }
    }
    let mut end = 0;
    for c in &doc.extras.cameras {
        if !c.optics.valid()
            || c.start != end
            || c.duration == 0
            || !c.zoom.is_finite()
            || !(0.01..=100.).contains(&c.zoom)
            || !c.angle.is_finite()
            || c.center.iter().any(|v| !v.is_finite() || v.abs() > 1e7)
        {
            return Err("Invalid camera sequence".into());
        }
        end = c.start + c.duration;
    }
    for folder in &doc.extras.folders {
        let mut parent = folder.parent;
        for depth in 0..=doc.extras.folders.len() {
            if let Some(id) = parent {
                if id == folder.id || depth == doc.extras.folders.len() {
                    return Err("Folder cycle".into());
                }
                parent = doc
                    .extras
                    .folders
                    .iter()
                    .find(|f| f.id == id)
                    .ok_or("Missing folder")?
                    .parent;
            } else {
                break;
            }
        }
    }
    for symbol in &doc.extras.symbols {
        if symbol.layers.is_empty() || symbol.layers.len() > 100 {
            return Err("Invalid symbol".into());
        }
        let count = symbol.layers[0].frames.len();
        if count == 0 || count > 10000 || symbol.layers.iter().any(|l| l.frames.len() != count) {
            return Err("Invalid symbol timeline".into());
        }
    }
    Ok(())
}

fn validate_resources(doc: &Document) -> Result<(), String> {
    fn folders(folders: &[Folder], layers: &[Layer]) -> Result<(), String> {
        if folders.len() > 200 {
            return Err("Too many folders".into());
        }
        let mut ids = std::collections::HashSet::new();
        for f in folders {
            if !ids.insert(f.id) {
                return Err("Duplicate folder ID".into());
            }
        }
        for f in folders {
            let mut parent = f.parent;
            for depth in 0..=32 {
                if let Some(id) = parent {
                    if id == f.id || depth == 32 {
                        return Err("Folder cycle or excessive depth".into());
                    }
                    parent = folders
                        .iter()
                        .find(|f| f.id == id)
                        .ok_or("Missing parent folder")?
                        .parent;
                } else {
                    break;
                }
            }
        }
        if layers
            .iter()
            .any(|l| l.options.folder.is_some_and(|id| !ids.contains(&id)))
        {
            return Err("Missing layer folder".into());
        }
        Ok(())
    }
    folders(&doc.extras.folders, &doc.layers)?;
    fn layers(layers: &[Layer], symbols: &[Symbol]) -> Result<(), String> {
        let mut checked = std::collections::HashSet::new();
        for layer in layers {
            if layer.options.mannequin.as_ref().is_some_and(|m| !m.valid()) {
                return Err("Invalid mannequin".into());
            }
            if layer
                .options
                .tweens
                .iter()
                .any(|s| s.start >= s.end || s.end >= layer.frames.len() || !s.curve.valid())
            {
                return Err("Invalid symbol motion".into());
            }
            if layer.frames.is_empty()
                || layer.frames.len() > 10000
                || !layer.options.depth.valid()
                || !layer.options.opacity.is_finite()
                || !(0. ..=1.).contains(&layer.options.opacity)
                || (!layer.options.keys.is_empty()
                    && (layer.options.keys.len() != layer.frames.len() || !layer.options.keys[0]))
            {
                return Err("Invalid layer or exposure timeline".into());
            }
            for mark in layer
                .frames
                .iter()
                .filter(|f| checked.insert(f.identity()))
                .flatten()
            {
                if mark
                    .style
                    .instance_color
                    .is_some_and(|c| c.iter().any(|v| !v.is_finite() || v.abs() > 1e7))
                {
                    return Err("Invalid instance color transform".into());
                }
                if mark.style.text.as_ref().is_some_and(|t| !t.valid()) {
                    return Err("Invalid text".into());
                }
                if !mark.width.is_finite()
                    || mark.width <= 0.
                    || mark.width > 100.
                    || !crate::width::validate(&mark.width_profile)
                    || mark
                        .points
                        .iter()
                        .flatten()
                        .chain(mark.nodes.iter().flat_map(|n| {
                            n.position
                                .iter()
                                .chain(n.in_handle.iter())
                                .chain(n.out_handle.iter())
                        }))
                        .chain(mark.holes.iter().flat_map(|h| h.iter().flatten()))
                        .any(|v| !v.is_finite() || v.abs() > 1e7)
                    || mark.style.gradient.as_ref().is_some_and(|g| !g.valid())
                {
                    return Err("Invalid symbol or scene geometry".into());
                }
                if let Some(i) = &mark.style.instance {
                    if !symbols.iter().any(|s| s.id == i.symbol)
                        || i.matrix.iter().any(|v| !v.is_finite() || v.abs() > 1e7)
                    {
                        return Err("Invalid symbol reference".into());
                    }
                }
            }
        }
        Ok(())
    }
    layers(&doc.layers, &doc.extras.symbols)?;
    let mut unique = std::collections::HashSet::new();
    for symbol in &doc.extras.symbols {
        if !unique.insert(symbol.id) {
            return Err("Duplicate symbol ID".into());
        }
        layers(&symbol.layers, &doc.extras.symbols)?;
        folders(&symbol.folders, &symbol.layers)?;
    }
    let graph = symbol_graph(&doc.extras.symbols);
    let mut memo = std::collections::HashMap::new();
    for id in graph.keys() {
        symbol_depth(
            *id,
            &graph,
            &mut std::collections::HashSet::new(),
            &mut memo,
        )?;
    }
    unique.clear();
    for folder in &doc.extras.folders {
        if !unique.insert(folder.id) {
            return Err("Duplicate folder ID".into());
        }
    }
    for layer in &doc.layers {
        if layer.options.folder.is_some_and(|id| !unique.contains(&id)) {
            return Err("Missing layer folder".into());
        }
    }
    let p = &doc.extras.perspective;
    if p.points.is_empty()
        || p.points.len() > 3
        || !(4..=32).contains(&p.rays)
        || !p.horizon.is_finite()
        || p.points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e7)
    {
        return Err("Invalid perspective guides".into());
    }
    unique.clear();
    for camera in &doc.extras.cameras {
        if !unique.insert(camera.id)
            || camera.start.saturating_add(camera.duration)
                > doc.layers.first().map_or(0, |l| l.frames.len())
        {
            return Err("Invalid camera cut".into());
        }
    }
    Ok(())
}

pub fn svg(doc: &Document, frame: usize) -> String {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"{}\" height=\"{}\" fill=\"white\"/>",
        doc.width, doc.height, doc.width, doc.height, doc.width, doc.height
    );
    let mut index = 0;
    let mut group = None;
    for (layer, _, original) in resolve_export(doc, frame, true) {
        if group != Some(layer) {
            if group.is_some() {
                out += "</g>";
            }
            group = Some(layer);
            if let Some(camera) = crate::depth::camera(doc, frame) {
                let optics = crate::animation::camera_optics(camera, frame);
                let radius =
                    optics.radius(doc.layers[layer].options.depth.distance - optics.position);
                if radius > 0.1 {
                    let pad = radius * 2. + 3.;
                    let filter = match optics.method {
                        crate::depth::BlurMethod::Gaussian => {
                            format!("<feGaussianBlur stdDeviation=\"{}\"/>", radius * 0.5)
                        }
                        crate::depth::BlurMethod::Lens => {
                            let kernel = (-5i32..=5)
                                .flat_map(|y| {
                                    (-5i32..=5)
                                        .map(move |x| if x * x + y * y <= 25 { "1" } else { "0" })
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            format!(
                                "<feConvolveMatrix order=\"11\" kernelMatrix=\"{kernel}\" divisor=\"81\" kernelUnitLength=\"{}\" edgeMode=\"none\"/>",
                                radius / 5.
                            )
                        }
                    };
                    out += &format!(
                        "<defs><filter id=\"dof-{layer}\" filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" color-interpolation-filters=\"sRGB\">{filter}</filter></defs><g filter=\"url(#dof-{layer})\">",
                        -pad,
                        -pad,
                        doc.width as f32 + pad * 2.,
                        doc.height as f32 + pad * 2.
                    );
                } else {
                    out += "<g>";
                }
            } else {
                out += "<g>";
            }
        }
        for mark in crate::width::export_marks(&original) {
            let mut fill = mark
                .fill
                .map(|c| format!("rgb({},{},{})", c[0], c[1], c[2]))
                .unwrap_or("none".into());
            let mut alpha = mark.fill.map_or(1., |c| c[3] as f32 / 255.);
            if let Some(g) = mark.style.gradient.as_ref().filter(|_| mark.fill.is_some()) {
                let name = format!("gradient-{index}");
                index += 1;
                let tag = if g.radial {
                    "radialGradient"
                } else {
                    "linearGradient"
                };
                if g.radial {
                    out += &format!(
                        "<defs><{tag} id=\"{name}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\">",
                        g.start[0],
                        g.start[1],
                        crate::vector::pos(g.start)
                            .distance(crate::vector::pos(g.end))
                            .max(0.001)
                    );
                } else {
                    out += &format!(
                        "<defs><{tag} id=\"{name}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">",
                        g.start[0], g.start[1], g.end[0], g.end[1]
                    );
                }
                for stop in &g.stops {
                    out += &format!(
                        "<stop offset=\"{}\" stop-color=\"rgb({},{},{})\" stop-opacity=\"{}\"/>",
                        stop.t,
                        stop.color[0],
                        stop.color[1],
                        stop.color[2],
                        stop.color[3] as f32 / 255.
                    );
                }
                out += &format!("</{tag}></defs>");
                fill = format!("url(#{name})");
                alpha = 1.;
            }
            let stroke = if mark.stroke {
                format!("rgb({},{},{})", mark.color[0], mark.color[1], mark.color[2])
            } else {
                "none".into()
            };
            if mark.points.len() == 1 && mark.nodes.is_empty() && mark.stroke && !mark.rectangle {
                out += &format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{stroke}\" fill-opacity=\"{}\"/>",
                    mark.points[0][0],
                    mark.points[0][1],
                    mark.width / 2.,
                    mark.color[3] as f32 / 255.
                );
            } else {
                out += &format!(
                    "<path d=\"{}\" fill=\"{fill}\" fill-opacity=\"{alpha}\" fill-rule=\"evenodd\" stroke=\"{stroke}\" stroke-opacity=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                    mark.svg_path(),
                    mark.color[3] as f32 / 255.,
                    mark.width
                );
            }
        }
    }
    if group.is_some() {
        out += "</g>";
    }
    out + "</svg>"
}

pub fn resolve_at(
    doc: &Document,
    frame: usize,
    camera: bool,
    clock: usize,
) -> Vec<(usize, usize, Mark)> {
    resolve_mode(doc, frame, camera, true, clock)
}
pub fn symbol_frame(i: &Instance, count: usize, frame: usize, clock: usize) -> usize {
    let count = count.max(1);
    let source = if i.clock == SymbolClock::Independent {
        clock
    } else {
        frame
    };
    let offset = source.saturating_sub(i.born).saturating_add(i.first);
    match i.mode {
        Playback::Loop => offset % count,
        Playback::Once => offset.min(count - 1),
        Playback::Single => i.first.min(count - 1),
    }
}

pub fn apply_color(m: &mut Mark, c: [f32; 8]) {
    let convert = |color: &mut [u8; 4]| {
        for i in 0..4 {
            color[i] = (color[i] as f32 * c[i] + c[i + 4]).round().clamp(0., 255.) as u8;
        }
    };
    convert(&mut m.color);
    if let Some(f) = m.fill.as_mut() {
        convert(f);
    }
    if let Some(g) = m.style.gradient.as_mut() {
        for s in &mut g.stops {
            convert(&mut s.color);
        }
    }
}

/// Deduplicate held exposures and dependency edges before traversing the DAG.
pub(crate) fn symbol_graph(symbols: &[Symbol]) -> std::collections::HashMap<u64, Vec<u64>> {
    symbols
        .iter()
        .map(|s| {
            let mut frames = std::collections::HashSet::new();
            let mut edges = std::collections::BTreeSet::new();
            for frame in s.layers.iter().flat_map(|l| l.frames.iter()) {
                if frames.insert(frame.identity()) {
                    for i in frame.iter().filter_map(|m| m.style.instance.as_ref()) {
                        edges.insert(i.symbol);
                    }
                }
            }
            (s.id, edges.into_iter().collect())
        })
        .collect()
}
fn symbol_depth(
    id: u64,
    graph: &std::collections::HashMap<u64, Vec<u64>>,
    active: &mut std::collections::HashSet<u64>,
    memo: &mut std::collections::HashMap<u64, usize>,
) -> Result<usize, String> {
    if let Some(d) = memo.get(&id) {
        return Ok(*d);
    }
    if active.len() >= 8 || !active.insert(id) {
        return Err("Recursive symbol or excessive nesting".into());
    }
    let mut depth = 1;
    for child in graph.get(&id).into_iter().flatten() {
        depth = depth.max(1 + symbol_depth(*child, graph, active, memo)?);
    }
    active.remove(&id);
    if depth > 8 {
        return Err("Recursive symbol or excessive nesting".into());
    }
    memo.insert(id, depth);
    Ok(depth)
}

/// Limit expanded geometry rather than counting shared resources only. A tiny
/// recursive-free DAG can otherwise instantiate billions of paths in one frame.
pub(crate) fn validate_expansion(doc: &Document) -> Result<(), String> {
    fn layers_cost(
        layers: &[Layer],
        symbols: &[Symbol],
        memo: &mut std::collections::HashMap<u64, usize>,
    ) -> usize {
        let mut total = 0usize;
        for layer in layers {
            let mut seen = std::collections::HashSet::new();
            let mut max = 0usize;
            for frame in &layer.frames {
                if !seen.insert(frame.identity()) {
                    continue;
                }
                let mut cost = 0usize;
                for mark in frame {
                    let value = if let Some(i) = &mark.style.instance {
                        if let Some(cost) = memo.get(&i.symbol) {
                            *cost
                        } else if let Some(s) = symbols.iter().find(|s| s.id == i.symbol) {
                            let cost = layers_cost(&s.layers, symbols, memo);
                            memo.insert(s.id, cost);
                            cost
                        } else {
                            1
                        }
                    } else {
                        mark.points
                            .len()
                            .saturating_add(mark.nodes.len().saturating_mul(3))
                            .saturating_add(mark.holes.iter().map(|h| h.len()).sum::<usize>())
                            .max(1)
                    };
                    cost = cost.saturating_add(value).min(2_000_001);
                    if cost > 2_000_000 {
                        break;
                    }
                }
                max = max.max(cost);
            }
            total = total.saturating_add(max).min(2_000_001);
        }
        total
    }
    let mut memo = std::collections::HashMap::new();
    for s in &doc.extras.symbols {
        if layers_cost(&s.layers, &doc.extras.symbols, &mut memo) > 2_000_000 {
            return Err("Expanded symbol geometry exceeds 2000000 control points per frame".into());
        }
    }
    if layers_cost(&doc.layers, &doc.extras.symbols, &mut memo) > 2_000_000 {
        return Err("Expanded scene geometry exceeds 2000000 control points per frame".into());
    }
    Ok(())
}
