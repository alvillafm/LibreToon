use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/libretoon.ico");
    println!("cargo:rerun-if-changed=build.rs");
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
    std::fs::write(
        &script,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
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
        .expect("Windows SDK resource compiler is required to embed the app icon");
    assert!(
        status.success(),
        "failed to compile the LibreToon icon resource"
    );
    println!("cargo:rustc-link-arg-bin=libretoon={}", resource.display());
}
