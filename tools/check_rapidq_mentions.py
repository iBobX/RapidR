#!/usr/bin/env python3
"""RapidR describes itself in its own words: user-facing text never names
RapidQ (Robert, 2026-10-24/30: "We don't care about RapidQ, we care about
RapidR"). It names it in two places only: the importer's own texts (its menu
item and dialog, `rapidr import-rapidq`, docs/manual/importing-rapidq.md) and
one line on the compatibility page (docs/manual/compatibility.md, and its
translations: the line that links to the importer's page) saying that RapidR
includes a layer to import older programs written for RapidQ.

This check fails when "RapidQ" turns up in text a user reads, outside
tools/rapidq-allowlist.txt. Run it from anywhere:

    python3 tools/check_rapidq_mentions.py            the check (exit 1 on a new hit)
    python3 tools/check_rapidq_mentions.py --list     every allowed hit, with its reason
    python3 tools/check_rapidq_mentions.py --count    hits per area (allowed + new)

What counts as text a user reads, by kind of file:

  *.md               the whole file: the manual (docs/manual/, the Spanish
                     pages, the generated reference), README.md,
                     examples/README.md, the top of CHANGELOG.md (before the
                     first version heading: its history stays)
  *.toml (registry)  every line of crates/rapidr-lang/data/ except comments
                     and the structural fields the importer and the compiler
                     read (`rapidq = "QBUTTON"`, `from = "…"`, `source = "…"`)
  *.inc, *.rr        string literals and comments of RapidR Studio's own
  (ide/, examples/)  program and of the examples (a header comment is read);
                     examples/rapidq/ is the importer's corpus: not scanned
  *.po, *.pot        msgid and msgstr (Studio's translations)
  *.rs               string literals (comments are not text a user reads)
  *.js, *.html       string literals / text of the web pages and the VS Code
  *.json             extension's user-visible files

Never a mention (names of files and commands, not wording): the importer's
command and component (`import-rapidq`, `ImportRapidQ`), the folder
`examples/rapidq/`, links to docs/rapidq-*.md, and the include file
`RAPIDQ.INC` / `RAPIDQ2.INC`, which an older program writes in its
`$INCLUDE` line and which a `.bas` program is told to add.

Allowlist lines are `path :: text` (see the file's header). An entry that no
longer matches anything is an error too, so the list cannot go stale.
"""
import fnmatch
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ALLOWLIST = os.path.join(ROOT, "tools", "rapidq-allowlist.txt")

# ("RapidQ", and RC.EXE, its compiler)
MENTION = re.compile(r"rapid[\s_-]?q|\bRC\.EXE\b", re.I)

# Names of files, folders and commands: part of a path or a command, not wording.
NAMES = re.compile(
    r"import-rapidq"
    r"|ImportRapidQ"
    r"|Import(?: a)? RapidQ(?: Project or File| Fol&?der| Program)?\b"
    r"|importing-rapidq"
    r"|rapidq-(?:corpus-status(?:-unix)?|ground-truth|review|truth)\b"
    r"|rapidq_(?:corpus|truth|import_check|mentions)"
    r"|rapidq-allowlist"
    r"|RAPIDQ2?\.INC"
    r"|(?<![\w])rapidq/"
    r"|RapidR\.RapidQSource|rapidq-source|text/x-rapidq-basic",
    re.I,
)

SCAN_DIRS = ["docs", "ide", "crates", "examples", "interpreter", "utilities", "tools/release"]
# (ANNOUNCEMENT.md — drafts for RapidQ's own user groups —, COMPILER_MANUAL.md,
# CONTRIBUTING.md and VSCODE_EXTENSION.md — guides for the people who work on
# RapidR, with the clean-room rules — are developers' text, not scanned.)
SCAN_FILES = ["README.md", "CHANGELOG.md"]
MAX_BYTES = 4 * 1024 * 1024

