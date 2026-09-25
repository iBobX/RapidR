#!/usr/bin/env python3
"""Compile a corpus of real RapidQ programs with the bytecode compiler and
summarise what breaks, most common first.

Usage (repo root, after building ./rapidr):
    python3 tools/rapidq_corpus.py <corpus-dir> [--include <rapidq include dir>]
                                   [--json out.json] [--show N] [--grep TEXT]

Each .bas file is compiled with `rapidr build-bc`. Errors are normalised
(line/column and quoted names stripped) and grouped, so the report shows which
missing features block the most programs. `--grep` lists the files (and first
matching error) for one error pattern.
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
ERROR_RE = re.compile(r"^(?P<file>.*?):(?P<line>\d+):(?P<col>\d+): error: (?P<msg>.*)$")


def normalise(msg):
    msg = re.sub(r"'[^']*'", "'X'", msg)
    msg = re.sub(r"`[^`]*`", "`X`", msg)
    msg = re.sub(r"\b\d+\b", "N", msg)
    return msg[:140]


def quoted(msg):
    m = re.search(r"'([^']*)'", msg)
    return m.group(1) if m else None


def compile_one(path, env):
    with tempfile.TemporaryDirectory() as tmp:
        try:
            proc = subprocess.run(
                [os.path.join(ROOT, "rapidr"), "build-bc", path, "-o", os.path.join(tmp, "out.rrbc")],
                capture_output=True, text=True, errors="replace", timeout=30, env=env,
            )
        except subprocess.TimeoutExpired:
            return False, ["<timeout>"]
    if proc.returncode == 0:
        return True, []
    errors = []
    for line in (proc.stderr + proc.stdout).splitlines():
        m = ERROR_RE.match(line)
        if m:
            errors.append(m.group("msg"))
        elif line.strip() and not errors and ("error" in line.lower() or "panicked" in line):
            errors.append(line.strip()[:200])
    return False, errors or ["<failed without an error message>"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus")
    ap.add_argument("--include", help="RapidQ include directory (searched after the program's own directory)")
    ap.add_argument("--json")
    ap.add_argument("--show", type=int, default=40)
    ap.add_argument("--grep")
    args = ap.parse_args()

    env = dict(os.environ)
    if args.include:
        env["RAPIDR_INCLUDE_PATH"] = os.path.abspath(args.include)

    files = sorted(
        os.path.join(d, f)
        for d, _, fs in os.walk(args.corpus)
        for f in fs
        if f.lower().endswith(".bas")
    )
    results = {}
    by_first = collections.Counter()   # programs whose FIRST error is this
    by_any = collections.Counter()     # programs with this error anywhere
    unknown_names = collections.Counter()
    for path in files:
        ok, errors = compile_one(path, env)
        results[os.path.relpath(path, args.corpus)] = errors
        if ok:
            continue
        by_first[normalise(errors[0])] += 1
        for key in {normalise(e) for e in errors}:
            by_any[key] += 1
        for e in errors:
            if e.startswith("Unknown SUB or FUNCTION") or "is not declared" in e or "Unknown" in e:
                name = quoted(e)
                if name:
                    unknown_names[name.upper()] += 1

    if args.grep:
        for rel, errors in results.items():
            hit = next((e for e in errors if args.grep.lower() in e.lower()), None)
            if hit:
                print(f"{rel}: {hit}")
        return

    passed = sum(1 for e in results.values() if not e)
    print(f"{passed}/{len(files)} programs compile ({100 * passed / max(1, len(files)):.1f}%)\n")
    print("Most common FIRST error (programs blocked):")
    for msg, n in by_first.most_common(args.show):
        print(f"  {n:4d}  {msg}")
    print("\nMost common unknown names (occurrences):")
    for name, n in unknown_names.most_common(args.show):
        print(f"  {n:4d}  {name}")
    if args.json:
        with open(args.json, "w") as f:
            json.dump({"passed": passed, "total": len(files), "results": results}, f, indent=1)


if __name__ == "__main__":
    sys.exit(main())
