#!/bin/bash
# The VS Code extension as a .vsix: dist/<version>/out/rapidr-<version>.vsix
# (finish.sh's SHA256SUMS covers it; it goes on the GitHub release beside the
# installers). Publishing it to the VS Code Marketplace and Open VSX is by
# hand, with the user's accounts: docs/vscode-publishing.md.
#
#   tools/release/vscode.sh
#
# Needs Node.js 22 or newer (npm). The extension's version is RapidR's
# (scripts/sync-version.js writes Cargo.toml's into package.json).
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT/utilities/vscodeext/rapidr"
need npm "Node.js 22 or newer: https://nodejs.org"

step "the VS Code extension ($VERSION)"
npm ci --no-audit --no-fund
node scripts/sync-version.js --check || die "package.json's version isn't RapidR's: npm run version:sync, then commit"
NAME="rapidr-$VERSION.vsix"
rm -f "$OUT/$NAME"
node scripts/package.js --out "$OUT/$NAME"
# (nothing but the bundle and the extension's assets: no sources, tests or node_modules)
LIST="$(unzip -Z1 "$OUT/$NAME")"
if grep -E '^extension/(src|test|scripts|node_modules)/' <<<"$LIST"; then
    die "$NAME carries sources, tests or node_modules"
fi
for f in extension/dist/extension.js extension/LICENSE.txt extension/THIRD_PARTY_NOTICES.md extension/readme.md; do
    grep -qix "$f" <<<"$LIST" || die "$NAME has no $f"      # (vsce writes readme.md, changelog.md)
done
echo "wrote $OUT/$NAME ($(du -h "$OUT/$NAME" | cut -f1))"
