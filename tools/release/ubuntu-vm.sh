#!/bin/bash
# Drives the Ubuntu VM (Parallels, "Ubuntu 24.04.3 ARM64"; RAPIDR_LINUX_VM
# names another) for the Linux release and its smoke test:
#
#   tools/release/ubuntu-vm.sh send <local file> <vm path>
#   tools/release/ubuntu-vm.sh run <local script> [args…]
#   tools/release/ubuntu-vm.sh root <local script> [args…]   (as root: installing packages, cleaning;
#                                    the only mode that works while nobody is logged in at the VM's desktop or its screen is locked)
#   tools/release/ubuntu-vm.sh fetch <vm path> <local file>
#
# prlctl exec's quirks: it drops the outer command's quoting and dash
# options (so commands go in script files), it needs a stdin, binary stdin
# fails, and over ~1 MB a stdin or stdout fails ("Invalid argument") — so
# files travel as base64 text in pieces, each retried, checked by SHA-256.
set -euo pipefail
VM="${RAPIDR_LINUX_VM:-Ubuntu 24.04.3 ARM64}"
PIECE=400000
# (Parallels sometimes answers "Invalid argument" to a call that is fine:
# every call is retried, and what it prints is taken from the one that worked)
USER_OPT=(--current-user)
# --current-user needs somebody logged in at the VM's desktop with the screen unlocked: a VM that was
# just started (the greeter shows) answers "Unable to authenticate the user" to every call, and the
# retries below would only wait. Then (pick_user, once per run/send/fetch) the calls go as root and
# `run` hands the script to the VM's first normal user (the helper's `asuser`: runuser, that user's
# HOME), so the build matrix and the smoke tests work on a VM nobody is logged in at.
# Two rules of Ubuntu's /tmp (fs.protected_regular): root cannot write a file the user owns there, and
# the user cannot write a root-owned one. So the root calls use names of their own (HELPER, PFX), the
# files root makes in /tmp stay root's and are removed after `run`, and a file sent to "~/..." goes to
# the user's home (resolved here: root's ~ is /root) and is given to the user.
AS_USER=0; HELPER=/tmp/rapidr-vm.sh; PFX=/tmp/rapidr-release-
# (the root helper has a name of its own per call: two calls at once, as the build matrix makes, must not delete each other's)
as_root() { USER_OPT=(); HELPER="/tmp/rapidr-vm-root.$$.sh"; PFX=/tmp/rapidr-release-root-; }
finish() { [ "$HELPER" = /tmp/rapidr-vm.sh ] || x bash "$HELPER" drop "$HELPER.none" > /dev/null 2>&1 || true; }
pick_user() {
    # (the user's own session only when a probe works; any other answer, an Invalid argument Parallels gives
    # to a call that is fine included, is asked again, and after three the root way is taken: it works
    # whether or not somebody is logged in)
    local try
    for try in 1 2 3; do
        prlctl exec "$VM" --current-user true < /dev/null > /dev/null 2>&1 && return 0
        sleep 1
    done
    as_root; AS_USER=1
    return 0
}
x() { xin /dev/null "$@"; }
# (the same, its stdin read from a file — each try from the start; a call that
# hangs is ended after RAPIDR_VM_CALL_TIMEOUT seconds, default 300, and tried again)
TIMEOUT="${RAPIDR_VM_CALL_TIMEOUT:-300}"
call() {
    # (`<&0`: a background command would read /dev/null; the watcher's output goes
    # nowhere, so `$(…)` doesn't wait for it)
    "$@" <&0 &
    local pid=$! rc=0
    ( sleep "$TIMEOUT"; kill "$pid" 2>/dev/null ) > /dev/null 2>&1 &
    local watch=$!
    wait "$pid" || rc=$?
    kill "$watch" 2>/dev/null || true
    return $rc
}
xin() {
    local in="$1" try out
    shift
    for try in 1 2 3 4 5 6; do
        if out="$(call prlctl exec "$VM" ${USER_OPT[@]+"${USER_OPT[@]}"} "$@" < "$in")"; then
            [ -n "$out" ] && printf '%s\n' "$out"
            return 0
        fi
        sleep 3
    done
    return 1
}
# (`bash -c "cat > f"` arrives as `bash -c cat > f`: the VM's shell keeps
# the redirection, so text goes through; scripts there do the rest)
put_text() { xin "$1" bash -c "cat > $2"; }
helpers() {
    local h; h="$(mktemp)"
    cat > "$h" <<'EOF'
# rapidr-vm.sh <command> …: the VM's side of tools/release/ubuntu-vm.sh
# (as root, the VM's first normal user: its home is "~", it owns the files sent into its home)
user_of() { getent passwd | awk -F: '$3 >= 1000 && $3 < 60000 && $6 ~ /^\/home\// { print $1; exit }'; }
home_of() { getent passwd "$1" | cut -d: -f6; }
expand() {
    case "$1" in
        "~/"*) if [ "$(id -u)" = 0 ] && [ -n "$(user_of)" ]; then echo "$(home_of "$(user_of)")/${1#"~/"}"; else echo "$HOME/${1#"~/"}"; fi ;;
        *) echo "$1" ;;
    esac
}
case "$1" in
    join) f="$(expand "$2")"; if ls "$f".part.* >/dev/null 2>&1; then cat "$f".part.* | base64 -d > "$f" && rm -f "$f".part.*; fi
          # (root's umask is 077: the user must be able to read what root sent)
          chmod a+r "$f" 2>/dev/null
          case "$f" in /tmp/*|/var/tmp/*) ;; *) if [ "$(id -u)" = 0 ] && [ -n "$(user_of)" ]; then chown "$(user_of)" "$f" 2>/dev/null; fi ;; esac
          sha256sum "$f" | cut -d" " -f1 ;;
    asuser) u="$(user_of)"; h="$(home_of "$u")"; shift; cd "$h" && exec runuser -u "$u" -- env HOME="$h" USER="$u" LOGNAME="$u" XDG_RUNTIME_DIR="/run/user/$(id -u "$u")" bash "$@" ;;
    split) f="$(expand "$2")"; rm -f /tmp/rapidr-fetch.*; split -b "$3" -d -a 5 "$f" /tmp/rapidr-fetch.; ls /tmp/rapidr-fetch.*; ;;
    piece) base64 "$2"; echo "sha256 $(sha256sum "$2" | cut -d" " -f1)" ;;
    sum) sha256sum "$(expand "$2")" | cut -d" " -f1 ;;
    clean) rm -f /tmp/rapidr-fetch.* ;;
    userhome) home_of "$(user_of)" ;;
    drop) rm -f "$2" "$2".part.* "$0" ;;
esac
EOF
    put_text "$h" "$HELPER"
    rm -f "$h"
}
# (a path in the user's home: "~/x" is written out when the calls run as root, whose shell's ~ is "/")
abs() {
    case "$1" in
        "~/"*) if [ ${#USER_OPT[@]} = 0 ]; then echo "$(x bash "$HELPER" userhome)/${1#"~/"}"; return; fi ;;
    esac
    echo "$1"
}
send() {
    local parts n=0 dst; dst="$(abs "$2")"
    parts="$(mktemp -d)"
    base64 -i "$1" | split -b $PIECE - "$parts/p."
    for p in "$parts"/p.*; do
        put_text "$p" "$dst.part.$(printf %05d $n)"
        n=$((n + 1))
    done
    rm -rf "$parts"
    local sum; sum="$(x bash "$HELPER" join "$dst")"
    [ "$sum" = "$(shasum -a 256 "$1" | cut -d' ' -f1)" ] || { echo "send $1: checksum differs" >&2; return 1; }
}
fetch() {
    local tmp bin try ok src; tmp="$(mktemp)"; bin="$(mktemp)"; src="$(abs "$1")"
    : > "$2"
    for p in $(x bash "$HELPER" split "$src" $PIECE); do
        # (each piece checked: an answer can come back cut short)
        ok=""
        for try in 1 2 3 4 5 6; do
            x bash "$HELPER" piece "$p" > "$tmp" || continue
            grep -v '^sha256 ' "$tmp" | base64 -d > "$bin"
            if [ "$(sed -n 's/^sha256 //p' "$tmp")" = "$(shasum -a 256 "$bin" | cut -d' ' -f1)" ]; then ok=1; break; fi
        done
        [ -n "$ok" ] || { echo "fetch $src: piece $p failed" >&2; return 1; }
        cat "$bin" >> "$2"
    done
    rm -f "$tmp" "$bin"
    x bash "$HELPER" clean
    [ "$(x bash "$HELPER" sum "$src")" = "$(shasum -a 256 "$2" | cut -d' ' -f1)" ] || { echo "fetch $src: checksum differs" >&2; return 1; }
}
case "${1:-}" in
    send) pick_user; helpers; rc=0; send "$2" "$3" || rc=$?; finish; exit $rc ;;
    run)
        script="$2"; shift 2
        pick_user
        name="$PFX$(basename "$script")"
        helpers; send "$script" "$name"
        rc=0
        if [ $AS_USER = 1 ]; then
            # (nobody logged in: the script runs as the VM's user, the copy root made is removed after)
            x bash "$HELPER" asuser "$name" "$@" || rc=$?
            x bash "$HELPER" drop "$name" || true
        else
            x bash "$name" "$@" || rc=$?
        fi
        exit $rc
        ;;
    fetch) pick_user; helpers; rc=0; fetch "$2" "$3" || rc=$?; finish; exit $rc ;;
    root)
        script="$2"; shift 2
        # (everything as root, the files travel too: --current-user needs somebody logged in at the
        # VM's desktop with the screen unlocked — "Unable to authenticate the user" otherwise — and
        # root works when nobody is. The /tmp files made are removed after, so a
        # later run as the user isn't met by root-owned ones.)
        as_root
        name="$PFX$(basename "$script")"
        helpers; send "$script" "$name"
        rc=0; x bash "$name" "$@" || rc=$?
        x bash "$HELPER" drop "$name" || true
        exit $rc
        ;;
    *) echo "usage: $0 send|run|fetch|root …" >&2; exit 2 ;;
esac
