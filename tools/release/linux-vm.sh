#!/bin/bash
# The Linux packages made in the Ubuntu VM (Parallels, aarch64), driven from
# the Mac — the other way to tools/release/linux-docker.sh, using no Mac disk:
#
#   tools/release/linux-vm.sh            build and package (linux.sh) in the
#                                        VM's ~/rapidr-release, then fetch the
#                                        packages into dist/<ver>/out/
#   tools/release/linux-vm.sh --clean    remove ~/rapidr-release from the VM
#
# After tools/release/prepare.sh. The VM needs Rust, python3, dpkg-dev and the
# desktop host's -dev packages (it has them); the build takes ~5 GB there.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
VM=tools/release/ubuntu-vm.sh
mkdir -p "$WORK"
SCRIPT="$WORK/linux-vm-job.sh"
trap 'rm -f "$SCRIPT"' EXIT

if [ "${1:-}" = --clean ]; then
    printf 'rm -rf "$HOME/rapidr-release"\necho removed ~/rapidr-release\n' > "$SCRIPT"
    bash "$VM" run "$SCRIPT"
    exit 0
fi
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"

step "send the release's source and prepared parts"
for f in src.tar commit rapidr-ide.rrbc web-runtime/rapidrintr.js web-runtime/rapidrintr_bg.wasm; do
    bash "$VM" send "$PREP/$f" "/tmp/rapidr-release-$(basename "$f")"
done

cat > "$SCRIPT" <<'EOF'
set -e
export PATH="$HOME/.cargo/bin:$PATH"
B="$HOME/rapidr-release"
rm -rf "$B/src" "$B/dist" && mkdir -p "$B/src" "$B/dist/prep/web-runtime"
for f in src.tar commit rapidr-ide.rrbc; do mv "/tmp/rapidr-release-$f" "$B/dist/prep/$f"; done
for f in rapidrintr.js rapidrintr_bg.wasm; do mv "/tmp/rapidr-release-$f" "$B/dist/prep/web-runtime/$f"; done
# (-m: the files' times are now, so cargo rebuilds what changed)
tar -m -x -C "$B/src" -f "$B/dist/prep/src.tar"
cd "$B/src"
RAPIDR_DIST="$B/dist" CARGO_TARGET_DIR="$B/target" bash tools/release/linux.sh > "$B/build.log" 2>&1 || { tail -30 "$B/build.log"; exit 1; }
grep -E "^==|wrote|links|  lib" "$B/build.log"
ls "$B/dist/out"
EOF
step "build and package in the VM (log: ~/rapidr-release/build.log there)"
bash "$VM" run "$SCRIPT"

step "fetch the packages"
printf 'ls "$HOME/rapidr-release/dist/out"\n' > "$SCRIPT"
for f in $(bash "$VM" run "$SCRIPT"); do
    bash "$VM" fetch "~/rapidr-release/dist/out/$f" "$OUT/$f"
    echo "  $OUT/$f ($(du -h "$OUT/$f" | cut -f1))"
done
