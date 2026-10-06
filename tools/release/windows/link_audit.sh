#!/bin/bash
# What of LLVM-MinGW's runtime (mingw-w64's libraries, compiler-rt, libunwind)
# ends up in a Windows program RapidR builds, and in rapidr.exe: each is
# cross-linked here (macOS or Linux) with an LLVM-MinGW release and a linker
# map, and the toolchain objects the link pulls in are listed. The link keeps
# every section it pulls in (--no-gc-sections): what is listed is what any
# program on the same runtime could contain, not only what this one keeps.
#
#   MINGW_SRC=<mingw-w64 source at llvm-mingw's pinned commit> \
#   tools/release/windows/link_audit.sh <llvm-mingw folder> <work folder> [aarch64|x86_64] [program.bas]
#
# Rust's gnullvm target comes from a throwaway RUSTUP_HOME in the work folder
# (the user's rustup untouched). The program (default tests/fixtures/list_items.bas,
# a GUI program on the full runtime) is generated with `rapidr codegen` and
# linked as `rapidr build` links it (+crt-static); rapidr.exe as the release
# scripts link it. Output: <work>/<what>.toolchain (the toolchain's objects
# pulled into it, one per line) and, with MINGW_SRC, each mingw-w64 member's source and
# licence, flagging Cephes-derived, Wine-imported or (L)GPL code and any licence the
# Windows notices don't carry (then the exit status is 1). docs/licensing.md §3.3 says
# what they showed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
MINGW="$(cd "$1" && pwd)"; WORK="$2"; ARCH="${3:-aarch64}"; BAS="${4:-$ROOT/tests/fixtures/list_items.bas}"
RUST="$(sed -n 's/^rust-version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
RUST="${RUST:-$(rustc --version | cut -d' ' -f2)}"
T="$ARCH-pc-windows-gnullvm"; UP="$(echo "$T" | tr 'a-z-' 'A-Z_')"; LO="${T//-/_}"
mkdir -p "$WORK"; WORK="$(cd "$WORK" && pwd)"
export RUSTUP_HOME="$WORK/rustup" RUSTUP_TOOLCHAIN="$RUST"
export RAPIDR_PRINT_TO="$WORK/prints" RAPIDR_REGISTRY="$WORK/registry.reg"
mkdir -p "$RUSTUP_HOME" "$RAPIDR_PRINT_TO"
rustup toolchain install "$RUST" --profile minimal --no-self-update --target "$T" > /dev/null
CC="$MINGW/bin/$ARCH-w64-mingw32-clang"
export "CARGO_TARGET_${UP}_LINKER=$CC" "CC_$LO=$CC" "AR_$LO=$MINGW/bin/llvm-ar"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

link() {   # link <name> <cargo args…>: a build of <name> with a linker map
    local name="$1"; shift
    # (the map and --no-gc-sections for the final link only: the crates build once for both)
    CARGO_TARGET_DIR="$WORK/target" RUSTFLAGS="-C target-feature=+crt-static" cargo rustc -q --target "$T" "$@" \
        -- -C "link-arg=-Wl,-Map=$WORK/$name.map" -C link-arg=-Wl,--no-gc-sections
    # (lld's map: one line per input section in the output, its file: a start-up object by path,
    # an archive member by its name — `lib<arch>_lib<library>_a-<source>.o` for
    # mingw-w64's, `<source>.c.obj` / `.cpp.obj` for libunwind's and compiler-rt's)
    # (left out: Rust's objects, and the crates' own C objects — `<hash>-<source>.o`, listed
    # with their crates — and rustc's symbols.o)
    grep -E '^\S+\s+\S+\s+\S+\s+\S+:\(' "$WORK/$name.map" | awk '{print $4}' | sed 's/:(.*//' | sort -u \
        | grep -vE -- '-[0-9a-f]{16}\.|\.rcgu\.o$|^rapidr_|(^|/)[0-9a-f]{16}-[^/]*\.o$|/symbols\.o$' > "$WORK/$name.toolchain" || true
    echo "== $name: what the toolchain put in it ($WORK/$name.toolchain)"
    sed -E 's#.*/##; s/-[^-]*$//' "$WORK/$name.toolchain" | sort | uniq -c | sort -rn
    if [ -n "${MINGW_SRC:-}" ]; then
        python3 "$ROOT/tools/release/windows/member_licences.py" "$WORK/$name.toolchain" "$MINGW_SRC/mingw-w64-crt" || touch "$WORK/flagged"   # (link runs in a subshell)
    fi
    rm -f "$WORK/$name.map"   # (over a GB in a debug build)
}

# a program, as `rapidr build` makes it
# (this machine's rapidr generates the program: built with the user's own Rust)
(cd "$ROOT" && env -u RUSTUP_TOOLCHAIN -u RUSTUP_HOME cargo build -q --release -p rapidr-cli)
RAPIDR="$(cd "$ROOT" && env -u RUSTUP_TOOLCHAIN -u RUSTUP_HOME cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/release/rapidr"
rm -rf "$WORK/program" "$WORK/flagged"
env -u RUSTUP_TOOLCHAIN -u RUSTUP_HOME "$RAPIDR" codegen "$BAS" "$WORK/program" > /dev/null
(cd "$WORK/program" && link program)
# rapidr.exe, as the release scripts make it
(cd "$ROOT" && link rapidr --release --locked -p rapidr-cli --bin rapidr)
[ ! -e "$WORK/flagged" ] || { echo "link_audit: flagged objects above: eliminate them, or update the notices (notices.rs) and member_licences.py together" >&2; exit 1; }
