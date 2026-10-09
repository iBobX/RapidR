#!/bin/bash
# The last step, on the Mac, once every platform's files are in
# dist/<ver>/out/: each package carries the licence files, then SHA256SUMS.
#
#   tools/release/finish.sh
#
# Signing the checksums with minisign is OFF unless you give it a key (this script never
# creates one): RAPIDR_MINISIGN_KEY=<path of your minisign secret key> (made once with
# `minisign -G`; it asks for the key's password) writes SHA256SUMS.minisig beside SHA256SUMS.
# People then check a download with `minisign -Vm SHA256SUMS -P <your public key>`.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$OUT"
LICENCES="LICENSE NOTICE LEGAL.md LICENSES.md THIRD_PARTY_NOTICES.md"
fail=0
listing() {
    case "$1" in
        *.zip) unzip -Z1 "$1" ;;
        *.tar.gz) tar -tzf "$1" ;;
        *.dmg)
            local m; m="$(mktemp -d)"
            hdiutil attach -quiet -nobrowse -readonly -mountpoint "$m" "$1" && (cd "$m" && find . -type f) ; hdiutil detach -quiet "$m"; rmdir "$m" ;;
        *.deb) ar p "$1" data.tar.zst 2>/dev/null | zstd -dc 2>/dev/null | tar -t 2>/dev/null || ar p "$1" data.tar.xz | tar -tJ ;;
        *) return 1 ;;
    esac
}
step "licence files in every package"
for f in *.zip *.tar.gz *.dmg *.deb; do
    [ -e "$f" ] || continue
    files="$(listing "$f" || true)"
    for l in $LICENCES; do
        if ! grep -qE "(^|/)${l//./\\.}\$" <<<"$files"; then
            echo "  $f: no $l"; fail=1
        fi
    done
done
ls *-setup.exe >/dev/null 2>&1 && echo "  (the Windows installers: their staged prefix has share\\doc\\rapidr\\, stage.py)"
[ $fail = 0 ] || die "a package lacks licence files"

step "SHA256SUMS"
rm -f SHA256SUMS
shasum -a 256 $(ls | grep -v -e '^SHA256SUMS$' -e '^RELEASE_NOTES.md$') > SHA256SUMS
cat SHA256SUMS
du -sh "$OUT"
if [ -n "${RAPIDR_MINISIGN_KEY:-}" ]; then
    step "SHA256SUMS.minisig (minisign, $RAPIDR_MINISIGN_KEY)"
    need minisign "brew install minisign"
    rm -f SHA256SUMS.minisig
    minisign -S -s "$RAPIDR_MINISIGN_KEY" -m SHA256SUMS -t "RapidR $VERSION"
    ls -l SHA256SUMS.minisig
else
    echo "(SHA256SUMS.minisig: not made — RAPIDR_MINISIGN_KEY isn't set)"
fi
