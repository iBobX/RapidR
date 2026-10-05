#!/bin/bash
# Drives the Ubuntu VM (Parallels, "Ubuntu 24.04.3 ARM64"; RAPIDR_LINUX_VM
# names another) for the Linux release's smoke test:
#
#   tools/release/ubuntu-vm.sh send <local file> <vm path>
#   tools/release/ubuntu-vm.sh run <local script> [args…]
#   tools/release/ubuntu-vm.sh fetch <vm path> <local file>
#
# prlctl exec's quirks: it drops the outer command's quoting and dash
# options (so commands go in script files), it needs a stdin, and binary
# stdin fails — files travel as base64 text.
set -euo pipefail
VM="${RAPIDR_LINUX_VM:-Ubuntu 24.04.3 ARM64}"
x() { prlctl exec "$VM" --current-user "$@"; }
# (`bash -c "cat > f"` arrives as `bash -c cat > f`: the VM's shell keeps the
# redirection, so plain text goes through; a small decoder does the rest)
send() {
    printf 'base64 -d "$1" > "$2" && rm -f "$1"\n' | x bash -c "cat > /tmp/rapidr-b64dec.sh"
    base64 -b 76 -i "$1" | x bash -c "cat > $2.b64"
    x bash /tmp/rapidr-b64dec.sh "$2.b64" "$2" < /dev/null
}
case "${1:-}" in
    send)
        send "$2" "$3"
        ;;
    run)
        script="$2"; shift 2
        name="/tmp/rapidr-release-$(basename "$script")"
        send "$script" "$name"
        x bash "$name" "$@" < /dev/null
        ;;
    fetch)
        printf 'f="$1"; case "$f" in "~/"*) f="$HOME/${f#"~/"}" ;; esac; base64 "$f"\n' > "${TMPDIR:-/tmp}/rapidr-fetch.sh"
        "$0" send "${TMPDIR:-/tmp}/rapidr-fetch.sh" /tmp/rapidr-fetch.sh
        x bash /tmp/rapidr-fetch.sh "$2" < /dev/null | base64 -d > "$3"
        rm -f "${TMPDIR:-/tmp}/rapidr-fetch.sh"
        ;;
    *) echo "usage: $0 send|run|fetch …" >&2; exit 2 ;;
esac
