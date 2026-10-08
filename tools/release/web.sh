#!/bin/bash
# RapidR Studio for the web as a static bundle: a folder of files any static
# host serves, zipped (dist/<version>/out/rapidr-web-<version>.zip, one top
# folder). It is tools/build_studio_web.sh's site (target/studio-web: the
# page, the shell's bytecode, the web runtime, the examples, the hosts'
# headers) and the licence files.
#
#   tools/release/web.sh [site folder]      (default: target/studio-web, built now)
#
# (RapidR Studio is the only web IDE: the old HTML / Monaco one is gone,
# docs/security-audit.md SEC-17.)
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
need zip "the bundle"
SITE="${1:-}"
if [ -z "$SITE" ]; then
    SITE=target/studio-web
    [ -f target/web/rapidrintr_bg.wasm ] && [ -f target/web/THIRD-PARTY-NOTICES.txt ] && [ -f target/web/rapidr-webview.html ] || tools/build_web_artifacts.sh
    bash tools/build_studio_web.sh --no-runtime "$SITE"
fi
for f in index.html studio.js run.html studio.rrbc runtime/rapidrintr.js runtime/rapidrintr_bg.wasm runtime/THIRD-PARTY-NOTICES.txt runtime/rapidr-webview.html; do
    [ -f "$SITE/$f" ] || die "$SITE has no $f: tools/build_studio_web.sh makes the site"
done
# (the fallback fonts beside the interpreter, as every web build has them)
[ -d "$SITE/runtime/fonts" ] || die "no $SITE/runtime/fonts: tools/build_web_artifacts.sh makes them"

NAME="rapidr-web-$VERSION"
STAGE="$WORK/web/$NAME"
rm -rf "$WORK/web" && mkdir -p "$STAGE"
trap 'rm -rf "$WORK/web"' EXIT
cp -Rp "$SITE/." "$STAGE/"
cp LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md "$STAGE/"
rm -f "$OUT/$NAME.zip"
(cd "$WORK/web" && zip -qr -X "$OUT/$NAME.zip" "$NAME")
echo "wrote $OUT/$NAME.zip ($(du -h "$OUT/$NAME.zip" | cut -f1))"
