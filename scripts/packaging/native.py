#!/usr/bin/env python3
"""Package already-built Linux/macOS payloads; no application compilation here."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
def digest(path):
    with Path(path).open("rb") as f:
        return hashlib.file_digest(f,"sha256").hexdigest()


def write(path,value):
    Path(path).write_text(json.dumps(value,indent=2)+"\n")


def download(url,path,sha):
    path=Path(path)
    if not path.exists():
        with urllib.request.urlopen(url,timeout=120) as response,path.open("wb") as target:
            shutil.copyfileobj(response,target)
    if digest(path)!=sha:
        raise RuntimeError("Packaging tool checksum mismatch: "+str(path))


def check_system_imports(binary):
    if sys.platform=="darwin":
        lines=subprocess.check_output(["otool","-L",str(binary)],text=True).splitlines()[1:]
        imports=[line.strip().split(" (",1)[0] for line in lines]
        if any(not p.startswith(("/usr/lib/","/System/Library/")) for p in imports):
            raise RuntimeError("Non-system library in Mac app: "+repr(imports))
    else:
        text=subprocess.check_output(["ldd",str(binary)],text=True)
        if "not found" in text:
            raise RuntimeError(text)
        imports=re.findall(r"^\s*(\S+)\s+=>",text,re.M)
        if any(not re.fullmatch(r"(libc|libm|libdl|libpthread|librt|libgcc_s)\.so[.0-9]*",p) for p in imports):
            raise RuntimeError("Unpackaged non-system library: "+repr(imports))


VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9.-]+)?", VERSION):
    raise RuntimeError("Unsafe package version")


def run(args, **kwargs):
    subprocess.run(list(map(str, args)), check=True, **kwargs)


def common(payload):
    payload.mkdir(parents=True)
    shutil.copy2(ROOT / "target/release/libretoon", payload / "libretoon")
    shutil.copytree(ROOT / "assets", payload / "assets",
                    ignore=lambda path, names: {"tools", "NotoSansCJK-Regular.ttc"} & set(names))
    shutil.copy2(ROOT / "LICENSE", payload / "LICENSE")
    check_system_imports(payload / "libretoon")
    forbidden={"ffmpeg.exe","ffprobe.exe","ffmpeg","ffprobe","ffmpeg-complete-source.zip"}
    if any(p.is_file() and p.name in forbidden for p in payload.rglob("*")):
        raise RuntimeError("FFmpeg entered the lightweight payload")
    (payload / "BUILD-INFO.txt").write_text(
        "LibreToon " + VERSION + "\nCommit: " + os.environ.get("GITHUB_SHA", "local") +
        "\nMIT application. FFmpeg is not bundled; video/GIF export offers a separate optional download.\n")


def mac(dest):
    work = ROOT / "work/packages/macos"
    work.mkdir(parents=True, exist_ok=False)
    app = work / "LibreToon.app"
    contents = app / "Contents"
    resources = contents / "Resources"
    common(resources)
    # Apple reserves MacOS for executable code; data belongs in Resources.
    payload = contents / "MacOS"
    payload.mkdir()
    shutil.move(resources / "libretoon", payload / "libretoon")
    iconset = work / "LibreToon.iconset"
    iconset.mkdir()
    for size in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            name = "icon_" + str(size) + "x" + str(size) + ("@2x" if scale == 2 else "") + ".png"
            run(["sips", "-z", str(size * scale), str(size * scale),
                 ROOT / "assets/libretoon-icon.png", "--out", iconset / name], stdout=subprocess.DEVNULL)
    run(["iconutil", "-c", "icns", iconset, "-o", resources / "LibreToon.icns"])
    with (contents / "Info.plist").open("wb") as f:
        plistlib.dump({
            "CFBundleName": "LibreToon", "CFBundleDisplayName": "LibreToon",
            "CFBundleIdentifier": "org.libretoon.app", "CFBundleExecutable": "libretoon",
            "CFBundlePackageType": "APPL", "CFBundleShortVersionString": VERSION,
            "CFBundleVersion": os.environ.get("GITHUB_RUN_NUMBER", "1"),
            "CFBundleIconFile": "LibreToon.icns", "NSHighResolutionCapable": True,
            "LSMinimumSystemVersion": "14.0",
        }, f)
    # Ad-hoc only. This is not Developer ID signing or Apple notarization.
    run(["codesign", "--force", "--sign", "-", "--identifier", "org.libretoon.app", app])
    run(["codesign", "--verify", "--strict", app])
    arch = "arm64" if os.uname().machine == "arm64" else "x64"
    stem = "LibreToon-" + VERSION + "-macOS-" + arch
    run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, dest / (stem + "-Portable.zip")])
    dmg = work / "dmg"
    dmg.mkdir()
    shutil.copytree(app, dmg / app.name, symlinks=True)
    (dmg / "Applications").symlink_to("/Applications", target_is_directory=True)
    (dmg / "README.txt").write_text("Drag LibreToon.app into Applications. Resources/notices are included. Video/GIF export offers an optional separate FFmpeg download.\n"
                                   "This Actions build is ad-hoc signed, not Apple-notarized.\n")
    run(["hdiutil", "create", "-volname", "LibreToon", "-srcfolder", dmg,
         "-fs", "HFS+", "-format", "UDZO", dest / (stem + ".dmg")])


def api(path):
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "LibreToon-packaging"}
    if os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    with urllib.request.urlopen(urllib.request.Request("https://api.github.com/repos/" + path, headers=headers), timeout=120) as r:
        return json.load(r)


def asset(release, name, destination):
    item = next(x for x in release["assets"] if x["name"] == name)
    checksum = item.get("digest", "")
    if not checksum.startswith("sha256:"):
        raise RuntimeError("Official packaging tool has no published SHA-256: " + name)
    download(item["browser_download_url"], destination, checksum.split(":", 1)[1])
    destination.chmod(0o755)
    return item


def appimage_runtime(work, appdir):
    tool_release = api("AppImage/appimagetool/releases/tags/1.9.1")
    tool = work / "appimagetool.AppImage"
    tool_asset = asset(tool_release, "appimagetool-x86_64.AppImage", tool)
    release = api("AppImage/type2-runtime/releases/tags/continuous")
    runtime = work / "runtime-x86_64"
    runtime_asset = asset(release, "runtime-x86_64", runtime)
    obj = api("AppImage/type2-runtime/git/ref/tags/continuous")["object"]
    while obj["type"] == "tag":
        obj = api("AppImage/type2-runtime/git/tags/" + obj["sha"])["object"]
    revision = obj["sha"]
    version = subprocess.check_output([str(runtime), "--appimage-version"], stderr=subprocess.STDOUT, text=True)
    if revision[:7] not in version:
        raise RuntimeError("AppImage runtime binary/source revision cannot be matched")
    notices = appdir / "usr/lib/libretoon/corresponding-source/appimage-runtime"
    notices.mkdir(parents=True)
    archive = notices / ("type2-runtime-" + revision + ".tar.gz")
    url = "https://codeload.github.com/AppImage/type2-runtime/tar.gz/" + revision
    with urllib.request.urlopen(url, timeout=120) as r, archive.open("wb") as f:
        shutil.copyfileobj(r, f)
    with tarfile.open(archive) as t:
        license_member = next(m for m in t.getmembers() if m.name.endswith("/LICENSE"))
        (notices / "LICENSE").write_bytes(t.extractfile(license_member).read())
    # Capture the upstream runtime help, which includes component license references.
    license_text = subprocess.check_output([str(runtime), "--appimage-help"], stderr=subprocess.STDOUT)
    (notices / "RUNTIME-LICENSES.txt").write_bytes(license_text)
    write(notices / "BUILD-IDENTITY.json", {
        "runtime": runtime_asset, "runtimeRevision": revision, "runtimeVersion": version,
        "sourceUrl": url, "sourceSha256": digest(archive),
        "appimagetool": tool_asset, "appimagetoolVersion": "1.9.1",
    })
    return tool, runtime


def linux(dest):
    work = ROOT / "work/packages/linux"
    work.mkdir(parents=True, exist_ok=False)
    appdir = work / "LibreToon.AppDir"
    payload = appdir / "usr/lib/libretoon"
    common(payload)
    desktop = "[Desktop Entry]\nType=Application\nName=LibreToon\nComment=2D animation editor\nExec=libretoon %F\nIcon=libretoon\nTerminal=false\nCategories=Graphics;2DGraphics;\n"
    (appdir / "usr/bin").mkdir(parents=True)
    (appdir / "usr/bin/libretoon").symlink_to("../lib/libretoon/libretoon")
    (appdir / "libretoon.desktop").write_text(desktop)
    shutil.copy2(ROOT / "assets/libretoon-icon.png", appdir / "libretoon.png")
    apprun = appdir / "AppRun"
    apprun.write_text('#!/bin/sh\nset -eu\nHERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"\nexec "$HERE/usr/lib/libretoon/libretoon" "$@"\n')
    apprun.chmod(0o755)
    # No bundled graphics drivers/glibc or unreviewed shared libraries.
    tool, runtime = appimage_runtime(work, appdir)
    stem = "LibreToon-" + VERSION + "-Linux-x64"
    env = dict(os.environ, ARCH="x86_64", APPIMAGE_EXTRACT_AND_RUN="1", VERSION=VERSION)
    run([tool, "--runtime-file", runtime, appdir, dest / (stem + ".AppImage")], env=env)
    (dest / (stem + ".AppImage")).chmod(0o755)
    deb = work / "deb"
    install = deb / "opt/libretoon"
    shutil.copytree(payload, install)
    control = deb / "DEBIAN"
    control.mkdir()
    (control / "control").write_text(
        "Package: libretoon\nVersion: " + VERSION + "\nSection: graphics\nPriority: optional\n"
        "Architecture: amd64\nMaintainer: LibreToon project <noreply@github.com>\n"
        "Depends: curl, coreutils, libc6 (>= 2.35), libgcc-s1, libgl1, libegl1, libx11-6, libxcursor1, libxi6, libxrandr2, libxkbcommon0, libwayland-client0\n"
        "Recommends: xdg-desktop-portal, xdg-desktop-portal-gtk\n"
        "Description: LibreToon native 2D animation editor\n Includes application resources; FFmpeg is downloaded separately on request.\n")
    bindir = deb / "usr/bin"
    bindir.mkdir(parents=True)
    (bindir / "libretoon").symlink_to("/opt/libretoon/libretoon")
    desktops = deb / "usr/share/applications"
    desktops.mkdir(parents=True)
    (desktops / "libretoon.desktop").write_text(desktop)
    icons = deb / "usr/share/icons/hicolor/256x256/apps"
    icons.mkdir(parents=True)
    shutil.copy2(appdir / "libretoon.png", icons / "libretoon.png")
    copyright = deb / "usr/share/doc/libretoon"
    copyright.mkdir(parents=True)
    shutil.copy2(ROOT / "LICENSE", copyright / "copyright")
    (copyright / "THIRD-PARTY.txt").write_text("Application notices: /opt/libretoon/assets. FFmpeg is not included in this package.\n")
    run(["dpkg-deb", "--root-owner-group", "--build", deb, dest / (stem + ".deb")])
    # Wrap the AppImage to preserve its executable bit through Actions artifact download.
    with tarfile.open(dest / (stem + "-AppImage.tar.gz"), "w:gz") as t:
        t.add(dest / (stem + ".AppImage"), arcname=stem + ".AppImage")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=["macos", "linux"])
    args = parser.parse_args()
    if args.platform == "macos" and sys.platform != "darwin" or args.platform == "linux" and sys.platform != "linux":
        parser.error("Package on the native runner")
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    (mac if args.platform == "macos" else linux)(dist)
    (dist / "SHA256SUMS.txt").write_text("".join(digest(p) + "  " + p.name + "\n" for p in sorted(dist.iterdir()) if p.is_file() and p.name != "SHA256SUMS.txt"))
