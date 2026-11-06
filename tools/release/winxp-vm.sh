#!/bin/bash
# Drives the REAL Windows XP VM (UTM, "Windows XP": plain XP SP3 32-bit, no One-Core-API) for the build
# matrix's "winxp-real" row: the Windows XP test zip (tools/winxp/zip.sh) used on an actual Windows XP.
# XP has no QEMU guest agent: this goes through RapidR's XP test helper (tools/winxp/helper/START-HERE.vbs,
# started once by hand in the VM, version 3 or later) and tools/winxp/xphost.py on the host-only network.
#
#   tools/release/winxp-vm.sh smoke <zip> [<folder for the pictures>]
#       unpacks the zip in C:\rapidr-matrix-xp (XP's own Compressed Folders), then checks, as a person would
#       use it (real processes and windows; Studio's test hooks are not used, they need the test-mode token):
#         run     rapidr.exe version, a console program's output, a window program staying up with its window
#         interp  rapidr build --interp of the console and the window program (no Rust on XP)
#         exe     the two built programs: the console one's output, the window one up with its window
#         other   RapidR Studio starting with a program open (its window, after up to 3 minutes)
#       prints the log, a "##cells ..." line and "== smoke test passed" (exit 0) or "== smoke test FAILED";
#       takes pictures (winxp-gui.png, winxp-built-gui.png, winxp-studio.png); then stops what it started (by
#       process number) and removes C:\rapidr-matrix-xp.
#   tools/release/winxp-vm.sh clean      removes C:\rapidr-matrix-xp
#   tools/release/winxp-vm.sh ready      exit 0 when the helper (v3+) asked for work in the last 30 s
#
# It never starts, stops or reboots the VM, and never starts the helper (ask Robert). Programs run with
# RAPIDR_AI_KEYSTORE=memory and RAPIDR_REGISTRY / RAPIDR_PRINT_TO pointing into the work folder.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
VMSH="$ROOT/tools/winxp/vm.sh"
SHARE="${SHARE:-/Volumes/RapidRBuild/winxp-share}"
W='C:\rapidr-matrix-xp'
die() { echo "error: $*" >&2; exit 1; }
vm() { "$VMSH" "$@"; }

ready() {
    local line t v
    line="$(cat "$SHARE/last-seen.txt" 2>/dev/null)" || return 1
    t="$(date -j -f '%Y-%m-%d %H:%M:%S' "$(echo "$line" | cut -c1-19)" +%s 2>/dev/null)" || return 1
    v="$(echo "$line" | sed -n 's/.*[?&]*v=\([0-9]*\).*/\1/p')"
    [ $(( $(date +%s) - t )) -le 30 ] && [ "${v:-0}" -ge 3 ]
}

FAILS=0
CELL_RUN=ok CELL_INTERP=ok CELL_EXE=ok CELL_OTHER=ok
fail() {   # <cell> <what> <detail>
    echo "  FAIL  $2"
    [ -n "${3:-}" ] && echo "        ${3}"
    FAILS=$((FAILS + 1))
    case "$1" in run) CELL_RUN=FAIL ;; interp) CELL_INTERP=FAIL ;; exe) CELL_EXE=FAIL ;; other) CELL_OTHER=FAIL ;; esac
}
pass() { echo "  ok    $1"; }
# a command line in the VM, with the work folder's settings (one line: cmd.exe runs it)
ENV="set RAPIDR_AI_KEYSTORE=memory& set RAPIDR_REGISTRY=$W\\registry.json& set RAPIDR_PRINT_TO=$W\\print.txt& cd /d $W\\R"
xp() { LIMIT="${LIMIT:-300}" vm exec "$ENV& $*" 2>&1 | tr -d '\r'; }
# the process numbers of programs whose command line has <text>
# (the text through the environment: `awk -v` would read "\r" and "\x" of a Windows path as escapes)
pids_of() { CAP=60 vm ps 2>/dev/null | tr -d '\r' | T="$1" awk -F'\t' 'index($3, ENVIRON["T"]) > 0 {print $1}'; }
stop_all() { local p; for p in $(pids_of "$1"); do CAP=30 vm kill "$p" > /dev/null 2>&1; done; }
window_up() { CAP=30 vm keys "$1" '' 2>/dev/null | grep -q '^#typed'; }
picture() {   # <name>
    [ -n "$SHOTS" ] || return 0
    CAP=60 vm exec "C:\\xp\\xpshot.exe $W\\shot-$1.bmp" > /dev/null 2>&1
    if CAP=300 vm pull "$W\\shot-$1.bmp" "$SHOTS/$1.bmp" > /dev/null 2>&1; then
        sips -s format png "$SHOTS/$1.bmp" --out "$SHOTS/$1.png" > /dev/null 2>&1 && rm -f "$SHOTS/$1.bmp"
        echo "  (picture: $SHOTS/$1.png)"
    fi
}
# waits up to <seconds> for a window titled <title>
wait_window() {   # <title> <seconds>
    local n=0
    while [ $n -lt "$2" ]; do window_up "$1" && return 0; sleep 10; n=$((n + 10)); done
    return 1
}

