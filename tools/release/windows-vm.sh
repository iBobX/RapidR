#!/bin/bash
# Runs the Windows release scripts in the Windows VM (Parallels; RAPIDR_WIN_VM
# names it). The VM reads this repository and prepare.sh's output through the
# Mac's home share (\\Mac\Home) — only reads; nothing is written to the share.
#
#   tools/release/windows-vm.sh tools        the build tools (setup-tools.ps1)
#   tools/release/windows-vm.sh build [windows.ps1 options…]
#                                            the installers, in the VM's
#                                            %USERPROFILE%\rapidr-release\out
#   tools/release/windows-vm.sh smoke <installer file name> [smoke.ps1 options…]
#   tools/release/windows-vm.sh ps <script.ps1 in the repo> [args…]
#   tools/release/windows-vm.sh fetch <file in …\rapidr-release\out> <local file>
#
# Each run is detached in the VM and its log polled (a long prlctl exec can drop
# while the VM goes on); the log is printed when it ends. A VM that paused itself
# ("Pause idle") is resumed first.
set -euo pipefail
source "$(dirname "$0")/common.sh"
VM="${RAPIDR_WIN_VM:-Windows 11 Pro}"
REL="${ROOT#"$HOME"/}"
[ "$REL" = "$ROOT" ] && die "the repository must be under your home folder (the VM sees it as \\\\Mac\\Home)"
SHARE="\\\\Mac\\Home\\${REL//\//\\}"
EXEC="$ROOT/tools/windows/vmexec.sh"

awake() {
    local state
    state="$(prlctl list -a -o status,name | awk -v vm="$VM" '{s=$1; $1=""; sub(/^ /,""); if ($0 == vm) print s}')"
    case "$state" in
        running) ;;
        paused|suspended) prlctl resume "$VM" >/dev/null ;;
        *) die "the VM \"$VM\" is $state" ;;
    esac
}

# A PowerShell script of the repository, detached, its output in a log in the VM.
ps() {
    local script="${1//\//\\}"; shift
    local job="rr-$(date +%s)"
    awake
    # (`(if …)`: an IF without parentheses would take the rest of the line as its command)
    "$EXEC" "(if not exist %USERPROFILE%\\rapidr-release mkdir %USERPROFILE%\\rapidr-release) & powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\detach.ps1 -Script $SHARE\\$script -Log %USERPROFILE%\\rapidr-release\\$job.log $*"
    echo "(in the VM: %USERPROFILE%\\rapidr-release\\$job.log)"
    local done=""
    while [ -z "$done" ]; do
        sleep 20
        awake
        done="$("$EXEC" "if exist %USERPROFILE%\\rapidr-release\\$job.log.done type %USERPROFILE%\\rapidr-release\\$job.log.done" 2>/dev/null | tr -d '\r ' || true)"
    done
    "$EXEC" "type %USERPROFILE%\\rapidr-release\\$job.log" | tr -d '\r'
    [ "$done" = 0 ]
}

case "${1:-}" in
    tools) ps tools/release/windows/setup-tools.ps1 ;;
    build) shift; ps tools/release/windows/windows.ps1 -Prep "$SHARE\\dist\\$VERSION\\prep" "$@" ;;
    smoke) inst="$2"; shift 2; ps tools/release/windows/smoke.ps1 -Installer "%USERPROFILE%\\rapidr-release\\out\\$inst" "$@" ;;
    ps) shift; ps "$@" ;;
    # (a short script, waited for: its output straight back)
    sync)
        shift; script="${1//\//\\}"; shift
        awake
        "$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\$script $*"
        ;;
    fetch)
        # (base64 over prlctl, in pieces checked by SHA-256: the share is only read)
        awake
        bash "$ROOT/tools/release/winvm-fetch.sh" "$VM" "$2" "$3"
        ;;
    *) die "usage: $0 tools|build|smoke|ps|fetch …" ;;
esac
