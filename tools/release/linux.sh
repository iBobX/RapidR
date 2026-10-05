#!/bin/bash
# Linux, x86_64 and aarch64, built on one Ubuntu (either architecture): the SDK
# and the Runtime, each as a .tar.gz (per user, no root: install.sh) and a .deb.
#
#   dist/<ver>/out/rapidr-<ver>-linux-<arch>.tar.gz          rapidr_<ver>_<deb arch>.deb
#   dist/<ver>/out/rapidr-runtime-<ver>-linux-<arch>.tar.gz  rapidr-runtime_<ver>_<deb arch>.deb
#
#   tools/release/linux.sh [x86_64] [aarch64]        (default: both)
#
# Linked by Zig (cargo-zigbuild) against glibc $GLIBC — Ubuntu 20.04, Debian 11 and
# newer — whichever the build machine's: the binaries ask for no newer symbol
# (checked). The other architecture's ALSA, FreeType and fontconfig come from
# multiarch -dev packages (fontique can't open fontconfig at run time: it is
# linked, as on any desktop it is installed). tools/release/linux/
# setup-tools.sh installs all of it. Run from the release's source (prepare.sh's
# src.tar, extracted): tools/release/linux-vm.sh does, in the Ubuntu VM.
# AppImage: not made (see docs/release-packaging.md).
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
[ -f "$PREP/rapidr-ide.rrbc" ] || die "no $PREP: run tools/release/prepare.sh on the Mac first"
[ -e "$ROOT/.git" ] && die "run from the release's source archive (prep/src.tar, extracted), not a checkout"
need dpkg-deb "the .deb"
need cargo-zigbuild "tools/release/linux/setup-tools.sh user"
need readelf "binutils"
GLIBC="${GLIBC:-2.31}"
ARCHS="${*:-x86_64 aarch64}"
TD="${CARGO_TARGET_DIR:-target}"
W="$WORK/linux"
rm -rf "$W" && mkdir -p "$W"
trap 'rm -rf "$W"' EXIT

step "the home: the runtime's sources, their crates vendored (both architectures)"
python3 tools/release/home.py --os linux --src "$ROOT" --out "$W/home"
RUST="$(sed -n 's/^rust = "\(.*\)"/\1/p' "$W/home/release.toml")"

# What a binary needs: its libraries, and the newest glibc symbol version.
needed() { readelf -d "$1" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p'; }
newest_glibc() { readelf -V -W "$1" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1; }
# Their packages, by names every Debian / Ubuntu since 2020 knows (24.04 renamed libasound2).
depends() {
    local deps=("libc6 (>= $GLIBC)")
    for l in $(needed "$1"); do
        case "$l" in
            libc.so.6|libm.so.6|libdl.so.2|libpthread.so.0|librt.so.1|ld-linux*) ;;
            libgcc_s.so.1) deps+=("libgcc-s1 | libgcc1") ;;
            libasound.so.2) deps+=("libasound2t64 | libasound2") ;;
            libfreetype.so.6) deps+=("libfreetype6") ;;
            libfontconfig.so.1) deps+=("libfontconfig1") ;;
            libz.so.1) deps+=("zlib1g") ;;
            *) die "$1 needs $l: add its package to depends() in linux.sh" ;;
        esac
    done
    printf '%s\n' "${deps[@]}" | awk '!seen[$0]++' | paste -sd, - | sed 's/,/, /g'
}

# The file types and their applications (the runtime's; the IDE's in the SDK).
desktop_files() {
    local stage="$1" kind="$2"
    mkdir -p "$stage/share/mime/packages" "$stage/share/applications"
    cp tools/release/linux/rapidr.xml "$stage/share/mime/packages/"
    cp tools/release/linux/rapidr-runtime.desktop "$stage/share/applications/"
    [ "$kind" = sdk ] && cp tools/release/linux/rapidr-ide.desktop "$stage/share/applications/"
    for size in 48 128 256; do
        mkdir -p "$stage/share/icons/hicolor/${size}x${size}/apps" "$stage/share/icons/hicolor/${size}x${size}/mimetypes"
        cp "tools/release/icons/rapidr-$size.png" "$stage/share/icons/hicolor/${size}x${size}/apps/rapidr.png"
        cp "tools/release/icons/rapidr-doc-$size.png" "$stage/share/icons/hicolor/${size}x${size}/mimetypes/application-x-rapidr-bytecode.png"
        cp "tools/release/icons/rapidr-doc-$size.png" "$stage/share/icons/hicolor/${size}x${size}/mimetypes/text-x-rapidr.png"
        cp "tools/release/icons/rapidr-doc-$size.png" "$stage/share/icons/hicolor/${size}x${size}/mimetypes/text-x-rapidq-basic.png"
    done
    return 0
}

