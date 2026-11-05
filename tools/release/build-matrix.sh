#!/bin/bash
# "Everybody builds": the one command that shows, before a release is published, whether a person
# can press Build (interpreted) and Build Native with the PACKAGED files on every system RapidR
# ships — and get a program that runs. It installs each artifact of dist/<ver>/out/ the way a user
# does (the existing smoke tests: macOS here, Linux and Windows in their VMs, the Windows 7 alpha
# unzipped in the Windows VM), builds and runs, and prints ONE table:
#
#   system | package | run | Build (interp) | Build Native | built exe runs | other | time
#
# with the first error line of every red cell under it. The table is also written to
# dist/<ver>/build-matrix.txt (the logs and pictures are in dist/<ver>/build-matrix/). Exit status 0
# only when every cell that can be checked here is green: wire it in front of "publish"
# (tools/release/README.md, docs/build-matrix.md says how to read it).
#
#   tools/release/build-matrix.sh <ver> [--quick] [--only mac,linux,windows,win7,win7real] [--shots] [--serial] [--table-only]
#
#   --quick     no native builds (the interpreted side only, minutes instead of an hour)
#   --only      some lanes: mac (the two dmgs, here), linux (the Ubuntu VM), windows (the Windows 11 VM:
#               x64 and arm64 installers), win7 (the alpha zip, in the Windows 11 VM: emulation), win7real (the
#               alpha zip on a REAL Windows 7, the UTM VM "Windows 7 x86": tools/release/win7-vm.sh)
#   --shots     also keep the pictures the test programs captured (Windows lanes), as PNGs under
#               dist/<ver>/build-matrix/shots/
#   --serial    one lane after the other (default: the three machines work at the same time)
#   --table-only  run nothing: print the table again from the results the last runs left
#
# What it needs: the artifacts and SHA256SUMS in dist/<ver>/out/ (from a worktree of the repository
# the main checkout's dist/ is used), both Parallels VMs running (it resumes a paused one; it never
# starts or stops one), tools/release/linux/setup-tools.sh and windows/setup-tools.ps1 done in them,
# the test-mode token in each VM user's .rapidr folder (README "The test-mode token"), and for the
# Windows native builds the release's source in the VM (…\rapidr-release\src: the conformance cases).
# The Mac's scratch space is MATRIX_TMP (default /Volumes/RapidRBuild/build-matrix-tmp when that
# volume is mounted): a native build leaves ~2 GB there while it runs, removed at the end.
#
# Nothing here pushes, uploads or signs. Every program run has its own registry, printer folder and
# settings (the smoke scripts), and RAPIDR_AI_KEYSTORE=memory.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
die() { echo "error: $*" >&2; exit 1; }
step() { echo "== $*"; }

ALL_LANES="mac,linux,windows,win7,win7real"
VER=""; QUICK=0; ONLY="$ALL_LANES"; SHOTS=0; SERIAL=0; TABLE_ONLY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --quick) QUICK=1 ;;
        --only) shift; ONLY="${1:-}" ;;
        --shots) SHOTS=1 ;;
        --serial) SERIAL=1 ;;
        --table-only) TABLE_ONLY=1 ;;
        -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
        -*) die "unknown option $1 (see the top of $0)" ;;
        *) [ -z "$VER" ] && VER="$1" || die "one version only" ;;
    esac
    shift
done
[ -n "$VER" ] || die "usage: $0 <ver> [--quick] [--only mac,linux,windows,win7,win7real] [--shots] [--serial] [--table-only]"

# Where the release's files are: dist/<ver> of this checkout, else of the main checkout (a worktree
# has none: dist/ is not in git).
DIST="${RAPIDR_DIST:-$ROOT/dist/$VER}"
if [ ! -f "$DIST/out/SHA256SUMS" ]; then
    main="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"; main="${main%/.git}"
    [ -n "$main" ] && [ -f "$main/dist/$VER/out/SHA256SUMS" ] && DIST="$main/dist/$VER"
