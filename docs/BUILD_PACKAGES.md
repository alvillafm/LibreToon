# Manual lightweight packages

Workflow: .github/workflows/windows.yml, **LibreToon manual packages**.
Only workflow_dispatch is enabled. Pushes, pull requests and tags do not build anything.
No release is created or published automatically. Repository permission is read-only.

After the maintainer commits/pushes the configuration, open GitHub > Actions >
LibreToon manual packages > Run workflow. Select all, windows, macos or linux.
Download the resulting artifacts from the run page. Retention: 30 days.

| Artifact | Packages |
| --- | --- |
| LibreToon-Windows-x64 | Portable ZIP, per-user EXE installer |
| LibreToon-macOS-arm64 | ZIP containing .app, drag-install DMG |
| LibreToon-macOS-x64 | ZIP and DMG for Intel |
| LibreToon-Linux-x64 | AppImage, permission-preserving AppImage.tar.gz, DEB |

Packages include LibreToon, icons, fonts/locales and original application-resource notices.
The embedded CJK font is not duplicated as a loose file.
**FFmpeg/ffprobe executables and FFmpeg source archives are excluded.**
Packaging checks reject accidental inclusion. Application builds/tests do not require an
FFmpeg build, toolchain source package, native link-map review or network installation.
Historical FFmpeg source/reproduction scripts and gates remain in the repository, dormant.

Windows targets Windows 10 1809 or later. macOS targets macOS 14+ and is ad-hoc signed,
not Developer-ID signed or Apple-notarized. Linux targets a normal Ubuntu 22.04+ x64
desktop (glibc 2.35 baseline); graphics drivers/X11/Wayland remain OS prerequisites.
DEB declares curl/coreutils for optional downloading; macOS and Windows use OS-supplied
curl and hash tools. If a downloader is unavailable, manual FFmpeg selection remains usable.
Use the AppImage tar.gz to retain executable permissions through Actions downloads.
AppImage runtime/tool source/notices remain distinct from FFmpeg and are retained as before.

## First video/GIF export

Without a configured external executable, LibreToon asks permission to download and
configure FFmpeg. Users can instead select an executable or its folder, or postpone.
SVG export does not require FFmpeg.

With consent, one background worker downloads directly from Imageio's PyPI distribution,
checks the pinned size and SHA-256, extracts only the executable/notices into a private
transaction directory, verifies encoder/muxer/filter capabilities, and then installs it
in the user's LibreToon data folder. No Python install, PATH changes, administrator
rights or executable-directory writes are needed. Progress and cancellation are provided.
Partial/failed transactions are cleaned up; existing installations are preserved.
Preferences retain the configured path. Normal installer/portable downloads stay small.

Pinned providers: assets/ffmpeg-downloads.json. These are upstream wheels, not LibreToon's
historical source-built executable. Version, provider notices, reported FFmpeg license and
download origin are retained beside the downloaded tool. No FFmpeg binary is redistributed
by the installer workflow; no claim is made that old source-access limitations are solved.

No hosted run or push is initiated by editing this configuration. The first hosted
packages still need installation checks on the target operating systems.

## Native packaging and text-test fixes

Linux and macOS use the same one-argument common(payload) staging function.
A no-network regression executes the Linux AppImage/DEB staging path with mocked
packaging tools and checks that assets/tools is excluded. It runs before Rust tests.
This catches stale references left over from bundled-FFmpeg packaging.

CI text tests use the existing licensed Noto Sans CJK collection through a test-only
LIBRETOON_TEST_FONT setting. Production font discovery now scans the actual macOS
system/user font directories, recursively finds Linux/user fonts, and supports TTC
collections as well as TTF/OTF. Tests are not skipped when a runner lacks extra fonts.
The step is simply named Run application regression tests; it never downloads FFmpeg.

## Export quality

The visible quality scale is now 1–100 (default 95), with a Maximum button.
For MP4/MKV, 1–99 maps monotonically to H.264 CRF 40–10; 100 uses libx264rgb,
RGB24 and CRF 0 to avoid both lossy compression and chroma subsampling.
This lossless H.264 profile creates larger files and is not supported by all players.
Use 99 for broadly compatible yuv420p output.

AVI maps the scale to MPEG-4 quantizer 31–1. MOV maps it to the ProRes quantizer and,
at 100, selects ProRes 4444 XQ with full 4:4:4 chroma. These formats do not promise
exact RGB round trips. GIF remains palette-limited (up to 256 colors); SVG is vector.
Maximum MP4/MKV is regression-tested by decoding two frames of one-pixel colored
edges and comparing every RGB byte with LibreToon's renderer.

On Linux, LibreToon uses XInput2 for pen pressure. In a Wayland session it
selects XWayland automatically when an XInput2 pressure axis is detected at
startup. Connect the tablet before opening LibreToon. To choose the window
backend explicitly, launch with `LIBRETOON_LINUX_BACKEND=x11` or
`LIBRETOON_LINUX_BACKEND=wayland`. The X11 choice needs a working X11 or
XWayland display. Native Wayland currently supports ordinary pointer input;
tablet pressure depends on the XInput2 path.
