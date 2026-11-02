#!/usr/bin/env python3
"""Native FFmpeg build/review gate. Run only from the manual packaging workflow.
The existing Windows source/link/reproduction gate is deliberately independent.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "scripts/ffmpeg/sources.lock.json"
RECIPE = Path(__file__).resolve()
POLICY = ROOT / "scripts/ffmpeg/NATIVE_POLICY.md"
SOURCE_IDS = {"ffmpeg", "x264", "zlib"}
ARCHIVES = {"libavcodec.a", "libavformat.a", "libavutil.a", "libavfilter.a",
            "libavdevice.a", "libswresample.a", "libswscale.a", "libpostproc.a",
            "libx264.a", "libz.a"}
RECIPES = ["scripts/ffmpeg/native.py", "scripts/ffmpeg/NATIVE_POLICY.md",
           "scripts/ffmpeg/sources.lock.json", "scripts/test-ffmpeg.ps1",
           "scripts/test-ffmpeg-app.ps1", "src/exporter.rs"]


def digest(path):
    with Path(path).open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def run(args, cwd=None, env=None, log=None):
    if log:
        with Path(log).open("w") as f:
            subprocess.run(list(map(str, args)), cwd=cwd, env=env, stdout=f,
                           stderr=subprocess.STDOUT, check=True)
        return ""
    return subprocess.check_output(list(map(str, args)), cwd=cwd, env=env,
                                   stderr=subprocess.STDOUT, text=True)


def download(url, path, sha, mirrors=(), expected_bytes=None):
    path = Path(path)
    history = []
    def accepted():
        return path.is_file() and digest(path) == sha and (expected_bytes is None or path.stat().st_size == expected_bytes)
    if accepted():
        return
    if path.exists():
        quarantine = path.with_name(path.name + ".rejected-" + uuid.uuid4().hex)
        history.append({"source": "cache", "actualSha256": digest(path), "preserved": str(quarantine)})
        path.rename(quarantine)
    for location in [url, *mirrors]:
        temporary = path.with_name(path.name + ".download-" + uuid.uuid4().hex)
        record = {"url": location, "expectedSha256": sha, "preserved": str(temporary)}
        try:
            request = urllib.request.Request(location, headers={"Accept-Encoding": "identity", "User-Agent": "LibreToon-SourceVerifier/1.0"})
            with urllib.request.urlopen(request, timeout=600) as r, temporary.open("wb") as f:
                record.update(status=r.status, contentType=r.headers.get("Content-Type"), contentEncoding=r.headers.get("Content-Encoding"))
                shutil.copyfileobj(r, f)
            record.update(actualSha256=digest(temporary), actualBytes=temporary.stat().st_size)
            record["accepted"] = record["actualSha256"] == sha and (expected_bytes is None or record["actualBytes"] == expected_bytes)
            history.append(record)
            write(path.with_name(path.name + ".download.json"), history)
            if record["accepted"]:
                temporary.rename(path)
                return
        except Exception as error:
            record["error"] = str(error)
            history.append(record)
            write(path.with_name(path.name + ".download.json"), history)
    raise RuntimeError("Locked download failed; responses and checksums preserved: " + str(path))

def link_archives(data):
    """Inspect ASCII archive/member delimiters without decoding arbitrary map bytes."""
    archives = {}
    pattern = rb"(?P<path>(?:[^\s()]+/)?(?P<name>lib[^\s/()]+\.a))(?:\[\d+\])?\("
    for match in re.finditer(pattern, data):
        try:
            name = match["name"].decode("ascii")
        except UnicodeDecodeError as error:
            raise RuntimeError("Unreviewed non-ASCII archive identifier") from error
        archives.setdefault(name, set()).add(os.fsdecode(match["path"]))
    return archives


GCC_ARCHIVES = {"libgcc.a", "libgcc_eh.a"}


def check_static_coverage(archives, runtime_sources):
    names = set(archives)
    if not {"libx264.a", "libz.a", "libavcodec.a"} <= names:
        raise RuntimeError("Incomplete static link map: " + repr(sorted(names)))
    extra = names - ARCHIVES
    if not extra:
        return
    covered = set()
    for source in runtime_sources:
        if source.get("component") == "gcc-runtime" and source.get("sourceVersion") and source.get("package") == "gcc-11-source":
            covered.update(source.get("archives", {}))
    if not extra <= GCC_ARCHIVES or not extra <= covered:
        raise RuntimeError("Static source coverage requires review: " + repr(sorted(extra)))


def gcc_runtime_sources(cc, work, archives, notices, system_root=Path("/")):
    """Bind libgcc actually selected by the linker to matching Ubuntu source packages."""
    extra = set(archives) - ARCHIVES
    if not extra:
        return []
    if sys.platform != "linux" or not extra <= GCC_ARCHIVES:
        raise RuntimeError("Unreviewed static runtime: " + repr(sorted(extra)))
    metadata_format = "${source:Package}\t${source:Version}\t${Version}"
    def metadata(package):
        values = run(["dpkg-query", "-W", "-f=" + metadata_format, package]).strip().split("\t")
        if len(values) != 3:
            raise RuntimeError("Missing Debian source identity for " + package)
        return values
    source_package, source_version, installed = metadata("gcc-11-source")
    if source_package != "gcc-11" or source_version != installed:
        raise RuntimeError("Unexpected GCC source package identity")
    static = {}
    for name in sorted(extra):
        file = Path(run([cc, "-print-file-name=" + name]).strip()).resolve(strict=True)
        if any(Path(p).resolve(strict=True) != file for p in archives[name]):
            raise RuntimeError("Link map refers to a different compiler runtime: " + name)
        owner_line = run(["dpkg-query", "-S", file]).strip().splitlines()
        if len(owner_line) != 1:
            raise RuntimeError("Ambiguous GCC archive owner")
        owner = owner_line[0].split(": ", 1)[0]
        origin, version, binary_version = metadata(owner)
        if origin != source_package or version != source_version:
            raise RuntimeError("GCC runtime/source package versions differ")
        static[name] = {"path": str(file), "sha256": digest(file), "owner": owner, "version": binary_version}
    source_root = system_root / "usr/src/gcc-11"
    package_files = [Path(p) for p in run(["dpkg-query", "-L", "gcc-11-source"]).splitlines()]
    source_files = sorted(p for p in package_files if p.is_file() and not p.is_symlink() and source_root in p.parents)
    if not any(p.name == "rules" and p.parent.name == "debian" for p in source_files):
        raise RuntimeError("GCC source packaging/build scripts absent")
    tarballs = [p for p in source_files if p.name.startswith("gcc-") and p.name.endswith(".tar.xz")]
    if len(tarballs) != 1:
        raise RuntimeError("Expected the exact GCC source tarball")
    licenses_found = set()
    with tarfile.open(tarballs[0]) as t:
        for member in t:
            base = Path(member.name).name
            if member.isfile() and base in {"COPYING.RUNTIME", "COPYING3"} and base not in licenses_found:
                original = t.extractfile(member).read()
                if base == "COPYING.RUNTIME" and b"GCC RUNTIME LIBRARY EXCEPTION" not in original:
                    raise RuntimeError("GCC runtime exception text missing")
                (notices / ("GCC-" + base + ".txt")).write_bytes(original)
                licenses_found.add(base)
                if len(licenses_found) == 2:
                    break
    if licenses_found != {"COPYING.RUNTIME", "COPYING3"}:
        raise RuntimeError("Original GCC license/exception absent")
    copyright_file = system_root / "usr/share/doc/gcc-11-source/copyright"
    shutil.copy2(copyright_file, notices / "GCC-Ubuntu-COPYRIGHT.txt")
    bundle = work / "sources/gcc-runtime-source.zip"
    identity = {"component": "gcc-runtime", "package": "gcc-11-source",
                "sourcePackage": source_package, "sourceVersion": source_version,
                "archives": static, "files": {str(p): digest(p) for p in source_files}}
    with zipfile.ZipFile(bundle, "w", zipfile.ZIP_DEFLATED) as z:
        for p in source_files:
            z.write(p, str(p.relative_to(system_root)))
        for p in package_files:
            if p.is_symlink() and source_root in p.parents:
                link = zipfile.ZipInfo(str(p.relative_to(system_root)))
                link.create_system = 3
                link.external_attr = 0o120777 << 16
                z.writestr(link, os.readlink(p))
        z.write(copyright_file, "copyright")
        z.writestr(zipfile.ZipInfo("IDENTITY.json"), json.dumps(identity, indent=2))
    identity.update(file=bundle.name, sha256=digest(bundle), bytes=bundle.stat().st_size)
    return [identity]


def unpack(archive, dest):
    dest.mkdir()
    with tarfile.open(archive) as t:
        # Drop the upstream wrapper; Python 3.12's data filter rejects escapes.
        members = []
        for m in t.getmembers():
            parts = Path(m.name).parts
            if len(parts) <= 1:
                continue
            m.name = str(Path(*parts[1:]))
            members.append(m)
        t.extractall(dest, members=members, filter="data")


def imports(binary):
    if sys.platform == "darwin":
        lines = run(["otool", "-L", binary]).splitlines()[1:]
        result = [line.strip().split(" (", 1)[0] for line in lines]
        if any(not x.startswith(("/usr/lib/", "/System/Library/")) for x in result):
            raise RuntimeError("Non-system Mach-O dependency: " + repr(result))
    else:
        output = run(["ldd", binary])
        if "not found" in output:
            raise RuntimeError(output)
        result = re.findall(r"^\s*(\S+)\s+=>", output, re.M)
        allowed = re.compile(r"^(libc|libm|libdl|libpthread|librt|libgcc_s)\.so[.0-9]*$")
        if any(not allowed.fullmatch(x) for x in result):
            raise RuntimeError("Unreviewed ELF dependency: " + repr(result))
    return result


def build(work):
    if sys.platform not in ("darwin", "linux"):
        raise RuntimeError("This recipe is native Linux/macOS only")
    work = work.resolve()
    work.mkdir(parents=True, exist_ok=False)  # Never discard an earlier build.
    for part in ["sources", "src", "logs", "output", "prefix", "x264", "ffmpeg"]:
        (work / part).mkdir()
    lock = read(LOCK)
    for item in lock["inputs"]:
        if item["id"] in SOURCE_IDS:
            archive = work / "sources" / item["file"]
            cached = ROOT.parent / "sources" / item["file"]
            if cached.is_file():
                shutil.copy2(cached, archive)
            download(item["url"], archive, item["sha256"], item.get("mirrors", ()), item["bytes"])
            log = archive.with_name(archive.name + ".download.json")
            if log.exists():
                shutil.copy2(log, work / "logs" / (item["id"] + "-download.json"))
            unpack(archive, work / "src" / item["id"])
    env = dict(os.environ, LC_ALL="C", TZ="UTC",
               SOURCE_DATE_EPOCH=str(lock["sourceDateEpoch"]), ZERO_AR_DATE="1")
    prefix = work / "prefix"
    env["PKG_CONFIG_LIBDIR"] = str(prefix / "lib/pkgconfig")
    env["PKG_CONFIG_PATH"] = ""
    cc = "clang" if sys.platform == "darwin" else "gcc-11"
    cflags = "-O2 -ffile-prefix-map=" + str(work) + "=/build -fdebug-prefix-map=" + str(work) + "=/build"
    env.update(CC=cc, CFLAGS=cflags)
    jobs = str(min(os.cpu_count() or 2, 4))
    logs = work / "logs"
    (logs / "compiler.txt").write_text(run([cc, "-v"]), encoding="utf-8")
    (logs / "host.txt").write_text(platform.platform() + "\n" + run(["uname", "-a"]), encoding="utf-8")
    def cmd(args, cwd, name):
        run(args, cwd, env, logs / name)
    zlib = work / "src/zlib"
    cmd(["sh", "configure", "--static", "--prefix=" + str(prefix)], zlib, "zlib-configure.txt")
    cmd(["make", "-j" + jobs], zlib, "zlib-build.txt")
    cmd(["make", "install"], zlib, "zlib-install.txt")
    x264 = work / "x264"
    # No optional external libraries; C-only x264 avoids hidden assembly tool inputs.
    cmd([work / "src/x264/configure", "--prefix=" + str(prefix), "--enable-static",
         "--disable-cli", "--disable-opencl", "--disable-lavf", "--disable-swscale",
         "--disable-asm", "--extra-cflags=" + cflags], x264, "x264-configure.txt")
    cmd(["make", "-j" + jobs], x264, "x264-build.txt")
    cmd(["make", "install"], x264, "x264-install.txt")
    for file in ["config.mak", "config.h"]:
        shutil.copy2(x264 / file, logs / ("x264-" + file))
    shutil.copy2(zlib / "zlib.pc", logs / "zlib.pc")
    ff = work / "ffmpeg"
    linkmap = str(logs / "link.map")
    ldflags = ("-Wl,-no_uuid,-map," if sys.platform == "darwin" else "-Wl,--build-id=none,-Map,") + linkmap
    flags = [
        "--prefix=" + str(prefix), "--cc=" + cc, "--enable-gpl", "--enable-version3",
        "--enable-libx264", "--enable-zlib", "--enable-static", "--disable-shared",
        "--disable-autodetect", "--disable-debug", "--disable-doc", "--disable-ffplay",
        "--disable-asm", "--disable-videotoolbox", "--disable-audiotoolbox",
        "--disable-securetransport", "--disable-everything",
        "--enable-protocol=file,pipe",
        "--enable-demuxer=rawvideo,mov,matroska,avi,gif,image2",
        "--enable-muxer=mp4,matroska,avi,mov,gif,image2,rawvideo,null",
        "--enable-encoder=libx264,mpeg4,prores_ks,gif,png,rawvideo",
        "--enable-decoder=h264,mpeg4,prores,gif,png,rawvideo",
        "--enable-parser=h264,mpeg4video,gif",
        "--enable-filter=pad,format,scale,split,palettegen,paletteuse,null,ssim,psnr,setsar,setpts",
        "--pkg-config-flags=--static", "--extra-version=LibreToon-1",
        "--extra-cflags=" + cflags + " -I" + str(prefix / "include"),
        "--extra-ldflags=" + ldflags + " -L" + str(prefix / "lib"),
    ]
    cmd([work / "src/ffmpeg/configure", *flags], ff, "ffmpeg-configure.txt")
    config = ff / "config.h"
    config.write_text(config.read_text().replace(str(work), "/build"))
    for binary in ["ffmpeg", "ffprobe"]:
        cmd(["make", "-j" + jobs, binary], ff, binary + "-build.txt")
        shutil.copy2(ff / binary, work / "output" / binary)
        shutil.copy2(logs / "link.map", logs / (binary + "-link.map"))
    for file in ["config.h", "config_components.h", "ffbuild/config.mak", "ffbuild/config.log"]:
        shutil.copy2(ff / file, logs / Path(file).name)
    version = run([work / "output/ffmpeg", "-version"])
    license_text = run([work / "output/ffmpeg", "-L"])
    (logs / "version.txt").write_text(version)
    (logs / "license.txt").write_text(license_text)
    if "--enable-gpl" not in version or "--enable-version3" not in version or "--enable-nonfree" in version:
        raise RuntimeError("Unexpected FFmpeg licensing configuration")
    static_paths = {}
    for binary in ["ffmpeg", "ffprobe"]:
        for name, paths in link_archives((logs / (binary + "-link.map")).read_bytes()).items():
            static_paths.setdefault(name, set()).update(paths)
    notices = work / "output/licenses"
    notices.mkdir()
    runtime_sources = gcc_runtime_sources(cc, work, static_paths, notices)
    check_static_coverage(static_paths, runtime_sources)
    static = set(static_paths)
    libraries = {b: imports(work / "output" / b) for b in ["ffmpeg", "ffprobe"]}
    for file in ["COPYING.GPLv2", "COPYING.GPLv3", "COPYING.LGPLv2.1", "COPYING.LGPLv3", "LICENSE.md"]:
        shutil.copy2(work / "src/ffmpeg" / file, notices / file)
    shutil.copy2(work / "src/x264/COPYING", notices / "x264-COPYING")
    shutil.copy2(work / "src/x264/x264.h", notices / "x264-header.h")
    shutil.copy2(work / "src/zlib/LICENSE", notices / "zlib-LICENSE")
    (work / "output/THIRD-PARTY-NOTICES.txt").write_text(
        "FFmpeg 7.1.5 is a separate GPLv3-or-later executable. x264 is GPLv2-or-later; "
        "the combined executable uses GPLv3-or-later. zlib retains its original license.\n"
        "Full source archives, exact configuration, build logs and MIT build recipes accompany this binary.\n"
        "This software is based in part on the work of the Independent JPEG Group.\n"
        "GCC runtime, when incorporated: see original GCC COPYING3, COPYING.RUNTIME and Ubuntu copyright notices.\n"
        "No Imageio binary/code is used in this native build. LibreToon's own code remains MIT.\n")
    receipt = {
        "schema": 1, "platform": sys.platform, "architecture": platform.machine(),
        "binarySha256": digest(work / "output/ffmpeg"),
        "probeSha256": digest(work / "output/ffprobe"),
        "recipes": {p: digest(ROOT / p) for p in RECIPES},
        "sourceInputs": [x for x in lock["inputs"] if x["id"] in SOURCE_IDS],
        "staticArchives": sorted(static), "systemImports": libraries,
        "runtimeSourceInputs": runtime_sources,
        "toolchain": (logs / "compiler.txt").read_text(),
        "configuration": flags, "sourcePatches": [],
    }
    write(work / "output/BUILD_RECEIPT.json", receipt)


def review(work, other):
    work, other = work.resolve(), other.resolve()
    out = work / "output"
    receipt = read(out / "BUILD_RECEIPT.json")
    reproduction = read(other / "output/BUILD_RECEIPT.json")
    for field, binary in [("binarySha256", "ffmpeg"), ("probeSha256", "ffprobe")]:
        if receipt[field] != reproduction[field] or receipt[field] != digest(out / binary) or receipt[field] != digest(other / "output" / binary):
            raise RuntimeError("Independent native reproduction differs: " + binary)
    check_static_coverage(receipt["staticArchives"], receipt.get("runtimeSourceInputs", []))
    if receipt.get("runtimeSourceInputs", []) != reproduction.get("runtimeSourceInputs", []):
        raise RuntimeError("Independent builds used different compiler runtime/source evidence")
    exports = read(out / "EXPORT_VALIDATION.json")
    app = read(out / "APPLICATION_TEST.json")
    if exports["status"] != "passed" or len(exports["cases"]) != 10 or exports["binarySha256"] != receipt["binarySha256"]:
        raise RuntimeError("Ten export format/quality cases are required")
    if app["status"] != "passed" or app["testExitCode"] != 0 or app["binarySha256"] != receipt["binarySha256"]:
        raise RuntimeError("Application export test required")
    if app["exporterSha256"] != digest(ROOT / "src/exporter.rs") or app["testScriptSha256"] != digest(ROOT / "scripts/test-ffmpeg-app.ps1") or exports["testScriptSha256"] != digest(ROOT / "scripts/test-ffmpeg.ps1"):
        raise RuntimeError("Stale application/export evidence")
    for p, sha in receipt["recipes"].items():
        if digest(ROOT / p) != sha:
            raise RuntimeError("Recipe changed: " + p)
    bundle = out / "ffmpeg-complete-source.zip"
    with zipfile.ZipFile(bundle, "w", zipfile.ZIP_DEFLATED) as z:
        for item in receipt["sourceInputs"] + receipt.get("runtimeSourceInputs", []):
            p = work / "sources" / item["file"]
            if digest(p) != item["sha256"]:
                raise RuntimeError("Source changed after build")
            z.write(p, "sources/" + p.name)
        for p in RECIPES + ["LICENSE"]:
            z.write(ROOT / p, "recipe/" + p)
        for p in sorted((work / "logs").iterdir()):
            if p.is_file():
                z.write(p, "logs/" + p.name)
        for p in sorted((out / "licenses").iterdir()):
            z.write(p, "licenses/" + p.name)
        for name in ["BUILD_RECEIPT.json", "APPLICATION_TEST.json", "EXPORT_VALIDATION.json", "THIRD-PARTY-NOTICES.txt"]:
            z.write(out / name, name)
        z.writestr("REBUILD.txt", "On the recorded native OS/architecture install the compiler, make, pkg-config, Python 3.12+ and PowerShell 7. "
                   "Extract recipe into a checkout. Run python3 scripts/ffmpeg/native.py build work/rebuild. "
                   "The hash-locked official archives are included in sources; network retrieval uses exactly the same hashes. "
                   "On Ubuntu, install the exact gcc-11/libgcc-11-dev/gcc-11-source package versions recorded in BUILD_RECEIPT.json. "
                   "The matching GCC source tree, Debian patches/build rules and notices are in sources/gcc-runtime-source.zip when libgcc is incorporated. "
                   "System dynamic libraries/build tools are not redistributed. Original input sources are unmodified; see BUILD_RECEIPT.json.\n")
    write(out / "SOURCE_STATUS.json", {
        "schema": 1, "status": "reviewed-native", "binarySha256": receipt["binarySha256"],
        "sourceSha256": digest(bundle), "reproducedBinarySha256": reproduction["binarySha256"],
        "exportsSha256": digest(out / "EXPORT_VALIDATION.json"),
        "applicationSha256": digest(out / "APPLICATION_TEST.json"),
        "receiptSha256": digest(out / "BUILD_RECEIPT.json"),
    })


def stage(work, destination):
    out = work.resolve() / "output"
    manifest = read(out / "SOURCE_STATUS.json")
    receipt = read(out / "BUILD_RECEIPT.json")
    required = {"binarySha256": "ffmpeg", "sourceSha256": "ffmpeg-complete-source.zip",
                "exportsSha256": "EXPORT_VALIDATION.json", "applicationSha256": "APPLICATION_TEST.json",
                "receiptSha256": "BUILD_RECEIPT.json"}
    if manifest["status"] != "reviewed-native" or manifest["reproducedBinarySha256"] != manifest["binarySha256"]:
        raise RuntimeError("Native distribution gate blocked")
    for key, file in required.items():
        if manifest[key] != digest(out / file):
            raise RuntimeError("Native distribution gate checksum mismatch: " + file)
    for p, sha in receipt["recipes"].items():
        if digest(ROOT / p) != sha:
            raise RuntimeError("Stale native recipe evidence: " + p)
    check_static_coverage(receipt["staticArchives"], receipt.get("runtimeSourceInputs", []))
    with zipfile.ZipFile(out / "ffmpeg-complete-source.zip") as z:
        for item in receipt["sourceInputs"] + receipt.get("runtimeSourceInputs", []):
            if hashlib.sha256(z.read("sources/" + item["file"])).hexdigest() != item["sha256"]:
                raise RuntimeError("Missing or changed corresponding source")
    imports(out / "ffmpeg")
    destination = destination.resolve()
    tools = destination / "assets/tools"
    tools.mkdir(parents=True, exist_ok=False)
    shutil.copy2(out / "ffmpeg", tools / "ffmpeg")
    shutil.copytree(out / "licenses", tools / "licenses")
    shutil.copy2(out / "THIRD-PARTY-NOTICES.txt", tools / "THIRD_PARTY.md")
    source = destination / "corresponding-source"
    source.mkdir()
    for file in ["ffmpeg-complete-source.zip", "BUILD_RECEIPT.json", "SOURCE_STATUS.json", "APPLICATION_TEST.json", "EXPORT_VALIDATION.json"]:
        shutil.copy2(out / file, source / file)
    (destination / "SOURCE-ACCESS.txt").write_text(
        "Complete corresponding source for the bundled FFmpeg is included at "
        "corresponding-source/ffmpeg-complete-source.zip, in this same download. "
        "Keep it and assets/tools/licenses when redistributing this exact executable. "
        "LibreToon's own code is MIT; FFmpeg is an external GPLv3-or-later program.\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["build", "review", "stage"])
    parser.add_argument("work", type=Path)
    parser.add_argument("other", type=Path, nargs="?")
    args = parser.parse_args()
    if args.operation == "build":
        build(args.work)
    elif args.other is None:
        parser.error("review needs a reproduced work directory; stage needs a destination")
    elif args.operation == "review":
        review(args.work, args.other)
    else:
        stage(args.work, args.other)
