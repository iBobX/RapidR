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
#   tools/release/windows-vm.sh clean [--dry-run] [--keep-out]
#                                            remove everything the release and smoke scripts left in
#                                            the VM (windows/clean.ps1: %USERPROFILE%\rapidr-release —
#                                            builds, cargo target, installers — the build cache, the
#                                            smoke tests' temporary folders and registrations, their
#                                            test apps); rapidr-tools, the user's install, Rust and
#                                            files stay. --dry-run lists with sizes; --keep-out keeps
#                                            the installers. Run it when a release is done.
#   tools/release/windows-vm.sh sync <script.ps1> [args…]    a short script, waited for (as SYSTEM)
#   tools/release/windows-vm.sh flashtest [control]
#                                            check that no console window opens on the VM's desktop
#
# No window on the user's desktop. A call that runs in the user's own session
# (`prlctl exec --current-user`) opens a console window there every time — with a poll every 20
# seconds and a file fetched in pieces, a flash all day. So every call here runs as SYSTEM (in
# Windows' session 0: no desktop; tools/windows/vmexec.sh -S), naming the user's files by their
# full path (windows/user.ps1 finds the profile), and the long jobs run as the user through a
# one-off scheduled task that starts `conhost.exe --headless powershell …` (windows/detach.ps1):
# a console that has no window. `flashtest` proves it (windows/watch-windows.ps1).
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
# One of the scripts of tools/release/windows/, run as SYSTEM: wps <script.ps1> [args…]
wps() { local s="$1"; shift; "$EXEC" -S "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\tools\\release\\windows\\$s $*" | tr -d '\r'; }
# A command line, run as SYSTEM
wsys() { "$EXEC" -S "$1" | tr -d '\r'; }

