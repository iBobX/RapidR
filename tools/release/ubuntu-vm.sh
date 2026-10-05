#!/bin/bash
# Drives the Ubuntu VM (Parallels, "Ubuntu 24.04.3 ARM64"; RAPIDR_LINUX_VM
# names another) for the Linux release and its smoke test:
#
#   tools/release/ubuntu-vm.sh send <local file> <vm path>
#   tools/release/ubuntu-vm.sh run <local script> [args…]
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
x() { xin /dev/null "$@"; }
# (the same, its stdin read from a file — each try from the start)
xin() {
    local in="$1" try out
    shift
    for try in 1 2 3 4 5 6; do
        if out="$(prlctl exec "$VM" --current-user "$@" < "$in")"; then
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
expand() { case "$1" in "~/"*) echo "$HOME/${1#"~/"}" ;; *) echo "$1" ;; esac; }
case "$1" in
    join) f="$(expand "$2")"; if ls "$f".part.* >/dev/null 2>&1; then cat "$f".part.* | base64 -d > "$f" && rm -f "$f".part.*; fi; sha256sum "$f" | cut -d" " -f1 ;;
    split) f="$(expand "$2")"; rm -f /tmp/rapidr-fetch.*; split -b "$3" -d -a 5 "$f" /tmp/rapidr-fetch.; ls /tmp/rapidr-fetch.*; ;;
    piece) base64 "$2"; echo "sha256 $(sha256sum "$2" | cut -d" " -f1)" ;;
    sum) sha256sum "$(expand "$2")" | cut -d" " -f1 ;;
    clean) rm -f /tmp/rapidr-fetch.* ;;
esac
EOF
    put_text "$h" /tmp/rapidr-vm.sh
    rm -f "$h"
}
send() {
    local parts n=0
    parts="$(mktemp -d)"
    base64 -i "$1" | split -b $PIECE - "$parts/p."
    for p in "$parts"/p.*; do
        put_text "$p" "$2.part.$(printf %05d $n)"
        n=$((n + 1))
    done
    rm -rf "$parts"
    local sum; sum="$(x bash /tmp/rapidr-vm.sh join "$2")"
    [ "$sum" = "$(shasum -a 256 "$1" | cut -d' ' -f1)" ] || { echo "send $1: checksum differs" >&2; return 1; }
}
fetch() {
    local tmp bin try ok; tmp="$(mktemp)"; bin="$(mktemp)"
    : > "$2"
    for p in $(x bash /tmp/rapidr-vm.sh split "$1" $PIECE); do
        # (each piece checked: an answer can come back cut short)
        ok=""
        for try in 1 2 3 4 5 6; do
            x bash /tmp/rapidr-vm.sh piece "$p" > "$tmp" || continue
            grep -v '^sha256 ' "$tmp" | base64 -d > "$bin"
            if [ "$(sed -n 's/^sha256 //p' "$tmp")" = "$(shasum -a 256 "$bin" | cut -d' ' -f1)" ]; then ok=1; break; fi
        done
        [ -n "$ok" ] || { echo "fetch $1: piece $p failed" >&2; return 1; }
        cat "$bin" >> "$2"
    done
    rm -f "$tmp" "$bin"
    x bash /tmp/rapidr-vm.sh clean
    [ "$(x bash /tmp/rapidr-vm.sh sum "$1")" = "$(shasum -a 256 "$2" | cut -d' ' -f1)" ] || { echo "fetch $1: checksum differs" >&2; return 1; }
}
case "${1:-}" in
    send) helpers; send "$2" "$3" ;;
    run)
        script="$2"; shift 2
        name="/tmp/rapidr-release-$(basename "$script")"
        helpers; send "$script" "$name"
        x bash "$name" "$@"
        ;;
    fetch) helpers; fetch "$2" "$3" ;;
    *) echo "usage: $0 send|run|fetch …" >&2; exit 2 ;;
esac