fi
OUT="$DIST/out"
[ -f "$OUT/SHA256SUMS" ] || die "no $OUT/SHA256SUMS (a finished release: tools/release/finish.sh)"
REPORT="$DIST/build-matrix"; LOGS="$REPORT/logs"; RES="$REPORT/.results"
mkdir "$REPORT.lock" 2>/dev/null || die "another build-matrix run is going ($REPORT.lock exists; remove it if none is)"
mkdir -p "$LOGS" "$REPORT/shots" "$RES"
# (only the lanes asked for are made again: the logs of the others stay as the last run left them)
[ $TABLE_ONLY = 1 ] || for lane in $(echo "$ONLY" | tr ',' ' '); do rm -f "$RES/$lane.tsv" "$LOGS/$lane-"*; done
LANE_PIDS=""
cleanup() { rmdir "$REPORT.lock" 2>/dev/null; }
trap cleanup EXIT
trap 'echo "(interrupted: the lanes still running are stopped)"; for p in $LANE_PIDS; do kill "$p" 2>/dev/null; done; exit 130' INT TERM HUP

# Nothing a program here runs reaches the user's keychain, printer or settings.
export RAPIDR_AI_KEYSTORE=memory
# (Preview 5's headless Studio checks look at a window after P5_DELAY seconds; this Mac is busy with the VMs)
export P5_DELAY="${P5_DELAY:-10}"
if [ -z "${MATRIX_TMP:-}" ] && [ -d /Volumes/RapidRBuild ]; then MATRIX_TMP=/Volumes/RapidRBuild/build-matrix-tmp; fi
if [ -n "${MATRIX_TMP:-}" ]; then mkdir -p "$MATRIX_TMP"; export TMPDIR="$MATRIX_TMP"; fi

want() { case ",$ONLY," in *",$1,"*) return 0 ;; esac; return 1; }
sum_of() { sed -n "s/^\([0-9a-f]\{64\}\)  $1\$/\1/p" "$OUT/SHA256SUMS" | head -1; }
now() { date +%s; }

# One result per row, one line in $RES/<lane>.tsv:
#   system <TAB> package <TAB> kind (sdk|runtime|zip) <TAB> native (yes|no:<why>) <TAB> seconds <TAB> log file <TAB> note
#   (an empty field is written as "-": `read` with a tab for IFS would swallow it and shift the rest)
record() { printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" "${6:--}" "${7:--}" >> "$RES/$8.tsv"; }
win_arch() { case "$1" in *arm64*) echo "ARM64" ;; *) echo "x64 (emulated)" ;; esac; }
linux_arch() { case "$1" in *aarch64*|*arm64*) echo "aarch64" ;; *) echo "x86_64 (Rosetta)" ;; esac; }
kind_of() { case "$1" in *Runtime*|*runtime*) echo runtime ;; *.zip) echo zip ;; *) echo sdk ;; esac; }

step "build matrix for $VER ($([ $QUICK = 1 ] && echo "quick, no native builds" || echo "with native builds")), lanes: $ONLY"
echo "   artifacts: $OUT"
echo "   report:    $REPORT"

# --- the files are the release's: every artifact against SHA256SUMS ------------------------------
bad=0; n=0
if [ $TABLE_ONLY = 0 ]; then
while read -r sum name; do
    [ -f "$OUT/$name" ] || continue
    n=$((n + 1))
    [ "$(shasum -a 256 "$OUT/$name" | cut -d' ' -f1)" = "$sum" ] || { echo "  CHECKSUM DIFFERS: $name"; bad=1; }
done < "$OUT/SHA256SUMS"
[ $bad = 0 ] || die "an artifact is not the one SHA256SUMS lists"
echo "   $n artifacts match SHA256SUMS"
fi

# --- the VMs --------------------------------------------------------------------------------
vm_awake() {
    local vm="$1" st
    st="$(prlctl list -a -o status,name | awk -v vm="$vm" '{s=$1; $1=""; sub(/^ /,""); if ($0 == vm) print s}')"
    case "$st" in
        running) ;;
        paused|suspended) prlctl resume "$vm" >/dev/null ;;
        *) die "the VM \"$vm\" is '${st:-not found}': ask Robert to start it (this script never starts or stops a VM)" ;;
    esac
}

