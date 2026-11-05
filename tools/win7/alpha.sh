#!/bin/bash
# Builds the Windows 7 / 8 / 8.1 ALPHA download: one zip a person unzips and
# runs (RapidR-Studio.bat, the program, Studio, the examples, the runner
# programs "Build" copies, the smoke tests, README-ALPHA.txt, every licence text
# and THIRD-PARTY-NOTICES.txt), plus its SHA-256.
#
#   tools/win7/alpha.sh [ALPHA_NUMBER]            (default 1)
#
# Build the programs first (the build matrix, tools/release/build-matrix.sh, reads this zip):
#   tools/win7/build.sh i686                       rapidr.exe
#   tools/win7/runners.sh i686                     rapidrintr-runner.exe, rapidrintr-runnerw.exe
# Environment (optional):
#   CARGO_TARGET_DIR  where build.sh built (default ./target)
#   ALPHA_OUT         where the zip goes (default ./dist/<version>-win7-alpha<N>)
#   ALPHA_WORK        scratch (default /Volumes/RapidRBuild/wt-win7-alpha/stage)
#   LLVM_MINGW        the toolchain (llvm-strip, for the symbols)
#   HOST_RAPIDR       a rapidr for THIS Mac, built from this checkout (default
#                     /Volumes/RapidRBuild/main/target/release/rapidr): it makes Studio's bytecode and the notices
# Also puts the zip (and VM-STEPS-ALPHA.txt) on the test VM's host-only share
# (tools/win7/serve.sh) when WIN7_SHARE is set, and makes
# $WIN7_ISO_DIR/RAPIDR-ALPHA1.iso when WIN7_ISO_DIR is set too; nothing is
# started or served here.
#
# The zip is laid out as an install (crates/rapidr-cli/src/home.rs finds an install from the
# executable, with no environment variable set, from any Command Prompt, from Studio, from the tests):
#
#   bin\rapidr.exe                            the program
#   lib\rapidr\release.toml                   what makes it an install (version, kind = "sdk")
#   lib\rapidr\ide\rapidr-ide.rrbc            Studio (its bytecode, as in every install), ide\assets, ide\lang
#   lib\rapidr\examples\                      the example programs
#   lib\rapidr\runners\windows-x86\           rapidrintr-runner.exe, rapidrintr-runnerw.exe: what Build copies
#   lib\rapidr\notices\windows-x86.txt        the THIRD-PARTY-NOTICES.txt a built program carries beside it
#   RapidR-Studio.bat, README-ALPHA.txt, tests\, licences    as before
#
# Alpha 1 of 2.121.0 / 2.122.1 had rapidr.exe, ide\studio.rr and examples\ side by side and no runners:
# "Build" on a PC with no Rust said "could not build or find rapidrintr-runnerw.exe (cargo build -p
# rapidr-runner-stub)". A native build is not in this package (no Rust for Windows 7); the interpreted
# Build is, and tools/release/windows/smoke-win7.ps1 (the build matrix) uses it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
N="${1:-1}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
BUILT="$TD/i686-win7-windows-gnu/release"
EXE="$BUILT/rapidr.exe"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${ALPHA_OUT:-$ROOT/dist/$VERSION-win7-alpha$N}"
WORK="${ALPHA_WORK:-/Volumes/RapidRBuild/wt-win7-alpha/stage}"
NAME="RapidR-$VERSION-win7-x86-alpha$N"
[ -f "$EXE" ] || { echo "build first: tools/win7/build.sh i686 (missing $EXE)" >&2; exit 1; }
for r in rapidrintr-runner rapidrintr-runnerw; do
    [ -f "$BUILT/$r.exe" ] || { echo "build first: tools/win7/runners.sh i686 (missing $BUILT/$r.exe)" >&2; exit 1; }
done

