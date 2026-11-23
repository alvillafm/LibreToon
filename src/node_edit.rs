//! Explicit node modes and topology tools. Selection never materializes geometry.
use crate::{Editor, Mark, document::Node, vector};
use eframe::egui::{self, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Id = (usize, usize, usize);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Inferred,
    Corner,
    Smooth,
    Symmetric,
    Automatic,
}
impl Mode {
    pub fn inferred(&self) -> bool {
        *self == Self::Inferred
    }
    pub fn effective(self, node: &Node) -> Self {
        if self != Self::Inferred {
            return self;
        }
        let a = vector::pos(node.in_handle) - vector::pos(node.position);
        let b = vector::pos(node.out_handle) - vector::pos(node.position);
        if a.length_sq() > 1e-8
            && b.length_sq() > 1e-8
            && a.normalized().dot(b.normalized()) < -0.999
        {
            Self::Smooth
        } else {
            Self::Corner
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Inferred => "Inferred",
            Self::Corner => "Corner",
            Self::Smooth => "Smooth",
            Self::Symmetric => "Symmetric",
            Self::Automatic => "Automatic",
        }
    }
}
pub(crate) struct State {
    pub all_handles: bool,
    preserve_delete: bool,
    transform: [f32; 5],
    simplify: Option<Simplify>,
    preview_cache: crate::paint::RenderCache,
}
impl Default for State {
    fn default() -> Self {
        Self {
            all_handles: false,
            preserve_delete: true,
            transform: [0., 0., 0., 100., 100.],
            simplify: None,
            preview_cache: Default::default(),
        }
    }
}