# A lane that cannot start (its VM is not running) says so in the table: "not run" is red, never blank.
NOT_RUN=""
vm_ready() {   # <vm>: running (or resumed) -> 0; else NOT_RUN says why, 1
    local vm="$1" st
    st="$(prlctl list -a -o status,name | awk -v vm="$vm" '{s=$1; $1=""; sub(/^ /,""); if ($0 == vm) print s}')"
    case "$st" in
        running) return 0 ;;
        paused|suspended) prlctl resume "$vm" >/dev/null; return 0 ;;
    esac
    NOT_RUN="the VM \"$vm\" is ${st:-not found} (this script never starts or stops a VM)"
    return 1
}

# --- lane mac: the dmgs, here ---------------------------------------------------------------
lane_mac() {
    local f name t0 secs log native
    for f in "RapidR-$VER-macos-universal.dmg" "RapidR-Runtime-$VER-macos-universal.dmg"; do
        name="$f"; log="$LOGS/mac-$name.log"
        if [ ! -f "$OUT/$name" ]; then record macOS "$name" "$(kind_of "$name")" "yes" 0 "" "the file is not in $OUT" mac; continue; fi
        native=$([ $QUICK = 1 ] && echo 0 || echo 1)
        echo "[mac] $name ..."
        t0=$(now)
        SMOKE_NATIVE=$native bash "$HERE/smoke.sh" "$OUT/$name" > "$log" 2>&1
        secs=$(( $(now) - t0 ))
        record "macOS (this Mac, $(uname -m))" "$name" "$(kind_of "$name")" "yes" "$secs" "$log" "" mac
        echo "[mac] $name done in ${secs}s"
    done
}

# --- lane linux: the Ubuntu VM ----------------------------------------------------------------
lane_linux() {
    local vm="${RAPIDR_LINUX_VM:-Ubuntu 24.04.3 ARM64}" ubu="$HERE/ubuntu-vm.sh" names f script list="" t0 secs
    if ! vm_ready "$vm"; then
        echo "[linux] not run: $NOT_RUN"
        record "Linux (aarch64 and x86_64)" "the Linux packages (.tar.gz and .deb, SDK and Runtime)" sdk yes 0 "" "not run: $NOT_RUN" linux
        return
    fi
    names="$(sed -n -E 's/^[0-9a-f]{64}  (rapidr(-runtime)?-[0-9.]+-linux-[a-z0-9_]+\.tar\.gz|rapidr(-runtime)?_[0-9.]+_[a-z0-9]+\.deb)$/\1/p' "$OUT/SHA256SUMS")"
    [ -n "$names" ] || { record Linux "(no Linux packages in SHA256SUMS)" sdk yes 0 "" "" linux; return; }
    script="$(mktemp "${TMPDIR:-/tmp}/matrix-linux.XXXXXX")"
    # Which of them the VM has already (the release build leaves them in ~/rapidr-release/dist/out), by checksum.
    printf 'mkdir -p "$HOME/rapidr-release/dist/out"\n' > "$script"
    for f in $names; do printf 'printf "%%s  %%s\\n" "$(sha256sum "$HOME/rapidr-release/dist/out/%s" 2>/dev/null | cut -d" " -f1)" "%s"\n' "$f" "$f" >> "$script"; done
    local have; have="$(bash "$ubu" run "$script" 2>/dev/null)"
    if [ -z "$have" ]; then
        echo "[linux] not run: the VM did not answer (ubuntu-vm.sh run)"
        record "Linux (aarch64 and x86_64)" "the Linux packages (.tar.gz and .deb, SDK and Runtime)" sdk yes 0 "" "not run: the VM \"$vm\" did not answer (tools/release/ubuntu-vm.sh run)" linux
        rm -f "$script"; return
    fi
    # (files go straight into the user's home: ubuntu-vm.sh also works while nobody is logged in at the VM's
    # desktop, when it sends as root, and /tmp files root makes could not be moved by the user)
    for f in $names; do
        if ! grep -q "^$(sum_of "$f")  $f\$" <<<"$have"; then
            echo "[linux] $f is not in the VM (or differs): sending it (slow, base64 over prlctl)"
            bash "$ubu" send "$OUT/$f" "~/rapidr-release/dist/out/$f"
        fi
    done
    bash "$ubu" send "$HERE/smoke.sh" "~/rapidr-release/smoke.sh"
    bash "$ubu" send "$HERE/p5check.sh" "~/rapidr-release/p5check.sh"
    for f in $names; do list="$list $f"; done
    # One detached job in the VM; each package has its log, exit code and seconds in ~/rapidr-release/matrix/.
    # (x86_64 packages run by Rosetta here: there is no x86_64 Rust toolchain host, so no native build for them.)
    cat > "$script" <<EOF
B="\$HOME/rapidr-release"; M="\$B/matrix"
rm -rf "\$M"; mkdir -p "\$M"
setsid nohup bash -c '
  export PATH="\$HOME/.cargo/bin:\$PATH"; B="\$1"; M="\$2"; NATIVE="\$3"; shift 3
  export TMPDIR="\$B/tmp"; rm -rf "\$TMPDIR"; mkdir -p "\$TMPDIR"
  host="\$(uname -m)"
  for n in "\$@"; do
    case "\$n" in *"\$host"*|*_\$(dpkg --print-architecture).deb) nat="\$NATIVE" ;; *) nat=0 ;; esac
    s=\$(date +%s)
    SMOKE_NATIVE=\$nat RAPIDR_AI_KEYSTORE=memory P5_DELAY=$P5_DELAY bash "\$B/smoke.sh" "\$B/dist/out/\$n" > "\$M/\$n.log" 2>&1
    echo \$? > "\$M/\$n.rc"; echo \$(( \$(date +%s) - s )) > "\$M/\$n.secs"
  done
  rm -rf /tmp/rapidr-smoke.* "\$TMPDIR"
  echo done > "\$M/done"' _ "\$B" "\$M" "$([ $QUICK = 1 ] && echo 0 || echo 1)"$list > /dev/null 2>&1 &
