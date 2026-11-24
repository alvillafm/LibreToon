//! COW-aware history accounting on a single bounded background task.
use crate::{Document, Editor, shared::SharedVec};
use std::{
    any::TypeId,
    collections::HashSet,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Seen {
    allocations: HashSet<(TypeId, usize)>,
    bytes: usize,
}
impl Seen {
    fn vector<T: 'static>(&mut self, values: &SharedVec<T>) -> bool {
        if !self
            .allocations
            .insert((TypeId::of::<T>(), values.identity()))
        {
            return false;
        }
        self.bytes = self
            .bytes
            .saturating_add(std::mem::size_of::<T>().saturating_mul(values.capacity()))
            .saturating_add(32);
        true
    }
    fn document(&mut self, doc: &Document) {
        self.bytes = self.bytes.saturating_add(std::mem::size_of::<Document>());
        let mut layers = |layers: &SharedVec<crate::document::Layer>| {
            if !self.vector(layers) {
                return;
            }
            for layer in layers {
                if !self.vector(&layer.frames) {
                    continue;
                }
                for cel in &layer.frames {
                    if !self.vector(cel) {
                        continue;
                    }
                    for mark in cel {
                        self.vector(&mark.points);
                        self.vector(&mark.nodes);
                        self.vector(&mark.width_profile);
                        if self.vector(&mark.holes) {
                            for hole in &mark.holes {
                                self.vector(hole);
                            }
                        }
                        for gradient in mark
                            .style
                            .gradient
                            .iter()
                            .chain(mark.style.stroke_gradient.iter())
                        {
                            self.vector(&gradient.stops);
                        }
                        self.vector(&mark.style.mouth_keys);
                        self.vector(&mark.style.mouth_routes);
                    }
                }
            }
        };
        layers(&doc.layers);
        for scene in &doc.extras.scenes {
            layers(&scene.layers);
        }
        for symbol in &doc.extras.symbols {
            layers(&symbol.layers);
        }
        if self.vector(&doc.extras.media.assets) {
            for asset in &doc.extras.media.assets {
                self.vector(&asset.data);
            }
        }
        if self.vector(&doc.extras.references) {
            for reference in &doc.extras.references {
                self.vector(&reference.data);
            }
        }
    }
}
fn retained(
    current: &Document,
    undo: &[Document],
    redo: &[Document],
    budget: usize,
) -> (usize, usize, usize) {
    let mut seen = Seen::default();
    seen.document(current);
    let base = seen.bytes;
    let mut keep_undo = 0;
    let mut undo_bytes = 0;
    for doc in undo.iter().rev() {
        seen.document(doc);
        if seen.bytes.saturating_sub(base) > budget && keep_undo > 0 {
            break;
        }
        keep_undo += 1;
        undo_bytes = seen.bytes.saturating_sub(base);
    }
    let mut keep_redo = 0;
    for doc in redo.iter().rev() {
        seen.document(doc);
        if seen.bytes.saturating_sub(base) > budget {
            break;
        }
        keep_redo += 1;
    }
    (keep_undo, keep_redo, undo_bytes)
}
#[derive(Clone, PartialEq)]
struct Key {
    recovery: std::path::PathBuf,
    layers: usize,
    assets: usize,
    symbols: usize,
    undo: usize,
    redo: usize,
    last_undo: usize,
    last_redo: usize,
    budget: usize,
}
struct Result {
    key: Key,
    undo: usize,
    redo: usize,
    bytes: usize,
}
#[derive(Default)]
pub(crate) struct State {
    key: Option<Key>,
    changed: Option<Instant>,
    job: Option<mpsc::Receiver<Result>>,
    pub(crate) bytes: usize,
}
impl Editor {
    pub(crate) fn history_memory_tick(&mut self, ctx: &eframe::egui::Context) {
        let key = Key {
            recovery: self.recovery.clone(),
            layers: self.doc.layers.identity(),
            assets: self.doc.extras.media.assets.identity(),
            symbols: self.doc.extras.symbols.identity(),
            undo: self.undo.len(),
            redo: self.redo.len(),
            last_undo: self.undo.last().map_or(0, |d| d.layers.identity()),
            last_redo: self.redo.last().map_or(0, |d| d.layers.identity()),
            budget: self.studio.prefs.history_memory_mib.clamp(32, 4096) * 1048576,
        };
        if self.memory.key.as_ref() != Some(&key) {
            self.memory.key = Some(key.clone());
            self.memory.changed = Some(Instant::now());
        }
        if let Some(job) = &self.memory.job {
            match job.try_recv() {
                Ok(result) => {
                    self.memory.job = None;
                    if result.key == key {
                        self.undo
                            .drain(..self.undo.len().saturating_sub(result.undo));
                        self.redo
                            .drain(..self.redo.len().saturating_sub(result.redo));
                        self.memory.bytes = result.bytes;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.memory.job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        if self.memory.job.is_none()
            && self
                .memory
                .changed
                .is_some_and(|t| t.elapsed() >= Duration::from_millis(250))
        {
            self.memory.changed = None;
            let current = self.doc.clone();
            let undo = self.undo.clone();
            let redo = self.redo.clone();
            let (tx, rx) = mpsc::sync_channel(1);
            self.memory.job = Some(rx);
            std::thread::spawn(move || {
                let (undo, redo, bytes) = retained(&current, &undo, &redo, key.budget);
                let _ = tx.send(Result {
                    key,
                    undo,
                    redo,
                    bytes,
                });
            });
        } else if self.memory.changed.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_budget_counts_shared_geometry_once_and_retains_recent_edit() {
        let mut doc = crate::fresh_document();
        doc.layers[0].frames[0] = vec![crate::Mark {
            points: vec![[1., 2.]; 100_000].into(),
            ..Default::default()
        }]
        .into();
        let shared = vec![doc.clone(); 40];
        assert_eq!(retained(&doc, &shared, &[], 200_000).0, 40);
        let mut undo = vec![];
        for i in 0..8 {
            undo.push(doc.clone());
            doc.layers[0].frames[0][0].points[0][0] = i as f32;
        }
        let (keep, _, _) = retained(&doc, &undo, &[], 1_000_000);
        assert_eq!(keep, 1);
        assert_eq!(undo.len(), 8, "measurement never edits snapshots");
    }
}
