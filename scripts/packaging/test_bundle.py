"""Combined-download regressions; no builds, network, scanners or package extraction."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("bundle_under_test", ROOT / "scripts/packaging/bundle.py")
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)
TEST_ROOT = ROOT / "work/packaging-tests"
TEST_ROOT.mkdir(parents=True, exist_ok=True)


class CombinedPackages(unittest.TestCase):
    def fixture(self, root, group):
        label = "Windows-x64-unsigned" if group == "Windows-x64" else group
        packages = root / ("LibreToon-" + label)
        packages.mkdir()
        name = "LibreToon-0.9.0-" + group + ".zip"
        (packages / name).write_bytes(b"fixture " + group.encode())
        checksum = bundle.digest(packages / name)
        (packages / "SHA256SUMS.txt").write_text(checksum + "  " + name + "\n", encoding="utf-8-sig")
        report_label = "Windows" if group == "Windows-x64" else group
        report = root / ("LibreToon-" + report_label + "-security-report")
        report.mkdir()
        (report / "security-report.json").write_text(json.dumps({
            "status": "passed", "files": [{"name": name, "sha256": checksum}],
        }), encoding="utf-8-sig")
        return packages, report, name

    def test_one_download_contains_every_architecture_and_verified_global_hashes(self):
        with tempfile.TemporaryDirectory(dir=TEST_ROOT) as temporary:
            root = Path(temporary)
            source = root / "input"
            source.mkdir()
            for group in bundle.GROUPS["all"]:
                self.fixture(source, group)
            output = root / "combined"
            bundle.assemble(source, output, "all", "0.9.0")
            for group in bundle.GROUPS["all"]:
                folder = "Windows-x64-unsigned" if group == "Windows-x64" else group
                self.assertTrue((output / folder / "SHA256SUMS.txt").is_file())
                self.assertTrue((output / "Security" / group / "security-report.json").is_file())
            for line in (output / "SHA256SUMS.txt").read_text().splitlines():
                checksum, name = line.split("  ", 1)
                self.assertEqual(bundle.digest(output / name), checksum)
            metadata = json.loads((output / "BUILD-INFO.json").read_text())
            self.assertEqual(metadata["architectures"], sorted(bundle.GROUPS["all"]))

    def test_windows_only_and_linux_only_selections(self):
        for selection in ["windows", "linux", "macos"]:
            with self.subTest(selection=selection), tempfile.TemporaryDirectory(dir=TEST_ROOT) as temporary:
                root = Path(temporary)
                source = root / "input"
                source.mkdir()
                for group in bundle.GROUPS[selection]:
                    self.fixture(source, group)
                bundle.assemble(source, root / "output", selection, "0.9.0")

    def test_command_line_exports_the_version_for_the_artifact_name(self):
        with tempfile.TemporaryDirectory(dir=TEST_ROOT) as temporary:
            root=Path(temporary)
            source=root/"input"
            source.mkdir()
            self.fixture(source,"Windows-x64")
            version_output=root/"github-output"
            subprocess.run([
                sys.executable,str(ROOT/"scripts/packaging/bundle.py"),
                "--platform","windows","--source",str(source),"--output",str(root/"output"),
            ],env=dict(os.environ,GITHUB_OUTPUT=str(version_output)),check=True,capture_output=True,text=True)
            version=bundle.tomllib.loads((ROOT/"Cargo.toml").read_text())["package"]["version"]
            self.assertEqual(version_output.read_text(),"version="+version+"\n")
            self.assertTrue((root/"output/Windows-x64-unsigned"/"SHA256SUMS.txt").is_file())

    def test_missing_arm64_does_not_publish_a_partial_linux_bundle(self):
        with tempfile.TemporaryDirectory(dir=TEST_ROOT) as temporary:
            root = Path(temporary)
            self.fixture(root, "Linux-x64")
            with self.assertRaisesRegex(RuntimeError, "Not all requested"):
                bundle.assemble(root, root.parent / (root.name + "-output"), "linux", "0.9.0")

    def test_changed_package_or_security_report_blocks_assembly(self):
        for corruption in ["package", "report", "failed-scan", "traversal", "extra-file"]:
            with self.subTest(corruption=corruption), tempfile.TemporaryDirectory(dir=TEST_ROOT) as temporary:
                root = Path(temporary)
                source = root / "input"
                source.mkdir()
                packages, report, name = self.fixture(source, "Windows-x64")
                report_file = report / "security-report.json"
                if corruption == "package":
                    (packages / name).write_bytes(b"changed after scanning")
                elif corruption == "report":
                    data = json.loads(report_file.read_text(encoding="utf-8-sig"))
                    data["files"][0]["sha256"] = "0" * 64
                    report_file.write_text(json.dumps(data))
                elif corruption == "failed-scan":
                    report_file.write_text('{"status":"failed"}')
                elif corruption == "traversal":
                    (packages / "SHA256SUMS.txt").write_text("0" * 64 + "  ../outside.zip\n")
                else:
                    (packages / "unchecked.exe").write_bytes(b"not in the manifest")
                with self.assertRaises(RuntimeError):
                    bundle.assemble(source, root / "output", "windows", "0.9.0")
                self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
