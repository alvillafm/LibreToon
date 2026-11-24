//! Bounded per-thread picking geometry. Owning the COW source prevents stale pointer keys.
use crate::Mark;
use eframe::egui::Pos2;
use std::{cell::RefCell, collections::VecDeque, sync::Arc};
pub(crate) struct Geometry {
    source_points: crate::shared::SharedVec<[f32; 2]>,
    source_nodes: crate::shared::SharedVec<crate::document::Node>,
    source_holes: crate::shared::SharedVec<crate::shared::SharedVec<[f32; 2]>>,
    closed: bool,
    rectangle: bool,
    text: bool,
    pub points: Vec<Pos2>,
    pub lengths: Vec<f32>,
    pub total: f32,
    bytes: usize,
}
thread_local! { static CACHE: RefCell<VecDeque<Arc<Geometry>>> = const {RefCell::new(VecDeque::new())}; }
pub(crate) fn geometry(mark: &Mark) -> Arc<Geometry> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(i) = cache.iter().position(|g| {
            g.source_points.identity() == mark.points.identity()
                && g.source_nodes.identity() == mark.nodes.identity()
                && g.source_holes.identity() == mark.holes.identity()
                && g.closed == mark.closed
                && g.rectangle == mark.rectangle
                && g.text == mark.style.text.is_some()
        }) {
            let g = cache.remove(i).unwrap();
            cache.push_back(g.clone());
            return g;
        }
        let points = crate::canvas::vertices(mark);
        let (lengths, total) = crate::width::lengths(&points);
        let bytes = points
            .len()
            .saturating_mul(12)
            .saturating_add(mark.points.len().saturating_mul(8))
            .saturating_add(
                mark.nodes
                    .len()
                    .saturating_mul(std::mem::size_of::<crate::document::Node>()),
            )
            .saturating_add(
                mark.holes
                    .iter()
                    .map(|h| h.len().saturating_mul(8))
                    .sum::<usize>(),
            );
        let geometry = Arc::new(Geometry {
            source_points: mark.points.clone(),
            source_nodes: mark.nodes.clone(),
            source_holes: mark.holes.clone(),
            closed: mark.closed,
            rectangle: mark.rectangle,
            text: mark.style.text.is_some(),
            points,
            lengths,
            total,
            bytes,
        });
        const LIMIT: usize = 4 * 1024 * 1024;
        if bytes <= LIMIT {
            let mut retained = cache.iter().map(|g| g.bytes).sum::<usize>();
            while retained.saturating_add(bytes) > LIMIT || cache.len() >= 128 {
                retained = retained.saturating_sub(cache.pop_front().unwrap().bytes);
            }
            cache.push_back(geometry.clone());
        }
        geometry
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picking_reuses_geometry_but_never_returns_a_changed_path() {
        let mut mark = Mark {
            points: vec![[0., 0.], [20., 30.]].into(),
            ..Default::default()
        };
        let first = geometry(&mark);
        assert!(Arc::ptr_eq(&first, &geometry(&mark)));
        mark.points[1] = [100., 200.];
        let second = geometry(&mark);
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(first.points.last().unwrap().x, 20.);
        assert_eq!(second.points.last().unwrap().x, 100.);
        for i in 0..500 {
            mark.points[1][0] = i as f32;
            geometry(&mark);
        }
        CACHE.with(|cache| {
            let c = cache.borrow();
            assert!(c.len() <= 128 && c.iter().map(|g| g.bytes).sum::<usize>() <= 4 * 1024 * 1024);
        });
    }
}
