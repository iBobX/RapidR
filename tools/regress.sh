#!/bin/bash
# The full local check before a commit (CI on GitHub runs only by hand):
# unit tests, the conformance suite on both backends, native examples,
# desktop GUI events (native + interpreted), and the web suites.
#
# Needs: ./rapidr built (cargo build --release -p rapidr-cli, then copy it),
# the web artifacts (tools/build_web_artifacts.sh) and the repo served on
# http://localhost:8765 for the browser tests (Playwright). Run one at a time.
cd "$(dirname "$0")/.."
echo "== unit"; cargo test --workspace 2>&1 | grep -E "test result: FAILED|panicked|^error" | head -5; echo "(unit done)"
echo "== conformance"; node tests/conformance/run.mjs 2>&1 | tail -1
echo "== native examples"; tools/native_examples.sh 2>&1 | tail -1
echo "== gui events"; node tests/native_gui_events.mjs 2>&1 | grep -E "✗|GUI events"
echo "== web conformance"; node tests/web_conformance.mjs 2>&1 | tail -1
echo "== web gui parity"; node tests/web_gui_parity.mjs 2>&1 | tail -1
echo "== web gui parity at 2x (high-DPI: what programs read is unchanged)"; RAPIDR_DPR=2 node tests/web_gui_parity.mjs 2>&1 | tail -1
echo "== web"; for t in tests/web_ide_*.mjs tests/web_bundle_*.mjs tests/web_end_timer.mjs; do r=$(node "$t" 2>&1 | tail -1); echo "$t: $r"; done 2>&1 | grep -v -i "PASSED\|PASS$\|passed"
echo ALLDONE
