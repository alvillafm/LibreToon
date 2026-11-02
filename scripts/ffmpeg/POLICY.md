# FFmpeg build/source review policy (version 1)

LibreToon's own build scripts and application code remain MIT. FFmpeg is an external executable under GPLv3-or-later, not a library linked into LibreToon.

The locked source inputs cover FFmpeg 7.1.5, x264, zlib, LLVM 18.1.8/compiler-rt, MinGW-w64 revision 7c9cfe6708cafc83c14a2654308b2db62b126eae and the llvm-mingw 20240619 build recipes. The official toolchain binary is pinned and every extracted file is checked against that ZIP. Its clang version reports LLVM commit 3b5b5c1ec4a3095ab096dd780e84d7ab81f3d7ff. The builder's recipe pins the matching LLVM and MinGW revisions. Full runtime sources and those build scripts travel in the corresponding-source ZIP.

Static link coverage:
- libav*.a / libsw*.a / libpostproc.a: FFmpeg source archive.
- libx264.a: x264 source archive.
- libz.a: zlib source archive.
- libclang_rt.builtins-x86_64.a: compiler-rt in the full LLVM archive; Apache-2.0 with LLVM exception and retained additional notices.
- libunwind.dll.a: LLVM source archive; read by the linker but no libunwind DLL import is allowed in the final PE.
- MinGW CRT startup .o, libm.a, libmingw32.a, libmingwex.a and libmoldname.a: full MinGW-w64 archive, including its mixed original notices.
- Windows import libraries: MinGW-w64 archive; dynamically resolved DLLs are ordinary Windows system components, not copied into the distribution. UCRT remains the OS prerequisite already supplied by Windows 10/11.
Unexpected libraries or non-system imports block review. No compiler, MSYS2 runtime, winpthreads DLL or downloaded runtime installer is shipped with LibreToon.

Build tools alone are normal compilation tools under the GPLv3 definition of Corresponding Source. Their locked binaries/URLs and configuration are recorded; executable build-tool packages are not redistributed in the source ZIP. LLVM and MinGW sources ARE included because portions of their runtimes/startup code can be incorporated. No hidden assumption that all toolchain code is excluded is made.

The source ZIP retains complete unmodified input archives, all notices/headers, the actual build/install scripts and logs. zlib.pc is generated from its template; generated FFmpeg config.h build-description paths are normalised to /build. These generated-file operations are in build.sh. No project source patches are applied to FFmpeg, x264 or zlib. Upstream toolchain recipes and any patches they contain are preserved as their full archive.

Approval conditions checked by review-ffmpeg.ps1:
1. Locked archive hashes, matching build receipt and actual executable.
2. Independent fresh-directory build produces byte-identical ffmpeg.exe AND ffprobe.exe.
3. Actual static link maps and imports match the source coverage above.
4. Five formats pass the declared export contract and quality comparison against the bundled baseline, using odd-size and Full HD fixtures.
5. LibreToon's existing export integration test passes with the candidate in an isolated directory.
6. Complete source ZIP and notices are included with the binary, with source access instructions.

"reviewed-complete" records these checks for this binary and this source package. It is not a legal opinion, a guarantee of every contributor's rights or a claim of exhaustive visual equivalence. The Rust dispatch copyright-documentation limit remains separate. A change in source inputs, recipe or runtime/link dependencies requires another review. No distribution gate may be bypassed because a build merely succeeds.

trace-link.sh repeats linking with diagnostics only (no source compilation); the byte-identical result is checked. Link maps plus ld.lld Reading traces record both potential archive inputs and actually selected object members. Original compiler-rt, CREDITS and libunwind notices are exposed separately in addition to their complete source archives.
