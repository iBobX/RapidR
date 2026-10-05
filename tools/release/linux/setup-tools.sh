#!/bin/bash
# The Linux release machine's tools: build both architectures on one Ubuntu
# (24.04, arm64 or amd64) and run the other one's programs under emulation.
#
#   sudo bash setup-tools.sh system    the other architecture's -dev libraries (multiarch,
#                                      from Ubuntu's archives: ports.ubuntu.com for arm64,
#                                      archive / security.ubuntu.com for amd64), qemu-user-static
#                                      and binfmt-support
#   bash setup-tools.sh user           Zig (ziglang.org, checked against the SHA-256 its
#                                      download index publishes), cargo-zigbuild (crates.io),
#                                      both Rust Linux targets
#   sudo bash setup-tools.sh undo      removes what `system` added
#
# Why: the Linux artifacts link against a glibc baseline (linux.sh: GLIBC) with Zig as the
# linker, and need the other architecture's ALSA / fontconfig / FreeType to link.
set -euo pipefail
ZIG="${ZIG:-0.14.1}"
HOST="$(dpkg --print-architecture)"
case "$HOST" in
    arm64) OTHER=amd64; OTHER_URIS="http://archive.ubuntu.com/ubuntu/"; OTHER_SEC="http://security.ubuntu.com/ubuntu/" ;;
    amd64) OTHER=arm64; OTHER_URIS="http://ports.ubuntu.com/ubuntu-ports/"; OTHER_SEC="http://ports.ubuntu.com/ubuntu-ports/" ;;
    *) echo "unsupported host $HOST"; exit 1 ;;
esac
CODENAME="$(. /etc/os-release && echo "$VERSION_CODENAME")"
LIBS="libasound2-dev libfontconfig-dev libfreetype-dev"
SOURCES=/etc/apt/sources.list.d/ubuntu.sources
OURS="/etc/apt/sources.list.d/rapidr-$OTHER.sources"

case "${1:-}" in
    system)
        [ "$(id -u)" = 0 ] || { echo "run as root"; exit 1; }
        # (the existing sources: this architecture only, so apt doesn't ask them for the other)
        grep -q "^Architectures:" "$SOURCES" || { cp "$SOURCES" "$SOURCES.rapidr-orig"; sed -i "/^Types:/a Architectures: $HOST" "$SOURCES"; }
        cat > "$OURS" <<EOF
# Added by RapidR's tools/release/linux/setup-tools.sh ($OTHER libraries for cross-linking)
Types: deb
URIs: $OTHER_URIS
Suites: $CODENAME $CODENAME-updates
Components: main restricted universe
Architectures: $OTHER
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: $OTHER_SEC
Suites: $CODENAME-security
Components: main restricted universe
Architectures: $OTHER
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
EOF
        dpkg --add-architecture "$OTHER"
        apt-get update -q
        DEBIAN_FRONTEND=noninteractive apt-get install -y -q $(for l in $LIBS; do echo "$l:$OTHER"; done) qemu-user-static binfmt-support
        echo "installed: $LIBS for $OTHER, qemu-user-static, binfmt-support"
        ;;
    undo)
        [ "$(id -u)" = 0 ] || { echo "run as root"; exit 1; }
        DEBIAN_FRONTEND=noninteractive apt-get purge -y -q $(dpkg-query -W -f '${Package}:${Architecture}\n' | grep ":$OTHER\$") || true
        dpkg --remove-architecture "$OTHER" || true
        rm -f "$OURS"
        [ -f "$SOURCES.rapidr-orig" ] && mv "$SOURCES.rapidr-orig" "$SOURCES"
        apt-get update -q
        ;;
    user)
        export PATH="$HOME/.cargo/bin:$PATH"
        TOOLS="$HOME/rapidr-tools"
        mkdir -p "$TOOLS"
        ARCH="$(uname -m)"
        DIR="$TOOLS/zig-$ARCH-linux-$ZIG"
        if [ ! -x "$DIR/zig" ]; then
            python3 - "$ZIG" "$ARCH" "$TOOLS" <<'EOF'
import hashlib, json, sys, urllib.request
ver, arch, tools = sys.argv[1:]
index = json.load(urllib.request.urlopen("https://ziglang.org/download/index.json"))
entry = index[ver][f"{arch}-linux"]
url, want = entry["tarball"], entry["shasum"]
data = urllib.request.urlopen(url).read()
have = hashlib.sha256(data).hexdigest()
if have != want:
    sys.exit(f"{url}: SHA-256 {have}, ziglang.org says {want}")
open(f"{tools}/{url.rsplit('/', 1)[1]}", "wb").write(data)
print(f"zig {ver}: SHA-256 ok ({have})")
EOF
            tar -xJf "$TOOLS"/zig-*"$ZIG".tar.xz -C "$TOOLS"
            rm -f "$TOOLS"/zig-*"$ZIG".tar.xz
        fi
        ln -sf "$DIR/zig" "$HOME/.cargo/bin/zig"
        command -v cargo-zigbuild >/dev/null || cargo install --locked cargo-zigbuild
        rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
        echo "zig: $(zig version) ($DIR); $(cargo-zigbuild --version)"
        ;;
    *) echo "usage: $0 system|user|undo"; exit 2 ;;
esac
