#!/bin/bash
# The full local check before a commit (CI on GitHub runs only by hand):
# unit tests, the conformance suite on both backends, native examples,
# desktop GUI events (native + interpreted), and the web suites.
#
# Needs: ./rapidr built (cargo build --release -p rapidr-cli, then copy it),
# the web artifacts (tools/build_web_artifacts.sh) and the repo served on
# http://localhost:8765 for the browser tests (Playwright). Run one at a time.
cd "$(dirname "$0")/.."
curl -s -o /dev/null localhost:8765/ || { echo "serve the repo on http://localhost:8765 first (python3 -m http.server 8765 --bind 127.0.0.1)"; exit 1; }
# The suites build into tests/conformance/.work; keep it bounded (a run
# adds ~35 GB, mostly per-case debug executables and the shared cache).
W=tests/conformance/.work
if [ -d "$W/cargo-target" ] && [ "$(du -sk "$W/cargo-target" | cut -f1)" -gt 31457280 ]; then rm -rf "$W/cargo-target"; fi
# Nothing a test runs may reach a real printer (Printer.EndDoc, LPRINT):
# documents go to PDFs here instead.
mkdir -p "$W/prints"
export RAPIDR_PRINT_TO="$PWD/$W/prints"
# …nor the user's QREGISTRY store: a scratch one.
export RAPIDR_REGISTRY="$PWD/$W/registry.reg"
trap 'find "$W/cargo-target/debug" -maxdepth 1 -type f -perm +111 -delete 2>/dev/null; rm -rf "$W/codegen" "$W/native_gui_events"' EXIT
echo "== unit"; cargo test --workspace 2>&1 | grep -E "test result: FAILED|panicked|^error" | head -5; echo "(unit done)"
echo "== conformance"; node tests/conformance/run.mjs 2>&1 | tail -1
echo "== native examples"; tools/native_examples.sh 2>&1 | tail -1
echo "== gui events"; node tests/native_gui_events.mjs 2>&1 | grep -E "✗|GUI events"
echo "== web conformance"; node tests/web_conformance.mjs 2>&1 | tail -1
echo "== web gui parity"; node tests/web_gui_parity.mjs 2>&1 | tail -1
echo "== web gui parity at 2x (high-DPI: what programs read is unchanged)"; RAPIDR_DPR=2 node tests/web_gui_parity.mjs 2>&1 | tail -1
echo "== web"; for t in tests/web_ide_*.mjs tests/web_bundle_*.mjs tests/web_end_timer.mjs; do
  out=$(node "$t" 2>&1) || { echo "$t: FAILED"; echo "$out" | grep -m3 -E "ASSERT|Error|✗"; }
done
echo ALLDONE
