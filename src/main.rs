#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod advanced;
mod animation;
#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod appearance_tests;
mod brush;
mod bucket;
mod canvas;
#[cfg(test)]
mod compat_tests;
mod design;
mod document;
mod documents;
mod drawing;
mod editing;
mod exporter;
mod features;
mod fla;
mod fla_zip;
mod i18n;
mod inspector;
mod library;
mod loading;
mod mannequin;
mod paint;
mod recovery;
mod rig;
#[cfg(test)]
mod rig_tests;
mod rig_ui;
mod scenes;
mod sequencer;
mod shared;
mod studio;
#[cfg(test)]
mod studio_tests;
mod tablet;
mod themes;
mod tools_editor;
mod typography;
mod vector;
mod visual_controls;
mod width;
mod workspace;
mod xfl;
use document::{Document, Mark};
use eframe::egui::{self, Color32};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Editor {
    loader: loading::Loader,
    import_report_open: bool,
    rigging: rig_ui::RigEditor,
    assets: library::LibraryState,
    hub: documents::Hub,
    drawing: drawing::Drawing,
    feature: features::FeatureState,
    animation: animation::State,
    studio: studio::Studio,
    doc: Document,
    frame: usize,
    layer: usize,
    color: Color32,
    fill_color: Color32,
    fill_enabled: bool,
    stroke_enabled: bool,
    width: f32,
    smoothing: f32,
    polygon_sides: usize,
    polygon_star: bool,
    star_ratio: f32,
    zoom_out: bool,
    wand_tolerance: f32,
    style_selection: Vec<(usize, usize)>,
    style_frame: usize,
    style_transaction: bool,
    width_profile: shared::SharedVec<width::WidthStop>,
    width_preset: width::Preset,
    profile_point: Option<usize>,
    gap: bucket::Gap,
    active_fill: bool,
    document_settings: Option<inspector::DocumentSettings>,
    timeline_duration: f32,
    duration_editing: bool,
    profile_transaction: bool,
    renaming_layer: Option<usize>,
    rename_focus: bool,
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
    recovery_writer: recovery::RecoveryWriter,
}
impl Editor {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        design::theme(&cc.egui_ctx);
        #[cfg(not(test))]
        let recovery = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Trazo")
            .join("recovery.json");
        #[cfg(test)]
        let recovery = std::env::temp_dir().join(format!(
            "libretoon-startup-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        let hub = documents::Hub::default();
        let recovered = if hub.has_sessions() {
            None
        } else {
            std::fs::read(&recovery)
                .ok()
                .and_then(|b| serde_json::from_slice::<Document>(&b).ok())
                .filter(|d| d.validate().is_ok())
        };
        let had_recovered = recovered.is_some();
        let status = if recovered.is_some() {
            "Sesión recuperada"
        } else {
            "Dibuja tu primer fotograma"
        }
        .into();
        let mut editor = Self {
            loader: Default::default(),
            import_report_open: false,
            rigging: Default::default(),
            assets: Default::default(),
            hub,
            drawing: Default::default(),
            feature: Default::default(),
            animation: Default::default(),
            studio: studio::Studio::load(),
            doc: recovered.unwrap_or_else(fresh_document),
            frame: 0,
            layer: 0,
            color: Color32::from_rgb(89, 72, 220),
            fill_color: Color32::from_rgb(89, 72, 220),
            fill_enabled: true,
            stroke_enabled: true,
            width: 4.,
            smoothing: 0.7,
            polygon_sides: 5,
            polygon_star: false,
            star_ratio: 0.5,
            zoom_out: false,
            wand_tolerance: 0.,
            style_selection: vec![],
            style_frame: 0,
            style_transaction: false,
            width_profile: Default::default(),
            width_preset: Default::default(),
            profile_point: None,
            gap: Default::default(),
            active_fill: true,
            document_settings: None,
            timeline_duration: 2.,
            duration_editing: false,
            profile_transaction: false,
            renaming_layer: None,
            rename_focus: false,
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
            recovery_writer: recovery::RecoveryWriter::default(),
        };
        editor.initialize_documents(had_recovered);
        i18n::set_locale(editor.studio.prefs.locale());
        themes::ensure(&cc.egui_ctx, editor.studio.prefs.theme);
        i18n::ensure_fonts(&cc.egui_ctx, editor.studio.prefs.locale());
        editor
    }
    fn checkpoint(&mut self) {
        self.hub.dirty();
        self.canvas.clear_selection();
        self.undo.push(self.doc.clone());
        if self.undo.len() > 60 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn recover(&mut self) {
        self.sync_text_styles();
        self.sync_exposures();
        self.recovery_writer
            .submit(self.recovery.clone(), self.project_document());
    }
    fn save_project(&mut self, save_as: bool) {
        self.finish_pending_pen();
        let target = if save_as || self.path.is_none() || self.doc.extras.imported {
            rfd::FileDialog::new()
                .add_filter("Adobe Animate / LibreToon", &["fla"])
                .set_file_name("animacion.fla")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(path) = target {
            let mut saved = self.project_document();
            saved.extras.imported = false;
            match write_document(&path, &saved) {
                Ok(()) => {
                    self.doc.extras.imported = false;
                    for (parent, _, _, _) in &mut self.feature.symbol_stack {
                        parent.extras.imported = false;
                    }
                    self.status = "Proyecto guardado".into();
                    self.path = Some(path);
                    self.saved_document();
                }
                Err(e) => self.status = e,
            }
        }
    }
    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Adobe Animate / LibreToon", &["fla"])
            .add_filter("Proyectos anteriores", &["trazo"])
            .pick_file()
        {
            self.open_document_path(path);
        }
    }
    fn history(&mut self, redo: bool) {
        self.rigging.cancel_gesture();
        let entry = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(d) = entry {
            self.hub.dirty();
            if redo {
                self.undo.push(self.doc.clone())
            } else {
                self.redo.push(self.doc.clone())
            };
            self.canvas.clear_selection();
            self.canvas.cancel_drafts();
            self.draft = None;
            self.doc = d;
            self.frame = self.frame.min(self.doc.layers[0].frames.len() - 1);
            self.layer = self.layer.min(self.doc.layers.len() - 1);
            self.recover();
        }
    }
}
fn fresh_document() -> Document {
    let mut doc = Document::default();
    let mut keys = vec![false; doc.layers[0].frames.len()];
    keys[0] = true;
    doc.layers[0].options.keys = keys.into();
    doc
}
fn write_document(path: &std::path::Path, doc: &Document) -> Result<(), String> {
    let temporary = path.with_extension("libretoon.tmp");
    let file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("fla"))
    {
        std::io::Write::write_all(&mut writer, &fla::encode(doc)?).map_err(|e| e.to_string())?;
    } else {
        serde_json::to_writer(&mut writer, doc).map_err(|e| e.to_string())?;
    }
    std::io::Write::flush(&mut writer).map_err(|e| e.to_string())?;
    drop(writer);
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}
impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_open(&ctx);
        self.drawing.pen.update(&ctx);
        if let Some(error) = self.recovery_writer.take_error() {
            self.status = format!("No se pudo recuperar sesión: {error}");
        }
        if !self.hub.home
            && !self.studio.preferences_open
            && self.studio.export.is_none()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
        {
            self.save_project(false);
        }
        if !self.hub.home
            && !self.studio.preferences_open
            && self.studio.export.is_none()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z))
        {
            if !self.canvas.undo_pen() {
                self.history(false);
            }
        }
        if !self.hub.home
            && !self.studio.preferences_open
            && self.studio.export.is_none()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y))
        {
            self.history(true);
        }
        if self.profile_transaction && !ctx.input(|i| i.pointer.primary_down()) {
            self.profile_transaction = false;
            self.recover();
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.finish_pending_pen();
            if self.hub.active.is_some() {
                self.recover();
            }
            self.persist_documents();
            if !self.hub.allow_quit && self.hub.tabs.iter().any(|t| t.record.dirty) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hub.quitting = true;
                self.hub.closing = self.hub.tabs.iter().position(|t| t.record.dirty);
            }
        }
        if self.playing {
            let elapsed = self.tick.elapsed().as_secs_f64();
            let steps = (elapsed * self.doc.fps as f64).floor() as usize;
            if steps > 0 {
                self.advance_animation(steps);
                self.tick += Duration::from_secs_f64(steps as f64 / self.doc.fps as f64);
            }
            let period = Duration::from_secs_f64(1. / self.doc.fps as f64);
            ctx.request_repaint_after(period.saturating_sub(self.tick.elapsed()));
        }
        crate::i18n::set_locale(self.studio.prefs.locale());
        themes::ensure(&ctx, self.studio.prefs.theme);
        i18n::ensure_fonts(&ctx, self.studio.prefs.locale());
        if !self.hub.home {
            self.clipboard_shortcuts(&ctx);
        }
        let was_home = self.hub.home;
        self.workspace(ui);
        if was_home != self.hub.home {
            ctx.request_repaint();
        }
        if !self.hub.home && !was_home {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(crate::themes::color(Color32::from_rgb(29, 30, 34)))
                        .inner_margin(0.),
                )
                .show(ui, |ui| {
                    self.canvas_ui(ui);
                });
        }
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "LibreToon",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_inner_size([1360., 860.])
                .with_min_inner_size([1050., 700.])
                .with_icon(design::window_icon()),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Editor::new(cc)))),
    )
}

