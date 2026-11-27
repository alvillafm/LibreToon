//! Persistent bucket boundaries. Geometry is rebuilt only at edit completion,
//! never while rendering or moving the pointer. Failed/open regions keep the
//! previous geometry, and therefore cannot flood the rest of the drawing.
use crate::{document::Mark, shared::SharedVec};
use eframe::egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Boundary {
    seed: [f32; 2],
    bounds: [[f32; 2]; 2],
    gap: u8,
    sources: Vec<(u64, u64)>,
}

impl Boundary {
    pub(crate) fn valid(&self) -> bool {
        self.gap < 4
            && !self.sources.is_empty()
            && self.sources.len() <= 100_000
            && self
                .seed
                .iter()
                .chain(self.bounds.iter().flatten())
                .all(|v| v.is_finite() && v.abs() <= 1e7)
    }
    pub(crate) fn transform(&mut self, f: &mut impl FnMut(Pos2) -> Pos2) {
        let p = f(Pos2::from(self.seed));
        self.seed = [p.x, p.y];
        let r = Rect::from_min_max(Pos2::from(self.bounds[0]), Pos2::from(self.bounds[1]));
        let b = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ]
        .into_iter()
        .map(f)
        .fold(Rect::NOTHING, |r, p| r.union(Rect::from_min_max(p, p)));
        self.bounds = [[b.min.x, b.min.y], [b.max.x, b.max.y]];
    }
}

pub(crate) fn remap_ids(mark: &mut Mark, map: &mut impl FnMut(u64) -> u64) {
    mark.style.boundary_id = mark.style.boundary_id.map(&mut *map);
    if let Some(link) = mark.style.fill_boundary.as_mut() {
        for (id, _) in &mut link.sources {
            *id = map(*id);
        }
    }
}

fn eligible(mark: &Mark) -> bool {
    (mark.stroke || mark.fill.is_some_and(|c| c[3] > 0))
        && !mark.bucket
        && !mark.style.guide
        && !mark.style.raster_tile
        && mark.style.instance.is_none()
        && mark.style.media_image.is_none()
        && mark.style.text.is_none()
}

fn geometry(mark: &Mark) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    mark.points.len().hash(&mut h);
    mark.nodes.len().hash(&mut h);
    mark.holes.len().hash(&mut h);
    for hole in &mark.holes {
        hole.len().hash(&mut h);
    }
    for p in mark.points.iter().chain(mark.holes.iter().flatten()) {
        p.map(f32::to_bits).hash(&mut h);
    }
    for n in &mark.nodes {
        n.position.map(f32::to_bits).hash(&mut h);
        n.in_handle.map(f32::to_bits).hash(&mut h);
        n.out_handle.map(f32::to_bits).hash(&mut h);
    }
    mark.stroke.hash(&mut h);
    mark.fill.is_some_and(|c| c[3] > 0).hash(&mut h);
    mark.rectangle.hash(&mut h);
    mark.closed.hash(&mut h);
    mark.width.to_bits().hash(&mut h);
    for stop in &mark.width_profile {
        stop.t.to_bits().hash(&mut h);
        stop.width.to_bits().hash(&mut h);
    }
    // Caps affect the occupied ink and gap closure too.
    std::mem::discriminant(&mark.style.start_cap).hash(&mut h);
    std::mem::discriminant(&mark.style.end_cap).hash(&mut h);
    h.finish()
}

pub(crate) fn attach(
    marks: &mut SharedVec<Mark>,
    fills: std::ops::Range<usize>,
    seed: [f32; 2],
    gap: u8,
) {
    let region = fills
        .clone()
        .filter_map(|i| marks.get(i))
        .fold(Rect::NOTHING, |r, m| {
            r.union(crate::vector::control_bounds(m))
        });
    let mut sources = Vec::new();
    let mut bounds = Rect::NOTHING;
    for mark in marks.iter_mut().filter(|m| eligible(m)) {
        let b = crate::vector::control_bounds(mark);
        if !region.expand(mark.width + 36.).intersects(b) {
            continue;
        }
        let id = *mark
            .style
            .boundary_id
            .get_or_insert_with(crate::features::id);
        sources.push((id, geometry(mark)));
        bounds = bounds.union(b);
    }
    if sources.is_empty() || !bounds.is_finite() {
        return;
    }
    let link = Boundary {
        seed,
        gap,
        sources,
        bounds: [[bounds.min.x, bounds.min.y], [bounds.max.x, bounds.max.y]],
    };
    for i in fills {
        if let Some(mark) = marks.get_mut(i) {
            mark.style.fill_boundary = Some(Box::new(link.clone()));
        }
    }
}

