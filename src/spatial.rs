//! Bounded uniform grid shared by geometric selection and gap-neighbor queries.
use eframe::egui::{Pos2, Rect};
use std::collections::{BTreeSet, HashMap};
pub(crate) struct Index {
    rects: Vec<Rect>,
    bins: HashMap<(i32, i32), Vec<usize>>,
    large: Vec<usize>,
    step: f32,
}
impl Index {
    pub(crate) fn new(rects: impl IntoIterator<Item = Rect>) -> Self {
        let rects: Vec<_> = rects.into_iter().collect();
        let bounds = rects
            .iter()
            .filter(|r| r.is_finite())
            .fold(Rect::NOTHING, |a, b| a.union(*b));
        let step = ((bounds.width() * bounds.height()).max(1.) / rects.len().max(1) as f32)
            .sqrt()
            .clamp(16., 512.);
        let mut result = Self {
            rects,
            bins: HashMap::new(),
            large: vec![],
            step,
        };
        for (i, r) in result.rects.iter().enumerate() {
            if !r.is_finite() {
                continue;
            }
            let a = result.cell(r.min);
            let b = result.cell(r.max);
            let count = (b.0 as i128 - a.0 as i128 + 1) * (b.1 as i128 - a.1 as i128 + 1);
            if !(1..=64).contains(&count) {
                result.large.push(i);
                continue;
            }
            for y in a.1..=b.1 {
                for x in a.0..=b.0 {
                    result.bins.entry((x, y)).or_default().push(i);
                }
            }
        }
        result
    }
    fn cell(&self, p: Pos2) -> (i32, i32) {
        (
            (p.x / self.step).floor() as i32,
            (p.y / self.step).floor() as i32,
        )
    }
    pub(crate) fn query(&self, rect: Rect) -> Vec<usize> {
        if !rect.is_finite() {
            return vec![];
        }
        let a = self.cell(rect.min);
        let b = self.cell(rect.max);
        let count = (b.0 as i128 - a.0 as i128 + 1) * (b.1 as i128 - a.1 as i128 + 1);
        if !(1..=4096).contains(&count) {
            return self
                .rects
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.intersects(rect).then_some(i))
                .collect();
        }
        let mut found: BTreeSet<_> = self.large.iter().copied().collect();
        for y in a.1..=b.1 {
            for x in a.0..=b.0 {
                if let Some(items) = self.bins.get(&(x, y)) {
                    found.extend(items);
                }
            }
        }
        found
            .into_iter()
            .filter(|&i| self.rects[i].intersects(rect))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queries_match_brute_force_and_keep_original_order() {
        let rects: Vec<_> = (0..10000)
            .map(|i| {
                Rect::from_min_size(
                    Pos2::new((i % 100) as f32 * 30., (i / 100) as f32 * 30.),
                    eframe::egui::vec2(12., 18.),
                )
            })
            .collect();
        let index = Index::new(rects.clone());
        for i in 0..100 {
            let query = Rect::from_center_size(
                Pos2::new(i as f32 * 25., i as f32 * 14.),
                eframe::egui::vec2(80., 80.),
            );
            let expected: Vec<_> = rects
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.intersects(query).then_some(i))
                .collect();
            assert_eq!(index.query(query), expected);
        }
        assert!(index.bins.values().map(Vec::len).sum::<usize>() <= 640000);
    }
}
