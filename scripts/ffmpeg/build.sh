#!/usr/bin/env bash
# LibreToon's build recipe; MIT, as declared in the repository root LICENSE.
set -euo pipefail
export PATH=/usr/bin:$PATH
export LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=1718755200
umask 022
WORK="$(cygpath -u "${FFMPEG_BUILD_ROOT:?set FFMPEG_BUILD_ROOT}")"
TOOLS="$(cygpath -u "${FFMPEG_TOOL_ROOT:-$FFMPEG_BUILD_ROOT/tools}")"
LLVM="$TOOLS/llvm-mingw-20240619-ucrt-x86_64"
export PATH="$LLVM/bin:$TOOLS/nasm-2.16.03:/usr/bin"
export PKG_CONFIG_LIBDIR="$WORK/prefix/lib/pkgconfig"
export PKG_CONFIG_PATH=
PREFIX="$WORK/prefix"
mkdir -p "$PREFIX/lib/pkgconfig" "$PREFIX/include" "$WORK/logs" "$WORK/build-x264" "$WORK/build-ffmpeg" "$WORK/output"
MAP="$(cygpath -m "$WORK")"
CFLAGS="-O2 -ffile-prefix-map=$MAP=/build -fdebug-prefix-map=$MAP=/build -fno-ident"
LDFLAGS="-Wl,--no-insert-timestamp"
JOBS="${FFMPEG_BUILD_JOBS:-4}"
CC=x86_64-w64-mingw32-clang
AR=x86_64-w64-mingw32-ar
RANLIB=x86_64-w64-mingw32-ranlib
STRIP=x86_64-w64-mingw32-strip
{
  "$CC" --version
  make --version | head -1
  nasm -v
  pkgconf --version
} > "$WORK/logs/tool-versions.txt"
(
 cd "$WORK/src/zlib"
 make -f win32/Makefile.gcc -j"$JOBS" libz.a CC="$CC" AR="$AR" CFLAGS="$CFLAGS" 2>&1
 cp libz.a "$PREFIX/lib/"
 cp zlib.h zconf.h "$PREFIX/include/"
 # zlib's upstream pkg-config template; substitute installation locations only.
 sed -e "s|@prefix@|$PREFIX|g" -e 's|@exec_prefix@|${prefix}|g' -e 's|@libdir@|${exec_prefix}/lib|g' -e 's|@sharedlibdir@|${libdir}|g' -e 's|@includedir@|${prefix}/include|g' -e 's|@VERSION@|1.3.1|g' zlib.pc.in > "$PREFIX/lib/pkgconfig/zlib.pc"
) > "$WORK/logs/zlib-build.txt" 2>&1
(
 cd "$WORK/build-x264"
 CC="$CC" AR="$AR" RANLIB="$RANLIB" STRIP="$STRIP" "$WORK/src/x264/configure" --host=x86_64-w64-mingw32 --prefix="$PREFIX" --enable-static --disable-cli --disable-opencl --disable-lavf --disable-swscale --extra-cflags="$CFLAGS" --extra-ldflags="$LDFLAGS"
 make -j"$JOBS"
 make install
) > "$WORK/logs/x264-build.txt" 2>&1
cp "$WORK/build-x264/config.mak" "$WORK/logs/x264-config.mak"
cp "$WORK/build-x264/config.h" "$WORK/logs/x264-config.h"
cp "$WORK/prefix/lib/pkgconfig/zlib.pc" "$WORK/logs/zlib.pc"
(
 cd "$WORK/build-ffmpeg"
 "$WORK/src/ffmpeg/configure" --prefix="$PREFIX" --target-os=mingw32 --arch=x86_64 --enable-cross-compile --cc="$CC" --cxx=x86_64-w64-mingw32-clang++ --ar="$AR" --ranlib="$RANLIB" --nm=x86_64-w64-mingw32-nm --strip="$STRIP" --pkg-config=pkgconf --pkg-config-flags=--static --enable-gpl --enable-version3 --enable-libx264 --enable-zlib --enable-static --disable-shared --disable-autodetect --disable-debug --disable-doc --disable-ffplay --disable-everything --enable-protocol=file,pipe --enable-demuxer=rawvideo,mov,matroska,avi,gif,image2 --enable-muxer=mp4,matroska,avi,mov,gif,image2,rawvideo,null --enable-encoder=libx264,mpeg4,prores_ks,gif,png,rawvideo --enable-decoder=h264,mpeg4,prores,gif,png,rawvideo --enable-parser=h264,mpeg4video,gif --enable-filter=pad,format,scale,split,palettegen,paletteuse,null,ssim,psnr,setsar,setpts --extra-cflags="$CFLAGS -I$PREFIX/include" --extra-ldflags="$LDFLAGS -L$PREFIX/lib -Wl,-Map,$(cygpath -m "$WORK/logs/link.map")" --extra-version=LibreToon-1
 # Normalise only generated build-description paths; actual commands remain in config.log.
 sed -i -e "s|$MAP|/build|g" -e "s|$WORK|/build|g" config.h
 make -j"$JOBS" ffmpeg.exe
 cp "$WORK/logs/link.map" "$WORK/logs/ffmpeg-link.map"
 make -j"$JOBS" ffprobe.exe
 cp "$WORK/logs/link.map" "$WORK/logs/ffprobe-link.map"
 cp ffmpeg.exe ffprobe.exe "$WORK/output/"
 cp config.h config_components.h ffbuild/config.mak ffbuild/config.log "$WORK/logs/"
 llvm-readobj --coff-imports "$WORK/output/ffmpeg.exe" > "$WORK/logs/pe-imports.txt"
 llvm-readobj --coff-imports "$WORK/output/ffprobe.exe" > "$WORK/logs/pe-imports-probe.txt"
 "$WORK/output/ffmpeg.exe" -version > "$WORK/logs/ffmpeg-version.txt" 2>&1
 "$WORK/output/ffmpeg.exe" -L > "$WORK/logs/ffmpeg-license.txt" 2>&1
) > "$WORK/logs/ffmpeg-build.txt" 2>&1