echo started
EOF
    echo "[linux] smoke tests started in the VM (~/rapidr-release/matrix there)"
    t0=$(now)
    bash "$ubu" run "$script" > /dev/null
    printf 'cat "$HOME/rapidr-release/matrix/done" 2>/dev/null || true\n' > "$script"
    local code=""
    while :; do
        sleep 30
        code="$(bash "$ubu" run "$script" 2>/dev/null || true)"
        [ -n "$code" ] && break
        vm_awake "$vm"
    done
    for f in $names; do
        printf 'cat "$HOME/rapidr-release/matrix/%s.secs" 2>/dev/null\n' "$f" > "$script"
        # (a call Parallels drops answers nothing: asked again, so a time is never recorded as 0:00)
        secs=""
        for try in 1 2 3 4; do
            secs="$(bash "$ubu" run "$script" 2>/dev/null | tail -1)"
            case "$secs" in ""|*[!0-9]*) secs="" ;; *) break ;; esac
        done
        bash "$ubu" fetch "~/rapidr-release/matrix/$f.log" "$LOGS/linux-$f.log" || echo "(could not fetch the log of $f)" > "$LOGS/linux-$f.log"
        local why=yes
        case "$f" in *aarch64*|*arm64*) ;; *) why="no:x86_64 here runs by Rosetta; there is no x86_64 Rust host to build for it" ;; esac
        record "Linux $(linux_arch "$f")" "$f" "$(kind_of "$f")" "$why" "${secs:-0}" "$LOGS/linux-$f.log" "" linux
    done
    rm -f "$script"
    echo "[linux] done in $(( $(now) - t0 ))s"
}

