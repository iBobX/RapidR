#!/bin/bash
# Linux, for this machine's architecture (x86_64 or aarch64): the SDK and the
# Runtime, each as a .tar.gz (per user, no root: install.sh) and a .deb.
#
#   dist/<ver>/out/rapidr-<ver>-linux-<arch>.tar.gz          rapidr_<ver>_<deb arch>.deb
#   dist/<ver>/out/rapidr-runtime-<ver>-linux-<arch>.tar.gz  rapidr-runtime_<ver>_<deb arch>.deb
#
# Run on Linux, from the source of the release (prepare.sh's src.tar) — the
# Mac runs it in Docker: tools/release/linux-docker.sh. Needs Rust, the
# desktop host's -dev packages (tools/linux/Dockerfile), python3, dpkg-deb.
# AppImage: not made (see docs/release-packaging.md).
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
[ -f "$PREP/rapidr-ide.rrbc" ] || die "no $PREP: run tools/release/prepare.sh on the Mac first"
[ -e "$ROOT/.git" ] && die "run from the release's source archive (prep/src.tar, extracted), not a checkout"
need dpkg-deb "the .deb"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) DEBARCH=amd64 ;;
    aarch64) DEBARCH=arm64 ;;
    *) die "unsupported architecture $ARCH" ;;
esac
TD="${CARGO_TARGET_DIR:-target}"
W="$WORK/linux-$ARCH"
rm -rf "$W" && mkdir -p "$W"
trap 'rm -rf "$W"' EXIT

step "build (linux-$ARCH)"
cargo build -q --locked --release -p rapidr-cli
cargo build -q --locked --profile runner -p rapidr-runner-stub --bin rapidrintr-runner
strip "$TD/release/rapidr"
echo "rapidr links (system libraries, not shipped):"
ldd "$TD/release/rapidr" | awk '{print "  " $1}'

step "the home: the runtime's sources, their crates vendored"
python3 tools/release/home.py --os linux --src "$ROOT" --out "$W/home"

step "stage"
RUST="$(sed -n 's/^rust = "\(.*\)"/\1/p' "$W/home/release.toml")"
python3 tools/release/stage.py --kind sdk --os linux --out "$W/sdk" --bin "$TD/release" --home "$W/home" \
    --runner "linux-$ARCH=$TD/runner" --web "$PREP/web-runtime" --ide "$PREP/rapidr-ide.rrbc"
python3 tools/release/stage.py --kind runtime --os linux --out "$W/runtime" --bin "$TD/release" --version "$VERSION" --rust "$RUST"

# The file types and their applications (the runtime's; the IDE's in the SDK).
desktop_files() {
    local stage="$1" kind="$2"
    mkdir -p "$stage/share/mime/packages" "$stage/share/applications"
    cp tools/release/linux/rapidr.xml "$stage/share/mime/packages/"
    cp tools/release/linux/rapidr-runtime.desktop "$stage/share/applications/"
    [ "$kind" = sdk ] && cp tools/release/linux/rapidr-ide.desktop "$stage/share/applications/"
    return 0
}

tarball() {
    local kind="$1" name="$2"
    local top="$W/tar/$name"
    mkdir -p "$W/tar"
    cp -R "$W/$kind" "$top"
    desktop_files "$top" "$kind"
    cp tools/release/linux/install.sh tools/release/linux/uninstall.sh "$top/"
    chmod +x "$top/install.sh" "$top/uninstall.sh"
    tar -C "$W/tar" --owner=0 --group=0 -czf "$OUT/$name.tar.gz" "$name"
    rm -rf "$top"
    echo "wrote $OUT/$name.tar.gz ($(du -h "$OUT/$name.tar.gz" | cut -f1))"
}

# A .deb: /usr/bin/rapidr and its home /usr/lib/rapidr (the CLI's rule:
# <exe>/../lib/rapidr); dpkg's triggers update the MIME and desktop
# databases. The SDK replaces the runtime (both have /usr/bin/rapidr).
deb() {
    local kind="$1" pkg="$2" desc="$3"
    local root="$W/deb-$pkg"
    mkdir -p "$root/usr" "$root/DEBIAN"
    cp -R "$W/$kind/bin" "$W/$kind/lib" "$root/usr/"
    mkdir -p "$root/usr/share/doc/$pkg"
    cp -R "$W/$kind/share/doc/rapidr/." "$root/usr/share/doc/$pkg/"
    cp LICENSE "$root/usr/share/doc/$pkg/copyright"
    desktop_files "$root/usr" "$kind"
    sed -i "s|@BIN@|/usr/bin|g" "$root"/usr/share/applications/*.desktop
    # (the system libraries rapidr links: dpkg-shlibdeps reads a source tree)
    local deps
    deps="$(cd "$W" && mkdir -p shlibs/debian && printf 'Source: rapidr\n' > shlibs/debian/control \
        && cd shlibs && dpkg-shlibdeps -O "$root/usr/bin/rapidr" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')"
    local other="rapidr-runtime"; [ "$pkg" = rapidr-runtime ] && other="rapidr"
    cat > "$root/DEBIAN/control" <<EOF
Package: $pkg
Version: $VERSION
Architecture: $DEBARCH
Maintainer: RapidR <https://github.com/iBobX/RapidR>
Installed-Size: $(du -sk "$root/usr" | cut -f1)
Depends: ${deps:-libc6}
Conflicts: $other
Replaces: $other
Section: devel
Priority: optional
Homepage: https://github.com/iBobX/RapidR
Description: $desc
 RapidR runs and builds RapidQ BASIC programs: compatible with RapidQ, on
 the interpreter, as native executables and on the web; an original
 implementation written from the ground up in pure Rust.
EOF
    dpkg-deb --root-owner-group --build "$root" "$OUT/${pkg}_${VERSION}_${DEBARCH}.deb" >/dev/null
    rm -rf "$root"
    echo "wrote $OUT/${pkg}_${VERSION}_${DEBARCH}.deb ($(du -h "$OUT/${pkg}_${VERSION}_${DEBARCH}.deb" | cut -f1))"
}

step "packages"
tarball sdk "rapidr-$VERSION-linux-$ARCH"
tarball runtime "rapidr-runtime-$VERSION-linux-$ARCH"
deb sdk rapidr "RapidR — BASIC compiler, interpreter and IDE (SDK)"
deb runtime rapidr-runtime "RapidR Runtime — runs RapidR and RapidQ programs"
