"""Security gate regressions with simulated scanners; no network, signing or AV changes."""
import datetime as dt
import hashlib
import io
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
TEST_ROOT=ROOT/'work/packaging-tests'
TEST_ROOT.mkdir(parents=True,exist_ok=True)
spec = importlib.util.spec_from_file_location('security_under_test', ROOT/'scripts/packaging/security_native.py')
security = importlib.util.module_from_spec(spec)
spec.loader.exec_module(security)


class Gates(unittest.TestCase):
    def fixture(self, root):
        dist=root/'dist';dist.mkdir()
        payload=root/'work/packages/macos/LibreToon.app';payload.mkdir(parents=True)
        archive=dist/'LibreToon-test.zip';archive.write_bytes(b'package fixture')
        (dist/'SHA256SUMS.txt').write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+archive.name+'\n')

    def test_scanner_detection_and_errors_are_never_successful(self):
        security.require_success(0,'scan')
        for code in [1,2,127,-1]:
            with self.assertRaises(RuntimeError):security.require_success(code,'scan')

    def test_unknown_and_stale_definitions_are_rejected(self):
        now=dt.datetime(2026,11,4,12,tzinfo=dt.timezone.utc)
        self.assertEqual(security.database_timestamp('ClamAV 1.4.3/123/Wed Nov  4 10:00:00 2026',now).hour,10)
        for version in ['ClamAV 1.4.3','ClamAV 1.4.3/123/Sun Nov  1 10:00:00 2026']:
            with self.assertRaises(RuntimeError):security.database_timestamp(version,now)

    def test_missing_scanner_never_creates_a_pass_report(self):
        with tempfile.TemporaryDirectory(dir=TEST_ROOT) as tmp:
            root=Path(tmp);self.fixture(root)
            with patch.object(security.shutil,'which',return_value=None):
                with self.assertRaises(RuntimeError):security.scan(root)
            self.assertEqual(json.loads((root/'work/package-security/native/security-report.json').read_text())['status'],'failed')

    def test_success_detection_and_failed_update_are_reported_accurately(self):
        for failure in [None,'scan','update']:
            with self.subTest(failure=failure),tempfile.TemporaryDirectory(dir=TEST_ROOT) as tmp:
                root=Path(tmp);self.fixture(root);calls=[]
                def fake(args,stdout,**kwargs):
                    calls.append(args)
                    if '--version' in args:
                        stdout.write('ClamAV 1.4.3/123/'+dt.datetime.now(dt.timezone.utc).strftime('%a %b %d %H:%M:%S %Y')+'\n')
                    else:stdout.write('scanner fixture\n')
                    code=2 if failure=='update' and args[0]=='freshclam' else 1 if failure=='scan' and '--recursive' in args else 0
                    return subprocess.CompletedProcess(args,code)
                with patch.object(security.shutil,'which',side_effect=lambda n:n),patch.object(security.subprocess,'run',side_effect=fake),patch.dict(security.os.environ,{'RUNNER_TEMP':str(root/'temp')},clear=False):
                    if failure:
                        with patch('sys.stderr', new_callable=io.StringIO) as stderr:
                            with self.assertRaises(RuntimeError):security.scan(root)
                        self.assertIn('scanner fixture', stderr.getvalue())
                    else:security.scan(root)
                report=json.loads((root/'work/package-security/native/security-report.json').read_text())
                self.assertEqual(report['status'],'failed' if failure else 'passed')
                if failure:
                    self.assertEqual(report['failedCommand']['phase'], 'definition update' if failure=='update' else 'antivirus command')
                if failure is None:
                    self.assertIn('--scan-archive=yes',calls[-1]);self.assertIn('--alert-exceeds-max=yes',calls[-1])
                    self.assertIn(str(root/'work/packages/macos/LibreToon.app'),calls[-1])

    def test_ubuntu_uses_distro_paths_and_only_elevates_the_updater(self):
        for failed_update in [False, True]:
            with self.subTest(failed_update=failed_update), tempfile.TemporaryDirectory(dir=TEST_ROOT) as tmp:
                root=Path(tmp);self.fixture(root);calls=[]
                def fake(args,stdout,**kwargs):
                    calls.append(args)
                    if '--version' in args:
                        stdout.write('ClamAV 1.5.4/123/'+dt.datetime.now(dt.timezone.utc).strftime('%a %b %d %H:%M:%S %Y')+'\n')
                    else:stdout.write('ubuntu updater fixture\n')
                    return subprocess.CompletedProcess(args,2 if failed_update and args[0]=='sudo' else 0)
                with patch.dict(security.os.environ,{'LIBRETOON_CLAMAV_MODE':'ubuntu-system'}), patch.object(security.sys,'platform','linux'), patch.object(security.shutil,'which',side_effect=lambda n:n), patch.object(Path,'is_file',return_value=True), patch.object(Path,'is_dir',return_value=True), patch.object(security.subprocess,'run',side_effect=fake), patch('sys.stderr',new_callable=io.StringIO):
                    if failed_update:
                        with self.assertRaises(RuntimeError):security.scan(root)
                    else:security.scan(root)
                self.assertEqual([arg.replace(chr(92),'/') for arg in calls[0]],['sudo','-n','freshclam','--config-file=/etc/clamav/freshclam.conf','--datadir=/var/lib/clamav','--stdout'])
                if failed_update:self.assertEqual(len(calls),1)
                else:
                    self.assertEqual(calls[-1][0],'clamscan')
                    self.assertIn('--database='+str(Path('/var/lib/clamav')),calls[-1])
                    self.assertIn('--scan-archive=yes',calls[-1])
                report=json.loads((root/'work/package-security/native/security-report.json').read_text())
                self.assertEqual(report['status'],'failed' if failed_update else 'passed')
                self.assertEqual(report['databaseMode'],'ubuntu-system')

    def test_tampered_package_manifest_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=TEST_ROOT) as tmp:
            root=Path(tmp);self.fixture(root);(root/'dist/LibreToon-test.zip').write_bytes(b'tampered')
            with self.assertRaises(RuntimeError):security.inventory(root/'dist')

    @unittest.skipUnless(shutil.which('pwsh'),'PowerShell assertions require Windows/pwsh')
    def test_windows_disabled_stale_and_failed_scanners_are_rejected(self):
        module=str(ROOT/'scripts/packaging/WindowsSecurity.psm1').replace("'","''")
        script="""
Import-Module 'MODULE' -Force
Assert-ScannerExit 0
foreach ($code in @(1,2,127,-1)) {
    $blocked=$false;try {Assert-ScannerExit $code} catch {$blocked=$true}
    if (-not $blocked) {throw 'Unsafe scanner result accepted'}
}
$ready=[pscustomobject]@{AMServiceEnabled=$true;AntivirusEnabled=$true;AntivirusSignatureLastUpdated=[DateTime]::UtcNow;AntivirusSignatureVersion='fixture'}
Assert-DefenderReady $ready
$ready.AntivirusEnabled=$false
$blocked=$false;try {Assert-DefenderReady $ready} catch {$blocked=$true};if(-not $blocked){throw 'Disabled scanner accepted'}
$ready.AntivirusEnabled=$true;$ready.AntivirusSignatureLastUpdated=[DateTime]::UtcNow.AddDays(-4)
$blocked=$false;try {Assert-DefenderReady $ready} catch {$blocked=$true};if(-not $blocked){throw 'Stale scanner accepted'}
""".replace('MODULE',module)
        subprocess.run(['pwsh','-NoProfile','-NonInteractive','-Command',script],check=True,capture_output=True,text=True)

    def test_workflow_remains_manual_and_uploads_packages_only_after_gates(self):
        source=(ROOT/'.github/workflows/windows.yml').read_text()
        self.assertIn('workflow_dispatch:',source)
        for trigger in ['  push:','  pull_request:','  schedule:','gh release create']:
            self.assertNotIn(trigger,source)
        self.assertLess(source.index('windows-security.ps1'),source.index('Upload Windows portable package'))
        self.assertLess(source.index('security_native.py'),source.index('Upload native download packages'))
        self.assertNotIn('continue-on-error',source)
        self.assertIn("LIBRETOON_CLAMAV_MODE: ${{ matrix.platform == 'linux' && 'ubuntu-system' || 'isolated' }}",source)
        self.assertIn('sudo systemctl stop clamav-freshclam.service',source)
        self.assertIn('SignedUninstaller=yes',(ROOT/'scripts/packaging/windows.iss').read_text())


if __name__=='__main__':unittest.main(verbosity=2)
