#!/usr/bin/env bash
# tests/full_matrix.sh — build every examples/*.rr in both
# **compiled** (Rust-codegen) and **interpreted** (bytecode + stub)
# modes, then run console examples and diff stdout. Emits a markdown
# matrix table at tests/results/matrix.md.
#
# Usage: ./tests/full_matrix.sh
#
# Times out long builds (3 min cap per cargo invocation) and runs
# (5 sec cap per console exe). GUI/WEB examples are build-only.

set -u
cd "$(dirname "$0")/.."

# Share a single cargo target dir across every per-example *_rust crate
# so FLTK + friends compile once instead of N times. Caller can override.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(pwd)/target/matrix-shared}"
mkdir -p "$CARGO_TARGET_DIR"


# Bash 3.2 (macOS default) lacks associative arrays. Use a plain
# space-separated string of "skip-run" example paths.
SKIP_RUN_LIST=" examples/demo_chat_server.rr examples/demo_chat_client.rr examples/demo_http.rr examples/demo_mysql.rr examples/test_mysql.rr examples/demo_sqlite.rr examples/test_sqlite_events.rr examples/test_dll.rr examples/test_file_io.rr examples/ide.rr examples/test_create.rr examples/test_advanced.rr examples/gui_app.rr examples/test_canvas.rr examples/test_phase18.rr examples/test_gui_phase18.rr examples/test_gui_quick.rr examples/test_new_components.rr examples/test_system_components.rr examples/test_font_properties.rr examples/test_multiform.rr examples/test_form_parent.rr "
should_skip_run() {
    case "$SKIP_RUN_LIST" in
        *" $1 "*) return 0;;
        *) return 1;;
    esac
}


CLI="./target/release/rapidr"
RESULTS="tests/results"
LOGS="$RESULTS/logs"
mkdir -p "$LOGS"
MATRIX="$RESULTS/matrix.md"

if [[ ! -x "$CLI" ]]; then
    echo "Building rapidr CLI (release)..."
    cargo build -p rapidr-cli --release >/dev/null 2>&1 || { echo "FAIL: cargo build rapidr-cli"; exit 1; }
fi

# Ensure the runner stub is built (release).
if [[ ! -x ./target/release/rapidrintr-runner ]]; then
    echo "Building rapidrintr-runner stub (release)..."
    cargo build -p rapidr-runner-stub --release >/dev/null 2>&1 || { echo "FAIL: cargo build rapidr-runner-stub"; exit 1; }
fi

# `gtimeout` from coreutils on macOS, plain `timeout` on Linux.
if command -v gtimeout >/dev/null 2>&1; then
    TIMEOUT=gtimeout
elif command -v timeout >/dev/null 2>&1; then
    TIMEOUT=timeout
else
    TIMEOUT=""
fi
run_with_timeout() {
    local secs=$1; shift
    if [[ -n "$TIMEOUT" ]]; then
        "$TIMEOUT" "$secs" "$@"
    else
        "$@"
    fi
}

# Examples that need network / external services / interactive input —
# build-only (do not run, even if console). See SKIP_RUN_LIST above.

apptype_of() {
    local f=$1
    local at
    at=$(grep -m1 '^\$APPTYPE' "$f" 2>/dev/null | awk '{print $2}')
    echo "${at:-CONSOLE}"
}

# Globals updated by per-example test routine
COMPILED_RES=""
INTERP_RES=""
RUN_RES=""
NOTE=""

# Build a single example in compiled mode. Sets COMPILED_RES + writes
# the produced binary into examples/<stem>.compiled (renamed from the
# default `examples/<stem>` so it does not clash with --interp output).
build_compiled() {
    local f=$1 stem=$2 apptype=$3
    local log="$LOGS/${stem}.compiled.log"
    rm -f "examples/${stem}" "examples/${stem}.compiled"
    local extra=""
    [[ "$apptype" == "WEB" ]] && extra="--web"
    # Use --debug so the matrix completes in a sane wall-clock time
    # (a release build still has to LTO every per-example crate).
    if run_with_timeout 600 "$CLI" build "$f" $extra --debug >"$log" 2>&1; then
        if [[ "$apptype" == "WEB" ]]; then
            # Web compiled produces a <stem>_web/ directory.
            if [[ -d "examples/${stem}_web" ]]; then
                COMPILED_RES="PASS"
            else
                COMPILED_RES="FAIL"
                NOTE="${NOTE}; compiled-web: no _web dir"
            fi
        else
            if [[ -x "examples/${stem}" ]]; then
                mv "examples/${stem}" "examples/${stem}.compiled"
                COMPILED_RES="PASS"
            else
                COMPILED_RES="FAIL"
                NOTE="${NOTE}; compiled: no binary"
            fi
        fi
    else
        COMPILED_RES="FAIL"
        local last
        last=$(tail -3 "$log" | tr '\n' ' ' | tr -s ' ' | cut -c1-80)
        NOTE="${NOTE}; compiled: ${last}"
    fi
}

