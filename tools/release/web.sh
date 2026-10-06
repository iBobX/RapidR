#!/bin/bash
# The web IDE as a static bundle: a folder of files any static host serves,
# zipped (dist/<version>/out/rapidr-web-<version>.zip, one top folder).
#
#   tools/release/web.sh [site folder]      (default: web-ide)
#
# The bundle is the site's git-tracked files as they are, its `runtime/`
# (the web interpreter, which git doesn't keep) filled from target/web, and
# the licence files. Nothing here knows what the site is: when the IDE drawn
# by the UI kernel replaces web-ide/, it is packaged the same way.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
need zip "the bundle"
SITE="${1:-web-ide}"
[ -f target/web/rapidrintr_bg.wasm ] && [ -f target/web/THIRD-PARTY-NOTICES.txt ] || tools/build_web_artifacts.sh

NAME="rapidr-web-$VERSION"
STAGE="$WORK/web/$NAME"
rm -rf "$WORK/web" && mkdir -p "$STAGE"
trap 'rm -rf "$WORK/web"' EXIT
git ls-files -z "$SITE" | while IFS= read -r -d '' f; do
    rel="${f#"$SITE"/}"
    [ -L "$f" ] && continue                      # (runtime/ → target/web: below)
    mkdir -p "$STAGE/$(dirname "$rel")"
    cp -p "$f" "$STAGE/$rel"
done
mkdir -p "$STAGE/runtime"
cp target/web/rapidrintr.js target/web/rapidrintr_bg.wasm target/web/THIRD-PARTY-NOTICES.txt "$STAGE/runtime/"
# (the fallback fonts beside the interpreter, as every web build has them)
if [ -d target/web/fonts ]; then
    cp -R target/web/fonts "$STAGE/runtime/fonts"
elif [ -f tools/fonts.py ]; then
    die "no target/web/fonts: tools/build_web_artifacts.sh makes them"
fi
cp LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md "$STAGE/"
rm -f "$OUT/$NAME.zip"
(cd "$WORK/web" && zip -qr -X "$OUT/$NAME.zip" "$NAME")
echo "wrote $OUT/$NAME.zip ($(du -h "$OUT/$NAME.zip" | cut -f1))"
