#!/usr/bin/env bash
set -euo pipefail

EXT_DIR="utilities/vscodeext/rapidr"
ROOT_DIR="$(pwd)"

# Ensure we're at the repo root
if [ ! -d "$EXT_DIR" ]; then
    echo "Error: Run this script from the RapidR project root."
    exit 1
fi

ACTION="${1:-package}"

case "$ACTION" in
    package)
        echo "Packaging RapidR VS Code extension..."
        cd "$EXT_DIR"
        vsce package
        VSIX=$(ls -t *.vsix 2>/dev/null | head -1)
        if [ -n "$VSIX" ]; then
            mv "$VSIX" "$ROOT_DIR/"
            echo "Created: $ROOT_DIR/$VSIX"
        fi
        ;;
    install)
        echo "Packaging and installing RapidR VS Code extension..."
        cd "$EXT_DIR"
        vsce package
        VSIX=$(ls -t *.vsix 2>/dev/null | head -1)
        if [ -n "$VSIX" ]; then
            mv "$VSIX" "$ROOT_DIR/"
            code --install-extension "$ROOT_DIR/$VSIX"
            echo "Installed: $VSIX"
        else
            echo "Error: No .vsix file found after packaging."
            exit 1
        fi
        ;;
    publish)
        echo "Publishing RapidR VS Code extension..."
        cd "$EXT_DIR"
        vsce publish
        ;;
    *)
        echo "Usage: ./build_vsc_extension.sh [package|install|publish]"
        echo ""
        echo "  package   Build the .vsix file (default)"
        echo "  install   Build and install into VS Code"
        echo "  publish   Publish to the VS Code Marketplace"
        exit 1
        ;;
esac
