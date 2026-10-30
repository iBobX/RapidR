#!/usr/bin/env python3
"""RapidR Studio's design guard (docs/design-guidelines.md): what the guide
says in numbers, checked in Studio's own code (ide/*.inc, ide/studio.rr).

  colour     a colour written as a literal — `&H…`, RGB(…), a number or a
             `cl…` constant assigned to Color / Font.Color / FontColor /
             PenColor / BrushColor, or RGB(…) anywhere: Studio takes every
             colour from the theme (TC("token"), Application.ThemeColor),
             so it is right in light, dark and high contrast
  token      TC("name") / ThemeColor("name") with a name no theme has
             (crates/rapidr-value/src/theme.rs, ide_theme.rs): it would
             draw -1
  height     a button's, combo box's, field's, check box's or radio
             button's Height — or the bh / fh a layout sizes them
             with — written as a number off the guide's scale
             (HEIGHTS below: compact 24, dialog buttons 32, classic 21 and
             25, one-line rows 16–22); UiCompactH / UiButtonH say them by
             name

What is there today is listed in tools/check_design_baseline.txt (file,
rule and the line's code, not its number): the check fails only on a hit
that isn't in it — a new one. Fixing an old one leaves a stale entry, which
is reported; `--update` writes the list again from the code (after fixing,
never to let a new hit in).

  python3 tools/check_design.py            check (exit 1 on a new hit)
  python3 tools/check_design.py --list     every hit, old and new
  python3 tools/check_design.py --update   the baseline from today's code
"""
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "tools" / "check_design_baseline.txt"
SOURCES = sorted((ROOT / "ide").glob("*.inc")) + [ROOT / "ide" / "studio.rr"]

# The guide's control heights at 1x (docs/design-guidelines.md, "Controls
# by context"): 24 compact / fields / combo boxes (21 classic), 32 dialog
# buttons (25 classic), 16–22 a one-line row (a check box, an option).
HEIGHTS = {16, 18, 20, 21, 22, 24, 25, 32}
# The components whose height the guide sets.
SIZED = {"RBUTTON", "QBUTTON", "RCOMBOBOX", "QCOMBOBOX", "REDIT", "QEDIT", "RCHECKBOX", "QCHECKBOX", "RRADIOBUTTON", "QRADIOBUTTON", "RCOOLBTN", "QCOOLBTN", "RSPINEDIT"}
# The layout variables that size them (bh: a button's height, fh: a
# field's).
HEIGHT_VARS = re.compile(r"\b(bh|fh|btnh|fieldh)\s*=\s*(\d+)\b", re.I)
COLOR_PROPS = re.compile(r"(?:^|[.\s:])(?:Color|FontColor|PenColor|BrushColor)\s*=\s*(&H[0-9A-F]+|RGB\s*\(|-?\d+\b|cl[A-Z]\w*)", re.I)
RGB_CALL = re.compile(r"\bRGB\s*\(", re.I)
TOKEN_USE = re.compile(r"\b(?:TC|ThemeColor)\s*\(\s*\"([^\"]+)\"\s*\)", re.I)
CREATE = re.compile(r"^\s*CREATE\s+(\w+)\s+AS\s+(\w+)", re.I)
DIM = re.compile(r"^\s*DIM\s+(\w+)(?:\s*\(\s*\w*\s*\))?\s+AS\s+(\w+)", re.I)
END_CREATE = re.compile(r"^\s*END\s+CREATE\b", re.I)
SET_HEIGHT = re.compile(r"\b(\w+)(?:\s*\([^)]*\))?\.Height\s*=\s*(\d+)\b", re.I)
BARE_HEIGHT = re.compile(r"(?:^|:)\s*Height\s*=\s*(\d+)\b", re.I)


def code_of(line):
    """The line without its comment and with its strings blanked (what the
    rules read), and the code kept as written for the report."""
    out, in_str = [], False
    for ch in line:
        if ch == '"':
            in_str = not in_str
            out.append('"')
        elif in_str:
            out.append(" ")
        elif ch == "'":
            break
        else:
            out.append(ch)
    code = "".join(out)
    if re.match(r"^\s*REM\b", code, re.I):
        return ""
    return code


