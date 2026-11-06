#!/bin/bash
# Builds tools/winxp/shim/tests/test_misc.exe (checks xp_misc.c; kernel32 + advapi32, no C runtime, PE
# subsystem 5.1).   tools/winxp/shim/tests/build_misc.sh [OUTDIR]
# Run on Windows 7 (VM) as `test_misc.exe` (Windows' own) and `test_misc.exe ours` (the emulation), and
# on Windows XP.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${1:-${SHARE:-/Volumes/RapidRBuild/winxp-share}/files}"
mkdir -p "$OUT"
"$LLVM_MINGW/bin/i686-w64-mingw32-clang" -O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -fno-stack-protector -msse2 -mno-stack-arg-probe -nostdlib \
    -DXP_MISC_TEST -Wl,-e,_test_main -Wl,--subsystem,console \
    -Wl,--major-subsystem-version,5 -Wl,--minor-subsystem-version,1 -Wl,--major-os-version,5 -Wl,--minor-os-version,1 \
    -o "$OUT/test_misc.exe" "$HERE/test_misc.c" "$HERE/../xp_misc.c" -lkernel32 -ladvapi32
"$LLVM_MINGW/bin/llvm-readobj" --file-headers "$OUT/test_misc.exe" | grep MajorSubsystemVersion
"$LLVM_MINGW/bin/llvm-readobj" --coff-imports "$OUT/test_misc.exe" | awk '/^  Name: /{d=$2} /Symbol: /{printf "%s ", d} END{print ""}' | tr ' ' '\n' | sort -u | tr '\n' ' '
echo
