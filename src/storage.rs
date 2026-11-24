//! Explicit, off-thread resource accounting. Never scan the document while painting.
use crate::{Document, Editor, i18n::tr};
use eframe::egui;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::mpsc,
};

#[derive(Default)]
pub(crate) struct State {
    open: bool,
    job: Option<mpsc::Receiver<Outcome>>,
    report: Option<Report>,
}
struct Outcome {
    report: Report,
    cleaned: Option<(Document, usize, Token)>,
}
#[derive(PartialEq)]
struct Token(std::path::PathBuf, Option<usize>, u64, usize);
#[derive(Default)]
struct Report {
    bytes: u64,
    metadata_bytes: Option<u64>,
    unused: Vec<u64>,
    duplicates: Vec<(u64, u64)>,
    rows: Vec<(String, usize, usize)>,
}
fn counts(doc: &Document) -> BTreeMap<u64, usize> {
    let mut result = BTreeMap::new();
    let mut cels = BTreeSet::new();
    for cel in doc
        .layers
        .iter()
        .chain(doc.extras.scenes.iter().flat_map(|s| s.layers.iter()))
        .chain(doc.extras.symbols.iter().flat_map(|s| s.layers.iter()))
        .flat_map(|l| l.frames.iter())
    {
        if !cels.insert(cel.identity()) {
            continue;
        }
        for id in cel.iter().filter_map(|m| m.style.media_image) {
            *result.entry(id).or_default() += 1;
        }
    }
    for id in doc
        .extras
        .media
        .tracks
        .iter()
        .chain(doc.extras.scenes.iter().flat_map(|s| s.media_tracks.iter()))
        .flat_map(|track| track.asset_ids())
    {
        *result.entry(id).or_default() += 1;
    }
    result
}
fn inspect(doc: &Document) -> Report {
    inspect_with(doc, true)
}
fn inspect_with(doc: &Document, metadata_size: bool) -> Report {
    let counts = counts(doc);
    let mut report = Report::default();
    struct Counter(u64);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| std::io::Error::other("Size overflow"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    let metadata = crate::native_project::without_media_payloads(doc);
    if metadata_size
        && crate::native_project::cels::compact(|| serde_json::to_writer(&mut counter, &metadata))
            .is_ok()
    {
        report.metadata_bytes = Some(counter.0);
    }

    let mut groups: BTreeMap<(usize, u32), Vec<&crate::media::Asset>> = BTreeMap::new();
    for asset in &doc.extras.media.assets {
        let count = counts.get(&asset.id).copied().unwrap_or(0);
        report.bytes += asset.data.len() as u64;
        report
            .rows
            .push((asset.name.clone(), asset.data.len(), count));
        if count == 0 {
            report.unused.push(asset.id);
        }
        let peers = groups
            .entry((asset.data.len(), crc32fast::hash(&asset.data)))
            .or_default();
        if let Some(previous) = peers.iter().find(|other| asset.same_payload(other)) {
            report.duplicates.push((asset.id, previous.id));
        } else {
            peers.push(asset);
        }
    }
    report.bytes += doc
        .extras
        .references
        .iter()
        .map(|r| r.data.len() as u64)
        .sum::<u64>();
    report
}

fn clean(doc: &mut Document, unused: bool, duplicates: bool) -> usize {
    let report = inspect_with(doc, false);
    let remap: BTreeMap<_, _> = if duplicates {
        report.duplicates.into_iter().collect()
    } else {
        BTreeMap::new()
    };
    let mut replacements = BTreeMap::new();
    let map_layers =
        |layers: &mut crate::shared::SharedVec<crate::document::Layer>,
         replacements: &mut BTreeMap<usize, crate::shared::SharedVec<crate::Mark>>| {
            // Only clone containers that actually refer to a removed duplicate.
            for l in 0..layers.len() {
                for f in 0..layers[l].frames.len() {
                    let cel = &layers[l].frames[f];
                    if !cel.iter().any(|m| {
                        m.style
                            .media_image
                            .is_some_and(|id| remap.contains_key(&id))
                    }) {
                        continue;
                    }
                    let id = cel.identity();
                    let replacement = replacements
                        .entry(id)
                        .or_insert_with(|| {
                            let mut next = cel.clone();
                            for mark in &mut next {
                                if let Some(new) =
                                    mark.style.media_image.and_then(|id| remap.get(&id))
                                {
                                    mark.style.media_image = Some(*new);
                                }
                            }
                            next
                        })
                        .clone();
                    layers[l].frames[f] = replacement;
                }
            }
        };
    if !remap.is_empty() {
        map_layers(&mut doc.layers, &mut replacements);
        for scene in &mut doc.extras.scenes {
            map_layers(&mut scene.layers, &mut replacements);
        }
        for symbol in &mut doc.extras.symbols {
            map_layers(&mut symbol.layers, &mut replacements);
        }
        for track in doc.extras.media.tracks.iter_mut().chain(
            doc.extras
                .scenes
                .iter_mut()
                .flat_map(|s| s.media_tracks.iter_mut()),
        ) {
            for (&old, &new) in &remap {
                track.replace_asset_reference(old, new);
            }
        }
    }
    let counts = counts(doc);
    let before = doc.extras.media.assets.len();
    doc.extras.media.assets.retain(|a| {
        !remap.contains_key(&a.id) && (!unused || counts.get(&a.id).copied().unwrap_or(0) > 0)
    });
    before - doc.extras.media.assets.len()
}

impl Editor {
    fn storage_token(&self) -> Token {
        Token(
            self.recovery.clone(),
            self.hub.active,
            self.hub
                .active
                .map_or(0, |i| self.hub.tabs[i].record.revision),
            self.doc.layers.identity(),
        )
    }
    pub(crate) fn open_storage(&mut self) {
        if self.storage.job.is_some() {
            self.storage.open = true;
            return;
        }
        let snapshot = self.project_document();
        let (tx, rx) = mpsc::sync_channel(1);
        self.storage.open = true;
        self.storage.report = None;
        self.storage.job = Some(rx);
        crate::geometry_jobs::spawn(move || {
            let _ = tx.send(Outcome {
                report: inspect(&snapshot),
                cleaned: None,
            });
        });
    }
    pub(crate) fn storage_ui(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.storage.job {
            match job.try_recv() {
                Ok(outcome) => {
                    self.storage.job = None;
                    if let Some((doc, removed, token)) = outcome.cleaned {
                        if token == self.storage_token() && self.feature.symbol_stack.is_empty() {
                            self.checkpoint();
                            self.doc = doc;
                            self.status = format!("{}: {removed}", tr("Removed resources"));
                            self.recover();
                            self.storage.report = Some(outcome.report);
                        } else {
                            self.storage.report = None;
                            self.status =
                                tr("Project changed during analysis. Refresh before cleaning.")
                                    .into();
                            self.open_storage();
                        }
                    } else {
                        self.storage.report = Some(outcome.report);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.storage.job = None;
                    self.status = tr("Storage analysis failed").into();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(80));
                }
            }
        }
        if !self.storage.open {
            return;
        }
        let mut open = true;
        let mut action = None;
        egui::Window::new(tr("Project storage")).open(&mut open).default_width(520.).show(ctx, |ui| {
            if let Some(report) = &self.storage.report {
                ui.label(format!("{}: {:.2} MiB", tr("Embedded resources"), report.bytes as f64 / 1048576.));
                if let Some(bytes)=report.metadata_bytes { ui.label(format!("{}: {:.2} MiB",tr("Compact geometry and metadata before compression"),bytes as f64/1048576.)); }
                ui.label(tr("Sizes describe embedded files before ZIP compression. Vector geometry is stored separately."));
                ui.horizontal_wrapped(|ui| {
                    let enabled = self.feature.symbol_stack.is_empty() && !self.project_save.busy();
                    if ui.add_enabled(enabled && !report.unused.is_empty(), egui::Button::new(format!("{} ({})", tr("Remove unused"), report.unused.len()))).on_hover_text(tr("Rechecks all scenes and symbols; undo restores removed resources. Finish symbol editing first.")).clicked() { action = Some((true, false)); }
                    if ui.add_enabled(enabled && !report.duplicates.is_empty(), egui::Button::new(format!("{} ({})", tr("Merge exact duplicates"), report.duplicates.len()))).on_hover_text(tr("Only byte-identical resources with matching media settings are merged. All references are updated.")).clicked() { action = Some((false, true)); }
                });
                egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                    egui::Grid::new("storage-resources").striped(true).show(ui, |ui| {
                        ui.strong(tr("Resource")); ui.strong(tr("Size")); ui.strong(tr("References")); ui.end_row();
                        for (name, bytes, uses) in &report.rows { ui.label(name); ui.label(format!("{:.1} KiB", *bytes as f64 / 1024.)); ui.label(uses.to_string()); ui.end_row(); }
                    });
                });
                if ui.button(tr("Refresh")).clicked() { action = Some((false, false)); }
            } else { ui.spinner(); ui.label(tr("Analysing project resources…")); }
        });
        self.storage.open = open;
        if let Some((unused, duplicates)) = action {
            if unused || duplicates {
                // One bounded job; editing can continue, but an outdated result cannot replace it.
                let token = self.storage_token();
                let mut snapshot = self.project_document();
                let (tx, rx) = mpsc::sync_channel(1);
                self.storage.job = Some(rx);
                self.storage.report = None;
                crate::geometry_jobs::spawn(move || {
                    let removed = clean(&mut snapshot, unused, duplicates);
                    let report = inspect(&snapshot);
                    let _ = tx.send(Outcome {
                        report,
                        cleaned: Some((snapshot, removed, token)),
                    });
                });
            } else {
                self.open_storage();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_cleanup_keeps_symbol_scene_and_held_cel_references() {
        let mut doc = crate::fresh_document();
        // Use a valid image fixture through the media importer representation.
        let value = serde_json::json!({"id": 9001, "name": "A", "kind":"Image", "data":[1,2,3], "size":[1,1], "seconds":0.0});
        let a: crate::media::Asset = serde_json::from_value(value.clone()).unwrap();
        let mut b = a.clone();
        b.id = 9002;
        doc.extras.media.assets = vec![a, b].into();
        let mut mark = crate::Mark::default();
        mark.style.media_image = Some(9002);
        doc.layers[0].frames[0] = vec![mark].into();
        doc.layers[0].frames[1] = doc.layers[0].frames[0].clone();
        assert_eq!(inspect(&doc).duplicates, vec![(9002, 9001)]);
        assert_eq!(clean(&mut doc, false, true), 1);
        assert_eq!(doc.layers[0].frames[0][0].style.media_image, Some(9001));
        assert_eq!(
            doc.layers[0].frames[0].identity(),
            doc.layers[0].frames[1].identity()
        );
        assert_eq!(clean(&mut doc, true, false), 0);
    }
}
