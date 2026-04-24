#!/usr/bin/env bash
# Launch the RapidR build server.
# Default port 8095, override with RAPIDR_BUILDSERVER_PORT.
set -e
cd "$(dirname "$0")"
export RAPIDR_REPO_ROOT="$PWD"
exec target/release/rapidr-buildserver
