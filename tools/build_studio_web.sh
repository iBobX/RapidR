#!/usr/bin/env bash
# RapidR Studio for the web (docs/ide-plan.md I1 / L-WEB): the shell
# (ide/studio.rr) compiled to bytecode, with the web runtime and the
# examples, as a static site.
#
#   tools/build_studio_web.sh [--no-runtime] [OUT]
#
# OUT (default target/studio-web) gets:
#   index.html, studio.js      the page (ide/web)
#   run.html                   the program under development's sandboxed frame
#   studio.rrbc                the shell, the same bytecode `rapidr ide` runs
#   runtime/                   the web runtime (target/web: rapidrintr.js /
#                              _bg.wasm, fonts/, THIRD-PARTY-NOTICES.txt,
#                              rapidr-webview.html)
#   _headers, .htaccess        the hosts' response headers (ide/web)
#   ide/assets/                the start page's brand (RapidR's lockups)
#   examples/                  RapidR's examples, as the Welcome page lists them
#
# The runtime is built first (tools/build_web_artifacts.sh) unless
# --no-runtime says target/web is current. Serve OUT with any static server
# (python3 -m http.server -d target/studio-web 8790) and open /index.html.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BUILD_RUNTIME=1
if [ "${1:-}" = "--no-runtime" ]; then BUILD_RUNTIME=0; shift; fi
OUT="${1:-$ROOT/target/studio-web}"

if [ "$BUILD_RUNTIME" = 1 ]; then
    bash tools/build_web_artifacts.sh
fi
[ -f target/web/rapidrintr_bg.wasm ] || { echo "error: no web runtime in target/web (run tools/build_web_artifacts.sh)"; exit 1; }

RAPIDR="${RAPIDR:-}"
if [ -z "$RAPIDR" ]; then
    cargo build --quiet --release -p rapidr-cli
    RAPIDR="$ROOT/target/release/rapidr"
fi

rm -rf "$OUT"
mkdir -p "$OUT/runtime"
"$RAPIDR" build-bc ide/studio.rr -o "$OUT/studio.rrbc"
cp ide/web/index.html ide/web/studio.js ide/web/run.html "$OUT/"
cp target/web/rapidrintr.js target/web/rapidrintr_bg.wasm target/web/THIRD-PARTY-NOTICES.txt "$OUT/runtime/"
# (the frame an RWEBVIEW's Html runs in: docs/security-audit.md SEC-15)
cp interpreter/rapidr-webbundle/web/rapidr-webview.html "$OUT/runtime/"
# (the hosts' headers: ide/web/_headers, .htaccess — no CORS, no framing by
# other sites; docs/security-audit.md SEC-17)
cp ide/web/_headers ide/web/.htaccess "$OUT/"
cp -R target/web/fonts "$OUT/runtime/fonts"
# (the start page's brand: ide/assets, read from --home as on the desktop)
mkdir -p "$OUT/ide/assets"
cp ide/assets/* "$OUT/ide/assets/"
# (the examples: sources and their data files)
mkdir -p "$OUT/examples"
(cd examples && find . -type f ! -name 'ide.rr' ! -name '*.md' -print0 | while IFS= read -r -d '' f; do
    mkdir -p "$OUT/examples/$(dirname "$f")"
    cp "$f" "$OUT/examples/$f"
done)
echo "RapidR Studio for the web: $OUT"
ls -lh "$OUT/studio.rrbc" "$OUT/runtime/rapidrintr_bg.wasm"