#[cfg(test)]
mod history_tests {
    use super::*;
    #[test]
    fn undo_redo_restore_geometry_after_shared_snapshot_edits() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut editor = Editor::new(&cc);
        editor.doc = Document::default();
        let path =
            std::env::temp_dir().join(format!("libretoon-history-{}.json", std::process::id()));
        editor.recovery = path.clone();
        editor.checkpoint();
        editor.doc.layers[0].frames[0].push(Mark {
            points: vec![[10., 20.], [30., 40.]].into(),
            color: [0, 0, 0, 255],
            width: 4.,
            rectangle: false,
            ..Mark::default()
        });
        editor.checkpoint();
        editor.doc.layers[0].frames[0][0].points[0] = [50., 60.];
        editor.history(false);
        assert_eq!(editor.doc.layers[0].frames[0][0].points[0], [10., 20.]);
        editor.history(false);
        assert!(editor.doc.layers[0].frames[0].is_empty());
        editor.history(true);
        assert_eq!(editor.doc.layers[0].frames[0][0].points[0], [10., 20.]);
        editor.history(true);
        assert_eq!(editor.doc.layers[0].frames[0][0].points[0], [50., 60.]);
        drop(editor);
        let restored: Document = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(restored.layers[0].frames[0][0].points[0], [50., 60.]);
        std::fs::remove_file(path).unwrap();
    }
}