/// Returns the number of links whose boundary is now open, missing or ambiguous.
pub(crate) fn refresh(marks: &mut SharedVec<Mark>) -> usize {
    if !marks.iter().any(|m| m.style.fill_boundary.is_some()) {
        return 0;
    }
    let mut failures = 0;
    let mut updates = Vec::new();
    let mut fingerprints = std::collections::HashMap::new();
    let sources: std::collections::HashMap<u64, Vec<&Mark>> = {
        let mut result = std::collections::HashMap::<u64, Vec<&Mark>>::new();
        for m in marks.iter().filter(|m| eligible(m)) {
            if let Some(id) = m.style.boundary_id {
                result.entry(id).or_default().push(m);
            }
        }
        result
    };
    for (index, fill) in marks.iter().enumerate() {
        let Some(link) = fill.style.fill_boundary.as_deref() else {
            continue;
        };
        if link.gap > 3
            || link.sources.is_empty()
            || !link.seed.iter().all(|v| v.is_finite())
            || !link.bounds.iter().flatten().all(|v| v.is_finite())
        {
            failures += 1;
            continue;
        }
        let mut current = Vec::new();
        let mut hashes = Vec::new();
        let mut bounds = Rect::NOTHING;
        for &(id, _) in &link.sources {
            let Some(source) = sources
                .get(&id)
                .filter(|list| list.len() == 1)
                .map(|list| list[0])
            else {
                break;
            };
            hashes.push((
                id,
                *fingerprints.entry(id).or_insert_with(|| geometry(source)),
            ));
            bounds = bounds.union(crate::vector::control_bounds(source));
            current.push(source.clone());
        }
        if hashes.len() != link.sources.len() {
            failures += 1;
            continue;
        }
        if hashes == link.sources {
            continue;
        }
        let previous = Rect::from_min_max(Pos2::from(link.bounds[0]), Pos2::from(link.bounds[1]));
        if !bounds.is_finite()
            || !previous.is_finite()
            || previous.width() <= 0.
            || previous.height() <= 0.
        {
            failures += 1;
            continue;
        }
        // Preserve the click's relative location when the whole boundary moves,
        // rotates or scales. A local edit normally keeps this interior point.
        let seed = Pos2::new(
            bounds.left() + (link.seed[0] - previous.left()) / previous.width() * bounds.width(),
            bounds.top() + (link.seed[1] - previous.top()) / previous.height() * bounds.height(),
        );
        let Some(color) = fill.fill else {
            continue;
        };
        let Ok(mut shapes) = crate::bucket::fill(
            &current,
            seed,
            crate::bucket::Gap::ALL[link.gap as usize],
            color,
        ) else {
            failures += 1;
            continue;
        };
        // Keep object identity, ordering and selection stable. A split face is
        // deliberately left unchanged instead of silently deleting artwork.
        if shapes.len() != 1 {
            failures += 1;
            continue;
        }
        let mut shape = shapes.remove(0);
        shape.style = fill.style.clone();
        let mut next = link.clone();
        next.seed = [seed.x, seed.y];
        next.bounds = [[bounds.min.x, bounds.min.y], [bounds.max.x, bounds.max.y]];
        next.sources = hashes;
        shape.style.fill_boundary = Some(Box::new(next));
        updates.push((index, shape));
    }
    for (index, shape) in updates {
        marks[index] = shape;
    }
    failures
}

impl crate::Editor {
    fn selected_fill_outline(&self) -> Option<(usize, usize, Option<usize>)> {
        if self.playing || !self.canvas.partial.is_empty() {
            return None;
        }
        let selected = &self.canvas.selected;
        if !(1..=2).contains(&selected.len()) {
            return None;
        }
        let layer = selected[0].0;
        let l = self.doc.layers.get(layer)?;
        if l.options.locked || !l.visible || selected.iter().any(|id| id.0 != layer) {
            return None;
        }
        let marks = &l.frames[self.frame];
        let outline = selected.iter().find_map(|&(_, index)| {
            marks
                .get(index)
                .filter(|m| eligible(m) && m.stroke && (m.closed || m.rectangle))
                .map(|_| index)
        })?;
        if selected.len() == 1 {
            return (marks[outline].fill.is_some() && !marks[outline].style.linked_outline_fill)
                .then_some((layer, outline, None));
        }
        let fill = selected.iter().find_map(|&(_, index)| {
            marks
                .get(index)
                .filter(|m| {
                    index != outline
                        && !m.stroke
                        && m.fill.is_some()
                        && m.style.instance.is_none()
                        && m.style.media_image.is_none()
                        && m.style.text.is_none()
                        && !m.style.raster_tile
                        && !m.style.guide
                })
                .map(|_| index)
        })?;
        Some((layer, outline, Some(fill)))
    }

