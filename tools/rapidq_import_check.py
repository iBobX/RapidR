#!/usr/bin/env python3
"""The RapidQ importer's proof on a corpus (crates/rapidr-import).

Usage (repo root, after `cargo build --release -p rapidr-cli`):
    python3 tools/rapidq_import_check.py <corpus-dir> --include <RapidQ include dir> [-o OUT] [--json out.json]

1. `rapidr import-rapidq <corpus-dir> -o OUT --include …` converts a copy of
   the whole corpus (.bas, .rqw, .rqb, .rq and their includes) and compiles
   every program and its copy to bytecode: they must be identical, byte for
   byte (the report's "Programs" table).
2. Each copy that compiled is compiled again on its own — no RapidQ include
   folder, from the copy's top folder — the way a user builds the copy:
   RapidQ's RAPIDQ.INC replaced by RapidR's constants, the includes the
   import copied.

The corpus is only read. Prints the numbers per file extension.
"""

import argparse
import collections
import json
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAPIDR = os.environ.get("RAPIDR_BIN", os.path.join(ROOT, "target", "release", "rapidr"))
ROW = re.compile(r"^\| `(?P<prog>[^`]+)` \| (?P<result>.*) \|$")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus")
    ap.add_argument("--include", required=True)
    ap.add_argument("-o", "--out", default=None)
    ap.add_argument("--json")
    args = ap.parse_args()
    out = args.out or tempfile.mkdtemp(prefix="rapidr-import-check-")
    env = dict(os.environ)
    env.pop("RAPIDR_INCLUDE_PATH", None)
    scratch = tempfile.mkdtemp(prefix="rapidr-import-env-")
    env.setdefault("RAPIDR_PRINT_TO", os.path.join(scratch, "print.out"))
    env.setdefault("RAPIDR_REGISTRY", os.path.join(scratch, "registry"))
    proc = subprocess.run([RAPIDR, "import-rapidq", args.corpus, "-o", out, "--include", args.include], capture_output=True, text=True, env=env)
    print(proc.stdout.strip())
    if proc.stderr.strip():
        print(proc.stderr.strip(), file=sys.stderr)
    report = open(os.path.join(out, "rapidr-import-report.md"), encoding="utf-8").read()
    rows = []
    in_programs = False
    for line in report.splitlines():
        if line.startswith("## Programs"):
            in_programs = True
            continue
        if in_programs and line.startswith("## "):
            break
        m = ROW.match(line)
        if in_programs and m:
            rows.append((m.group("prog"), m.group("result")))
    by_ext = collections.defaultdict(collections.Counter)
    standalone_fail = []
    for prog, result in rows:
        ext = os.path.splitext(prog)[1].lower() or "(none)"
        if result.startswith("yes"):
            kind = "identical"
        elif "differs" in result:
            kind = "different"
        elif "copy doesn't compile" in result:
            kind = "copy fails"
        else:
            kind = "original doesn't compile"
        by_ext[ext][kind] += 1
        by_ext["all"][kind] += 1
        if kind == "identical":
            with tempfile.TemporaryDirectory() as tmp:
                p = subprocess.run([RAPIDR, "build-bc", os.path.join(out, prog), "-o", os.path.join(tmp, "x.rrbc")], cwd=out, capture_output=True, text=True, errors="replace", env=env)
            ok = p.returncode == 0
            by_ext[ext]["standalone ok" if ok else "standalone fails"] += 1
            by_ext["all"]["standalone ok" if ok else "standalone fails"] += 1
            if not ok:
                standalone_fail.append((prog, (p.stderr or p.stdout).strip().splitlines()[:1]))
    print()
    kinds = ["identical", "different", "copy fails", "original doesn't compile", "standalone ok", "standalone fails"]
    print(f"{'ext':8} " + " ".join(f"{k:>24}" for k in kinds))
    for ext in sorted(by_ext, key=lambda e: (e == "all", e)):
        print(f"{ext:8} " + " ".join(f"{by_ext[ext][k]:>24}" for k in kinds))
    for prog, err in standalone_fail:
        print(f"standalone fails: {prog}: {err}")
    if args.json:
        json.dump({e: dict(c) for e, c in by_ext.items()}, open(args.json, "w"), indent=1)
    bad = by_ext["all"]["different"] + by_ext["all"]["copy fails"]
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
