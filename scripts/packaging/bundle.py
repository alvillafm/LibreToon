#!/usr/bin/env python3
"""Assemble verified Actions artifacts into one directory; Actions creates the ZIP."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tomllib

ROOT = Path(__file__).resolve().parents[2]
GROUPS = {
    "windows": {"Windows-x64"},
    "macos": {"macOS-arm64", "macOS-x64"},
    "linux": {"Linux-arm64", "Linux-x64"},
}
GROUPS["all"] = set().union(*GROUPS.values())


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def plain_files(directory):
    files = sorted(directory.rglob("*"))
    if directory.is_symlink() or any(p.is_symlink() for p in files):
        raise RuntimeError("Symbolic links are not allowed in downloaded artifacts")
    return [p for p in files if p.is_file()]


def verify_packages(directory, report):
    manifest = directory / "SHA256SUMS.txt"
    actual = plain_files(directory)
    names = {p.name for p in actual if p != manifest}
    if any(p.parent != directory for p in actual) or not names:
        raise RuntimeError("Missing or nested package files: " + directory.name)
    listed = {}
    for line in manifest.read_text(encoding="utf-8-sig").splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r"([a-fA-F0-9]{64})  ([^/\\]+)", line)
        if not match or match[2] in (".", "..", "SHA256SUMS.txt") or match[2] in listed:
            raise RuntimeError("Invalid package checksum entry: " + line)
        listed[match[2]] = match[1].lower()
    if names != listed.keys():
        raise RuntimeError("Package list differs from its checksum manifest: " + directory.name)
    checked = {f["name"]: f["sha256"].lower() for f in report.get("files", [])}
    for name, checksum in listed.items():
        if digest(directory / name) != checksum or checked.get(name) != checksum:
            raise RuntimeError("Package differs from its security report or checksum: " + name)


def assemble(source, output, selection, version):
    source, output = Path(source), Path(output)
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9.-]+)?", version):
        raise RuntimeError("Invalid package version")
    if output.exists() or source.resolve() in output.resolve().parents:
        raise RuntimeError("Choose a new output directory outside the downloaded artifacts")
    packages, reports = {}, {}
    for directory in sorted(source.iterdir()):
        if not directory.is_dir() or not directory.name.startswith("LibreToon-"):
            raise RuntimeError("Unexpected artifact: " + directory.name)
        label = directory.name.removeprefix("LibreToon-")
        if label.endswith("-security-report"):
            group = label.removesuffix("-security-report")
            if group == "Windows":
                group = "Windows-x64"
            if group not in GROUPS[selection] or group in reports:
                raise RuntimeError("Unexpected security report: " + label)
            plain_files(directory)
            report = json.loads((directory / "security-report.json").read_text(encoding="utf-8-sig"))
            if report.get("status") != "passed":
                raise RuntimeError("Security checks did not pass: " + label)
            reports[group] = (directory, report)
        else:
            group = label
            if label in ("Windows-x64-signed", "Windows-x64-unsigned"):
                group = "Windows-x64"
            if group not in GROUPS[selection] or group in packages:
                raise RuntimeError("Unexpected or duplicate package architecture: " + label)
            packages[group] = directory
    if packages.keys() != GROUPS[selection] or reports.keys() != GROUPS[selection]:
        raise RuntimeError("Not all requested architectures and security reports were produced")
    for group, directory in packages.items():
        verify_packages(directory, reports[group][1])
    output.mkdir(parents=True)
    for group, directory in packages.items():
        shutil.copytree(directory, output / directory.name.removeprefix("LibreToon-"))
        shutil.copytree(reports[group][0], output / "Security" / group)
    (output / "README.txt").write_text(
        f"LibreToon {version} - {selection} packages\n\n"
        "Choose the folder for your operating system and processor.\n"
        "Windows: use the Setup.exe installer OR extract the Portable.zip.\n"
        "macOS: choose arm64 for Apple Silicon or x64 for Intel; use the DMG or ZIP.\n"
        "Linux: choose x64 for Intel/AMD or arm64 for 64-bit ARM (aarch64). ARM32 is not included.\n"
        "Linux packages target Ubuntu 22.04+ (glibc 2.35). Graphics drivers remain system prerequisites.\n"
        "For an AppImage: chmod +x LibreToon-*.AppImage, then run it.\n"
        "The AppImage.tar.gz preserves executable permissions when extracted on Linux.\n"
        "FFmpeg is not bundled. Linux ARM64 video export requires selecting a separately installed FFmpeg.\n\n"
        "Security/ contains the per-platform verification reports.\n"
        "SHA256SUMS.txt covers this directory; each platform also retains its original manifest.\n"
        "A passing security scan does not guarantee absence of malware or removal of browser warnings.\n",
        encoding="utf-8",
    )
    (output / "BUILD-INFO.json").write_text(json.dumps({
        "version": version, "selection": selection, "architectures": sorted(packages),
        "commit": os.environ.get("GITHUB_SHA", "local"),
    }, indent=2) + "\n", encoding="utf-8")
    files = plain_files(output)
    (output / "SHA256SUMS.txt").write_text("".join(
        digest(p) + "  " + p.relative_to(output).as_posix() + "\n" for p in files
    ), encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=GROUPS, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    assemble(args.source, args.output, args.platform, version)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write("version=" + version + "\n")
    print("Combined download ready:", args.output)
