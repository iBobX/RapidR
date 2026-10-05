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
#   sudo bash setup-tools.sh qemu      a newer qemu-user for the other architecture's programs
#                                      (24.04's crashes on Rust programs): see below
#   sudo bash setup-tools.sh undo      removes what `system` added (qemu-undo: what `qemu` did)
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
        umask 022
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
        "$0" qemu-undo || true
        DEBIAN_FRONTEND=noninteractive apt-get purge -y -q $(dpkg-query -W -f '${Package}:${Architecture}\n' | grep ":$OTHER\$") || true
        dpkg --remove-architecture "$OTHER" || true
        rm -f "$OURS"
        [ -f "$SOURCES.rapidr-orig" ] && mv "$SOURCES.rapidr-orig" "$SOURCES"
        apt-get update -q
        ;;
    qemu)
        # Ubuntu 24.04's qemu-user (8.2) crashes on every Rust program of the other
        # architecture (reading /proc/self/maps: "QEMU internal SIGSEGV"). A newer one
        # from a later Ubuntu release's archive (QEMU_SUITE, default questing), checked:
        # InRelease against Ubuntu's archive key, Packages.xz and the .deb against their
        # SHA-256s; unpacked into /usr/local/lib/rapidr-qemu (not installed), and used
        # for the other architecture's programs instead of the system's (binfmt, until
        # reboot or `qemu-undo`).
        [ "$(id -u)" = 0 ] || { echo "run as root"; exit 1; }
        case "$OTHER" in amd64) QARCH=x86_64 ;; arm64) QARCH=aarch64 ;; esac
        DEST=/usr/local/lib/rapidr-qemu
        python3 - "${QEMU_SUITE:-questing}" "$HOST" "$OTHER_URIS" "$QARCH" "$DEST" <<'EOF'
import hashlib, lzma, os, re, subprocess, sys, tempfile, urllib.request
suite, arch, base, qarch, dest = sys.argv[1:]
if arch == "arm64":
    base = "http://ports.ubuntu.com/ubuntu-ports/"
else:
    base = "http://archive.ubuntu.com/ubuntu/"
get = lambda u: urllib.request.urlopen(u).read()
tmp = tempfile.mkdtemp()
inrel = os.path.join(tmp, "InRelease")
open(inrel, "wb").write(get(f"{base}dists/{suite}/InRelease"))
clear = os.path.join(tmp, "Release")
subprocess.run(["gpgv", "--keyring", "/usr/share/keyrings/ubuntu-archive-keyring.gpg", "--output", clear, inrel], check=True, capture_output=True)
text = open(clear).read()
sha = text[text.index("SHA256:"):]
found = None
for comp in ("main", "universe"):
    want = re.search(rf"^ ([0-9a-f]{{64}})\s+\d+ {comp}/binary-{arch}/Packages.xz$", sha, re.M).group(1)
    pk = get(f"{base}dists/{suite}/{comp}/binary-{arch}/Packages.xz")
    assert hashlib.sha256(pk).hexdigest() == want, f"{comp} Packages.xz: SHA-256 differs"
    for stanza in lzma.decompress(pk).decode().split("\n\n"):
        f = dict(re.findall(r"^(\w[\w-]*): (.*)$", stanza, re.M))
        # (qemu-user is static since QEMU 9 in Debian / Ubuntu; qemu-user-static before)
        if f.get("Package") in ("qemu-user", "qemu-user-static") and int(f.get("Installed-Size", "0")) > 10000:
            found = f
            break
    if found:
        break
if not found:
    sys.exit("no qemu-user in " + suite)
f = found
deb = get(base + f["Filename"])
assert hashlib.sha256(deb).hexdigest() == f["SHA256"], "the .deb: SHA-256 differs"
path = os.path.join(tmp, "q.deb")
open(path, "wb").write(deb)
out = os.path.join(dest, f["Version"])
subprocess.run(["rm", "-rf", dest], check=True)
os.makedirs(out)
subprocess.run(["dpkg-deb", "-x", path, out], check=True)
binary = os.path.join(out, "usr/bin", f"qemu-{qarch}")
if not os.path.exists(binary):
    binary = os.path.join(out, "usr/bin", f"qemu-{qarch}-static")
os.symlink(binary, os.path.join(dest, "qemu"))
print(f"qemu-user {f['Version']} ({suite}): signature and SHA-256s ok -> {binary}")
EOF
        file -L "$DEST/qemu" | grep -Eq "statically linked|static-pie linked" || { echo "$DEST/qemu isn't static"; exit 1; }
        # (the system's entry's magic and mask, the new interpreter)
        STOCK="/proc/sys/fs/binfmt_misc/qemu-$QARCH"
        MAGIC="$(sed -n 's/^magic //p' "$STOCK" | sed 's/../\\x&/g')"
        MASK="$(sed -n 's/^mask //p' "$STOCK" | sed 's/../\\x&/g')"
        [ -e "/proc/sys/fs/binfmt_misc/rapidr-$QARCH" ] && echo -1 > "/proc/sys/fs/binfmt_misc/rapidr-$QARCH"
        echo 0 > "$STOCK"
        printf ':rapidr-%s:M::%s:%s:%s:POF\n' "$QARCH" "$MAGIC" "$MASK" "$DEST/qemu" > /proc/sys/fs/binfmt_misc/register
        echo "binfmt: $QARCH programs run by $DEST/qemu (the system's qemu-$QARCH entry disabled until reboot or qemu-undo)"
        ;;
    qemu-undo)
        [ "$(id -u)" = 0 ] || { echo "run as root"; exit 1; }
        case "$OTHER" in amd64) QARCH=x86_64 ;; arm64) QARCH=aarch64 ;; esac
        [ -e "/proc/sys/fs/binfmt_misc/rapidr-$QARCH" ] && echo -1 > "/proc/sys/fs/binfmt_misc/rapidr-$QARCH"
        [ -e "/proc/sys/fs/binfmt_misc/qemu-$QARCH" ] && echo 1 > "/proc/sys/fs/binfmt_misc/qemu-$QARCH"
        rm -rf /usr/local/lib/rapidr-qemu
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
