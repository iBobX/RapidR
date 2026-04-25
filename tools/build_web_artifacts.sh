#!/usr/bin/env bash
# Build the combined RapidR web wasm package (compiler + runtime) used by
# both `rapidr bundle-bc` (per-app static bundles) and the in-browser
# IDE under web-ide/.
#
# Output: target/web/{rapidrintr.js, rapidrintr_bg.wasm, *.d.ts}
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v wasm-pack >/dev/null; then
    echo "error: wasm-pack not installed (cargo install wasm-pack)"
    exit 1
fi

echo "Building combined wasm (rapidr-vm-host-web → rapidrintr) …"
wasm-pack build interpreter/rapidr-vm-host-web \
    --target web \
    --out-dir "$ROOT/target/web" \
    --out-name rapidrintr \
    --release

echo "Done. Artifacts in target/web/"
ls -lh target/web/rapidrintr.js target/web/rapidrintr_bg.wasm
