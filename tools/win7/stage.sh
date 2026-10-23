#!/bin/bash
# Puts a Windows 7 test drop together: the built rapidr.exe, Studio's files,
# the smoke tests, and (if present) the QEMU guest agent kit; then makes the
# disc image (ISO) as the fallback to the host-only HTTP transfer.
#
#   tools/win7/stage.sh [i686|x86_64]
#
# Result:
#   /Volumes/RapidRBuild/win7-share/            what tools/win7/serve.sh serves
#   /Volumes/RapidRBuild/win7-share-iso/RAPIDR-WIN7.iso   the same, as a disc
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCH="${1:-i686}"
TARGET="${ARCH}-win7-windows-gnu"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
EXE="$TD/$TARGET/release/rapidr.exe"
SHARE="${WIN7_SHARE:-/Volumes/RapidRBuild/win7-share}"
ISODIR="${WIN7_ISO_DIR:-/Volumes/RapidRBuild/win7-share-iso}"
GUEST="${WIN7_GUEST_KIT:-/Volumes/RapidRBuild/wt-win7-1/stage-virtio}"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
[ -f "$EXE" ] || { echo "build first: tools/win7/build.sh $ARCH (missing $EXE)" >&2; exit 1; }

# only ever empty the folder this script made (it carries a marker)
if [ -e "$SHARE" ] && [ ! -f "$SHARE/.win7-share" ]; then
    echo "$SHARE exists and is not a stage folder; refusing" >&2; exit 1
fi
rm -rf "$SHARE/rapidr" "$SHARE/guest" "$SHARE/READ-ME.txt" "$SHARE/index.html" "$SHARE/rapidr-win7.zip" "$SHARE/guest-agent-kit.zip"
mkdir -p "$SHARE/rapidr/tests" "$SHARE/guest" "$ISODIR"
touch "$SHARE/.win7-share"

# the program, without its symbols (80 MB -> about half)
cp "$EXE" "$SHARE/rapidr/rapidr.exe"
"$LLVM_MINGW/bin/llvm-strip" "$SHARE/rapidr/rapidr.exe"

# Studio's own files (it runs from source: ide/studio.rr) and the examples
cd "$ROOT"
git ls-files ide examples | while IFS= read -r f; do
    mkdir -p "$SHARE/rapidr/$(dirname "$f")"
    cp "$f" "$SHARE/rapidr/$f"
done

# the tests (batch files with Windows line ends)
cp examples/basics/hello.rr examples/basics/language.rr tools/win7/tests/form_auto.rr "$SHARE/rapidr/tests/"
for b in run-tests.bat run-studio.bat; do
    sed 's/$/\r/' "tools/win7/tests/$b" > "$SHARE/rapidr/tests/$b"
done

# the QEMU guest agent kit (virtio-win 0.1.173: the last with Windows 7 drivers)
if [ -d "$GUEST" ]; then
    cp -R "$GUEST/." "$SHARE/guest/"
fi

sed 's/$/\r/' tools/win7/READ-ME.txt > "$SHARE/READ-ME.txt"

# one file each for the browser route (Windows 7 opens .zip in Explorer)
( cd "$SHARE" && zip -qr rapidr-win7.zip rapidr && { [ ! -d guest ] || zip -qr guest-agent-kit.zip guest; } )

# an index page for the browser in the VM (python's own listing is plain too)
( cd "$SHARE" && python3 -I - <<'EOF'
import os
rows = []
for dp, dn, fn in os.walk("."):
    dn[:] = sorted(d for d in dn if not d.startswith("."))
    for f in sorted(fn):
        if f.startswith("."):
            continue
        p = os.path.join(dp, f)[2:]
        rows.append(f'<li><a href="{p}">{p}</a> ({os.path.getsize(p):,} bytes)</li>')
open("index.html", "w").write("<!doctype html><meta charset=utf-8><title>RapidR for Windows 7</title><h1>RapidR for Windows 7</h1><p>Right-click a file, Save target as. Read READ-ME.txt first.</p><ul>" + "\n".join(rows) + "</ul>")
EOF
)

rm -f "$ISODIR/RAPIDR-WIN7.iso"
hdiutil makehybrid -quiet -iso -joliet -default-volume-name RAPIDR_WIN7 -o "$ISODIR/RAPIDR-WIN7.iso" "$SHARE"
ls -l "$ISODIR/RAPIDR-WIN7.iso"
du -sh "$SHARE"
