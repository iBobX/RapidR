#!/usr/bin/env bash
# tests/bc_smoke.sh — smoke-test the bytecode pipeline against a list of
# console-only example programs. Builds each to .rrbc, runs it through
# the NativeHost, and prints a compact PASS/FAIL line per file.
#
# Usage:  ./tests/bc_smoke.sh
#
# Exits non-zero if any example fails to build or its run exits with a
# non-zero status. Output lines are not diffed against rust-codegen
# output; that's a future enhancement.

set -u
cd "$(dirname "$0")/.."

CLI="./target/debug/rapidr"
if [[ ! -x "$CLI" ]]; then
    echo "Building rapidr CLI…"
    cargo build -p rapidr-cli >/dev/null 2>&1 || { echo "FAIL: cargo build"; exit 1; }
fi

# Console-only examples (no GUI / no interactive input).
EXAMPLES=(
    examples/hello_world.rr
    examples/test_phase3_builtins.rr
    examples/test_builtins.rr
    examples/test_with.rr
)

fails=0
total=0
for f in "${EXAMPLES[@]}"; do
    total=$((total + 1))
    bc="/tmp/$(basename "${f%.rr}").rrbc"
    if ! "$CLI" build-bc "$f" -o "$bc" >/dev/null 2>&1; then
        echo "FAIL build: $f"
        fails=$((fails + 1))
        continue
    fi
    if ! out=$("$CLI" run-bc "$bc" 2>&1); then
        echo "FAIL run:   $f"
        fails=$((fails + 1))
        continue
    fi
    lines=$(printf '%s\n' "$out" | wc -l | tr -d ' ')
    echo "PASS:       $f  (${lines} lines of output)"
done

echo
echo "Summary: $((total - fails))/$total passed"

# ---- Bundle smoke (Phase 6) ----
echo
echo "--- bundle-bc smoke ---"
mkdir -p target/web
[[ -f target/web/rapidrintr.js ]] || printf 'export default async function init(){}\nexport function rapidr_run_bc(){}\n' > target/web/rapidrintr.js
[[ -f target/web/rapidrintr_bg.wasm ]] || printf '\x00asm\x01\x00\x00\x00' > target/web/rapidrintr_bg.wasm
zip_path="/tmp/hello_web-bundle-smoke.zip"
if "$CLI" bundle-bc examples/hello_web.rr -o "$zip_path" >/dev/null 2>&1; then
    if unzip -l "$zip_path" 2>/dev/null | grep -q "hello_web.rrbc"; then
        size=$(stat -f%z "$zip_path" 2>/dev/null || stat -c%s "$zip_path")
        echo "PASS bundle: $zip_path (${size} bytes)"
    else
        echo "FAIL bundle: hello_web.rrbc missing from $zip_path"
        fails=$((fails + 1))
    fi
else
    echo "FAIL bundle: $CLI bundle-bc failed"
    fails=$((fails + 1))
fi

exit $fails
