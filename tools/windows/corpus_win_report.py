#!/usr/bin/env python3
"""The Windows table of docs/rapidq-corpus-status.md from corpus_win.py's
results (tools/windows/corpus_win.py; the last line per program counts).

    python3 tools/windows/corpus_win_report.py results.jsonl [--doc docs/rapidq-corpus-status.md]
        [--notes notes.json] [--summary]

Each RapidR column (interpreted, native) says, against RC.EXE's run of the
same program in the same Windows:

- **same**: no error; a console program printed what RC.EXE's did; a GUI
  one showed as many windows, of the same size (allowing RapidR's own form
  frame: docs/rapidq-corpus-status.md, "Form frames"), looking alike (the
  captures, RapidR's classic theme against RC.EXE's).
- **differs**: it runs, but something isn't RC.EXE's — the note says what.
- **fails**: a compile error, a run-time error, a crash, or no window where
  RC.EXE shows one.
- **RC.EXE fails**: RC.EXE doesn't compile it or its program stops the
  same way (RapidR's result is given beside it).
- **not compared**: it runs, but there is nothing of RC.EXE's to compare it
  with (RC.EXE's output was lost to the file it was redirected to; the first
  window is one of Windows' file dialogs, which the test run answers).
- **not run**: it would print, touch the system or need hardware (built only).

--notes: a JSON object {program: note} added to the table (what a person saw
comparing the captures). Writes the table between the doc's
`<!-- corpus-win:start -->` and `<!-- corpus-win:end -->` lines.
"""
import argparse
import os
import collections
import json
import re

ap = argparse.ArgumentParser()
ap.add_argument("results", nargs="+", help="results.jsonl files: a later file's parts (rc, interp, native) replace an earlier's")
ap.add_argument("--doc")
ap.add_argument("--notes")
ap.add_argument("--summary", action="store_true")
args = ap.parse_args()

recs = {}
for path in args.results:
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if line:
            r = json.loads(line)
            recs.setdefault(r["prog"], {}).update({k: v for k, v in r.items() if v is not None or k not in recs.get(r["prog"], {})})
notes = json.load(open(args.notes)) if args.notes else {}

# A difference in the captures below this (mean grey difference, 0-255) is
# "alike": fonts and anti-aliasing differ a little everywhere.
ALIKE = 12.0


EXAMPLES = os.path.join(os.path.expanduser(os.environ.get("RAPIDQ_DIR", "~/Downloads/Rapidq")), "examples")
# (output that depends on random numbers or the clock: tools/corpus_run.py's)
VARIES = re.compile(r"\b(rnd|randomize|timer|time\$|date\$|tickcount|gettickcount)\b", re.I)
# Windows' common dialogs (their titles in English Windows)
SYSTEM_DIALOG = re.compile(r"^(Open|Save As|Save|Page Setup|Print|Color|Font|Browse For Folder)$")


def source(prog):
    """The program's text (RapidQ's examples folder, read-only), or ""."""
    try:
        return open(os.path.join(EXAMPLES, prog), encoding="latin-1").read()
    except OSError:
        return ""


# (only RapidQ's programs: a record of a name that isn't one — a run given a
# bad list — is left out)
if os.path.isdir(EXAMPLES):
    recs = {p: r for p, r in recs.items() if os.path.isfile(os.path.join(EXAMPLES, p))}


def clean(msg):
    msg = (msg or "").replace("|", "/").replace("\n", " ").strip()
    msg = re.sub(r"\s+", " ", msg)
    return msg[:110]


def rc_state(rec):
    """('fails' | 'runs' | 'not run', text, rc record)."""
    rc = rec.get("rc")
    if not rc:
        return "none", "—", {}
    if rc.get("compile") != "ok":
        m = re.sub(r"^Line (\d+)(, \d+)?: (ERROR: )?", "", clean(rc.get("msg")).split(" / ")[0])
        return "fails", f"compile error: {m}", rc
    run = rc.get("run") or ""
    if run.startswith("not run"):
        return "not run", f"builds ({run})", rc
    if run == "harness":
        return "fails", "harness error", rc
    if rc.get("kind") == "console" and not rc.get("windows"):
        return "runs", "console" + (", runs until stopped" if run == "timeout" else ""), rc
    wins = rc.get("windows") or []
    if not wins:
        return "fails", f"no window (exit {rc.get('exit')})", rc
    # (RapidQ's run-time errors are message boxes: "Exception EAccessViolation …";
    # its DirectX objects' "D3DRM.dll not loaded", "Display mode cannot be changed")
    for w in wins:
        t = w.get("text") or ""
        m = re.search(r"Exception (\w+)[^|]*?\.\s*([^|]*)", t) or re.search(r"(D3DRM\.dll not loaded|Display mode cannot be changed[^|]*)", t)
        if m:
            return "fails", "error box: " + clean(" ".join(g for g in m.groups() if g)), rc
    desc = ", ".join(f"{w.get('w')}×{w.get('h')}" + (" (dialog)" if w.get("cls") in ("#32770", "TMessageForm") else "") for w in wins)
    return "runs", desc, rc


