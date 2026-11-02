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