# --- lane windows: the Windows 11 VM -------------------------------------------------------------
# $1 = the VM script command (smoke | smoke-win7), the rest its arguments
winvm() { "$HERE/windows-vm.sh" "$@"; }
vm_put() {   # <local file>: into the VM's out folder unless it has the same checksum already
    local f="$1" name; name="$(basename "$1")"
    PUT_SUM="$(sum_of "$name")" winvm put "$f" "$name" 2>&1 | tail -2
}
lane_windows() {
    local vm="${RAPIDR_WIN_VM:-Windows 11 Pro}" f t0 secs log args kind arch
    if ! vm_ready "$vm"; then
        echo "[windows] not run: $NOT_RUN"
        record "Windows (x64 and ARM64)" "the Windows installers (SDK and Runtime)" sdk yes 0 "" "not run: $NOT_RUN" windows
        return
    fi
    for f in "RapidR-$VER-windows-x64-setup.exe" "RapidR-$VER-windows-arm64-setup.exe" "RapidR-Runtime-$VER-windows-x64-setup.exe" "RapidR-Runtime-$VER-windows-arm64-setup.exe"; do
        log="$LOGS/windows-$f.log"; kind="$(kind_of "$f")"; arch="$(win_arch "$f")"
        if [ ! -f "$OUT/$f" ]; then record "Windows $arch" "$f" "$kind" yes 0 "" "the file is not in $OUT" windows; continue; fi
        echo "[windows] $f: into the VM ..."
        vm_put "$OUT/$f" > "$log" 2>&1
        args=(); [ $QUICK = 0 ] && [ "$kind" = sdk ] && args+=(Native)
        [ $SHOTS = 1 ] && args+=("Shots=matrix-shots")
        args+=("P5Delay=$P5_DELAY")
        echo "[windows] $f: smoke test ${args[*]:-}"
        t0=$(now)
        winvm smoke "$f" ${args[@]+"${args[@]}"} >> "$log" 2>&1
        secs=$(( $(now) - t0 ))
        record "Windows $arch" "$f" "$kind" yes "$secs" "$log" "" windows
        echo "[windows] $f done in ${secs}s"
    done
}

# --- lane win7: the alpha zip, in the Windows 11 VM ---------------------------------------------
# The VM is Windows 11 (x86 emulation): the package is used for real, but Windows 7 itself is not
# checked by it. So the import tables of the 32-bit programs are read on the Mac, and any function
# Windows 7 lacks is a red "other" cell: a program that Windows 7 refuses to start is not a program that runs.
win7_static() {   # <zip> <log>: appends the verdict to the log
    local zip="$1" log="$2" d bad=0 f out any=0
    d="$(mktemp -d "${TMPDIR:-/tmp}/matrix-win7.XXXXXX")"
    unzip -q -o "$zip" -d "$d" 2>/dev/null
    while IFS= read -r f; do
        any=1
        out="$(python3 -I "$ROOT/tools/win7/imports.py" "$f" 2>&1 | sed -n '/== static/,/== delay/p' | grep "NOT ON WINDOWS 7" || true)"
        if [ -n "$out" ]; then
            bad=1
            echo "  FAIL  Windows 7 would refuse to start $(basename "$f") (static import check): $(echo "$out" | sed -E 's/.*missing on Windows 7: //' | tr '\n' ' ')" >> "$log"
        else
            echo "  ok    $(basename "$f"): every function it needs at start is in Windows 7 (static import check)" >> "$log"
        fi
    done < <(find "$d" \( -name rapidr.exe -o -name 'rapidrintr-runner*.exe' \) -type f)
    [ $any = 1 ] || echo "  FAIL  no rapidr.exe in the zip" >> "$log"
    rm -rf "$d"
    [ $bad = 0 ] && [ $any = 1 ]
}
lane_win7() {
    local vm="${RAPIDR_WIN_VM:-Windows 11 Pro}" zip="${MATRIX_WIN7_ZIP:-}" log t0 secs name args static
    if [ -z "$zip" ]; then zip="$(ls "$OUT"/RapidR-"$VER"-win7-x86-alpha*.zip 2>/dev/null | tail -1)"; fi
    if [ -z "$zip" ] || [ ! -f "$zip" ]; then record "Windows 7 (x86, in the Win 11 VM)" "(no win7 zip in $OUT)" zip "no:" 0 "" "no Windows 7 package for this release" win7; return; fi
    name="$(basename "$zip")"; log="$LOGS/win7-$name.log"
    if ! vm_ready "$vm"; then
        echo "[win7] not run: $NOT_RUN"
        record "Windows 7 (x86 alpha; run in the Win 11 VM)" "$name" zip "no:Windows 7 has no Rust to build for it" 0 "" "not run: $NOT_RUN" win7
        return
    fi
    echo "[win7] $name: into the VM ..."
    # (a zip made elsewhere is copied in whatever its checksum; the release's own is compared with SHA256SUMS)
    if [ "$(dirname "$zip")" = "$OUT" ]; then PUT_SUM="$(sum_of "$name")" winvm put "$zip" "$name" > "$log" 2>&1; else winvm put "$zip" "$name" > "$log" 2>&1; fi
    args=(); [ $SHOTS = 1 ] && args+=("Shots=matrix-shots")
    echo "[win7] $name: smoke test"
    t0=$(now)
    winvm smoke-win7 "$name" ${args[@]+"${args[@]}"} >> "$log" 2>&1
    secs=$(( $(now) - t0 ))
    win7_static "$zip" "$log"; static=$?
    record "Windows 7 (x86 alpha; run in the Win 11 VM)" "$name" zip "no:Windows 7 has no Rust to build for it" "$secs" "$log" "$([ $static = 0 ] && echo static-ok || echo static-FAIL)" win7
    echo "[win7] $name done in ${secs}s"
}

