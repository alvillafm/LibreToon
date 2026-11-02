# Bundled FFmpeg — owned, source-locked build

Updated 2 November 2026. Current executable: **FFmpeg 7.1.5-LibreToon-1**, Windows x64, GPL version 3 or later. LibreToon's own code remains MIT; it invokes FFmpeg as a separate process through pipes and does not link its libraries.

- Binary SHA-256: 48d3a36fdf33cd0027fa87806287f92149fa29f0ab60c5465bc04c2468556055.
- Size: 7,582,208 bytes.
- FFmpeg source: 3a0867c2bfda4a4d4309ca1a8cbdc6175e67f587 (7.1.5).
- x264 source: 0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee, GPLv2-or-later, used in the GPLv3-or-later combination.
- zlib source: 1.3.1, original zlib license.
- Toolchain: llvm-mingw 20240619 UCRT, clang 18.1.8. Incorporated runtime/startup/import-library source coverage includes LLVM/compiler-rt and MinGW-w64.
- The exact download URLs, SHA-256 values and roles are in [the input lock](../../scripts/ffmpeg/sources.lock.json). [BINARY_METADATA.txt](BINARY_METADATA.txt) records version/configuration and the runtime license notice.
- [BUILD_EVIDENCE.json](BUILD_EVIDENCE.json) records the 25 archive inputs, their source coverage, 17 normal Windows system DLL imports, reproducibility and validation hashes. No extra DLL installation is required on supported Windows 10/11 systems.

The root MIT license does not cover this FFmpeg/x264 combination. Original license texts are in [licenses/](licenses/), including GPL, LGPL, zlib, x264, LLVM/compiler-rt, libunwind and MinGW notices. Additional per-file notices remain in the complete original source archives supplied with the distribution.

**This software is based in part on the work of the Independent JPEG Group.** No LibreToon source patches were applied to FFmpeg's libjpeg-derived files. Full source archives are preserved byte-for-byte. Generated configuration/build-description path normalisation is documented in the build recipe.

## Corresponding source and redistribution

Use the Windows Actions artifact or the documented packaging check. The downloadable package must contain:
- the exact executable;
- these original licenses/notices;
- corresponding-source/ffmpeg-complete-source.zip and review/validation records;
- SOURCE-ACCESS.txt.

The source ZIP contains full FFmpeg, x264, zlib, LLVM, MinGW-w64 and llvm-mingw recipe archives, the actual build/install scripts, generated configuration, link maps/traces and build receipts. Sources and object code are supplied together, with no extra charge. If the binary is published elsewhere, the matching source must remain available alongside it while that binary is offered. Do not publish a standalone executable with only a generic upstream link.

[SOURCE_STATUS.json](SOURCE_STATUS.json) describes the verified local build. Its work/ paths are checkout evidence paths, not public download URLs. A fresh checkout must regenerate and review the source package using [the build guide](../../docs/FFMPEG_BUILD.md); CI does this automatically and packages its own matching manifest. CI-generated archive hashes can differ because logs contain checkout paths; the executable was reproduced byte-for-byte in two different clean directories.

The distribution gate remains active and checks source hashes, source ZIP contents, matching binary, recipe/policy, reproduction and both export validations before upload. A status flag alone is insufficient. These checks document the stated build/source obligations; they are not a legal originality certificate or a guarantee about every possible redistribution context.

## Historical Imageio/Gyan files

The previous 7.1 Gyan executable was supplied through imageio-ffmpeg 0.7.0. It was replaced only after the owned build passed the checks. A full local copy is retained under work/ffmpeg-legacy-backup-1793613143638/tools/.

The retained Imageio LICENSE/METADATA/README and GYAN-7.1-README.txt document that earlier acquisition. Imageio's BSD-2-Clause license covers its wrapper/distribution materials, not the GPL FFmpeg program. The current executable was built from the locked sources above, not obtained from Imageio or Gyan. Historical notices are not a new license for it.
