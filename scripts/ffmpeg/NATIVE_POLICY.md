# Native macOS/Linux FFmpeg distribution gate

This extends packaging to native FFmpeg 7.1.5 + x264 + zlib. It does not replace,
weaken or reuse a Windows approval for a different binary.

- Use only the three original source archives and SHA-256 values already in
  sources.lock.json. Keep unmodified archives, original notices, configuration,
  toolchain identity, link maps and exact native recipe with each binary.
- Disable autodetected/optional dependencies and assembly in this initial native
  recipe. This trades encoding speed for a smaller build surface; codec settings
  and output formats remain the same.
- Link maps must contain x264, zlib and FFmpeg archives and no unknown archives.
  An incorporated compiler-runtime archive blocks packaging until its source and
  license coverage is explicitly added. Do not add wildcard exceptions.
- Dynamic imports must be ordinary OS libraries, not Homebrew/runner libraries.
  These OS components are prerequisites and are not copied into the package.
- Two fresh native builds on the same runner must be byte-identical. This does
  not certify reproducibility across different Xcode/GCC/SDK versions.
- Require all ten odd-size/Full HD export fixture cases, decoded colors/order,
  dimensions, timing, palette, loop, faststart and quality checks, plus LibreToon's
  isolated application export test. Native quality comparisons use the independently
  rebuilt native binary, not the Windows executable (which cannot run natively).
- Stage only after the native review binds the binary, source ZIP, receipts,
  current exporter and test scripts. No unreviewed FFmpeg artifacts are uploaded.
- Toolchains are OS build prerequisites, recorded rather than redistributed.
  Complete FFmpeg/x264/zlib sources and original notices travel in every package.
  Unexpected linked code fails closed instead of being declared covered.

No native build has been run while preparing this workflow. First-run gate or
reproduction failures must be investigated; they are not permission to skip gates.
This evidence is limited to FFmpeg source/configuration and declared tests, not
an unrestricted legal certification of the entire application or platform.
