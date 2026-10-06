#!/bin/bash
# Installs a release artifact into a temporary prefix and checks it works as
# a user's install would — never the real user's: files, file types and
# settings go to temporary folders, and nothing is registered with the
# system (macOS: the apps' Info.plist is checked, Launch Services untouched).
#
#   tools/release/smoke.sh RapidR-<ver>-macos-<arch>.dmg          (macOS; both apps)
#   tools/release/smoke.sh RapidR-Runtime-<ver>-macos-<arch>.dmg
#   tools/release/smoke.sh rapidr[-runtime]-<ver>-linux-<arch>.tar.gz   (Linux: install.sh, uninstall.sh)
#   tools/release/smoke.sh rapidr[-runtime]_<ver>_<arch>.deb      (Linux: unpacked, not dpkg -i)
#
# Checks: an interpreted program (run, a #! script, .rrbc, COMMAND$ and
# Application.ExeName), `rapidr info` and the bytecode header (a program for
# a newer runtime says which), a standalone interpreted executable built
# with no Rust reachable, a GUI program and the IDE headless (the GUI tests'
# hooks), a downloaded file asking once (answers kept per file), a console
# program opened from the desktop getting a terminal ($TERMINAL, a stand-in),
# the file-type registrations, a native build when Rust is here (offline,
# from the shipped sources, an empty cargo home), and what a runtime-only
# install refuses. SMOKE_NATIVE=0 skips the native build (minutes, ~2 GB: setup
# downloads RapidR's toolchain into a throwaway RUSTUP_HOME / CARGO_HOME).
set -uo pipefail
ART="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
[ -f "$ART" ] || { echo "usage: $0 <artifact>"; exit 2; }
T="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/rapidr-smoke.XXXXXX")" && pwd -P)"
MNT=""
cleanup() {
    [ -n "$MNT" ] && { hdiutil detach -quiet "$MNT" 2>/dev/null; rmdir "$MNT" 2>/dev/null; }
    rm -rf "$T"
}
trap cleanup EXIT
FAIL=0
ok() { echo "  ok    $*"; }
bad() { echo "  FAIL  $*"; FAIL=1; }
check() { local what="$1"; shift; if "$@"; then ok "$what"; else bad "$what"; fi; }
has() { grep -q -- "$2" <<<"$1"; }

# Nothing reaches the user's printer, registry store, settings or file types.
export RAPIDR_PRINT_TO="$T/prints" RAPIDR_REGISTRY="$T/registry.reg" RAPIDR_CONFIG_DIR="$T/config/rapidr"
export XDG_DATA_HOME="$T/data" XDG_CONFIG_HOME="$T/config"
mkdir -p "$RAPIDR_PRINT_TO" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME"
# Rust, as the user has it (for the native build), before PATH is narrowed.
CARGO_BIN="$(dirname "$(command -v cargo 2>/dev/null || echo "$HOME/.cargo/bin/cargo")")"
BASE_PATH="/usr/bin:/bin:/usr/sbin:/sbin"