case "${1:-}" in
    ready) ready; exit $? ;;
    clean) CAP=120 vm exec "if exist $W rmdir /s /q $W"; exit 0 ;;
    smoke) ;;
    *) sed -n '2,24p' "$0" >&2; exit 2 ;;
esac

ZIP="${2:?usage: winxp-vm.sh smoke <zip> [<folder for the pictures>]}"
SHOTS="${3:-}"
[ -f "$ZIP" ] || die "no $ZIP"
[ -n "$SHOTS" ] && mkdir -p "$SHOTS"
vm host > /dev/null
ready || die "the XP test helper is not asking for work (version 3+, in the last 30 s): ask Robert to start C:\\xp\\START-HERE.vbs in the XP VM"
NAME="$(basename "$ZIP" .zip)"
FILES="$(unzip -Z1 "$ZIP" | grep -vc '/$')"
echo "== the Windows XP smoke test: $NAME ($FILES files) on $(CAP=60 vm exec ver | tr -d '\r' | grep -m1 Windows)"

# --- into the VM
CAP=60 vm exec "if exist $W rmdir /s /q $W& mkdir $W" > /dev/null
sed 's/$/\r/' "$ROOT/tools/winxp/helper/unzip.js" > "$SHARE/files/unzip.js"
CAP=60 vm push "$SHARE/files/unzip.js" 'C:\xp\unzip.js' > /dev/null
[ -f "$SHARE/files/xpshot.exe" ] || "$ROOT/tools/winxp/probe/build.sh" > /dev/null
CAP=60 vm push "$SHARE/files/xpshot.exe" 'C:\xp\xpshot.exe' > /dev/null
cp "$ZIP" "$SHARE/files/matrix-xp.zip"
r="$(CAP=900 vm push "$SHARE/files/matrix-xp.zip" 'C:\xp\matrix-xp.zip')"
rm -f "$SHARE/files/matrix-xp.zip"
case "$r" in "#ok "*) pass "the zip copied in (${r#\#ok } bytes)" ;; *) fail other "the zip could not be copied in" "$r" ;; esac
r="$(LIMIT=900 vm exec "cscript //nologo C:\\xp\\unzip.js C:\\xp\\matrix-xp.zip $W\\unzip $FILES" | tr -d '\r')"
CAP=120 vm exec "move $W\\unzip\\$NAME $W\\R" > /dev/null
case "$r" in *"#unzipped $FILES of $FILES"*) pass "unpacked by XP: $FILES files" ;; *) fail other "the zip did not unpack" "$(echo "$r" | head -3)" ;; esac

# --- the test programs (written here, as the user would)
printf '%s\r\n' "' the matrix's console check on Windows XP" "DIM i AS INTEGER, s AS INTEGER" \
    "FOR i = 1 TO 10: s = s + i: NEXT i" 'PRINT "xp-console-ok "; s' > "$SHARE/files/xpcon.rr"
printf '%s\r\n' "' the matrix's window check on Windows XP" "CREATE Form AS RForm" '    Caption = "RapidR XP matrix"' \
    "    Width = 320: Height = 120: Center" "    CREATE Hello AS RLabel" '        Caption = "A RapidR window on Windows XP"' \
    "        Left = 16: Top = 20: Width = 280" "    END CREATE" "END CREATE" "Form.ShowModal" > "$SHARE/files/xpgui.rr"
