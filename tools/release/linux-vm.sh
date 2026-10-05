#!/bin/bash
# The Linux packages, both architectures, made in an Ubuntu VM (Parallels:
# RAPIDR_LINUX_VM, default the aarch64 one; an x86_64 one works the same),
# driven from the Mac — the heavy work stays on the VM's disk:
#
#   tools/release/linux-vm.sh            build and package (linux.sh) in the
#                                        VM's ~/rapidr-release, then fetch the
#                                        packages into dist/<ver>/out/
#   tools/release/linux-vm.sh wait       wait for a build already started, then fetch
#   tools/release/linux-vm.sh smoke      tools/release/smoke.sh on every package
#                                        there (the other architecture's under
#                                        qemu-user: no native build)
#   tools/release/linux-vm.sh --clean    remove ~/rapidr-release from the VM
#
# After tools/release/prepare.sh. The VM needs tools/release/linux/setup-tools.sh
# (system, as root: tools/release/ubuntu-vm.sh root …; then user); ~8 GB there.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
VM=tools/release/ubuntu-vm.sh
mkdir -p "$WORK"
SCRIPT="$WORK/linux-vm-job.sh"
trap 'rm -f "$SCRIPT"' EXIT

if [ "${1:-}" = --clean ]; then
    printf 'rm -rf "$HOME/rapidr-release" /tmp/rapidr-release-* /tmp/rapidr-vm.sh /tmp/rapidr-fetch.* /tmp/rapidr-smoke.*\necho removed ~/rapidr-release\n' > "$SCRIPT"
    bash "$VM" run "$SCRIPT"
    exit 0
fi
if [ "${1:-}" = smoke ]; then
    bash "$VM" send tools/release/smoke.sh /tmp/rapidr-release-smoke.sh
    cat > "$SCRIPT" <<'EOF'
B="$HOME/rapidr-release"
mv /tmp/rapidr-release-smoke.sh "$B/smoke.sh"
rm -f "$B/smoke.done"
setsid nohup bash -c '
  export PATH="$HOME/.cargo/bin:$PATH"; B="$1"; host="$(uname -m)"; fail=0
  for a in "$B"/dist/out/*.deb "$B"/dist/out/*.tar.gz; do
    case "$a" in *"$host"*|*_$(dpkg --print-architecture).deb) native=1 ;; *) native=0 ;; esac
    SMOKE_NATIVE=$native bash "$B/smoke.sh" "$a" || fail=1
  done > "$B/smoke.log" 2>&1
  echo $fail > "$B/smoke.done"' _ "$B" > /dev/null 2>&1 &
echo started
EOF
    step "smoke tests in the VM (log: ~/rapidr-release/smoke.log there)"
    bash "$VM" run "$SCRIPT"
    printf 'cat "$HOME/rapidr-release/smoke.done" 2>/dev/null || true\n' > "$SCRIPT"
    while :; do
        sleep 30
        code="$(bash "$VM" run "$SCRIPT" 2>/dev/null || true)"
        [ -n "$code" ] && break
    done
    printf 'grep -E "^==|FAIL" "$HOME/rapidr-release/smoke.log"\n' > "$SCRIPT"
    bash "$VM" run "$SCRIPT"
    [ "$code" = 0 ] || die "a smoke test failed (the VM's ~/rapidr-release/smoke.log)"
    exit 0
fi
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"

# (`wait`: a build already started — this script stopped while it ran — waited for and fetched)
if [ "${1:-}" != wait ]; then
step "send the release's source and prepared parts"
for f in src.tar commit rapidr-ide.rrbc web-runtime/rapidrintr.js web-runtime/rapidrintr_bg.wasm; do
    bash "$VM" send "$PREP/$f" "/tmp/rapidr-release-$(basename "$f")"
done

cat > "$SCRIPT" <<'EOF'
set -e
B="$HOME/rapidr-release"
pgrep -f "tools/release/linux.sh" > /dev/null && { echo running; exit 0; }
# (run again when Parallels reported a failure that wasn't one: no harm)
if [ -f /tmp/rapidr-release-src.tar ]; then
    rm -rf "$B/src" "$B/dist" "$B/done" && mkdir -p "$B/src" "$B/dist/prep/web-runtime"
    for f in src.tar commit rapidr-ide.rrbc; do mv "/tmp/rapidr-release-$f" "$B/dist/prep/$f"; done
    for f in rapidrintr.js rapidrintr_bg.wasm; do mv "/tmp/rapidr-release-$f" "$B/dist/prep/web-runtime/$f"; done
    # (-m: the files' times are now, so cargo rebuilds what changed)
    tar -m -x -C "$B/src" -f "$B/dist/prep/src.tar"
fi
pgrep -f "tools/release/linux.sh" > /dev/null && { echo running; exit 0; }
[ -f "$B/done" ] && { echo done; exit 0; }
# (detached: a long prlctl session can drop while the VM goes on)
cd "$B/src"
setsid nohup bash -c 'export PATH="$HOME/.cargo/bin:$PATH"; RAPIDR_DIST="$1/dist" CARGO_TARGET_DIR="$1/target" bash tools/release/linux.sh > "$1/build.log" 2>&1; echo $? > "$1/done"' _ "$B" > /dev/null 2>&1 &
echo started
EOF
step "build and package in the VM (log: ~/rapidr-release/build.log there)"
bash "$VM" run "$SCRIPT"
fi
printf 'cat "$HOME/rapidr-release/done" 2>/dev/null || true\n' > "$SCRIPT"
while :; do
    sleep 30
    code="$(bash "$VM" run "$SCRIPT" 2>/dev/null || true)"
    [ -n "$code" ] && break
done
printf 'grep -E "^==|wrote|links|  lib|error" "$HOME/rapidr-release/build.log" | tail -40\n' > "$SCRIPT"
bash "$VM" run "$SCRIPT"
[ "$code" = 0 ] || die "the VM's build failed (exit $code)"

step "fetch the packages"
printf 'ls "$HOME/rapidr-release/dist/out"\n' > "$SCRIPT"
for f in $(bash "$VM" run "$SCRIPT"); do
    bash "$VM" fetch "~/rapidr-release/dist/out/$f" "$OUT/$f"
    echo "  $OUT/$f ($(du -h "$OUT/$f" | cut -f1), sha256 $(shasum -a 256 "$OUT/$f" | cut -d' ' -f1))"
done
