#!/usr/bin/env python3
"""Checks the links in RapidR's Markdown documents.

    python3 tools/check_doc_links.py [paths…]     (default: README.md, examples/README.md, docs/)

For every `[text](target)` and `![alt](target)` that isn't a web address:
the file must exist, and a `#anchor` must be a heading of that file (GitHub's
way of naming headings) or an `<a id="…">` in it. Links inside code blocks and code spans are not
links. Exits with 1 and says where when something is broken.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINK = re.compile(r"!?\[(?:[^\[\]]|\[[^\]]*\])*\]\(\s*(<[^>]*>|[^)\s]*(?:\([^)\s]*\)[^)\s]*)*)(?:\s+\"[^\"]*\")?\s*\)")
HEADING = re.compile(r"^ {0,3}(#{1,6})\s+(.*?)\s*#*\s*$")


def slug(text):
    """GitHub's anchor for a heading's text."""
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[*_~]", lambda m: m.group(0) if m.group(0) == "_" else "", text)
    text = text.strip().lower()
    out = []
    for ch in text:
        if ch.isalnum() or ch in "_-":
            out.append(ch)
        elif ch == " ":
            out.append("-")
    return "".join(out)


def read_lines(path):
    with open(path, encoding="utf-8") as f:
        return f.read().split("\n")


def prose(lines):
    """(line number, text) of every line outside fenced code blocks."""
    fence = None
    for n, line in enumerate(lines, 1):
        m = re.match(r"^ {0,3}(`{3,}|~{3,})", line)
        if m:
            if fence is None:
                fence = m.group(1)[0] * 3
            elif line.strip().startswith(fence):
                fence = None
            continue
        if fence is None:
            yield n, line


_anchors = {}


def anchors(path):
    if path not in _anchors:
        seen, out = {}, set()
        for _, line in prose(read_lines(path)):
            out.update(a.lower() for a in re.findall(r"<a\s+(?:id|name)=[\"']([^\"']+)[\"']", line))
            m = HEADING.match(line)
            if m:
                s = slug(m.group(2))
                k = seen.get(s, 0)
                seen[s] = k + 1
                out.add(s if k == 0 else f"{s}-{k}")
        _anchors[path] = out
    return _anchors[path]


def markdown_files(paths):
    for p in paths:
        p = os.path.join(ROOT, p) if not os.path.isabs(p) else p
        if os.path.isfile(p):
            yield p
            continue
        for d, _, files in os.walk(p):
            for f in sorted(files):
                if f.endswith(".md"):
                    yield os.path.join(d, f)


def check(paths):
    problems = []
    count = 0
    for path in sorted(set(markdown_files(paths))):
        rel = os.path.relpath(path, ROOT)
        for n, line in prose(read_lines(path)):
            text = re.sub(r"`[^`]*`", "", line)
            for m in LINK.finditer(text):
                target = m.group(1).strip("<>")
                if not target or re.match(r"^[a-z][a-z0-9+.-]*:", target, re.I):
                    continue
                count += 1
                file_part, _, frag = target.partition("#")
                dest = path if file_part == "" else os.path.normpath(os.path.join(os.path.dirname(path), file_part.replace("%20", " ")))
                if not os.path.exists(dest):
                    problems.append(f"{rel}:{n}: {target}: no such file")
                elif frag and dest.endswith(".md") and frag.lower() not in anchors(dest):
                    problems.append(f"{rel}:{n}: {target}: no heading #{frag} in {os.path.relpath(dest, ROOT)}")
    return count, problems


def main():
    paths = sys.argv[1:] or ["README.md", "examples/README.md", "docs"]
    count, problems = check(paths)
    if problems:
        sys.exit(f"check_doc_links: {len(problems)} broken of {count} links:\n  " + "\n  ".join(problems))
    print(f"check_doc_links: {count} links, all good")


if __name__ == "__main__":
    main()