CAP=60 vm push "$SHARE/files/xpcon.rr" "$W\\xpcon.rr" > /dev/null
CAP=60 vm push "$SHARE/files/xpgui.rr" "$W\\xpgui.rr" > /dev/null

# --- run
out="$(xp 'bin\rapidr.exe version')"
case "$out" in *"RapidR "*"#exit 0"*) pass "rapidr.exe version: $(echo "$out" | grep -m1 RapidR)" ;; *) fail run "rapidr.exe version" "$(echo "$out" | head -3)" ;; esac
out="$(xp "bin\\rapidr.exe run $W\\xpcon.rr")"
case "$out" in *"xp-console-ok 55"*"#exit 0"*) pass "a console program ran: xp-console-ok 55" ;; *) fail run "a console program" "$(echo "$out" | head -4)" ;; esac
CAP=30 vm spawn "cmd /c $ENV& bin\\rapidr.exe run $W\\xpgui.rr" > /dev/null
if wait_window "RapidR XP matrix" 120; then
    sleep 5
    if [ -n "$(pids_of 'run C:\rapidr-matrix-xp\xpgui.rr')" ]; then pass "a window program is up with its window"; picture winxp-gui
    else fail run "the window program ended by itself"; fi
else fail run "the window program's window did not show (120 s)" "$(CAP=60 vm ps | grep -i rapidr)"; fi
stop_all 'xpgui.rr'

# --- Build (interpreted) and the built programs
for p in xpcon xpgui; do
    out="$(LIMIT=600 xp "bin\\rapidr.exe build $W\\$p.rr --interp")"
    exe="$(echo "$out" | sed -n 's/^Built interpreted program: \(.*\.exe\) (.*/\1/p' | head -1)"
    if [ -n "$exe" ] && echo "$out" | grep -q '#exit 0'; then pass "rapidr build $p.rr --interp: $exe"; eval "EXE_$p=\"\$exe\""
    else fail interp "rapidr build $p.rr --interp" "$(echo "$out" | head -4)"; eval "EXE_$p=''"; fi
done
if [ -n "${EXE_xpcon:-}" ]; then
    out="$(xp "\"$EXE_xpcon\"")"
    case "$out" in *"xp-console-ok 55"*"#exit 0"*) pass "the built console program ran: xp-console-ok 55" ;; *) fail exe "the built console program" "$(echo "$out" | head -4)" ;; esac
else fail exe "no built console program to run"; fi
if [ -n "${EXE_xpgui:-}" ]; then
    CAP=30 vm spawn "\"$EXE_xpgui\"" > /dev/null
    if wait_window "RapidR XP matrix" 120; then pass "the built window program is up with its window"; picture winxp-built-gui
    else fail exe "the built window program's window did not show (120 s)"; fi
    stop_all "$(basename "${EXE_xpgui//\\//}")"
else fail exe "no built window program to run"; fi

# --- RapidR Studio
CAP=30 vm spawn "cmd /c $ENV& bin\\rapidr.exe ide $W\\xpcon.rr" > /dev/null
if wait_window "xpcon - RapidR Studio" 240; then sleep 10; pass "RapidR Studio started with xpcon.rr open"; picture winxp-studio
else fail other "RapidR Studio's window did not show (240 s)" "$(CAP=60 vm ps | grep -i rapidr)"; fi
stop_all ' ide C:\rapidr-matrix-xp'

# --- clean up
sleep 3
stop_all 'C:\rapidr-matrix-xp'
CAP=120 vm exec "if exist $W rmdir /s /q $W& del C:\\xp\\matrix-xp.zip" > /dev/null 2>&1
echo "##cells run=$CELL_RUN interp=$CELL_INTERP exe=$CELL_EXE native=- nexe=- other=$CELL_OTHER"
if [ $FAILS = 0 ]; then echo "== smoke test passed"; exit 0; else echo "== smoke test FAILED ($FAILS)"; exit 1; fi
