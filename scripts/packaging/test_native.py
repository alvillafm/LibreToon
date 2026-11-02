"""Execute Linux package staging with mocked packaging tools; no network or builds."""
import ast
import importlib.util
import io
import os
from pathlib import Path
import shutil
import sys
import tarfile
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