# --- lane win7real: the alpha zip on a REAL Windows 7 (UTM) -------------------------------------
# The Windows 11 lane above proves the package; this one proves Windows 7: its loader starts rapidr.exe, both
# runner programs and the programs Build made, the six functions it lacks are answered by RapidR's stand-ins,
# and a built window shows on its desktop. Driven through UTM's guest agent (tools/win7/vm.sh); the VM must
# be started by Robert, with somebody logged in at its desktop. windows/smoke-win7-real.ps1 is the test.
lane_win7real() {
    local zip="${MATRIX_WIN7_ZIP:-}" log t0 secs name st sys="Windows 7 (real, UTM VM)"
    if [ -z "$zip" ]; then zip="$(ls "$OUT"/RapidR-"$VER"-win7-x86-alpha*.zip 2>/dev/null | tail -1)"; fi
    if [ -z "$zip" ] || [ ! -f "$zip" ]; then record "$sys" "(no win7 zip in $OUT)" zip "no:" 0 "" "no Windows 7 package for this release" win7real; return; fi
    name="$(basename "$zip")"; log="$LOGS/win7real-$name.log"
    st="$("$ROOT/tools/win7/vm.sh" status 2>/dev/null | tr -d '\r' | head -1)"
    if [ "$st" != started ]; then
        NOT_RUN="the Windows 7 VM (UTM) is '${st:-not found}' (this script never starts or stops a VM: ask Robert)"
        echo "[win7real] not run: $NOT_RUN"
        record "$sys" "$name" zip "no:Windows 7 has no Rust to build for it" 0 "" "not run: $NOT_RUN" win7real
        return
    fi
    echo "[win7real] $name: on the real Windows 7 ..."
    t0=$(now)
    bash "$HERE/win7-vm.sh" smoke "$zip" "$REPORT/shots" > "$log" 2>&1
    secs=$(( $(now) - t0 ))
    record "$sys" "$name" zip "no:Windows 7 has no Rust to build for it" "$secs" "$log" "" win7real
    echo "[win7real] $name done in ${secs}s"
}

# --- run the lanes ------------------------------------------------------------------------------
run_lane() {   # <lane>
    local l="$1"
    if [ $SERIAL = 1 ]; then "lane_$l"; else "lane_$l" & LANE_PIDS="$LANE_PIDS $!"; fi
}
if [ $TABLE_ONLY = 0 ]; then
want mac && run_lane mac
want linux && run_lane linux
# (one Windows VM: the installers, then the zip)
if want windows || want win7; then
    if [ $SERIAL = 1 ]; then
        want windows && lane_windows
        want win7 && lane_win7
    else
        ( want windows && lane_windows; want win7 && lane_win7 ) & LANE_PIDS="$LANE_PIDS $!"
    fi
