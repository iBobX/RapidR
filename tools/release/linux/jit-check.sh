#!/bin/bash
# The JIT on Linux (docs/jit-design.md), in the Ubuntu VM: a branch's committed sources (a
# `git archive` tar, sent first) built in ~/rapidr-jit/<name>, then the JIT's own tests (every
# edge value through compiled code), the conformance suite with the interpreter alone and with
# the JIT compiling all it can (RAPIDR_JIT=always), and three programs' speed both ways.
#
#   tools/release/ubuntu-vm.sh send <src.tar> /tmp/rapidr-jit-src.tar
#   tools/release/ubuntu-vm.sh run tools/release/linux/jit-check.sh /tmp/rapidr-jit-src.tar [name]
#
# Its own folder, kept for the next run's incremental build; nothing of the release's folders
# is touched. Nothing here prints, touches the user's registry store or settings.
set -uo pipefail
SRC_TAR="${1:?the sources' tar}"
NAME="${2:-host}"
W="$HOME/rapidr-jit/$NAME"
rm -rf "$W/src" && mkdir -p "$W/src" "$W/prints"
tar -m -xf "$SRC_TAR" -C "$W/src" || { echo "extracting the sources failed"; exit 1; }
export CARGO_TARGET_DIR="$W/target"
export RAPIDR_PRINT_TO="$W/prints" RAPIDR_REGISTRY="$W/registry.reg" RAPIDR_AI_KEYSTORE=memory
export RAPIDR_BUILD_CACHE="$W/build-cache" LANGUAGE=en_US.UTF-8 RAPIDR_LANG=en
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cd "$W/src" || exit 1
fail=0
echo "== $(rustc -vV | grep host), building rapidr"
cargo build --release -q -p rapidr-cli 2>&1 | grep -E '^error' -A6 | head -20
[ -x "$W/target/release/rapidr" ] || { echo "the build failed"; exit 1; }
cp "$W/target/release/rapidr" ./rapidr
echo "== the JIT's tests (every edge value)"
out="$(cargo test --release -q -p rapidr-jit 2>&1)" || fail=1
echo "$out" | grep -E 'test result|panicked|^error' | head -20
echo "== rapidr run --verbose"
for p in loop10m fcalls dsp_scalars calls; do
  s=$(date +%s%N); ./rapidr run --verbose "tools/bench/vm/$p.rr" 2>&1; m=$(date +%s%N)
  RAPIDR_JIT=off ./rapidr run "tools/bench/vm/$p.rr" > /dev/null 2>&1; e=$(date +%s%N)
  echo "   $p: $(( (m - s) / 1000000 )) ms with the JIT, $(( (e - m) / 1000000 )) ms without (wall)"
done
if command -v node > /dev/null; then
  echo "== conformance (the interpreter, and RAPIDR_JIT=always)"
  node tests/conformance/run.mjs --backend vm,vm-jit > "$W/conformance.txt" 2>&1
  grep -E '^FAIL|^XPASS|passed,|^\s+\|' "$W/conformance.txt" | head -60
  grep -qE '[0-9]+ passed, [0-9]+ known failures \(xfail\), 0 failed' "$W/conformance.txt" || fail=1
else
  echo "no node: conformance skipped"
fi
[ $fail = 0 ] && echo "jit-check: all passed" || echo "jit-check: FAILED"
exit $fail
