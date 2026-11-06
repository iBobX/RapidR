#!/bin/bash
# Builds tools/winxp/shim/tests/test_dwrite.exe (checks xp_dwrite.c; kernel32 only, no C runtime, PE
# subsystem 5.1).   tools/winxp/shim/tests/build_dwrite.sh [OUTDIR]
# Run on Windows 7 (VM) as `test_dwrite.exe` and `test_dwrite.exe ours`, and on Windows XP.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${1:-${SHARE:-/Volumes/RapidRBuild/winxp-share}/files}"
mkdir -p "$OUT"
"$LLVM_MINGW/bin/i686-w64-mingw32-clang" -O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -fno-stack-protector -msse2 -mno-stack-arg-probe -nostdlib \
    -DXP_DWRITE_TEST -Wl,-e,_test_main -Wl,--subsystem,console \
    -Wl,--major-subsystem-version,5 -Wl,--minor-subsystem-version,1 -Wl,--major-os-version,5 -Wl,--minor-os-version,1 \
    -o "$OUT/test_dwrite.exe" "$HERE/test_dwrite.c" "$HERE/../xp_dwrite.c" -lkernel32
echo "$OUT/test_dwrite.exe"
