#!/bin/bash
# macOS: RapidR.app (the IDE, the CLI, the SDK) and RapidR Runtime.app (runs
# programs: .rrbc on a double click, "Open With" on .rr / .bas), universal
# (arm64 + x86_64), in two disk images:
#
#   dist/<ver>/out/RapidR-<ver>-macos-universal.dmg          both apps
#   dist/<ver>/out/RapidR-Runtime-<ver>-macos-universal.dmg  the runtime only
#
#   tools/release/macos.sh [--arch universal|arm64|x86_64]
#        [--sign "Developer ID Application: Name (TEAMID)"] [--notarize <notarytool keychain profile>]
#
# After tools/release/prepare.sh. Unsigned (the default) the apps are signed
# ad hoc: they run here and, downloaded, open with right-click > Open (see
# docs/release-packaging.md). Needs: Xcode's command line tools (lipo,
# codesign, hdiutil), `rustup target add x86_64-apple-darwin` for universal.
# About 6 GB of disk while it runs (two release builds); work/ is removed.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
ARCH=universal SIGN="" NOTARY=""
while [ $# -gt 0 ]; do
    case "$1" in
        --arch) ARCH="$2"; shift 2 ;;
        --sign) SIGN="$2"; shift 2 ;;
        --notarize) NOTARY="$2"; shift 2 ;;
        *) die "unknown option $1" ;;
    esac
done
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"
[ "$(git rev-parse HEAD)" = "$(cat "$PREP/commit")" ] || die "HEAD isn't the commit prepare.sh archived"
[ -n "$NOTARY" ] && [ -z "$SIGN" ] && die "--notarize needs --sign (a Developer ID)"
need lipo "Xcode command line tools"; need hdiutil "macOS"; need codesign "Xcode command line tools"
case "$ARCH" in
    universal) ARCHS="aarch64 x86_64" ;;
    arm64) ARCHS="aarch64" ;;
    x86_64) ARCHS="x86_64" ;;
    *) die "--arch universal, arm64 or x86_64" ;;
esac
for a in $ARCHS; do
    rustup target list --installed | grep -qx "$a-apple-darwin" || die "rustup target add $a-apple-darwin"
done
W="$WORK/macos"
rm -rf "$W" && mkdir -p "$W/bin"
trap 'rm -rf "$W"' EXIT

step "build ($ARCHS)"
RUNNERS=()
for a in $ARCHS; do
    t="$a-apple-darwin"
    cargo build -q --locked --release --target "$t" -p rapidr-cli -p rapidr-launcher
    cargo build -q --locked --profile runner --target "$t" -p rapidr-runner-stub --bin rapidrintr-runner
    RUNNERS+=(--runner "macos-$a=target/$t/runner")
done
for b in rapidr rapidrw; do
    lipo -create $(for a in $ARCHS; do echo "target/$a-apple-darwin/release/$b"; done) -output "$W/bin/$b"
done
lipo -info "$W/bin/rapidr"

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
    cp tools/release/icons/rapidr.icns tools/release/icons/rapidr-doc.icns "$app/Contents/Resources/"
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
step "sign (${SIGN:-ad hoc})"
sign "$W/apps/RapidR.app"
sign "$W/apps/RapidR Runtime.app"

make_dmg() {
    local file="$1" vol="$2"; shift 2
    local src="$W/dmg-$vol"
    rm -rf "$src" && mkdir -p "$src"
    for a in "$@"; do cp -R "$W/apps/$a.app" "$src/"; done
    ln -s /Applications "$src/Applications"
    cp LICENSE LICENSES.md THIRD_PARTY_NOTICES.md "$src/"
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
