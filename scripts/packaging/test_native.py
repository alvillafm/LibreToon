"""Execute Linux package staging with mocked packaging tools; no network or builds."""
import ast
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import tarfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("packaging_under_test",ROOT/"scripts/packaging/native.py")
packaging=importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class NativePackages(unittest.TestCase):
    def test_all_common_calls_have_the_lightweight_signature(self):
        tree=ast.parse((ROOT/"scripts/packaging/native.py").read_text())
        calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Name) and n.func.id=="common"]
        self.assertEqual(len(calls),2)
        self.assertTrue(all(len(n.args)==1 and not n.keywords for n in calls))

    def test_mac_places_only_executable_in_macos_before_signing_and_verifying(self):
        (ROOT/"work/packaging-tests").mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT/"work/packaging-tests") as temporary:
            root=Path(temporary)
            (root/"target/release").mkdir(parents=True)
            (root/"target/release/libretoon").write_bytes(b"application fixture")
            (root/"assets/locales").mkdir(parents=True)
            (root/"assets/locales/ja.json").write_text('{"fixture":"resource"}')
            (root/"assets/libretoon-icon.png").write_bytes(b"icon fixture")
            (root/"assets/tools").mkdir()
            (root/"assets/tools/ffmpeg").write_bytes(b"must not be packaged")
            (root/"LICENSE").write_text("MIT fixture")
            destination=root/"dist"
            destination.mkdir()
            commands=[]
            def fake_run(args,**kwargs):
                args=list(map(str,args))
                commands.append(args)
                if args[0]=="codesign":
                    contents=Path(args[-1])/"Contents"
                    self.assertEqual([p.name for p in (contents/"MacOS").iterdir()],["libretoon"])
                    self.assertEqual((contents/"Resources/LICENSE").read_text(),"MIT fixture")
                    self.assertTrue((contents/"Resources/assets/locales/ja.json").is_file())
                    self.assertTrue((contents/"Resources/BUILD-INFO.txt").is_file())
                    self.assertFalse((contents/"Resources/assets/tools").exists())
                    self.assertTrue((contents/"Info.plist").is_file())
                elif args[0] in ("iconutil","ditto","hdiutil"):
                    Path(args[-1]).write_bytes(b"tool output fixture")
            def fake_symlink(path,target,*args,**kwargs):
                path.write_text(str(target))
            with patch.object(packaging,"ROOT",root), patch.object(packaging,"check_system_imports"), \
                 patch.object(packaging.os,"uname",create=True) as uname, \
                 patch.object(packaging,"run",side_effect=fake_run),patch.object(Path,"symlink_to",new=fake_symlink):
                uname.return_value.machine="arm64"
                packaging.mac(destination)
            signatures=[args for args in commands if args[0]=="codesign"]
            self.assertEqual(len(signatures),2)
            self.assertIn("--sign",signatures[0])
            self.assertIn("--verify",signatures[1])
            self.assertIn("--strict",signatures[1])
            self.assertTrue(any(p.suffix==".zip" for p in destination.iterdir()))
            self.assertTrue(any(p.suffix==".dmg" for p in destination.iterdir()))

    def test_linux_stages_both_architectures_without_ffmpeg_or_sources(self):
        for machine,label,architecture,debian in [
            ("x86_64","x64","x86_64","amd64"),
            ("aarch64","arm64","aarch64","arm64"),
        ]:
            with self.subTest(machine=machine):
                self.stage_linux(machine,label,architecture,debian)

    def stage_linux(self,machine,label,architecture,debian):
        (ROOT/"work/packaging-tests").mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT/"work/packaging-tests") as temporary:
            root=Path(temporary)
            (root/"target/release").mkdir(parents=True)
            (root/"target/release/libretoon").write_bytes(b"application fixture")
            (root/"assets/tools").mkdir(parents=True)
            (root/"assets/tools/ffmpeg.exe").write_bytes(b"must not be packaged")
            (root/"assets/tools/ffmpeg-complete-source.zip").write_bytes(b"must not be packaged")
            (root/"assets/libretoon-icon.png").write_bytes(b"icon fixture")
            (root/"LICENSE").write_text("MIT fixture")
            destination=root/"dist"
            destination.mkdir()
            calls=[]
            environments=[]
            def fake_run(args,**kwargs):
                calls.append(args)
                if "env" in kwargs:
                    environments.append(kwargs["env"])
                Path(args[-1]).write_bytes(b"package tool output fixture")
            def fake_symlink(path,target,*args,**kwargs):
                # Windows CI does not require developer-mode symlink privileges for this test.
                path.write_text(str(target))
            with patch.object(packaging,"ROOT",root), patch.object(packaging,"check_system_imports"), \
                 patch.object(packaging.platform,"machine",return_value=machine), \
                 patch.object(packaging,"appimage_runtime",return_value=(Path("appimagetool-fixture"),Path("runtime-fixture"))) as runtime, \
                 patch.object(packaging,"run",side_effect=fake_run),patch.object(Path,"symlink_to",new=fake_symlink):
                packaging.linux(destination)
            self.assertEqual(len(calls),2)
            self.assertEqual(runtime.call_args.args[2],architecture)
            self.assertEqual(environments[0]["ARCH"],architecture)
            self.assertIn("Architecture: "+debian+"\n",(root/"work/packages/linux/deb/DEBIAN/control").read_text())
            stem="LibreToon-"+packaging.VERSION+"-Linux-"+label
            self.assertTrue((destination/(stem+".AppImage")).is_file())
            with tarfile.open(destination/(stem+"-AppImage.tar.gz")) as archive:
                self.assertEqual(archive.getnames(),[stem+".AppImage"])
                self.assertTrue(archive.getmembers()[0].mode & 0o111)
            self.assertTrue(any(p.suffix==".AppImage" for p in destination.iterdir()))
            self.assertTrue(any(p.suffix==".deb" for p in destination.iterdir()))
            self.assertTrue(any(p.name.endswith("-AppImage.tar.gz") for p in destination.iterdir()))
            self.assertFalse((root/"work/packages/linux/LibreToon.AppDir/usr/lib/libretoon/assets/tools").exists())
            self.assertFalse((root/"work/packages/linux/deb/opt/libretoon/assets/tools").exists())

    def test_linux_rejects_unsupported_architectures(self):
        for machine in ["armv7l","i686","unknown"]:
            with self.subTest(machine=machine), self.assertRaises(RuntimeError):
                packaging.linux_architecture(machine)
        self.assertEqual(packaging.linux_architecture("ARM64"),("arm64","aarch64","arm64"))
        self.assertEqual(packaging.linux_architecture("AMD64"),("x64","x86_64","amd64"))

    def test_appimage_uses_matching_official_tools_and_records_architecture(self):
        revision="abcdef0123456789abcdef0123456789abcdef01"
        source=io.BytesIO()
        with tarfile.open(fileobj=source,mode="w:gz") as archive:
            item=tarfile.TarInfo("runtime/LICENSE")
            item.size=3
            archive.addfile(item,io.BytesIO(b"MIT"))
        (ROOT/"work/packaging-tests").mkdir(parents=True,exist_ok=True)
        for architecture in ["x86_64","aarch64"]:
            with self.subTest(architecture=architecture), tempfile.TemporaryDirectory(dir=ROOT/"work/packaging-tests") as temporary:
                work=Path(temporary)
                def fake_asset(release,name,path):
                    path.write_bytes(b"verified binary fixture")
                    return {"name":name}
                with patch.object(packaging,"api",side_effect=[{}, {}, {"object":{"type":"commit","sha":revision}}]), \
                     patch.object(packaging,"asset",side_effect=fake_asset) as assets, \
                     patch.object(packaging.subprocess,"check_output",side_effect=[revision[:7],b"runtime licenses"]), \
                     patch.object(packaging.urllib.request,"urlopen",side_effect=lambda *a,**k:io.BytesIO(source.getvalue())):
                    packaging.appimage_runtime(work,work/"AppDir",architecture)
                self.assertEqual([call.args[1] for call in assets.call_args_list],["appimagetool-"+architecture+".AppImage","runtime-"+architecture])
                record=work/"AppDir/usr/lib/libretoon/corresponding-source/appimage-runtime/BUILD-IDENTITY.json"
                self.assertEqual(json.loads(record.read_text())["architecture"],architecture)


if __name__=="__main__":
    unittest.main(verbosity=2)
