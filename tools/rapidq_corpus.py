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
        elif line.strip() and not errors and ("error" in line.lower() or "panicked" in line or "not found" in line):
            errors.append(line.strip()[:200])
    return False, errors or ["<failed without an error message>"]


DIRECTX_RE = re.compile(r"\bAS\s+Q(DX|D3D)\w*", re.I)
OLE_RE = re.compile(r"\bAS\s+QOLE\w*", re.I)
# (a program declaring DLL routines of its own — Windows', or a third-party DLL)
DLL_RE = re.compile(r"^\s*DECLARE\s+(SUB|FUNCTION)\s+\S+\s+LIB\b", re.I | re.M)


INCLUDE_RE = re.compile(r'^\s*\$INCLUDE\s+["<]([^">]+)[">]', re.I | re.M)
# (DOS-era port I/O and raw memory: no modern system lets a program do it)
HARDWARE_RE = re.compile(r"\bINP\s*\(|^\s*OUT\s+[^=]", re.I | re.M)


# Programs whose source isn't valid RapidQ (checked one by one, 2026-10-03):
# another BASIC's code, or a typo RapidQ's own compiler rejects too. They
# don't count against the portable percentage.
NOT_RAPIDQ = {
    "Compile/RQdecl.bas": "`IF … AND _` continued by a second `IF` (line 118)",
    "Database/SQL_blobs/SQ3.BAS": "C's `|` for flags (not a RapidQ operator)",
    "Encryption/RC4.bas": "typo `DIM Encrypted$YY`",
    "FreeImage/MFreeImage.bas": "Visual Basic (`Public Enum`)",
    "Linear_Solver/Linear.bas": "`if xlow not > 0 or then` (OR with nothing after it)",
    "Math/Linear_Solver/Linear.bas": "`if xlow not > 0 or then` (OR with nothing after it)",
    "Math/quest.bas": "missing `)` in `LOGe(P(N2+x)`",
    "Network/ftp/QListView dir and files.bas": "a string literal split over two lines",
    "Network/wsksock.bas": "Visual Basic (`Debug.Print`)",
    "sockets/wsksock.bas": "Visual Basic (`Debug.Print`)",
    "OLE/MSscript.bas": "typo `.Language = %T\"`",
    "OLE/xls/ExcelReader.bas": "a SUB call without parentheses assigned (`b$ = RiEd.addstring \"sheet \"…`)",
    "Object/Examples/Cd.bas": "typo `\"…\"+,`",
    "VideoCapture/WebCam_Class.bas": "`CASE 5` after `END SELECT`",
    "buttons/XPBtn.bas": "`SUB $XP_MANIFEST` (a directive's name as a SUB name)",
    "dialogs/colordlg/RGBV11.BAS": "`#H000000` (RapidQ's hex is `&H`)",
    "direct3d/billboard_ex.bas": "C's `//` comments",
    "dll/Resource in DLL/emptyDLL.bas": "FreeBASIC (`… export`)",
    "files/FileSearch_FB_DLL.bas": "FreeBASIC (`UBYTE PTR`)",
    "games/asteroids.bas": "typo `thrusta _ (` for `thrusta - (`",
    "games/ateroids2/asteroids.bas": "typo `thrusta _ (` for `thrusta - (`",
    "mysql/rqlibsql/mySQL_API.bas": "`End If` after a single-line IF",
    "zlib/QunZip.bas": "Visual Basic (`AddressOf`)",
}


def program_text(path):
    """A program's source and its own includes (next to it), not RapidQ's."""
    try:
        src = open(path, encoding="latin-1").read()
    except OSError:
        return ""
    texts = [src]
    for inc in INCLUDE_RE.findall(src):
        local = os.path.join(os.path.dirname(path), inc.replace("\\", "/"))
        if os.path.isfile(local):
            try:
                texts.append(open(local, encoding="latin-1").read())
            except OSError:
                pass
    return "\n".join(texts)


CORPUS_ROOT = None


def category(path, errors):
    """What a program needs: 'portable' (only the API the Windows and Linux
    RapidQ shared), 'dll' (Windows DLL calls), 'directx', 'ole', or
    'hardware' (DOS-era port I/O, raw memory) or 'incomplete' (an include
    or resource missing from the corpus)."""
    src = program_text(path)
    if OLE_RE.search(src):
        return "ole"
    if DIRECTX_RE.search(src):
        return "directx"
    # (DLL calls compile everywhere since docs/windows-dll-calls.md; the
    # programs run on Windows, and say so elsewhere)
    if DLL_RE.search(src):
        return "dll"
    # (an include that isn't in the corpus at all: the program can't build
    # anywhere, RapidQ included)
    if any(e.startswith("Include file not found") or "file not found" in e for e in errors):
        return "incomplete"
    # (port I/O compiles too, and stops the program at the INP / OUT)
    if HARDWARE_RE.search(src):
        return "hardware"
    rel = os.path.relpath(path, CORPUS_ROOT) if CORPUS_ROOT else path
    if errors and rel in NOT_RAPIDQ:
        return "not-rapidq"
    return "portable"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus")
    ap.add_argument("--include", help="RapidQ include directory (searched after the program's own directory)")
    ap.add_argument("--json")
    ap.add_argument("--show", type=int, default=40)
    ap.add_argument("--grep")
    args = ap.parse_args()
    global CORPUS_ROOT
    CORPUS_ROOT = os.path.abspath(args.corpus)

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
    print(f"{passed}/{len(files)} programs compile ({100 * passed / max(1, len(files)):.1f}%)")
    cats = collections.defaultdict(lambda: [0, 0])
    for rel, errors in results.items():
        c = category(os.path.join(args.corpus, rel), errors)
        cats[c][1] += 1
        cats[c][0] += 0 if errors else 1
    for c in ("portable", "dll", "directx", "ole", "hardware", "incomplete", "not-rapidq"):
        ok, n = cats[c]
        print(f"  {c:9s} {ok}/{n} compile ({100 * ok / max(1, n):.1f}%)")
    print()
    # (what blocks the portable programs only)
    by_first = collections.Counter(normalise(e[0]) for rel, e in results.items() if e and category(os.path.join(args.corpus, rel), e) == "portable")
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