# only ever empty a folder this script made (it carries a marker)
if [ -e "$WORK" ] && [ ! -f "$WORK/.alpha-stage" ]; then
    echo "$WORK exists and is not an alpha stage folder; refusing" >&2; exit 1
fi
mkdir -p "$WORK"; touch "$WORK/.alpha-stage"
DIR="$WORK/$NAME"
LIB="$DIR/lib/rapidr"
rm -rf "$DIR" "$WORK/$NAME.zip"
mkdir -p "$DIR/bin" "$DIR/tests" "$DIR/licenses" "$LIB/ide" "$LIB/runners/windows-x86" "$LIB/notices" "$OUT"

# a rapidr for this Mac, of this checkout's version: Studio's bytecode and the notices come from it
HOST_RAPIDR="${HOST_RAPIDR:-/Volumes/RapidRBuild/main/target/release/rapidr}"
[ -x "$HOST_RAPIDR" ] || { echo "need a rapidr for this Mac (HOST_RAPIDR=…; cargo build --release -p rapidr-cli)" >&2; exit 1; }
[ "$("$HOST_RAPIDR" version)" = "RapidR $VERSION" ] || { echo "HOST_RAPIDR is '$("$HOST_RAPIDR" version)', this checkout is RapidR $VERSION: build it from this checkout" >&2; exit 1; }

# the programs, without their symbols (80 MB -> about half); version info stays
strip() { "$LLVM_MINGW/bin/llvm-strip" "$1"; }
cp "$EXE" "$DIR/bin/rapidr.exe"
strip "$DIR/bin/rapidr.exe"
"$LLVM_MINGW/bin/llvm-readobj" --coff-resources "$DIR/bin/rapidr.exe" | grep -q 'VERSIONINFO' || { echo "rapidr.exe has no version information" >&2; exit 1; }
# (the runners: no function Windows 7 lacks in what they need to start — runners.sh checked, and this is the zip's own check)
for r in rapidrintr-runner rapidrintr-runnerw; do
    cp "$BUILT/$r.exe" "$LIB/runners/windows-x86/$r.exe"
    strip "$LIB/runners/windows-x86/$r.exe"
done
for f in "$DIR/bin/rapidr.exe" "$LIB/runners/windows-x86/rapidrintr-runner.exe" "$LIB/runners/windows-x86/rapidrintr-runnerw.exe"; do
    if python3 -I "$ROOT/tools/win7/imports.py" "$f" | sed -n '/== static/,/== delay/p' | grep -q 'NOT ON WINDOWS 7'; then
        echo "$(basename "$f") needs a function Windows 7 does not have (tools/win7/imports.py)" >&2; exit 1
    fi
done

# the files a checkout tracks (a source archive has only those: no .git, then every file there)
tracked() { if [ -e "$ROOT/.git" ]; then git ls-files "$@"; else find "$@" -type f | sort; fi; }
crlf() { sed 's/$/\r/' "$1" > "$2"; }   # (Windows 7's Notepad needs Windows line ends)
# ... and for text with accents, marked as UTF-8 (Notepad there reads it as ANSI otherwise)
text() { { printf '\357\273\277'; sed 's/$/\r/' "$1"; } > "$2"; }

cd "$ROOT"
# What makes the folder an install (home.rs): version, no Rust (a native build is not in this package), the SDK's kind.
{
    echo "# An installed RapidR's home (crates/rapidr-cli/src/home.rs). Windows 7 alpha: interpreted builds only."
    echo "version = \"$VERSION\""
    echo "rust = \"\""
    echo "kind = \"sdk\""
} > "$LIB/release.toml"
# Studio: its bytecode (as the installers ship it), the start page's pictures and the translations
# (never the pseudo-language xx, a test's)
RAPIDR_HOME="$ROOT" RAPIDR_REGISTRY="$WORK/notices-env/registry" RAPIDR_PRINT_TO="$WORK/notices-env/print" RAPIDR_AI_KEYSTORE=memory \
    "$HOST_RAPIDR" build-bc ide/studio.rr -o "$LIB/ide/rapidr-ide.rrbc" > /dev/null
