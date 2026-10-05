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

# SQLite's C sources go into the wasm (RSQLITE): archived by llvm-ar, or
# without one by the system's ar (tools/wasm-ar.sh).
export AR_wasm32_unknown_unknown="${AR_wasm32_unknown_unknown:-$ROOT/tools/wasm-ar.sh}"

echo "Building combined wasm (rapidr-vm-host-web → rapidrintr) …"
# wasm SIMD (docs/web-host-plan.md §3.4, Stage W3): the UI kernel's CPU
# renderer (vello_cpu's fearless_simd) runs on simd128 — 2.4–2.8× faster
# for the same pixels — and every 2026 browser has it, so the build is
# SIMD-only. The flags rebuild every crate: its own target directory
# (target/wasm-simd, as tools/build_web_host_spike.sh's), not target/'s.
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+simd128" CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/wasm-simd}" \
wasm-pack build interpreter/rapidr-vm-host-web \
    --target web \
    --out-dir "$ROOT/target/web" \
    --out-name rapidrintr \
    --release

# The fallback fonts (fonts/fallback, docs/web-host-plan.md §3.7): Noto's
# chunks beside the runtime, loaded by a page as its text needs them. The
# CJK ones are fetched once into target/fonts-src (offline: left out, and
# the script says how to get them).
python3 tools/fonts.py build target/web/fonts

echo "Done. Artifacts in target/web/"
ls -lh target/web/rapidrintr.js target/web/rapidrintr_bg.wasm