echo "== install $(basename "$ART") into $T"
case "$ART" in
    *.dmg)
        # (a mount point on the system disk: hdiutil refuses one on some volumes)
        MNT="$(mktemp -d /tmp/rapidr-smoke-mnt.XXXXXX)"; mkdir -p "$T/Applications"
        hdiutil attach -quiet -nobrowse -readonly -mountpoint "$MNT" "$ART" || { echo "cannot mount"; exit 1; }
        cp -R "$MNT"/*.app "$T/Applications/"
        hdiutil detach -quiet "$MNT"; rmdir "$MNT"; MNT=""
        if [ -d "$T/Applications/RapidR.app" ]; then APP="$T/Applications/RapidR.app"; else APP="$T/Applications/RapidR Runtime.app"; fi
        BIN="$APP/Contents/MacOS"
        for app in "$T/Applications"/*.app; do
            check "$(basename "$app"): signature valid" codesign --verify --deep --strict "$app"
            check "$(basename "$app"): Info.plist valid" plutil -lint -s "$app/Contents/Info.plist"
            check "$(basename "$app"): executable is the launcher" test "$(plutil -extract CFBundleExecutable raw "$app/Contents/Info.plist")" = rapidrw
        done
        # (the document types: who opens .rrbc / .rr / .bas, with which rank)
        types() { python3 -c 'import plistlib,sys; p=plistlib.load(open(sys.argv[1],"rb"))
for d in p.get("CFBundleDocumentTypes",[]):
    for t in d["LSItemContentTypes"]: print(t.rsplit(".",1)[-1], d["CFBundleTypeRole"], d["LSHandlerRank"])
for k in ("UTExportedTypeDeclarations","UTImportedTypeDeclarations"):
    for u in p.get(k,[]): print(k[2:10].lower(), *u["UTTypeTagSpecification"]["public.filename-extension"])' "$1"; }
        RT="$T/Applications/RapidR Runtime.app/Contents/Info.plist"
        if [ -f "$RT" ]; then
            t="$(types "$RT")"
            check "Runtime.app owns .rrbc (runs it)" has "$t" "bytecode Viewer Owner"
            check "Runtime.app runs .rr/.bas (Open With)" has "$t" "rapidq-source Viewer Alternate"
            check "Runtime.app declares rrbc" has "$t" "exported rrbc"
        fi
        if [ -f "$T/Applications/RapidR.app/Contents/Info.plist" ]; then
            t="$(types "$T/Applications/RapidR.app/Contents/Info.plist")"
            check "RapidR.app edits .rr/.bas (Owner)" has "$t" "source Editor Owner"
            check "RapidR.app declares rr and bas" has "$t" "exported bas"
        fi
        ;;
    *.tar.gz)
        mkdir -p "$T/x" && tar -xzf "$ART" -C "$T/x"
        top="$(ls -d "$T"/x/*/)"
        PREFIX="$T/prefix" sh "$top/install.sh" > "$T/install.log" 2>&1 || { cat "$T/install.log"; bad "install.sh"; }
        BIN="$T/prefix/bin"
        check "installed: bin/rapidr, lib/rapidr" test -x "$BIN/rapidr" -a -f "$T/prefix/lib/rapidr/release.toml"
        check "LEGAL.md and the notices installed" test -f "$T/prefix/share/doc/rapidr/LEGAL.md" -a -f "$T/prefix/share/doc/rapidr/THIRD-PARTY-NOTICES.txt"
        check "MIME types registered" test -f "$XDG_DATA_HOME/mime/packages/rapidr.xml"
        check "rapidr-runtime.desktop runs with the installed rapidr" grep -q "^Exec=$T/prefix/bin/rapidr open %f" "$XDG_DATA_HOME/applications/rapidr-runtime.desktop"
        if command -v update-mime-database >/dev/null; then
            check "MIME database knows *.rrbc" grep -q "rrbc" "$XDG_DATA_HOME/mime/globs2"
        fi
        if command -v xdg-mime >/dev/null; then
            check ".rrbc opens with rapidr-runtime.desktop" test "$(xdg-mime query default application/x-rapidr-bytecode)" = rapidr-runtime.desktop
        fi
        ;;
    *.deb)
        command -v dpkg-deb >/dev/null || { echo "dpkg-deb is needed"; exit 1; }
        dpkg-deb -x "$ART" "$T/root"
        info="$(dpkg-deb -f "$ART")"
        check ".deb: package, version, depends" has "$info" "^Depends: .*libc6"
        check ".deb: MIME types and desktop files" test -f "$T/root/usr/share/mime/packages/rapidr.xml" -a -f "$T/root/usr/share/applications/rapidr-runtime.desktop"
        check ".deb: desktop file runs /usr/bin/rapidr" grep -q "^Exec=/usr/bin/rapidr open %f" "$T/root/usr/share/applications/rapidr-runtime.desktop"
        BIN="$T/root/usr/bin"
        ;;
    *) echo "unknown artifact"; exit 2 ;;
esac
R="$BIN/rapidr"
KIND="$("$R" setup --check 2>/dev/null | sed -nE 's/^home: .*\((sdk|runtime) install .*/\1/p')"
echo "== $KIND: $("$R" version)"
check "rapidr finds its installed home" test -n "$KIND"
W="$T/work"; mkdir -p "$W"; cd "$W"

echo "== interpreted programs"
printf '$APPTYPE CONSOLE\nPRINT "hello "; COMMAND$\nPRINT Application.ExeName\n' > hello.bas
out="$(PATH="$BASE_PATH" "$R" run hello.bas a b 2>&1)"
check "rapidr run hello.bas a b" has "$out" "hello a b"
check "Application.ExeName is the program" has "$out" "hello.bas"
printf '#!/usr/bin/env rapidr\nPRINT "script "; COMMAND$\n' > script.rr && chmod +x script.rr
check "#! script" has "$(PATH="$BIN:$BASE_PATH" ./script.rr x 2>&1)" "script x"
"$R" build-bc hello.bas -o hello.rrbc >/dev/null
check "rapidr hello.rrbc" has "$("$R" hello.rrbc z 2>&1)" "hello z"
check "rapidr info: console" has "$("$R" info hello.rrbc)" "apptype: console"
# (macOS: every executable universal, checked statically — nothing x86_64 is run here:
# macOS 28 drops Rosetta, and running Intel code flags the app)
universal() { local a=" $(lipo -archs "$1" 2>/dev/null) "; [[ "$a" == *" arm64 "* && "$a" == *" x86_64 "* ]]; }
if [[ "$ART" == *.dmg ]]; then
    notuni=""
    while IFS= read -r -d '' f; do
        file -b "$f" | grep -q Mach-O && ! universal "$f" && notuni="$notuni $f"
    done < <(find "$T/Applications" -type f -perm -u+x -print0)
    check "every executable in the apps is universal (arm64 + x86_64)" test -z "$notuni"
    [ -n "$notuni" ] && echo "        not universal:$notuni"
