#!/bin/bash
# Drives the Windows XP test VM from the Mac through the test helper
# (tools/winxp/helper/START-HERE.vbs, started once in the VM by hand) and
# tools/winxp/xphost.py (run by `vm.sh host`, host-only network only).
# XP has no QEMU guest agent; this is the helper's way instead.
#
#   tools/winxp/vm.sh iso                       the helper's disc: $SHARE/RAPIDR-XP-HELPER.iso
#   tools/winxp/vm.sh host                      start xphost.py in the background (its PID in $SHARE/host.pid)
#   tools/winxp/vm.sh status                    when the helper last asked for work
#   tools/winxp/vm.sh exec  'dir C:\'           a command line in the VM (cmd.exe), its output and "#exit N"
#   tools/winxp/vm.sh push  local 'C:\x\y.exe'  a file to the VM
#   tools/winxp/vm.sh pull  'C:\x\y.png' local  a file from the VM
#   tools/winxp/vm.sh ps | kill <pid>           the VM's processes; stop one by its number
#
# SHARE (default /Volumes/RapidRBuild/winxp-share); CAP=<seconds> (default 60).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SHARE="${SHARE:-/Volumes/RapidRBuild/winxp-share}"
CAP="${CAP:-60}"
mkdir -p "$SHARE/files" "$SHARE/queue" "$SHARE/results"

# a job: its number, written whole (the host hands out only *.job files)
job() {
    local id kind="$1"; shift
    id="$(date +%s)$RANDOM"
    { printf '%s\n%s' "$id" "$kind"; for a in "$@"; do printf '\n%s' "$a"; done; } > "$SHARE/queue/$id.tmp"
    mv "$SHARE/queue/$id.tmp" "$SHARE/queue/$id.job"
    echo "$id"
}

# waits for job $1's answer (.txt or .bin), CAP seconds at most
wait_for() {
    local n=0
    while [ ! -f "$SHARE/results/$1.txt" ] && [ ! -f "$SHARE/results/$1.bin" ]; do
        sleep 0.5; n=$((n + 1))
        if [ "$n" -ge $((CAP * 2)) ]; then echo "vm.sh: no answer after $CAP s (is the helper running? vm.sh status)" >&2; return 124; fi
    done
}

case "${1:-}" in
    iso)
        D="$SHARE/iso-src"; rm -rf "$D"; mkdir -p "$D"
        sed 's/$/\r/' "$ROOT/tools/winxp/helper/START-HERE.vbs" > "$D/START-HERE.vbs"
        [ -n "${ISO_EXTRA:-}" ] && cp -R $ISO_EXTRA "$D/"
        rm -f "$SHARE/RAPIDR-XP-HELPER.iso"
        hdiutil makehybrid -quiet -iso -joliet -default-volume-name RAPIDR_XP -o "$SHARE/RAPIDR-XP-HELPER.iso" "$D"
        ls -l "$SHARE/RAPIDR-XP-HELPER.iso" ;;
    host)
        if [ -f "$SHARE/host.pid" ] && kill -0 "$(cat "$SHARE/host.pid")" 2>/dev/null; then echo "running, PID $(cat "$SHARE/host.pid")"; exit 0; fi
        nohup python3 -I "$ROOT/tools/winxp/xphost.py" "$SHARE" >> "$SHARE/host.log" 2>&1 &
        echo $! > "$SHARE/host.pid"; sleep 1; tail -1 "$SHARE/host.log" ;;
    stop-host)
        [ -f "$SHARE/host.pid" ] && kill "$(cat "$SHARE/host.pid")" 2>/dev/null; rm -f "$SHARE/host.pid"; echo stopped ;;
    status) cat "$SHARE/last-seen.txt" 2>/dev/null || echo "the helper has not asked yet"; date '+now %Y-%m-%d %H:%M:%S' ;;
    exec)
        shift; id="$(job exec "$*")"; wait_for "$id"; tr -d '\r' < "$SHARE/results/$id.txt"; echo ;;
    push)
        name="push-$(date +%s)-$RANDOM-$(basename "$2")"
        cp "$2" "$SHARE/files/$name"
        id="$(job get "$name" "$3")"; wait_for "$id"; tr -d '\r' < "$SHARE/results/$id.txt"; echo
        rm -f "$SHARE/files/$name" ;;
    pull)
        id="$(job put "$2")"; wait_for "$id"
        if [ -f "$SHARE/results/$id.bin" ]; then mv "$SHARE/results/$id.bin" "$3"; ls -l "$3"; else tr -d '\r' < "$SHARE/results/$id.txt"; echo; exit 1; fi ;;
    ps) id="$(job ps)"; wait_for "$id"; tr -d '\r' < "$SHARE/results/$id.txt" ;;
    kill) id="$(job kill "$2")"; wait_for "$id"; tr -d '\r' < "$SHARE/results/$id.txt"; echo ;;
    quit) id="$(job quit)"; wait_for "$id" || true ;;
    *) sed -n '2,17p' "$0" >&2; exit 2 ;;
esac
