# Sourced (not run) by jit-check.sh, smoke.sh and the build matrix's Linux job: build and run
# x86_64 Linux programs AS x86_64 in an arm64 Ubuntu VM that has Rosetta for Linux (Parallels).
#
#   . tools/release/linux/x86-env.sh
#   x86_available                  yes (0) when this VM has what follows
#   x86_env <folder for the wrappers>   exports the environment of an x86_64 build
#
# What it needs: the x86_64 Rust host in $RAPIDR_X86_RUST (default ~/rust-x86_64: `setup-tools.sh
# x86rust`, rustup's own, beside the arm64 Rust, which it does not touch), Zig and cargo-zigbuild
# (`setup-tools.sh user`: the release build links with them), and the amd64 -dev libraries
# (`setup-tools.sh system`).
#
# Why each part (found 2026-11-05):
# - ROSETTA_DISABLE_AOT=1: without it every program that loads rustc's 150 MB librustc_driver.so
#   (rustc, rustdoc) dies with SIGSEGV inside Rosetta's own runtime (a fault at 0x8000....., just after
#   it maps its translation cache). cargo, rustup and the programs Rust builds run fine either way.
#   Rosetta translates at run time instead: rustc is slower, the programs it builds are not changed.
# - the linker: an x86_64 rustc calls `cc`, which here is arm64's gcc (and there is no x86_64 one:
#   nothing is installed from apt for this). Zig links x86_64 with the amd64 libraries (glibc 2.31
#   baseline, as the release build does); the wrappers are what cargo-zigbuild makes, without it.
# - pkg-config finds the amd64 libraries (alsa, fontconfig, freetype, openssl), as linux.sh's cross
#   builds do. A real x86_64 PC needs none of this: its own gcc and pkg-config are x86_64.
# A program built this way is a real x86_64 program, run by Rosetta.

X86_HOME="${RAPIDR_X86_RUST:-$HOME/rust-x86_64}"

x86_available() {
    [ "$(uname -m)" = aarch64 ] && [ -x "$X86_HOME/cargo/bin/rustup" ] &&
        grep -qs '^enabled' /proc/sys/fs/binfmt_misc/RosettaLinux &&
        [ -x "$HOME/.cargo/bin/zig" ] && [ -x "$HOME/.cargo/bin/cargo-zigbuild" ]
}

x86_env() {
    local w="$1" glibc="${GLIBC:-2.31}" tool
    mkdir -p "$w"
    for tool in cc c++; do
        printf '#!/bin/sh\nexport CARGO_ZIGBUILD_ZIG_COMMAND=%s\nexec %s zig %s -- -fno-sanitize=all -target x86_64-linux-gnu.%s "$@"\n' \
            "$HOME/.cargo/bin/zig" "$HOME/.cargo/bin/cargo-zigbuild" "$tool" "$glibc" > "$w/$tool"
        chmod +x "$w/$tool"
    done
    export ROSETTA_DISABLE_AOT=1
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$w/cc"
    export CC_x86_64_unknown_linux_gnu="$w/cc" CXX_x86_64_unknown_linux_gnu="$w/c++"
    export PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_SYSROOT_DIR=/
    export PKG_CONFIG_LIBDIR="/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig"
    # (-idirafter: Zig's compiler doesn't look in the multiarch include folder, where OpenSSL's
    # opensslconf.h is; searched last, so the C library's headers stay Zig's)
    export CFLAGS_x86_64_unknown_linux_gnu="-idirafter /usr/include/x86_64-linux-gnu"
}

# (the build's environment ends where the build does: the checks after it run the package as it is)
x86_env_unset() {
    unset ROSETTA_DISABLE_AOT CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER CC_x86_64_unknown_linux_gnu CXX_x86_64_unknown_linux_gnu \
        PKG_CONFIG_ALLOW_CROSS PKG_CONFIG_SYSROOT_DIR PKG_CONFIG_LIBDIR CFLAGS_x86_64_unknown_linux_gnu
}
