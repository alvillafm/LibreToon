use crate::{
    ffmpeg_setup::{system_program, verify_hash},
    i18n::tr,
};
use eframe::egui;
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const REPO: &str = "alvillafm/LibreToon";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
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
            "https://api.github.com/repos/{REPO}/releases?per_page=100"
        ))
        .output()
        .map_err(|e| e.to_string())?;
    if output.stdout.ends_with(b"404") {
        return Err(tr("No public release was found on GitHub. Unable to confirm whether LibreToon is up to date.").into());
    }
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    parse_release(&output.stdout[..output.stdout.len().saturating_sub(3)])
}

fn parse_release(json: &[u8]) -> Result<Option<Release>, String> {
    // Public LibreToon versions are currently marked as GitHub pre-releases.
    // /releases/latest excludes them. Compare numeric tags from the public list,
    // never drafts, and do not assume GitHub's publication order is version order.
    let releases: Vec<Release> = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let mut public = false;
    let mut latest = None;
    for release in releases.into_iter().filter(|r| !r.draft) {
        public = true;
        if let Some(number) = version(&release.tag_name)
            && latest.as_ref().is_none_or(|(best, _)| number > *best)
        {
            latest = Some((number, release));
        }
    }
    let Some((next, mut release)) = latest else {
        return Err(tr(if public { "Unrecognized release version:" } else { "No public release was found on GitHub. Unable to confirm whether LibreToon is up to date." }).into());
    };
    let current = version(env!("CARGO_PKG_VERSION")).ok_or("Invalid installed version")?;
    if next <= current {
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

fn download(asset: Asset, folder: PathBuf, progress: Arc<AtomicUsize>) -> Result<PathBuf, String> {
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
    let mut child = curl()
        .args(["--max-time", "600", "--max-filesize"])
        .arg(asset.size.to_string())
        .arg("--output")
        .arg(&staged)
        .arg(&asset.browser_download_url)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        let bytes = staged.metadata().map(|m| m.len()).unwrap_or(0);
        progress.store(
            (bytes.saturating_mul(90) / asset.size.max(1)).min(90) as usize,
            Ordering::Relaxed,
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    progress.store(92, Ordering::Relaxed);
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

enum Outcome {
    Release(Option<Release>),
    Prepared(crate::update_install::Plan),
}

#[derive(Default)]
pub(crate) struct Updater {
    started: bool,
    visible: bool,
    manual: bool,
    release: Option<Release>,
    status: Option<String>,
    pub(crate) prepared: Option<crate::update_install::Plan>,
    pub(crate) install_pending: bool,
    progress: Arc<AtomicUsize>,
    updating: bool,
    defer_install: bool,
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

    pub(crate) fn install_error(&mut self, error: String) {
        self.status = Some(error);
        self.visible = true;
        self.install_pending = false;
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
        self.prepared = None;
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
                Ok(Outcome::Prepared(plan)) => {
                    self.prepared = Some(plan);
                    self.install_pending = !self.defer_install;
                    self.updating = false;
                }
                Err(error) if self.manual || self.release.is_some() => {
                    self.status = Some(error);
                    self.updating = false;
                }
                _ => {}
            }
            self.manual = false;
        }
        if !self.visible {
            return;
        }
        if self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
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
                ui.label(format!(
                    "{} {}",
                    tr("Current version:"),
                    env!("CARGO_PKG_VERSION")
                ));
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
                    if self.updating {
                        ui.label(tr("Downloading and verifying update…"));
                        ui.add(
                            egui::ProgressBar::new(
                                self.progress.load(Ordering::Relaxed) as f32 / 100.,
                            )
                            .show_percentage(),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(tr("Buscando o descargando la actualización…"));
                        });
                    }
                } else if self.prepared.is_some() {
                    ui.label(tr(
                        "The update is ready. LibreToon will close, update this copy and reopen.",
                    ));
                    if crate::design::button(
                        ui,
                        crate::design::Icon::Redo,
                        "Restart and install",
                        "Restart and install",
                        false,
                        true,
                    )
                    .clicked()
                    {
                        self.install_pending = true;
                    }
                } else if let Some(release) = &self.release {
                    match crate::update_install::Host::detect() {
                        Err(error) => {
                            ui.label(error);
                        }
                        Ok(host) => {
                            let asset = release.assets.iter().find(|a| host.accepts(&a.name));
                            if let Some(asset) = asset {
                                if crate::design::button(
                                    ui,
                                    crate::design::Icon::Export,
                                    "Update now",
                                    "Update now",
                                    false,
                                    asset.checksum().is_some(),
                                )
                                .on_disabled_hover_text(tr("Falta la suma SHA-256 del paquete."))
                                .clicked()
                                {
                                    let asset = asset.clone();
                                    let hash = asset.checksum().unwrap().to_owned();
                                    let locale = crate::i18n::locale();
                                    let progress = self.progress.clone();
                                    progress.store(0, Ordering::Relaxed);
                                    self.status = None;
                                    self.defer_install = false;
                                    self.updating = true;
                                    self.run(ctx, move || {
                                        let nonce = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .map_err(|e| e.to_string())?
                                            .as_nanos();
                                        let folder = crate::ffmpeg_setup::data_dir()
                                            .join("updates")
                                            .join(format!("{}-{nonce}", std::process::id()));
                                        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                                        let package = download(asset, folder, progress.clone())?;
                                        let plan = crate::update_install::prepare(
                                            host, package, hash, locale,
                                        )?;
                                        progress.store(100, Ordering::Relaxed);
                                        Ok(Outcome::Prepared(plan))
                                    });
                                }
                            } else {
                                ui.label(tr("Esta versión no tiene un paquete compatible."));
                            }
                        }
                    }
                }
                later = ui.button(tr("Más tarde")).clicked();
            });
        if !open || later {
            self.defer_install = true;
            self.install_pending = false;
        }
        self.visible = open && !later;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_tags_cannot_report_up_to_date() {
        let json = br#"[{"tag_name":"Release 0.2.3","body":null,"assets":[]}]"#;
        assert!(parse_release(json).is_err());
    }

    #[test]
    fn valid_newer_and_current_releases_are_distinguished() {
        let current = format!(
            r#"[{{"tag_name":"v{}","body":null,"assets":[]}}]"#,
            env!("CARGO_PKG_VERSION")
        );
        assert!(parse_release(current.as_bytes()).unwrap().is_none());
        assert!(
            parse_release(br#"[{"tag_name":"v99.0.0","body":null,"assets":[]}]"#)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn public_prereleases_are_found_and_drafts_are_excluded() {
        let json=br#"[{"tag_name":"v98.0.0","prerelease":true,"assets":[]},{"tag_name":"v99.0.0","prerelease":true,"assets":[]},{"tag_name":"v100.0.0","draft":true,"assets":[]}]"#;
        assert_eq!(parse_release(json).unwrap().unwrap().tag_name, "v99.0.0");
        assert!(parse_release(b"[]").is_err());
    }

    #[test]
    fn compares_numeric_stable_versions_and_rejects_prerelease_tags() {
        assert!(version("v0.10.0") > version("0.9.99"));
        assert!(version("v1.0.0-beta.1").is_none());
        assert!(version("release/latest").is_none());
        assert!(version("1.0.0.1").is_none());
    }
}
