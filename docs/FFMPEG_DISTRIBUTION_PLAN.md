# FFmpeg distribution: follow-up investigation and concrete proposal

Date: 2 November 2026. This is the historical investigation/proposal. **The user subsequently approved implementation; see [FFMPEG_BUILD.md](FFMPEG_BUILD.md) for the completed build, verified evidence and remaining limits.** Statements below describing the then-current Gyan binary or future work are retained as investigation history.

## 1. Gyan: what is and is not publicly available

The executable remains SHA-256 `2ce797a0f88d7f067180338fb227f7b1928ea727bd9a4d7a1d022f7c52af71a3`. Earlier verification matched it byte for byte to Gyan 7.1 essentials and the imageio-ffmpeg 0.7.0 Windows wheel.

Rechecked:
- [Gyan's exact 7.1 release](https://github.com/GyanD/codexffmpeg/releases/tag/7.1) lists six binary archives (essentials/full/full-shared, ZIP/7z), with no separate complete source/dependency/build archive.
- The previously downloaded ZIP contains the builder README and license, not the complete corresponding-source set. Its reference to FFmpeg commit b08d7969c550a804a59511c7b83f2dd8cc0499b8 identifies the core, not every statically incorporated component.
- [The public support repository](https://github.com/GyanD/codexffmpeg) currently contains only README.md and .github/FUNDING.yml, not the build scripts and dependency sources.
- [The builds page](https://www.gyan.dev/ffmpeg/builds/) links FFmpeg core source and describes builds, dependencies and contact, but the inspected page did not expose a matching full source bundle for our hash.
- In [issue 246](https://github.com/GyanD/codexffmpeg/issues/246#issuecomment-6141911951), concerning a DIFFERENT version, Gyan asks for the request from an official email address. [Issue 249](https://github.com/GyanD/codexffmpeg/issues/249#issuecomment-6264377917) similarly requests an official email and app/organisation link. Those responses do not prove availability or source completeness for our 7.1 executable.

**Finding:** no publicly downloadable complete matching bundle was located in these primary sources. This is not a claim that Gyan has no private archive or cannot provide it. The next route for keeping this exact binary is a request by the project owner to the official contact on the builds page (builds at gyan.dev), specifying the hash and requesting all exact dependency/transitive sources, applied patches, compilation/install scripts, configuration and notices. No email or issue has been sent on the user's behalf. We cannot promise recipients a complete source set we do not possess.

## 2. Alternative investigated: BtbN

[FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) publishes Windows GPL builds and a public Docker build system, dependency recipes and patches. Inspected recipe revision:
`5ece62897c650b863c1ddda7b6964845344aef2d`.

The recipe repository's own LICENSE is MIT (Copyright 2020–2021 BtbN); this does NOT make its FFmpeg executables MIT. The x264 recipe fixes source commit 0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee. build.sh shows how it configures/builds/packages FFmpeg. download.sh collects dependency source downloads, while the workflow caches/uploads downloads as build artifacts.

However, the inspected public release assets expose binary archives and checksums, not a retained complete source bundle tied to each binary. build.sh follows a release branch/master unless overridden, and images normally use floating latest tags. A recipe checkout alone therefore is not proof of every input in an already-built ZIP. Release retention is finite. We should not rely on a latest URL or temporary Actions cache to satisfy long-term corresponding-source access.

A concrete prebuilt candidate exists in the inspected release `autobuild-2026-11-01-12-49`:
`ffmpeg-n8.1.3-29-gad3b795b72-win64-gpl-8.1.zip`.
Its existence was verified from the official release metadata, but it was NOT downloaded, executed or certified as having complete corresponding source. Swapping to this ZIP alone is **not** the recommendation.

Sources: [pinned README](https://github.com/BtbN/FFmpeg-Builds/blob/5ece62897c650b863c1ddda7b6964845344aef2d/README.md), [build.sh](https://github.com/BtbN/FFmpeg-Builds/blob/5ece62897c650b863c1ddda7b6964845344aef2d/build.sh), [x264 recipe](https://github.com/BtbN/FFmpeg-Builds/blob/5ece62897c650b863c1ddda7b6964845344aef2d/scripts.d/50-x264.sh), [release metadata](https://github.com/BtbN/FFmpeg-Builds/releases/tag/autobuild-2026-11-01-12-49).

## 3. Recommendation: our own pinned Windows x64 GPL build

Build FFmpeg **7.1.5** ourselves with **libx264** and **zlib**, using the public BtbN recipes as a documented reference rather than trusting an opaque prebuilt artifact. Keep FFmpeg a separate bundled executable. This retains the 7.1 release family and the existing codec choices, while limiting external libraries to those actually needed by the application's exports and PNG capabilities. Third-party source/runtime dependencies discovered during linking must still be added; “three source archives” is not yet a closed bill of materials.

Verified SOURCE inputs, downloaded from the official endpoints, with hashes recorded in [SOURCE_PROPOSAL.json](licensing/ffmpeg-candidate/SOURCE_PROPOSAL.json):

| Component | Exact revision | License evidence inspected |
|---|---|---|
| FFmpeg 7.1.5 | 3a0867c2bfda4a4d4309ca1a8cbdc6175e67f587 | LICENSE.md and COPYING files; GPL-3.0-or-later is the proposed --enable-gpl/--enable-version3 configuration. Upstream base code includes LGPL and other notices, which must be preserved. |
| x264 | 0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee | COPYING (GPLv2 text) and x264.h (“version 2 ... or ... any later version”); permits the proposed GPLv3 combination. This is the exact source revision pinned by the inspected BtbN recipe, not an inferred source for Gyan's older x264. |
| zlib 1.3.1 | Published fossil release archive | LICENSE from that archive, including its original copyright and permissive zlib conditions. Included to retain PNG-related capabilities as well. |

The source archives are retained in work/license-compliance/ as investigation evidence; source license copies are under docs/licensing/ffmpeg-candidate/. These are **candidate** notices, not claims about the currently shipped Gyan binary. The FFmpeg signed tag was resolved through the official GitHub API; GitHub reports its signature verified. No independent local GPG verification was performed. Download hashes are local evidence, not claims of independent signed upstream checksums.

Official inputs:
- [FFmpeg n7.1.5](https://github.com/FFmpeg/FFmpeg/releases/tag/n7.1.5), [pinned source archive](https://codeload.github.com/FFmpeg/FFmpeg/tar.gz/3a0867c2bfda4a4d4309ca1a8cbdc6175e67f587).
- [x264 pinned source archive](https://code.videolan.org/videolan/x264/-/archive/0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee/x264-0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee.tar.gz).
- [zlib 1.3.1 source archive](https://zlib.net/fossils/zlib-1.3.1.tar.gz).

The root LibreToon MIT license stays unchanged. GPL applies to the FFmpeg/x264 program combination. The integration remains an external program exchanging raster frames through pipes; the distribution is intended as aggregation, not linkage of GPL libraries into our Rust executable. This architecture must remain intact; a subprocess boundary alone does not certify every possible integration as legally independent.

## 4. Required export contract — no exporter rewrite proposed

Read directly from src/exporter.rs:

| Export | Current encoder/settings that the candidate must retain |
|---|---|
| MP4 | libx264, yuv420p, preset medium, existing user CRF; pad to even dimensions; +faststart. |
| MKV | Same libx264/preset/CRF/pixel-format/padding path. |
| AVI | Native mpeg4 encoder, -q:v 3; same padding. |
| MOV | Native prores_ks, profile 3 (422 HQ), yuv422p10le; same padding. |
| GIF | Native GIF encoder/muxer; split, palettegen, paletteuse, configured colours/dither/loops. |
| Input/support | rawvideo RGB24 over pipe; pad/format/scale and automatic colour conversions; decoders needed by export validation. Preserve native PNG export and zlib-backed PNG support. |

Maintaining the same encoders/settings is the basis for equivalent quality, **not a completed quality test**. x264 revisions differ, so neither identical bytes nor measured equivalence is claimed.

## 5. Technical changes needed AFTER approval — not made now

1. Create a separate reproducible builder (Linux/container cross-compilation to Windows x64 is suitable), with FFmpeg/x264/zlib revisions and download SHA-256 locked. Pin toolchain packages and container digests; do not use floating latest images/branches.
2. Use --disable-autodetect and explicitly enable libx264/zlib with --enable-gpl and --enable-version3. For x264 use a static library, disable its CLI and unused lavf/swscale/OpenCL integrations; retain Windows threading and assembly optimisations. Exact working configure flags and ABI compatibility must be established by an actual build, not copied from this proposal as if tested.
3. Retain an auditable link-input inventory. Determine actual MinGW-w64/CRT/GCC runtime or other transitive inputs, applicable exceptions and notices. Do not silently assume every toolchain/runtime component is exempt. The compiler as a normal build tool and a runtime copied into the executable are different roles.
4. Produce the executable and complete corresponding-source ZIP in the SAME build: exact sources, any modifications/patches (or an explicit no-patches record), configure/build/install scripts, generated inputs and applicable notices. Preserve source headers and FFmpeg's IJG acknowledgement. Include build logs/configuration, toolchain manifest and binary/source hashes.
5. Test the candidate from a separate path before replacing assets/tools/ffmpeg.exe. Run the existing export integration test, enumerate encoders/muxers/filters, validate timing/dimensions/frame count/order, GIF loops/colour handling, MP4 faststart and all quality options. Compare representative animation output against the existing binary for visual quality and performance. No such tests or build have been run in this investigation.
6. Only after approval and review replace the bundled executable at its existing path. No change should be needed in src/exporter.rs if the command contract above passes. Users still receive FFmpeg in the application bundle; no manual installation or runtime download is needed.
7. Keep scripts/check-ffmpeg-distribution.ps1 and the Actions check. Update SOURCE_STATUS.json ONLY to match the newly built binary and reviewed complete source bundle, with hashes/review evidence. The binary and corresponding-source ZIP must be downloadable together with no extra charge, preserving the source access for the period of binary distribution. Do not use a temporary CI artifact as the sole retained source archive for a separately published installer.
8. Inspect the final Windows artifact: root MIT LICENSE, separate FFmpeg and dependency notices, SOURCE-ACCESS.txt, complete source ZIP, unchanged export behaviour and no accidental proprietary/nonfree additions.

This makes the binary/source relationship controllable. It is still a plan, not a claim that distribution compliance has been resolved. **Current SOURCE_STATUS.json, the executable, exporter and Actions checks were deliberately left unchanged.**

## 6. Rust attribution follow-up

Ten original-notice gaps are now closed: eight egui-family entries using the official release plus a source-content comparison, and two r-efi versions using their packaged AUTHORS files.

dispatch 0.2.0 has an attribution record containing its original MIT/author declaration and an explicitly labelled canonical MIT template. Neither the original crate nor its inspected upstream revision supplies a standalone copyright notice. The author declaration is Steven Sheldon; no year or copyright-holder identity was fabricated. Recovery/confirmation of that missing original notice remains a documented limit, rather than being counted as an eleventh verified original notice.

See [the dependency inventory](../assets/licenses/RUST-DEPENDENCIES.md) and [supplemental records](../assets/licenses/supplemental/dispatch-0.2.0/ATTRIBUTION.md). No dependency versions or functional code were changed.