tarball() {
    local arch="$1" kind="$2" name="$3"
    local top="$W/tar/$name"
    mkdir -p "$W/tar"
    cp -R "$W/$arch-$kind" "$top"
    desktop_files "$top" "$kind"
    cp tools/release/linux/install.sh tools/release/linux/uninstall.sh "$top/"
    chmod +x "$top/install.sh" "$top/uninstall.sh"
    tar -C "$W/tar" --owner=0 --group=0 -czf "$OUT/$name.tar.gz" "$name"
    rm -rf "$top"
    echo "wrote $OUT/$name.tar.gz ($(du -h "$OUT/$name.tar.gz" | cut -f1))"
}

# A .deb: /usr/bin/rapidr and its home /usr/lib/rapidr (the CLI's rule:
# <exe>/../lib/rapidr); dpkg's triggers update the MIME, desktop and icon
# databases. The SDK replaces the runtime (both have /usr/bin/rapidr).
deb() {
    local arch="$1" kind="$2" pkg="$3" desc="$4" debarch
    case "$arch" in x86_64) debarch=amd64 ;; aarch64) debarch=arm64 ;; esac
    local root="$W/deb-$pkg"
    mkdir -p "$root/usr" "$root/DEBIAN"
    cp -R "$W/$arch-$kind/bin" "$W/$arch-$kind/lib" "$root/usr/"
    mkdir -p "$root/usr/share/doc/$pkg"
    cp -R "$W/$arch-$kind/share/doc/rapidr/." "$root/usr/share/doc/$pkg/"
    cp LICENSE "$root/usr/share/doc/$pkg/copyright"
    desktop_files "$root/usr" "$kind"
    sed -i "s|@BIN@|/usr/bin|g" "$root"/usr/share/applications/*.desktop
    local other="rapidr-runtime"; [ "$pkg" = rapidr-runtime ] && other="rapidr"
    cat > "$root/DEBIAN/control" <<EOF
Package: $pkg
Version: $VERSION
Architecture: $debarch
Maintainer: RapidR <https://github.com/iBobX/RapidR>
Installed-Size: $(du -sk "$root/usr" | cut -f1)
Depends: $(depends "$root/usr/bin/rapidr")
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
    dpkg-deb --root-owner-group -Zxz --build "$root" "$OUT/${pkg}_${VERSION}_${debarch}.deb" >/dev/null
    rm -rf "$root"
    echo "wrote $OUT/${pkg}_${VERSION}_${debarch}.deb ($(du -h "$OUT/${pkg}_${VERSION}_${debarch}.deb" | cut -f1))"
}

for ARCH in $ARCHS; do
    T="$ARCH-unknown-linux-gnu"
    step "build $T (glibc $GLIBC)"
    # (pkg-config finds that architecture's libraries)
    export PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_SYSROOT_DIR=/
    export PKG_CONFIG_LIBDIR="/usr/lib/$ARCH-linux-gnu/pkgconfig:/usr/share/pkgconfig"
    export CARGO_PROFILE_RELEASE_STRIP=symbols
    cargo zigbuild -q --locked --release --target "$T.$GLIBC" -p rapidr-cli
    cargo zigbuild -q --locked --profile runner --target "$T.$GLIBC" -p rapidr-runner-stub --bin rapidrintr-runner
    for b in "$TD/$T/release/rapidr" "$TD/$T/runner/rapidrintr-runner"; do
        g="$(newest_glibc "$b")"
        [ "$(printf '%s\n%s\n' "$g" "$GLIBC" | sort -V | tail -1)" = "$GLIBC" ] || die "$b needs glibc $g, newer than $GLIBC"
        echo "  $(basename "$b"): glibc ≤ $g; needs $(needed "$b" | paste -sd' ' -)"
    done

    step "stage $ARCH"
    python3 tools/release/stage.py --kind sdk --os linux --out "$W/$ARCH-sdk" --bin "$TD/$T/release" --home "$W/home" \
        --runner "linux-$ARCH=$TD/$T/runner" --web "$PREP/web-runtime" --ide "$PREP/rapidr-ide.rrbc"
    python3 tools/release/stage.py --kind runtime --os linux --out "$W/$ARCH-runtime" --bin "$TD/$T/release" --version "$VERSION" --rust "$RUST"

    step "packages $ARCH"
    tarball "$ARCH" sdk "rapidr-$VERSION-linux-$ARCH"
    tarball "$ARCH" runtime "rapidr-runtime-$VERSION-linux-$ARCH"
    deb "$ARCH" sdk rapidr "RapidR — BASIC compiler, interpreter and IDE (SDK)"
    deb "$ARCH" runtime rapidr-runtime "RapidR Runtime — runs RapidR and RapidQ programs"
    rm -rf "$W/$ARCH-sdk" "$W/$ARCH-runtime"
done
