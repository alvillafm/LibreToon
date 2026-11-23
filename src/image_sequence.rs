//! Ordered image-sequence import with bounded decoding and one atomic undo step.
use crate::{Editor, Mark, document::Layer, media::Asset, shared::SharedVec};
use eframe::egui;
use std::{
    cmp::Ordering,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering as MemoryOrder},
        mpsc,
    },
};

pub(crate) struct Dialog {
    files: Vec<PathBuf>,
    name: String,
    owner: PathBuf,
    start: usize,
    exposure: usize,
    fps: u32,
    set_fps: bool,
    fit: bool,
    job: Option<mpsc::Receiver<Result<Vec<Asset>, String>>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    error: Option<String>,
}
impl Drop for Dialog {
    fn drop(&mut self) {
        self.cancel.store(true, MemoryOrder::Relaxed);
    }
}
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        if a[x].is_ascii_digit() && b[y].is_ascii_digit() {
            let (start_x, start_y) = (x, y);
            while x < a.len() && a[x].is_ascii_digit() {
                x += 1;
            }
            while y < b.len() && b[y].is_ascii_digit() {
                y += 1;
            }
            let (mut ax, mut by) = (start_x, start_y);
            while ax + 1 < x && a[ax] == b'0' {
                ax += 1;
            }
            while by + 1 < y && b[by] == b'0' {
                by += 1;
            }
            let order = (x - ax)
                .cmp(&(y - by))
                .then_with(|| a[ax..x].cmp(&b[by..y]));
            if order != Ordering::Equal {
                return order;
            }
        } else {
            let order = a[x].to_ascii_lowercase().cmp(&b[y].to_ascii_lowercase());
            if order != Ordering::Equal {
                return order;
            }
            x += 1;
            y += 1;
        }
    }
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
fn read(
    files: Vec<PathBuf>,
    ffmpeg: PathBuf,
    cancel: &AtomicBool,
    progress: &AtomicUsize,
) -> Result<Vec<Asset>, String> {
    let mut assets = vec![];
    let mut bytes = 0usize;
    for (index, file) in files.into_iter().enumerate() {
        if cancel.load(MemoryOrder::Relaxed) {
            return Err("Image sequence import canceled".into());
        }
        let asset =
            Editor::read_media(&file, &ffmpeg).map_err(|e| format!("{}: {e}", file.display()))?;
        bytes = bytes.saturating_add(asset.data.len());
        if bytes > 256 * 1024 * 1024 {
            return Err("Image sequence exceeds the 256 MiB media budget".into());
        }
        assets.push(asset);
        progress.store(index + 1, MemoryOrder::Relaxed);
    }
    Ok(assets)
}
impl Editor {
    pub(crate) fn import_image_sequence(&mut self) {
        if self.image_sequence.is_some() {
            return;
        }
        let Some(mut files) = rfd::FileDialog::new()
            .add_filter(
                "Image sequence",
                &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp"],
            )
            .pick_files()
        else {
            return;
        };
        if files.is_empty() || files.len() > 10000 {
            self.status = "Choose between 1 and 10000 images".into();
            return;
        }
        files.sort_by(|a, b| {
            natural_cmp(
                &a.file_name().unwrap_or_default().to_string_lossy(),
                &b.file_name().unwrap_or_default().to_string_lossy(),
            )
        });
        self.image_sequence = Some(Dialog {
            files,
            name: "Image sequence".into(),
            owner: self.recovery.clone(),
            start: self.frame + 1,
            exposure: 1,
            fps: self.doc.fps,
            set_fps: false,
            fit: false,
            job: None,
            cancel: Arc::default(),
            progress: Arc::default(),
            error: None,
        });
    }
    pub(crate) fn image_sequence_ui(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.image_sequence.take() else {
            return;
        };
        if let Some(job) = &dialog.job {
            let completed = match job.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Image import interrupted".into()))
                }
                _ => None,
            };
            if let Some(result) = completed {
                dialog.job = None;
                match result {
                    Ok(assets) if dialog.owner==self.recovery&&!self.hub.home=>{
                        match self.insert_image_sequence(assets,&dialog.name,dialog.start-1,dialog.exposure,dialog.set_fps.then_some(dialog.fps),dialog.fit){
                            Ok(())=>return,Err(error)=>dialog.error=Some(error)
                        }
                    },
                    Ok(_)=>dialog.error=Some("The active document changed. Return to the original document and import again.".into()),
                    Err(error)=>dialog.error=Some(error),
                }
            }
        }
        let mut open = true;
        let mut begin = false;
        let mut canceled = false;
        egui::Window::new("Import Image Sequence")
            .open(&mut open)
            .collapsible(false)
            .default_width(460.)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} images · numeric filename order",
                    dialog.files.len()
                ));
                if let (Some(first), Some(last)) = (dialog.files.first(), dialog.files.last()) {
                    ui.label(format!(
                        "{} → {}",
                        first.file_name().unwrap_or_default().to_string_lossy(),
                        last.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                if dialog.job.is_some() {
                    ui.add(
                        egui::ProgressBar::new(
                            dialog.progress.load(MemoryOrder::Relaxed) as f32
                                / dialog.files.len() as f32,
                        )
                        .show_percentage()
                        .text("Decoding images…"),
                    );
                    if crate::design::button(
                        ui,
                        crate::design::Icon::Close,
                        "Cancel import",
                        "",
                        false,
                        true,
                    )
                    .clicked()
                    {
                        canceled = true;
                    }
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer name");
                        ui.add(egui::TextEdit::singleline(&mut dialog.name).char_limit(256));
                    });
                    ui.horizontal(|ui| {
                        ui.label("First frame");
                        ui.add(egui::DragValue::new(&mut dialog.start).range(1..=10000));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Frames per image");
                        ui.add(egui::DragValue::new(&mut dialog.exposure).range(1..=240));
                    });
                    ui.checkbox(&mut dialog.fit, "Fit images to stage (keep aspect ratio)");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut dialog.set_fps, "Set project FPS");
                        ui.add_enabled(
                            dialog.set_fps,
                            egui::DragValue::new(&mut dialog.fps).range(1..=240),
                        );
                    });
                    let end = (dialog.start - 1)
                        .saturating_add(dialog.files.len().saturating_mul(dialog.exposure));
                    ui.label(format!("New layer · frames {}–{}", dialog.start, end));
                    if end > 10000 {
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            "The sequence would exceed 10000 frames.",
                        );
                    }
                    if let Some(error) = &dialog.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    begin = crate::design::button(
                        ui,
                        crate::design::Icon::Film,
                        "Import sequence",
                        "",
                        false,
                        end <= 10000 && dialog.owner == self.recovery && !self.hub.home,
                    )
                    .clicked();
                }
            });
        if begin {
            let files = dialog.files.clone();
            let cancel = dialog.cancel.clone();
            let progress = dialog.progress.clone();
            let repaint = ctx.clone();
            let ffmpeg = crate::exporter::ffmpeg_path(&self.studio.prefs.ffmpeg);
            let (tx, rx) = mpsc::channel();
            dialog.job = Some(rx);
            dialog.error = None;
            std::thread::spawn(move || {
                let _ = tx.send(read(files, ffmpeg, &cancel, &progress));
                repaint.request_repaint();
            });
        }
        if open && !canceled {
            self.image_sequence = Some(dialog);
        }
    }
    pub(crate) fn insert_image_sequence(
        &mut self,
        assets: Vec<Asset>,
        name: &str,
        start: usize,
        exposure: usize,
        fps: Option<u32>,
        fit: bool,
    ) -> Result<(), String> {
        let end = start
            .checked_add(
                assets
                    .len()
                    .checked_mul(exposure)
                    .ok_or("Sequence is too long")?,
            )
            .ok_or("Sequence is too long")?;
        if assets.is_empty()
            || exposure == 0
            || end > 10000
            || self.doc.layers.len() >= 100
            || name.chars().count() > 256
            || fps.is_some_and(|fps| !(1..=240).contains(&fps))
        {
            return Err("Unsupported sequence length, FPS or layer count".into());
        }
        if assets
            .iter()
            .any(|a| a.dimensions().iter().any(|&side| side == 0 || side > 4096))
        {
            return Err("The sequence must contain supported still images only".into());
        }
        let total = self
            .doc
            .extras
            .media
            .assets
            .iter()
            .chain(assets.iter())
            .try_fold(0usize, |n, a| n.checked_add(a.data.len()))
            .ok_or("Media budget exceeded")?;
        if total > 256 * 1024 * 1024 {
            return Err("Media library exceeds 256 MiB".into());
        }
        self.finish_pending_pen();
        self.checkpoint();
        self.extend_media_timeline(end);
        if let Some(fps) = fps {
            self.doc.fps = fps;
        }
        let count = self.doc.layers[0].frames.len();
        let mut layer = Layer {
            name: name.trim().to_owned(),
            visible: true,
            frames: vec![SharedVec::default(); count].into(),
            options: Default::default(),
        };
        layer.options.keys = (0..count).map(|f| f == 0).collect::<Vec<_>>().into();
        for (index, asset) in assets.into_iter().enumerate() {
            let [w, h] = asset.dimensions();
            let scale = if fit {
                (self.doc.width as f32 / w as f32).min(self.doc.height as f32 / h as f32)
            } else {
                1.
            };
            let size = egui::vec2(w as f32, h as f32) * scale;
            let center = egui::pos2(self.doc.width as f32 / 2., self.doc.height as f32 / 2.);
            let rect = egui::Rect::from_center_size(center, size);
            let mark = Mark {
                points: vec![rect.min.into(), rect.max.into()].into(),
                rectangle: true,
                closed: true,
                stroke: false,
                fill: Some([0; 4]),
                style: crate::features::MarkStyle {
                    media_image: Some(asset.id),
                    track: crate::features::id(),
                    ..Default::default()
                },
                ..Default::default()
            };
            let cel: SharedVec<Mark> = vec![mark].into();
            let frame = start + index * exposure;
            layer.options.keys[frame] = true;
            for f in frame..frame + exposure {
                layer.frames[f] = cel.clone();
            }
            self.doc.extras.media.assets.push(asset);
        }
        if end < count {
            layer.options.keys[end] = true;
        }
        self.doc.layers.push(layer);
        self.layer = self.doc.layers.len() - 1;
        self.frame = start;
        self.playing = false;
        self.status = format!("Imported image sequence · {}–{}", start + 1, end);
        self.recover();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_sort_numerically_without_integer_overflow() {
        let mut names = vec![
            "shot10.png",
            "shot2.png",
            "shot001.png",
            "shot9999999999999999999999.png",
        ];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec![
                "shot001.png",
                "shot2.png",
                "shot10.png",
                "shot9999999999999999999999.png"
            ]
        );
    }
    #[test]
    fn sequence_creates_exposures_embeds_media_and_undoes_once() {
        let ctx = egui::Context::default();
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        e.doc = crate::fresh_document();
        let original = serde_json::to_value(&e.doc).unwrap();
        let assets = (0..3)
            .map(|i| {
                crate::media::bitmap([i * 80, 20, 30, 255].repeat(16), 4, 4, format!("frame{i}"))
            })
            .collect();
        e.insert_image_sequence(assets, "Sequence", 2, 2, Some(24), true)
            .unwrap();
        let layer = e.doc.layers.last().unwrap();
        assert!(!layer.key(3));
        assert!(layer.key(4));
        assert!(layer.key(8));
        assert!(layer.frames[8].is_empty());
        e.doc.validate().unwrap();
        let bytes = crate::native_project::encode(&e.doc).unwrap();
        let restored = crate::native_project::decode_if_native(&bytes)
            .unwrap()
            .unwrap();
        restored.validate().unwrap();
        let mut cache = Default::default();
        assert_ne!(
            crate::exporter::raster(&restored, 2, 64, 36, &ctx, &mut cache),
            crate::exporter::raster(&restored, 4, 64, 36, &ctx, &mut cache)
        );
        assert_eq!(e.undo.len(), 1);
        e.history(false);
        assert_eq!(serde_json::to_value(&e.doc).unwrap(), original);
    }
}
