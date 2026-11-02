#!/usr/bin/env bash
set -euo pipefail
export PATH=/usr/bin:$PATH LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=1718755200
WORK="$(cygpath -u "${FFMPEG_BUILD_ROOT:?}")"
TOOLS="$(cygpath -u "${FFMPEG_TOOL_ROOT:?}")"
export PATH="$TOOLS/llvm-mingw-20240619-ucrt-x86_64/bin:$TOOLS/nasm-2.16.03:/usr/bin"
cd "$WORK/build-ffmpeg"
printf '%s\n' 'override LDFLAGS += -Wl,--verbose' > "$WORK/trace-overrides.mak"
for program in ffmpeg ffprobe; do
 # --verbose affects diagnostics only. -W forces linking, not recompiling sources.
 make V=1 -f Makefile -f "$WORK/trace-overrides.mak" -W "fftools/$program.o" "${program}_g.exe" > "$WORK/logs/$program-link-trace.txt" 2>&1
 make "$program.exe" >> "$WORK/logs/$program-link-trace.txt" 2>&1
 cp "$program.exe" "$WORK/output/"
done
