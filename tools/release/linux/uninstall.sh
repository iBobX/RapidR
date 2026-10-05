#!/bin/sh
# Removes what install.sh installed (its list: installed.txt beside this),
# and RapidR's defaults from mimeapps.list.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
LIST="$HERE/installed.txt"
[ -f "$LIST" ] || { echo "no $LIST: nothing to remove"; exit 1; }
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
FILES="$(cat "$LIST")"
for f in $FILES; do
    case "$f" in */lib/rapidr) continue ;; esac
    rm -rf "$f"
done
# (its own folder last: this script is in it)
for f in $FILES; do
    case "$f" in */lib/rapidr) rm -rf "$f" ;; esac
done
if [ -f "$CONFIG/mimeapps.list" ]; then
    sed -i.bak -e '/rapidr-runtime\.desktop/d' -e '/rapidr-ide\.desktop/d' "$CONFIG/mimeapps.list" && rm -f "$CONFIG/mimeapps.list.bak"
fi
command -v update-mime-database >/dev/null && update-mime-database "$DATA/mime" || true
command -v update-desktop-database >/dev/null && update-desktop-database "$DATA/applications" || true
[ "${1:-}" = --quiet ] || echo "RapidR removed"
