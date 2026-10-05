#!/bin/bash
# The Linux packages, made on the Mac in Docker (Ubuntu 24.04: the image of
# tools/linux/Dockerfile), from prepare.sh's src.tar:
#
#   tools/release/linux-docker.sh aarch64     native on Apple silicon
#   tools/release/linux-docker.sh x86_64      emulated (Rosetta / QEMU): slow
#
# The builds stay in Docker volumes (rapidr-release-target-<arch>, about 4 GB
# each; `docker volume rm` them after the release); the packages land in
# dist/<ver>/out/.
set -euo pipefail
source "$(dirname "$0")/common.sh"
cd "$ROOT"
need docker "Docker Desktop"
ARCH="${1:-aarch64}"
case "$ARCH" in
    aarch64) PLATFORM=linux/arm64 IMAGE=rapidr-release-arm64 ;;
    x86_64) PLATFORM=linux/amd64 IMAGE=rapidr-release-amd64 ;;
    *) die "aarch64 or x86_64" ;;
esac
[ -f "$PREP/src.tar" ] || die "run tools/release/prepare.sh first"
docker image inspect "$IMAGE" >/dev/null 2>&1 || docker build --platform "$PLATFORM" -t "$IMAGE" tools/linux
docker run --rm --platform "$PLATFORM" \
    -v "$DIST":/dist \
    -v "rapidr-release-target-$ARCH":/target \
    -v "rapidr-linux-registry-$ARCH":/root/.cargo/registry \
    -e RAPIDR_DIST=/dist -e CARGO_TARGET_DIR=/target -e CARGO_BUILD_JOBS=3 \
    "$IMAGE" bash -c '
        set -e
        rm -rf /src && mkdir -p /src && tar -x -C /src -f /dist/prep/src.tar && cd /src
        bash tools/release/linux.sh'
