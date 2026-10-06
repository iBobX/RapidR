#!/bin/bash
# The full local check before a commit (CI on GitHub runs only by hand):
# unit tests, the conformance suite on both backends, the examples (each
# run on every runtime it claims), desktop GUI events (native +
# interpreted), the web suites, and the legal
# checks (licences, and the notices every build carries).
#
# Needs: ./rapidr built (cargo build --release -p rapidr-cli, then copy it),
# the web artifacts (tools/build_web_artifacts.sh) and the repo served on
# http://localhost:8765 (or RAPIDR_URL) for the browser tests (Playwright).
# Run one at a time.
#
#   tools/regress.sh                  every stage, then the build caches go
#   tools/regress.sh gui web          only these stages (unit, conformance,
#                                     examples, gui, web, legal), caches kept
#   tools/regress.sh --clean          (with stages) remove the caches after
cd "$(dirname "$0")/.."
# The suites build into tests/conformance/.work and the unit tests into
# target/debug (together ~90 GB a run): both go when the run ends, however
# it ends, so the disk never fills (the next run builds them again — a few
# minutes more).
W=tests/conformance/.work
# Nothing a test runs may reach a real printer (Printer.EndDoc, LPRINT):
# documents go to PDFs here instead.
mkdir -p "$W/prints"
export RAPIDR_PRINT_TO="$PWD/$W/prints"
# …nor the user's QREGISTRY store: a scratch one.
export RAPIDR_REGISTRY="$PWD/$W/registry.reg"
STAGES=(); CLEAN=0
for a in "$@"; do if [ "$a" = --clean ]; then CLEAN=1; else STAGES+=("$a"); fi; done
[ ${#STAGES[@]} -eq 0 ] && { STAGES=(unit conformance examples gui web legal); CLEAN=1; }
# (what's inside $W: it may be a link to a build volume)
[ $CLEAN = 1 ] && trap 'rm -rf "$W"/* target/debug target/wasm32-unknown-unknown/debug' EXIT
want() { [[ " ${STAGES[*]} " == *" $1 "* ]]; }
# (the browser tests need the repo served: RAPIDR_URL, else port 8765)
if want web || want examples; then curl -s -o /dev/null "${RAPIDR_URL:-http://localhost:8765}/" || { echo "serve the repo on ${RAPIDR_URL:-http://localhost:8765} first (python3 -m http.server 8765 --bind 127.0.0.1)"; exit 1; }; fi
if want unit; then echo "== unit"; cargo test --workspace 2>&1 | grep -E "test result: FAILED|panicked|^error" | head -5
  # (the UI kernel and the program glue stay GUI-free: they must build for
  # the browser too)
  cargo check -q -p rapidr-ui-kernel -p rapidr-ui-app -p rapidr-session -p rapidr-project --target wasm32-unknown-unknown 2>&1 | grep -E "^error" -A5 | head -10
  # (RapidR's icons: the sources, the crate's tables and the manual's copies
  # match the geometry in design/icons/tools)
  python3 design/icons/tools/build.py --check; python3 design/icons/tools/export.py --check
  # (the user manual's generated reference pages match the source)
  python3 tools/manual_reference.py --check; echo "(unit done)"; fi
if want conformance; then echo "== conformance"; node tests/conformance/run.mjs 2>&1 | tail -1; fi
# (every example in examples/ on every runtime it claims — rapidr run,
# interpreted and native executables, the web — GUI ones by the test hooks)
if want examples; then echo "== examples"; node tests/examples_run.mjs 2>&1 | grep -E "✗|^    |Examples:"; fi
if want gui; then echo "== gui events (the UI kernel's headless host, native + interpreted)"; node tests/native_gui_events.mjs 2>&1 | grep -E "✗|GUI events"
  echo "== gui events at 2x (high-DPI: what programs read is unchanged)"; RAPIDR_SCALE=2 node tests/native_gui_events.mjs 2>&1 | grep -E "✗|GUI events"; fi
if want web; then
  echo "== web conformance"; node tests/web_conformance.mjs 2>&1 | tail -1
  # (the UI kernel hosts the web — docs/web-host-plan.md: the cases' dumps
  # by the desktop's own test hooks, and every window and accessibility tree
  # byte for byte against the desktop's own captures, made here by
  # tests/gui_captures.mjs at 1× and 2×; what the page shows outside the
  # windows by the cases' webCheck. Known ≠: menus / themes (the desktop's
  # macOS menu bar), message_icons / message_dialogs, design_surface 1×,
  # modal_result 2× (wasm SIMD's rounding, one or two pixels by one level))
  echo "== desktop captures for the web"; node tests/gui_captures.mjs "$PWD/$W/gui_captures" 2>&1 | tail -1
  echo "== web gui parity"; RAPIDR_DESKTOP_CAPTURES="$PWD/$W/gui_captures" node tests/web_gui_parity.mjs 2>&1 | grep -E "✗|≠|Kernel host|parity"
  echo "== web gui parity at 2x (high-DPI: what programs read is unchanged)"; RAPIDR_DPR=2 RAPIDR_DESKTOP_CAPTURES="$PWD/$W/gui_captures" node tests/web_gui_parity.mjs 2>&1 | grep -E "✗|≠|Kernel host|parity"
  # (Chrome's accessibility tree over the mirror = the kernel's)
  echo "== web accessibility"; node tests/web_a11y.mjs 2>&1 | grep -E "✗|^    |Kernel host|Web accessibility"
  echo "== web"; for t in tests/web_ide_*.mjs tests/debug_e2e_*.mjs tests/web_session.mjs tests/web_bundle_*.mjs tests/web_end_timer.mjs tests/web_vm_yield.mjs tests/web_overlays.mjs tests/web_fonts.mjs tests/web_webapi.mjs tests/web_sqlite.mjs; do
    out=$(node "$t" 2>&1) || { echo "$t: FAILED"; echo "$out" | grep -m3 -E "ASSERT|Error|✗"; }
  done
fi
# Licences and notices (LEGAL.md, docs/licensing.md): a licence outside
# deny.toml's allowlist (permissive only) or a replaced crate (its [bans])
# anywhere in the workspace fails; THIRD_PARTY_NOTICES.md is current; every
# kind of output keeps to the permissive list and carries a
# THIRD-PARTY-NOTICES.txt listing every crate in it. Any of these failing
# fails the run (exit 1).
LEGAL_FAILED=0
if want legal; then echo "== legal"
  out=$(cargo deny check licenses bans 2>&1) || { LEGAL_FAILED=1; echo "$out" | grep -E "^error" -A8 | head -40; }
  echo "$out" | tail -1
  python3 tools/third_party_notices.py --check || LEGAL_FAILED=1
  out=$(python3 tools/check_notices.py --rapidr ./rapidr 2>&1) || LEGAL_FAILED=1
  echo "$out" | grep -E "FAIL|notices: all ok"; fi
[ $LEGAL_FAILED = 1 ] && { echo "LEGAL CHECKS FAILED"; exit 1; }
echo ALLDONE
