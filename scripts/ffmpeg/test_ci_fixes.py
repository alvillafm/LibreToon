"""Downloader/parser regression tests only. Does not compile or execute FFmpeg."""
import gzip
import hashlib
import http.server
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import threading
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ffmpeg_native_under_test", ROOT / "scripts/ffmpeg/native.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
GOOD = gzip.compress(b"trusted download fixture")
SHA = hashlib.sha256(GOOD).hexdigest()


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        good = self.path == "/official.tar.gz"
        data = GOOD if good else b"<html>upstream gateway error</html>"
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Content-Type", "application/gzip" if good else "text/html")
        if good:
            # This header made an automatically decompressing HTTP client mutate bytes.
            self.send_header("Content-Encoding", "gzip")
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


class Fixes(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.url = "http://127.0.0.1:" + str(cls.server.server_port)
        (ROOT / "work/ci-regression").mkdir(parents=True, exist_ok=True)

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def test_macho_map_non_utf8_and_member_indices(self):
        data = b"# symbol \xa9 not UTF-8\n[ 12] /build/lib/libavcodec.a[3](decoder.o)\n[ 13] /build/lib/libx264.a(x.o)"
        self.assertEqual(native.link_archives(data), {
            "libavcodec.a": {"/build/lib/libavcodec.a"}, "libx264.a": {"/build/lib/libx264.a"}})

    def test_gnu_map_ignores_unused_archive_loads(self):
        data = b"LOAD /usr/lib/libgcc_eh.a\n/usr/lib/gcc/11/libgcc.a(_divti3.o)\n/build/lib/libz.a(deflate.o)\n"
        self.assertEqual(set(native.link_archives(data)), {"libgcc.a", "libz.a"})

    def test_missing_runtime_sources_and_unknown_archives_still_block(self):
        core = {"libx264.a", "libz.a", "libavcodec.a"}
        with self.assertRaises(RuntimeError):
            native.check_static_coverage(core | {"libgcc.a"}, [])
        coverage = [{"component": "gcc-runtime", "package": "gcc-11-source", "sourceVersion": "fixture", "archives": {"libgcc.a": {}}}]
        native.check_static_coverage(core | {"libgcc.a"}, coverage)
        with self.assertRaises(RuntimeError):
            native.check_static_coverage(core | {"libgcc.a", "libunknown.a"}, coverage)
        with self.assertRaises(RuntimeError):
            native.check_static_coverage({"libgcc.a"}, coverage)

    def test_native_download_preserves_rejected_cache_and_uses_same_hash_mirror(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "work/ci-regression") as directory:
            path = Path(directory) / "locked.tar.gz"
            path.write_bytes(b"bad cached response")
            native.download(self.url + "/gateway", path, SHA, [self.url + "/official.tar.gz"], len(GOOD))
            self.assertEqual(path.read_bytes(), GOOD)
            self.assertTrue(list(path.parent.glob("*.rejected-*")))
            records = json.loads(path.with_name(path.name + ".download.json").read_text())
            self.assertFalse(records[1]["accepted"])
            self.assertTrue(records[-1]["accepted"])
            with self.assertRaises(RuntimeError):
                native.download(self.url + "/gateway", path.parent / "must-fail", SHA)

    def test_gcc_sources_match_archive_owner_and_reject_version_drift(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "work/ci-regression") as directory:
            work = Path(directory)
            system = work / "system"
            source = system / "usr/src/gcc-11"
            (source / "debian/patches").mkdir(parents=True)
            (work / "sources").mkdir()
            notices = work / "notices"
            notices.mkdir()
            archive = source / "gcc-11.4.0.tar.xz"
            with tarfile.open(archive, "w:xz") as t:
                for name, data in [("gcc/COPYING3", b"test fixture license placeholder"),
                                   ("gcc/COPYING.RUNTIME", b"GCC RUNTIME LIBRARY EXCEPTION - test fixture"),
                                   ("gcc/libgcc/libgcc2.c", b"test source fixture")]:
                    entry = tarfile.TarInfo(name)
                    entry.size = len(data)
                    t.addfile(entry, io.BytesIO(data))
            (source / "debian/rules").write_text("test build recipe")
            (source / "debian/patches/fixture.diff").write_text("test patch")
            copyright = system / "usr/share/doc/gcc-11-source/copyright"
            copyright.parent.mkdir(parents=True)
            copyright.write_text("test notice fixture")
            libgcc = system / "usr/lib/gcc/11/libgcc.a"
            libgcc.parent.mkdir(parents=True)
            libgcc.write_bytes(b"test archive fixture")
            source_files = [archive, source / "debian/rules", source / "debian/patches/fixture.diff"]
            def query(command, *args, **kwargs):
                if command[0] == "gcc-11":
                    return str(libgcc)
                if command[0] == "dpkg-query" and command[1] == "-S":
                    return "libgcc-11-dev:amd64: " + str(libgcc)
                if command[0] == "dpkg-query" and command[1] == "-L":
                    return "\n".join(map(str,source_files))
                return "gcc-11\t11.4.0-fixture\t11.4.0-fixture"
            paths = {"libgcc.a": {str(libgcc)}}
            with patch.object(native.sys, "platform", "linux"), patch.object(native, "run", side_effect=query):
                evidence = native.gcc_runtime_sources("gcc-11", work, paths, notices, system)
                again = native.gcc_runtime_sources("gcc-11", work, paths, notices, system)
            self.assertEqual(evidence, again, "runtime source evidence must be deterministic")
            self.assertEqual(evidence[0]["archives"]["libgcc.a"]["sha256"], native.digest(libgcc))
            import zipfile
            with zipfile.ZipFile(work / "sources/gcc-runtime-source.zip") as z:
                self.assertIn("usr/src/gcc-11/debian/patches/fixture.diff", z.namelist())
                self.assertIn("usr/src/gcc-11/gcc-11.4.0.tar.xz", z.namelist())
            def mismatched(command,*args,**kwargs):
                if command[-1] == "libgcc-11-dev:amd64":
                    return "gcc-11\t11.3.0-wrong\t11.3.0-wrong"
                return query(command,*args,**kwargs)
            with patch.object(native.sys, "platform", "linux"), patch.object(native, "run", side_effect=mismatched):
                with self.assertRaisesRegex(RuntimeError, "versions differ"):
                    native.gcc_runtime_sources("gcc-11", work, paths, notices, system)

    def test_windows_downloader_reads_wire_bytes_and_checks_mirrors(self):
        if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
            self.skipTest("Pass the installed pwsh executable path")
        with tempfile.TemporaryDirectory(dir=ROOT / "work/ci-regression") as directory:
            work = Path(directory)
            script = work / "downloader-only.ps1"
            source = ROOT / "scripts/build-ffmpeg.ps1"
            script.write_text("""
param([string]$Source, [string]$Work, [string]$Url, [string]$Sha, [long]$Size)
$ErrorActionPreference = 'Stop'
$tokens=$null; $errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile($Source,[ref]$tokens,[ref]$errors)
if ($errors.Count) {throw $errors}
$function=$ast.Find({param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-LockedInput'}, $true)
# Load only the downloader function, never the build script's top-level statements.
Invoke-Expression $function.Extent.Text
$input=[pscustomobject]@{id='fixture';file='fixture.tar.gz';url="$Url/gateway";mirrors=@("$Url/official.tar.gz");sha256=$Sha;bytes=$Size}
$path=Join-Path $Work $input.file
[IO.File]::WriteAllText($path,'bad cached response')
$actual=Get-LockedInput $input $Work (Join-Path $Work 'downloads.jsonl')
if ((Get-FileHash -LiteralPath $actual).Hash.ToLowerInvariant() -ne $Sha) {throw 'Integrity was bypassed'}
$input.file='must-fail.tar.gz'; $input.mirrors=@()
$failed=$false
try {Get-LockedInput $input $Work (Join-Path $Work 'downloads.jsonl') | Out-Null} catch {$failed=$true}
if (-not $failed) {throw 'Bad source was accepted'}
""", encoding="utf-8")
            result = subprocess.run([sys.argv[1], "-NoProfile", "-File", str(script),
                                     "-Source", str(source), "-Work", str(work),
                                     "-Url", self.url, "-Sha", SHA, "-Size", str(len(GOOD))],
                                    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual((work / "fixture.tar.gz").read_bytes(), GOOD)
            self.assertTrue(list(work.glob("fixture.tar.gz.rejected-*")))
            records = [json.loads(line) for line in (work / "downloads.jsonl").read_text(encoding="utf-8-sig").splitlines()]
            self.assertTrue(any(x.get("accepted") is False for x in records))
            self.assertTrue(any(x.get("accepted") is True for x in records))


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]], verbosity=2)
