#!/bin/bash
# RapidQ's own compiler (RC.EXE) as the ground truth (docs/rapidq-ground-truth.md),
# from the Mac: the .bas files in <dir> compiled by RC.EXE in the Parallels
# Windows VM, and the console ones run — tools/windows/rc_probe.ps1 run there.
#
#   tools/rc_probe.sh <dir> [timeout seconds]
#
# <dir> must be under your home folder: the VM sees it through Parallels'
# shared folders as \\Mac\Home\…, as it sees RapidQ's folder
# (RAPIDQ_DIR, default ~/Downloads/Rapidq) and this repository.
# RAPIDR_VM names the VM (default "Windows 11 Pro"); a paused or suspended
# VM is resumed first (it pauses itself when idle).
set -u
dir=$(cd "${1:?usage: tools/rc_probe.sh <dir with .bas files> [timeout]}" && pwd -P)
timeout=${2:-10}
vm=${RAPIDR_VM:-Windows 11 Pro}
rapidq=$(cd "${RAPIDQ_DIR:-$HOME/Downloads/Rapidq}" && pwd -P)
here=$(cd "$(dirname "$0")" && pwd -P)
home=$(cd "$HOME" && pwd -P)
unc() {
  case "$1" in
    "$home"/*) printf '\\\\Mac\\Home\\%s' "$(printf '%s' "${1#"$home"/}" | tr '/' '\\')" ;;
    *) echo "not under $home: $1" >&2; exit 2 ;;
  esac
}
state=$(prlctl list -a -o status,name | awk -v vm="$vm" '{ s = $1; $1 = ""; sub(/^ /, ""); if ($0 == vm) print s }')
case "$state" in
  paused | suspended) prlctl resume "$vm" > /dev/null || exit 1 ;;
  running) ;;
  *) echo "VM \"$vm\": ${state:-not found}" >&2; exit 1 ;;
esac
prlctl exec "$vm" --current-user powershell -NoProfile -ExecutionPolicy Bypass -File "$(unc "$here/windows/rc_probe.ps1")" \
  -Programs "$(unc "$dir")" -RapidQ "$(unc "$rapidq")" -Timeout "$timeout" | tr -d '\r'
