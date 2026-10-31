#!/bin/bash
# Preview 5's features, looked at headlessly in an installed (or built) RapidR
# Studio, on macOS and Linux (windows/p5check.ps1 does the same on Windows):
#
#   * Versions: a copy of a small project is put under versions, "first" is
#     saved, a line is changed and "second" saved, Go Back to "first" (the
#     question answered): three versions, the file as it was
#   * the Coach dial is in RAI's pane (Build / Coach), Coach can be chosen
#   * Set Up RAI > On this computer: with no local AI app, a calm
#     "No local AI app found running on this computer" (nothing started)
#   * RapidR Studio in Spanish (--lang es): Spanish menus, the dial in Spanish
#   * School Lock's dialog opens (Tools > School Lock...), in English
#
#   tools/release/p5check.sh <the rapidr command> [<folder to keep the captures in>]
#
# The test hooks of a release build act only with a signed test-mode token
# (SEC-24, docs/security/test-mode.md): RAPIDR_TEST_MODE, or the harnesses'
# own ~/.rapidr/test-mode.token. Nothing here touches the user's settings,
# keychain, printer, registry store, documents or projects: every run has its
# own folders. P5_PROBES=<url> sends Set Up RAI's search for local AI apps to
# one address (a computer with Ollama on it: a closed port, to see the empty
# state); RAPIDR_HOME names RapidR's home for a command that isn't an install's
# (a checkout's target/release/rapidr).
# Exit status 0 when every check passed.
set -uo pipefail
R="${1:-}"
[ -n "$R" ] && [ -x "$R" ] || { echo "usage: $0 <the rapidr command> [<folder to keep the captures in>]"; exit 2; }
if [ -n "${2:-}" ]; then T="$2"; mkdir -p "$T"; KEEP=1; else T="$(mktemp -d "${TMPDIR:-/tmp}/rapidr-p5.XXXXXX")"; KEEP=0; fi
T="$(cd "$T" && pwd -P)"
cleanup() { [ "$KEEP" = 1 ] || rm -rf "$T"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
if [ -z "${RAPIDR_TEST_MODE:-}" ] && [ -f "$HOME/.rapidr/test-mode.token" ]; then export RAPIDR_TEST_MODE="$HOME/.rapidr/test-mode.token"; fi
FAIL=0
ok() { echo "  ok    $*"; }
bad() { echo "  FAIL  $*"; FAIL=1; }
# check <what> <file> <regex>: a line of the file matches
has() { grep -Eq -- "$3" "$2" 2>/dev/null; }
check() { if has "$@"; then ok "$1"; else bad "$1"; echo "        (wanted /$3/ in $2:)"; sed 's/^/        | /' "$2" 2>/dev/null | head -12; fi; }

# studio <case> <language|-> <steps> <dump keys> <events|-> [<file to open>]
# One headless Studio run on its own folders; its dump (and anything it said) in $T/<case>/out.txt,
# the window's picture in $T/<case>/window-1.bmp.
studio() {
    local name="$1" lang="$2" steps="$3" keys="$4" events="$5" file="${6:-}" d="$T/$1"
    rm -rf "$d"; mkdir -p "$d/docs" "$d/prints"
    local args=(ide --fresh --theme rapidr-light)
    [ "$lang" != "-" ] && args+=(--lang "$lang")
    args+=(--do "$steps")
    [ -n "$file" ] && args+=("$file")
    local envs=(RAPIDR_CAPTURE="$d/window" RAPIDR_CAPTURE_DELAY="${P5_DELAY:-5}" RAPIDR_MENU=window
        RAPIDR_TEST_DUMP="$keys,studio.caption"
        RAPIDR_PRINT_TO="$d/prints" RAPIDR_REGISTRY="$d/reg.reg" RAPIDR_CONFIG_DIR="$d/config" RAPIDR_DOCUMENTS="$d/docs"
        RAPIDR_AI_KEYSTORE=memory RAPIDR_AI_CHATS="$d/chats" RAPIDR_STUDIO_DATA="$d/studio" RAPIDR_LOGIN_PATH=off
        ANTHROPIC_API_KEY= OPENAI_API_KEY= DEEPSEEK_API_KEY= ANTHROPIC_WORKSPACE_ID=)
    [ "$events" != "-" ] && envs+=(RAPIDR_TEST_EVENTS="$events")
    env "${envs[@]}" "$R" "${args[@]}" > "$d/out.txt" 2>&1 < /dev/null
    echo "rc=$?" >> "$d/out.txt"
}
CLICK() { echo "$1.__mousedown_$2_$3,$1.__mouseup_$2_$3"; }
IDLE() { local i out=""; for ((i = 0; i < $1; i++)); do out="${out:+$out,}aionnote.onclick"; done; echo "$out"; }
AI_ON="ai:on,ai:provider=compatible,ai:base=http://127.0.0.1:9/v1,ai:model=mock-small"
echo "== $("$R" version | head -1), test hooks: $("$R" dev test-mode status 2>&1 | grep '^hooks:')"

echo "== Versions: save, change, save, go back (a copy of a small project)"
P="$T/versions-project"
rm -rf "$P"; mkdir -p "$P"
cat > "$P/Hello.rrproj" <<'EOF'
# RapidR Studio project (format 2)

format = 2
name = "Hello"
main = "main.rr"

[[files]]
path = "main.rr"
kind = "module"

[[files]]
path = "greet.rr"
kind = "module"
EOF
cat > "$P/greet.rr" <<'EOF'
' greet.rr: the greeting Hello prints.
FUNCTION Greeting(Who AS STRING) AS STRING
    Result = "Hello, " + Who + "!"
END FUNCTION
EOF
cat > "$P/main.rr" <<'EOF'
' Hello: a greeting and a count.
$INCLUDE "greet.rr"

DIM Name AS STRING
Name = "Ana"
PRINT Greeting(Name)
PRINT "Counting to three:"
FOR i = 1 TO 3
    PRINT i
NEXT
PRINT "Done."
EOF
studio versions - 'versions.pane,versions.start,versions:author=Ana,versions:save=first,code:three=>four,file.save,versions:save=second,versions:pick=first,versions.goBack,wait,versions.pane' \
    'repo.versioned,vertimeline.itemcount,repo.experiment,verundonote.visible,outputbox.text' \
    "$(CLICK consentyes 40 14),verpanel.__wait_2500" "$P/Hello.rrproj"
check "Studio started and kept running" "$T/versions/out.txt" '^studio.caption=Hello - RapidR Studio$'
check "the project is under versions" "$T/versions/out.txt" '^repo.versioned=(1|-1|True)$'
check "three versions: first, second, and the one that went back" "$T/versions/out.txt" '^vertimeline.itemcount=3$'
check "Go Back said what it did" "$T/versions/out.txt" 'Went back to .+first'
check "Undo is offered" "$T/versions/out.txt" '^verundonote.visible=(1|-1|True)$'
check "main.rr is as it was in the first version" "$P/main.rr" 'Counting to three:'
check "the versions are a repository on disk" <(ls "$P/.git/HEAD" 2>&1) 'HEAD'

echo "== Coach: the dial, and Coach chosen"
studio coach - "$AI_ON,ai.assistant,ai:coach:mode=coach,wait" 'aidialbuildtext.caption,aidialcoachtext.caption,aitools.mode,ai.insertlinks' - "$T/versions-project/main.rr"
check "the dial says Build" "$T/coach/out.txt" '^aidialbuildtext.caption=Build$'
check "the dial says Coach" "$T/coach/out.txt" '^aidialcoachtext.caption=Coach$'
check "Coach is the mode" "$T/coach/out.txt" '^aitools.mode=coach$'
check "no Insert in Coach mode" "$T/coach/out.txt" '^ai.insertlinks=(0|False)$'

echo "== Set Up RAI > On this computer: no local AI app found"
studio localai - "${P5_PROBES:+ai:setupprobes=$P5_PROBES,}ai:setup,wait" 'aisufoundtext.caption,aisumodels.visible' "$(IDLE 8),$(CLICK aisunextbtn 30 14),$(IDLE 4)" "$T/versions-project/main.rr"
LA=localai
# (a computer that has a local AI app: its models are found — good —, and the empty state is looked at with a closed port instead)
if ! has x "$T/localai/out.txt" '^aisufoundtext.caption=No local AI app found' && has x "$T/localai/out.txt" '^aisufoundtext.caption=.*Found [0-9]+ model'; then
    ok "this computer has a local AI app: Set Up RAI found its models ($(grep -o 'Found [0-9]* model[s]*' "$T/localai/out.txt" | head -1)); the empty state next, with a closed port"
    studio localai-empty - "ai:setupprobes=http://127.0.0.1:9/v1,ai:setup,wait" 'aisufoundtext.caption,aisumodels.visible' "$(IDLE 8),$(CLICK aisunextbtn 30 14),$(IDLE 4)" "$T/versions-project/main.rr"
    LA=localai-empty
fi
check "says calmly that no local AI app was found" "$T/$LA/out.txt" '^aisufoundtext.caption=No local AI app found running on this computer'
check "lists no models" "$T/$LA/out.txt" '^aisumodels.visible=(0|False)$'

echo "== Studio in Spanish"
studio spanish es "$AI_ON,ai.assistant,wait" 'mi(0).caption,mi(1).caption,mi(2).caption,mi(9).caption,aidialbuildtext.caption,aidialcoachtext.caption' - "$T/versions-project/main.rr"
check "the File menu is Archivo" "$T/spanish/out.txt" '^mi\(0\)\.caption=&Archivo$'
check "the Edit menu is Edici" "$T/spanish/out.txt" '^mi\(1\)\.caption=&Edici.+n$'
check "the Help menu is Ayuda" "$T/spanish/out.txt" '^mi\(9\)\.caption=Ay&uda$'
check "the dial is Construir / Tutor" "$T/spanish/out.txt" '^aidialbuildtext.caption=Construir$'
check "the dial's Coach is Tutor" "$T/spanish/out.txt" '^aidialcoachtext.caption=Tutor$'

echo "== School Lock's dialog opens"
studio lock - "tools.schoolLock" 'lockdialog.visible,lockpwedit.visible,lockdialog.caption' - "$T/versions-project/main.rr"
check "the dialog is open" "$T/lock/out.txt" '^lockdialog.visible=(1|-1|True)$'
check "it asks for the password" "$T/lock/out.txt" '^lockpwedit.visible=(1|-1|True)$'
check "it is School Lock" "$T/lock/out.txt" '^lockdialog.caption=.*School Lock'

echo "== the pictures"
for c in versions coach "$LA" spanish lock; do
    [ -f "$T/$c/window-1.bmp" ] && ok "$c: $(wc -c < "$T/$c/window-1.bmp" | tr -d ' ') bytes of window" || bad "$c: no picture of Studio's window"
done
[ "$KEEP" = 1 ] && echo "(the runs' folders, with the pictures, are kept in $T)"
if [ "$FAIL" = 0 ]; then echo "Preview 5 checks: all passed"; else echo "Preview 5 checks: FAILED"; fi
exit "$FAIL"
