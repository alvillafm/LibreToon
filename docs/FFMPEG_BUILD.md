# Build and distribute LibreToon's FFmpeg

Implemented 2 November 2026. The former Gyan build is no longer the bundled executable. FFmpeg remains an external process and LibreToon's own code remains MIT.

## Locked implementation

scripts/ffmpeg/sources.lock.json fixes all twelve input archives by SHA-256: FFmpeg 7.1.5, x264, zlib, LLVM/compiler-rt, MinGW-w64, llvm-mingw build recipes and the portable build tools. No globally installed compiler/MSYS2 is required. Use PowerShell 7 on Windows x64 and a checkout/build directory without spaces.

The toolchain ZIP is checked against its fixed hash, and every extracted toolchain file is checked against the original ZIP. Compiler, shell, make, NASM and pkgconf versions are retained. The compiler is prebuilt and hash-locked; this is not an independent bootstrap/rebuild of LLVM itself. Published toolchain recipes identify its LLVM/MinGW revisions, and full runtime sources/notices accompany the artifact.

The build uses native Windows threads, assembly optimisations and explicit libx264/zlib dependencies. Its selected codecs, demuxers, muxers, filters and parsers preserve the current export contract, including animated GIF parsing and PNG support. It is an application-focused FFmpeg build, not a promise of all unrelated capabilities of Gyan's general-purpose package.

No FFmpeg/x264/zlib source patches are applied. zlib.pc and normal generated configuration are retained. Absolute paths in generated FFmpeg build-description metadata are normalised to /build. PE timestamps are suppressed. trace-link.sh repeats linking with diagnostics only; its result must be byte-identical.

## Rebuild and validate

From the repository root:

```powershell
./scripts/build-ffmpeg.ps1 -WorkDirectory work/ffmpeg-owned-final
./scripts/build-ffmpeg.ps1 -WorkDirectory work/ffmpeg-reproduced -ToolDirectory work/ffmpeg-owned-final/tools
./scripts/test-ffmpeg.ps1 -CandidateDirectory work/ffmpeg-owned-final/output
./scripts/test-ffmpeg-app.ps1 -CandidateDirectory work/ffmpeg-owned-final/output
./scripts/review-ffmpeg.ps1 -CandidateDirectory work/ffmpeg-owned-final/output -ReproducedDirectory work/ffmpeg-reproduced/output
./scripts/check-ffmpeg-distribution.ps1 -Destination dist -ManifestPath work/ffmpeg-owned-final/output/SOURCE_STATUS.json -BinaryPath work/ffmpeg-owned-final/output/ffmpeg.exe
```

Use fresh WorkDirectory paths: existing compilation directories are rejected and preserved. DownloadDirectory may be shared; all cached archives are hash-checked. No script silently replaces assets/tools/ffmpeg.exe. Source finalisation retains previous generated ZIPs rather than deleting them.

The build creates ffmpeg.exe, ffprobe.exe (validation tool), ffmpeg-complete-source.zip, notices and a build receipt. Source finalisation exposes compiler-rt's original license/CREDITS and libunwind notices and retains the complete source archives. The review requires the separate export, application and reproduction evidence; build success alone does not approve distribution.

The source ZIP is self-contained for source inputs and build recipes. To rebuild it independently, extract recipe/ into a writable directory, copy sources/* into work/ffmpeg-downloads/, then run scripts/build-ffmpeg.ps1. Missing portable build tools are obtained from their locked official URLs. Application-integration tests additionally require the LibreToon checkout; LibreToon's code is not a library incorporated in FFmpeg.

## Validation performed locally

- Two independent fresh-directory builds: identical ffmpeg.exe and ffprobe.exe hashes. A subsequent full execution of the final build/package orchestrator in a third fresh directory completed successfully and produced the same hashes; see assets/tools/PIPELINE_VALIDATION.json.
- FFmpeg SHA-256: 48d3a36fdf33cd0027fa87806287f92149fa29f0ab60c5465bc04c2468556055.
- FFprobe SHA-256: f88b9ae20f3624b90189a8d8bc08728e4284ea7f6f9d365c119567b3a2296b3a.
- Ten format/fixture comparisons against the former Gyan executable: MP4, MKV, AVI, MOV and GIF, with 193×109 input (padding where appropriate) and Full HD input. All passed.
- Checked decoded RGB frame counts/order/primary colours, stream dimensions and timing, two x264 CRF settings, GIF palette/dither/loop settings, and MP4 faststart. MSE/PSNR measurements matched the baseline for these fixtures; this is not a claim of identical quality for every possible project.
- The isolated existing LibreToon export integration test passed; it also passed using the promoted executable at its normal bundled path.
- cargo test --locked: 189 passed, 11 ignored. The export test was subsequently run explicitly in release mode: 1 passed.
- cargo fmt --all -- --check passed.
- Source review checked 25 archive inputs, all covered by the six supplied source/recipe archives, and 17 imports belonging to normal Windows system DLLs.
- Distribution gate passed the complete artifact and rejected an incomplete legacy manifest, a changed binary hash and a forged approved status backed by an empty source ZIP.

Tests detected and corrected a missing GIF parser during implementation. Portable tool-name and ZIP-entry timestamp issues were also corrected. Only passing final outputs were promoted; the original Gyan executable and metadata were preserved.

## Actions and packaging

.github/workflows/windows.yml builds twice, runs quality/application tests and source review, stages the candidate, invokes the legal gate, and only then uploads LibreToon-Windows-x64. There is no intermediate upload of an unreviewed FFmpeg binary. Rust regression/format checks remain.

CI does not mutate the checked-in fallback. It stages its newly reviewed executable, notices and evidence in dist/assets/tools and includes the corresponding-source ZIP in the SAME artifact. No user installation, runtime download, exporter rewrite or MIT license change is involved.

Actions has been edited and its underlying scripts exercised locally, but no remote workflow run or publication was initiated in this task. The final Actions artifact must be checked on its first remote run. Do not publish binaries separately from their matching source/notices, and preserve source access for the period in which each binary is distributed.

## Scope and independent pending items

scripts/ffmpeg/POLICY.md defines the source/link/import review. reviewed-complete records the checks performed for an exact binary/source pair, not a legal opinion, bootstrapped compiler certification or exhaustive artwork/codec-patent review.

This work covers the new owned binary. Historical distributions of the Gyan executable retain their separate unresolved corresponding-source status; the new source package must not be presented as their source.

The separate Rust dispatch 0.2.0 original-copyright documentation limit remains recorded in assets/licenses/supplemental/. It is not silently declared resolved by the FFmpeg work. AI-assisted asset provenance and third-party font notices remain unchanged.
