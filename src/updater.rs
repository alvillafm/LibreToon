use crate::{
    ffmpeg_setup::{system_program, verify_hash},
    i18n::tr,
};
use eframe::egui;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{atomic::AtomicBool, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

const REPO: &str = "alvillafm/LibreToon";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}

impl Asset {
    fn checksum(&self) -> Option<&str> {
        self.digest
            .as_deref()?
            .strip_prefix("sha256:")
            .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    }

    fn compatible(&self) -> bool {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            _ => return false,
        };
        let (platform, suffixes): (_, &[&str]) = match std::env::consts::OS {
            "windows" => ("Windows", &[".exe", ".zip"]),
            "macos" => ("macOS", &[".dmg", ".zip"]),
            "linux" => ("Linux", &[".AppImage", ".deb"]),
            _ => return false,
        };
        self.name.starts_with("LibreToon-")
            && self.name.contains(&format!("-{platform}-{arch}"))
            && suffixes.iter().any(|suffix| self.name.ends_with(suffix))
            && self
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    }
}

fn version(tag: &str) -> Option<[u32; 3]> {
    let mut parts = tag.trim_start_matches('v').split('.');
    let numbers = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(numbers)
}

fn curl() -> Command {
    let mut command = Command::new(system_program("curl"));
    command.args([
        "--disable",
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "10",
        "--user-agent",
        "LibreToon",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn latest() -> Result<Option<Release>, String> {
    let output = curl()
        .args([
            "--max-time",
            "20",
            "--max-filesize",
            "4194304",
            "--write-out",
            "%{http_code}",
            "--header",
            "Accept: application/vnd.github+json",
        ])
        .arg(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .output()
        .map_err(|e| e.to_string())?;
    if output.stdout.ends_with(b"404") {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let mut release: Release =
        serde_json::from_slice(&output.stdout[..output.stdout.len().saturating_sub(3)])
            .map_err(|e| e.to_string())?;
    let current = version(env!("CARGO_PKG_VERSION")).unwrap_or_default();
    if !version(&release.tag_name).is_some_and(|next| next > current) {
        return Ok(None);
    }
    release.assets.retain(Asset::compatible);
    Ok(Some(release))
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn download(asset: Asset, folder: PathBuf) -> Result<PathBuf, String> {
    let hash = asset
        .checksum()
        .ok_or("Missing SHA-256 for this release file")?
        .to_ascii_lowercase();
    let prefix = format!("https://github.com/{REPO}/releases/download/");
    if !asset.compatible() || !asset.browser_download_url.starts_with(&prefix) {
        return Err("Invalid release asset".into());
    }
    let destination = folder.join(&asset.name);
    if destination.exists() {
        return Err("This file already exists in the selected folder".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let scratch = folder.join(format!(".libretoon-update-{}-{nonce}", std::process::id()));
    fs::create_dir(&scratch).map_err(|e| e.to_string())?;
    let scratch = Staging(scratch);
    let staged = scratch.0.join(&asset.name);
    let output = curl()
        .args(["--max-time", "600", "--max-filesize"])
        .arg(asset.size.to_string())
        .arg("--output")
        .arg(&staged)
        .arg(&asset.browser_download_url)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    if staged.metadata().map_err(|e| e.to_string())?.len() != asset.size {
        return Err("Release file size mismatch".into());
    }
    verify_hash(&staged, &hash, &scratch.0, &AtomicBool::new(false))?;
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|e| e.to_string())?;
    let copied = fs::File::open(&staged)
        .and_then(|mut input| std::io::copy(&mut input, &mut target))
        .and_then(|_| target.sync_all());
    drop(target);
    if let Err(error) = copied {
        let _ = fs::remove_file(&destination);
        return Err(error.to_string());
    }
    #[cfg(unix)]
    if asset.name.ends_with(".AppImage") {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(&destination, fs::Permissions::from_mode(0o755)) {
            let _ = fs::remove_file(&destination);
            return Err(error.to_string());
        }
    }
    Ok(destination)
}

fn open_package(path: &Path) -> Result<(), String> {
    let extension = path.extension().and_then(|value| value.to_str());
    let mut command = if cfg!(windows) && extension == Some("exe") {
        Command::new(path)
    } else if cfg!(windows) {
        let mut command = Command::new("explorer.exe");
        command.arg(path.parent().unwrap_or(path));
        command
    } else {
        let mut command = Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        });
        command.arg(if matches!(extension, Some("zip" | "AppImage")) {
            path.parent().unwrap_or(path)
        } else {
            path
        });
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

enum Outcome {
    Release(Option<Release>),
    Downloaded(PathBuf),
}

#[derive(Default)]
pub(crate) struct Updater {
    started: bool,
    visible: bool,
    manual: bool,
    release: Option<Release>,
    status: Option<String>,
    downloaded: Option<PathBuf>,
    job: Option<mpsc::Receiver<Result<Outcome, String>>>,
}

impl Updater {
    fn run(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce() -> Result<Outcome, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.job = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
    }

    pub(crate) fn check(&mut self, ctx: &egui::Context, manual: bool) {
        self.started = true;
        self.manual |= manual;
        self.visible |= manual;
        if self.job.is_some() {
            return;
        }
        self.release = None;
        self.status = None;
        self.downloaded = None;
        self.run(ctx, || latest().map(Outcome::Release));
    }

    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        if !self.started && !cfg!(test) {
            self.check(ctx, false);
        }
        if let Some(result) = self.job.as_ref().and_then(|job| match job.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Update worker stopped".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        }) {
            self.job = None;
            match result {
                Ok(Outcome::Release(Some(release))) => {
                    self.release = Some(release);
                    self.visible = true;
                }
                Ok(Outcome::Release(None)) if self.manual => {
                    self.status = Some(tr("Ya tienes la versión más reciente.").into());
                }
                Ok(Outcome::Downloaded(path)) => self.downloaded = Some(path),
                Err(error) if self.manual || self.release.is_some() => self.status = Some(error),
                _ => {}
            }
            self.manual = false;
        }
        if !self.visible {
            return;
        }
        let mut open = self.visible;
        let mut later = false;
        egui::Window::new(tr("Actualizaciones de LibreToon"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(470.);
                if let Some(release) = &self.release {
                    ui.heading(format!("{} {}", tr("Nueva versión"), release.tag_name));
                    if let Some(notes) = release.body.as_deref().filter(|notes| !notes.is_empty()) {
                        egui::ScrollArea::vertical()
                            .max_height(130.)
                            .show(ui, |ui| {
                                ui.label(notes);
                            });
                    }
                    ui.label(tr(
                        "Puedes actualizar desde Ayuda > Buscar actualizaciones.",
                    ));
                }
                if let Some(status) = &self.status {
                    ui.label(status);
                }
                if self.job.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Buscando o descargando la actualización…"));
                    });
                } else if let Some(path) = &self.downloaded {
                    ui.label(tr(
                        "Descarga verificada. Guarda tu trabajo antes de instalar.",
                    ));
                    ui.label(path.display().to_string());
                    if ui.button(tr("Abrir descarga")).clicked() {
                        self.status = open_package(path).err();
                    }
                } else if let Some(release) = &self.release {
                    if release.assets.is_empty() {
                        ui.label(tr("Esta versión no tiene un paquete compatible."));
                    }
                    for asset in &release.assets {
                        if ui
                            .add_enabled(
                                asset.checksum().is_some(),
                                egui::Button::new(format!("{} {}", tr("Descargar"), asset.name)),
                            )
                            .on_disabled_hover_text(tr("Falta la suma SHA-256 del paquete."))
                            .clicked()
                            && let Some(folder) = rfd::FileDialog::new().pick_folder()
                        {
                            let asset = asset.clone();
                            self.status = None;
                            self.run(ctx, move || {
                                download(asset, folder).map(Outcome::Downloaded)
                            });
                            break;
                        }
                    }
                }
                later = ui.button(tr("Más tarde")).clicked();
            });
        self.visible = open && !later;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numeric_stable_versions_and_rejects_prerelease_tags() {
        assert!(version("v0.10.0") > version("0.9.99"));
        assert!(version("v1.0.0-beta.1").is_none());
        assert!(version("release/latest").is_none());
        assert!(version("1.0.0.1").is_none());
    }
}
