#!/bin/bash
# Builds the Windows XP test VM's two small tools with LLVM-MinGW (i686, no C runtime, PE subsystem
# and OS version 5.1 so plain XP starts them) and lists their imports:
#   pipeprobe.exe  (probe/pipeprobe.c + shim/xp_pipes.c)   do std's child pipes work here; do ours
#   xpshot.exe     (probe/xpshot.c)                        a BMP of the screen
#   xpclick.exe    (probe/xpclick.c)                       a click (or double click) at a screen point
#
#   tools/winxp/probe/build.sh [OUTDIR]      (default $SHARE/files, SHARE=/Volumes/RapidRBuild/winxp-share)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${1:-${SHARE:-/Volumes/RapidRBuild/winxp-share}/files}"
mkdir -p "$OUT"
CC="$LLVM_MINGW/bin/i686-w64-mingw32-clang"
XP=(-Wl,--major-subsystem-version,5 -Wl,--minor-subsystem-version,1 -Wl,--major-os-version,5 -Wl,--minor-os-version,1)
FLAGS=(-O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -fno-stack-protector -mno-sse -nostdlib)
"$CC" "${FLAGS[@]}" "${XP[@]}" -DXP_PIPES_TEST -Wl,-e,_probe_main -Wl,--subsystem,console -o "$OUT/pipeprobe.exe" \
    "$HERE/pipeprobe.c" "$HERE/../shim/xp_pipes.c" -lkernel32 -ladvapi32
"$CC" "${FLAGS[@]}" "${XP[@]}" -Wl,-e,_shot_main -Wl,--subsystem,windows -o "$OUT/xpshot.exe" \
    "$HERE/xpshot.c" -lkernel32 -luser32 -lgdi32
"$CC" "${FLAGS[@]}" "${XP[@]}" -Wl,-e,_click_main -Wl,--subsystem,windows -o "$OUT/xpclick.exe" \
    "$HERE/xpclick.c" -lkernel32 -luser32
for f in pipeprobe xpshot xpclick; do
    echo "== $f.exe: $("$LLVM_MINGW/bin/llvm-readobj" --file-headers "$OUT/$f.exe" | awk '/MajorSubsystemVersion/{a=$2} /MinorSubsystemVersion/{b=$2} END{print "subsystem " a "." b}')"
    "$LLVM_MINGW/bin/llvm-readobj" --coff-imports "$OUT/$f.exe" | awk '/^  Name: /{d=$2} /Symbol: /{printf "%s!%s ", d, $2} END{print ""}'
done