# Build a single example in interpreted mode.
build_interp() {
    local f=$1 stem=$2 apptype=$3
    local log="$LOGS/${stem}.interp.log"
    rm -f "examples/${stem}" "examples/${stem}.interp" "examples/${stem}-web.zip"
    local extra=""
    [[ "$apptype" == "WEB" ]] && extra="--web"
    if run_with_timeout 60 "$CLI" build "$f" $extra --interp >"$log" 2>&1; then
        if [[ "$apptype" == "WEB" ]]; then
            if [[ -f "examples/${stem}-web.zip" ]]; then
                INTERP_RES="PASS"
            else
                INTERP_RES="FAIL"
                NOTE="${NOTE}; interp-web: no zip"
            fi
        else
            if [[ -x "examples/${stem}" ]]; then
                mv "examples/${stem}" "examples/${stem}.interp"
                INTERP_RES="PASS"
            else
                INTERP_RES="FAIL"
                NOTE="${NOTE}; interp: no binary"
            fi
        fi
    else
        INTERP_RES="FAIL"
        local last
        last=$(tail -3 "$log" | tr '\n' ' ' | tr -s ' ' | cut -c1-80)
        NOTE="${NOTE}; interp: ${last}"
    fi
}

# Run both binaries (console only) and compare stdout.
run_and_diff() {
    local stem=$1
    local c_out="$LOGS/${stem}.compiled.out"
    local i_out="$LOGS/${stem}.interp.out"
    local diff_log="$LOGS/${stem}.diff"

    if [[ "$COMPILED_RES" != "PASS" || "$INTERP_RES" != "PASS" ]]; then
        RUN_RES="N/A"
        return
    fi

    run_with_timeout 5 "examples/${stem}.compiled" >"$c_out" 2>&1
    local c_status=$?
    run_with_timeout 5 "examples/${stem}.interp" >"$i_out" 2>&1
    local i_status=$?

    # Normalise (strip trailing whitespace) and compare; emit unified
    # diff into diff log.
    if diff -bw "$c_out" "$i_out" >"$diff_log" 2>&1; then
        RUN_RES="MATCH"
    else
        local lines
        lines=$(wc -l <"$diff_log" | tr -d ' ')
        RUN_RES="DIFF (${lines}L)"
    fi
    [[ $c_status -ne 0 ]] && NOTE="${NOTE}; compiled-rc=${c_status}"
    [[ $i_status -ne 0 ]] && NOTE="${NOTE}; interp-rc=${i_status}"
}

# Header
{
    printf '# RapidR Build Matrix\n\n'
    printf 'Generated: %s\n\n' "$(date)"
    printf '| Example | APPTYPE | Compiled | Interpreted | stdout diff | Notes |\n'
    printf '|---|---|---|---|---|---|\n'
} > "$MATRIX"

total=0; compiled_pass=0; interp_pass=0; matches=0
for f in examples/*.rr; do
    total=$((total+1))
    stem=$(basename "${f%.rr}")
    apptype=$(apptype_of "$f")
    NOTE=""

    build_compiled "$f" "$stem" "$apptype"
    build_interp "$f" "$stem" "$apptype"

    if [[ "$apptype" == "CONSOLE" ]] && ! should_skip_run "$f"; then
        run_and_diff "$stem"
    else
        RUN_RES="—"
    fi

    [[ "$COMPILED_RES" == "PASS" ]] && compiled_pass=$((compiled_pass+1))
    [[ "$INTERP_RES"   == "PASS" ]] && interp_pass=$((interp_pass+1))
    [[ "$RUN_RES"      == "MATCH" ]] && matches=$((matches+1))

    note_clean=$(echo "$NOTE" | sed 's/^; //; s/|/\\|/g')
    printf '| %s | %s | %s | %s | %s | %s |\n' \
        "$stem" "$apptype" "$COMPILED_RES" "$INTERP_RES" "$RUN_RES" "$note_clean" >> "$MATRIX"

    printf '%-40s %-7s C:%-4s I:%-4s D:%-12s\n' \
        "$stem" "$apptype" "$COMPILED_RES" "$INTERP_RES" "$RUN_RES"
done

{
    printf '\n## Summary\n\n'
    printf -- '- Total examples: %d\n' "$total"
    printf -- '- Compiled builds: %d / %d\n' "$compiled_pass" "$total"
    printf -- '- Interpreted builds: %d / %d\n' "$interp_pass" "$total"
    printf -- '- Console stdout matches: %d\n' "$matches"
    printf '\nLogs: `%s/`\n' "$LOGS"
} >> "$MATRIX"

echo
echo "Matrix written to $MATRIX"
echo "Compiled: $compiled_pass/$total — Interp: $interp_pass/$total — Matches: $matches"
