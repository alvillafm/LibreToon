use crate::{Document, Editor, canvas::View, i18n::tr};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
#[derive(Clone)]
pub struct ExportSettings {
    pub name: String,
    pub folder: String,
    pub format: usize,
    pub first: usize,
    pub last: usize,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: u32,
    pub loops: i32,
    pub colors: u32,
    pub dither: bool,
    pub svg_sequence: bool,
}
const FORMATS: &[&str] = &["MP4", "SVG", "MKV", "AVI", "MOV", "GIF"];
impl ExportSettings {
    fn extension(&self) -> &str {
        match self.format {
            0 => "mp4",
            1 => "svg",
            2 => "mkv",
            3 => "avi",
            4 => "mov",
            _ => "gif",
        }
    }
    fn target(&self) -> Result<PathBuf, String> {
        if self.name.trim().is_empty()
            || self
                .name
                .chars()
                .any(|c| r#"<>:"/\|?*"#.contains(c) || c.is_control())
            || self.name.ends_with(['.', ' '])
        {
            return Err(tr("Nombre de archivo no válido").into());
        }
        let folder = PathBuf::from(&self.folder);
        if !folder.is_dir() {
            return Err(tr("La carpeta no existe").into());
        }
        let mut path = folder.join(self.name.trim());
        path.set_extension(self.extension());
        Ok(path)
    }
}
fn hidden(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        _command.creation_flags(0x08000000);
    }
}
pub fn ffmpeg_path(configured: &str) -> PathBuf {
    if !configured.is_empty() {
        return configured.into();
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().take(5) {
            let path = dir.join("assets/tools/ffmpeg.exe");
            if path.is_file() {
                return path;
            }
        }
    }
    PathBuf::from("ffmpeg")
}
impl Editor {
    pub fn open_export(&mut self, gif: bool) {
        self.leave_symbol_context_for_document_change();
        self.finish_pending_pen();
        if self.studio.prefs.ffmpeg.is_empty()
            || !std::path::Path::new(&self.studio.prefs.ffmpeg).is_file()
        {
            let bundled = ffmpeg_path("");
            if bundled.is_file() {
                self.studio.prefs.ffmpeg = bundled.to_string_lossy().into_owned();
            }
        }
        let folder = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        self.studio.export = Some(ExportSettings {
            name: "animation".into(),
            folder,
            format: if gif { 5 } else { 0 },
            first: 1,
            last: crate::animation::movie_count(&self.doc),
            width: ((self.doc.width as f32
                * (4096. / self.doc.width.max(self.doc.height) as f32).min(1.))
            .round() as u32)
                .max(2),
            height: ((self.doc.height as f32
                * (4096. / self.doc.width.max(self.doc.height) as f32).min(1.))
            .round() as u32)
                .max(2),
            fps: if gif {
                self.doc.fps.min(100)
            } else {
                self.doc.fps
            },
            quality: 20,
            loops: 0,
            colors: 256,
            dither: true,
            svg_sequence: false,
        });
    }
    pub fn export_ui(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.studio.export_job {
            match receiver.try_recv() {
                Ok(result) => {
                    self.status = match result {
                        Ok(p) => format!("{} {}", tr("Exportado:"), p.display()),
                        Err(e) => e,
                    };
                    self.studio.export_job = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.status = tr("La exportación se interrumpió").into();
                    self.studio.export_job = None;
                }
                _ => {}
            }
        }
        if self.studio.export_job.is_some() {
            egui::Window::new(tr("Exportando…"))
                .id(egui::Id::unique("export-progress"))
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    let value = self.studio.export_progress.load(Ordering::Relaxed) as f32 / 1000.;
                    ui.add(egui::ProgressBar::new(value).show_percentage());
                    if ui.button(tr("Cancelar")).clicked() {
                        self.studio.export_cancel.store(true, Ordering::Relaxed);
                    }
                });
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        let Some(mut s) = self.studio.export.take() else {
            return;
        };
        let mut open = true;
        let mut start = false;
        egui::Window::new(tr(if s.format == 5 {
            "Exportar GIF…"
        } else {
            "Exportar vídeo…"
        }))
        .id(egui::Id::unique("export-dialog"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Grid::new("export-fields")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label(tr("Nombre"));
                    ui.text_edit_singleline(&mut s.name);
                    ui.end_row();
                    ui.label(tr("Carpeta"));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut s.folder);
                        if ui.button("…").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .set_directory(&s.folder)
                                .pick_folder()
                            {
                                s.folder = p.to_string_lossy().into_owned();
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Formato"));
                    if s.format == 5 {
                        ui.label("GIF");
                    } else {
                        egui::ComboBox::from_id_salt("export-format")
                            .selected_text(FORMATS[s.format])
                            .show_ui(ui, |ui| {
                                for (i, f) in FORMATS.iter().take(5).enumerate() {
                                    ui.selectable_value(&mut s.format, i, *f);
                                }
                            });
                    }
                    ui.end_row();
                    if s.format == 1 {
                        ui.label(tr("SVG"));
                        ui.checkbox(&mut s.svg_sequence, tr("Secuencia de fotogramas"));
                        ui.end_row();
                    } else {
                        ui.label(tr("Resolución"));
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut s.width).range(2..=4096));
                            ui.label("×");
                            ui.add(egui::DragValue::new(&mut s.height).range(2..=4096));
                            if ui
                                .small_button("↔")
                                .on_hover_text(tr("Conservar proporción"))
                                .clicked()
                            {
                                s.height = ((s.width as f32 * self.doc.height as f32
                                    / self.doc.width as f32)
                                    .round() as u32)
                                    .clamp(2, 4096);
                            }
                        });
                        ui.end_row();
                        ui.label("FPS");
                        ui.add(
                            egui::DragValue::new(&mut s.fps)
                                .range(1..=if s.format == 5 { 100 } else { 240 }),
                        );
                        ui.end_row();
                    }
                    if s.format != 1 || s.svg_sequence {
                        ui.label(tr("Fotogramas"));
                        ui.horizontal(|ui| {
                            let count = crate::animation::movie_count(&self.doc);
                            ui.add(egui::DragValue::new(&mut s.first).range(1..=count));
                            ui.label("–");
                            ui.add(egui::DragValue::new(&mut s.last).range(s.first..=count));
                        });
                        ui.end_row();
                    }
                    match s.format {
                        0 | 2 => {
                            ui.label(tr("Calidad (CRF)"));
                            ui.add(egui::Slider::new(&mut s.quality, 0..=40));
                            ui.end_row();
                            ui.label(tr("Códec"));
                            ui.label("H.264 / yuv420p");
                            ui.end_row();
                        }
                        3 => {
                            ui.label(tr("Códec"));
                            ui.label("MPEG-4 / AVI");
                            ui.end_row();
                        }
                        4 => {
                            ui.label(tr("Códec"));
                            ui.label("Apple ProRes 422 HQ");
                            ui.end_row();
                        }
                        5 => {
                            ui.label(tr("Colores"));
                            ui.add(egui::Slider::new(&mut s.colors, 4..=256));
                            ui.end_row();
                            ui.label(tr("Repeticiones (0 = infinito)"));
                            ui.add(egui::DragValue::new(&mut s.loops).range(0..=100));
                            ui.end_row();
                            ui.label(tr("Tramado"));
                            ui.checkbox(&mut s.dither, "Sierra");
                            ui.end_row();
                        }
                        _ => {}
                    }
                });
            if s.format == 1 && !s.svg_sequence {
                ui.label(tr("Exporta el fotograma actual como SVG vectorial."));
            }
            if s.format == 5 {
                ui.label(tr(
                    "GIF usa intervalos de centésimas de segundo; los FPS pueden redondearse.",
                ));
            }
            if s.format != 1 {
                ui.collapsing("FFmpeg", |ui| {
                    ui.label(tr("FFmpeg incluido: no requiere descarga."));
                    ui.text_edit_singleline(&mut self.studio.prefs.ffmpeg);
                    if ui.button(tr("Buscar FFmpeg…")).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("FFmpeg", &["exe"])
                            .pick_file()
                        {
                            self.studio.prefs.ffmpeg = path.to_string_lossy().into_owned();
                        }
                    }
                    if ui.button(tr("Configurar FFmpeg incluido")).clicked() {
                        self.studio.prefs.ffmpeg = ffmpeg_path("").to_string_lossy().into_owned();
                        if let Err(e) = self.studio.save() {
                            self.status = e;
                        }
                    }
                });
            }
            ui.separator();
            if ui.button(tr("Exportar")).clicked() {
                start = true;
            }
        });
        s.first = s.first.clamp(1, crate::animation::movie_count(&self.doc));
        s.last = s
            .last
            .clamp(s.first, crate::animation::movie_count(&self.doc));
        if start {
            if let Err(e) = self.studio.save() {
                self.status = e;
            }
            match s.target() {
                Ok(path) => {
                    let mut targets = vec![path.clone()];
                    if s.format == 1 && s.svg_sequence {
                        targets = (s.first..=s.last)
                            .map(|f| {
                                path.with_file_name(format!(
                                    "{}-{f:05}.svg",
                                    path.file_stem().unwrap().to_string_lossy()
                                ))
                            })
                            .collect();
                    }
                    let overwrite = targets.iter().any(|p| p.exists());
                    if overwrite
                        && rfd::MessageDialog::new()
                            .set_title(tr("Reemplazar archivo"))
                            .set_description(tr("Ya existe un archivo de destino. ¿Reemplazarlo?"))
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .show()
                            != rfd::MessageDialogResult::Yes
                    {
                        self.studio.export = Some(s);
                        return;
                    }
                    let doc = self.project_document();
                    let current = crate::animation::movie_offset(&doc, self.frame);
                    let ffmpeg = ffmpeg_path(&self.studio.prefs.ffmpeg);
                    self.studio.export_progress.store(0, Ordering::Relaxed);
                    self.studio.export_cancel.store(false, Ordering::Relaxed);
                    let progress = self.studio.export_progress.clone();
                    let cancel = self.studio.export_cancel.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.studio.export_job = Some(rx);
                    std::thread::spawn(move || {
                        let result = run_export(&doc, &s, current, &path, ffmpeg, progress, cancel);
                        let _ = tx.send(result.map(|_| path));
                    });
                }
                Err(e) => {
                    self.status = e;
                    self.studio.export = Some(s);
                }
            }
        } else if open {
            self.studio.export = Some(s);
        }
    }
}
fn run_export(
    doc: &Document,
    s: &ExportSettings,
    current: usize,
    path: &std::path::Path,
    ffmpeg: PathBuf,
    progress: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    if s.format > 5
        || s.first == 0
        || s.first > s.last
        || s.last > crate::animation::movie_count(doc)
        || !(1..=240).contains(&s.fps)
        || (s.format != 1 && (!(2..=4096).contains(&s.width) || !(2..=4096).contains(&s.height)))
    {
        return Err("Parámetros de exportación fuera de rango".into());
    }
    if s.format == 1 {
        if s.svg_sequence {
            for f in s.first..=s.last {
                if cancel.load(Ordering::Relaxed) {
                    return Err(tr("Exportación cancelada").into());
                }
                let p = path.with_file_name(format!(
                    "{}-{f:05}.svg",
                    path.file_stem().unwrap().to_string_lossy()
                ));
                std::fs::write(p, crate::animation::movie_svg(doc, f - 1))
                    .map_err(|e| e.to_string())?;
                progress.store(
                    (f - s.first + 1) * 1000 / (s.last - s.first + 1),
                    Ordering::Relaxed,
                );
            }
        } else {
            std::fs::write(path, crate::animation::movie_svg(doc, current))
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let w = s.width;
    let h = s.height;
    let temp = path.with_file_name(format!(
        ".libretoon-export-{}-{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        s.extension()
    ));
    let mut command = Command::new(ffmpeg);
    hidden(&mut command);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgb24",
        "-video_size",
        &format!("{w}x{h}"),
        "-framerate",
        &s.fps.to_string(),
        "-i",
        "pipe:0",
        "-an",
    ]);
    match s.format {
        0 | 2 => {
            command.args([
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-preset",
                "medium",
                "-crf",
                &s.quality.to_string(),
            ]);
            if s.format == 0 {
                command.args(["-movflags", "+faststart"]);
            }
        }
        3 => {
            command.args([
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white",
                "-c:v",
                "mpeg4",
                "-q:v",
                "3",
            ]);
        }
        4 => {
            command.args([
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white",
                "-c:v",
                "prores_ks",
                "-profile:v",
                "3",
                "-pix_fmt",
                "yuv422p10le",
            ]);
        }
        5 => {
            let filter = format!(
                "split[a][b];[a]palettegen=max_colors={}:stats_mode=single[p];[b][p]paletteuse=new=1:dither={}",
                s.colors,
                if s.dither { "sierra2_4a" } else { "none" }
            );
            command.args(["-filter_complex", &filter, "-loop", &s.loops.to_string()]);
        }
        _ => return Err("Unsupported format".into()),
    }
    command
        .arg(&temp)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        format!(
            "{}: {e}",
            tr("No se pudo iniciar FFmpeg. Selecciona ffmpeg.exe en las opciones de exportación")
        )
    })?;
    let mut stderr = child.stderr.take().unwrap();
    let log = std::thread::spawn(move || read_log(&mut stderr));
    let mut input = child.stdin.take().unwrap();
    let ctx = egui::Context::default();
    let mut cache = crate::paint::RenderCache::default();
    let count = s.last - s.first + 1;
    let frames = ((count as f64 * s.fps as f64 / doc.fps as f64).ceil() as usize).max(1);
    let result = (|| {
        for index in 0..frames {
            if cancel.load(Ordering::Relaxed) {
                return Err(tr("Exportación cancelada").into());
            }
            let frame = s.first - 1
                + ((index as u64 * doc.fps as u64 / s.fps as u64) as usize).min(count - 1);
            let pixels = raster(doc, frame, w, h, &ctx, &mut cache);
            input.write_all(&pixels).map_err(|e| e.to_string())?;
            progress.store((index + 1) * 950 / frames, Ordering::Relaxed);
        }
        Ok(())
    })();
    drop(input);
    if result.is_err() {
        let _ = child.kill();
    }
    // Encoding also supports cancellation after all frames were sent.
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => break Err(e.to_string()),
        }
    };
    let log = log.join().unwrap_or_default();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(if log.is_empty() {
            e
        } else {
            format!("{e}\n{log}")
        });
    }
    if cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&temp);
        return Err(tr("Exportación cancelada").into());
    }
    if !status?.success() {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("FFmpeg: {log}"));
    }
    std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
    progress.store(1000, Ordering::Relaxed);
    Ok(())
}
/// Rasterize the same tessellated geometry as the editor, with no UI or onion skin.
pub(crate) fn raster(
    doc: &Document,
    frame: usize,
    w: u32,
    h: u32,
    ctx: &egui::Context,
    cache: &mut crate::paint::RenderCache,
) -> Vec<u8> {
    let size = Vec2::new(w as f32, h as f32);
    let scale = (w as f32 / doc.width as f32).min(h as f32 / doc.height as f32);
    let view = View {
        center: Pos2::ZERO + size / 2.,
        scale,
        angle: 0.,
        stage_center: Pos2::new(doc.width as f32 / 2., doc.height as f32 / 2.),
    };
    cache.begin_frame();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        },
        |ui| {
            let painter = ui
                .painter()
                .with_clip_rect(Rect::from_min_size(Pos2::ZERO, size));
            for (_, _, m) in crate::animation::resolve_movie(doc, frame, true) {
                crate::paint::draw(&painter, &m, view, false, false, Some(cache));
            }
        },
    );
    output.textures_delta.clear();
    let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut pixels = vec![255u8; (w * h * 3) as usize];
    for primitive in meshes {
        if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
            let clip = primitive
                .clip_rect
                .intersect(Rect::from_min_size(Pos2::ZERO, size));
            for t in mesh.indices.chunks_exact(3) {
                let vertices = [
                    mesh.vertices[t[0] as usize],
                    mesh.vertices[t[1] as usize],
                    mesh.vertices[t[2] as usize],
                ];
                triangle(&mut pixels, w, h, vertices, clip);
            }
        }
    }
    pixels
}
fn triangle(pixels: &mut [u8], w: u32, h: u32, v: [egui::epaint::Vertex; 3], clip: Rect) {
    let [a, b, c] = v.map(|v| v.pos);
    let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
    let det = cross(b - a, c - a);
    if det.abs() < 1e-6 {
        return;
    }
    let min = Pos2::new(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y)).max(clip.min);
    let max = Pos2::new(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y)).min(clip.max);
    let colors = v.map(|v| v.color.to_array());
    for y in (min.y.floor().max(0.) as u32)..(max.y.ceil().max(0.) as u32).min(h) {
        for x in (min.x.floor().max(0.) as u32)..(max.x.ceil().max(0.) as u32).min(w) {
            let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
            let weights = [
                cross(b - p, c - p) / det,
                cross(c - p, a - p) / det,
                cross(a - p, b - p) / det,
            ];
            if weights.iter().any(|&v| v < 0.) {
                continue;
            }
            // Top-left coverage prevents drawing a shared edge twice.
            let mut covered = true;
            for (i, weight) in weights.iter().enumerate() {
                if weight.abs() < 1e-7 {
                    let (u, z) = match i {
                        0 => (b, c),
                        1 => (c, a),
                        _ => (a, b),
                    };
                    let d = if det > 0. { z - u } else { u - z };
                    if !(d.y < 0. || (d.y == 0. && d.x > 0.)) {
                        covered = false;
                    }
                }
            }
            if !covered {
                continue;
            }
            let alpha = (0..3)
                .map(|i| weights[i] * colors[i][3] as f32)
                .sum::<f32>()
                / 255.;
            let offset = ((y * w + x) * 3) as usize;
            for channel in 0..3 {
                let source = (0..3)
                    .map(|i| weights[i] * colors[i][channel] as f32)
                    .sum::<f32>();
                pixels[offset + channel] = (source + pixels[offset + channel] as f32 * (1. - alpha))
                    .clamp(0., 255.)
                    .round() as u8;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movie_raster_respects_scene_order_motion_and_camera_keys() {
        let mut d = crate::fresh_document();
        d.width = 64;
        d.height = 64;
        let mark = crate::document::Mark {
            rectangle: true,
            closed: true,
            stroke: false,
            fill: Some([220, 40, 20, 255]),
            points: vec![[4., 16.], [20., 48.]].into(),
            ..Default::default()
        };
        d.layers[0].frames[0] = vec![mark.clone()].into();
        d.layers[0].frames[10] =
            vec![crate::vector::transform(&mark, |p| p + Vec2::new(40., 0.))].into();
        d.layers[0].options.keys[10] = true;
        d.layers[0].options.tweens.push(crate::animation::Tween {
            start: 0,
            end: 10,
            curve: Default::default(),
        });
        let first = crate::animation::scene_from(&d, 1, "Motion".into());
        let mut second = crate::fresh_document();
        let mut blue = mark.clone();
        blue.fill = Some([20, 40, 220, 255]);
        second.layers[0].frames[0] = vec![blue].into();
        d.extras.scenes = vec![
            first,
            crate::animation::scene_from(&second, 2, "Blue".into()),
        ]
        .into();
        let ctx = egui::Context::default();
        let mut cache = Default::default();
        let p = raster(&d, 5, 64, 64, &ctx, &mut cache);
        let pixel = |p: &[u8], x: usize| p[(32 * 64 + x) * 3..(32 * 64 + x) * 3 + 3].to_vec();
        assert_eq!(pixel(&p, 32), vec![220, 40, 20]);
        assert_eq!(pixel(&p, 10), vec![255, 255, 255]);
        let p = raster(&d, 24, 64, 64, &ctx, &mut cache);
        assert_eq!(pixel(&p, 10), vec![20, 40, 220]);
        d.extras.cameras.push(crate::features::Camera {
            id: 1,
            name: "Camera".into(),
            start: 0,
            duration: 24,
            center: [32., 32.],
            zoom: 1.,
            angle: 0.,
            animation: Default::default(),
        });
        crate::animation::camera_record(
            &mut d.extras.cameras[0],
            crate::animation::CameraKey {
                frame: 10,
                center: [52., 32.],
                zoom: 1.,
                angle: 0.,
            },
        );
        let p = raster(&d, 5, 64, 64, &ctx, &mut cache);
        assert_eq!(pixel(&p, 22), vec![220, 40, 20]);
        assert_eq!(pixel(&p, 37), vec![255, 255, 255]);
    }
    #[test]
    fn radial_gradient_is_rasterized_with_multiple_colors_and_alpha() {
        let mut doc = Document::default();
        doc.width = 64;
        doc.height = 64;
        let mut mark = crate::document::Mark {
            points: vec![[0., 0.], [64., 64.]].into(),
            rectangle: true,
            closed: true,
            fill: Some([255, 255, 255, 255]),
            stroke: false,
            ..Default::default()
        };
        mark.style.gradient = Some(crate::features::Gradient {
            radial: true,
            start: [32., 32.],
            end: [60., 32.],
            stops: vec![
                crate::features::Stop {
                    t: 0.,
                    color: [255, 0, 0, 255],
                },
                crate::features::Stop {
                    t: 0.5,
                    color: [0, 255, 0, 255],
                },
                crate::features::Stop {
                    t: 1.,
                    color: [0, 0, 255, 128],
                },
            ]
            .into(),
        });
        doc.layers[0].frames[0].push(mark);
        let pixels = raster(
            &doc,
            0,
            64,
            64,
            &egui::Context::default(),
            &mut Default::default(),
        );
        let pixel = |x: usize| &pixels[(32 * 64 + x) * 3..(32 * 64 + x) * 3 + 3];
        assert!(pixel(32)[0] > 200);
        assert!(pixel(45)[1] > 180);
        assert!(pixel(59)[2] > 200);
        assert!(pixel(59)[0] > 80);
    }
    #[test]
    fn raster_uses_only_visible_document_geometry() {
        let mut d = Document::default();
        d.width = 64;
        d.height = 64;
        d.layers[0].frames[0].push(crate::document::Mark {
            points: vec![[10., 10.], [54., 10.], [54., 54.], [10., 54.]].into(),
            fill: Some([220, 40, 20, 255]),
            stroke: false,
            closed: true,
            ..Default::default()
        });
        let ctx = egui::Context::default();
        let mut cache = Default::default();
        let pixels = raster(&d, 0, 64, 64, &ctx, &mut cache);
        assert_eq!(
            &pixels[(32 * 64 + 32) * 3..(32 * 64 + 32) * 3 + 3],
            &[220, 40, 20]
        );
        assert_eq!(&pixels[0..3], &[255, 255, 255]);
        d.layers[0].visible = false;
        assert!(
            raster(&d, 0, 64, 64, &ctx, &mut cache)
                .iter()
                .all(|&v| v == 255)
        );
    }
    #[test]
    #[ignore]
    fn export_all_containers_with_ffmpeg() {
        let mut d = Document::default();
        d.width = 64;
        d.height = 64;
        d.fps = 24;
        for (f, color) in [[230, 30, 20, 255], [20, 230, 30, 255], [30, 20, 230, 255]]
            .into_iter()
            .enumerate()
        {
            d.layers[0].frames[f].push(crate::document::Mark {
                points: vec![[10., 10.], [54., 54.]].into(),
                rectangle: true,
                closed: true,
                fill: Some(color),
                stroke: false,
                ..Default::default()
            });
        }
        let dir =
            std::env::temp_dir().join(format!("libretoon-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for format in 0..6 {
            let s = ExportSettings {
                name: "test".into(),
                folder: dir.to_string_lossy().into_owned(),
                format,
                first: 1,
                last: 3,
                width: 64,
                height: 64,
                fps: 24,
                quality: 20,
                loops: 0,
                colors: 64,
                dither: true,
                svg_sequence: false,
            };
            let path = s.target().unwrap();
            run_export(
                &d,
                &s,
                0,
                &path,
                ffmpeg_path(""),
                Default::default(),
                Default::default(),
            )
            .unwrap();
            assert!(std::fs::metadata(&path).unwrap().len() > 50);
            if format != 1 {
                let mut cmd = Command::new(ffmpeg_path(""));
                hidden(&mut cmd);
                let decoded = cmd
                    .args(["-hide_banner", "-loglevel", "error", "-i"])
                    .arg(&path)
                    .args([
                        "-fps_mode",
                        "passthrough",
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "rgb24",
                        "pipe:1",
                    ])
                    .output()
                    .unwrap();
                assert!(
                    decoded.status.success(),
                    "{}",
                    String::from_utf8_lossy(&decoded.stderr)
                );
                assert_eq!(decoded.stdout.len(), 64 * 64 * 3 * 3, "format={format}");
                for f in 0..3 {
                    let offset = f * 64 * 64 * 3 + (32 * 64 + 32) * 3;
                    let pixel = &decoded.stdout[offset..offset + 3];
                    assert!(pixel[f] > 180, "format={format} frame={f} pixel={pixel:?}");
                    assert!(pixel[(f + 1) % 3] < 80);
                }
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Drain stderr fully to avoid pipe deadlocks, retaining only the last 64 KiB.
fn read_log(reader: &mut impl Read) -> String {
    let mut tail = Vec::new();
    let mut block = [0u8; 4096];
    loop {
        match reader.read(&mut block) {
            Ok(0) => break,
            Ok(n) => {
                if tail.len() + n > 65536 {
                    let remove = tail.len() + n - 65536;
                    tail.drain(..remove);
                }
                tail.extend_from_slice(&block[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&tail).into_owned()
}
#[cfg(test)]
mod audit_log_tests {
    #[test]
    fn ffmpeg_logs_are_drained_with_bounded_storage() {
        let mut bytes = std::io::Cursor::new(vec![b'x'; 300000]);
        let result = super::read_log(&mut bytes);
        assert_eq!(bytes.position(), 300000);
        assert_eq!(result.len(), 65536);
    }
}
