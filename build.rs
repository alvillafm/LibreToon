use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/libretoon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    // Prefer the developer shell SDK, then locate an installed Windows SDK.
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    let mut candidates = Vec::new();
    if let Some(bin) = env::var_os("WindowsSdkVerBinPath") {
        candidates.push(PathBuf::from(bin).join(arch).join("rc.exe"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        let bin = PathBuf::from(program_files).join("Windows Kits/10/bin");
        if let Ok(entries) = bin.read_dir() {
            let mut versions: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            versions.sort();
            candidates.extend(
                versions
                    .into_iter()
                    .rev()
                    .map(|v| v.join(arch).join("rc.exe")),
            );
        }
    }
    let compiler = candidates
        .into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| "rc.exe".into());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let icon =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/libretoon.ico");
    let script = out.join("icon.rc");
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numeric = ["MAJOR", "MINOR", "PATCH"].map(|part| {
        env::var(format!("CARGO_PKG_VERSION_{part}"))
            .unwrap()
            .parse::<u16>()
            .unwrap()
    });
    std::fs::write(
        &script,
        format!(
            r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "CompanyName", "alvillafm\0"
   VALUE "FileDescription", "LibreToon 2D animation editor\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "InternalName", "libretoon\0"
   VALUE "OriginalFilename", "libretoon.exe\0"
   VALUE "ProductName", "LibreToon\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "LegalCopyright", "Copyright (c) 2026 alvillafm\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#,
            icon = icon.display().to_string().replace('\\', "/"),
            major = numeric[0],
            minor = numeric[1],
            patch = numeric[2],
        ),
    )
    .unwrap();
    let resource = out.join("icon.res");
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&script)
        .status()
        .expect("Windows SDK resource compiler is required for the app icon and version metadata");
    assert!(
        status.success(),
        "failed to compile the LibreToon icon resource"
    );
    println!("cargo:rustc-link-arg-bin=libretoon={}", resource.display());
}