pub(crate) fn editable(mark: &Mark) -> Mark {
    if !mark.nodes.is_empty() {
        return mark.clone();
    }
    let mut out = vector::from_cubics(&vector::cubics(mark), mark);
    out.holes = mark.holes.clone();
    out
}
pub(crate) fn supported(mark: &Mark) -> bool {
    mark.style.instance.is_none()
        && mark.style.media_image.is_none()
        && mark.style.text.is_none()
        && !mark.style.raster_tile
}
fn direction(mark: &Mark, index: usize) -> (Vec2, f32, f32) {
    let n = mark.nodes.len();
    let p = vector::pos(mark.nodes[index].position);
    let prev = if index > 0 {
        vector::pos(mark.nodes[index - 1].position)
    } else if mark.closed {
        vector::pos(mark.nodes[n - 1].position)
    } else {
        p
    };
    let next = if index + 1 < n {
        vector::pos(mark.nodes[index + 1].position)
    } else if mark.closed {
        vector::pos(mark.nodes[0].position)
    } else {
        p
    };
    let d = next - prev;
    (
        if d.length_sq() > 1e-8 {
            d.normalized()
        } else {
            Vec2::X
        },
        p.distance(prev) / 3.,
        p.distance(next) / 3.,
    )
}
pub(crate) fn set_mode(mark: &mut Mark, index: usize, mode: Mode) {
    if index >= mark.nodes.len() {
        return;
    }
    let (fallback, prev, next) = direction(mark, index);
    let node = &mut mark.nodes[index];
    let p = vector::pos(node.position);
    let a = p - vector::pos(node.in_handle);
    let b = vector::pos(node.out_handle) - p;
    let axis = if mode == Mode::Automatic {
        fallback
    } else if b.length_sq() > 1e-8 {
        b.normalized()
    } else if a.length_sq() > 1e-8 {
        a.normalized()
    } else {
        fallback
    };
    let (mut inside, mut outside) = (
        if a.length_sq() > 1e-8 {
            a.length()
        } else {
            prev
        },
        if b.length_sq() > 1e-8 {
            b.length()
        } else {
            next
        },
    );
    if mode == Mode::Automatic {
        inside = prev;
        outside = next;
    }
    if mode == Mode::Symmetric {
        let length = if a.length_sq() + b.length_sq() > 1e-8 {
            (a.length() + b.length()) / 2.
        } else {
            prev.max(next)
        };
        inside = length;
        outside = length;
    }
    if mode != Mode::Corner {
        node.in_handle = vector::xy(p - axis * inside);
        node.out_handle = vector::xy(p + axis * outside);
    }
    node.mode = mode;
}
pub(crate) fn update_automatic(mark: &mut Mark) {
    for index in 0..mark.nodes.len() {
        if mark.nodes[index].mode == Mode::Automatic {
            set_mode(mark, index, Mode::Automatic);
        }
    }
}
pub(crate) fn move_handle(
    mark: &mut Mark,
    index: usize,
    handle: u8,
    delta: Vec2,
    independent: bool,
) {
    let Some(node) = mark.nodes.get_mut(index) else {
        return;
    };
    let anchor = vector::pos(node.position);
    let mode = node.mode.effective(node);
    if handle == 0 {
        node.position = vector::xy(anchor + delta);
        node.in_handle = vector::xy(vector::pos(node.in_handle) + delta);
        node.out_handle = vector::xy(vector::pos(node.out_handle) + delta);
    } else if handle == 3 && node.in_handle == node.position && node.out_handle == node.position {
        node.in_handle = vector::xy(anchor - delta);
        node.out_handle = vector::xy(anchor + delta);
        node.mode = Mode::Smooth;
    } else {
        let (moved, other) = if handle == 1 {
            (&mut node.in_handle, &mut node.out_handle)
        } else {
            (&mut node.out_handle, &mut node.in_handle)
        };
        let length = vector::pos(*other).distance(anchor);
        let value = vector::pos(*moved) + delta;
        *moved = vector::xy(value);
        if independent {
            node.mode = Mode::Corner;
        } else if matches!(mode, Mode::Smooth | Mode::Symmetric | Mode::Automatic) {
            let d = value - anchor;
            if d.length_sq() > 1e-8 {
                *other = vector::xy(
                    anchor
                        - d.normalized()
                            * if mode == Mode::Symmetric {
                                d.length()
                            } else {
                                length
                            },
                );
            }
            node.mode = if mode == Mode::Automatic {
                Mode::Smooth
            } else {
                mode
            };
        }
    }
    if handle == 0 {
        update_automatic(mark);
    }
}
pub(crate) fn translate(mark: &Mark, indices: &BTreeSet<usize>, delta: Vec2) -> Mark {
    let mut out = editable(mark);
    for &index in indices {
        if let Some(node) = out.nodes.get_mut(index) {
            for p in [
                &mut node.position,
                &mut node.in_handle,
                &mut node.out_handle,
            ] {
                *p = vector::xy(vector::pos(*p) + delta);
            }
        }
    }
    update_automatic(&mut out);
    out
}
pub(crate) fn insert(mark: &Mark, segment: usize, t: f32) -> Option<Mark> {
    let mut out = editable(mark);
    let curves = vector::cubics(&out);
    let curve = *curves.get(segment)?;
    let t = t.clamp(0.001, 0.999);
    let (a, b) = curve.split(t);
    let next = (segment + 1) % out.nodes.len();
    out.nodes[segment].out_handle = vector::xy(a.0[1]);
    out.nodes[next].in_handle = vector::xy(b.0[2]);
    // Splitting must preserve the curve exactly, even when an endpoint was automatic or symmetric.
    for index in [segment, next] {
        if matches!(out.nodes[index].mode, Mode::Automatic | Mode::Symmetric) {
            out.nodes[index].mode = Mode::Smooth;
        }
    }
    out.nodes.insert(
        segment + 1,
        Node {
            mode: Mode::Smooth,
            position: vector::xy(a.0[3]),
            in_handle: vector::xy(a.0[2]),
            out_handle: vector::xy(b.0[1]),
        },
    );
    Some(out)
}
fn fitted_handles(samples: &[Pos2], a: Pos2, d: Pos2, ta: Vec2, td: Vec2) -> (Pos2, Pos2) {
    let (lengths, total) = crate::width::lengths(samples);
    if total < 1e-6 {
        return (a, d);
    }
    let mut aa = 0.;
    let mut ab = 0.;
    let mut bb = 0.;
    let mut ax = 0.;
    let mut bx = 0.;
    for (p, length) in samples.iter().zip(lengths) {
        let t = length / total;
        let u = 1. - t;
        let b1 = 3. * u * u * t;
        let b2 = 3. * u * t * t;
        let error = *p - (a * (u * u * u + b1) + d.to_vec2() * (b2 + t * t * t));
        let x = ta * b1;
        let y = td * b2;
        aa += x.dot(x);
        ab += x.dot(y);
        bb += y.dot(y);
        ax += x.dot(error);
        bx += y.dot(error);
    }
    let det = aa * bb - ab * ab;
    let chord = a.distance(d).max(total * 0.1);
    let (alpha, beta) = if det.abs() > 1e-8 {
        ((ax * bb - bx * ab) / det, (bx * aa - ax * ab) / det)
    } else {
        (chord / 3., chord / 3.)
    };
    (
        a + ta * alpha.clamp(0., total * 2.),
        d + td * beta.clamp(0., total * 2.),
    )
}
pub(crate) fn remove(mark: &Mark, selected: &BTreeSet<usize>, preserve: bool) -> Option<Mark> {
    let source = editable(mark);
    let n = source.nodes.len();
    let survivors = (0..n).filter(|i| !selected.contains(i)).collect::<Vec<_>>();
    if survivors.len() < 2 {
        return None;
    }
    let mut out = source.clone();
    out.nodes = survivors
        .iter()
        .map(|&i| source.nodes[i].clone())
        .collect::<Vec<_>>()
        .into();
    if preserve {
        let curves = vector::cubics(&source);
        let pairs = survivors.len() - 1 + usize::from(source.closed);
        for i in 0..pairs {
            let j = (i + 1) % survivors.len();
            let first = survivors[i];
            let last = survivors[j];
            let count = if last > first {
                last - first
            } else {
                n - first + last
            };
            if count <= 1 {
                continue;
            }
            let mut samples = vec![vector::pos(source.nodes[first].position)];
            for step in 0..count {
                let curve = curves[(first + step) % curves.len()];
                for t in 1..=16 {
                    samples.push(curve.at(t as f32 / 16.));
                }
            }
            let a = vector::pos(source.nodes[first].position);
            let d = vector::pos(source.nodes[last].position);
            let ta = (vector::pos(source.nodes[first].out_handle) - a).normalized();
            let td = (vector::pos(source.nodes[last].in_handle) - d).normalized();
            let ta = if ta.length_sq() > 0.5 {
                ta
            } else {
                (samples[1] - a).normalized()
            };
            let td = if td.length_sq() > 0.5 {
                td
            } else {
                (samples[samples.len() - 2] - d).normalized()
            };
            let (b, c) = fitted_handles(&samples, a, d, ta, td);
            out.nodes[i].out_handle = vector::xy(b);
            out.nodes[j].in_handle = vector::xy(c);
            out.nodes[i].mode = Mode::Inferred;
            out.nodes[j].mode = Mode::Inferred;
        }
    }
    if !source.closed && (survivors[0] != 0 || *survivors.last().unwrap() + 1 != n) {
        crate::width::remap(mark, &mut out);
    }
    Some(out)
}
fn reversed(mut mark: Mark) -> Mark {
    mark.nodes.reverse();
    for node in &mut mark.nodes {
        std::mem::swap(&mut node.in_handle, &mut node.out_handle);
    }
    mark.width_profile = mark
        .width_profile
        .iter()
        .rev()
        .map(|s| crate::width::WidthStop {
            t: 1. - s.t,
            width: s.width,
        })
        .collect::<Vec<_>>()
        .into();
    std::mem::swap(&mut mark.style.start_cap, &mut mark.style.end_cap);
    mark
}
pub(crate) fn join(
    a: &Mark,
    ai: usize,
    b: &Mark,
    bi: usize,
    merge: bool,
    same: bool,
) -> Result<Mark, String> {
    let mut a = editable(a);
    let mut b = editable(b);
    if a.closed || b.closed || !same && (!a.holes.is_empty() || !b.holes.is_empty()) {
        return Err("Choose endpoints of open paths without holes.".into());
    }
    if ![0, a.nodes.len() - 1].contains(&ai) || ![0, b.nodes.len() - 1].contains(&bi) {
        return Err("Choose two endpoints.".into());
    }
    if same {
        if ai == bi {
            return Err("Choose different endpoints.".into());
        }
        if (merge
            || vector::pos(a.nodes[0].position)
                .distance(vector::pos(a.nodes.last().unwrap().position))
                < 1e-6)
            && a.nodes.len() > 2
        {
            let last = a.nodes.pop().unwrap();
            let node = &mut a.nodes[0];
            let mid = vector::pos(node.position).lerp(vector::pos(last.position), 0.5);
            node.out_handle =
                vector::xy(vector::pos(node.out_handle) + (mid - vector::pos(node.position)));
            node.in_handle =
                vector::xy(vector::pos(last.in_handle) + (mid - vector::pos(last.position)));
            node.position = vector::xy(mid);
            node.mode = Mode::Inferred;
        }
        a.closed = true;
        return Ok(a);
    }
    if a.fill.is_some()
        || b.fill.is_some()
        || a.color != b.color
        || (a.width - b.width).abs() > 1e-4
        || a.style.palette_stroke != b.style.palette_stroke
    {
        return Err("Join separate open strokes with matching color and width. Filled paths can be closed individually.".into());
    }
    let mut style_a = a.style.clone();
    let mut style_b = b.style.clone();
    for style in [&mut style_a, &mut style_b] {
        style.track = 0;
        style.primitive = false;
        style.linked_outline_fill = false;
        style.start_cap = Default::default();
        style.end_cap = Default::default();
    }
    if style_a != style_b {
        return Err(
            "These strokes have different object or rendering settings; keep them separate.".into(),
        );
    }
    if ai == 0 {
        a = reversed(a);
    }
    if bi + 1 == b.nodes.len() {
        b = reversed(b);
    }
    let la = crate::width::lengths(&vector::flatten(&a, 0.1)).1;
    let lb = crate::width::lengths(&vector::flatten(&b, 0.1)).1;
    let gap =
        vector::pos(a.nodes.last().unwrap().position).distance(vector::pos(b.nodes[0].position));
    let total = (la + lb + if merge { 0. } else { gap }).max(1e-6);
    let mut profile = Vec::new();
    if !a.width_profile.is_empty() || !b.width_profile.is_empty() {
        if (merge || gap < 1e-6)
            && (crate::width::sample(&a.width_profile, 1.)
                - crate::width::sample(&b.width_profile, 0.))
            .abs()
                > 1e-4
        {
            return Err("Endpoint widths differ. Connect with a segment or match the endpoint widths before merging.".into());
        }
        for (mark, offset, length) in [(&a, 0., la), (&b, total - lb, lb)] {
            let stops = if mark.width_profile.is_empty() {
                vec![
                    crate::width::WidthStop { t: 0., width: 1. },
                    crate::width::WidthStop { t: 1., width: 1. },
                ]
            } else {
                mark.width_profile.to_vec()
            };
            for stop in stops {
                let value = crate::width::WidthStop {
                    t: (offset + stop.t * length) / total,
                    width: stop.width,
                };
                if profile
                    .last()
                    .is_none_or(|old: &crate::width::WidthStop| (old.t - value.t).abs() > 1e-7)
                {
                    profile.push(value);
                }
            }
        }
        if profile.len() > 128 {
            return Err(
                "The combined width profile exceeds 128 stops; keep these strokes separate.".into(),
            );
        }
    }
    if merge {
        let start = b.nodes.remove(0);
        let end = a.nodes.last_mut().unwrap();
        let mid = vector::pos(end.position).lerp(vector::pos(start.position), 0.5);
        end.in_handle = vector::xy(vector::pos(end.in_handle) + (mid - vector::pos(end.position)));
        end.out_handle =
            vector::xy(vector::pos(start.out_handle) + (mid - vector::pos(start.position)));
        end.position = vector::xy(mid);
        end.mode = Mode::Inferred;
    } else {
        let last = a.nodes.last_mut().unwrap();
        last.out_handle = last.position;
        b.nodes[0].in_handle = b.nodes[0].position;
        last.mode = Mode::Corner;
        b.nodes[0].mode = Mode::Corner;
    }
    a.nodes.extend(b.nodes.iter().cloned());
    a.style.end_cap = b.style.end_cap;
    a.width_profile = profile.into();
    Ok(a)
}
pub(crate) fn split(mark: &Mark, index: usize) -> Vec<Mark> {
    let mut mark = editable(mark);
    let n = mark.nodes.len();
    if n < 2 || index >= n {
        return vec![];
    }
    if mark.closed {
        let mut nodes = mark.nodes[index..].to_vec();
        nodes.extend_from_slice(&mark.nodes[..=index]);
        nodes[0].in_handle = nodes[0].position;
        let last = nodes.last_mut().unwrap();
        last.out_handle = last.position;
        let mut open = mark.clone();
        open.nodes = nodes.into();
        open.closed = false;
        crate::width::remap(&mark, &mut open);
        return vec![open];
    }
    if index == 0 || index + 1 == n {
        return vec![];
    }
    let mut left = mark.clone();
    let mut right = mark.clone();
    left.nodes = mark.nodes[..=index].to_vec().into();
    right.nodes = mark.nodes[index..].to_vec().into();
    left.nodes.last_mut().unwrap().out_handle = left.nodes.last().unwrap().position;
    right.nodes[0].in_handle = right.nodes[0].position;
    left.fill = None;
    right.fill = None;
    left.holes.clear();
    right.holes.clear();
    crate::width::remap(&mark, &mut left);
    crate::width::remap(&mark, &mut right);
    right.style.track = crate::features::id();
    let mut out = Vec::new();
    if mark.fill.is_some() {
        mark.stroke = false;
        mark.style.track = crate::features::id();
        out.push(mark);
    }
    out.extend([left, right]);
    out
}

