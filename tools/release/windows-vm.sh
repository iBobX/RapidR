#!/bin/bash
# Runs the Windows release scripts in the Windows VM (Parallels; RAPIDR_WIN_VM
# names it). The VM reads this repository through the Mac's home share
# (\\Mac\Home) — only reads; nothing is written to the share.
#
#   tools/release/windows-vm.sh tools        the build tools (setup-tools.ps1)
#   tools/release/windows-vm.sh build [Name=value…]   the installers (windows.ps1's
#                                            parameters: Arch=x86_64, Toolchain=msvc,
#                                            Sign=…), in the VM's %USERPROFILE%\rapidr-release\out
#   tools/release/windows-vm.sh smoke <installer file name> [Native] [Associations]
#   tools/release/windows-vm.sh fetch <file in …\rapidr-release\out> <local file>
#   tools/release/windows-vm.sh stop         stop the release jobs running in the VM
#   tools/release/windows-vm.sh clean        remove %USERPROFILE%\rapidr-release (rapidr-tools stays)
#   tools/release/windows-vm.sh sync <script.ps1> [args…]    a short script, waited for
#
# A job is started detached in the VM (windows/detach.ps1) and its log polled — a
# long prlctl exec drops; the log is printed when it ends. prepare.sh's output is
# read from a copy in the repository's .release-share/ (dist/ may be a link to
# another volume, which the share doesn't follow), removed afterwards. A VM that
# paused itself ("Pause idle") is resumed first.
set -euo pipefail
source "$(dirname "$0")/common.sh"
VM="${RAPIDR_WIN_VM:-Windows 11 Pro}"
REL="${ROOT#"$HOME"/}"
[ "$REL" = "$ROOT" ] && die "the repository must be under your home folder (the VM sees it as \\\\Mac\\Home)"
SHARE="\\\\Mac\\Home\\${REL//\//\\}"
EXEC="$ROOT/tools/windows/vmexec.sh"
VMREL='%USERPROFILE%\rapidr-release'

awake() {
    local state
    state="$(prlctl list -a -o status,name | awk -v vm="$VM" '{s=$1; $1=""; sub(/^ /,""); if ($0 == vm) print s}')"
    case "$state" in
        running) ;;
        paused|suspended) prlctl resume "$VM" >/dev/null ;;
        *) die "the VM \"$VM\" is $state" ;;
    esac
}

# One of tools/release/windows/*.ps1, detached, its output in a log in the VM:
# job <script> [-Prep <share path>] [Name=value | Switch]…
job() {
    local script="$1"; shift
    local prep="" pass=()
    if [ "${1:-}" = -Prep ]; then prep="-Prep $2"; shift 2; fi
    for a in "$@"; do pass+=("$a"); done
    # (';'-separated, unquoted: cmd reads `|` as a pipe, and quotes inside `cmd /c "…"` get lost)
    local joined; joined="$(IFS=';'; echo "${pass[*]:-}")"
    [ -n "$joined" ] && joined="-Pass $joined"
    local name="rr-$(date +%s)"
    awake
    # (`(if …)`: an IF without parentheses would take the rest of the line as its command)
    "$EXEC" "(if not exist $VMREL mkdir $VMREL) & powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\detach.ps1 -Script $script -Log $VMREL\\$name.log $prep $joined"
    echo "(in the VM: $VMREL\\$name.log)"
    local done=""
    while [ -z "$done" ]; do
        sleep 20
        awake
        done="$("$EXEC" "if exist $VMREL\\$name.log.done type $VMREL\\$name.log.done" 2>/dev/null | tr -d '\r ' || true)"
    done
    "$EXEC" "type $VMREL\\$name.log" | tr -d '\r'
    [ "$done" = 0 ]
}

case "${1:-}" in
    tools) job setup-tools.ps1 ;;
    build)
        shift
        [ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"
        rm -rf "$ROOT/.release-share" && mkdir -p "$ROOT/.release-share"
        cp -R "$PREP" "$ROOT/.release-share/prep"
        trap 'rm -rf "$ROOT/.release-share"' EXIT
        job windows.ps1 -Prep "$SHARE\\.release-share\\prep" "$@"
        ;;
    smoke) inst="$2"; shift 2; job smoke.ps1 "Installer=$VMREL\\out\\$inst" "$@" ;;
    stop) awake; "$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\stop.ps1" ;;
    clean) awake; "$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\stop.ps1" > /dev/null; "$EXEC" "rmdir /s /q $VMREL & echo removed $VMREL" ;;
    sync)
        shift; script="${1//\//\\}"; shift
        awake
        "$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\$script $*"
        ;;
    fetch)
        # (base64 over prlctl in pieces, each checked by SHA-256: the share is only read)
        awake
        out="$3"; : > "$out"
        n="$("$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\piece.ps1 -File $VMREL\\out\\$2 -Count" | tr -d '\r ')"
        tmp="$(mktemp)"; bin="$(mktemp)"
        for ((i = 0; i < n; i++)); do
            ok=""
            for try in 1 2 3 4 5; do
                "$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\piece.ps1 -File $VMREL\\out\\$2 -Index $i" > "$tmp" || continue
                grep -v '^sha256 ' "$tmp" | tr -d '\r' | base64 -d > "$bin"
                [ "$(sed -n 's/^sha256 //p' "$tmp" | tr -d '\r')" = "$(shasum -a 256 "$bin" | cut -d' ' -f1)" ] && { ok=1; break; }
            done
            [ -n "$ok" ] || die "fetch $2: piece $i failed"
            cat "$bin" >> "$out"
        done
        rm -f "$tmp" "$bin"
        want="$("$EXEC" "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\piece.ps1 -File $VMREL\\out\\$2 -Sum" | tr -d '\r ')"
        [ "$want" = "$(shasum -a 256 "$out" | cut -d' ' -f1)" ] || die "fetch $2: checksum differs"
        echo "fetched $2 ($(du -h "$out" | cut -f1), sha256 $want)"
        ;;
    *) die "usage: $0 tools|build|smoke|fetch|sync …" ;;
esac