awake() {
    local state
    state="$(prlctl list -a -o status,name | awk -v vm="$VM" '{s=$1; $1=""; sub(/^ /,""); if ($0 == vm) print s}')"
    case "$state" in
        running) ;;
        paused|suspended) prlctl resume "$VM" >/dev/null ;;
        *) die "the VM \"$VM\" is $state" ;;
    esac
}
# The user's profile folder in the VM (asked once): SYSTEM's %USERPROFILE% is not it.
UP=""
profile() {
    [ -n "$UP" ] && return
    local who; who="$(wps whoami.ps1 || true)"
    UP="$(printf '%s\n' "$who" | sed -n '/^[A-Za-z]:\\/{p;q;}')"
    [ -n "$UP" ] || die "no user's profile found in the VM: $who"
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
    awake; profile
    wps detach.ps1 -Script "$script" -Name "$name" $prep $joined
    echo "(in the VM: $UP\\rapidr-release\\$name.log)"
    local done=""
    while [ -z "$done" ]; do
        sleep 20
        awake
        done="$(wsys "if exist $UP\\rapidr-release\\$name.log.done type $UP\\rapidr-release\\$name.log.done" 2>/dev/null | tr -d ' ' || true)"
    done
    wsys "type $UP\\rapidr-release\\$name.log"
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
    smoke) inst="$2"; shift 2; awake; profile; job smoke.ps1 "Installer=$UP\\rapidr-release\\out\\$inst" "$@" ;;
    stop) awake; wps stop.ps1 ;;
    clean)
        shift; flags=(-Delete)
        for a in "$@"; do
            case "$a" in
                --dry-run) flags=() ;;
                --keep-out) flags+=(-KeepOut) ;;
                *) die "usage: $0 clean [--dry-run] [--keep-out]" ;;
            esac
        done
        awake
        # (what is still running of the release jobs ends first: a build holds its folder open)
        [ "${flags[0]:-}" = -Delete ] && wps stop.ps1 > /dev/null
        wps clean.ps1 ${flags[@]+"${flags[@]}"}
        ;;
    sync)
        shift; script="${1//\//\\}"; shift
        awake
        wsys "powershell -NoProfile -ExecutionPolicy Bypass -File $SHARE\\$script $*"
        ;;
    fetch)
        # (base64 over prlctl in pieces of 6 MB, each checked by SHA-256: the share is only read)
        awake; profile
        out="$3"; : > "$out"
        src="$UP\\rapidr-release\\out\\$2"
        n="$(wps piece.ps1 -File "$src" -Count | tr -d ' ')"
        tmp="$(mktemp)"; bin="$(mktemp)"
        for ((i = 0; i < n; i++)); do
            ok=""
            for try in 1 2 3 4 5; do
                wps piece.ps1 -File "$src" -Index $i > "$tmp" || continue
                grep -v '^sha256 ' "$tmp" | base64 -d > "$bin" 2>/dev/null || continue
                [ "$(sed -n 's/^sha256 //p' "$tmp")" = "$(shasum -a 256 "$bin" | cut -d' ' -f1)" ] && { ok=1; break; }
            done
            [ -n "$ok" ] || die "fetch $2: piece $i failed"
            cat "$bin" >> "$out"
        done
        rm -f "$tmp" "$bin"
        want="$(wps piece.ps1 -File "$src" -Sum | tr -d ' ')"
        [ "$want" = "$(shasum -a 256 "$out" | cut -d' ' -f1)" ] || die "fetch $2: checksum differs"
        echo "fetched $2 ($(du -h "$out" | cut -f1), sha256 $want)"
        ;;
    flashtest)
        # A watcher in the user's session logs every window that appears; meanwhile a job is
        # started, polled and read, and a small file fetched — the calls of a release. `control`
        # adds one call in the user's session, which must show up: the watcher works.
        awake; profile
        t="$(date +%s)"; watch="flashtest-watch-$t"; small="flashtest-job-$t"; R="$UP\\rapidr-release"
        stamp() { echo "$(date +%T)  $*"; }
        donefile() { wsys "if exist $R\\$1.log.done type $R\\$1.log.done" | tr -d ' '; }
        wps detach.ps1 -Script watch-windows.ps1 -Name "$watch" "-Pass Seconds=45;Out=$R\\$watch.txt" > /dev/null
        sleep 12
        # (2 seconds long, and its window titled FLASHCONTROL: a window that is there for a few
        # milliseconds slips between two polls, and another caller's windows are about)
        if [ "${2:-}" = control ]; then stamp "control: a call in the user's session"; "$EXEC" "title FLASHCONTROL & ping -n 3 127.0.0.1 > nul" > /dev/null; sleep 2; fi
        stamp "start a job"; wps detach.ps1 -Script whoami.ps1 -Name "$small" > /dev/null
        for i in 1 2 3; do sleep 2; stamp "poll"; donefile "$small" > /dev/null; done
        wsys "type $R\\$small.log" | sed 's/^/        job log: /'
        stamp "fetch a file"
        wsys "(if not exist $R\\out mkdir $R\\out) & fsutil file createnew $R\\out\\flashtest.bin 7000000" > /dev/null
        got="$(mktemp)"; "$0" fetch flashtest.bin "$got" | sed 's/^/        /'
        wsys "del /q $R\\out\\flashtest.bin" > /dev/null
        stamp "calls done; waiting for the watcher"
        while [ -z "$(donefile "$watch")" ]; do sleep 3; done
        wsys "type $R\\$watch.txt" > "$got"
        echo "console windows that appeared (time, process, class, title; cmd= when it could be told):"
        grep CONSOLE "$got" | sed 's/^/  /' || echo "  none"
        echo "(other callers' --current-user calls show up here too — their cmd= or the time says whose)"
        if [ "${2:-}" = control ]; then
            grep -q FLASHCONTROL "$got" && echo "control: seen (title FLASHCONTROL) — the watcher works, a call in the user's session does open a window" \
                || echo "control: NOT seen — the watcher missed it; run again"
        fi
        rm -f "$got"
        ;;
    *) die "usage: $0 tools|build|smoke|fetch|stop|clean|sync|flashtest …" ;;
esac
