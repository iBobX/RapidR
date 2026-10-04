#!/usr/bin/env bash
# Build the web host spike (docs/web-host-plan.md, "Spike results"):
# crates/rapidr-ui-host-web as the wasm package tests/web_host_spike.html
# loads, and the desktop's captures of the same forms that
# tests/web_host_spike.mjs compares the browser's pixels with.
#
#   target/web-host-spike/           with wasm SIMD: the page's default
#   target/web-host-spike/desktop*/  the desktop's captures (the CPU path,
#                                    as the headless host draws)
#   target/web-host-spike-scalar/    without SIMD (?pkg=web-host-spike-scalar)
#   target/web-host-spike-gpu/       with vello on WebGPU too (?gpu=1);
#                                    only with --gpu
#
# wasm SIMD (`-C target-feature=+simd128`): vello_cpu's fearless_simd runs
# on simd128 instead of its scalar fallback — 2.4–2.8× faster for the same
# pixels, and every 2026 browser has it. The flags rebuild every crate, so
# the SIMD builds get their own target directory (target/wasm-simd) rather
# than overwriting the scalar builds' in target/.
#
# Usage (repo root): tools/build_web_host_spike.sh [--gpu] [--no-scalar]
# (wasm-pack, and wasm-bindgen-cli 0.2.129: the README's prerequisites)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

GPU=0
SCALAR=1
for arg in "$@"; do
    case "$arg" in
        --gpu) GPU=1 ;;
        --no-scalar) SCALAR=0 ;;
        *) echo "usage: $0 [--gpu] [--no-scalar]"; exit 2 ;;
    esac
done

if ! command -v wasm-pack >/dev/null; then
    echo "error: wasm-pack not installed (cargo install wasm-pack)"
    exit 1
fi

simd() {
    RUSTFLAGS="-C target-feature=+simd128" CARGO_TARGET_DIR="$ROOT/target/wasm-simd" "$@"
}

echo "Building the spike with wasm SIMD → target/web-host-spike …"
simd wasm-pack build crates/rapidr-ui-host-web --target web --release --out-dir "$ROOT/target/web-host-spike"

if [ "$SCALAR" = 1 ]; then
    echo "Building the spike without SIMD → target/web-host-spike-scalar …"
    wasm-pack build crates/rapidr-ui-host-web --target web --release --out-dir "$ROOT/target/web-host-spike-scalar"
fi

if [ "$GPU" = 1 ]; then
    echo "Building the spike with vello on WebGPU (wasm SIMD) → target/web-host-spike-gpu …"
    simd wasm-pack build crates/rapidr-ui-host-web --target web --release --out-dir "$ROOT/target/web-host-spike-gpu" -- --features gpu
fi

echo "Capturing the desktop's frames of the same forms …"
cargo run -p rapidr-ui-host-web --example desktop_capture --release -- "$ROOT/target/web-host-spike"

echo "Done. Serve the repository (e.g. python3 -m http.server 8782 --bind 127.0.0.1)"
echo "and open /tests/web_host_spike.html, or run: node tests/web_host_spike.mjs"
ls -lh target/web-host-spike*/rapidr_ui_host_web_bg.wasm
