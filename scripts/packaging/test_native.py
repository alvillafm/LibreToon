"""Execute Linux package staging with mocked packaging tools; no network or builds."""
import ast
import importlib.util
from pathlib import Path
import sys
import tempfile
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

    def test_linux_stages_appimage_and_deb_without_ffmpeg_or_sources(self):
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
            def fake_run(args,**kwargs):
                calls.append(args)
                Path(args[-1]).write_bytes(b"package tool output fixture")
            def fake_symlink(path,target,*args,**kwargs):
                # Windows CI does not require developer-mode symlink privileges for this test.
                path.write_text(str(target))
            with patch.object(packaging,"ROOT",root), patch.object(packaging,"check_system_imports"), \
                 patch.object(packaging,"appimage_runtime",return_value=(Path("appimagetool-fixture"),Path("runtime-fixture"))), \
                 patch.object(packaging,"run",side_effect=fake_run),patch.object(Path,"symlink_to",new=fake_symlink):
                packaging.linux(destination)
            self.assertEqual(len(calls),2)
            self.assertTrue(any(p.suffix==".AppImage" for p in destination.iterdir()))
            self.assertTrue(any(p.suffix==".deb" for p in destination.iterdir()))
            self.assertTrue(any(p.name.endswith("-AppImage.tar.gz") for p in destination.iterdir()))
            self.assertFalse((root/"work/packages/linux/LibreToon.AppDir/usr/lib/libretoon/assets/tools").exists())
            self.assertFalse((root/"work/packages/linux/deb/opt/libretoon/assets/tools").exists())


if __name__=="__main__":
    unittest.main(verbosity=2)
