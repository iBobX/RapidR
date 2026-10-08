#!/usr/bin/env bash
# esp32_check.sh: QCOMPORT against a real ESP32 board, by hand (it needs the
# board plugged in; the CI-safe tests use scripted ports instead).
#
# It runs tools/esp32_check.rr interpreted (rapidr run) and native (rapidr
# build): each lists the serial ports with their USB IDs, opens the board at
# 115200, pulses a reset through DTR / RTS and prints the board's boot log.
#
# It NEVER writes to the board: no byte is sent, only the modem lines move,
# and the reset keeps IO0 high (the board boots its own program; download
# mode and the flash are never touched). The port is closed at the end.
#
#   tools/esp32_check.sh [port] [seconds]     (default: the first USB port, 4 s)
#   RAPIDR=path/to/rapidr tools/esp32_check.sh
set -euo pipefail
cd "$(dirname "$0")/.."

RAPIDR=${RAPIDR:-}
if [ -z "$RAPIDR" ]; then
  for c in ./target/debug/rapidr ./target/release/rapidr ./rapidr; do
    [ -x "$c" ] && { RAPIDR=$c; break; }
  done
fi
[ -n "$RAPIDR" ] || { echo "no rapidr: cargo build -p rapidr-cli (or RAPIDR=…)"; exit 1; }

# (never the real printer, never the real registry)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export RAPIDR_PRINT_TO="$TMP/print.out" RAPIDR_REGISTRY="$TMP/registry"
# (a real port: no scripted ones)
unset RAPIDR_TEST_COMPORT

PORT=${1:-}
SECS=${2:-4}

status=0
check() { # name, output
  local out=$2
  echo "$out"
  if echo "$out" | grep -q "rst:0x"; then
    echo "== $1: the boot log came (rst:…)"
  else
    echo "== $1: FAILED — no boot log"; status=1
  fi
}

echo "== interpreted ($RAPIDR run)"
check interpreted "$("$RAPIDR" run tools/esp32_check.rr "$PORT" "$SECS" 2>&1)"

echo "== native ($RAPIDR build)"
if "$RAPIDR" build tools/esp32_check.rr "$TMP/native" >"$TMP/build.log" 2>&1; then
  check native "$("$TMP/native/esp32_check" "$PORT" "$SECS" 2>&1)"
else
  tail -20 "$TMP/build.log"; echo "== native: the build FAILED"; status=1
fi
exit $status
