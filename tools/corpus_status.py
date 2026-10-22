#!/usr/bin/env python3
"""RapidQ's examples on each system and runtime, one table: the sweeps of
tools/corpus_run.py (interpreted / native, per system) and
tests/corpus_web_compare.mjs (the web against the desktop interpreter) put
together into docs/rapidq-corpus-status-unix.md (macOS and Linux; the
Windows lane writes docs/rapidq-corpus-status.md the same way).

Usage (repo root):
    python3 tools/corpus_status.py --corpus corpus.json --examples DIR
        [--run "macOS interp=mac-vm.json:vm"] [--run "macOS native=mac-native.json:native"]
        [--web "Web=corpus-web/report.json"] [--run "Linux interp=linux-vm.json:vm"] …
        [--title "…"] [--intro intro.md] [--notes notes.md] -o out.md

Each --run is `column=file.json:backend` (a corpus_run.py --json, several
files joined with `+`, later ones winning per program); --web is
`column=report.json`. Which programs: the portable ones
(tools/rapidq_corpus.py's category), plus those it calls portable but that
stop at a Windows call made by an include (counted as Windows-only).

A cell is **ok**, **ok\\*** (a difference RC.EXE's build has too, listed
under the table), *varies* (output that depends on the clock, random
numbers or the machine: not compared), *not run* (it would use the network,
print, start programs or open a serial port), or the problem
(corpus_run.py's kinds).
"""

