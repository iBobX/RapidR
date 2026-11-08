#!/bin/bash
# The runners other systems' Studios download (docs/build-for-every-platform-plan.md,
# section 3): one runner file per target and the list that covers them, made
# from the packages the platform scripts already built. Run it after
# macos.sh, linux-vm.sh, the Windows build and tools/win7/alpha.sh, before finish.sh:
#
#   tools/release/runners.sh [--allow-missing]
#
# Makes, in dist/<ver>/out/:
#   runners/<target>.zst        windows-x86_64, windows-aarch64, windows-x86, macos, linux-x86_64,
#                               linux-aarch64, web: a zstd-compressed tar (tools/release/runners.py)
#   runners-manifest.json       their version, target, size, SHA-256 (and contents)
# finish.sh lists the manifest in SHA256SUMS and signs it (runners-manifest.json.minisig, with
# RAPIDR_MINISIGN_KEY: yours; this script never touches a key). The runners/ folder is NOT part of the
# GitHub release: it goes to the runners host, next to the signed manifest, at
# https://studio.rapidr.dev/runners/<version>/ (uploaded by hand: nothing here uploads).
# After finish.sh signed it, `target/release/rapidr runners check dist/<ver>/out` is the gate: it reads
# the list and every runner as a Studio does (signature, version, SHA-256, unpacking), offline.
#
# Where each comes from (the same files the installs ship, so a downloaded runner and an installed one
# are the same program):
#   macos, web      RapidR-<ver>-macos-universal.dmg   (RapidR Studio.app/Contents/lib/rapidr)
#   linux-*         rapidr-<ver>-linux-<arch>.tar.gz
#   windows-x86     RapidR-<ver>-win7-x86-alpha*.zip   (tools/win7/alpha.sh builds the runners and their notices)
#   windows-x86_64, windows-aarch64
#                   dist/<ver>/prep/runners/windows-<arch>.tar, which the Windows build writes
#                   (windows.ps1: runners.py tar) and you fetch:
#                     tools/release/windows-vm.sh fetch windows-x86_64.tar  dist/<ver>/prep/runners/windows-x86_64.tar
#                     tools/release/windows-vm.sh fetch windows-aarch64.tar dist/<ver>/prep/runners/windows-aarch64.tar
#                   (the installers are Inno Setup files: they can't be opened on a Mac)
#
# Without --allow-missing, a target with no source stops the script: a release's list has all seven.
# Needs: python3, zstd (brew install zstd), hdiutil (macOS), tar, unzip. About a minute per target
# (zstd -19), 600 MB of scratch space in dist/<ver>/work/. Nothing is uploaded or signed here.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
ALLOW_MISSING=0
while [ $# -gt 0 ]; do
    case "$1" in
        --allow-missing) ALLOW_MISSING=1; shift ;;
        *) die "unknown option $1" ;;
    esac
done
need python3 "the release scripts"
need zstd "brew install zstd"
RUNNERS="$OUT/runners"
W="$WORK/runners"
rm -rf "$W" && mkdir -p "$W" "$RUNNERS"
MNT=""
cleanup() { [ -z "$MNT" ] || hdiutil detach -quiet "$MNT" 2>/dev/null || true; rm -rf "$W"; }
trap cleanup EXIT
PY="python3 tools/release/runners.py"
START=$SECONDS
missing=()
t0=$SECONDS
lap() { echo "   ($((SECONDS - t0)) s)"; t0=$SECONDS; }

step "macOS and the web, from the disk image"
DMG="$OUT/RapidR-$VERSION-macos-universal.dmg"
if [ -f "$DMG" ]; then
    MNT="$W/dmg"; mkdir -p "$MNT"
    hdiutil attach -quiet -nobrowse -readonly -mountpoint "$MNT" "$DMG"
    $PY prefix "$MNT/RapidR Studio.app/Contents/lib/rapidr" --out "$RUNNERS" --only macos web
    hdiutil detach -quiet "$MNT"; MNT=""
else
    echo "  no $DMG"; missing+=(macos web)
fi
lap

for arch in x86_64 aarch64; do
    step "linux-$arch, from the package"
    TGZ="$OUT/rapidr-$VERSION-linux-$arch.tar.gz"
    if [ -f "$TGZ" ]; then
        mkdir -p "$W/linux-$arch"
        base="rapidr-$VERSION-linux-$arch/lib/rapidr"
        tar -xzf "$TGZ" -C "$W/linux-$arch" "$base/runners/linux-$arch" "$base/notices/linux-$arch.txt"
        $PY prefix "$W/linux-$arch/$base" --out "$RUNNERS" --only "linux-$arch"
    else
        echo "  no $TGZ"; missing+=("linux-$arch")
    fi
    lap
done

step "windows-x86 (Windows 7 and later, 32-bit), from the Windows 7 package"
ZIP="$(ls -t "$OUT"/RapidR-"$VERSION"-win7-x86-alpha*.zip 2>/dev/null | sed -n 1p || true)"
if [ -n "$ZIP" ]; then
    mkdir -p "$W/win7"
    # (sed, not head: head would end the pipe early and pipefail would stop the script)
    base="$(unzip -Z1 "$ZIP" | sed -n '1s,/.*,,p')/lib/rapidr"
    unzip -q "$ZIP" "$base/runners/windows-x86/*" "$base/notices/windows-x86.txt" -d "$W/win7"
    $PY prefix "$W/win7/$base" --out "$RUNNERS" --only windows-x86
else
    echo "  no RapidR-$VERSION-win7-x86-alpha*.zip (tools/win7/alpha.sh makes it)"; missing+=(windows-x86)
fi
lap

for arch in x86_64 aarch64; do
    step "windows-$arch, from the Windows build's tar"
    TAR="$PREP/runners/windows-$arch.tar"
    if [ -f "$TAR" ]; then
        $PY bundle --target "windows-$arch" --tar "$TAR" --out "$RUNNERS"
    else
        echo "  no $TAR (tools/release/windows-vm.sh fetch windows-$arch.tar $TAR)"; missing+=("windows-$arch")
    fi
    lap
done

if [ ${#missing[@]} -gt 0 ]; then
    if [ $ALLOW_MISSING = 1 ]; then
        echo "== missing (allowed): ${missing[*]}"
    else
        die "no runner for: ${missing[*]} (--allow-missing to list the others anyway)"
    fi
fi

step "runners-manifest.json"
# (a signature from an earlier run is of the earlier list: finish.sh signs this one)
rm -f "$OUT/runners-manifest.json.minisig"
$PY manifest --dir "$RUNNERS" --out "$OUT/runners-manifest.json" --version "$VERSION"
$PY check --dir "$RUNNERS" --manifest "$OUT/runners-manifest.json" --version "$VERSION" || [ $ALLOW_MISSING = 1 ]
du -sh "$RUNNERS" | sed 's/^/  /'
echo "done in $((SECONDS - START)) s: $RUNNERS and $OUT/runners-manifest.json"