    pub(crate) fn relink_selected_fill_outline(&mut self) {
        let Some((layer, outline, fill)) = self.selected_fill_outline() else {
            return;
        };
        let selected = self.canvas.selected.clone();
        self.checkpoint();
        self.materialize_motion(&selected);
        let marks = &mut self.doc.layers[layer].frames[self.frame];
        let mut combined = marks[outline].clone();
        if let Some(fill) = fill {
            combined.fill = marks[fill].fill;
            combined.style.gradient = marks[fill].style.gradient.clone();
            combined.style.palette_fill = marks[fill].style.palette_fill;
        }
        combined.style.linked_outline_fill = true;
        combined.style.fill_boundary = None;
        marks[outline] = combined;
        let index = if let Some(fill) = fill {
            marks.remove(fill);
            outline - usize::from(fill < outline)
        } else {
            outline
        };
        self.canvas.selected = vec![(layer, index)];
        self.canvas.set_node_ids(Vec::new());
        self.recover();
    }

    pub(crate) fn fill_outline_link_ui(&mut self, ui: &mut eframe::egui::Ui) {
        if self.selected_fill_outline().is_some()
            && crate::design::button(
                ui,
                crate::design::Icon::JoinNodes,
                "Link fill and outline",
                "Fit the selected fill to the outline and keep their nodes together",
                false,
                true,
            )
            .clicked()
        {
            self.relink_selected_fill_outline();
        }
    }
    pub(crate) fn sync_linked_fills(&mut self) {
        let mut failures = 0;
        for l in 0..self.doc.layers.len() {
            let layer = &self.doc.layers[l];
            let Some(marks) = layer.frames.get(self.frame) else {
                continue;
            };
            if self.exposure_sync.get(l) == Some(&(self.frame, marks.identity())) {
                continue;
            }
            if !marks.iter().any(|m| m.style.fill_boundary.is_some()) {
                continue;
            }
            failures += refresh(&mut self.doc.layers[l].frames[self.frame]);
        }
        if failures > 0 {
            self.status =
                crate::i18n::tr("Linked fill kept: close or restore its outline to update it.")
                    .into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drawing() -> SharedVec<Mark> {
        let corners = [[0., 0.], [100., 0.], [100., 100.], [0., 100.]];
        let mut marks: SharedVec<Mark> = (0..4)
            .map(|i| Mark {
                points: vec![corners[i], corners[(i + 1) % 4]].into(),
                width: 2.,
                ..Default::default()
            })
            .collect();
        let fill = crate::bucket::fill(
            &marks,
            Pos2::new(50., 50.),
            crate::bucket::Gap::None,
            [255, 0, 0, 255],
        )
        .unwrap();
        assert_eq!(fill.len(), 1);
        marks.splice(0..0, fill);
        attach(&mut marks, 0..1, [50., 50.], 0);
        marks
    }
    fn widen(marks: &mut SharedVec<Mark>) {
        for m in marks.iter_mut().skip(1) {
            for p in &mut m.points {
                if p[0] == 100. {
                    p[0] = 150.;
                }
            }
        }
    }
    #[test]
    fn linked_bucket_follows_multiple_outlines_and_round_trips() {
        let mut marks = drawing();
        let bytes = serde_json::to_vec(&marks).unwrap();
        marks = serde_json::from_slice(&bytes).unwrap();
        let before = marks.clone();
        widen(&mut marks);
        assert_eq!(refresh(&mut marks), 0);
        assert!(crate::vector::contains_fill(
            &marks[0],
            Pos2::new(140., 50.)
        ));
        assert!(!crate::vector::contains_fill(
            &before[0],
            Pos2::new(140., 50.)
        ));
        let identity = marks.identity();
        assert_eq!(refresh(&mut marks), 0);
        assert_eq!(
            identity,
            marks.identity(),
            "unchanged links must not copy frame data"
        );
        assert_eq!(marks[0].fill, before[0].fill);
    }
    #[test]
    fn open_or_deleted_boundary_keeps_last_valid_fill_and_can_recover() {
        let mut marks = drawing();
        let original = marks[0].clone();
        let wall = marks.remove(2);
        assert_eq!(refresh(&mut marks), 1);
        assert!(marks[0] == original);
        marks.insert(2, wall);
        widen(&mut marks);
        assert_eq!(refresh(&mut marks), 0);
        assert!(crate::vector::contains_fill(
            &marks[0],
            Pos2::new(140., 50.)
        ));
        marks[2].points[0] = [500., 500.];
        let good = marks[0].clone();
        assert_eq!(refresh(&mut marks), 1);
        assert!(marks[0] == good);
    }
    #[test]
    fn group_transform_preserves_link_and_dependent_fill_style() {
        let mut marks = drawing();
        let gradient = crate::features::Gradient::default();
        marks[0].style.gradient = Some(gradient);
        marks[0].style.palette_fill = Some(42);
        for mark in &mut marks {
            *mark = crate::vector::transform(mark, |p| Pos2::new(400. - p.y, 100. + p.x));
        }
        assert_eq!(refresh(&mut marks), 0);
        assert!(crate::vector::contains_fill(
            &marks[0],
            Pos2::new(350., 150.)
        ));
        assert!(marks[0].style.gradient.is_some());
        assert_eq!(marks[0].style.palette_fill, Some(42));
    }
    #[test]
    fn bezier_outline_changes_rebuild_bucket_geometry() {
        let mut marks = drawing();
        let mut a = crate::document::Node::corner([0., 0.]);
        a.out_handle = [25., -70.];
        let mut b = crate::document::Node::corner([100., 0.]);
        b.in_handle = [75., -70.];
        marks[1].nodes = vec![a, b].into();
        assert_eq!(refresh(&mut marks), 0);
        assert!(crate::vector::contains_fill(
            &marks[0],
            Pos2::new(50., -30.)
        ));
    }
}

#[cfg(test)]
#[test]
fn copied_boundary_ids_do_not_collide_with_originals() {
    let mut drawing: SharedVec<Mark> = (0..4)
        .map(|i| {
            let p = [[0., 0.], [100., 0.], [100., 100.], [0., 100.]];
            Mark {
                points: vec![p[i], p[(i + 1) % 4]].into(),
                ..Default::default()
            }
        })
        .collect();
    let fills = crate::bucket::fill(
        &drawing,
        Pos2::new(50., 50.),
        crate::bucket::Gap::None,
        [255; 4],
    )
    .unwrap();
    drawing.splice(0..0, fills);
    attach(&mut drawing, 0..1, [50., 50.], 0);
    let mut copy = drawing.clone();
    let mut ids = std::collections::HashMap::new();
    for m in &mut copy {
        remap_ids(m, &mut |id| {
            *ids.entry(id).or_insert_with(crate::features::id)
        });
        *m = crate::vector::transform(m, |p| p + eframe::egui::Vec2::new(300., 0.));
    }
    drawing.extend(copy.iter().cloned());
    assert_eq!(refresh(&mut drawing), 0);
    assert!(crate::vector::contains_fill(
        &drawing[5],
        Pos2::new(350., 50.)
    ));
    assert!(crate::vector::contains_fill(
        &drawing[0],
        Pos2::new(50., 50.)
    ));
}

#[cfg(test)]
#[test]
fn filled_brush_silhouettes_can_bound_a_linked_region() {
    let mut marks: SharedVec<Mark> = [
        [[0., 0.], [100., 4.]],
        [[96., 0.], [100., 100.]],
        [[0., 96.], [100., 100.]],
        [[0., 0.], [4., 100.]],
    ]
    .into_iter()
    .map(|points| Mark {
        points: points.to_vec().into(),
        rectangle: true,
        closed: true,
        stroke: false,
        fill: Some([0, 0, 0, 255]),
        ..Default::default()
    })
    .collect();
    let fill = crate::bucket::fill(
        &marks,
        Pos2::new(50., 50.),
        crate::bucket::Gap::None,
        [255; 4],
    )
    .unwrap();
    marks.splice(0..0, fill);
    attach(&mut marks, 0..1, [50., 50.], 0);
    assert!(marks[0].style.fill_boundary.is_some());
    for m in marks.iter_mut().skip(1) {
        for p in &mut m.points {
            if p[0] >= 96. {
                p[0] += 50.;
            }
        }
    }
    assert_eq!(refresh(&mut marks), 0);
    assert!(crate::vector::contains_fill(
        &marks[0],
        Pos2::new(140., 50.)
    ));
}
