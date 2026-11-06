#!/usr/bin/env python3
"""Two runs of tools/dll_breadth.py side by side: the programs whose outcome
differs (compiled or not, how the run ended and its message, and the end of
what it printed), then a count of each outcome in both.

    python3 tools/dll_breadth_compare.py BEFORE/results.jsonl AFTER/results.jsonl
"""
import collections
import json
import re
import sys


def load(path):
    out = {}
    try:
        for line in open(path, encoding="utf-8"):
            line = line.strip()
            if line:
                r = json.loads(line)
                out[r["prog"]] = r
    except OSError as e:
        sys.exit(f"dll_breadth_compare: {e}")
    return out


def key(r):
    if r is None:
        return ("missing",)
    if r.get("bc_build") != "ok":
        return ("compile", r.get("bc_build_error", ""))
    if r.get("compile_only"):
        return ("built only", r["compile_only"])
    # (addresses and timings differ from run to run: digits aren't compared)
    msg = re.sub(r"\d+", "#", r.get("bc_msg") or "")
    return (r.get("bc_run"), msg)


before, after = load(sys.argv[1]), load(sys.argv[2])
progs = sorted(set(before) | set(after))
changed = [p for p in progs if key(before.get(p)) != key(after.get(p))]
for p in changed:
    print(f"DIFF {p}\n     before: {key(before.get(p))}\n     after:  {key(after.get(p))}")
same_out = [p for p in progs if p not in changed and (before.get(p) or {}).get("bc_out") != (after.get(p) or {}).get("bc_out")]
for p in same_out:
    print(f"OUT  {p}: the same outcome, other output (last lines differ)")
for name, runs in (("before", before), ("after", after)):
    tally = collections.Counter(key(r)[0] for r in runs.values())
    print(f"{name}: {len(runs)} programs; " + ", ".join(f"{k} {v}" for k, v in sorted(tally.items(), key=lambda kv: str(kv[0]))))
print(f"{len(changed)} programs with another outcome, {len(same_out)} with other output")
