#!/bin/bash
# macOS: RapidR.app (the IDE, the CLI, the SDK) and RapidR Runtime.app (runs
# programs: .rrbc on a double click, "Open With" on .rr / .bas), universal
# (arm64 + x86_64), in two disk images:
#
#   dist/<ver>/out/RapidR-<ver>-macos-universal.dmg          both apps
#   dist/<ver>/out/RapidR-Runtime-<ver>-macos-universal.dmg  the runtime only
#
#   tools/release/macos.sh [--sign "Developer ID Application: Name (TEAMID)"] [--notarize <notarytool keychain profile>]
#
# Every executable in the apps is universal — the CLI, the launcher and the one
# runner `--interp` executables start from (runners/macos/) — built for macOS
# MACOSX_DEPLOYMENT_TARGET (10.13 on Intel; 11.0 on Apple silicon, its first). Nothing
# Intel-only: macOS 28 drops Rosetta. The scan at the end checks every Mach-O
# statically (lipo -archs, otool's LC_BUILD_VERSION); nothing x86_64 is run here.
#
# After tools/release/prepare.sh. Unsigned (the default) the apps are signed
# ad hoc: they run here and, downloaded, open with right-click > Open (see
# docs/release-packaging.md). Needs: Xcode's command line tools (lipo,
# codesign, hdiutil), `rustup target add x86_64-apple-darwin` for universal.
# About 6 GB of disk while it runs (two release builds); work/ is removed.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
SIGN="" NOTARY=""
while [ $# -gt 0 ]; do
    case "$1" in
        --sign) SIGN="$2"; shift 2 ;;
        --notarize) NOTARY="$2"; shift 2 ;;
        *) die "unknown option $1" ;;
    esac
done
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"
# (the code shipped is the archived commit's: release scripts and docs may have moved on)
git diff --quiet "$(cat "$PREP/commit")" HEAD -- crates interpreter examples web-ide Cargo.toml Cargo.lock LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md \
    || die "the code differs from the commit prepare.sh archived ($(cat "$PREP/commit")): run prepare.sh again"
[ -n "$NOTARY" ] && [ -z "$SIGN" ] && die "--notarize needs --sign (a Developer ID)"
need lipo "Xcode command line tools"; need hdiutil "macOS"; need codesign "Xcode command line tools"
ARCH=universal ARCHS="aarch64 x86_64"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-10.13}"
for a in $ARCHS; do
    rustup target list --installed | grep -qx "$a-apple-darwin" || die "rustup target add $a-apple-darwin"
done
W="$WORK/macos"
rm -rf "$W" && mkdir -p "$W/bin"
trap 'rm -rf "$W"' EXIT

step "build ($ARCHS, macOS $MACOSX_DEPLOYMENT_TARGET on)"
for a in $ARCHS; do
    t="$a-apple-darwin"
    cargo build -q --locked --release --target "$t" -p rapidr-cli -p rapidr-launcher
    cargo build -q --locked --profile runner --target "$t" -p rapidr-runner-stub --bin rapidrintr-runner
done
mkdir -p "$W/runner"
for b in rapidr rapidrw; do
    lipo -create $(for a in $ARCHS; do echo "target/$a-apple-darwin/release/$b"; done) -output "$W/bin/$b"
done
lipo -create $(for a in $ARCHS; do echo "target/$a-apple-darwin/runner/rapidrintr-runner"; done) -output "$W/runner/rapidrintr-runner"
RUNNERS=(--runner "macos=$W/runner")
lipo -info "$W/bin/rapidr" "$W/runner/rapidrintr-runner"

step "the home: the runtime's sources, their crates vendored"
mkdir -p "$W/src" && tar -x -C "$W/src" -f "$PREP/src.tar"
python3 tools/release/home.py --os macos --src "$W/src" --out "$W/home"
rm -rf "$W/src"

step "stage"
python3 tools/release/stage.py --kind sdk --os macos --out "$W/sdk" --bin "$W/bin" --home "$W/home" \
    "${RUNNERS[@]}" --web "$PREP/web-runtime" --ide "$PREP/rapidr-ide.rrbc"
python3 tools/release/stage.py --kind runtime --os macos --out "$W/runtime" --bin "$W/bin" --version "$VERSION" \
    --rust "$(sed -n 's/^rust = "\(.*\)"/\1/p' "$W/home/release.toml")"
rm -rf "$W/home"

