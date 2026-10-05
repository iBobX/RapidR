#!/bin/bash
# RapidR on Linux, locally (Docker; no remote CI): the committed tree (HEAD)
# built in a fresh Ubuntu container, then the conformance suite and the
# desktop GUI events — headless, and through real X11 windows on a virtual
# display (Xvfb: the winit host, AccessKit, the CPU renderer or Mesa's
# software Vulkan).
#
#   tools/linux/check.sh                 everything
#   tools/linux/check.sh gui [case…]     only the GUI events (these cases)
#
# Needs: docker; the image (docker build -t rapidr-linux tools/linux).
# Builds stay in Docker volumes (rapidr-linux-*), never in this checkout.
cd "$(dirname "$0")/../.."
docker image inspect rapidr-linux >/dev/null 2>&1 || docker build -t rapidr-linux tools/linux || exit 1
STAGE=${1:-all}; shift 2>/dev/null
CASES="$*"
docker run --rm \
  -v "$PWD":/repo:ro \
  -v rapidr-linux-target:/target \
  -v rapidr-linux-registry:/root/.cargo/registry \
  -v rapidr-linux-work:/work \
  -e STAGE="$STAGE" -e CASES="$CASES" \
  rapidr-linux bash -c '
    set -o pipefail
    git config --global --add safe.directory /repo
    rm -rf /src && mkdir -p /src && git -C /repo archive HEAD | tar -x -C /src && cd /src
    # (the suites build into tests/conformance/.work: a volume, kept between runs)
    mkdir -p /work && ln -s /work tests/conformance/.work
    export CARGO_TARGET_DIR=/target CARGO_BUILD_JOBS=3
    # (nothing a test runs may print or touch a registry outside the container)
    export RAPIDR_PRINT_TO=/tmp/prints RAPIDR_REGISTRY=/tmp/r.reg && mkdir -p /tmp/prints
    echo "== build"
    cargo build -q --release -p rapidr-cli 2>&1 | grep -E "^(error|warning: unused)" -A5 | head -30
    cp /target/release/rapidr ./rapidr || exit 1
    if [ "$STAGE" = all ]; then
      echo "== conformance"; node tests/conformance/run.mjs 2>&1 | grep -E "^FAIL|passed" | tail -15
    fi
    echo "== gui events (headless)"; node tests/native_gui_events.mjs $CASES 2>&1 | grep -E "✗|got:|GUI events" | head -40
    echo "== gui events (real X11 windows on Xvfb)"
    RAPIDR_CAPTURE_WINDOWS=1 xvfb-run -a -s "-screen 0 1280x1024x24" node tests/native_gui_events.mjs $CASES 2>&1 | grep -E "✗|got:|GUI events" | head -40
  '
