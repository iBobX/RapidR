#!/bin/bash
# The import table of a Windows .exe as DLL!Function lines (sorted), and what plain Windows XP SP3 lacks
# of it (tools/winxp/xp-missing.txt: functions Microsoft Learn gives a minimum client after XP, and
# the DLLs XP does not have).
#
#   tools/winxp/imports.sh program.exe                 every import, then "XP lacks:" and the rest
#   tools/winxp/imports.sh program.exe other.exe       what one has that the other has not
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
list() { "$LLVM_MINGW/bin/llvm-readobj" --coff-imports "$1" | awk '/^  Name: /{d=tolower($2)} /Symbol: /{print d "!" $2}' | sort -u; }
if [ "$#" -eq 2 ]; then
    a="$(mktemp)"; b="$(mktemp)"
    list "$1" > "$a"; list "$2" > "$b"
    echo "== only in $1:"; comm -23 "$a" "$b"
    echo "== only in $2:"; comm -13 "$a" "$b"
    rm -f "$a" "$b"
    exit 0
fi
all="$(list "$1")"
echo "$(wc -l <<< "$all" | tr -d ' ') imports from $(cut -d! -f1 <<< "$all" | sort -u | wc -l | tr -d ' ') DLLs"
echo "== XP lacks:"
missing=0
while read -r entry; do
    dll="${entry%%!*}"; fn="${entry#*!}"
    if grep -qx "dll $dll" "$HERE/xp-missing.txt" || grep -qx "fn $fn" "$HERE/xp-missing.txt"; then
        echo "  $entry"; missing=$((missing + 1))
    fi
done <<< "$all"
echo "$missing missing on plain Windows XP SP3"
[ "$missing" -eq 0 ]