fi
want win7real && run_lane win7real
for p in $LANE_PIDS; do wait "$p" || true; done
fi

# --- the table ----------------------------------------------------------------------------------
cellof() { sed -n "s/.*[ ]$2=\([A-Za-z-]*\).*/\1/p" <<<"$1" | head -1; }
mm() { printf '%d:%02d' $(($1 / 60)) $(($1 % 60)); }
# (the gate is a run of every lane; a run of some writes build-matrix.partial.txt and leaves the full one)
if [ "$ONLY" = "$ALL_LANES" ]; then TXT="$DIST/build-matrix.txt"; else TXT="$DIST/build-matrix.partial.txt"; fi
TABLE="$REPORT/table.tmp"; DETAILS="$REPORT/details.tmp"; : > "$TABLE"; : > "$DETAILS"
RED=0
# show <value> <how it counts when it is "-">: ok | FAIL | n/a (with the reason) | skipped
show() {   # <cell value> <when absent: FAIL|n/a|skipped>
    case "$1" in ok) echo ok ;; FAIL) echo FAIL ;; *) echo "$2" ;; esac
}
printrow() {
    local system="$1" package="$2" kind="$3" native="$4" secs="$5" logf="$6" note="$7" cells r i x n ne o why="" absent=""
    local a_run a_interp a_exe a_native a_nexe a_other
    case "$note" in
        "not run:"*)
            printf '%s\t%s\tnot run\tnot run\tnot run\tnot run\tnot run\t-\n' "$system" "$package" >> "$TABLE"
            echo "- $package: $note" >> "$DETAILS"; RED=1; return ;;
    esac
    if [ -z "$logf" ] || [ ! -f "$logf" ]; then
        printf '%s\t%s\tFAIL\tFAIL\tFAIL\tFAIL\tFAIL\t-\n' "$system" "$package" >> "$TABLE"
        echo "- $package: no result ($note)" >> "$DETAILS"; RED=1; return
    fi
    cells="$(grep '^##cells ' "$logf" | tail -1)"
    if [ -z "$cells" ]; then
        printf '%s\t%s\tFAIL\tFAIL\tFAIL\tFAIL\tFAIL\t%s\n' "$system" "$package" "$(mm "$secs")" >> "$TABLE"
        echo "- $package: the smoke test did not finish (no ##cells line); its last line: $(grep -v '^$' "$logf" | tail -1 | cut -c1-200)" >> "$DETAILS"; RED=1; return
    fi
    a_run="$(cellof "$cells" run)"; a_interp="$(cellof "$cells" interp)"; a_exe="$(cellof "$cells" exe)"
    a_native="$(cellof "$cells" native)"; a_nexe="$(cellof "$cells" nexe)"; a_other="$(cellof "$cells" other)"
    r="$(show "$a_run" FAIL)"; [ "$a_run" = ok ] || absent="$absent run"
    if [ "$kind" = runtime ]; then
        i="n/a (Runtime)"; n="n/a (Runtime)"; x="n/a"
    else
        i="$(show "$a_interp" FAIL)"; [ "$a_interp" = ok ] || absent="$absent interp"
        x="interp $(show "$a_exe" FAIL)"; [ "$a_exe" = ok ] || absent="$absent exe"
        if [ "${native#no:}" != "$native" ]; then n="n/a"; why="${native#no:}"
        elif [ "$QUICK" = 1 ]; then n="skipped"
        else
            n="$(show "$a_native" FAIL)"; [ "$a_native" = ok ] || absent="$absent native"
            ne="$(show "$a_nexe" FAIL)"; [ "$a_nexe" = ok ] || absent="$absent native-exe"
            x="$x / native $ne"
        fi
    fi
    o="$(show "$a_other" ok)"
    [ "$note" = static-FAIL ] && o=FAIL
    # (a failure the cells did not tag: the verdict line, or a FAIL line of a helper script)
    if grep -q '^== smoke test FAILED' "$logf" || grep -q '^  FAIL ' "$logf"; then [ "$o" = ok ] && o=FAIL; fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$system" "$package" "$r" "$i" "$n" "$x" "$o" "$(mm "$secs")" >> "$TABLE"
    case "$r $i $n $x $o" in *FAIL*) RED=1 ;; esac
    if grep -q '^  FAIL ' "$logf"; then
        RED=1
        { echo "- $package:"; grep -m3 -A2 '^  FAIL ' "$logf" | grep -v '^--$' | cut -c1-230 | sed 's/^/    /'; } >> "$DETAILS"
    elif [ -n "$absent" ]; then
        echo "- $package: no check ran for:$absent (the smoke test stopped early or skipped them)" >> "$DETAILS"
    fi
    [ -n "$why" ] && echo "- $package: Build Native: $why" >> "$DETAILS.notes"
}
: > "$DETAILS.notes"
for lane in mac linux windows win7 win7real; do
    want "$lane" || continue
    [ -f "$RES/$lane.tsv" ] || continue
    while IFS=$'\t' read -r system package kind native secs logf note; do
        [ "$logf" = - ] && logf=""; [ "$note" = - ] && note=""
        printrow "$system" "$package" "$kind" "$native" "$secs" "$logf" "$note"
    done < "$RES/$lane.tsv"