fi
RTR="$T/Applications/RapidR Runtime.app/Contents/MacOS/rapidr"
if [ -x "$RTR" ] && [ "$RTR" != "$R" ]; then
    check "RapidR Runtime.app runs it too" has "$("$RTR" run hello.rrbc y 2>&1)" "hello y"
fi
python3 - hello.rrbc newer.rrbc <<'EOF'
import sys
b = bytearray(open(sys.argv[1], "rb").read())
b[10:12] = (99).to_bytes(2, "little")      # min_runtime major = 99
open(sys.argv[2], "wb").write(b)
EOF
check "a program for a newer runtime says which" has "$("$R" run newer.rrbc 2>&1)" "needs RapidR Runtime 99"

echo "== a GUI program, headless"
printf 'CREATE Form AS QFORM\n  Caption = "smoke"\n  CREATE Lbl AS QLABEL\n    Caption = "ready"\n  END CREATE\nEND CREATE\nForm.ShowModal\n' > gui.bas
check "rapidr info: gui" has "$("$R" info gui.bas)" "apptype: gui"
check "GUI program runs (capture)" has "$(RAPIDR_CAPTURE="$W/cap" RAPIDR_CAPTURE_DELAY=0.3 RAPIDR_TEST_DUMP=lbl.caption "$R" run gui.bas 2>&1)" "lbl.caption=ready"

echo "== a downloaded file asks once"
cp hello.bas downloaded.bas
case "$(uname)" in
    Darwin) xattr -w com.apple.quarantine "0081;00000000;Safari;" downloaded.bas ;;
    *) python3 -c 'import os,sys; os.setxattr(sys.argv[1], "user.xdg.origin.url", b"https://example.com/downloaded.bas")' downloaded.bas ;;
esac
check "rapidr info: downloaded" has "$("$R" info downloaded.bas)" "downloaded: yes"
hook() { RAPIDR_CAPTURE="$W/cap" RAPIDR_CAPTURE_DELAY=0.2 RAPIDR_TEST_MESSAGE_DIALOG="$1" "$R" run downloaded.bas "$2" < /dev/null 2>&1; }
check "asked, answered Yes: runs" has "$(hook Yes first)" "hello first"
check "the answer is kept (per file hash)" grep -q " run $W/downloaded.bas" "$RAPIDR_CONFIG_DIR/trusted-files.txt"
check "not asked again (a No now changes nothing)" has "$(hook No second)" "hello second"

echo "== a console program opened from the desktop gets a terminal"
printf '#!/bin/sh\nprintf "%%s\\n" "$@" > "%s/terminal.args"\n' "$W" > fake-terminal && chmod +x fake-terminal
TERMINAL="$W/fake-terminal" "$R" open hello.rrbc < /dev/null > /dev/null 2>&1
for _ in 1 2 3 4 5 6 7 8 9 10; do [ -s "$W/terminal.args" ] && break; sleep 0.5; done
check "the terminal runs rapidr run <program>" grep -q "run '$W/hello.rrbc'" "$W/terminal.args"

