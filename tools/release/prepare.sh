#!/bin/bash
# The first step of a release, on the Mac, from a clean, committed tree:
#
#   dist/<ver>/prep/src.tar           the commit (git archive): every platform
#                                     builds and assembles its home from it
#   dist/<ver>/prep/commit            its hash
#   dist/<ver>/prep/web-runtime/      the web interpreter (rapidrintr.js + .wasm)
#   dist/<ver>/prep/rapidr-ide.rrbc   the IDE, compiled
#   dist/<ver>/out/                   the web bundle, the VS Code extension (.vsix),
#                                     the SBOM, RELEASE_NOTES.md
#
#   tools/release/prepare.sh
#   RAPIDR_RELEASE_DIRTY=1 tools/release/prepare.sh    (trying the scripts out:
#                                     the archive is still HEAD, without the changes)
#
# Needs: Rust, wasm-pack, python3, git, zip, Node.js 22+ (npm). About 3 GB of
# disk for the web interpreter's build (target/wasm32-unknown-unknown); prep/
# is ~20 MB.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
need python3 "the release scripts"
need cargo "https://rustup.rs"
need wasm-pack "cargo install wasm-pack"
need npm "the VS Code extension: Node.js 22 or newer"

if [ -n "$(git status --porcelain --untracked-files=no)" ] && [ -z "${RAPIDR_RELEASE_DIRTY:-}" ]; then
    die "the tree has uncommitted changes: a release is built from a commit (RAPIDR_RELEASE_DIRTY=1 to try anyway)"
fi
rm -rf "$PREP" && mkdir -p "$PREP"

step "the source ($(git rev-parse --short HEAD))"
git archive --format=tar -o "$PREP/src.tar" HEAD
git rev-parse HEAD > "$PREP/commit"

if [ -f tools/fonts.py ]; then
    step "the web's fallback fonts' sources (pinned, checked; fetched once into target/fonts-src)"
    python3 tools/fonts.py fetch || die "the Noto fonts' sources couldn't be fetched (no network, no cache in target/fonts-src): a release needs the full set"
fi

step "the web interpreter (rapidrintr.js + .wasm, the fallback fonts)"
tools/build_web_artifacts.sh
mkdir -p "$PREP/web-runtime"
cp target/web/rapidrintr.js target/web/rapidrintr_bg.wasm "$PREP/web-runtime/"
if [ -f tools/fonts.py ]; then
    [ -f target/web/fonts/index.json ] || die "tools/build_web_artifacts.sh made no target/web/fonts"
    cp -R target/web/fonts "$PREP/web-runtime/fonts"
fi

step "the IDE's bytecode"
cargo build -q --release --locked -p rapidr-cli
target/release/rapidr build-bc ide/studio.rr -o "$PREP/rapidr-ide.rrbc"

step "the web bundle"
tools/release/web.sh

step "the VS Code extension"
tools/release/vscode.sh

step "SBOM, release notes"
python3 tools/release/sbom.py --out "$OUT/rapidr-$VERSION.cdx.json"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@COMMIT@/$(git rev-parse --short HEAD)/g" docs/release-notes-template.md > "$OUT/RELEASE_NOTES.md"
du -sh "$PREP"/* | sed 's/^/  /'
