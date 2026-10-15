#!/usr/bin/env bash
# compare.sh: the frame benchmark (tools/bench/README.md) on everything
# available here, then one table.
#
#   tools/bench/compare.sh [rows]        (default 1000000; run from anywhere)
#   RUNS=9 tools/bench/compare.sh        (the median of 9 runs; default 3)
#
# Runs RapidR's engine (frame_bench: all cores, and one thread - what a web
# page gets), the same work as a RapidR program (bench_rapidr.rr, interpreted
# with ./rapidr run and built native with ./rapidr build, when ./rapidr
# exists: cargo build --release -p rapidr-cli && cp target/release/rapidr
# rapidr), pandas and Polars (when installed; never installed for you).
# The data is written once to tests/results/bench/ (git-ignored).

cd "$(dirname "$0")/../.." || exit 1
ROWS="${1:-1000000}"
RUNS="${RUNS:-3}"
DATA=tests/results/bench
mkdir -p "$DATA"

echo "building frame_bench (release) ..."
cargo build --release -q -p rapidr-value --example frame_bench || exit 1
BENCH=target/release/examples/frame_bench
[ -f "$DATA/people_$ROWS.csv" ] || "$BENCH" "$ROWS" --write-csv "$DATA" || exit 1
rm -f "$DATA"/compare_*.txt

run() {
    local name=$1
    shift
    echo
    echo "== $*"
    "$@" 2>&1 | tee "$DATA/compare_$name.txt"
}

run engine "$BENCH" "$ROWS" --runs "$RUNS"
run engine1 "$BENCH" "$ROWS" --runs "$RUNS" --threads 1
if [ -x ./rapidr ]; then
    run interpreted ./rapidr run tools/bench/bench_rapidr.rr "$ROWS"
    # (a release build; cargo's files go to the build cache: RAPIDR_BUILD_CACHE)
    rm -rf "$DATA/native"
    if ./rapidr build tools/bench/bench_rapidr.rr --output "$DATA/native" >/dev/null 2>&1; then
        NATIVE=$(find "$DATA/native" -type f \( -name bench_rapidr -o -name bench_rapidr.exe \) | head -1)
        [ -n "$NATIVE" ] && run native "$NATIVE" "$ROWS"
    fi
else
    echo
    echo "(no ./rapidr: the RapidR program isn't run - cargo build --release -p rapidr-cli && cp target/release/rapidr rapidr)"
fi
run pandas python3 tools/bench/bench_pandas.py "$ROWS" --runs "$RUNS"
run polars python3 tools/bench/bench_polars.py "$ROWS" --runs "$RUNS"
rm -f "$DATA/saved_by_rapidr.csv"

# The table: an operation a row, a tool a column.
title() {
    local f="$DATA/compare_$1.txt"
    case $1 in
        engine) echo "RapidR engine, $(sed -n 's/.*, \([0-9]*\) of [0-9]* cores.*/\1/p' "$f" | head -1) threads" ;;
        engine1) echo "RapidR engine, 1 thread (as the web)" ;;
        interpreted) echo "RapidR program, rapidr run" ;;
        native) echo "RapidR program, native" ;;
        pandas) echo "pandas $(sed -n 's/^pandas \([^,]*\),.*/\1/p' "$f" | head -1)" | sed 's/ $//' ;;
        polars) echo "Polars $(sed -n 's/^polars \([^,]*\),.*/\1/p' "$f" | head -1)" | sed 's/ $//' ;;
    esac
}
cols=""
for n in engine engine1 interpreted native pandas polars; do
    [ -f "$DATA/compare_$n.txt" ] && cols="$cols $n"
done
echo
echo "$ROWS rows (tests/results/bench/people_$ROWS.csv), milliseconds, median of $RUNS runs (the RapidR program: one run):"
echo
line="| operation |"
rule="|---|"
for n in $cols; do
    line="$line $(title "$n") |"
    rule="$rule--:|"
done
echo "$line"
echo "$rule"
for label in load filter sort group-by join describe save pipeline; do
    line="| $label |"
    for n in $cols; do
        f="$DATA/compare_$n.txt"
        v=$(sed -n "s/^#$label \([0-9.]*\)$/\1/p" "$f" | head -1)
        if [ -z "$v" ]; then
            if grep -q "not installed" "$f"; then v="not installed"; else v="-"; fi
        fi
        line="$line $v |"
    done
    echo "$line"
done
