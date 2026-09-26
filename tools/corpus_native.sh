#!/bin/bash
# Native builds of the RapidQ example corpus: generates Rust for every program
# that compiles to bytecode (per a `tools/rapidq_corpus.py --json` report) and
# `cargo check`s it. Prints each failure and a summary; the compiler errors
# of failing programs stay in $OUT/<name>.err.
#
#   tools/corpus_native.sh <corpus.json> [examples dir] [include dir]
set -u
cd "$(dirname "$0")/.."
JSON=${1:?usage: tools/corpus_native.sh <corpus.json> [examples dir] [include dir]}
CORPUS=${2:-$HOME/Downloads/Rapidq/examples}
export RAPIDR_INCLUDE_PATH=${3:-$HOME/Downloads/Rapidq/include}
export CARGO_TARGET_DIR=$PWD/tests/conformance/.work/cargo-examples
OUT=$PWD/tests/conformance/.work/corpus-native
mkdir -p "$OUT"
pass=0; fail=0; refused=0
while IFS= read -r rel; do
  n=$(echo "$rel" | tr '/ ' '__' | sed 's/\.bas$//I'); out=$OUT/$n
  rm -rf "$out"
  if ! ./rapidr codegen "$CORPUS/$rel" "$out" >/dev/null 2>"$OUT/$n.err"; then
    echo "CODEGEN-REFUSED $rel"; refused=$((refused+1)); continue
  fi
  if (cd "$out" && cargo check --quiet 2>"$OUT/$n.err"); then
    pass=$((pass+1)); rm -rf "$out" "$OUT/$n.err"
  else
    echo "CHECK-FAIL $rel"; fail=$((fail+1))
  fi
done < <(python3 -c "
import json, sys
for k, v in sorted(json.load(open(sys.argv[1]))['results'].items()):
    if not v: print(k)
" "$JSON")
echo "pass=$pass check_fail=$fail codegen_refused=$refused"