def tokens():
    """Every colour name a theme has: Theme::colors and IdeColors::colors."""
    names = set()
    for f in ("theme.rs", "ide_theme.rs"):
        text = (ROOT / "crates" / "rapidr-value" / "src" / f).read_text(encoding="utf-8")
        names.update(re.findall(r'^\s*\("([a-z0-9_.]+)",', text, re.M))
    key = lambda s: re.sub(r"[ ._-]", "", s).lower()
    return {key(n) for n in names}, key


def types():
    """Each component's type by name, from every CREATE and DIM in Studio."""
    kinds = {}
    for f in SOURCES:
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            m = CREATE.match(line) or DIM.match(line)
            if m:
                kinds.setdefault(m.group(1).lower(), m.group(2).upper())
    return kinds


def hits():
    """(file, line number, rule, code) for every hit in Studio's code."""
    known, key = tokens()
    kinds = types()
    found = []
    for f in SOURCES:
        rel = f.relative_to(ROOT).as_posix()
        stack = []
        for n, raw in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            m = CREATE.match(raw)
            if m:
                stack.append(m.group(2).upper())
            elif END_CREATE.match(raw):
                if stack:
                    stack.pop()
                continue
            # (token names are inside strings: read on the raw line, comment off)
            raw_code = raw.split("'")[0] if raw.count('"') == 0 else raw
            for name in TOKEN_USE.findall(raw_code):
                if key(name) not in known and not raw.lstrip().startswith("'"):
                    found.append((rel, n, "token", raw.strip()))
            code = code_of(raw)
            if not code.strip():
                continue
            if COLOR_PROPS.search(code) or RGB_CALL.search(code):
                found.append((rel, n, "colour", raw.strip()))
            for v in HEIGHT_VARS.finditer(code):
                if int(v.group(2)) not in HEIGHTS:
                    found.append((rel, n, "height", raw.strip()))
                    break
            for name, h in SET_HEIGHT.findall(code):
                if kinds.get(name.lower()) in SIZED and int(h) not in HEIGHTS:
                    found.append((rel, n, "height", raw.strip()))
                    break
            if stack and stack[-1] in SIZED and not m:
                for h in BARE_HEIGHT.findall(code):
                    if int(h) not in HEIGHTS:
                        found.append((rel, n, "height", raw.strip()))
                        break
    return found


def entry(h):
    rel, _, rule, code = h
    return f"{rel}\t{rule}\t{' '.join(code.split())}"


def read_baseline():
    if not BASELINE.exists():
        return Counter()
    lines = [l for l in BASELINE.read_text(encoding="utf-8").splitlines() if l and not l.startswith("#")]
    return Counter(lines)


def main():
    found = hits()
    if "--list" in sys.argv:
        for rel, n, rule, code in found:
            print(f"{rel}:{n}: {rule}: {code}")
        print(f"{len(found)} hits")
        return 0
    if "--update" in sys.argv:
        head = ("# tools/check_design.py's baseline: the design guide's hits Studio's code\n"
                "# had when the guard came (2026-10-29), file, rule and code. Fix them and\n"
                "# run `python3 tools/check_design.py --update`; never add a new one here.\n")
        BASELINE.write_text(head + "".join(entry(h) + "\n" for h in sorted(found, key=entry)), encoding="utf-8")
        print(f"{BASELINE.relative_to(ROOT)}: {len(found)} entries")
        return 0
    base = read_baseline()
    seen = Counter()
    new = []
    for h in found:
        e = entry(h)
        seen[e] += 1
        if seen[e] > base[e]:
            new.append(h)
    stale = base - seen
    for rel, n, rule, code in new:
        why = {
            "colour": "a colour written as a number: take it from the theme, TC(\"token\") (docs/design-guidelines.md, Colour)",
            "token": "no theme has this colour name (docs/theme-tokens.md)",
            "height": "a control height off the guide's scale: UiCompactH (24) in panes and strips, UiButtonH (32) for a dialog's buttons (docs/design-guidelines.md, Controls by context)",
        }[rule]
        print(f"{rel}:{n}: {rule}: {code}\n    {why}")
    if stale:
        print(f"check_design: {sum(stale.values())} baseline entr{'y is' if sum(stale.values()) == 1 else 'ies are'} gone from the code (fixed? run --update):")
        for e in sorted(stale):
            print("    " + e.replace("\t", "  "))
    if new:
        print(f"check_design: FAILED, {len(new)} new hit{'s' if len(new) != 1 else ''} (docs/design-guidelines.md)")
        return 1
    print(f"check_design: ok ({len(found)} known, none new)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
