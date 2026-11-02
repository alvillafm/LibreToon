//! Optional user-owned FFmpeg installation. No encoder is shipped with LibreToon.
//! Fetch immutable, SHA-256-pinned Imageio/PyPI archives; never install Python.
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Deserialize)]
pub struct Download {
    pub target: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Deserialize)]
struct Catalog {
    version: String,
    downloads: Vec<Download>,
}

#[derive(Default)]
pub struct Setup {
    pub prompt: bool,
    pub complete: bool,
    pub error: Option<String>,
    pub job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub progress: Arc<AtomicUsize>,
    pub phase: Arc<AtomicUsize>,
    pub cancel: Arc<AtomicBool>,
}
pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("LibreToon");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/LibreToon")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("LibreToon")
    }
}
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}
fn target() -> String {
    format!(
        "{}-{}",
        if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        },
        std::env::consts::ARCH
    )
}
fn catalog() -> Catalog {
    serde_json::from_str(include_str!("../assets/ffmpeg-downloads.json"))
        .expect("validated FFmpeg download catalog")
}
pub fn managed_path() -> PathBuf {
    data_dir()
        .join("tools")
        .join(format!("imageio-ffmpeg-{}-{}", catalog().version, target()))
        .join(binary_name())
}
pub fn available(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() && Path::new(configured).is_file() {
        return Some(configured.into());
    }
    let managed = managed_path();
    if managed.is_file() {
        return Some(managed);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join(binary_name()))
            .find(|p| p.is_file())
    })
}
pub fn in_folder(folder: &Path) -> Option<PathBuf> {
    [
        folder.join(binary_name()),
        folder.join("bin").join(binary_name()),
    ]
    .into_iter()
    .find(|p| p.is_file())
}
fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    let _ = command;
}
fn system_program(name: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()))
            .join("System32")
            .join(format!("{name}.exe"))
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/usr/bin").join(name)
    } else {
        name.into()
    }
}
fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Download cancelled".into())
    } else {
        Ok(())
    }
}
fn wait(
    mut command: Command,
    cancel: &AtomicBool,
    timeout: Duration,
    mut tick: impl FnMut(),
) -> Result<(), String> {
    hidden(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start download/verification tool: {e}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("Download or FFmpeg verification failed ({status})"))
                };
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
            _ => {}
        }
        if cancel.load(Ordering::Relaxed) || start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.load(Ordering::Relaxed) {
                "Download cancelled"
            } else {
                "Download or verification timed out"
            }
            .into());
        }
        tick();
        std::thread::sleep(Duration::from_millis(50));
    }
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        // Only this uniquely-created transaction directory, never the tools/user folder.
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn hash_token(text: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| line.split_whitespace().next())
        .find(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}
