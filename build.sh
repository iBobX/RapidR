#!/usr/bin/env bash
set -euo pipefail

PROFILE="release"
for arg in "$@"; do
    case "$arg" in
        --debug|-d) PROFILE="debug" ;;
        --release|-r) PROFILE="release" ;;
        *) echo "Unknown option: $arg"; echo "Usage: ./build.sh [--release|-r] [--debug|-d]"; exit 1 ;;
    esac
done

echo "Building RapidR compiler ($PROFILE)..."
if [ "$PROFILE" = "release" ]; then
    cargo build --release
else
    cargo build
fi

BIN="target/$PROFILE/rapidr"
if [ -f "$BIN" ]; then
    cp "$BIN" ./rapidr
    chmod +x ./rapidr
    echo "Installed: ./rapidr"
else
    echo "Error: binary not found at $BIN"
    exit 1
fi