if [ "$KIND" = sdk ]; then
    echo "== a standalone interpreted executable, no Rust reachable"
    out="$(PATH="$BASE_PATH" CARGO_HOME="$T/no-cargo" "$R" build hello.bas --interp 2>&1)"
    check "rapidr build --interp" test -x "$W/hello"
    check "the executable runs" has "$("$W/hello" q 2>&1)" "hello q"
    if [[ "$ART" == *.dmg ]]; then
        check "it is universal" universal "$W/hello"
        mkdir -p "$W/arm" "$W/intel"
        "$R" build hello.bas "$W/arm" --interp --target macos-arm64 > /dev/null 2>&1
        "$R" build hello.bas "$W/intel" --interp --target macos-x86_64 > /dev/null 2>&1
        check "--target macos-arm64: the arm64 slice, it runs" has "$(lipo -archs "$W/arm/hello"; "$W/arm/hello" a 2>&1)" "arm64"
        check "--target macos-x86_64: the x86_64 slice (not run)" test "$(lipo -archs "$W/intel/hello" 2>/dev/null)" = x86_64
    fi
    check "its THIRD-PARTY-NOTICES.txt beside it (the install's)" grep -q "Rust standard library" "$W/THIRD-PARTY-NOTICES.txt"
    check "the IDE starts (headless)" has "$(RAPIDR_CAPTURE="$W/ide" RAPIDR_CAPTURE_DELAY=0.5 RAPIDR_TEST_DUMP=statusbar.caption "$R" ide 2>&1)" "statusbar.caption=Ready"
    check "the IDE opens a file" has "$(RAPIDR_CAPTURE="$W/ide" RAPIDR_CAPTURE_DELAY=0.5 RAPIDR_TEST_DUMP=statusbar.caption "$R" ide "$W/hello.bas" 2>&1)" "Opened: $W/hello.bas"
    if [ "${SMOKE_NATIVE:-1}" = 1 ] && [ -x "$CARGO_BIN/cargo" ]; then
        echo "== rapidr setup and a native build: a throwaway rustup and cargo home, offline, the shipped sources"
        # (the user's own Rust is never touched: its default, read before and after)
        users_rust() { env -u RUSTUP_HOME -u CARGO_HOME -u RUSTUP_TOOLCHAIN "$CARGO_BIN/rustup" default 2>&1; env -u RUSTUP_HOME -u CARGO_HOME -u RUSTUP_TOOLCHAIN "$CARGO_BIN/rustup" show active-toolchain 2>&1; }
        users_before="$(users_rust)"
        export RUSTUP_HOME="$T/rustup-home" CARGO_HOME="$T/cargo-home"
        mkdir -p "$RUSTUP_HOME" "$CARGO_HOME"
        case "$(uname)" in Darwin) host=aarch64-apple-darwin ;; *) host="$(uname -m)-unknown-linux-gnu" ;; esac
        "$CARGO_BIN/rustup" set default-host "$host" > /dev/null 2>&1
        rust_state() { "$CARGO_BIN/rustup" default 2>&1; "$CARGO_BIN/rustup" show 2>&1 | grep -i "default host"; }
        before="$(rust_state)"
        PATH="$CARGO_BIN:$BASE_PATH" "$R" setup --yes --no-path > setup.log 2>&1
        check "setup installs RapidR's toolchain beside, the defaults unchanged" test "$(rust_state)" = "$before"
        check "setup: native builds ready" grep -q "native builds ready" setup.log || tail -5 setup.log
        printf '$APPTYPE CONSOLE\nPRINT "native "; 6 * 7\n' > native.bas
        PATH="$CARGO_BIN:$BASE_PATH" CARGO_TARGET_DIR="$T/native-target" "$R" build native.bas > native.log 2>&1
        check "rapidr build (native)" has "$(./native 2>&1)" "native 42" || tail -5 native.log
        if [[ "$ART" == *.dmg ]]; then
            if grep -q "this Mac's architecture only" native.log; then
                echo "  (native build: one architecture — Rust has one macOS target here)"
            else
                check "the native build is universal" universal ./native
            fi
        fi
        rm -rf "$T/native-target"
        unset RUSTUP_HOME CARGO_HOME
        check "the user's own Rust unchanged (rustup default, active toolchain)" test "$(users_rust)" = "$users_before"
    else
        echo "  (no native build: Rust not found, or SMOKE_NATIVE=0)"
    fi
else
    echo "== what the runtime alone doesn't do"
    check "no executables: says it's the runtime" has "$("$R" build hello.bas --interp 2>&1)" "This is the RapidR Runtime"
    check "no IDE: says so" has "$("$R" ide 2>&1)" "not found"
fi

if [[ "$ART" == *.tar.gz ]]; then
    echo "== uninstall"
    sh "$T/prefix/lib/rapidr/uninstall.sh" > /dev/null 2>&1
    check "files removed" test ! -e "$T/prefix/bin/rapidr" -a ! -e "$T/prefix/lib/rapidr"
    check "file types removed" test ! -e "$XDG_DATA_HOME/mime/packages/rapidr.xml" -a ! -e "$XDG_DATA_HOME/applications/rapidr-runtime.desktop"
    if [ -f "$XDG_CONFIG_HOME/mimeapps.list" ]; then
        check "defaults removed" sh -c "! grep -q rapidr '$XDG_CONFIG_HOME/mimeapps.list'"
    fi
fi
echo "== $( [ $FAIL = 0 ] && echo "smoke test passed" || echo "smoke test FAILED" ): $(basename "$ART")"
exit $FAIL
