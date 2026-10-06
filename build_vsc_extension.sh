#!/usr/bin/env bash
# The VS Code extension (utilities/vscodeext/rapidr): a .vsix, versioned as
# RapidR (Cargo.toml's [workspace.package] version).
#
#   ./build_vsc_extension.sh [package]   utilities/vscodeext/rapidr/dist/rapidr-<ver>.vsix
#   ./build_vsc_extension.sh test        its tests (unit, then VS Code's integration
#                                        tests against target/debug/rapidr or $RAPIDR_PATH)
#
# Needs Node.js 22 or newer (npm). Publishing to the Marketplace and Open VSX
# is done by hand, with your own accounts: docs/vscode-publishing.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$ROOT/utilities/vscodeext/rapidr"
ACTION="${1:-package}"

command -v npm >/dev/null || { echo "error: npm is needed (Node.js 22 or newer: https://nodejs.org)" >&2; exit 1; }
cd "$EXT_DIR"
if [ -f package-lock.json ]; then npm ci --no-audit --no-fund; else npm install --no-audit --no-fund; fi

case "$ACTION" in
    package)
        npm run package
        ;;
    test)
        npm test
        ;;
    *)
        echo "Usage: ./build_vsc_extension.sh [package|test]" >&2
        exit 2
        ;;
esac
