#!/bin/bash
# Drives the Windows 7 test VM in UTM from the Mac, with no clicks, once its
# QEMU guest agent is installed (README.md, "Automating the Windows 7 VM").
# Every call is capped (UTM's own tool waits forever when the guest is busy).
#
#   tools/win7/vm.sh status
#   tools/win7/vm.sh stage                    put install-agent.ps1 (+ the driver and agent files) on the share
#   tools/win7/vm.sh exec  'dir C:\'          run a command line in the guest (cmd.exe /c), print its output
#   tools/win7/vm.sh push  local.zip 'C:\x.zip'
#   tools/win7/vm.sh pull  'C:\rapidr\tests\results.txt' ./results.txt
#
# VM=<name or UUID> (default "Windows 7 x86"); CAP=<seconds> (default 60; push/pull of big files: 300).
set -euo pipefail

UTMCTL=/Applications/UTM.app/Contents/MacOS/utmctl
VM="${VM:-Windows 7 x86}"
CAP="${CAP:-60}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SHARE="${WIN7_SHARE:-/Volumes/RapidRBuild/win7-share}"

# run "$@" with a time limit; its exit status is the command's (124 when cut off)
capped() {
    "$@" <&0 &   # (a background command would otherwise lose stdin: push needs it)
    local p=$! n=0
    while kill -0 "$p" 2>/dev/null && [ "$n" -lt "$CAP" ]; do sleep 1; n=$((n + 1)); done
    if kill -0 "$p" 2>/dev/null; then kill "$p" 2>/dev/null || true; echo "vm.sh: cut off after $CAP s" >&2; return 124; fi
    wait "$p"
}

case "${1:-}" in
    status) "$UTMCTL" status "$VM" ;;
    stage)
        [ -f "$SHARE/.win7-share" ] || { echo "$SHARE is not the share (tools/win7/stage.sh makes it)" >&2; exit 1; }
        ADDR="${ADDR:-192.168.128.1}"
        sed "s/@ADDR@/$ADDR/g; s/\$/\r/" "$ROOT/tools/win7/vm/install-agent.ps1" > "$SHARE/install-agent.ps1"
        echo "staged $SHARE/install-agent.ps1 (address $ADDR). Serve it: tools/win7/serve.sh"
        ;;
    exec) shift; capped "$UTMCTL" exec "$VM" --cmd "cmd.exe /c $*" ;;
    push) capped "$UTMCTL" file push "$VM" "$3" < "$2" ;;
    pull) capped "$UTMCTL" file pull "$VM" "$2" > "$3" ;;
    *) sed -n '2,13p' "$0" >&2; exit 2 ;;
esac