# What is not scanned, whatever its kind: not text a user reads.
SKIP_PREFIXES = (
    "examples/rapidq/",            # the importer's corpus
    "docs/legal/",                 # the legal record names its subject
    "docs/release-notes/",         # published history
    "docs/rapidq-",                # the compatibility reports' own pages
    "docs/ai_eval", "tools/ai_eval", "tools/rai_train/",
    "crates/rapidr-ai/src/knowledge",  # (checked as .md below if it is one)
    "utilities/vscodeext/rapidr/test/",
    "utilities/vscodeext/rapidr/CHANGELOG.md",   # the extension's published history
    "utilities/vscodeext/rapidr/node_modules/",
)
# The importer's own page, in every language (docs/manual/<lang>/): it is about
# the importer, so it names its source.
IMPORTER_PAGE = re.compile(r"^docs/manual/(?:[A-Za-z-]+/)?importing-rapidq\.md$")
# The compatibility page, in every language: one line says that RapidR
# includes a layer to import older programs written for RapidQ (the line that
# links to the importer's page); no other line names it.
COMPAT_PAGE = re.compile(r"^docs/manual/(?:[A-Za-z-]+/)?compatibility\.md$")
# Plans and engineering notes (not the manual): developers' text.
PLAN_DOCS = re.compile(r"^docs/(?!manual/)[^/]+\.md$|^docs/(?:security|i18n)/")
SKIP_PARTS = ("/tests/", "/target/", "/node_modules/", "/.work/", "/fixtures/")
# Developers' text inside crates: build scripts, test files, dev tools.
SKIP_RUST = re.compile(r"(?:^|/)(?:build\.rs|tests\.rs|[^/]*_tests\.rs)$|^crates/[^/]+/examples/|^crates/[^/]+/benches/")


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


def skipped(rel):
    if rel.startswith(SKIP_PREFIXES) or PLAN_DOCS.match(rel):
        return True
    if IMPORTER_PAGE.match(rel):
        return True
    if rel.endswith(".rs") and SKIP_RUST.search(rel):
        return True
    if re.match(r"^crates/.*(?:README|CHANGELOG)\.md$", rel):
        return True
    if any(p in "/" + rel for p in SKIP_PARTS):
        return True
    return False


# ---- extracting the text a user reads, by kind of file ----

