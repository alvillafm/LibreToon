use crate::{
    Document, Editor,
    document::{Layer, Mark},
    features::{Camera, Folder, Perspective},
    shared::SharedVec,
};
use eframe::egui::{Pos2, Vec2};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Curve {
    pub ease: [[f32; 2]; 2],
    pub path: [[f32; 2]; 2],
}
impl Default for Curve {
    fn default() -> Self {
        Self {
            ease: [[1. / 3., 1. / 3.], [2. / 3., 2. / 3.]],
            path: [[0., 0.], [0., 0.]],
        }
    }
}
impl Curve {
    pub fn time(&self, t: f32) -> f32 {
        if self.ease[0][0] == self.ease[0][1] && self.ease[1][0] == self.ease[1][1] {
            return t.clamp(0., 1.);
        }
        if t <= 0. {
            return 0.;
        }
        if t >= 1. {
            return 1.;
        }
        let cubic = |t: f32, a: f32, b: f32| {
            3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * b + t * t * t
        };
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..18 {
            let mid = (lo + hi) / 2.;
            if cubic(mid, self.ease[0][0], self.ease[1][0]) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        cubic((lo + hi) / 2., self.ease[0][1], self.ease[1][1])
    }
    pub fn point(&self, a: Pos2, b: Pos2, t: f32) -> Pos2 {
        let p = a + (b - a) / 3. + crate::vector::pos(self.path[0]).to_vec2();
        let q = a + (b - a) * 2. / 3. + crate::vector::pos(self.path[1]).to_vec2();
        a + (p - a) * (3. * (1. - t).powi(2) * t)
            + (q - a) * (3. * (1. - t) * t * t)
            + (b - a) * (t * t * t)
    }
    pub fn valid(&self) -> bool {
        self.ease
            .iter()
            .flatten()
            .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            && self
                .path
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() < 1e7)
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Tween {
    pub start: usize,
    pub end: usize,
    pub curve: Curve,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraKey {
    pub frame: usize,
    pub center: [f32; 2],
    pub zoom: f32,
    pub angle: f32,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraAnimation {
    pub keys: SharedVec<CameraKey>,
    pub curve: Curve,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: u64,
    pub name: String,
    pub layers: SharedVec<Layer>,
    pub folders: SharedVec<Folder>,
    pub cameras: SharedVec<Camera>,
    pub perspective: Perspective,
}
#[derive(Default)]
pub struct State {
    pub scenes_open: bool,
    pub scene_rename: Option<u64>,
    pub scene_delete: Option<u64>,
    pub scene_drag: Option<usize>,
    pub play_all: bool,
    pub picker: Option<(usize, usize, usize)>,
    pub stop: usize,
    pub gradient_drag: Option<usize>,
    pub ease_handle: Option<usize>,
    pub path_handle: Option<usize>,
    pub camera_mode: usize,
    pub thumbnail_cache: crate::paint::RenderCache,
}
pub fn scene_from(doc: &Document, id: u64, name: String) -> Scene {
    Scene {
        id,
        name,
        layers: doc.layers.clone(),
        folders: doc.extras.folders.clone(),
        cameras: doc.extras.cameras.clone(),
        perspective: doc.extras.perspective.clone(),
    }
}
pub fn store_scene(doc: &mut Document) {
    if !doc.extras.scenes.is_empty() {
        let index = doc.extras.active_scene.min(doc.extras.scenes.len() - 1);
        let old = &doc.extras.scenes[index];
        let scene = scene_from(doc, old.id, old.name.clone());
        doc.extras.scenes[index] = scene;
    }
}
pub fn scene_doc(doc: &Document, index: usize) -> Document {
    let mut result = doc.clone();
    if !doc.extras.scenes.is_empty() && index != doc.extras.active_scene {
        let s = &doc.extras.scenes[index];
        result.layers = s.layers.clone();
        result.extras.folders = s.folders.clone();
        result.extras.cameras = s.cameras.clone();
        result.extras.perspective = s.perspective.clone();
    }
    result.extras.scenes = Default::default();
    result.extras.active_scene = 0;
    result
}
pub fn movie_count(doc: &Document) -> usize {
    if doc.extras.scenes.is_empty() {
        return doc.layers[0].frames.len();
    }
    doc.extras
        .scenes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == doc.extras.active_scene {
                doc.layers[0].frames.len()
            } else {
                s.layers[0].frames.len()
            }
        })
        .sum()
}
pub fn locate(doc: &Document, mut frame: usize) -> (usize, usize) {
    if doc.extras.scenes.is_empty() {
        return (0, frame.min(doc.layers[0].frames.len() - 1));
    }
    for (i, s) in doc.extras.scenes.iter().enumerate() {
        let count = if i == doc.extras.active_scene {
            doc.layers[0].frames.len()
        } else {
            s.layers[0].frames.len()
        };
        if frame < count {
            return (i, frame);
        }
        frame -= count;
    }
    let i = doc.extras.scenes.len() - 1;
    (i, scene_doc(doc, i).layers[0].frames.len() - 1)
}
pub fn movie_offset(doc: &Document, local: usize) -> usize {
    doc.extras
        .scenes
        .iter()
        .take(doc.extras.active_scene)
        .map(|s| s.layers[0].frames.len())
        .sum::<usize>()
        + local
}
pub fn resolve_movie(doc: &Document, f: usize, camera: bool) -> Vec<(usize, usize, Mark)> {
    let (i, f) = locate(doc, f);
    crate::features::resolve(&scene_doc(doc, i), f, camera)
}
pub fn movie_svg(doc: &Document, f: usize) -> String {
    let (i, f) = locate(doc, f);
    scene_doc(doc, i).svg(f)
}
fn control(mark: &Mark) -> Vec<Pos2> {
    if mark.rectangle {
        return crate::vector::flatten(mark, 0.1);
    }
    if !mark.nodes.is_empty() {
        mark.nodes
            .iter()
            .take(86)
            .flat_map(|n| [n.position, n.in_handle, n.out_handle])
            .map(crate::vector::pos)
            .collect()
    } else {
        mark.points
            .iter()
            .take(256)
            .copied()
            .map(crate::vector::pos)
            .collect()
    }
}
pub fn delta(a: &Mark, b: &Mark) -> [f32; 4] {
    let p = control(a);
    let q = control(b);
    if p.len() != q.len() || p.len() < 2 {
        let a = crate::vector::control_bounds(a).size();
        let b = crate::vector::control_bounds(b).size();
        return [b.x / a.x.max(0.001), 0., 0., b.y / a.y.max(0.001)];
    }
    let origin = p[0];
    let i = (1..p.len())
        .max_by(|&i, &j| {
            p[i].distance_sq(origin)
                .total_cmp(&p[j].distance_sq(origin))
        })
        .unwrap();
    let u = p[i] - origin;
    let v = q[i] - q[0];
    let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
    let j = (0..p.len())
        .max_by(|&i, &j| {
            cross(u, p[i] - origin)
                .abs()
                .total_cmp(&cross(u, p[j] - origin).abs())
        })
        .unwrap();
    let w = p[j] - origin;
    let z = q[j] - q[0];
    let det = cross(u, w);
    if det.abs() > 0.001 {
        [
            (v.x * w.y - z.x * u.y) / det,
            (v.y * w.y - z.y * u.y) / det,
            (z.x * u.x - v.x * w.x) / det,
            (z.y * u.x - v.y * w.x) / det,
        ]
    } else {
        let len = u.length_sq().max(0.001);
        let x = u.dot(v) / len;
        let y = cross(u, v) / len;
        [x, y, -y, x]
    }
}
pub fn shortest(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
pub fn interpolate(a: &Mark, b: &Mark, t: f32, curve: &Curve) -> Mark {
    let t = curve.time(t);
    let center = crate::vector::control_bounds(a).center();
    let target = crate::vector::control_bounds(b).center();
    let position = curve.point(center, target, t);
    let m = delta(a, b);
    let sx = Vec2::new(m[0], m[1]).length().max(0.00001);
    let sy = (m[0] * m[3] - m[1] * m[2]) / sx;
    let angle = shortest(m[1].atan2(m[0])) * t;
    let scale = Vec2::new(1. + (sx - 1.) * t, 1. + (sy - 1.) * t);
    let mut out = crate::vector::transform(a, |p| {
        position + crate::canvas::rotate((p - center) * scale, angle)
    });
    if let (Some(first), Some(last)) = (out.style.instance.as_mut(), b.style.instance.as_ref()) {
        first.first = (first.first as f32 * (1. - t) + last.first as f32 * t).round() as usize;
    }
    out
}
pub fn span(layer: &Layer, frame: usize) -> Option<&Tween> {
    layer
        .options
        .tweens
        .iter()
        .find(|s| frame >= s.start && frame <= s.end)
}
pub fn segment(layer: &Layer, frame: usize) -> Option<(usize, usize, &Tween)> {
    let span = span(layer, frame)?;
    let a = (span.start..=frame)
        .rev()
        .find(|&f| layer.key(f))
        .unwrap_or(span.start);
    let b = (frame + 1..=span.end)
        .find(|&f| layer.key(f))
        .unwrap_or(span.end);
    (a < b).then_some((a, b, span))
}
pub fn marks(layer: &Layer, frame: usize) -> SharedVec<Mark> {
    if layer.key(frame) {
        return layer.frames[frame].clone();
    }
    let Some((a, b, span)) = segment(layer, frame) else {
        return layer.frames[frame].clone();
    };
    let t = (frame - a) as f32 / (b - a) as f32;
    let targets: std::collections::HashMap<_, _> = layer.frames[b]
        .iter()
        .filter(|m| m.style.track != 0)
        .map(|m| (m.style.track, m))
        .collect();
    layer.frames[a]
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let target = targets
                .get(&m.style.track)
                .copied()
                .or_else(|| layer.frames[b].get(i));
            target.map_or_else(|| m.clone(), |n| interpolate(m, n, t, &span.curve))
        })
        .collect()
}
pub fn camera_at(camera: &Camera, frame: usize) -> CameraKey {
    let local = frame.saturating_sub(camera.start).min(camera.duration - 1);
    let base = CameraKey {
        frame: 0,
        center: camera.center,
        zoom: camera.zoom,
        angle: camera.angle,
    };
    let a = camera
        .animation
        .keys
        .iter()
        .rev()
        .find(|k| k.frame <= local)
        .unwrap_or(&base);
    let Some(b) = camera.animation.keys.iter().find(|k| k.frame > local) else {
        return a.clone();
    };
    let t = camera
        .animation
        .curve
        .time((local - a.frame) as f32 / (b.frame - a.frame) as f32);
    CameraKey {
        frame: local,
        center: crate::vector::xy(camera.animation.curve.point(
            crate::vector::pos(a.center),
            crate::vector::pos(b.center),
            t,
        )),
        zoom: a.zoom * (1. - t) + b.zoom * t,
        angle: a.angle + shortest(b.angle - a.angle) * t,
    }
}
pub fn camera_record(camera: &mut Camera, key: CameraKey) {
    if key.frame == 0 {
        camera.center = key.center;
        camera.zoom = key.zoom;
        camera.angle = key.angle;
    }
    if let Some(k) = camera
        .animation
        .keys
        .iter_mut()
        .find(|k| k.frame == key.frame)
    {
        *k = key;
    } else {
        camera.animation.keys.push(key);
        camera.animation.keys.sort_by_key(|k| k.frame);
    }
}
pub fn validate(doc: &Document) -> Result<(), String> {
    for layer in &doc.layers {
        for s in &layer.options.tweens {
            if s.start >= s.end || s.end >= layer.frames.len() || !s.curve.valid() {
                return Err("Invalid motion span".into());
            }
        }
        if layer.options.tweens.len() > 10000 {
            return Err("Too many motion spans".into());
        }
    }
    for c in &doc.extras.cameras {
        if !c.animation.curve.valid()
            || c.animation.keys.len() > 10000
            || c.animation
                .keys
                .windows(2)
                .any(|k| k[0].frame >= k[1].frame)
            || c.animation.keys.iter().any(|k| {
                k.frame >= c.duration
                    || !k.zoom.is_finite()
                    || !(0.01..=100.).contains(&k.zoom)
                    || !k.angle.is_finite()
                    || k.center.iter().any(|v| !v.is_finite() || v.abs() > 1e7)
            })
        {
            return Err("Invalid camera animation".into());
        }
    }
    if doc.extras.scenes.len() > 100
        || (!doc.extras.scenes.is_empty() && doc.extras.active_scene >= doc.extras.scenes.len())
    {
        return Err("Invalid scene catalogue".into());
    }
    let mut ids = std::collections::HashSet::new();
    for (i, s) in doc.extras.scenes.iter().enumerate() {
        if !ids.insert(s.id) || s.layers.is_empty() {
            return Err("Invalid scene".into());
        }
        if i != doc.extras.active_scene {
            scene_doc(doc, i).validate()?;
        }
    }
    if !doc.extras.scenes.is_empty() && movie_count(doc) > 100000 {
        return Err("Movie exceeds 100000 frames".into());
    }
    Ok(())
}
impl Editor {
    pub fn autokey_motion(&mut self, ids: &[(usize, usize)]) {
        self.materialize_motion(ids);
        let mut layers: Vec<_> = ids.iter().map(|v| v.0).collect();
        layers.sort_unstable();
        layers.dedup();
        for l in layers {
            if self.doc.layers[l].options.locked || self.doc.layers[l].key(self.frame) {
                continue;
            }
            let layer = &mut self.doc.layers[l];
            let start = layer.exposure(self.frame).start;
            layer.init_keys();
            let mut source = layer.frames[start].clone();
            for m in &mut source {
                if m.style.track == 0 {
                    m.style.track = crate::features::id();
                }
            }
            layer.frames[start] = source.clone();
            layer.frames[self.frame] = source;
            layer.options.keys[self.frame] = true;
            if start < self.frame {
                layer.options.tweens.push(Tween {
                    start,
                    end: self.frame,
                    curve: Default::default(),
                });
            }
        }
    }
    pub fn materialize_motion(&mut self, ids: &[(usize, usize)]) {
        let mut layers: Vec<_> = ids.iter().map(|i| i.0).collect();
        layers.sort_unstable();
        layers.dedup();
        for l in layers {
            if span(&self.doc.layers[l], self.frame).is_some()
                && !self.doc.layers[l].key(self.frame)
            {
                let marks = marks(&self.doc.layers[l], self.frame);
                let layer = &mut self.doc.layers[l];
                layer.init_keys();
                layer.frames[self.frame] = marks;
                layer.options.keys[self.frame] = true;
            }
        }
    }
    pub fn create_motion(&mut self) {
        let l = self.layer;
        if self.doc.layers[l].options.locked || self.doc.layers[l].frames[self.frame].is_empty() {
            return;
        }
        let ids = self.canvas.selected.clone();
        self.checkpoint();
        let start = self.frame;
        let count = self.doc.layers[l].frames.len();
        let end = (start + 1..count)
            .find(|&f| self.doc.layers[l].key(f) && !self.doc.layers[l].frames[f].is_empty())
            .unwrap_or((start + 9).min(count - 1));
        if end <= start {
            self.canvas.selected = ids;
            return;
        }
        let layer = &mut self.doc.layers[l];
        layer.init_keys();
        layer.options.keys[start] = true;
        let mut source = layer.frames[start].clone();
        for m in &mut source {
            if m.style.track == 0 {
                m.style.track = crate::features::id();
            }
        }
        layer.frames[start] = source.clone();
        if layer.frames[end].is_empty() {
            layer.frames[end] = source.clone();
        } else {
            for (i, m) in layer.frames[end].iter_mut().enumerate() {
                if m.style.track == 0 {
                    m.style.track = source
                        .get(i)
                        .map_or_else(crate::features::id, |m| m.style.track);
                }
            }
        }
        layer.options.keys[end] = true;
        for f in start + 1..end {
            if layer.frames[f].is_empty() || !layer.key(f) {
                layer.options.keys[f] = false;
                layer.frames[f] = source.clone();
            }
        }
        layer
            .options
            .tweens
            .retain(|s| s.end <= start || s.start >= end);
        layer.options.tweens.push(Tween {
            start,
            end,
            curve: Default::default(),
        });
        layer.options.tweens.sort_by_key(|s| s.start);
        self.canvas.selected = ids;
        self.recover();
    }
    pub fn advance_animation(&mut self, steps: usize) {
        if self.animation.play_all && !self.doc.extras.scenes.is_empty() {
            let global = (movie_offset(&self.doc, self.frame) + steps) % movie_count(&self.doc);
            let (index, local) = locate(&self.doc, global);
            if index != self.doc.extras.active_scene {
                self.switch_scene(index);
            }
            self.frame = local;
        } else {
            self.frame = (self.frame + steps) % self.doc.layers[0].frames.len();
        }
    }
}
