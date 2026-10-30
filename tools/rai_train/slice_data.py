#!/usr/bin/env python3
"""A small, seeded slice of the corpus for a smoke run (or a quick experiment).

    slice_data.py --data $RAI_ROOT/data --out $RAI_ROOT/runs/smoke/data --rows 200 --max-tokens 3500

Takes rows evenly from every kind and set-up, none longer than --max-tokens (Qwen tokens),
shuffled with --seed; the valid file gets --valid rows from the corpus's valid.jsonl.
"""

from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rai_corpus import row_tokens  # noqa: E402


def kind_of(row: dict) -> str:
    sys_msg = row["messages"][0]["content"]
    profile = "mcp" if sys_msg.startswith("You are working in RapidR Studio") else "lite" if "Remember:" in sys_msg and len(sys_msg) < 9000 else "full"
    last = row["messages"][-1]
    call = last["tool_calls"][0]["function"]["name"] if last.get("tool_calls") else "answer"
    return f"{profile}/{call}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rows", type=int, default=200)
    ap.add_argument("--valid", type=int, default=20)
    ap.add_argument("--max-tokens", type=int, default=3500)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--profiles", default="lite", help="comma list of set-ups to take from: lite, full, mcp")
    a = ap.parse_args()
    data, out = Path(a.data), Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    rng = random.Random(a.seed)
    want = set(a.profiles.split(","))
    for name, n in (("train", a.rows), ("valid", a.valid)):
        rows = [json.loads(l) for l in (data / f"{name}.jsonl").open()]
        rng.shuffle(rows)
        buckets: dict[str, list[dict]] = {}
        for r in rows:
            k = kind_of(r)
            if k.split("/")[0] in want:
                buckets.setdefault(k, []).append(r)
        chosen: list[dict] = []
        keys = sorted(buckets)
        i = 0
        while len(chosen) < n and any(buckets.values()):
            k = keys[i % len(keys)]
            i += 1
            while buckets[k]:
                r = buckets[k].pop()
                if row_tokens(r) <= a.max_tokens:
                    chosen.append(r)
                    break
        rng.shuffle(chosen)
        with (out / f"{name}.jsonl").open("w") as f:
            for r in chosen:
                f.write(json.dumps(r, ensure_ascii=False) + "\n")
        print(f"{name}: {len(chosen)} rows -> {out / (name + '.jsonl')}  (" + ", ".join(f"{k} {sum(1 for r in chosen if kind_of(r) == k)}" for k in keys) + ")")
    return 0


if __name__ == "__main__":
    sys.exit(main())