impl Editor {
    fn node_sources(&self) -> BTreeMap<(usize, usize), Mark> {
        let mut layers = BTreeMap::new();
        self.canvas
            .selected
            .iter()
            .filter_map(|&(l, m)| {
                let marks = layers
                    .entry(l)
                    .or_insert_with(|| crate::animation::marks(&self.doc.layers[l], self.frame));
                marks.get(m).map(|mark| ((l, m), mark.clone()))
            })
            .collect()
    }
    fn selected_nodes(&self) -> Vec<Id> {
        let mut counts = BTreeMap::new();
        self.canvas
            .node_ids()
            .into_iter()
            .filter(|&(l, m, n)| {
                let count = counts.entry((l, m)).or_insert_with(|| {
                    self.doc
                        .layers
                        .get(l)
                        .filter(|layer| layer.visible && !layer.options.locked)
                        .and_then(|layer| layer.frames[self.frame].get(m))
                        .filter(|mark| supported(mark))
                        .map_or(0, |mark| editable(mark).nodes.len())
                });
                n < *count
            })
            .collect()
    }
    fn groups(&self) -> BTreeMap<(usize, usize), BTreeSet<usize>> {
        let mut groups: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for (l, m, n) in self.selected_nodes() {
            if self.doc.layers.get(l).is_some_and(|layer| {
                layer.visible
                    && !layer.options.locked
                    && layer.frames[self.frame].get(m).is_some_and(supported)
            }) {
                groups.entry((l, m)).or_default().insert(n);
            }
        }
        groups
    }
    fn change_nodes(&mut self, mut change: impl FnMut(&mut Mark, &BTreeSet<usize>)) {
        let mut updates = Vec::new();
        for ((l, m), indices) in self.groups() {
            let original = crate::animation::marks(&self.doc.layers[l], self.frame)[m].clone();
            let mut mark = editable(&original);
            change(&mut mark, &indices);
            if mark != original {
                if mark.fill.is_some() && mark.stroke {
                    mark.style.linked_outline_fill = true;
                }
                updates.push(((l, m), mark));
            }
        }
        if updates.is_empty() {
            return;
        }
        let selected = self.canvas.selected.clone();
        let nodes = self.selected_nodes();
        self.checkpoint();
        self.materialize_motion(&selected);
        for ((l, m), mark) in updates {
            self.doc.layers[l].frames[self.frame][m] = mark;
        }
        self.canvas.selected = selected;
        self.canvas.set_node_ids(nodes);
        self.recover();
    }
    pub(crate) fn node_mode(&mut self, mode: Mode) {
        self.change_nodes(|mark, indices| {
            for &n in indices {
                set_mode(mark, n, mode);
            }
        });
    }
    pub(crate) fn select_all_nodes(&mut self) {
        let mut nodes = Vec::new();
        for &(l, m) in &self.canvas.selected {
            let marks = crate::animation::marks(&self.doc.layers[l], self.frame);
            if let Some(mark) = marks.get(m).filter(|m| supported(m)) {
                nodes.extend((0..editable(mark).nodes.len()).map(|n| (l, m, n)));
            }
        }
        self.canvas.set_node_ids(nodes);
    }
    pub(crate) fn delete_nodes(&mut self) -> bool {
        let groups = self.groups();
        if groups.is_empty() {
            return false;
        }
        let selected = self.canvas.selected.clone();
        self.checkpoint();
        self.materialize_motion(&selected);
        let mut remain = Vec::new();
        for ((l, m), indices) in groups.into_iter().rev() {
            let marks = &mut self.doc.layers[l].frames[self.frame];
            if let Some(mut mark) = remove(&marks[m], &indices, self.nodes.preserve_delete) {
                if mark.fill.is_some() && mark.stroke {
                    mark.style.linked_outline_fill = true;
                }
                marks[m] = mark;
                remain.push((l, m));
            } else {
                marks.remove(m);
                for (layer, index) in &mut remain {
                    if *layer == l && *index > m {
                        *index -= 1;
                    }
                }
            }
        }
        self.canvas.selected = remain;
        self.canvas.set_node_ids(Vec::new());
        self.recover();
        true
    }
    fn node_segment(&mut self, straight: bool) {
        self.change_nodes(|mark, indices| {
            let count = mark.nodes.len();
            for n in 0..count {
                let next = (n + 1) % count;
                if !mark.closed && next == 0 {
                    continue;
                }
                if !indices.contains(&n) || !indices.contains(&next) {
                    continue;
                }
                let a = vector::pos(mark.nodes[n].position);
                let b = vector::pos(mark.nodes[next].position);
                mark.nodes[n].out_handle =
                    vector::xy(if straight { a } else { a.lerp(b, 1. / 3.) });
                mark.nodes[next].in_handle =
                    vector::xy(if straight { b } else { a.lerp(b, 2. / 3.) });
                mark.nodes[n].mode = Mode::Corner;
                mark.nodes[next].mode = Mode::Corner;
            }
        });
    }
    fn align_nodes(&mut self, axis: usize, distribute: bool) {
        let nodes = self.selected_nodes();
        let sources = self.node_sources();
        let mut points: Vec<_> = nodes
            .iter()
            .filter_map(|&(l, m, n)| {
                editable(&sources[&(l, m)]).nodes.get(n).map(|p| {
                    (
                        (l, m, n),
                        self.depth_point(vector::pos(p.position), l, self.layer),
                    )
                })
            })
            .collect();
        if points.len() < 2 || distribute && points.len() < 3 {
            return;
        }
        points.sort_by(|a, b| a.1[axis].total_cmp(&b.1[axis]));
        let min = points.first().unwrap().1[axis];
        let max = points.last().unwrap().1[axis];
        let mut targets = BTreeMap::new();
        let count = points.len();
        for (i, (id, mut point)) in points.into_iter().enumerate() {
            point[axis] = if distribute {
                min + (max - min) * i as f32 / (count - 1) as f32
            } else {
                (min + max) / 2.
            };
            targets.insert(id, self.depth_point(point, self.layer, id.0));
        }
        let selected = self.canvas.selected.clone();
        self.checkpoint();
        self.materialize_motion(&selected);
        for ((l, m, n), point) in targets {
            let marks = &mut self.doc.layers[l].frames[self.frame];
            let mut mark = editable(&marks[m]);
            let delta = point - vector::pos(mark.nodes[n].position);
            move_handle(&mut mark, n, 0, delta, false);
            marks[m] = mark;
        }
        self.canvas.selected = selected;
        self.canvas.set_node_ids(nodes);
        self.recover();
    }
    fn transform_nodes(&mut self, values: [f32; 5]) {
        if !values.iter().all(|v| v.is_finite()) {
            return;
        }
        let ids = self.selected_nodes();
        let sources = self.node_sources();
        if ids.is_empty() {
            return;
        }
        let mut bounds = Rect::NOTHING;
        for &(l, m, n) in &ids {
            if let Some(node) = editable(&sources[&(l, m)]).nodes.get(n) {
                bounds.extend_with(self.depth_point(vector::pos(node.position), l, self.layer));
            }
        }
        let center = bounds.center();
        let scale = Vec2::new(values[3] / 100., values[4] / 100.);
        let angle = values[2].to_radians();
        let offset = Vec2::new(values[0], values[1]);
        let mut updates = Vec::new();
        for ((l, m), indices) in self.groups() {
            let mut mark = editable(&sources[&(l, m)]);
            for n in indices {
                if let Some(node) = mark.nodes.get_mut(n) {
                    for p in [
                        &mut node.position,
                        &mut node.in_handle,
                        &mut node.out_handle,
                    ] {
                        let world = self.depth_point(vector::pos(*p), l, self.layer);
                        *p = vector::xy(self.depth_point(
                            center
                                + crate::canvas::rotate((world - center) * scale, angle)
                                + offset,
                            self.layer,
                            l,
                        ));
                    }
                }
            }
            update_automatic(&mut mark);
            updates.push(((l, m), mark));
        }
        let paths = self.canvas.selected.clone();
        self.checkpoint();
        self.materialize_motion(&paths);
        for ((l, m), mark) in updates {
            self.doc.layers[l].frames[self.frame][m] = mark;
        }
        self.canvas.selected = paths;
        self.canvas.set_node_ids(ids);
        self.recover();
    }
    fn join_nodes(&mut self, merge: bool) {
        let nodes = self.selected_nodes();
        if nodes.len() != 2 {
            return;
        }
        let [(l, a, ai), (other, b, bi)] = [nodes[0], nodes[1]];
        if l != other {
            self.status = "Choose endpoints on the same vector layer.".into();
            return;
        }
        let evaluated = crate::animation::marks(&self.doc.layers[l], self.frame);
        let result = join(&evaluated[a], ai, &evaluated[b], bi, merge, a == b);
        let mut mark = match result {
            Ok(mark) => mark,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        if a != b {
            mark.style.track = crate::features::id();
        }
        self.checkpoint();
        self.materialize_motion(&[(l, a), (l, b)]);
        let first = a.min(b);
        let marks = &mut self.doc.layers[l].frames[self.frame];
        if a != b {
            marks.remove(a.max(b));
        }
        marks[first] = mark;
        self.canvas.selected = vec![(l, first)];
        self.canvas.set_node_ids(Vec::new());
        self.recover();
    }
    fn split_node(&mut self) {
        let Some(&(l, m, n)) = self
            .selected_nodes()
            .first()
            .filter(|_| self.selected_nodes().len() == 1)
        else {
            return;
        };
        let evaluated = crate::animation::marks(&self.doc.layers[l], self.frame);
        let pieces = split(&evaluated[m], n);
        if pieces.is_empty() {
            self.status = "Choose an interior node, or any node on a closed path.".into();
            return;
        }
        let count = pieces.len();
        self.checkpoint();
        self.materialize_motion(&[(l, m)]);
        self.doc.layers[l].frames[self.frame].splice(m..=m, pieces);
        self.canvas.selected = (m..m + count).map(|m| (l, m)).collect();
        self.canvas.set_node_ids(Vec::new());
        self.recover();
    }
    fn separate_fill_stroke(&mut self) {
        let selected = self.canvas.selected.clone();
        let mut changes = selected
            .iter()
            .copied()
            .filter(|&(l, m)| {
                self.doc.layers.get(l).is_some_and(|layer| {
                    !layer.options.locked
                        && layer.frames[self.frame].get(m).is_some_and(|mark| {
                            supported(mark) && mark.stroke && mark.fill.is_some()
                        })
                })
            })
            .collect::<Vec<_>>();
        if changes.is_empty() {
            return;
        }
        changes.sort_unstable();
        changes.dedup();
        self.checkpoint();
        self.materialize_motion(&selected);
        let mut lines = Vec::new();
        for (l, m) in changes.into_iter().rev() {
            let marks = &mut self.doc.layers[l].frames[self.frame];
            let mut fill = marks[m].clone();
            let mut stroke = fill.clone();
            fill.stroke = false;
            fill.style.palette_stroke = None;
            fill.style.track = crate::features::id();
            fill.style.linked_outline_fill = false;
            stroke.fill = None;
            stroke.style.palette_fill = None;
            stroke.style.gradient = None;
            stroke.style.linked_outline_fill = false;
            marks[m] = fill;
            marks.insert(m + 1, stroke);
            for (layer, index) in &mut lines {
                if *layer == l && *index > m {
                    *index += 1;
                }
            }
            lines.push((l, m + 1));
        }
        self.canvas.selected = lines;
        self.canvas.set_node_ids(Vec::new());
        self.recover();
    }
    pub(crate) fn insert_node_at(
        &mut self,
        state: &mut crate::canvas::Canvas,
        world: Pos2,
        scale: f32,
    ) -> bool {
        let mut best = None;
        let mut distance = 8. / scale.max(0.001);
        let mut evaluated = BTreeMap::new();
        for &(l, m) in &state.selected {
            let marks = evaluated
                .entry(l)
                .or_insert_with(|| crate::animation::marks(&self.doc.layers[l], self.frame));
            let mark = &marks[m];
            if !supported(mark) {
                continue;
            }
            let p = self.depth_point(world, self.layer, l);
            if editable(mark)
                .nodes
                .iter()
                .any(|node| vector::pos(node.position).distance(p) < 8. / scale.max(0.001))
            {
                continue;
            }
            for (segment, curve) in vector::cubics(mark).into_iter().enumerate() {
                // Bracket with bounded samples, then refine the nearest parameter without flattening the path.
                let mut t = 0.;
                let mut d = f32::INFINITY;
                for i in 0..=24 {
                    let value = i as f32 / 24.;
                    let e = curve.at(value).distance_sq(p);
                    if e < d {
                        d = e;
                        t = value;
                    }
                }
                let (mut left, mut right) = ((t - 1. / 24.).max(0.), (t + 1. / 24.).min(1.));
                for _ in 0..16 {
                    let a = left + (right - left) / 3.;
                    let b = right - (right - left) / 3.;
                    if curve.at(a).distance_sq(p) < curve.at(b).distance_sq(p) {
                        right = b;
                    } else {
                        left = a;
                    }
                }
                t = (left + right) / 2.;
                let d = curve.at(t).distance(p);
                if d < distance && (0.002..0.998).contains(&t) {
                    distance = d;
                    best = Some((l, m, segment, t));
                }
            }
        }
        let Some((l, m, segment, t)) = best else {
            return false;
        };
        let Some(mark) = insert(&evaluated[&l][m], segment, t) else {
            return false;
        };
        let paths = state.selected.clone();
        self.checkpoint();
        self.materialize_motion(&paths);
        self.doc.layers[l].frames[self.frame][m] = mark;
        state.set_node_ids(vec![(l, m, segment + 1)]);
        self.recover();
        true
    }
    pub(crate) fn node_properties(&mut self, ui: &mut egui::Ui) {
        use crate::design::{Icon, button};
        if !matches!(
            self.tool,
            crate::canvas::Tool::Node | crate::canvas::Tool::Pen
        ) {
            return;
        }
        crate::design::category(ui, "VECTOR NODES");
        let ids = self.selected_nodes();
        let count = ids.len();
        ui.label(format!("{count} selected nodes"));
        let mode = ids
            .first()
            .and_then(|&(l, m, n)| {
                self.doc.layers.get(l)?.frames[self.frame]
                    .get(m)
                    .map(editable)?
                    .nodes
                    .get(n)
                    .cloned()
            })
            .map(|n| n.mode.effective(&n));
        ui.horizontal_wrapped(|ui| {
            for (kind, icon) in [
                (Mode::Corner, Icon::NodeCorner),
                (Mode::Smooth, Icon::NodeSmooth),
                (Mode::Symmetric, Icon::NodeSymmetric),
                (Mode::Automatic, Icon::NodeAuto),
            ] {
                if button(
                    ui,
                    icon,
                    kind.label(),
                    kind.label(),
                    mode == Some(kind),
                    count > 0,
                )
                .clicked()
                {
                    self.node_mode(kind);
                }
            }
        });
        ui.horizontal(|ui| {
            if button(
                ui,
                Icon::Node,
                "Select all nodes",
                "Ctrl/Cmd+A",
                false,
                !self.canvas.selected.is_empty(),
            )
            .clicked()
            {
                self.select_all_nodes();
            }
            ui.checkbox(&mut self.nodes.all_handles, "All handles");
        });
        ui.horizontal_wrapped(|ui| {
            if button(
                ui,
                Icon::Line,
                "Straight segment",
                "Select both endpoints of a segment",
                false,
                count >= 2,
            )
            .clicked()
            {
                self.node_segment(true);
            }
            if button(
                ui,
                Icon::NodeSmooth,
                "Curve segment",
                "Select both endpoints of a segment",
                false,
                count >= 2,
            )
            .clicked()
            {
                self.node_segment(false);
            }
        });
        crate::design::category(ui, "PATH OPERATIONS");
        ui.horizontal_wrapped(|ui| {
            if button(
                ui,
                Icon::JoinNodes,
                "Connect",
                "Connect two selected endpoints",
                false,
                count == 2,
            )
            .clicked()
            {
                self.join_nodes(false);
            }
            if button(
                ui,
                Icon::JoinNodes,
                "Merge",
                "Merge selected endpoints at their midpoint",
                false,
                count == 2,
            )
            .clicked()
            {
                self.join_nodes(true);
            }
            if button(
                ui,
                Icon::SplitNodes,
                "Break / Open",
                "Break at one node; open closed paths without removing their fill",
                false,
                count == 1,
            )
            .clicked()
            {
                self.split_node();
            }
        });
        ui.checkbox(
            &mut self.nodes.preserve_delete,
            "Preserve curve when deleting nodes",
        );
        if button(
            ui,
            Icon::Trash,
            "Delete nodes",
            "Delete / Backspace",
            false,
            count > 0,
        )
        .clicked()
        {
            self.delete_nodes();
        }
        let combined = self.canvas.selected.iter().any(|&(l, m)| {
            self.doc.layers[l].frames[self.frame]
                .get(m)
                .is_some_and(|mark| mark.stroke && mark.fill.is_some())
        });
        if combined
            && button(
                ui,
                Icon::Layers,
                "Separate fill / stroke",
                "Make fill and outline independently editable",
                false,
                true,
            )
            .clicked()
        {
            self.separate_fill_stroke();
        }
        ui.collapsing("Align and distribute", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (axis, label) in [(0, "Same X"), (1, "Same Y")] {
                    if button(
                        ui,
                        Icon::Node,
                        label,
                        "Align selected anchors",
                        false,
                        count >= 2,
                    )
                    .clicked()
                    {
                        self.align_nodes(axis, false);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (axis, label) in [(0, "Distribute X"), (1, "Distribute Y")] {
                    if button(
                        ui,
                        Icon::Node,
                        label,
                        "Distribute selected anchors evenly",
                        false,
                        count >= 3,
                    )
                    .clicked()
                    {
                        self.align_nodes(axis, true);
                    }
                }
            });
        });
        ui.collapsing("Transform selected nodes", |ui| {
            for (i, label) in ["Move X", "Move Y", "Rotation °", "Scale X %", "Scale Y %"]
                .into_iter()
                .enumerate()
            {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(
                        egui::DragValue::new(&mut self.nodes.transform[i]).range(if i >= 3 {
                            0.1..=10000.
                        } else {
                            -100000.0..=100000.0
                        }),
                    );
                });
            }
            if button(
                ui,
                Icon::Transform,
                "Apply transform",
                "Transform the selected anchors and their handles",
                false,
                count > 0 && self.nodes.transform != [0., 0., 0., 100., 100.],
            )
            .clicked()
            {
                let values = self.nodes.transform;
                self.transform_nodes(values);
                self.nodes.transform = [0., 0., 0., 100., 100.];
            }
        });
        if button(
            ui,
            Icon::Simplify,
            "Simplify path…",
            "Preview fewer nodes before applying",
            false,
            !self.canvas.selected.is_empty(),
        )
        .clicked()
        {
            self.begin_simplify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve() -> Mark {
        vector::from_cubics(
            &[vector::Cubic([
                Pos2::new(0., 0.),
                Pos2::new(35., -70.),
                Pos2::new(65., 70.),
                Pos2::new(100., 0.),
            ])],
            &Mark {
                width: 8.,
                ..Default::default()
            },
        )
    }
    #[test]
    fn modes_link_handles_or_break_them_and_persist_natively() {
        let mut mark = Mark {
            nodes: vec![Node {
                position: [100., 100.],
                in_handle: [90., 100.],
                out_handle: [130., 100.],
                mode: Mode::Smooth,
            }]
            .into(),
            ..Default::default()
        };
        move_handle(&mut mark, 0, 2, Vec2::new(0., 40.), false);
        assert!(
            (vector::pos(mark.nodes[0].in_handle).distance(Pos2::new(100., 100.)) - 10.).abs()
                < 0.001
        );
        set_mode(&mut mark, 0, Mode::Symmetric);
        move_handle(&mut mark, 0, 2, Vec2::new(15., -10.), false);
        assert!(
            (vector::pos(mark.nodes[0].in_handle)
                + vector::pos(mark.nodes[0].out_handle).to_vec2()
                - Pos2::new(200., 200.))
            .length()
                < 0.001
        );
        let other = mark.nodes[0].in_handle;
        move_handle(&mut mark, 0, 2, Vec2::new(10., 15.), true);
        assert_eq!(mark.nodes[0].in_handle, other);
        assert_eq!(mark.nodes[0].mode, Mode::Corner);
        let mut doc = crate::Document::default();
        doc.layers[0].frames[0] = vec![mark].into();
        let bytes = crate::native_project::encode(&doc).unwrap();
        let restored = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        assert_eq!(restored.layers[0].frames[0][0].nodes[0].mode, Mode::Corner);
        let old = r#"{"position":[100,100],"in_handle":[90,100],"out_handle":[130,100]}"#;
        let node: Node = serde_json::from_str(old).unwrap();
        assert_eq!(node.mode, Mode::Inferred);
        assert_eq!(node.mode.effective(&node), Mode::Smooth);
    }
    #[test]
    fn automatic_nodes_follow_neighbors_without_changing_other_node_positions() {
        let mut mark = Mark {
            nodes: vec![
                Node::corner([0., 0.]),
                Node::corner([50., 30.]),
                Node::corner([100., 0.]),
            ]
            .into(),
            ..Default::default()
        };
        set_mode(&mut mark, 1, Mode::Automatic);
        let result = translate(&mark, &BTreeSet::from([2]), Vec2::new(0., 60.));
        assert_eq!(result.nodes[1].position, [50., 30.]);
        let a = vector::pos(result.nodes[1].in_handle) - Pos2::new(50., 30.);
        let b = vector::pos(result.nodes[1].out_handle) - Pos2::new(50., 30.);
        assert!(a.normalized().dot(b.normalized()) < -0.999);
        assert_ne!(result.nodes[1].out_handle, mark.nodes[1].out_handle);
    }
    #[test]
    fn insertion_preserves_every_point_of_the_original_cubic_and_its_width_profile() {
        let mut mark = curve();
        mark.width_profile = crate::width::Preset::Tapered.stops();
        let split = insert(&mark, 0, 0.37).unwrap();
        let before = vector::cubics(&mark)[0];
        let after = vector::cubics(&split);
        for i in 0..=200 {
            let t = i as f32 / 200.;
            let p = if t <= 0.37 {
                after[0].at(t / 0.37)
            } else {
                after[1].at((t - 0.37) / 0.63)
            };
            assert!(p.distance(before.at(t)) < 0.001);
        }
        assert_eq!(mark.width_profile, split.width_profile);
    }
    #[test]
    fn connecting_reversed_endpoints_preserves_curves_and_exact_width_stops() {
        let mut a = curve();
        a.width_profile = vec![
            crate::width::WidthStop { t: 0., width: 0.5 },
            crate::width::WidthStop { t: 0.3, width: 1.2 },
            crate::width::WidthStop { t: 1., width: 1. },
        ]
        .into();
        let mut b = vector::transform(&curve(), |p| p + Vec2::X * 150.);
        b.width_profile = vec![
            crate::width::WidthStop { t: 0., width: 1. },
            crate::width::WidthStop { t: 1., width: 0.25 },
        ]
        .into();
        let a = reversed(a);
        let joined = join(&a, 0, &b, 0, false, false).unwrap();
        assert_eq!(joined.nodes.first().unwrap().position, [0., 0.]);
        assert_eq!(joined.nodes.last().unwrap().position, [250., 0.]);
        assert!(crate::width::validate(&joined.width_profile));
        assert!(
            joined
                .width_profile
                .iter()
                .any(|p| (p.width - 1.2).abs() < 1e-6)
        );
        assert_eq!(
            vector::cubics(&joined)[0].0,
            vector::cubics(&reversed(a))[0].0
        );
    }
    #[test]
    fn opening_and_reclosing_a_filled_path_preserves_shape_and_holes() {
        let mut mark = crate::editing::shape(
            crate::canvas::Tool::Rectangle,
            Pos2::new(0., 0.),
            Pos2::new(100., 100.),
            4,
            false,
            0.5,
            &Mark {
                fill: Some([200, 20, 50, 255]),
                ..Default::default()
            },
        );
        mark.holes = vec![vec![[25., 25.], [75., 25.], [75., 75.], [25., 75.]].into()].into();
        let open = split(&mark, 2).pop().unwrap();
        assert!(!open.closed);
        assert_eq!(open.fill, mark.fill);
        assert_eq!(open.holes, mark.holes);
        let closed = join(&open, 0, &open, open.nodes.len() - 1, false, true).unwrap();
        assert!(closed.closed);
        assert_eq!(closed.nodes.len(), mark.nodes.len());
        use geo::{Area, BooleanOps};
        let difference =
            geo::MultiPolygon(vec![vector::polygon(&mark)]).xor(&vector::polygon(&closed));
        assert!(difference.unsigned_area() < 0.001);
    }
    #[test]
    fn deleting_a_run_preserves_endpoints_without_overshooting_straight_art() {
        let mark = Mark {
            nodes: (0..100)
                .map(|i| Node::corner([i as f32, 20.]))
                .collect::<Vec<_>>()
                .into(),
            ..Default::default()
        };
        let reduced = remove(&mark, &(1..99).collect(), true).unwrap();
        assert_eq!(reduced.nodes.len(), 2);
        assert_eq!(reduced.nodes[0].position, [0., 20.]);
        assert_eq!(reduced.nodes[1].position, [99., 20.]);
        for p in vector::flatten(&reduced, 0.01) {
            assert!((p.y - 20.).abs() < 0.001 && p.x >= -0.001 && p.x <= 99.001);
        }
    }
    #[test]
    fn simplify_reduces_a_dense_closed_contour_and_keeps_native_style() {
        let mark = Mark {
            points: (0..128)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 128.;
                    [100. + 80. * a.cos(), 100. + 80. * a.sin()]
                })
                .collect::<Vec<_>>()
                .into(),
            closed: true,
            fill: Some([20, 100, 220, 255]),
            width: 6.,
            ..Default::default()
        };
        let original = mark.clone();
        let simplified = simplified(&mark, 0.5, true, &Default::default()).unwrap();
        assert!(simplified.nodes.len() < 20);
        assert_eq!(simplified.fill, mark.fill);
        assert_eq!(simplified.width, 6.);
        assert!(simplified.closed);
        assert!(mark == original);
        use geo::Area;
        let ratio =
            vector::polygon(&simplified).unsigned_area() / vector::polygon(&mark).unsigned_area();
        assert!((ratio - 1.).abs() < 0.02, "area ratio {ratio}");
        let mut doc = crate::Document::default();
        doc.layers[0].frames[0] = vec![simplified].into();
        doc.validate().unwrap();
    }
    #[test]
    fn numeric_node_transform_uses_evaluated_tween_and_undoes_once() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        e.doc = crate::fresh_document();
        let mut mark = curve();
        mark.style.track = crate::features::id();
        for frame in &mut e.doc.layers[0].frames {
            *frame = vec![mark.clone()].into();
        }
        e.doc.layers[0].options.keys[10] = true;
        e.doc.layers[0]
            .options
            .tweens
            .push(crate::animation::Tween {
                shape: false,
                start: 0,
                end: 10,
                curve: Default::default(),
            });
        e.frame = 10;
        e.autokey_motion(&[(0, 0)]);
        e.doc.layers[0].frames[10][0] = vector::transform(&mark, |p| p + Vec2::new(100., 0.));
        e.frame = 5;
        e.canvas.selected = vec![(0, 0)];
        e.canvas.set_node_ids(vec![(0, 0, 0)]);
        let displayed = crate::animation::marks(&e.doc.layers[0], 5)[0].nodes[0].position;
        let before = serde_json::to_value(&e.doc).unwrap();
        let history = e.undo.len();
        e.transform_nodes([f32::NAN, 0., 0., 100., 100.]);
        assert_eq!(e.undo.len(), history);
        e.transform_nodes([15., 20., 0., 100., 100.]);
        assert_eq!(
            e.doc.layers[0].frames[5][0].nodes[0].position,
            [displayed[0] + 15., displayed[1] + 20.]
        );
        assert_eq!(e.undo.len(), history + 1);
        e.history(false);
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), before);
    }
    #[test]
    fn canceled_preview_does_not_change_art_or_history() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        e.doc = crate::fresh_document();
        e.doc.layers[0].frames[0] = vec![curve()].into();
        e.canvas.selected = vec![(0, 0)];
        let before = serde_json::to_value(&e.doc).unwrap();
        let undo = e.undo.len();
        e.begin_simplify();
        for _ in 0..4 {
            let mut out = ctx.run_ui(Default::default(), |_| e.node_simplify_ui(&ctx));
            out.textures_delta.clear();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        e.nodes.simplify.as_mut().unwrap().open = false;
        let mut out = ctx.run_ui(Default::default(), |_| e.node_simplify_ui(&ctx));
        out.textures_delta.clear();
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), before);
        assert_eq!(e.undo.len(), undo);
    }
}

