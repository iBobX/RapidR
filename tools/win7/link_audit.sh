#!/bin/bash
# What of LLVM-MinGW's runtime (mingw-w64's libraries, compiler-rt, libunwind,
# winpthreads) ends up in the Windows 7 rapidr.exe, and under which licence:
# the program is relinked with a linker map and --no-gc-sections (so the map
# shows everything the link pulls in, not only what it keeps), the toolchain's
# objects are listed, and each mingw-w64 member's source header is read
# (tools/release/windows/member_licences.py). Exit status 1 when an object is
# flagged (Cephes-derived, Wine-imported, (L)GPL, a licence not in the notices).
#
#   tools/win7/link_audit.sh <mingw-w64 source folder> [work folder]
#
# <mingw-w64 source folder> holds mingw-w64-crt/ at the commit LLVM-MinGW names
# (build-mingw-w64.sh's MINGW_W64_VERSION; docs/legal/review-2026-10-23-win7-alpha.md).
# Env as build.sh (CARGO_TARGET_DIR, LLVM_MINGW). Afterwards the program is
# built again the usual way, so the .exe left behind is the normal one.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$(cd "$1" && pwd)"
WORK="${2:-/Volumes/RapidRBuild/wt-win7-alpha/audit}"
mkdir -p "$WORK"; WORK="$(cd "$WORK" && pwd)"

WIN7_CARGO=rustc "$ROOT/tools/win7/build.sh" i686 -p rapidr-cli --bin rapidr -- \
    -C "link-arg=-Wl,-Map=$WORK/rapidr.map" -C link-arg=-Wl,--no-gc-sections

# (lld's map: one line per input section, its file as the 4th field: a start-up
# object by path, an archive member by its name. Left out: Rust's objects, the
# crates' own C objects (<hash>-<source>.o), rustc's symbols.o, and this build's
# own version-information object, build.rs's program.o.)
grep -E '^\S+\s+\S+\s+\S+\s+\S+:\(' "$WORK/rapidr.map" | awk '{print $4}' | sed 's/:(.*//' | sort -u \
    | grep -vE -- '-[0-9a-f]{16}\.|\.rcgu\.o$|^rapidr_|(^|/)[0-9a-f]{16}-[^/]*\.o$|/symbols\.o$|/out/program\.o$' > "$WORK/rapidr.toolchain" || true
rm -f "$WORK/rapidr.map"   # (40 MB)

echo "== the toolchain's objects in rapidr.exe ($WORK/rapidr.toolchain)"
sed -E 's#.*/##; s/_a-.*$//; s/\.(c|cpp|S)\.obj$/ (libunwind)/' "$WORK/rapidr.toolchain" | sort | uniq -c | sort -rn
echo "== winpthreads, compiler-rt builtins, libgcc: any of them?"
grep -iE 'pthread|clang_rt|libgcc|builtins' "$WORK/rapidr.toolchain" || echo "none"

status=0
python3 -I "$ROOT/tools/release/windows/member_licences.py" "$WORK/rapidr.toolchain" "$SRC/mingw-w64-crt" || status=1

# the usual build again (only the program is relinked)
"$ROOT/tools/win7/build.sh" i686 -p rapidr-cli --bin rapidr
exit $status
