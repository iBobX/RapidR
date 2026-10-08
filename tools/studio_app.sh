#!/bin/bash
# RapidR Studio as a macOS app from this checkout, for trying it out: a
# double-click (or `open`) starts the IDE with its icon in the Dock, the
# Studio being this checkout's ide/studio.rr (edit it, open the app again).
#
#   tools/studio_app.sh [--debug] [OUT]      # default: target/RapidR Studio.app, release build
#
# It is the release app's layout (tools/release/macos.sh): Contents/MacOS has
# rapidrw (the launcher Finder starts: it hands .rr / .bas files to the IDE)
# and rapidr, Contents/Resources RapidR's icons; Info.plist is RapidR.plist's
# with the name RapidR Studio and RAPIDR_HOME (LSEnvironment) naming this
# checkout — no lib/rapidr inside, so `rapidr` builds programs from the
# checkout as `cargo run` would. This Mac's architecture only; signed ad hoc.
# The release apps (universal, the SDK inside) come from tools/release/macos.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[ "$(uname)" = Darwin ] || { echo "studio_app.sh: macOS only (the Linux and Windows installs come from tools/release/)"; exit 1; }

PROFILE=release
if [ "${1:-}" = "--debug" ]; then PROFILE=debug; shift; fi
OUT="${1:-$ROOT/target/RapidR Studio.app}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

echo "== building rapidr and rapidrw ($PROFILE)"
if [ "$PROFILE" = release ]; then
    cargo build --release -p rapidr-cli -p rapidr-launcher
else
    cargo build -p rapidr-cli -p rapidr-launcher
fi
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"

echo "== $OUT"
rm -rf "$OUT"
mkdir -p "$OUT/Contents/MacOS" "$OUT/Contents/Resources"
cp "$TARGET/$PROFILE/rapidr" "$TARGET/$PROFILE/rapidrw" "$OUT/Contents/MacOS/"
cp design/brand/icons/macos/*.icns "$OUT/Contents/Resources/"
printf 'APPL????' > "$OUT/Contents/PkgInfo"
python3 - "$ROOT" "$VERSION" "$OUT/Contents/Info.plist" <<'PY'
import sys, xml.sax.saxutils as x
root, version, out = sys.argv[1:]
text = open(f"{root}/tools/release/macos/RapidR.plist").read().replace("@VERSION@", version)
text = text.replace("<string>RapidR</string>\n\t<key>CFBundleDisplayName</key>\n\t<string>RapidR</string>",
                    "<string>RapidR Studio</string>\n\t<key>CFBundleDisplayName</key>\n\t<string>RapidR Studio</string>")
text = text.replace("<string>io.github.ibobx.rapidr</string>", "<string>dev.rapidr.studio.checkout</string>")
env = (f"\t<key>LSEnvironment</key>\n\t<dict>\n\t\t<key>RAPIDR_HOME</key>\n\t\t<string>{x.escape(root)}</string>\n\t</dict>\n")
text = text.replace("\t<key>CFBundleDocumentTypes</key>", env + "\t<key>CFBundleDocumentTypes</key>", 1)
# (a checkout's app opens .rr / .bas files when asked, but owns no file
# type: an installed RapidR keeps them)
text = text.replace("<string>Owner</string>", "<string>Alternate</string>")
start = text.find("\t<key>UTExportedTypeDeclarations</key>")
end = text.find("\n\t</array>\n", start) + len("\n\t</array>\n")
text = text[:start] + text[end:]
assert "RapidR Studio" in text and "LSEnvironment" in text and "UTExported" not in text
open(out, "w").write(text)
PY
plutil -lint -s "$OUT/Contents/Info.plist"
codesign --force --sign - --entitlements tools/release/macos/rapidr.entitlements "$OUT/Contents/MacOS/rapidr"
codesign --force --sign - "$OUT/Contents/MacOS/rapidrw"
codesign --force --sign - "$OUT"
codesign --verify --strict "$OUT"
# (Finder and the Dock read the new icon and name)
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$OUT" || true
echo "RapidR Studio: $OUT (open it, or double-click it in Finder)"
