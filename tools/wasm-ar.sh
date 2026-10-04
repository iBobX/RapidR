#!/bin/sh
# The archiver for C compiled to wasm — SQLite in the web runtime (the cc
# crate's AR_wasm32_unknown_unknown, which tools/build_web_artifacts.sh and
# `rapidr build --web` set unless it's set already).
#
# llvm-ar when there is one. Otherwise the system's ar, without the symbol
# index it can't build for wasm objects (Apple's then drops the objects
# altogether): rust-lld reads an archive's members itself.
if command -v llvm-ar >/dev/null 2>&1; then
    exec llvm-ar "$@"
fi
op="$1"
shift
case "$op" in
    # (the index alone, as ranlib: none)
    s) exit 0 ;;
    *) exec ar "$(printf %s "$op" | tr -d s)S" "$@" ;;
esac
