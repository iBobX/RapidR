#!/bin/bash
# RapidQ's look, for the visual gallery (tests/visual/README.md): the
# programs in tests/visual/cases built by RapidQ's RC.EXE in the Parallels
# Windows VM and run there, each window captured at 1× and 2×
# (tools/windows/rc_shots.ps1), the PNGs saved as
# tests/visual/rapidq/<name>@<scale>x-<n>.png.
#
#   tools/visual/rq_shots.sh [name,…]
#
# Needs the VM (RAPIDR_VM, default "Windows 11 Pro"; resumed when paused) and
# RapidQ's folder (RAPIDQ_DIR, default ~/Downloads/Rapidq) — see
# docs/rapidq-ground-truth.md. The repository must be under your home
# folder (the VM reads it as \\Mac\Home\…). Nothing is written to the share.
set -u
root=$(cd "$(dirname "$0")/../.." && pwd -P)
only=${1:-}
vm=${RAPIDR_VM:-Windows 11 Pro}
rapidq=$(cd "${RAPIDQ_DIR:-$HOME/Downloads/Rapidq}" && pwd -P)
home=$(cd "$HOME" && pwd -P)
out="$root/tests/visual/rapidq"
mkdir -p "$out"
unc() { printf '\\\\Mac\\Home\\%s' "$(printf '%s' "${1#"$home"/}" | tr '/' '\\')"; }
state=$(prlctl list -a -o status,name | awk -v vm="$vm" '{ s = $1; $1 = ""; sub(/^ /, ""); if ($0 == vm) print s }')
case "$state" in
  paused | suspended) prlctl resume "$vm" > /dev/null || exit 1 ;;
  running) ;;
  *) echo "VM \"$vm\": ${state:-not found}" >&2; exit 1 ;;
esac
log=$(mktemp)
args=(-Programs "$(unc "$root/tests/visual/cases")" -RapidQ "$(unc "$rapidq")")
[ -n "$only" ] && args+=(-Only "$only")
prlctl exec "$vm" --current-user powershell -NoProfile -ExecutionPolicy Bypass -File "$(unc "$root/tools/windows/rc_shots.ps1")" \
  "${args[@]}" < /dev/null | tr -d '\r' > "$log"
python3 - "$log" "$out" <<'PY'
import base64, sys
log, out = sys.argv[1], sys.argv[2]
name = None
for line in open(log, encoding="utf-8", errors="replace"):
    line = line.rstrip("\n")
    if line.startswith("-- shot "):
        f = line.split()
        name = f"{f[2]}@{f[3]}x-{f[4]}.png"
    elif line.startswith("-- png64 ") and name:
        open(f"{out}/{name}", "wb").write(base64.b64decode(line[9:]))
        print("saved", name)
        name = None
    elif line.startswith("-- skip") or "rror" in line:
        print(line)
PY
rm -f "$log"
