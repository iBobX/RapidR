#!/bin/bash
# Builds test_stand_ins.c (a check of win7_stand_ins.c) with LLVM-MinGW for i686 or x86_64:
#
#   tools/win7/shim/test.sh [i686|x86_64] [output .exe]
#
# then run the .exe on a Windows 8+ machine (the Windows 11 VM: copy it in and run it twice, with and
# without the argument "absent"; README of tools/win7). Same LLVM_MINGW as build.sh.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ARCH="${1:-i686}"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${2:-${TMPDIR:-/tmp}/stand_ins_test.exe}"
"$LLVM_MINGW/bin/${ARCH}-w64-mingw32-clang" -O1 -Wall -Wextra -Wno-unused-function -fno-builtin -msse2 -mfpmath=sse -o "$OUT" "$HERE/test_stand_ins.c"
echo "$OUT"
