#!/bin/bash
# Drives the REAL Windows 7 VM (UTM, "Windows 7 x86", tools/win7/vm.sh through its QEMU guest agent) for the
# build matrix: the Windows 7 alpha zip is used on an actual Windows 7, not on Windows 11's emulation.
#
#   tools/release/win7-vm.sh smoke <zip> [<folder for the pictures>]
#       puts the zip, windows/smoke-win7-real.ps1 and the test-mode token in C:\rapidr-matrix, runs the script as
#       the user logged in at the VM's desktop (a scheduled task: the guest agent itself runs as SYSTEM in
#       session 0, where a window has no desktop), prints the script's log, fetches the pictures it took
#       (win7-desktop.png, win7-gui-hello.png), then removes C:\rapidr-matrix and the task. Exit 0 only when
#       the log ends "== smoke test passed".
#   tools/release/win7-vm.sh clean
#       removes C:\rapidr-matrix and the task (what a stopped smoke test may have left)
#
# Needs: the VM started and somebody logged in at its desktop (it never starts, stops or reboots the VM: ask
# Robert), the guest agent working (tools/win7/README.md, "Automating the Windows 7 VM"), and the test-mode
# token (~/.rapidr/test-mode.token, README "The test-mode token": the GUI checks fail without it; it is copied
# into C:\rapidr-matrix only, never into the user's profile, and goes away with the folder).
# Environment: WIN7_WAIT (seconds to wait for the smoke test, default 2400), RAPIDR_TEST_MODE_FILE.
# Everything made in the VM is under C:\rapidr-matrix (and a task called rapidr-matrix); the zip stays the
# same size on a second run: a copy with the same size is not sent again (the script checks its SHA-256).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
VMSH="$ROOT/tools/win7/vm.sh"
W='C:\rapidr-matrix'
TASK=rapidr-matrix
die() { echo "error: $*" >&2; exit 1; }
vm() { "$VMSH" "$@"; }

state() { vm status 2>/dev/null | tr -d '\r' | head -1; }
guest() { CAP="${CAP:-120}" vm exec "$@"; }
clean_guest() {
    guest "schtasks /delete /tn $TASK /f" > /dev/null 2>&1
    guest "if exist $W rmdir /s /q $W" > /dev/null 2>&1
    guest "if exist $W echo STILL-THERE"
}

case "${1:-}" in
    clean)
        [ "$(state)" = started ] || die "the Windows 7 VM is '$(state)'"
        out="$(clean_guest)"
        case "$out" in *STILL-THERE*) die "C:\\rapidr-matrix is still there" ;; esac
        echo "removed $W and the task"
        exit 0 ;;
    smoke) ;;
    *) sed -n '2,19p' "$0" >&2; exit 2 ;;
esac

ZIP="${2:-}"; SHOTS="${3:-}"
[ -f "$ZIP" ] || die "usage: $0 smoke <zip> [<folder for the pictures>]"
st="$(state)"
[ "$st" = started ] || die "the Windows 7 VM is '${st:-not found}' (this script never starts or stops a VM: ask Robert)"
SUM="$(shasum -a 256 "$ZIP" | cut -d' ' -f1)"
SIZE="$(stat -f %z "$ZIP")"
TOKEN="${RAPIDR_TEST_MODE_FILE:-$HOME/.rapidr/test-mode.token}"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/win7-vm.XXXXXX")"
trap 'rm -rf "$TMP"; clean_guest > /dev/null 2>&1' EXIT
trap 'echo "(interrupted: C:\\rapidr-matrix is removed)"; exit 130' INT TERM HUP

# Who is at the desktop? (the task runs as that user, "only when logged on": no password is needed or typed)
USER_AT_DESK="$(guest 'query user' | awk 'NR==2 { sub(/^>/, "", $1); print $1 }')"
[ -n "$USER_AT_DESK" ] || die "nobody is logged in at the Windows 7 desktop (the GUI programs need one)"
echo "== Windows 7 VM: $(guest ver | tr -d '\r' | grep -i version), user at the desktop: $USER_AT_DESK"

guest "if not exist $W mkdir $W" > /dev/null
# the zip: sent unless the VM has a file of this size (the script checks the SHA-256)
have="$(guest "dir /-c $W\\z.zip" 2>/dev/null | tr -d '\r' | awk '$NF == "z.zip" { print $(NF-1); exit }')"
if [ "$have" = "$SIZE" ]; then
    echo "== the zip is in the VM already ($SIZE bytes)"
else
    echo "== sending $(basename "$ZIP") ($SIZE bytes) to the VM"
    CAP=1200 vm push "$ZIP" "$W\\z.zip" || die "could not send the zip"
fi
CAP=120 vm push "$HERE/windows/smoke-win7-real.ps1" "$W\\smoke.ps1" || die "could not send the script"
tokenarg=""
if [ -f "$TOKEN" ]; then
    CAP=60 vm push "$TOKEN" "$W\\test-mode.token" && tokenarg="-Token $W\\test-mode.token"
else
    echo "(no test-mode token at $TOKEN: the GUI checks will say so)"
fi
# the task's program: the script with its output in a log, then the marker the Mac waits for
printf '@echo off\r\npowershell -NoProfile -ExecutionPolicy Bypass -File %s\\smoke.ps1 -Zip %s\\z.zip -Sha256 %s %s -Work %s > %s\\log.txt 2>&1\r\necho %%errorlevel%%> %s\\exit.txt\r\necho done> %s\\done.txt\r\n' "$W" "$W" "$SUM" "$tokenarg" "$W" "$W" "$W" "$W" > "$TMP/run.cmd"
CAP=60 vm push "$TMP/run.cmd" "$W\\run.cmd" || die "could not send run.cmd"

guest "schtasks /create /tn $TASK /tr $W\\run.cmd /sc once /st 23:59 /ru $USER_AT_DESK /it /f" | tr -d '\r' | grep -v '^$'
guest "schtasks /run /tn $TASK" | tr -d '\r' | grep -v '^$'
t0=$(date +%s)
WAIT="${WIN7_WAIT:-2400}"
while :; do
    sleep 20
    if guest "if exist $W\\done.txt echo FINISHED" 2>/dev/null | grep -q FINISHED; then break; fi
    now=$(( $(date +%s) - t0 ))
    if [ "$now" -gt "$WAIT" ]; then
        echo "== no end after ${WAIT}s: the log so far" >&2
        CAP=120 vm pull "$W\\log.txt" "$TMP/log.txt" 2>/dev/null && cat "$TMP/log.txt"
        echo "== smoke test FAILED (timeout; C:\\rapidr-matrix is removed, any program still running there is not)"
        exit 1
    fi
done
CAP=300 vm pull "$W\\log.txt" "$TMP/log.txt" || die "could not fetch the log"
tr -d '\r' < "$TMP/log.txt"
echo "(the smoke test took $(( $(date +%s) - t0 )) s on the Windows 7 VM)"
if [ -n "$SHOTS" ]; then
    mkdir -p "$SHOTS"
    for p in win7-desktop.png win7-gui-hello.png; do
        if CAP=300 vm pull "$W\\shots\\$p" "$TMP/$p" 2>/dev/null && [ "$(head -c 4 "$TMP/$p" | od -An -c | tr -d ' ')" = '211PNG' ]; then
            cp "$TMP/$p" "$SHOTS/$p"; echo "picture: $SHOTS/$p"
        fi
    done
fi
grep -q '^== smoke test passed' "$TMP/log.txt"
