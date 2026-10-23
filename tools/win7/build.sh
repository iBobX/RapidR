#!/bin/bash
# Builds RapidR for Windows 7 / 8 / 8.1 (and 10 / 11) from a Mac: a 32-bit
# (or 64-bit) .exe that links against msvcrt.dll + the system DLLs only.
#
#   tools/win7/build.sh [i686|x86_64] [cargo args...]      (default: i686, -p rapidr-cli)
#   tools/win7/build.sh i686 -p rapidr-cli --bin rapidr
#
# Needs (nothing is installed system-wide):
#   * rustup's nightly + rust-src:  rustup toolchain install nightly --profile minimal
#                                   rustup component add rust-src --toolchain nightly
#     (the *-win7-* targets are tier 3: no prebuilt standard library, so
#      `-Z build-std` builds it. The stable toolchain is not used here.)
#   * LLVM-MinGW with the MSVCRT flavour (Windows 7 has msvcrt.dll, not the
#     Universal CRT) in $LLVM_MINGW. LLVM's macOS release is UCRT-only: the
#     tools/win7/README.md says how the msvcrt sysroot is added to it.
#
# Environment (all optional):
#   LLVM_MINGW         the toolchain (default /Volumes/RapidRBuild/tools/llvm-mingw-msvcrt)
#   CARGO_TARGET_DIR   where to build (default ./target)
#   CARGO_BUILD_JOBS   default 3
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCH="${1:-i686}"
shift || true
case "$ARCH" in
    i686 | x86_64) ;;
    *) echo "first argument: i686 or x86_64" >&2; exit 2 ;;
esac
TARGET="${ARCH}-win7-windows-gnu"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
export CARGO_TARGET_DIR="$TD"

CLANG="$LLVM_MINGW/bin/${ARCH}-w64-mingw32-clang"
[ -x "$CLANG" ] || { echo "missing $CLANG (set LLVM_MINGW)" >&2; exit 1; }

# Rust's *-windows-gnu targets name -lgcc, -lgcc_eh and -lgcc_s. LLVM-MinGW
# has no libgcc: compiler-rt (the builtins) and libunwind do that job. Three
# small libraries in a folder of ours say so (an empty libgcc.a, and
# libunwind.a under the other two names; static, so no libgcc_s DLL).
SHIMS="$TD/win7-shims/$ARCH"
if [ ! -f "$SHIMS/libgcc_s.a" ]; then
    mkdir -p "$SHIMS"
    cp "$LLVM_MINGW/${ARCH}-w64-mingw32/lib/libunwind.a" "$SHIMS/libgcc_eh.a"
    cp "$SHIMS/libgcc_eh.a" "$SHIMS/libgcc_s.a"
    rm -f "$SHIMS/libgcc.a"
    "$LLVM_MINGW/bin/llvm-ar" rcs "$SHIMS/libgcc.a"
fi

UPPER="$(echo "${TARGET}" | tr 'a-z-' 'A-Z_')"
export "CARGO_TARGET_${UPPER}_LINKER=$CLANG"
# --cfg windows_raw_dylib: the windows / windows-sys crates import each function
# by name from the DLL (rustc makes the import entries), not through the
# prebuilt GNU import libraries -- only then can the linker delay-load the
# Windows 8+ ones (crates/rapidr-cli/build.rs, src/win7.rs).
export "CARGO_TARGET_${UPPER}_RUSTFLAGS=-C link-arg=-L$SHIMS --cfg windows_raw_dylib"
# SQLite's C (libsqlite3-sys, bundled) through the cc crate
LOWER="$(echo "${TARGET}" | tr '-' '_')"
export "CC_${LOWER}=$CLANG"
export "AR_${LOWER}=$LLVM_MINGW/bin/llvm-ar"

# rustc asks for <arch>-w64-mingw32-dlltool by that name (raw-dylib imports).
# At the END of PATH: the Mac's own tools keep priority for build scripts.
export PATH="$PATH:$LLVM_MINGW/bin"

cd "$ROOT"
if [ "$#" -eq 0 ]; then set -- -p rapidr-cli; fi
exec cargo +nightly build --release -Z build-std=std,panic_abort --target "$TARGET" "$@"
