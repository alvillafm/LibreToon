//! Previewed vector conversions, computed away from the UI and committed once.
use crate::{Editor, Mark, i18n::tr};
use eframe::egui::{self, Color32};
use geo::Buffer;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
type Paths = Vec<((usize, usize), Vec<Mark>)>;
#[derive(Default)]
pub(crate) struct State {
    preview: Option<Preview>,
}
struct Preview {
    owner: std::path::PathBuf,
    epoch: usize,
    frame: usize,
    originals: Vec<((usize, usize), Mark)>,
    expand: bool,
    distance: f32,
    keep: bool,
    job: Option<mpsc::Receiver<Result<Paths, String>>>,
    result: Option<Paths>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    error: Option<String>,
}
fn converted(mark: &Mark, expand: bool, distance: f32) -> Result<Vec<Mark>, String> {
    if mark.nodes.len().saturating_add(mark.points.len()) > 6000 {
        return Err(tr("Convert a smaller path first (maximum 6000 control points).").into());
    }
    if !crate::node_edit::supported(mark) {
        return Err(tr("Select editable vector paths.").into());
    }
    if expand {
        if !mark.stroke {
            return Err(tr("Expand Stroke needs a visible stroke.").into());
        }
        let mut result = Vec::new();
        if mark.fill.is_some() {
            let mut fill = mark.clone();
            fill.stroke = false;
            fill.style.stroke_gradient = None;
            result.push(fill);
        }
        for mut shape in crate::vector::polygons_to_marks(crate::width::outline(mark), mark.color) {
            shape.style = mark.style.clone();
            shape.style.gradient = mark.style.stroke_gradient.clone();
            shape.style.stroke_gradient = None;
            shape.style.palette_fill = mark.style.palette_stroke;
            shape.style.palette_stroke = None;
            shape.style.linked_outline_fill = false;
            result.push(shape);
        }
        Ok(result)
    } else {
        let Some(color) = mark.fill.filter(|_| mark.closed || mark.rectangle) else {
            return Err(tr("Offset Path needs a closed filled shape.").into());
        };
        let polygon = crate::vector::polygon(mark).buffer(f64::from(distance));
        Ok(crate::vector::polygons_to_marks(polygon, color)
            .into_iter()
            .map(|mut shape| {
                shape.style = mark.style.clone();
                shape.style.stroke_gradient = None;
                shape.style.palette_stroke = None;
                shape.style.linked_outline_fill = false;
                shape
            })
            .collect())
    }
}
impl Editor {
    pub(crate) fn begin_geometry_conversion(&mut self, expand: bool) {
        if self
            .geometry_ops
            .preview
            .as_ref()
            .is_some_and(|p| p.job.is_some())
        {
            return;
        }
        let originals: Vec<_> = self
            .canvas
            .selected
            .iter()
            .filter_map(|&(l, m)| {
                let layer = self.doc.layers.get(l).filter(|l| !l.options.locked)?;
                let evaluated = crate::animation::marks(layer, self.frame);
                Some(((l, m), evaluated.get(m)?.clone()))
            })
            .collect();
        if originals.is_empty() {
            return;
        }
        self.geometry_ops.preview = Some(Preview {
            owner: self.recovery.clone(),
            epoch: self.doc.layers.identity(),
            frame: self.frame,
            originals,
            expand,
            distance: 2.,
            keep: true,
            job: None,
            result: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            error: None,
        });
    }
    pub(crate) fn geometry_conversion_overlay(
        &self,
        painter: &egui::Painter,
        view: crate::canvas::View,
    ) {
        let Some(p) = &self.geometry_ops.preview else {
            return;
        };
        if p.epoch != self.doc.layers.identity()
            || p.frame != self.frame
            || p.owner != self.recovery
        {
            return;
        }
        if let Some(paths) = &p.result {
            for ((l, _), paths) in paths {
                for mark in paths {
                    let mut candidate = mark.clone();
                    candidate.fill = None;
                    candidate.stroke = true;
                    candidate.style.gradient = None;
                    candidate.style.stroke_gradient = None;
                    candidate.color = [65, 220, 205, 255];
                    let local = self.camera_depth_view(view, *l);
                    candidate.width = 1.5 / local.scale.max(0.001);
                    crate::paint::draw(painter, &candidate, local, false, false, None);
                }
            }
        }
    }
    pub(crate) fn geometry_conversion_ui(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.geometry_ops.preview.take() else {
            return;
        };
        let mut open = p.owner == self.recovery
            && p.epoch == self.doc.layers.identity()
            && p.frame == self.frame;
        if let Some(job) = &p.job {
            match job.try_recv() {
                Ok(result) => {
                    p.job = None;
                    match result {
                        Ok(paths) => p.result = Some(paths),
                        Err(error) => p.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    p.job = None;
                    p.error = Some(tr("Geometry preview interrupted").into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
            }
        }
        let mut calculate = false;
        let mut apply = false;
        if open {
            egui::Window::new(tr(if p.expand { "Expand Stroke" } else { "Offset Path" })).open(&mut open).default_width(360.).show(ctx, |ui| {
                ui.label(tr("The original stays unchanged until Apply. Converted boundaries are flattened to 0.04 px tolerance."));
                if !p.expand && ui.add_enabled(p.job.is_none(), egui::Slider::new(&mut p.distance, -100. ..=100.).text(tr("Offset · px"))).changed() { p.result = None; p.error = None; }
                ui.checkbox(&mut p.keep, tr("Keep original as a separate object"));
                ui.colored_label(Color32::from_rgb(65,220,205), tr("Cyan = candidate boundary"));
                if p.job.is_some() { ui.add(egui::ProgressBar::new(p.progress.load(Ordering::Relaxed) as f32 / p.originals.len() as f32).show_percentage()); }
                if let Some(error) = &p.error { ui.label(error); }
                ui.horizontal(|ui| {
                    calculate = ui.add_enabled(p.job.is_none(), egui::Button::new(tr("Preview"))).clicked();
                    apply = ui.add_enabled(p.result.is_some() && p.job.is_none(), egui::Button::new(tr("Apply"))).clicked();
                    if ui.button(tr("Cancel")).clicked() { p.cancel.store(true,Ordering::Relaxed); }
                });
            });
        }
        if !open || p.cancel.load(Ordering::Relaxed) {
            p.cancel.store(true, Ordering::Relaxed);
            return;
        }
        if calculate {
            let originals = p.originals.clone();
            let cancel = p.cancel.clone();
            let progress = p.progress.clone();
            let (tx, rx) = mpsc::sync_channel(1);
            p.job = Some(rx);
            p.result = None;
            p.error = None;
            progress.store(0, Ordering::Relaxed);
            let expand = p.expand;
            let distance = p.distance;
            crate::geometry_jobs::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut paths = Vec::new();
                    for (i, (id, mark)) in originals.iter().enumerate() {
                        if cancel.load(Ordering::Relaxed) {
                            return Err(tr("Canceled").into());
                        }
                        paths.push((*id, converted(mark, expand, distance)?));
                        progress.store(i + 1, Ordering::Relaxed);
                    }
                    Ok(paths)
                }))
                .unwrap_or_else(|_| Err(tr("Could not convert this geometry").into()));
                let _ = tx.send(result);
            });
        }
        if apply {
            if let Some(mut paths) = p.result {
                let ids: Vec<_> = paths.iter().map(|(id, _)| *id).collect();
                self.checkpoint();
                self.materialize_motion(&ids);
                paths.sort_by_key(|(id, _)| *id);
                for ((l, m), mut converted) in paths.into_iter().rev() {
                    for mark in &mut converted {
                        mark.style.track = crate::features::id();
                    }
                    let cel = &mut self.doc.layers[l].frames[self.frame];
                    if p.keep {
                        cel.extend(converted);
                    } else {
                        cel.splice(m..m + 1, converted);
                    }
                }
                self.recover();
            }
            return;
        }
        self.geometry_ops.preview = Some(p);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offset_and_expand_keep_original_and_gradient_channels() {
        let mark = Mark {
            points: vec![[10., 10.], [30., 30.]].into(),
            rectangle: true,
            closed: true,
            fill: Some([255; 4]),
            width: 4.,
            ..Default::default()
        };
        let before = mark.clone();
        let out = converted(&mark, false, 2.).unwrap();
        assert!(crate::vector::control_bounds(&out[0]).width() > 23.9);
        let mut stroke = mark.clone();
        stroke.style.stroke_gradient = Some(Default::default());
        let expanded = converted(&stroke, true, 0.).unwrap();
        assert!(expanded.iter().all(|m| !m.stroke));
        assert!(
            expanded
                .iter()
                .skip(1)
                .all(|m| m.style.gradient.is_some() && m.style.stroke_gradient.is_none())
        );
        assert!(mark == before);
    }
}