type Paths = Vec<((usize, usize), Mark)>;
struct Candidate {
    paths: Paths,
    before: usize,
    after: usize,
    preserve: bool,
}
struct Simplify {
    open: bool,
    owner: std::path::PathBuf,
    frame: usize,
    epoch: usize,
    originals: Paths,
    tolerance: f32,
    preserve: bool,
    job: Option<std::sync::mpsc::Receiver<Result<(f32, Candidate), String>>>,
    requested: Option<(u32, bool)>,
    result: Option<(f32, Candidate)>,
    error: Option<String>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
fn simplified(
    mark: &Mark,
    tolerance: f32,
    preserve: bool,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Mark, String> {
    let source = editable(mark);
    let curves = vector::cubics(&source);
    if curves.is_empty() {
        return Ok(mark.clone());
    }
    if curves.len() > 6000 {
        return Err("Simplify one smaller path at a time (maximum 6000 segments).".into());
    }
    let mut corners = BTreeSet::new();
    let mut cuts = BTreeSet::from([0, curves.len()]);
    for (index, node) in source.nodes.iter().enumerate() {
        let n = source.nodes.len();
        let prev = vector::pos(source.nodes[(index + n - 1) % n].position);
        let p = vector::pos(node.position);
        let next = vector::pos(source.nodes[(index + 1) % n].position);
        let sharp = preserve
            && (node.mode == Mode::Corner
                || node.mode == Mode::Inferred
                    && (p - prev).length_sq() > 1e-8
                    && (next - p).length_sq() > 1e-8
                    && (p - prev).normalized().dot((next - p).normalized()) < 0.819);
        if sharp {
            cuts.insert(index.min(curves.len()));
            corners.insert((node.position[0].to_bits(), node.position[1].to_bits()));
        }
    }
    if source.closed && cuts.len() == 2 {
        cuts.insert(curves.len() / 2);
    }
    let cuts = cuts.into_iter().collect::<Vec<_>>();
    let mut reduced = Vec::new();
    for range in cuts.windows(2) {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Simplification canceled".into());
        }
        let mut points = vec![vector::xy(curves[range[0]].0[0])];
        for curve in &curves[range[0]..range[1]] {
            for step in 1..=16 {
                points.push(vector::xy(curve.at(step as f32 / 16.)));
            }
        }
        reduced.extend(vector::fit(&points, tolerance.max(0.01)));
    }
    let mut out = vector::from_cubics(&reduced, mark);
    out.holes = mark.holes.clone();
    out.closed = source.closed;
    for node in &mut out.nodes {
        if corners.contains(&(node.position[0].to_bits(), node.position[1].to_bits())) {
            node.mode = Mode::Corner;
        }
    }
    if out.nodes.len() >= source.nodes.len() {
        Ok(mark.clone())
    } else {
        Ok(out)
    }
}
impl Editor {
    fn begin_simplify(&mut self) {
        if self
            .nodes
            .simplify
            .as_ref()
            .is_some_and(|s| s.job.is_some())
        {
            self.status = "Finishing the previous simplify preview…".into();
            return;
        }
        let mut originals = Vec::new();
        for &(l, m) in &self.canvas.selected {
            if let Some(mark) = crate::animation::marks(&self.doc.layers[l], self.frame)
                .get(m)
                .filter(|m| supported(m))
            {
                originals.push(((l, m), mark.clone()));
            }
        }
        if originals.is_empty() {
            return;
        }
        self.nodes.simplify = Some(Simplify {
            open: true,
            owner: self.recovery.clone(),
            frame: self.frame,
            epoch: self.doc.layers.identity(),
            originals,
            tolerance: 0.5,
            preserve: true,
            job: None,
            requested: None,
            result: None,
            error: None,
            cancel: Default::default(),
        });
    }
    pub(crate) fn node_preview_active(&self) -> bool {
        self.nodes.simplify.as_ref().is_some_and(|s| {
            s.open
                && s.owner == self.recovery
                && s.frame == self.frame
                && s.epoch == self.doc.layers.identity()
                && s.result.as_ref().is_some_and(|(tol, c)| {
                    tol.to_bits() == s.tolerance.to_bits() && c.preserve == s.preserve
                })
        })
    }
    pub(crate) fn node_simplify_overlays(
        &mut self,
        painter: &egui::Painter,
        view: crate::canvas::View,
    ) {
        if !self.node_preview_active() {
            return;
        }
        let paths = self
            .nodes
            .simplify
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .1
            .paths
            .clone();
        self.nodes.preview_cache.begin_frame();
        for ((l, _), mut mark) in paths {
            let local = self.camera_depth_view(view, l);
            mark.fill = None;
            mark.stroke = true;
            mark.color = [75, 222, 207, 255];
            mark.width = (1.5 / local.scale.max(0.001)).clamp(0.01, 100.);
            mark.width_profile.clear();
            crate::paint::draw(
                painter,
                &mark,
                local,
                false,
                false,
                Some(&mut self.nodes.preview_cache),
            );
            for node in &editable(&mark).nodes {
                painter.circle_filled(
                    local.screen(vector::pos(node.position)),
                    2.5,
                    egui::Color32::from_rgb(75, 222, 207),
                );
            }
        }
    }
    pub(crate) fn node_simplify_ui(&mut self, ctx: &egui::Context) {
        let Some(mut preview) = self.nodes.simplify.take() else {
            return;
        };
        if preview.owner != self.recovery
            || preview.frame != self.frame
            || preview.epoch != self.doc.layers.identity()
        {
            preview.open = false;
            preview
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = &preview.job {
            let result = match job.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Simplify preview interrupted".into()))
                }
                _ => None,
            };
            if let Some(result) = result {
                preview.job = None;
                match result {
                    Ok(result) => {
                        preview.result = Some(result);
                        preview.error = None;
                    }
                    Err(error) => preview.error = Some(error),
                }
            }
        }
        let mut open = preview.open;
        let mut apply = false;
        let mut canceled = false;
        if preview.open {
            egui::Window::new("Simplify Vector Path")
                .open(&mut open)
                .collapsible(false)
                .default_width(380.)
                .show(ctx, |ui| {
                    ui.label("The original drawing stays unchanged until Apply.");
                    ui.add(
                        egui::Slider::new(&mut preview.tolerance, 0.05..=10.)
                            .logarithmic(true)
                            .text("Tolerance · px"),
                    );
                    ui.checkbox(&mut preview.preserve, "Preserve corner nodes");
                    ui.label(
                        egui::RichText::new("Cyan = candidate contour and its nodes")
                            .color(egui::Color32::from_rgb(75, 222, 207)),
                    );
                    if preview.job.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Calculating preview…");
                        });
                    }
                    let ready = preview.result.as_ref().filter(|(tol, c)| {
                        tol.to_bits() == preview.tolerance.to_bits()
                            && c.preserve == preview.preserve
                    });
                    if let Some((_, result)) = ready {
                        ui.label(format!("{} → {} nodes", result.before, result.after));
                        if result.after >= result.before {
                            ui.label("No reduction at this tolerance.");
                        }
                    }
                    if let Some(error) = &preview.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    ui.horizontal(|ui| {
                        apply = crate::design::button(
                            ui,
                            crate::design::Icon::Simplify,
                            "Apply",
                            "Apply the preview as one undo step",
                            false,
                            ready.is_some_and(|(_, c)| c.after < c.before),
                        )
                        .clicked();
                        canceled = crate::design::button(
                            ui,
                            crate::design::Icon::Close,
                            "Cancel",
                            "Keep the original paths",
                            false,
                            true,
                        )
                        .clicked();
                    });
                });
        }
        preview.open = open && !canceled && !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if apply && preview.open {
            let selected = self.canvas.selected.clone();
            let (_, candidate) = preview.result.take().unwrap();
            self.checkpoint();
            self.materialize_motion(&selected);
            for ((l, m), mut mark) in candidate.paths {
                if mark.fill.is_some() && mark.stroke {
                    mark.style.linked_outline_fill = true;
                }
                self.doc.layers[l].frames[self.frame][m] = mark;
            }
            self.canvas.selected = selected;
            self.canvas.set_node_ids(Vec::new());
            self.recover();
            preview.open = false;
        }
        if preview.open
            && preview.job.is_none()
            && preview.requested != Some((preview.tolerance.to_bits(), preview.preserve))
        {
            let originals = preview.originals.clone();
            let tolerance = preview.tolerance;
            let cancel = preview.cancel.clone();
            let repaint = ctx.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let preserve = preview.preserve;
            preview.requested = Some((tolerance.to_bits(), preserve));
            preview.job = Some(rx);
            preview.error = None;
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut paths = Vec::new();
                    let (mut before, mut after) = (0, 0);
                    for (id, mark) in originals {
                        before += editable(&mark).nodes.len();
                        let result = simplified(&mark, tolerance, preserve, &cancel)?;
                        after += editable(&result).nodes.len();
                        paths.push((id, result));
                    }
                    Ok((
                        tolerance,
                        Candidate {
                            paths,
                            before,
                            after,
                            preserve,
                        },
                    ))
                }))
                .unwrap_or_else(|_| Err("Could not simplify this geometry".into()));
                let _ = tx.send(result);
                repaint.request_repaint();
            });
        }
        if !preview.open {
            preview
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if preview.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if preview.open || preview.job.is_some() {
            self.nodes.simplify = Some(preview);
        }
    }
    pub(crate) fn node_toolbar(&mut self, ctx: &egui::Context, canvas: Rect) {
        use crate::{
            canvas::Tool,
            design::{Icon, button},
        };
        if self.playing
            || !matches!(self.tool, Tool::Node | Tool::Pen)
            || !self.canvas.selected.iter().any(|&(l, m)| {
                self.doc
                    .layers
                    .get(l)
                    .and_then(|layer| layer.frames.get(self.frame))
                    .and_then(|marks| marks.get(m))
                    .is_some_and(supported)
            })
        {
            return;
        }
        let ids = self.selected_nodes();
        let mode = ids
            .first()
            .and_then(|&(l, m, n)| {
                self.doc.layers[l].frames[self.frame]
                    .get(m)
                    .map(editable)?
                    .nodes
                    .get(n)
                    .cloned()
            })
            .map(|n| n.mode.effective(&n));
        let position = egui::pos2(
            canvas.center().x,
            canvas.bottom()
                - if self.in_character_symbol() {
                    102.
                } else {
                    10.
                },
        );
        egui::Area::new(egui::Id::unique("vector-node-tools"))
            .order(egui::Order::Middle)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .fixed_pos(position)
            .constrain_to(canvas)
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Nodes");
                        for (kind, icon) in [
                            (Mode::Corner, Icon::NodeCorner),
                            (Mode::Smooth, Icon::NodeSmooth),
                            (Mode::Symmetric, Icon::NodeSymmetric),
                            (Mode::Automatic, Icon::NodeAuto),
                        ] {
                            if button(
                                ui,
                                icon,
                                "",
                                kind.label(),
                                mode == Some(kind),
                                !ids.is_empty(),
                            )
                            .clicked()
                            {
                                self.node_mode(kind);
                            }
                        }
                        ui.separator();
                        if button(
                            ui,
                            Icon::JoinNodes,
                            "",
                            "Connect two endpoints",
                            false,
                            ids.len() == 2,
                        )
                        .clicked()
                        {
                            self.join_nodes(false);
                        }
                        if button(
                            ui,
                            Icon::SplitNodes,
                            "",
                            "Break / Open path",
                            false,
                            ids.len() == 1,
                        )
                        .clicked()
                        {
                            self.split_node();
                        }
                        if button(ui, Icon::Simplify, "", "Simplify with preview", false, true)
                            .clicked()
                        {
                            self.begin_simplify();
                        }
                    });
                });
            });
    }
}