# <name>.app from a staged prefix: Contents/MacOS = bin/, Contents/lib = lib/
# (the CLI finds its home at <exe>/../lib/rapidr), Contents/Resources/doc.
make_app() {
    local name="$1" stage="$2" plist="$3" app="$W/apps/$1.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    sed "s/@VERSION@/$VERSION/g" "tools/release/macos/$plist" > "$app/Contents/Info.plist"
    plutil -lint -s "$app/Contents/Info.plist"
    printf 'APPL????' > "$app/Contents/PkgInfo"
    cp "$stage/bin/rapidr" "$stage/bin/rapidrw" "$app/Contents/MacOS/"
    cp -R "$stage/lib" "$app/Contents/lib"
    cp -R "$stage/share/doc/rapidr" "$app/Contents/Resources/doc"
    # (design/brand/icons: the app's own, and the file types')
    cp design/brand/icons/macos/*.icns "$app/Contents/Resources/"
}
step "apps"
make_app "RapidR" "$W/sdk" RapidR.plist
make_app "RapidR Runtime" "$W/runtime" RapidR-Runtime.plist

# Inside out: every executable, then the bundle. A Developer ID signs with
# the hardened runtime and a timestamp (notarization needs both); without
# one, ad hoc (`-`): consistent, runs here, not trusted elsewhere.
sign() {
    local app="$1"
    local opts=(--force --sign "${SIGN:--}")
    [ -n "$SIGN" ] && opts+=(--options runtime --timestamp)
    find "$app/Contents/lib" -type f -perm -u+x -name 'rapidrintr-runner*' -print0 | while IFS= read -r -d '' f; do
        codesign "${opts[@]}" "$f"
    done
    codesign "${opts[@]}" --entitlements tools/release/macos/rapidr.entitlements "$app/Contents/MacOS/rapidr"
    codesign "${opts[@]}" "$app/Contents/MacOS/rapidrw"
    codesign "${opts[@]}" "$app"
    codesign --verify --deep --strict "$app"
}
# A slice's minimum macOS (LC_BUILD_VERSION's minos; LC_VERSION_MIN_MACOSX's version
# for an older target).
minos() {
    otool -arch "$1" -l "$2" | awk '/LC_BUILD_VERSION|LC_VERSION_MIN_MACOSX/{b=1} b&&/^ *(minos|version) /{print $2; exit}'
}
# Every Mach-O in an app: universal (arm64 and x86_64), its minimum macOS.
scan() {
    local app="$1" f archs bad=0
    while IFS= read -r -d '' f; do
        file -b "$f" | grep -q "Mach-O" || continue
        archs="$(lipo -archs "$f")"
        [[ " $archs " == *" arm64 "* && " $archs " == *" x86_64 "* ]] || { echo "  NOT UNIVERSAL: $f ($archs)"; bad=1; }
        echo "  ${f#"$W/apps/"}: $archs; minimum macOS $(minos arm64 "$f") (arm64), $(minos x86_64 "$f") (x86_64)"
    done < <(find "$app" -type f -perm -u+x -print0)
    [ $bad = 0 ] || die "$app has executables that aren't universal"
}
step "every executable universal"
scan "$W/apps/RapidR.app"
scan "$W/apps/RapidR Runtime.app"

step "sign (${SIGN:-ad hoc})"
sign "$W/apps/RapidR.app"
sign "$W/apps/RapidR Runtime.app"

make_dmg() {
    local file="$1" vol="$2"; shift 2
    local src="$W/dmg-$vol"
    rm -rf "$src" && mkdir -p "$src"
    for a in "$@"; do cp -R "$W/apps/$a.app" "$src/"; done
    ln -s /Applications "$src/Applications"
    cp LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md "$src/"
    rm -f "$OUT/$file"
    hdiutil create -quiet -volname "$vol" -srcfolder "$src" -fs HFS+ -format ULFO "$OUT/$file"
    rm -rf "$src"
    if [ -n "$SIGN" ]; then
        codesign --sign "$SIGN" --timestamp "$OUT/$file"
    fi
    if [ -n "$NOTARY" ]; then
        xcrun notarytool submit "$OUT/$file" --keychain-profile "$NOTARY" --wait
        xcrun stapler staple "$OUT/$file"
    fi
    echo "wrote $OUT/$file ($(du -h "$OUT/$file" | cut -f1))"
}
step "disk images"
make_dmg "RapidR-$VERSION-macos-$ARCH.dmg" "RapidR $VERSION" "RapidR" "RapidR Runtime"
make_dmg "RapidR-Runtime-$VERSION-macos-$ARCH.dmg" "RapidR Runtime $VERSION" "RapidR Runtime"
