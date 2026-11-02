# Manual Windows, macOS and Linux packages

The only workflow is .github/workflows/windows.yml, displayed as **LibreToon manual packages**.
Its only event is workflow_dispatch: no push, pull request, schedule, tag or release event.
It has read-only repository permission and no release creation/upload step.

After these changes are committed and pushed **by the maintainer**, open GitHub > Actions >
LibreToon manual packages > Run workflow. Choose all, windows, macos or linux.
A workflow must be present on the default branch for its manual Run workflow button to appear.
This task did not push, dispatch a workflow, compile, install build dependencies or create a release.

## Download outputs

Successful runs expose artifacts for 30 days (subject to the repository's retention policy):

| Artifact | Contents |
| --- | --- |
| LibreToon-Windows-x64 | Portable ZIP, per-user Inno Setup EXE, SHA256SUMS.txt |
| LibreToon-macOS-arm64 | Portable ZIP containing LibreToon.app, drag-to-Applications DMG, checksums |
| LibreToon-macOS-x64 | Same formats for Intel |
| LibreToon-Linux-x64 | AppImage, permission-preserving AppImage.tar.gz, DEB, checksums |

Windows installs under the current user's LocalAppData/Programs; no administrator prompt.
macOS packages target macOS 14+. Intel and Apple Silicon have separate downloads.
The macOS app is ad-hoc signed, **not Developer ID signed/notarized**. Apple security
prompts are expected until a separate signing/notarization task is configured.
No certificates, signing secrets or release credentials are required for these test packages.
The DMG is a normal drag-install image, not a PKG installer.

Linux builds on Ubuntu 22.04 x64 (glibc 2.35 baseline). The DEB declares desktop/graphics
dependencies and recommends desktop portal services for file dialogs. AppImage expects
a normal Linux desktop, OpenGL/EGL drivers, X11 or Wayland libraries; it does not replace
the operating system or bundle graphics drivers/glibc. FFmpeg requires no manual install.
Actions can remove the executable bit on a loose AppImage download: use the supplied
tar.gz, which preserves it, or chmod +x the AppImage. Modern runtime supports extract-and-run
where FUSE mounts are unavailable.

## Included resources and source access

Every format contains the executable, assets (including fonts/locales/icon resources),
MIT LICENSE, original third-party notices and a separately executed native FFmpeg.
The large embedded CJK font is not duplicated outside the executable; its notices remain.

Complete matching FFmpeg source ZIP, build recipes, input archives, notices, configuration
and test evidence are **inside every portable/installer package**, not merely another
expiring link. See SOURCE-ACCESS.txt beside the executable (inside Contents/MacOS on Mac).
On the DMG a visible FFmpeg Sources shortcut also exposes the directory.
If artifacts are redistributed elsewhere, keep their matching source and notices together.

AppImage packaging uses official appimagetool 1.9.1. The runtime release asset is checked
against GitHub's published SHA-256 and matched to its source revision; packaging fails if
that binding cannot be established. Its source snapshot, original LICENSE and upstream
runtime help/license references are retained under corresponding-source/appimage-runtime.
The runtime snapshot is distinct from FFmpeg's reviewed complete corresponding source.
This workflow does not certify the licensing of unreviewed changes in upstream runtime
dependencies; review upstream's build/dependency notices before public redistribution.

## Gates and first-run limitations

Windows retains both clean FFmpeg builds, the ten format/quality fixtures, application
export integration, source/link/import review and check-ffmpeg-distribution.ps1.
The checked-in fallback FFmpeg is never replaced by CI.

macOS/Linux build the same official hash-locked FFmpeg 7.1.5/x264/zlib inputs natively.
scripts/ffmpeg/native.py and NATIVE_POLICY.md add a separate, fail-closed source,
link-import, reproduction and export gate for those binaries. C-only native codecs avoid
optional assembly/tool inputs, at a possible encoding-speed cost. Native fixture quality
is compared with the independent native rebuild, not the Windows-only baseline.
Any unexpected static compiler/runtime library blocks the job until explicitly covered.

Rust formatting, regression tests, release builds and export integration execute only
when the workflow is manually run. Packages upload only after gates and packaging pass.
There are no uploads of intermediate/unreviewed FFmpeg executables.

These changes received static checks only. Native builds, actual installer execution,
the hosted runner environment, SDK/toolchain reproduction and download installation
remain to be validated in the first explicitly requested Actions run. If a gate fails,
fix the evidence/build, never bypass it to make an artifact appear.

Official references:
- https://docs.github.com/actions/managing-workflow-runs/manually-running-a-workflow
- https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts
- https://jrsoftware.org/ishelp/topic_compilercmdline.htm
- https://github.com/AppImage/appimagetool
- https://github.com/AppImage/type2-runtime

## First hosted-run fixes

- zlib 1.3.1: the official maintainer release asset and zlib.net fossil were downloaded
  and compared byte-for-byte (1,512,791 bytes, 290 TAR members, zlib.h version 1.3.1).
  Both match the **unchanged** locked SHA-256
  9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23.
  The maintainer's release asset is now primary, with the verified fossil as fallback.
  Downloaders request identity encoding, preserve rejected files/cache entries, and
  require the same hash and size on every origin. Windows disables automatic HTTP
  decompression. Receipt logs record actual/expected digests and HTTP metadata.
  The original failed Windows response was not provided and the old error omitted
  its actual checksum, so HTML/error responses versus transport transformations
  cannot be distinguished retrospectively; no such cause is asserted as proven.
- macOS: raw-byte map parsing avoids UTF-8 decoding failures and accepts the actual
  Mach-O archive/member index notation. Original maps remain unchanged in source logs.
- Ubuntu: use gcc-11 and the matching gcc-11-source/libgcc-11-dev packages.
  Incorporation of libgcc.a/libgcc_eh.a requires exact source-version and link-path
  checks. Complete source/patch/build-rule files and original GPL/runtime-exception
  notices are retained and checksum-bound, rather than adding a blind allow-list.
- Six isolated downloader/parser/source-identity regressions run before heavy builds.
  They do not compile or run FFmpeg/LibreToon. Only these script tests and static checks
  were run for this correction; no new hosted or application compilation was started.
- Failed jobs expose only diagnostic logs/receipts for seven days, never unreviewed
  executables. Application-package uploads still require all original source/export
  and independent reproduction gates. workflow_dispatch remains the only trigger.

Official records: https://github.com/madler/zlib/releases/tag/v1.3.1,
https://zlib.net/fossils/, https://packages.ubuntu.com/jammy-updates/gcc-11-source,
https://gcc.gnu.org/onlinedocs/libstdc++/manual/license.html.
