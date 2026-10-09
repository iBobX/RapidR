#!/bin/sh
# Installs this RapidR for you alone — no root: the files into ~/.local
# (bin/rapidr, lib/rapidr, share/doc/rapidr), RapidR's file types and their
# applications into ~/.local/share (mime/, applications/), and makes them the
# defaults: a .rrbc runs on a double click; a .rr / .bas / .rrproj opens in
# RapidR Studio (with the SDK; "Open With > RapidR Runtime" runs a source file).
#
#   ./install.sh                 PREFIX=~/.local, XDG_DATA_HOME, XDG_CONFIG_HOME are honoured
#   ~/.local/lib/rapidr/uninstall.sh      removes all of it
#
# (The folder also runs as it is, without installing: bin/rapidr.)
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
KIND="$(sed -n 's/^kind = "\(.*\)"/\1/p' "$HERE/lib/rapidr/release.toml")"

if [ -e "$PREFIX/lib/rapidr/uninstall.sh" ]; then
    echo "removing the RapidR installed before"
    sh "$PREFIX/lib/rapidr/uninstall.sh" --quiet
fi
mkdir -p "$PREFIX/bin" "$PREFIX/lib" "$PREFIX/share/doc" "$DATA/mime/packages" "$DATA/applications"
cp "$HERE/bin/rapidr" "$PREFIX/bin/rapidr"
cp -R "$HERE/lib/rapidr" "$PREFIX/lib/rapidr"
cp -R "$HERE/share/doc/rapidr" "$PREFIX/share/doc/rapidr"
cp "$HERE/uninstall.sh" "$PREFIX/lib/rapidr/uninstall.sh"

cp "$HERE/share/mime/packages/rapidr.xml" "$DATA/mime/packages/rapidr.xml"
ICONS="$(cd "$HERE/share/icons" && find hicolor -type f)"
for i in $ICONS; do mkdir -p "$DATA/icons/$(dirname "$i")"; cp "$HERE/share/icons/$i" "$DATA/icons/$i"; done
DESKTOP="rapidr-runtime.desktop"
[ "$KIND" = sdk ] && DESKTOP="$DESKTOP rapidr-ide.desktop"
for d in $DESKTOP; do
    sed "s|@BIN@|$PREFIX/bin|g" "$HERE/share/applications/$d" > "$DATA/applications/$d"
done
# (what uninstall.sh removes)
cat > "$PREFIX/lib/rapidr/installed.txt" <<EOF
$PREFIX/bin/rapidr
$PREFIX/lib/rapidr
$PREFIX/share/doc/rapidr
$DATA/mime/packages/rapidr.xml
$(for d in $DESKTOP; do echo "$DATA/applications/$d"; done)
$(for i in $ICONS; do echo "$DATA/icons/$i"; done)
EOF

command -v update-mime-database >/dev/null && update-mime-database "$DATA/mime" || true
command -v update-desktop-database >/dev/null && update-desktop-database "$DATA/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$DATA/icons/hicolor" || true
if command -v xdg-mime >/dev/null; then
    xdg-mime default rapidr-runtime.desktop application/x-rapidr-bytecode
    if [ "$KIND" = sdk ]; then
        xdg-mime default rapidr-ide.desktop text/x-rapidr text/x-rapidq-basic application/x-rapidr-project
    else
        xdg-mime default rapidr-runtime.desktop text/x-rapidr text/x-rapidq-basic
    fi
fi

echo "RapidR installed in $PREFIX"
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "add $PREFIX/bin to PATH:  echo 'export PATH=\"$PREFIX/bin:\$PATH\"' >> ~/.bashrc" ;;
esac
[ "$KIND" = sdk ] && echo "native builds need Rust: rapidr setup"
exit 0