def norm_out(s):
    # (RapidR draws CLS / COLOR / LOCATE with the terminal's escape sequences,
    # also into a file; RC.EXE's console calls put nothing there: the text
    # is compared)
    s = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07", "", s or "")
    lines = [l.rstrip() for l in s.replace("\r", "").split("\n")]
    lines = [l for l in lines if "[rapidr]" not in l and "[WARN]" not in l]
    while lines and not lines[-1]:
        lines.pop()
    return lines


def rr_state(rec, which, rcs):
    """(category, text) for one RapidR runtime."""
    r = rec.get(which)
    if r is None:
        if which == "native" and (rec.get("interp") or rec.get("check") or {}).get("compile") == "error":
            return rr_state(rec, "interp", rcs)[0], "compile error (as interpreted)"
        return "none", "—"
    if r.get("compile") != "ok":
        if rcs[0] == "fails" and rcs[1].startswith("compile error"):
            return "rc fails", f"compile error: {clean(r.get('msg'))}"
        return "fails", f"compile error: {clean(r.get('msg'))}"
    if rec.get("skip"):
        return "not run", f"builds (not run: {rec['skip']})"
    run = r.get("run")
    if run in ("error", "panic", "crash"):
        return "fails", f"{'run-time error' if run == 'error' else run}: {clean(r.get('msg'))}"
    if run == "harness":
        return "fails", "harness error"
    state, _, rc = rcs
    wins = r.get("windows") or []
    if state != "runs":
        what = "runs" if run == "ok" else "runs until stopped" if run == "timeout" else run
        if wins:
            what += f", {len(wins)} window(s)"
        return ("rc fails" if state == "fails" else "runs"), what
    if rc.get("kind") == "console" and not rc.get("windows"):
        a, b = norm_out(rc.get("out")), norm_out(r.get("out"))
        rc_loops, rr_loops = rc.get("run") == "timeout", run == "timeout"
        if a == b and rc_loops == rr_loops:
            return "same", "same output" + (" (runs until stopped)" if rr_loops else "")
        # (nothing of RC.EXE's reached the file — its program waited for a
        # key, or was stopped: nothing to compare RapidR's with)
        if not a and b:
            return "runs", "it prints; RC.EXE's output to a file is lost"
        # (RC.EXE's build loses the end of what it printed when its output
        # goes to a file — `TEL <host or ip` for `TEL <host or ip>`, or all
        # of it: RapidR's begins with RC.EXE's)
        if len(a) <= len(b) and a[:-1] == b[: len(a) - 1] and (not a or b[len(a) - 1].startswith(a[-1])):
            return "same", "same output (RC.EXE's, redirected to a file, stops short)"
        if rc_loops and rr_loops:
            # (both still running when stopped: where they were differs by
            # the moment; the output's last part is kept)
            return "same", "runs until stopped, as RC.EXE's"
        if VARIES.search(source(rec["prog"])):
            return "same", "output depends on the clock or random numbers"
        if wins and not b:
            return "differs", "shows a window"
        for i, (x, y) in enumerate(zip(a, b)):
            if x != y:
                # (where, not what: the programs' text stays out of the table)
                return "differs", f"output line {i + 1} differs from RC.EXE's"
        if len(a) != len(b):
            return "differs", f"{len(b)} output lines (RC.EXE {len(a)})" + (", runs until stopped" if rr_loops and not rc_loops else "")
        return "differs", "runs until stopped" if rr_loops else "ends (RC.EXE runs until stopped)"
    # GUI
    rw = rc.get("windows") or []
    if run == "timeout":
        return "differs", "didn't end at the capture (a wait outside the message loop?)"
    # (the windows as the screen showed them, else as RapidR saved them)
    shots = [w for w in (r.get("screens") or []) if w.get("png")] or [w for w in wins if w.get("png")]
    # (RC.EXE's program opened Windows' own file, print or colour dialog
    # first; RapidR's run had its test hook answer that dialog: nothing to
    # compare)
    if not shots and rw and all(w.get("cls") == "#32770" and SYSTEM_DIALOG.match(w.get("title") or "") for w in rw):
        return "runs", f"its first window is Windows' {rw[0].get('title')!r} dialog, which the test run answers (not compared)"
    if not shots:
        return "fails", "no window"
    issues = []
    framed = False
    if len(shots) != len(rw):
        issues.append(f"{len(shots)} window(s) (RC.EXE {len(rw)})")
    for k, (a, b) in enumerate(zip(rw, shots)):
        dw, dh = b["w"] - a["w"], b["h"] - a["h"]
        if (dw, dh) == (0, 0):
            continue
        # RapidR's own form frame (docs/rapidq-corpus-status.md, "Form frames"):
        # a form sized by Width / Height has a larger inside than RC.EXE's on
        # Windows 11 (sizeable 16 × 39, single 6 × 29 against RapidR's
        # 2 × 31) — and RC.EXE's smaller one may need scroll bars (17) —;
        # RapidR's main menu is inside its window (28), RC.EXE's above the
        # client area (20).
        menu = 20 if a.get("menu") else 0
        # (± 1: RC.EXE's windows are scaled by Windows from 96 dpi)
        near = lambda v, opts: any(abs(v - o) <= 1 for o in opts)
        if (near(dw, (4, 14, 21, 31)) and near(dh - menu, (-2, 8, 15, 25))) or (dw == 0 and dh == (28 if menu else 0)):
            framed = True
            continue
        if abs(dw) > 2 or abs(dh) > 2:
            issues.append(f"window {k + 1} {b['w']}×{b['h']} (RC.EXE {a['w']}×{a['h']})")
    d = r.get("diff") or [None, None]
    if d[0] is not None and d[0] > ALIKE:
        issues.append(f"looks different (capture difference {d[0]})")
    if issues:
        return "differs", "; ".join(issues)
    return "same", ("same windows, RapidR's form frame" if framed else "same windows") + (f", difference {d[0]}" if d[0] is not None else "")


