#!/usr/bin/env python3
"""The frame benchmark in pandas: the same table and the same operations as
RapidR's frame_bench (tools/bench/README.md), each timed as the median of a
few runs, for comparison only (pandas is never a dependency of RapidR).

    cargo run --release -p rapidr-value --example frame_bench -- 1000000 --write-csv tests/results/bench
    python3 tools/bench/bench_pandas.py [rows] [--runs N]

Loading reads the CSV from memory (bytes already read), as frame_bench
does; saving writes CSV text to memory.
"""

import io
import os
import statistics
import sys
import time

try:
    import pandas as pd
except ImportError:
    print("pandas: not installed here - skipped (pip install pandas to compare)")
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
    depts = pd.read_csv(os.path.join(data, "depts.csv"))
    print(f"pandas {pd.__version__}, {rows} rows ({len(text) / 1e6:.1f} MB of CSV), median of {runs} runs")

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

    base = timed("load", "load (CSV text)", lambda: pd.read_csv(io.BytesIO(text)))
    assert len(base) == rows
    timed("filter", "filter salary > 75000", lambda: base[base["salary"] > 75000])
    timed("sort", "sort by salary, highest first", lambda: base.sort_values("salary", ascending=False, kind="stable"))
    # (RapidR's group-by means every other column; name isn't a number: null there)
    timed("group-by", "group-by dept, mean", lambda: base.groupby("dept", sort=True)[["id", "salary", "years"]].mean())
    timed("join", "join dept (left)", lambda: base.merge(depts, on="dept", how="left"))
    timed("describe", "describe", lambda: base.describe())
    timed("save", "save (CSV text)", lambda: base.to_csv(index=False))

    def pipeline():
        df = pd.read_csv(io.BytesIO(text))
        df = df[df["salary"] > 75000]
        df = df.sort_values("salary", ascending=False, kind="stable")
        return df.groupby("dept", sort=True)[["id", "salary", "years"]].mean()

    out = timed("pipeline", "pipeline: load+filter+sort+group-by", pipeline)
    print(out.to_string())
    for label, ms in results:
        print(f"#{label} {ms:.1f}")


if __name__ == "__main__":
    main()