done
# lanes asked for that recorded nothing (the lane died)
for lane in $(echo "$ONLY" | tr ',' ' '); do
    [ -f "$RES/$lane.tsv" ] || { echo "- lane $lane: no result (the lane stopped before it recorded anything)" >> "$DETAILS"; RED=1; }
done

{
    echo "RapidR $VER build matrix — $(date '+%Y-%m-%d %H:%M') — $([ $QUICK = 1 ] && echo "quick (no native builds)" || echo "with native builds")"
    echo "  run = rapidr run of a console and a GUI program; Build (interp) = a standalone executable with RapidR's runner, no Rust reachable;"
    echo "  Build Native = rapidr setup + a native build offline; built exe runs = the programs those builds made, started; other = install, file types, Studio, uninstall."
    echo
    { printf 'system\tpackage\trun\tBuild (interp)\tBuild Native\tbuilt exe runs\tother\ttime\n'; cat "$TABLE"; } | python3 -I -c '
import sys
rows = [l.rstrip("\n").split("\t") for l in sys.stdin]
w = [max(len(r[i]) for r in rows) for i in range(len(rows[0]))]
for k, r in enumerate(rows):
    print(" | ".join(c.ljust(w[i]) for i, c in enumerate(r)).rstrip())
    if k == 0:
        print("-+-".join("-" * x for x in w))
'
    echo
    if [ -s "$DETAILS.notes" ]; then echo "Not checked here (n/a):"; sort -u "$DETAILS.notes"; echo; fi
    if [ -s "$DETAILS" ]; then echo "First error of each red row:"; cat "$DETAILS"; echo; fi
    if [ $RED = 0 ]; then echo "ALL GREEN: every cell that can be checked here passed."; else echo "RED: see above. Logs: $LOGS"; fi
} | tee "$TXT"

# --- the pictures (--shots): the BMPs the Windows programs captured, as PNGs ------------------------
if [ $SHOTS = 1 ] && { want windows || want win7; }; then
    step "fetch the pictures"
    for f in $("$HERE/windows-vm.sh" ls 'out\matrix-shots' 2>/dev/null | tr -d '\r' | grep -i '\.bmp$'); do
        if "$HERE/windows-vm.sh" fetch "matrix-shots\\$f" "$REPORT/shots/$f" > /dev/null 2>&1; then
            sips -s format png "$REPORT/shots/$f" --out "$REPORT/shots/${f%.bmp}.png" > /dev/null 2>&1 && rm -f "$REPORT/shots/$f"
            echo "  $REPORT/shots/${f%.bmp}.png"
        fi
    done
    "$HERE/windows-vm.sh" shots-clear > /dev/null 2>&1 || true
fi
rm -f "$TABLE" "$DETAILS" "$DETAILS.notes"
[ $RED = 0 ]
