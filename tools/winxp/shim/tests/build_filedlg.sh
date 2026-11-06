#!/bin/bash
# Builds tools/winxp/shim/tests/test_filedlg.exe (checks xp_filedlg.c; kernel32, user32, ole32; no C
# runtime; PE subsystem 5.1).   tools/winxp/shim/tests/build_filedlg.sh [OUTDIR]
# Run on Windows XP (the dialogs are ours there), and on a newer Windows as `test_filedlg.exe` (Windows'
# own dialogs) and `test_filedlg.exe ours` (ours forced). It needs a desktop: it shows each dialog and a
# second thread presses OK or Cancel in it.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${1:-${SHARE:-/Volumes/RapidRBuild/winxp-share}/files}"
mkdir -p "$OUT"
"$LLVM_MINGW/bin/i686-w64-mingw32-clang" -O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -fno-stack-protector -msse2 -mno-stack-arg-probe -nostdlib \
    -Wl,-e,_test_main -Wl,--subsystem,console \
    -Wl,--major-subsystem-version,5 -Wl,--minor-subsystem-version,1 -Wl,--major-os-version,5 -Wl,--minor-os-version,1 \
    -o "$OUT/test_filedlg.exe" "$HERE/test_filedlg.c" "$HERE/../xp_filedlg.c" -lkernel32 -luser32 -lole32
"$LLVM_MINGW/bin/llvm-readobj" --coff-imports "$OUT/test_filedlg.exe" | awk '/^  Name: /{d=$2} /Symbol: /{printf "%s!%s ", d, $2} END{print ""}'