fn verify_hash(
    archive: &Path,
    expected: &str,
    scratch: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let output = scratch.join("sha256.txt");
    let mut command = if cfg!(windows) {
        let mut c = Command::new(system_program("certutil"));
        c.args(["-hashfile"]).arg(archive).arg("SHA256");
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new(system_program("shasum"));
        c.args(["-a", "256"]).arg(archive);
        c
    } else {
        let mut c = Command::new("sha256sum");
        c.arg(archive);
        c
    };
    command
        .stdout(File::create(&output).map_err(|e| e.to_string())?)
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    wait(command, cancel, Duration::from_secs(30), || {})?;
    let bytes = fs::read(output).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    if hash_token(&text).as_deref() != Some(expected) {
        return Err("FFmpeg download checksum mismatch; nothing was installed".into());
    }
    Ok(())
}
fn unpack(archive: &[u8], destination: &Path) -> Result<PathBuf, String> {
    let zip = crate::fla_zip::Archive::new(archive)?;
    let binaries: Vec<_> = zip
        .names()
        .filter(|name| {
            name.starts_with(b"imageio_ffmpeg/binaries/ffmpeg-")
                && !name[b"imageio_ffmpeg/binaries/".len()..].contains(&b'/')
        })
        .collect();
    if binaries.len() != 1 {
        return Err("Unexpected FFmpeg archive contents".into());
    }
    let bytes = zip
        .entry_limited(binaries[0], 128 * 1024 * 1024)?
        .ok_or("FFmpeg executable missing")?;
    let valid = if cfg!(windows) {
        bytes.starts_with(b"MZ")
    } else if cfg!(target_os = "macos") {
        [
            [0xcf, 0xfa, 0xed, 0xfe],
            [0xce, 0xfa, 0xed, 0xfe],
            [0xca, 0xfe, 0xba, 0xbe],
            [0xbe, 0xba, 0xfe, 0xca],
        ]
        .iter()
        .any(|magic| bytes.starts_with(magic))
    } else {
        bytes.starts_with(b"\x7fELF")
    };
    if !valid {
        return Err("FFmpeg executable does not match this platform".into());
    }
    let executable = destination.join(binary_name());
    fs::write(&executable, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    fs::create_dir(destination.join("licenses")).map_err(|e| e.to_string())?;
    for (i, name) in zip
        .names()
        .filter(|name| {
            let text = String::from_utf8_lossy(name);
            text.contains("LICENSE") || text.contains("COPYING") || text.ends_with("/METADATA")
        })
        .enumerate()
    {
        let bytes = zip
            .entry_limited(name, 4 * 1024 * 1024)?
            .ok_or("Missing archive notice")?;
        fs::write(
            destination
                .join("licenses")
                .join(format!("upstream-notice-{i}.txt")),
            bytes,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(executable)
}
fn capture(
    executable: &Path,
    option: &str,
    scratch: &Path,
    cancel: &AtomicBool,
) -> Result<String, String> {
    let path = scratch.join("validation.txt");
    let mut command = Command::new(executable);
    command
        .args(["-hide_banner", option])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(File::create(&path).map_err(|e| e.to_string())?);
    wait(command, cancel, Duration::from_secs(20), || {})?;
    let mut text = String::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Ok(text)
}
fn validate(executable: &Path, scratch: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let version = capture(executable, "-version", scratch, cancel)?;
    if !version.starts_with("ffmpeg version") {
        return Err("The selected executable is not FFmpeg".into());
    }
    for (option, required) in [
        (
            "-encoders",
            &["libx264", "libx264rgb", "mpeg4", "prores_ks", "gif"][..],
        ),
        ("-muxers", &["mp4", "matroska", "avi", "mov", "gif"][..]),
        (
            "-filters",
            &[
                "pad",
                "format",
                "scale",
                "split",
                "palettegen",
                "paletteuse",
            ][..],
        ),
    ] {
        let text = capture(executable, option, scratch, cancel)?;
        let tokens: Vec<_> = text
            .split(|c: char| c.is_whitespace() || c == ',')
            .collect();
        if required.iter().any(|r| !tokens.contains(r)) {
            return Err(format!(
                "FFmpeg is missing required export capabilities ({option})"
            ));
        }
    }
    Ok(version)
}
fn install(download: &Download, destination: &Path, state: &Setup) -> Result<PathBuf, String> {
    cancelled(&state.cancel)?;
    if !download.url.starts_with("https://files.pythonhosted.org/")
        || download.sha256.len() != 64
        || download.bytes > 64 * 1024 * 1024
    {
        return Err("Invalid FFmpeg download catalog".into());
    }
    let parent = destination.parent().ok_or("Invalid FFmpeg data folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let transaction = parent.join(format!(".download-{}-{nonce}", std::process::id()));
    fs::create_dir(&transaction).map_err(|e| e.to_string())?;
    let scratch = Scratch(transaction);
    let archive = scratch.0.join("download.whl");
    let mut curl = Command::new(system_program("curl"));
    curl.args([
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--tlsv1.2",
        "--connect-timeout",
        "20",
        "--max-time",
        "600",
        "--retry",
        "2",
        "--output",
    ])
    .arg(&archive)
    .arg(&download.url)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    wait(curl, &state.cancel, Duration::from_secs(650), || {
        let bytes = fs::metadata(&archive).map_or(0, |m| m.len());
        state.progress.store(
            ((bytes.saturating_mul(800) / download.bytes.max(1)).min(800)) as usize,
            Ordering::Relaxed,
        );
    })?;
    if fs::metadata(&archive).map_err(|e| e.to_string())?.len() != download.bytes {
        return Err("FFmpeg download size mismatch".into());
    }
    state.phase.store(1, Ordering::Relaxed);
    state.progress.store(820, Ordering::Relaxed);
    verify_hash(&archive, &download.sha256, &scratch.0, &state.cancel)?;
    cancelled(&state.cancel)?;
    state.phase.store(2, Ordering::Relaxed);
    let staged = scratch.0.join("installed");
    fs::create_dir(&staged).map_err(|e| e.to_string())?;
    let executable = unpack(&fs::read(&archive).map_err(|e| e.to_string())?, &staged)?;
    state.phase.store(3, Ordering::Relaxed);
    state.progress.store(950, Ordering::Relaxed);
    let version = validate(&executable, &scratch.0, &state.cancel)?;
    fs::write(staged.join("FFMPEG-VERSION.txt"), version).map_err(|e| e.to_string())?;
    let license = capture(&executable, "-L", &scratch.0, &state.cancel)?;
    fs::write(staged.join("FFMPEG-LICENSE.txt"), license).map_err(|e| e.to_string())?;
    fs::write(staged.join("DOWNLOAD-ORIGIN.txt"),format!("Downloaded directly by the user from Imageio's PyPI distribution.\nPackage: {}\nURL: {}\nSHA-256: {}\nUpstream: https://github.com/imageio/imageio-ffmpeg\nFFmpeg: https://ffmpeg.org/\nThis external program is not part of LibreToon's MIT code or installer.\n",download.filename,download.url,download.sha256)).map_err(|e|e.to_string())?;
    cancelled(&state.cancel)?;
    // Keep every existing user installation, including a potentially in-use one.
    let promoted = if destination.exists() {
        parent.join(format!(
            "{}-{nonce}",
            destination
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ))
    } else {
        destination.to_path_buf()
    };
    fs::rename(&staged, &promoted).map_err(|e| e.to_string())?;
    state.progress.store(990, Ordering::Relaxed);
    Ok(promoted.join(binary_name()))
}
impl Setup {
    pub fn start(&mut self) {
        if self.job.is_some() {
            return;
        }
        let Some(download) = catalog()
            .downloads
            .into_iter()
            .find(|d| d.target == target())
        else {
            self.error = Some(
                "Automatic FFmpeg download is unavailable on this platform. Select it manually."
                    .into(),
            );
            return;
        };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.complete = false;
        self.error = None;
        self.progress.store(0, Ordering::Relaxed);
        self.phase.store(0, Ordering::Relaxed);
        self.cancel.store(false, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        let mut worker = Self::default();
        worker.progress = self.progress.clone();
        worker.phase = self.phase.clone();
        worker.cancel = self.cancel.clone();
        let destination = managed_path().parent().unwrap().to_path_buf();
        self.thread = Some(std::thread::spawn(move || {
            let result = install(&download, &destination, &worker);
            let _ = tx.send(result);
        }));
    }
}

impl Drop for Setup {
    fn drop(&mut self) {
        if self.job.is_some() {
            self.cancel.store(true, Ordering::Relaxed);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_only_contains_pinned_https_downloads_for_supported_targets() {
        let c = catalog();
        assert_eq!(c.downloads.len(), 4);
        for d in c.downloads {
            assert!(d.url.starts_with("https://files.pythonhosted.org/"));
            assert_eq!(d.sha256.len(), 64);
            assert!(d.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!((1_000_000..64 * 1024 * 1024).contains(&d.bytes));
        }
    }
    #[test]
    fn digest_parser_handles_localized_certutil_and_unix_output() {
        let sha = "7a22b45f60321d4172bdf2bc61f716bce4c81b85727bd59c7c7e371055a6a440";
        assert_eq!(
            hash_token(&format!(
                "SHA256 archivo:\n{}\nCorrecto",
                sha.to_uppercase()
            )),
            Some(sha.into())
        );
        assert_eq!(
            hash_token(&format!("{sha}  /tmp/archive")),
            Some(sha.into())
        );
        assert_eq!(hash_token("download failed"), None);
    }
    #[test]
    fn wheel_selection_rejects_missing_or_ambiguous_binaries() {
        let temp = std::env::temp_dir();
        assert!(unpack(&crate::fla_zip::pack(&[("unexpected", vec![])]), &temp).is_err());
        let zip = crate::fla_zip::pack(&[
            ("imageio_ffmpeg/binaries/ffmpeg-a", vec![]),
            ("imageio_ffmpeg/binaries/ffmpeg-b", vec![]),
        ]);
        assert!(unpack(&zip, &temp).is_err());
    }
    #[test]
    fn cancellation_never_starts_a_download_or_changes_an_installation() {
        let state = Setup::default();
        state.cancel.store(true, Ordering::Relaxed);
        let download = catalog().downloads.into_iter().next().unwrap();
        let destination = std::env::temp_dir().join(format!(
            "ffmpeg-cancelled-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert_eq!(
            install(&download, &destination, &state).unwrap_err(),
            "Download cancelled"
        );
        assert!(!destination.exists());
    }
    #[test]
    #[ignore = "manual network install test; uses isolated LOCALAPPDATA"]
    fn download_configure_ffmpeg_and_verify_export_capabilities() {
        let download = catalog()
            .downloads
            .into_iter()
            .find(|d| d.target == target())
            .unwrap();
        let destination = managed_path().parent().unwrap().to_path_buf();
        let executable = if destination.join(binary_name()).is_file() {
            destination.join(binary_name())
        } else {
            install(&download, &destination, &Setup::default()).unwrap()
        };
        assert!(executable.is_file());
        println!("Verified optional FFmpeg: {}", executable.display());
    }
}
