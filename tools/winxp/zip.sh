#!/bin/bash
# The Windows XP TEST zip (unofficial; lane WINXP): a Windows 7 alpha zip with its three programs
# (bin\rapidr.exe and the two runner stubs) replaced by the plain-XP flavour's (tools/winxp/build.sh
# with XP_HEADER=5.1), stripped as the alpha's are. Everything else (Studio's files, examples,
# notices, licences, RapidR-Studio.bat) is the Windows 7 zip's, unchanged.
#
#   tools/winxp/zip.sh <win7 zip> [out folder]     -> RapidR-<ver>-winxp-x86-test.zip (+ .sha256)
#
# Default out folder: $CARGO_TARGET_DIR/winxp-zip. The XP programs come from $CARGO_TARGET_DIR/winxp-5.1.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
LLVM_MINGW="${LLVM_MINGW:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt}"
TD="${CARGO_TARGET_DIR:-$ROOT/target}"
W7="${1:?usage: tools/winxp/zip.sh <win7 alpha zip> [out folder]}"
OUTDIR="${2:-$TD/winxp-zip}"
XP="$TD/winxp-5.1"
for f in rapidr rapidrintr-runner rapidrintr-runnerw; do
    [ -f "$XP/$f.exe" ] || { echo "build first: XP_HEADER=5.1 tools/winxp/build.sh (missing $XP/$f.exe)" >&2; exit 1; }
done
VERSION="$(basename "$W7" | sed -n 's/^RapidR-\([0-9.]*\)-win7-.*/\1/p')"
[ -n "$VERSION" ] || { echo "$W7: not a RapidR-<ver>-win7-... zip" >&2; exit 1; }
NAME="RapidR-$VERSION-winxp-x86-test"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/winxp-zip.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUTDIR"
(cd "$WORK" && unzip -q "$W7")
TOP="$(ls "$WORK")"
[ "$(echo "$TOP" | wc -l | tr -d ' ')" = 1 ] || { echo "$W7: expected one top folder" >&2; exit 1; }
mv "$WORK/$TOP" "$WORK/$NAME"
D="$WORK/$NAME"
cp "$XP/rapidr.exe" "$D/bin/rapidr.exe"
for r in rapidrintr-runner rapidrintr-runnerw; do cp "$XP/$r.exe" "$D/lib/rapidr/runners/windows-x86/$r.exe"; done
"$LLVM_MINGW/bin/llvm-strip" "$D/bin/rapidr.exe" "$D/lib/rapidr/runners/windows-x86/"*.exe
# (every program in it must start on plain XP: header 5.1 and nothing XP lacks in its import table)
for f in "$D/bin/rapidr.exe" "$D/lib/rapidr/runners/windows-x86/"*.exe; do
    v="$("$LLVM_MINGW/bin/llvm-readobj" --file-headers "$f" | awk '/MajorSubsystemVersion/{a=$2} /MinorSubsystemVersion/{b=$2} END{print a "." b}')"
    [ "$v" = 5.1 ] || { echo "$f: subsystem $v, not 5.1" >&2; exit 1; }
    "$HERE/imports.sh" "$f" > /dev/null || { echo "$f: imports something plain XP lacks" >&2; "$HERE/imports.sh" "$f" | tail -5 >&2; exit 1; }
done
printf '%s\r\n' \
    "RapidR $VERSION for Windows XP - TEST BUILD (unofficial)." \
    "" \
    "Windows XP is not supported. This build runs RapidR on Windows XP SP3 (32-bit) for testing:" \
    "RapidR's own stand-ins answer for what XP lacks (docs/winxp-plan.md in RapidR's source)." \
    "It needs a CPU with SSE2 (Pentium 4, Athlon 64 or newer). No DirectWrite, no UI Automation" \
    "(no screen reader), no Vista file dialogs, no modern TLS on plain XP. Start: RapidR-Studio.bat." \
    > "$D/README-WINXP-TEST.txt"
rm -f "$OUTDIR/$NAME.zip"
(cd "$WORK" && zip -qr -9 "$OUTDIR/$NAME.zip" "$NAME")
(cd "$OUTDIR" && shasum -a 256 "$NAME.zip" > "$NAME.zip.sha256")
ls -l "$OUTDIR/$NAME.zip"