import argparse
import collections
import datetime
import importlib.util
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def load_tool(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(ROOT, "tools", name + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


corpus = load_tool("rapidq_corpus")

# Programs whose run here matches RC.EXE's own build of them (checked in the
# Windows VM: tools/corpus_run_notes.md, "Differences left").
RC_SAME = {
    "CGI/cgi_test.bas": "RC.EXE refuses it too (`Member INITCGI not part of class CGI`)",
    "CGI/info.bas": "RC.EXE refuses it too (`Member INITCGI not part of class CGI`)",
    "QToolbar/OldStyle/Toolbar.bas": "an include file: RC.EXE's build stops with an access violation, RapidR shows nothing",
    "Toolbar/Toolbar.bas": "an include file: RC.EXE's build stops with an access violation, RapidR shows nothing",
    "Linear_Solver/crap.bas": "no window in RC.EXE's build either",
    "sound/SBFORM.BAS": "no window in RC.EXE's build either",
    "sound/sbbas/SBFORM.BAS": "no window in RC.EXE's build either",
    "forms/Two Windows.bas": "an empty form in RC.EXE's build too",
    "files/Read_all_Files_IN_Dir.bas": "ends in `cpuhog: GOTO cpuhog`, a loop RC.EXE's build never leaves either",
}

WINDOWS_CALL = re.compile(r"is a Windows function \((\w+)\)")


def short(kind, detail):
    m = WINDOWS_CALL.search(detail)
    if m:
        return f"Windows call ({m.group(1)})"
    kind = kind.replace(" (terminal)", "")
    if kind == "native build fails":
        return "native build fails"
    if kind in ("warning", "run-time error", "error", "exit"):
        d = re.sub(r"\(at [^)]*\)", "", detail)
        d = re.sub(r"/[^\s:]+", "…", d).strip()
        return f"{kind}: {d[:70]}"
    return kind


def load_runs(spec):
    col, _, rest = spec.partition("=")
    files, _, backend = rest.rpartition(":")
    by = {}
    for f in files.split("+"):
        for e in json.load(open(f)):
            by.setdefault(e["program"], {}).update(e)
    return col, backend, by


def cell_run(e, backend):
    if e is None:
        return None, None
    if e.get("skipped"):
        return "not run", None
    r = e.get(backend)
    if r is None:
        return None, None
    probs = r["problems"]
    if not probs:
        return "ok", None
    # (the Windows call first: it's why the rest happened)
    win = [p for p in probs if WINDOWS_CALL.search(p[1])]
    first = win[0] if win else probs[0]
    return short(*first), probs


WEB_DLL = re.compile(r"is a function of a DLL \((\w+)\)")


def cell_web(e):
    """(the cell, the Windows library the program stopped calling or None)."""
    if e is None:
        return None, None
    m = next((WEB_DLL.search(x) for x in e.get("webErrors", []) if WEB_DLL.search(x)), None)
    if m:
        return f"Windows call ({m.group(1)})", m.group(1)
    return cell_web_result(e), None


def cell_web_result(e):
    r = e.get("result", "")
    if r.startswith("same"):
        return "ok"
    if r.startswith("skipped"):
        return "not run"
    if r.startswith("differs ("):
        return "varies"
    if r == "differs":
        d = "; ".join(e.get("diffs", [])[:2])
        return f"differs: {d[:80]}"
    return r


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--examples", required=True)
    ap.add_argument("--run", action="append", default=[])
    ap.add_argument("--web", action="append", default=[])
    ap.add_argument("--title", default="RapidQ's examples under RapidR on macOS, Linux and the web")
    ap.add_argument("--intro")
    ap.add_argument("--notes")
    ap.add_argument("-o", "--out", required=True)
    args = ap.parse_args()
    corpus.CORPUS_ROOT = os.path.abspath(args.examples)
    compiled = json.load(open(args.corpus))["results"]
    runs = [load_runs(s) for s in args.run]
    webs = []
    for s in args.web:
        col, _, f = s.partition("=")
        webs.append((col, {e["program"]: e for e in json.load(open(f))}))
    columns = [c for c, _, _ in runs] + [c for c, _ in webs]

    rows, through_include, failing_compile = [], [], []
    for rel, errors in sorted(compiled.items(), key=lambda kv: kv[0].lower()):
        if corpus.category(os.path.join(args.examples, rel), errors) != "portable":
            continue
        if errors:
            failing_compile.append((rel, errors[0]))
            continue
        cells, calls = [], set()
        for col, backend, by in runs:
            c, probs = cell_run(by.get(rel), backend)
            for _, d in probs or []:
                m = WINDOWS_CALL.search(d)
                if m:
                    calls.add(m.group(1))
            if c not in (None, "ok", "not run") and rel in RC_SAME:
                c = "ok*"
            cells.append(c)
        for col, by in webs:
            c, lib = cell_web(by.get(rel))
            if lib:
                calls.add(lib)
            if c not in (None, "ok", "not run", "varies") and rel in RC_SAME:
                c = "ok*"
            cells.append(c)
        if calls:
            through_include.append((rel, sorted(calls), cells))
            continue
        rows.append((rel, cells))

    L = [f"# {args.title}", ""]
    L.append(f"*Generated by `tools/corpus_status.py` ({datetime.date.today().isoformat()}) from the sweeps' results; do not edit by hand.*")
    L.append("")
    if args.intro:
        L.append(open(args.intro).read().rstrip())
        L.append("")
    L.append("## Counts")
    L.append("")
    L.append("A cell is **ok** (it ran without a problem: no RapidR error, warning, crash or hang, its windows shown, its output RC.EXE's where that is known; on the web, the same output, forms and components as the desktop interpreter), **ok\\*** (a difference RC.EXE's own build has too, listed under the table), *varies* (what it prints depends on the clock, random numbers or the machine — its file name, the screen's size — so it isn't compared), *not run* (it would use the network or a serial port, print, or start other programs), or the problem.")
    L.append("")
    L.append("| Runtime | ran | ok | ok\\* (as RC.EXE) | varies | problems | not run |")
    L.append("|---|---:|---:|---:|---:|---:|---:|")
    for i, col in enumerate(columns):
        vals = [cells[i] for _, cells in rows]
        n = collections.Counter("ok" if v == "ok" else "ok*" if v == "ok*" else "varies" if v == "varies" else "not run" if v == "not run" else None if v is None else "problem" for v in vals)
        ran = n["ok"] + n["ok*"] + n["varies"] + n["problem"]
        L.append(f"| {col} | {ran} | {n['ok']} | {n['ok*']} | {n['varies']} | {n['problem']} | {n['not run']} |")
    L.append("")
    L.append(f"Portable programs (RapidQ's shared Windows / Linux API, `tools/rapidq_corpus.py`): {len(rows) + len(through_include) + len(failing_compile)}; "
             f"{len(failing_compile)} don't compile, {len(through_include)} call Windows through an include (below), {len(rows)} in the table.")
    L.append("")
    L.append("## Programs")
    L.append("")
    L.append("| Program | " + " | ".join(columns) + " |")
    L.append("|---|" + "---|" * len(columns))
    for rel, cells in rows:
        L.append(f"| `{rel}` | " + " | ".join(c if c else "—" for c in cells) + " |")
    L.append("")
    stars = [r for r, cells in rows if "ok*" in cells]
    if stars:
        L.append("**ok\\*** — as RC.EXE's build of the program:")
        L.append("")
        for r in stars:
            L.append(f"- `{r}`: {RC_SAME[r]}")
        L.append("")
    if through_include:
        L.append("## Windows calls through an include")
        L.append("")
        L.append("Programs that declare no DLL routine themselves but stop at one an include file calls (RapidQ's `rapidq2.inc` calls kernel32's `GetVersion` when it starts; `windows.inc`-based libraries, ODBC). They run on Windows (docs/windows-dll-calls.md); on macOS, Linux and the web they stop there with RapidR's error naming the call:")
        L.append("")
        for rel, calls, cells in through_include:
            L.append(f"- `{rel}` — {', '.join(calls)}")
        L.append("")
    if failing_compile:
        L.append("## Not compiling")
        L.append("")
        for rel, e in failing_compile:
            L.append(f"- `{rel}`: {e}")
        L.append("")
    if args.notes:
        L.append(open(args.notes).read().rstrip())
        L.append("")
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as f:
        f.write("\n".join(L))
    print(f"wrote {args.out}: {len(rows)} programs, {len(through_include)} through an include, {len(failing_compile)} not compiling")


if __name__ == "__main__":
    sys.exit(main())
