# Bundled FFmpeg

`ffmpeg.exe` is the Windows x86_64 FFmpeg 7.1 essentials static build from Gyan, distributed in the `imageio-ffmpeg` 0.7.0 PyPI wheel (`ffmpeg-win-x86_64-v7.1.exe`). It runs as a separate process. LibreToon's Rust code does not link FFmpeg.

- FFmpeg: https://ffmpeg.org/
- Upstream source release: https://ffmpeg.org/releases/ffmpeg-7.1.tar.xz
- Binary builder and build/source information: https://www.gyan.dev/ffmpeg/builds/
- Imageio distribution: https://github.com/imageio/imageio-ffmpeg
- Exact wheel: https://pypi.org/project/imageio-ffmpeg/0.7.0/
- FFmpeg build: GPL version 3 or later (`--enable-gpl --enable-version3`). Run `ffmpeg.exe -L` for its license notice and `ffmpeg.exe -version` for the configuration. The `LICENSE` file here is the BSD license of the Imageio wrapper/distribution, not the license of FFmpeg.
- GPLv3 text: https://www.gnu.org/licenses/gpl-3.0.html

Retain the applicable notices and comply with FFmpeg and included libraries' source-distribution obligations when redistributing this binary. LibreToon's own license remains in the repository root.
