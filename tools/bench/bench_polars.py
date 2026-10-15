#!/usr/bin/env python3
"""The frame benchmark in Polars: the same table and the same operations as
RapidR's frame_bench (tools/bench/README.md), each timed as the median of a
few runs, for comparison only (Polars is never a dependency of RapidR).

    cargo run --release -p rapidr-value --example frame_bench -- 1000000 --write-csv tests/results/bench
    python3 tools/bench/bench_polars.py [rows] [--runs N]

Loading reads the CSV from memory (bytes already read), as frame_bench
does; saving writes CSV text to memory.
"""

import io
import os
import statistics
import sys
import time

try:
    import polars as pl
except ImportError:
    print("polars: not installed here - skipped (pip install polars to compare)")
    sys.exit(0)

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def main():
    rows, runs = 1_000_000, 3
    args = sys.argv[1:]
    while args:
        a = args.pop(0)
        if a == "--runs" and args:
            runs = max(1, int(args.pop(0)))
        else:
            rows = int(a.replace("_", ""))
    data = os.path.join(ROOT, "tests", "results", "bench")
    people = os.path.join(data, f"people_{rows}.csv")
    if not os.path.exists(people):
        print(f"no {people} yet: cargo run --release -p rapidr-value --example frame_bench -- {rows} --write-csv tests/results/bench")
        sys.exit(2)
    with open(people, "rb") as f:
        text = f.read()
    depts = pl.read_csv(os.path.join(data, "depts.csv"))
    print(f"polars {pl.__version__}, {rows} rows ({len(text) / 1e6:.1f} MB of CSV), median of {runs} runs")

    results = []

    def timed(label, what, fn):
        times = []
        out = None
        for _ in range(runs):
            t = time.perf_counter()
            out = fn()
            times.append((time.perf_counter() - t) * 1e3)
        ms = statistics.median(times)
        results.append((label, ms))
        print(f"  {what:<34} {ms:>9.1f} ms")
        return out

    means = [pl.col("id").mean(), pl.col("salary").mean(), pl.col("years").mean()]
    base = timed("load", "load (CSV text)", lambda: pl.read_csv(io.BytesIO(text)))
    assert base.height == rows
    timed("filter", "filter salary > 75000", lambda: base.filter(pl.col("salary") > 75000))
    timed("sort", "sort by salary, highest first", lambda: base.sort("salary", descending=True, maintain_order=True))
    timed("group-by", "group-by dept, mean", lambda: base.group_by("dept").agg(means).sort("dept"))
    timed("join", "join dept (left)", lambda: base.join(depts, on="dept", how="left"))
    timed("describe", "describe", lambda: base.describe())
    timed("save", "save (CSV text)", lambda: base.write_csv())

    def pipeline():
        df = pl.read_csv(io.BytesIO(text))
        df = df.filter(pl.col("salary") > 75000)
        df = df.sort("salary", descending=True, maintain_order=True)
        return df.group_by("dept").agg(means).sort("dept")

    out = timed("pipeline", "pipeline: load+filter+sort+group-by", pipeline)
    print(out)
    for label, ms in results:
        print(f"#{label} {ms:.1f}")


if __name__ == "__main__":
    main()