tracked ide/assets ide/lang | grep -vE '/xx\.po$|\.pot$' | while IFS= read -r f; do
    mkdir -p "$LIB/$(dirname "$f")"
    cp "$f" "$LIB/$f"
done
# the examples (what git tracks of examples/)
tracked examples | while IFS= read -r f; do
    mkdir -p "$LIB/$(dirname "$f")"
    cp "$f" "$LIB/$f"
done

# the launcher, the readme and the smoke tests
# (README-ALPHA.txt says @VERSION@ and @ALPHA@: this build's)
sed -e "s/@VERSION@/$VERSION/g" -e "s/@ALPHA@/$N/g" tools/win7/alpha/README-ALPHA.txt > "$WORK/README-ALPHA.txt"
text "$WORK/README-ALPHA.txt" "$DIR/README-ALPHA.txt"
crlf tools/win7/alpha/RapidR-Studio.bat "$DIR/RapidR-Studio.bat"
cp examples/basics/hello.rr examples/basics/language.rr tools/win7/tests/form_auto.rr tools/win7/tests/dll_calls.rr "$DIR/tests/"
for b in run-tests.bat run-studio.bat; do crlf "tools/win7/tests/$b" "$DIR/tests/$b"; done

# RapidR's own licence texts, and what the toolchain links into the programs
for f in LICENSE NOTICE LEGAL.md THIRD_PARTY_NOTICES.md LICENSES.md; do text "$f" "$DIR/$f"; done
text tools/win7/licenses/README.txt "$DIR/licenses/README.txt"
mkdir -p "$DIR/licenses/mingw-w64" "$DIR/licenses/llvm"
for f in tools/win7/licenses/mingw-w64/*; do text "$f" "$DIR/licenses/mingw-w64/$(basename "$f")"; done
text tools/win7/licenses/llvm/LICENSE.TXT "$DIR/licenses/llvm/LICENSE.TXT"

# THIRD-PARTY-NOTICES.txt: the full licence texts and copyright lines of every
# library and of Rust's standard library, made by the project's own notices tool
# (`rapidr notices tools-windows`: the Windows set of RapidR's programs, a
# superset of this build's crates) from this checkout's Cargo.lock. It reads the checkout.
mkdir -p "$WORK/notices-env"
RAPIDR_HOME="$ROOT" RAPIDR_REGISTRY="$WORK/notices-env/registry" RAPIDR_PRINT_TO="$WORK/notices-env/print" RAPIDR_AI_KEYSTORE=memory \
    "$HOST_RAPIDR" notices tools-windows -o "$WORK/THIRD-PARTY-NOTICES.txt"
# every crate of THIS build's graph must be in it (the i686 graph has windows_i686_gnu,
# which the tool's x86_64/aarch64 sets don't; it is windows-rs's, same text as its siblings)
# (the runner stub's graph too: the same crates as the program's, less)
for pkg in rapidr-cli rapidr-runner-stub; do
cargo +nightly tree --target i686-win7-windows-gnu -e normal --prefix none --offline -p "$pkg" 2>/dev/null \
    | python3 -I -c '
import re, sys
notes = open(sys.argv[1], encoding="utf-8").read()
have = set(re.findall(r"^(\S+) \d[\w.+-]*$", notes, re.M))
extra = []
for line in sys.stdin:
    p = line.split()
    if len(p) < 2 or p[0].startswith("rapidr-") or p[0] in have:
        continue
    if p[0].startswith("windows_"):
        extra.append((p[0], p[1].lstrip("v")))
    else:
        sys.exit("not in THIRD-PARTY-NOTICES.txt: " + p[0] + " " + p[1])
if extra:
    with open(sys.argv[1], "a", encoding="utf-8") as f:
        f.write("\n\nADDED FOR THE 32-BIT WINDOWS 7 BUILD\n" + "-" * 78 + "\n")
        for name, version in sorted(set(extra)):
            f.write(f"{name} {version}\n    Licence: MIT OR Apache-2.0 (used under MIT): the same text and copyright line\n    as the windows_* crates listed in PART 1 (Microsoft Corporation, windows-rs).\n    https://github.com/microsoft/windows-rs\n")
        print("added to the notices:", ", ".join(n for n, _ in sorted(set(extra))))
' "$WORK/THIRD-PARTY-NOTICES.txt"
done
text "$WORK/THIRD-PARTY-NOTICES.txt" "$DIR/THIRD-PARTY-NOTICES.txt"
# what a program that Build makes carries beside it (home.rs: lib\rapidr\notices\<target>.txt): the same
# text, a superset of the runner's graph (the runner is the program's own crates and the toolchain's pieces)
cp "$WORK/THIRD-PARTY-NOTICES.txt" "$LIB/notices/windows-x86.txt"

# the zip (a folder inside, so unzipping makes one folder), and its checksum
( cd "$WORK" && zip -qr -X -9 "$NAME.zip" "$NAME" )
cp "$WORK/$NAME.zip" "$OUT/$NAME.zip"
( cd "$OUT" && shasum -a 256 "$NAME.zip" > "$NAME.zip.sha256" && cat "$NAME.zip.sha256" )
ls -l "$OUT/$NAME.zip"

# the VM's share, if asked for (tools/win7/serve.sh serves it; its index lists
# the files) and a small disc image with the zip, for a VM with no network
if [ -n "${WIN7_SHARE:-}" ] && [ -f "$WIN7_SHARE/.win7-share" ]; then
    cp "$OUT/$NAME.zip" "$OUT/$NAME.zip.sha256" "$WIN7_SHARE/"
    sed -e "s/@VERSION@/$VERSION/g" -e "s/@ALPHA@/$N/g" tools/win7/alpha/VM-STEPS.txt > "$WORK/VM-STEPS.txt"
    crlf "$WORK/VM-STEPS.txt" "$WIN7_SHARE/VM-STEPS-ALPHA.txt"
    ( cd "$WIN7_SHARE" && python3 -I - <<'EOF'
import os
rows = []
for dp, dn, fn in os.walk("."):
    dn[:] = sorted(d for d in dn if not d.startswith("."))
    for f in sorted(fn):
        if f.startswith("."):
            continue
        p = os.path.join(dp, f)[2:]
        rows.append(f'<li><a href="{p}">{p}</a> ({os.path.getsize(p):,} bytes)</li>')
open("index.html", "w").write("<!doctype html><meta charset=utf-8><title>RapidR for Windows 7</title><h1>RapidR for Windows 7</h1><p>Right-click a file, Save target as. For the alpha: VM-STEPS-ALPHA.txt first.</p><ul>" + "\n".join(rows) + "</ul>")
EOF
    )
    echo "copied to $WIN7_SHARE"
    if [ -n "${WIN7_ISO_DIR:-}" ]; then
        mkdir -p "$WORK/iso" "$WIN7_ISO_DIR"
        rm -rf "$WORK/iso"/* "$WIN7_ISO_DIR/RAPIDR-ALPHA1.iso"
        cp "$OUT/$NAME.zip" "$OUT/$NAME.zip.sha256" "$WORK/iso/"
        crlf "$WORK/VM-STEPS.txt" "$WORK/iso/VM-STEPS-ALPHA.txt"
        hdiutil makehybrid -quiet -iso -joliet -default-volume-name RAPIDR_ALPHA1 -o "$WIN7_ISO_DIR/RAPIDR-ALPHA1.iso" "$WORK/iso"
        ls -l "$WIN7_ISO_DIR/RAPIDR-ALPHA1.iso"
        rm -rf "$WORK/iso"
    fi
fi
rm -rf "$DIR"
