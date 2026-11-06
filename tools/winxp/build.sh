#!/bin/bash
# The Windows XP flavour of RapidR (32-bit; lane WINXP, docs/winxp-plan.md): the Windows 7 build's
# libraries, unchanged, and three programs linked again with an object of XP stand-ins.
#
#   tools/winxp/build.sh            -> $CARGO_TARGET_DIR/winxp/i686/{rapidr,rapidrintr-runner,rapidrintr-runnerw}.exe
#
# How it stays apart from every other build (Robert's rule, 2026-11-05):
#   - nothing here changes a source file, a Cargo.toml, a RUSTFLAGS or a build script: the crates are
#     the Windows 7 build's, built by tools/win7/build.sh with its flags, and only the three programs
#     are linked again (`cargo rustc ... -- -C link-arg=...`), with tools/winxp/shim's object and,
#     with XP_HEADER=5.1, the PE header's subsystem / OS version 5.1;
#   - those links write the Windows 7 programs' paths (target/i686-win7-windows-gnu/release); this
#     script copies its results to target/winxp/i686 and then links the Windows 7 programs again as
#     tools/win7/build.sh and runners.sh make them, so the Windows 7 files are what they were
#     (WINXP_KEEP_WIN7=0 skips that).
#
# Environment: as tools/win7/build.sh (LLVM_MINGW, CARGO_TARGET_DIR, CARGO_BUILD_JOBS);
#   XP_HEADER=6.0 (default: the header the Windows 7 build has; XP + One-Core-API starts it) or 5.1
#   (plain Windows XP refuses an image above 5.1: "not a valid Win32 application").
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
export CARGO_TARGET_DIR="$TD"
REL="$TD/i686-win7-windows-gnu/release"
OUT="$TD/winxp/i686"
OBJDIR="$TD/winxp-shims"
mkdir -p "$OUT" "$OBJDIR"
CC="$LLVM_MINGW/bin/i686-w64-mingw32-clang"

# the XP stand-in objects: every tools/winxp/shim/*.c
OBJS=()
for c in "$HERE"/shim/*.c; do
    o="$OBJDIR/$(basename "$c" .c).o"
    "$CC" -O2 -Wall -Wextra -Wno-unused-parameter -fno-builtin -msse2 -mfpmath=sse -c "$c" -o "$o"
    OBJS+=("$o")
done

# The stand-ins are put FIRST in each program's link (tools/winxp/xp-link.sh), not appended as
# `-C link-arg` objects: appended, they come after every `-lkernel32`, and the import library defines
# the Vista+ thunks (e.g. _ReleaseSRWLockExclusive@4) before the stand-in is read, which both collides
# with the stand-in and leaves the function in the import table (plain XP then refuses the .exe). Read
# first, the stand-in's definitions win and those functions never enter the import table.
LINKER="$HERE/xp-link.sh"
export XP_REAL_CC="$CC"
LINK=(-C "linker=$LINKER")
case "${XP_HEADER:-6.0}" in
    6.0) ;;
    5.1) LINK+=(-C link-arg=-Wl,--major-subsystem-version,5 -C link-arg=-Wl,--minor-subsystem-version,1
                -C link-arg=-Wl,--major-os-version,5 -C link-arg=-Wl,--minor-os-version,1) ;;
    *) echo "XP_HEADER: 6.0 or 5.1" >&2; exit 2 ;;
esac

# the Windows 7 libraries (and programs), as the Windows 7 build makes them
"$ROOT/tools/win7/build.sh" i686 -p rapidr-cli
"$CC" -O2 -Wall -Wextra -fno-builtin -msse2 -mfpmath=sse -c "$ROOT/tools/win7/shim/win7_stand_ins.c" -o "$OBJDIR/win7_stand_ins.o"

# the three programs, linked through the wrapper with the XP stand-ins first (rapidr.exe has
# src/win7.rs for its six functions; the runners need the Windows 7 C stand-ins' six too, so those go
# in their stand-in set as well)
XP_SHIM_OBJS="${OBJS[*]}" WIN7_CARGO=rustc "$ROOT/tools/win7/build.sh" i686 -p rapidr-cli --bin rapidr -- "${LINK[@]}"
cp "$REL/rapidr.exe" "$OUT/rapidr.exe"
for bin in rapidrintr-runner rapidrintr-runnerw; do
    XP_SHIM_OBJS="${OBJS[*]} $OBJDIR/win7_stand_ins.o" WIN7_CARGO=rustc "$ROOT/tools/win7/build.sh" i686 -p rapidr-runner-stub --bin "$bin" -- "${LINK[@]}"
    cp "$REL/$bin.exe" "$OUT/$bin.exe"
done

if [ "${WINXP_KEEP_WIN7:-1}" = 1 ]; then
    # (the Windows 7 programs again, linked as their own scripts link them)
    WIN7_CARGO=rustc "$ROOT/tools/win7/build.sh" i686 -p rapidr-cli --bin rapidr
    "$ROOT/tools/win7/runners.sh" i686 > /dev/null
fi
ls -l "$OUT"
