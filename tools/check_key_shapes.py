#!/usr/bin/env python3
"""No API key's shape in the repository, real or made up.

A made-up test key in a real key's shape gets a push refused by the host's
secret scanning (2026-10-22: a test of chats' key removal) and looks like a
leak to anyone reading the code. Tests build them from two pieces instead:
concat!("sk", "-…") in Rust, "sk" + "-…" in JavaScript, "sk" + "-…" in BASIC.

Every tracked text file is read; a match prints masked (its first 6
characters) with its file and line, and the exit code is 1. Words that only
look like the start of one ("sk-learn", lower-case words joined by hyphens)
pass.

    python3 tools/check_key_shapes.py
"""

import re
import subprocess
import sys

SHAPES = [
    ("Anthropic", r"sk-ant-[A-Za-z0-9]+-[A-Za-z0-9_-]{20,}"),
    ("OpenAI", r"sk-(?:proj|svcacct|admin)-[A-Za-z0-9_-]{20,}"),
    ("sk- key", r"sk-[A-Za-z0-9]{32,}"),
    ("Google", r"AIza[0-9A-Za-z_-]{35}"),
    ("GitHub", r"(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{40,}"),
    ("AWS", r"AKIA[0-9A-Z]{16}"),
    ("Slack", r"xox[abprs]-[A-Za-z0-9-]{10,}"),
]
PATTERN = re.compile("|".join(f"(?P<g{i}>{p})" for i, (_, p) in enumerate(SHAPES)))


def words_only(s: str) -> bool:
    # ("sk-the-quick-brown-fox": lower-case words, no digits — not a key)
    return re.fullmatch(r"[a-z]+(?:-[a-z]+)+", s) is not None


def main() -> int:
    files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout.split(b"\0")
    found = 0
    for name in filter(None, files):
        path = name.decode("utf-8", "replace")
        try:
            text = open(path, encoding="utf-8").read()
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            for m in PATTERN.finditer(line):
                if words_only(m.group(0)):
                    continue
                kind = SHAPES[int(m.lastgroup[1:])][0]
                print(f"{path}:{n}: a {kind} key's shape: {m.group(0)[:6]}… (build it from two pieces)")
                found += 1
    if found:
        print(f"key shapes: {found} found")
        return 1
    print("key shapes: none")
    return 0


if __name__ == "__main__":
    sys.exit(main())
