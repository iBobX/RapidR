#!/usr/bin/env python3
"""RapidR describes itself in its own words (Robert, 2026-10-21: "We are
RapidR!!!"): no other IDEs or products as models or comparisons in the code
comments, docs, manual, UI strings or plans.

This check fails when one of those product words turns up in a text file
(tracked, or new and not ignored) outside tools/words-allowlist.txt. Run it from anywhere:

    python3 tools/check_words.py          the check (exit 1 on a new hit)
    python3 tools/check_words.py --list   every allowed hit, with its reason

What is scanned: docs/ ide/ crates/ examples/ interpreter/ tools/ tests/ and
the top-level guides (README, ROADMAP, CONTRIBUTING, ANNOUNCEMENT,
COMPILER_MANUAL, SECURITY, VSCODE_EXTENSION). Not scanned, by design: the
legal and history texts (CHANGELOG, LEGAL, LICENSE*, NOTICE,
THIRD_PARTY_NOTICES), the VS Code extension's own folder (utilities/), and
.reference/. Words are matched whole (so `delphi_digits`, a code symbol, is
not a hit); the allowlist lists every place a word stays, and why.

Allowlist lines are `path :: text` (see the file's header). An entry that no
longer matches anything is an error too, so the list cannot go stale.
"""
import fnmatch
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Whole words, any case. (A product or tool RapidR is not, and does not
# compare itself with.)
WORDS = [
    r"delphi", r"xojo", r"xcode", r"jetbrains", r"vb6", r"intellisense",
    r"codelens", r"codea", r"qb64", r"freebasic", r"purebasic", r"lazarus",
    r"vcl", r"swiftui", r"visual\s+basic",
]
RX = re.compile(r"\b(" + "|".join(WORDS) + r")\b", re.I)

SCAN_DIRS = ["docs", "ide", "crates", "examples", "interpreter", "tools", "tests"]
SCAN_FILES = ["README.md", "ROADMAP.md", "CONTRIBUTING.md", "ANNOUNCEMENT.md",
              "COMPILER_MANUAL.md", "SECURITY.md", "VSCODE_EXTENSION.md"]
MAX_BYTES = 6 * 1024 * 1024


def tracked_files():
    try:
        out = subprocess.run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--"] + SCAN_DIRS + SCAN_FILES,
                             cwd=ROOT, capture_output=True, check=True).stdout
        return [p for p in out.decode("utf-8", "replace").split("\0") if p]
    except (OSError, subprocess.CalledProcessError):
        found = []
        for d in SCAN_DIRS:
            for base, dirs, files in os.walk(os.path.join(ROOT, d)):
                dirs[:] = [x for x in dirs if x not in ("target", ".work", "node_modules", ".git")]
                for f in files:
                    found.append(os.path.relpath(os.path.join(base, f), ROOT))
        found += [f for f in SCAN_FILES if os.path.exists(os.path.join(ROOT, f))]
        return found


class Entry:
    def __init__(self, path, text, lineno):
        self.path, self.text, self.lineno = path, text, lineno
        self.uses = 0

    def covers(self, rel):
        p = self.path
        if p == "*":
            return True
        if p.endswith("/"):
            return rel.startswith(p)
        if any(c in p for c in "*?["):
            return fnmatch.fnmatch(rel, p)
        return rel == p


def load_allowlist():
    entries = []
    path = os.path.join(ROOT, "tools", "words-allowlist.txt")
    with open(path, encoding="utf-8") as fh:
        for n, raw in enumerate(fh, 1):
            line = raw.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if " :: " in line:
                p, t = line.split(" :: ", 1)
            elif line.rstrip().endswith(" ::"):
                p, t = line.rstrip()[:-3], ""
            else:
                print(f"tools/words-allowlist.txt:{n}: expected `path :: text`")
                sys.exit(2)
            entries.append(Entry(p.strip(), t.strip(), n))
    return entries


def main():
    show_list = "--list" in sys.argv
    entries = load_allowlist()
    bad = []
    allowed = []
    for rel in tracked_files():
        full = os.path.join(ROOT, rel)
        try:
            if os.path.getsize(full) > MAX_BYTES:
                continue
            with open(full, "rb") as fh:
                data = fh.read()
        except OSError:
            continue
        if b"\0" in data:
            continue
        mine = [e for e in entries if e.covers(rel)]
        for n, line in enumerate(data.decode("utf-8", "replace").split("\n"), 1):
            if not RX.search(line):
                continue
            rest = line
            used = []
            for e in mine:
                if e.text == "":
                    used.append(e)
                    rest = ""
                    break
                if e.text.lower() in rest.lower():
                    rest = re.sub(re.escape(e.text), "", rest, flags=re.I)
                    used.append(e)
            if RX.search(rest):
                bad.append((rel, n, RX.search(rest).group(0), line.strip()))
            else:
                for e in used:
                    e.uses += 1
                allowed.append((rel, n, line.strip(), used))
    stale = [e for e in entries if e.uses == 0]
    if show_list:
        for rel, n, line, used in allowed:
            print(f"{rel}:{n}: {line[:160]}")
        return 0
    status = 0
    if bad:
        status = 1
        print("Words that RapidR's own text doesn't use (describe the feature in RapidR's own words;")
        print("if one really has to stay, add it, with its reason, to tools/words-allowlist.txt):")
        for rel, n, w, line in bad:
            print(f"  {rel}:{n}: {w}: {line[:150]}")
    if stale:
        status = 1
        print("Unused entries in tools/words-allowlist.txt (remove them):")
        for e in stale:
            print(f"  line {e.lineno}: {e.path} :: {e.text}")
    if status == 0:
        print(f"words: all ok ({len(allowed)} allowed uses, {len(entries)} allowlist entries)")
    return status


if __name__ == "__main__":
    sys.exit(main())
