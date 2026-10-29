#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod brush;
mod canvas;
mod design;
mod document;
mod fla;
mod fla_zip;
mod workspace;
use document::{Document, Layer, Mark};
use eframe::egui::{self, Color32};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Editor {
    doc: Document,
    frame: usize,
    layer: usize,
    color: Color32,
    width: f32,
    smoothing: f32,
    tool: canvas::Tool,
    canvas: canvas::Canvas,
    onion: bool,
    playing: bool,
    tick: Instant,
    draft: Option<Mark>,
    undo: Vec<Document>,
    redo: Vec<Document>,
    path: Option<PathBuf>,
    status: String,
    recovery: PathBuf,
}
impl Editor {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        design::theme(&cc.egui_ctx);
        let recovery = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Trazo")
            .join("recovery.json");
        let recovered = std::fs::read(&recovery)
            .ok()
            .and_then(|b| serde_json::from_slice::<Document>(&b).ok())
            .filter(|d| d.validate().is_ok());
        let status = if recovered.is_some() {
            "Sesión recuperada"
        } else {
            "Dibuja tu primer fotograma"
        }
        .into();
        Self {
            doc: recovered.unwrap_or_default(),
            frame: 0,
            layer: 0,
            color: Color32::from_rgb(89, 72, 220),
            width: 4.,
            smoothing: 0.7,
            tool: canvas::Tool::Brush,
            canvas: canvas::Canvas::default(),
            onion: true,
            playing: false,
            tick: Instant::now(),
            draft: None,
            undo: vec![],
            redo: vec![],
            path: None,
            status,
            recovery,
        }
    }
    fn checkpoint(&mut self) {
        self.canvas.clear_selection();
        self.undo.push(self.doc.clone());
        if self.undo.len() > 60 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn recover(&mut self) {
        if let Some(parent) = self.recovery.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = write_document(&self.recovery, &self.doc) {
            self.status = format!("No se pudo recuperar sesión: {e}");
        }
    }
    fn save_project(&mut self, save_as: bool) {
        let target = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Adobe Animate / LibreToon", &["fla"])
                .set_file_name("animacion.fla")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(path) = target {
            match write_document(&path, &self.doc) {
                Ok(()) => {
                    self.status = "Proyecto guardado".into();
                    self.path = Some(path);
                }
                Err(e) => self.status = e,
            }
        }
    }
    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Adobe Animate / LibreToon", &["fla"])
            .add_filter("Proyectos Trazo anteriores", &["trazo"])
            .pick_file()
        {
            let result = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|b| {
                    if b.starts_with(b"PK")
                        || path
                            .extension()
                            .is_some_and(|e| e.eq_ignore_ascii_case("fla"))
                    {
                        fla::decode(&b)
                    } else {
                        serde_json::from_slice::<Document>(&b).map_err(|e| e.to_string())
                    }
                })
                .and_then(|d| {
                    d.validate()?;
                    Ok(d)
                });
            match result {
                Ok(d) => {
                    self.checkpoint();
                    self.canvas.clear_selection();
                    self.doc = d;
                    self.frame = 0;
                    self.layer = 0;
                    self.path = if path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("fla"))
                    {
                        Some(path)
                    } else {
                        None
                    };
                    self.playing = false;
                    self.recover();
                    self.status = "Proyecto abierto".into();
                }
                Err(e) => self.status = format!("No se pudo abrir: {e}"),
            }
        }
    }
    fn history(&mut self, redo: bool) {
        let entry = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(d) = entry {
            if redo {
                self.undo.push(self.doc.clone())
            } else {
                self.redo.push(self.doc.clone())
            };
            self.canvas.clear_selection();
            self.doc = d;
            self.frame = self.frame.min(self.doc.layers[0].frames.len() - 1);
            self.layer = self.layer.min(self.doc.layers.len() - 1);
            self.recover();
        }
    }
}
fn write_document(path: &std::path::Path, doc: &Document) -> Result<(), String> {
    let bytes = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("fla"))
    {
        fla::encode(doc)?
    } else {
        serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?
    };
    let temporary = path.with_extension("libretoon.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}
impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_project(false);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.history(false);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
            self.history(true);
        }
        if self.playing {
            let elapsed = self.tick.elapsed().as_secs_f64();
            let steps = (elapsed * self.doc.fps as f64).floor() as usize;
            if steps > 0 {
                self.frame = (self.frame + steps) % self.doc.layers[0].frames.len();
                self.tick = Instant::now();
            }
            ctx.request_repaint_after(Duration::from_millis(8));
        }
        self.workspace(ui);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(29, 30, 34))
                    .inner_margin(24.),
            )
            .show(ui, |ui| {
                self.canvas_ui(ui);
            });
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "LibreToon · Animación 2D",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1360., 860.])
                .with_min_inner_size([1050., 700.])
                .with_icon(design::window_icon()),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Editor::new(cc)))),
    )
}
