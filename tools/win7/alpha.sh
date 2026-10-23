#!/bin/bash
# Builds the Windows 7 / 8 / 8.1 ALPHA download: one zip a person unzips and
# runs (rapidr.exe, the Studio launcher, Studio's files, the examples, the
# smoke tests, README-ALPHA.txt, every licence text), plus its SHA-256.
#
#   tools/win7/alpha.sh [ALPHA_NUMBER]            (default 1)
#
# Build the program first (tools/win7/build.sh i686). Environment (optional):
#   CARGO_TARGET_DIR  where build.sh built (default ./target)
#   ALPHA_OUT         where the zip goes (default ./dist/<version>-win7-alpha<N>)
#   ALPHA_WORK        scratch (default /Volumes/RapidRBuild/wt-win7-alpha/stage)
#   LLVM_MINGW        the toolchain (llvm-strip, for the symbols)
# Also puts the zip (and VM-STEPS-ALPHA.txt) on the test VM's host-only share
# (tools/win7/serve.sh) when WIN7_SHARE is set, and makes
# $WIN7_ISO_DIR/RAPIDR-ALPHA1.iso when WIN7_ISO_DIR is set too; nothing is
# started or served here.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
N="${1:-1}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
EXE="$TD/i686-win7-windows-gnu/release/rapidr.exe"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
OUT="${ALPHA_OUT:-$ROOT/dist/$VERSION-win7-alpha$N}"
WORK="${ALPHA_WORK:-/Volumes/RapidRBuild/wt-win7-alpha/stage}"
NAME="RapidR-$VERSION-win7-x86-alpha$N"
[ -f "$EXE" ] || { echo "build first: tools/win7/build.sh i686 (missing $EXE)" >&2; exit 1; }

# only ever empty a folder this script made (it carries a marker)
if [ -e "$WORK" ] && [ ! -f "$WORK/.alpha-stage" ]; then
    echo "$WORK exists and is not an alpha stage folder; refusing" >&2; exit 1
fi
mkdir -p "$WORK"; touch "$WORK/.alpha-stage"
DIR="$WORK/$NAME"
rm -rf "$DIR" "$WORK/$NAME.zip"
mkdir -p "$DIR/tests" "$DIR/licenses" "$OUT"

# the program, without its symbols (80 MB -> about half); version info stays
cp "$EXE" "$DIR/rapidr.exe"
"$LLVM_MINGW/bin/llvm-strip" "$DIR/rapidr.exe"
"$LLVM_MINGW/bin/llvm-readobj" --coff-resources "$DIR/rapidr.exe" | grep -q 'VERSIONINFO' || { echo "rapidr.exe has no version information" >&2; exit 1; }

crlf() { sed 's/$/\r/' "$1" > "$2"; }   # (Windows 7's Notepad needs Windows line ends)
# ... and for text with accents, marked as UTF-8 (Notepad there reads it as ANSI otherwise)
text() { { printf '\357\273\277'; sed 's/$/\r/' "$1"; } > "$2"; }

cd "$ROOT"
# Studio runs from its own source (ide/studio.rr) and the examples
git ls-files ide examples | while IFS= read -r f; do
    mkdir -p "$DIR/$(dirname "$f")"
    cp "$f" "$DIR/$f"
done

# the launcher, the readme and the smoke tests
text tools/win7/alpha/README-ALPHA.txt "$DIR/README-ALPHA.txt"
crlf tools/win7/alpha/RapidR-Studio.bat "$DIR/RapidR-Studio.bat"
cp examples/basics/hello.rr examples/basics/language.rr tools/win7/tests/form_auto.rr "$DIR/tests/"
for b in run-tests.bat run-studio.bat; do crlf "tools/win7/tests/$b" "$DIR/tests/$b"; done

# RapidR's own licence texts, and what the toolchain links into the .exe
for f in LICENSE NOTICE LEGAL.md THIRD_PARTY_NOTICES.md LICENSES.md; do text "$f" "$DIR/$f"; done
text tools/win7/licenses/README.txt "$DIR/licenses/README.txt"
mkdir -p "$DIR/licenses/mingw-w64" "$DIR/licenses/llvm"
for f in tools/win7/licenses/mingw-w64/*; do text "$f" "$DIR/licenses/mingw-w64/$(basename "$f")"; done
text tools/win7/licenses/llvm/LICENSE.TXT "$DIR/licenses/llvm/LICENSE.TXT"

# the zip (a folder inside, so unzipping makes one folder), and its checksum
( cd "$WORK" && zip -qr -X -9 "$NAME.zip" "$NAME" )
cp "$WORK/$NAME.zip" "$OUT/$NAME.zip"
( cd "$OUT" && shasum -a 256 "$NAME.zip" > "$NAME.zip.sha256" && cat "$NAME.zip.sha256" )
ls -l "$OUT/$NAME.zip"

# the VM's share, if asked for (tools/win7/serve.sh serves it; its index lists
# the files) and a small disc image with the zip, for a VM with no network
if [ -n "${WIN7_SHARE:-}" ] && [ -f "$WIN7_SHARE/.win7-share" ]; then
    cp "$OUT/$NAME.zip" "$OUT/$NAME.zip.sha256" "$WIN7_SHARE/"
    crlf tools/win7/alpha/VM-STEPS.txt "$WIN7_SHARE/VM-STEPS-ALPHA.txt"
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
        crlf tools/win7/alpha/VM-STEPS.txt "$WORK/iso/VM-STEPS-ALPHA.txt"
        hdiutil makehybrid -quiet -iso -joliet -default-volume-name RAPIDR_ALPHA1 -o "$WIN7_ISO_DIR/RAPIDR-ALPHA1.iso" "$WORK/iso"
        ls -l "$WIN7_ISO_DIR/RAPIDR-ALPHA1.iso"
        rm -rf "$WORK/iso"
    fi
fi
rm -rf "$DIR"