CATS = ["same", "differs", "fails", "rc fails", "not run", "runs", "none"]
rows = []
counts = {"interp": collections.Counter(), "native": collections.Counter(), "rc": collections.Counter()}
causes = collections.Counter()
for prog in sorted(recs, key=str.lower):
    rec = recs[prog]
    rcs = rc_state(rec)
    counts["rc"][rcs[0]] += 1
    cells = []
    for which in ("interp", "native"):
        cat, text = rr_state(rec, which, rcs)
        counts[which][cat] += 1
        if cat == "fails":
            # (names, addresses and long numbers left out: one line per kind)
            causes[re.sub(r"(?<![A-Za-z])'[^']*'|`[^`]*`|\"[^\"]*\"|0x[0-9a-fA-F]+|-?\d{3,}|\(at [^)]*\)", "…", text)[:90]] += 1
        label = {"same": "same", "differs": "differs", "fails": "fails", "rc fails": "RC.EXE fails; RapidR", "not run": "not run", "runs": "not compared", "none": "—"}[cat]
        cells.append(label if text in ("—",) else f"{label}: {text}" if cat not in ("same",) else f"same ({text})")
    rows.append((prog, cells[0], cells[1], rcs[1], notes.get(prog, "")))

out = ["| Program | Windows interpreted | Windows native | RC.EXE | Notes |", "|---|---|---|---|---|"]
for p, a, b, c, n in rows:
    out.append(f"| {p} | {a} | {b} | {c} | {n} |")
table = "\n".join(out)

summ = ["| | same | differs | fails | RC.EXE fails | not compared | not run | — |", "|---|---|---|---|---|---|---|---|"]
for which, name in (("interp", "Windows interpreted"), ("native", "Windows native")):
    c = counts[which]
    summ.append(f"| {name} | {c['same']} | {c['differs']} | {c['fails']} | {c['rc fails']} | {c['runs']} | {c['not run']} | {c['none']} |")
rc = counts["rc"]
summ_rc = f"RC.EXE: {rc['runs']} run, {rc['fails']} fail (compile error, no window), {rc['not run']} built but not run, of {len(recs)} programs."
summary = "\n".join(summ) + "\n\n" + summ_rc

if args.summary or not args.doc:
    print(summary)
    print()
    print("Most common failures:")
    for k, n in causes.most_common(25):
        print(f"  {n:3d}  {k}")
if args.doc:
    doc = open(args.doc, encoding="utf-8").read()
    for tag, body in (("corpus-win-summary", summary), ("corpus-win", table)):
        a, b = f"<!-- {tag}:start -->", f"<!-- {tag}:end -->"
        if a in doc:
            pre, rest = doc.split(a, 1)
            _, post = rest.split(b, 1)
            doc = pre + a + "\n" + body + "\n" + b + post
    open(args.doc, "w", encoding="utf-8").write(doc)
