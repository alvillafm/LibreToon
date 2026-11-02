# Native macOS/Linux FFmpeg distribution gate

This extends packaging to native FFmpeg 7.1.5 + x264 + zlib. It does not replace,
weaken or reuse a Windows approval for a different binary.

- Use only the three original source archives and SHA-256 values already in
  sources.lock.json. Keep unmodified archives, original notices, configuration,
  toolchain identity, link maps and exact native recipe with each binary.
- Disable autodetected/optional dependencies and assembly in this initial native
  recipe. This trades encoding speed for a smaller build surface; codec settings
  and output formats remain the same.
- Link maps are parsed as raw bytes (including Mach-O archive member indices), never
  decoded with a platform default text codec. Original bytes are retained in source logs.
- Link maps must contain x264, zlib and FFmpeg archives and no unknown archives.
  Ubuntu gcc-11 libgcc.a/libgcc_eh.a are covered only when their actual map paths match
  the compiler-selected archives, dpkg identifies their owning source package/version,
  and gcc-11-source has that exact source version. Keep the full installed source
  archive, Debian patches/build rules, symlinks, package identity, archive hashes,
  original COPYING3/COPYING.RUNTIME and Ubuntu copyright notice in the source ZIP.
  Missing sources or version/path mismatches still block distribution. Other compiler
  runtimes remain blocked; no wildcard exception is allowed.
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
