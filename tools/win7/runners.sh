#!/bin/bash
# Builds the two runner programs for Windows 7 / 8 / 8.1: rapidrintr-runner.exe (console) and
# rapidrintr-runnerw.exe (windowed). `rapidr build --interp` copies one of them, with the program's
# bytecode after it, into the standalone .exe a person gets; an install ships them in
# lib\rapidr\runners\windows-x86\ (crates/rapidr-cli/src/home.rs). The Windows 7 alpha zip must
# carry them (tools/win7/alpha.sh does): without them "Build" on a PC with no Rust says
# "could not build or find rapidrintr-runnerw.exe (cargo build -p rapidr-runner-stub)".
#
#   tools/win7/runners.sh [i686|x86_64]            (default i686)
#
# Same environment as build.sh (CARGO_TARGET_DIR, LLVM_MINGW). Result:
#   $CARGO_TARGET_DIR/<arch>-win7-windows-gnu/release/rapidrintr-runner.exe and ...runnerw.exe
#
# Windows 7 refuses to start a program whose import table names a function it lacks. rapidr.exe has
# crates/rapidr-cli/src/win7.rs for the six that matter; the runner stub is another crate that does
# not, so tools/win7/shim/win7_stand_ins.c (the same six, in C) is compiled here and handed to the
# linker of these two programs, and then the import tables are read (imports.py) to prove no Windows 8
# only function is left. (The permanent fix is win7.rs in the stub crate too; this keeps the runtime
# crates as they are.)
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
ARCH="${1:-i686}"
case "$ARCH" in i686 | x86_64) ;; *) echo "first argument: i686 or x86_64" >&2; exit 2 ;; esac
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
TARGET="${ARCH}-win7-windows-gnu"

OBJ="$TD/win7-shims/$ARCH/win7_stand_ins.o"
mkdir -p "$(dirname "$OBJ")"
"$LLVM_MINGW/bin/${ARCH}-w64-mingw32-clang" -O2 -Wall -Wextra -c "$HERE/shim/win7_stand_ins.c" -o "$OBJ"

# `cargo rustc` hands the object to these two programs' link only: the libraries they use are not
# built again (a RUSTFLAGS change would).
for bin in rapidrintr-runner rapidrintr-runnerw; do
    WIN7_CARGO=rustc "$HERE/build.sh" "$ARCH" -p rapidr-runner-stub --bin "$bin" -- -C "link-arg=$OBJ"
done

OUT="$TD/$TARGET/release"
status=0
for bin in rapidrintr-runner rapidrintr-runnerw; do
    exe="$OUT/$bin.exe"
    [ -f "$exe" ] || { echo "missing $exe" >&2; exit 1; }
    flagged="$(python3 -I "$HERE/imports.py" "$exe" | sed -n '/== static/,/== delay/p' | grep 'NOT ON WINDOWS 7' || true)"
    if [ -n "$flagged" ]; then
        echo "$bin.exe: Windows 7 would refuse to start it:" >&2
        echo "$flagged" >&2
        status=1
    else
        echo "$bin.exe: every function it needs at start is in Windows 7 ($(du -h "$exe" | cut -f1))"
    fi
done
exit $status
