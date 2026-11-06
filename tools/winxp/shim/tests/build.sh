#!/bin/bash
# Builds tools/winxp/shim/tests/test_sync.exe (a stress test of xp_sync.c; kernel32 only, no C runtime,
# PE subsystem 5.1) with LLVM-MinGW:   tools/winxp/shim/tests/build.sh [OUTDIR]
# Run it on Windows 7 (VM) as `test_sync.exe` and `test_sync.exe ours`, and on Windows XP.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${1:-${SHARE:-/Volumes/RapidRBuild/winxp-share}/files}"
mkdir -p "$OUT"
"$LLVM_MINGW/bin/i686-w64-mingw32-clang" -O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -fno-stack-protector -msse2 -nostdlib \
    -DXP_SYNC_TEST -Wl,-e,_test_main -Wl,--subsystem,console \
    -Wl,--major-subsystem-version,5 -Wl,--minor-subsystem-version,1 -Wl,--major-os-version,5 -Wl,--minor-os-version,1 \
    -o "$OUT/test_sync.exe" "$HERE/test_sync.c" "$HERE/../xp_sync.c" -lkernel32
"$LLVM_MINGW/bin/llvm-readobj" --file-headers "$OUT/test_sync.exe" | grep -E "TLSTableRVA|MajorSubsystemVersion"
"$LLVM_MINGW/bin/llvm-readobj" --coff-imports "$OUT/test_sync.exe" | awk '/Symbol: /{printf "%s ", $2} END{print ""}'