def rust_text(src):
    """(line, text) of every string literal of a Rust file (no comments, no
    char literals): the strings a program shows or a test compares."""
    out = []
    # (the unit tests are at the end of a file: what they compare is not
    # text a user reads)
    cut = re.search(r"^#\[cfg\(test\)\]\s*\n\s*mod\s+\w+\s*\{", src, re.M)
    if cut:
        src = src[:cut.start()]
    i, n, line = 0, len(src), 1
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth += 1
                    i += 2
                elif src.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    if src[i] == "\n":
                        line += 1
                    i += 1
        elif c == "r" and re.match(r'r#*"', src[i:i + 12]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            m = re.match(r'r(#*)"', src[i:])
            close = '"' + m.group(1)
            start = i + m.end()
            j = src.find(close, start)
            j = n if j < 0 else j
            body = src[start:j]
            for k, part in enumerate(body.split("\n")):
                out.append((line + k, part))
            line += body.count("\n")
            i = j + len(close)
        elif c == '"':
            j, start, buf = i + 1, i + 1, []
            while j < n and src[j] != '"':
                if src[j] == "\\":
                    j += 1
                j += 1
            body = src[start:j]
            # (a literal with no space in it is an identifier or a key a
            # program accepts — "rapidq", "rapidqCompatible" —, not wording)
            if re.search(r"\s", body):
                for k, part in enumerate(body.split("\n")):
                    out.append((line + k, part))
            line += body.count("\n")
            i = j + 1
        elif c == "'":
            m = re.match(r"'(?:\\.[^']{0,8}|[^'\\])'", src[i:i + 12])
            i += m.end() if m else 1
        else:
            i += 1
    return out


def basic_text(src, strings_only):
    """A BASIC source's text: all of it for an example (its header comment
    is read), only the string literals for RapidR Studio's own program (a
    comment there is the developers')."""
    lines = src.split("\n")
    if not strings_only:
        return [(n, l) for n, l in enumerate(lines, 1)]
    out = []
    for n, l in enumerate(lines, 1):
        if re.match(r"\s*(?:REM\b|')", l, re.I):
            continue
        buf, in_str = [], False
        for ch in l:
            if ch == '"':
                if in_str:
                    text = "".join(buf)
                    # (no space in it: an id or a key a program accepts)
                    if re.search(r"\s", text):
                        out.append((n, text))
                    buf = []
                in_str = not in_str
            elif in_str:
                buf.append(ch)
            elif ch == "'":
                break
    return out


TOML_STRUCTURAL = re.compile(
    r"""(?:\brapidq\s*=\s*"[^"]*"|\bfrom\s*=\s*"[^"]*"|\bsource\s*=\s*"[^"]*"|\borigin\s*=\s*"[^"]*")""")


def toml_text(src):
    out = []
    for n, l in enumerate(src.split("\n"), 1):
        if l.lstrip().startswith("#"):
            continue
        out.append((n, TOML_STRUCTURAL.sub("", l)))
    return out


def md_text(src, rel):
    lines = src.split("\n")
    if rel == "CHANGELOG.md":
        # The intro and [Unreleased] (the entries being written now): the
        # published versions below are history.
        head = []
        for n, l in enumerate(lines, 1):
            if re.match(r"^##\s+\[\d", l):
                break
            head.append((n, l))
        return head
    return [(n, l) for n, l in enumerate(lines, 1)]


def po_text(src):
    out = []
    for n, l in enumerate(src.split("\n"), 1):
        if l.startswith("#"):
            continue
        out.append((n, l))
    return out


def js_text(src):
    out = []
    for n, l in enumerate(src.split("\n"), 1):
        s = l.strip()
        if s.startswith("//") or s.startswith("*") or s.startswith("/*"):
            continue
        out.append((n, l))
    return out


def json_text(src):
    """A package.json's words: its descriptions and titles (not its ids,
    keywords or settings' keys)."""
    return [(n, l) for n, l in enumerate(src.split("\n"), 1)
            if re.search(r'"(?:description|markdownDescription|displayName|title|enumDescriptions?)"\s*:', l)]


def release_text(src):
    """Installer and package metadata a user sees (file-type names, package
    descriptions); the ids of file types (`RapidR.RapidQSource`, a MIME type,
    a UTI) are names, and so is a keywords line."""
    return [(n, l) for n, l in enumerate(src.split("\n"), 1) if not re.match(r"\s*(?:#|;|Keywords=)", l)]


def text_of(rel, src):
    ext = os.path.splitext(rel)[1].lower()
    if rel.startswith("utilities/"):
        if ext == ".md":
            return md_text(src, rel)
        return json_text(src) if rel.endswith("package.json") else None
    if rel.startswith("tools/release/"):
        return release_text(src) if ext in (".sh", ".plist", ".desktop", ".xml", ".iss", ".ps1", ".txt") else None
    if ext == ".md":
        return md_text(src, rel)
    if ext == ".toml":
        return toml_text(src) if rel.startswith("crates/rapidr-lang/data/") else None
    if ext in (".inc", ".rr", ".bas"):
        return basic_text(src, rel.startswith(("ide/", "crates/")))
    if ext in (".po", ".pot"):
        return po_text(src)
    if ext == ".rs":
        return rust_text(src)
    if ext in (".js", ".mjs", ".html", ".json", ".css"):
        return js_text(src)
    return None


def area_of(rel):
    if rel.startswith("docs/manual/es/"):
        return "manual (es)"
    if rel.startswith("docs/manual/reference/"):
        return "manual reference (generated)"
    if rel.startswith("docs/manual/"):
        return "manual (en)"
    if rel.startswith("crates/rapidr-lang/data/"):
        return "registry docs (toml)"
    if rel.startswith("ide/"):
        return "Studio (ide/)"
    if rel.startswith("examples/") and rel != "examples/README.md":
        return "examples"
    if rel.endswith(".md"):
        return "README / CHANGELOG / top-level"
    if rel.startswith(("crates/", "interpreter/")):
        return "Rust strings (kernel, runtimes, Studio)"
    if rel.startswith("utilities/"):
        return "VS Code extension"
    return "other"


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
    with open(ALLOWLIST, encoding="utf-8") as fh:
        for n, raw in enumerate(fh, 1):
            line = raw.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if " :: " in line:
                p, t = line.split(" :: ", 1)
            elif line.rstrip().endswith(" ::"):
                p, t = line.rstrip()[:-3], ""
            else:
                print(f"tools/rapidq-allowlist.txt:{n}: expected `path :: text`")
                sys.exit(2)
            entries.append(Entry(p.strip(), t.strip(), n))
    return entries


def scan():
    entries = load_allowlist()
    bad, allowed = [], []
    for rel in tracked_files():
        if skipped(rel):
            continue
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
        src = data.decode("utf-8", "replace")
        lines = text_of(rel, src)
        if lines is None:
            continue
        mine = [e for e in entries if e.covers(rel)]
        compat_line_left = 1 if COMPAT_PAGE.match(rel) else 0
        for n, line in lines:
            if not MENTION.search(line):
                continue
            rest = NAMES.sub("", line)
            if not MENTION.search(rest):
                continue
            if compat_line_left and "importing-rapidq.md" in line:
                compat_line_left -= 1
                allowed.append((rel, n, line.strip()))
                continue
            used = []
            for e in mine:
                if e.text == "":
                    used.append(e)
                    rest = ""
                    break
                if e.text.lower() in rest.lower():
                    rest = re.sub(re.escape(e.text), "", rest, flags=re.I)
                    used.append(e)
            if MENTION.search(rest):
                bad.append((rel, n, line.strip()))
            else:
                for e in used:
                    e.uses += 1
                allowed.append((rel, n, line.strip()))
    return entries, bad, allowed


def main():
    entries, bad, allowed = scan()
    if "--count" in sys.argv:
        counts = {}
        for rel, n, line in bad:
            counts.setdefault(area_of(rel), [0, 0])[0] += 1
        for rel, n, line in allowed:
            counts.setdefault(area_of(rel), [0, 0])[1] += 1
        print(f"{'area':44} {'new':>5} {'allowed':>8}")
        for a in sorted(counts):
            print(f"{a:44} {counts[a][0]:5} {counts[a][1]:8}")
        print(f"{'total':44} {sum(c[0] for c in counts.values()):5} {sum(c[1] for c in counts.values()):8}")
        return 0
    if "--list" in sys.argv:
        for rel, n, line in allowed:
            print(f"{rel}:{n}: {line[:160]}")
        return 0
    stale = [e for e in entries if e.uses == 0]
    status = 0
    if bad:
        status = 1
        print("RapidQ named in text a user reads (describe the behaviour in RapidR's own words;")
        print("only the importer's texts and the compatibility page's one line may name it — see")
        print("tools/rapidq-allowlist.txt):")
        for rel, n, line in bad:
            print(f"  {rel}:{n}: {line[:150]}")
    if stale:
        status = 1
        print("Unused entries in tools/rapidq-allowlist.txt (remove them):")
        for e in stale:
            print(f"  line {e.lineno}: {e.path} :: {e.text}")
    if status == 0:
        print(f"rapidq mentions: all ok ({len(allowed)} allowed uses, {len(entries)} allowlist entries)")
    return status


if __name__ == "__main__":
    sys.exit(main())
