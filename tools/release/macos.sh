#!/bin/bash
# macOS: RapidR Studio.app (the IDE, the `rapidr` command, the SDK) and RapidR
# Runtime.app (runs programs: .rrbc on a double click, "Open With" on .rr / .bas),
# universal (arm64 + x86_64), in two disk images:
#
#   dist/<ver>/out/RapidR-<ver>-macos-universal.dmg          both apps
#   dist/<ver>/out/RapidR-Runtime-<ver>-macos-universal.dmg  the runtime only
#
# Each image opens on the app(s) and an Applications alias (drag the app onto it), and a
# Licenses folder (LICENSE, NOTICE, LEGAL.md, LICENSES.md, THIRD_PARTY_NOTICES.md).
# The `rapidr` command line is inside the app (Contents/MacOS/rapidr); its first
# run of `rapidr setup` offers to link it into /usr/local/bin or ~/.local/bin.
#
#   tools/release/macos.sh      (inputs: prepare.sh's dist/<ver>/prep/; run on the Mac)
#   tools/release/macos.sh --sign "Developer ID Application: Name (TEAMID)" --notarize <profile>
#
# Signing hooks, OFF unless set (the first release ships ad hoc signed only; this script
# never creates a key or a certificate):
#   --sign "<identity>"     or env RAPIDR_MAC_SIGN_IDENTITY: a Developer ID Application identity
#                           (`security find-identity -v -p codesigning`). Signs inside-out with the
#                           hardened runtime and a timestamp, then the disk images.
#   --notarize <profile>    or env RAPIDR_MAC_NOTARY_PROFILE: a notarytool keychain profile (made
#                           once: `xcrun notarytool store-credentials <profile> --apple-id … --team-id …`).
#                           Submits each .dmg with `xcrun notarytool submit --wait` and staples it.
#   Without them the apps are signed ad hoc: they run on the Mac that built them and, downloaded,
#   macOS asks once for System Settings > Privacy & Security > Open Anyway (docs/manual/getting-started.md).
#
# Every executable in the apps is universal — the CLI, the launcher and the one
# runner `--interp` executables start from (runners/macos/) — built for macOS
# MACOSX_DEPLOYMENT_TARGET (10.13 on Intel; 11.0 on Apple silicon, its first). Nothing
# Intel-only: macOS 28 drops Rosetta. The scan at the end checks every Mach-O
# statically (lipo -archs, otool's LC_BUILD_VERSION); nothing x86_64 is run here.
#
# After tools/release/prepare.sh. Unsigned (the default) the apps are signed
# ad hoc: they run here and, downloaded, are allowed once in System Settings >
# Privacy & Security > Open Anyway (docs/manual/getting-started.md). Needs: Xcode's command line tools (lipo,
# codesign, hdiutil), `rustup target add x86_64-apple-darwin` for universal.
# About 6 GB of disk while it runs (two release builds); work/ is removed.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
# (unsigned unless one is given: the flags, or the environment)
SIGN="${RAPIDR_MAC_SIGN_IDENTITY:-}" NOTARY="${RAPIDR_MAC_NOTARY_PROFILE:-}"
while [ $# -gt 0 ]; do
    case "$1" in
        --sign) SIGN="$2"; shift 2 ;;
        --notarize) NOTARY="$2"; shift 2 ;;
        *) die "unknown option $1" ;;
    esac
done
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"
# (the code shipped is the archived commit's: release scripts and docs may have moved on)
git diff --quiet "$(cat "$PREP/commit")" HEAD -- crates interpreter examples ide Cargo.toml Cargo.lock LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md \
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
make_app "RapidR Studio" "$W/sdk" RapidR-Studio.plist
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
scan "$W/apps/RapidR Studio.app"
scan "$W/apps/RapidR Runtime.app"

step "sign (${SIGN:-ad hoc})"
sign "$W/apps/RapidR Studio.app"
sign "$W/apps/RapidR Runtime.app"

# The disk image's Finder window (icon view, the app on the left and the Applications alias on
# the right: drag one onto the other), set through Finder's AppleScript on a writable copy, which
# Finder keeps in the image's .DS_Store. Best effort: with no Finder (an ssh session) the image
# is still right, only its icons are sorted by name.
layout_dmg() {
    local rw="$1" vol="$2"; shift 2
    hdiutil detach -quiet "/Volumes/$vol" 2>/dev/null || true
    hdiutil attach -quiet -noverify -noautoopen -mountpoint "/Volumes/$vol" "$rw" || return 0
    local first="$1" row2=""
    local positions="set position of item \"$first.app\" of container window to {150, 120}"
    positions+=$'\n'"set position of item \"Applications\" of container window to {450, 120}"
    if [ $# -gt 1 ]; then
        positions+=$'\n'"set position of item \"$2.app\" of container window to {150, 270}"
        positions+=$'\n'"set position of item \"Licenses\" of container window to {450, 270}"
    else
        positions+=$'\n'"set position of item \"Licenses\" of container window to {300, 270}"
    fi
    osascript <<EOF || echo "  (the image's window layout was skipped: no Finder)"
tell application "Finder"
  tell disk "$vol"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {200, 120, 800, 580}
    set opts to the icon view options of container window
    set arrangement of opts to not arranged
    set icon size of opts to 112
    set text size of opts to 13
    $positions
    update without registering applications
    delay 1
    close
  end tell
end tell
EOF
    sync; sleep 1
    local i; for i in 1 2 3 4 5; do hdiutil detach -quiet "/Volumes/$vol" 2>/dev/null && return 0; sleep 2; done
    hdiutil detach -quiet -force "/Volumes/$vol" || true
}
# <file> <volume name> <app> [<another app>]: the apps, an Applications alias and a Licenses
# folder, in a compressed (ULFO) image; signed and notarized when asked.
make_dmg() {
    local file="$1" vol="$2"; shift 2
    local src="$W/dmg-$vol" rw="$W/$vol.rw.dmg"
    rm -rf "$src" && mkdir -p "$src/Licenses"
    for a in "$@"; do cp -R "$W/apps/$a.app" "$src/"; done
    ln -s /Applications "$src/Applications"
    cp LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md "$src/Licenses/"
    rm -f "$OUT/$file" "$rw"
    hdiutil create -quiet -volname "$vol" -srcfolder "$src" -fs HFS+ -format UDRW "$rw"
    layout_dmg "$rw" "$vol" "$@"
    hdiutil convert -quiet "$rw" -format ULFO -o "$OUT/$file"
    rm -rf "$src" "$rw"
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
make_dmg "RapidR-$VERSION-macos-$ARCH.dmg" "RapidR $VERSION" "RapidR Studio" "RapidR Runtime"
make_dmg "RapidR-Runtime-$VERSION-macos-$ARCH.dmg" "RapidR Runtime $VERSION" "RapidR Runtime"
