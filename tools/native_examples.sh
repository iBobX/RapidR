#!/bin/bash
# Native builds of the repo's examples: generates each examples/*.rr's Rust
# and `cargo check`s it (shared target dir). Prints failures and a summary.
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR=$PWD/tests/conformance/.work/cargo-examples
WORK=$PWD/tests/conformance/.work/examples-native
mkdir -p "$WORK"
pass=0; fail=0
for f in examples/*.rr; do
  n=$(basename "$f" .rr); out=$WORK/$n
  rm -rf "$out"
  if ! ./rapidr codegen "$f" "$out" >/dev/null 2>"$WORK/$n.err"; then echo "CODEGEN-FAIL $n"; fail=$((fail+1)); continue; fi
  if (cd "$out" && cargo check --quiet 2>"$WORK/$n.err"); then pass=$((pass+1)); rm -f "$WORK/$n.err"; else echo "CHECK-FAIL $n"; fail=$((fail+1)); fi
done
echo "pass=$pass fail=$fail"
