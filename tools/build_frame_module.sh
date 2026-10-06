#!/usr/bin/env bash
# RDATAFRAME's engine for the web (crates/rapidr-frame-web: polars), a
# wasm module of its own: the web runtime loads it only before a program
# that uses data frames (docs/ide-plan.md, D7). Built for size (the
# `wasm-frame` profile), in its own target directory: getrandom's browser
# backend is a cfg flag no other crate's build should see.
#
# Output: target/web/{rapidrframe.js, rapidrframe_bg.wasm, *.d.ts}
# (tools/build_web_artifacts.sh runs this.)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${1:-$ROOT/target/web}"
TARGET="${FRAME_TARGET_DIR:-$ROOT/target/wasm-frame}"
echo "Building the data-frame module (rapidr-frame-web → rapidrframe) …"
RUSTFLAGS='--cfg getrandom_backend="wasm_js" -C target-feature=+simd128' CARGO_TARGET_DIR="$TARGET" \
    cargo build -q -p rapidr-frame-web --target wasm32-unknown-unknown --profile wasm-frame
mkdir -p "$OUT"
wasm-bindgen --target web --out-dir "$OUT" --out-name rapidrframe "$TARGET/wasm32-unknown-unknown/wasm-frame/rapidr_frame_web.wasm"
ls -lh "$OUT/rapidrframe.js" "$OUT/rapidrframe_bg.wasm"
