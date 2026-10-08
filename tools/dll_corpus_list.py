#!/usr/bin/env python3
"""The corpus programs that declare DLL routines — in their own
source, or in any include they pull in (their folder's, or RapidQ's
include folder), transitively — OLE / DirectX ones included
(`--no-ole-dx` leaves them out). One relative path per line; the count on
stderr. `--direct` lists only rapidq_corpus.py's `dll` category (own
source + local includes). docs/windows-dll-calls.md §6."""
import os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rapidq_corpus as rc

root = os.path.expanduser("~/Downloads/Rapidq/examples")
incdir = os.path.expanduser("~/Downloads/Rapidq/include")
rc.CORPUS_ROOT = root
INC = re.compile(r'^\s*\$INCLUDE\s+["<]([^">]+)[">]', re.I | re.M)


def ci_find(d, rel):
    """A file under d named rel, case-insensitively."""
    p = d
    for part in rel.replace("\\", "/").split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            p = os.path.dirname(p)
            continue
        try:
            names = {n.lower(): n for n in os.listdir(p)}
        except OSError:
            return None
        n = names.get(part.lower())
        if n is None:
            return None
        p = os.path.join(p, n)
    return p if os.path.isfile(p) else None


def all_text(path, seen):
    if path in seen:
        return ""
    seen.add(path)
    try:
        src = open(path, encoding="latin-1").read()
    except OSError:
        return ""
    texts = [src]
    for inc in INC.findall(src):
        f = ci_find(os.path.dirname(path), inc) or ci_find(incdir, inc)
        if f:
            texts.append(all_text(f, seen))
    return "\n".join(texts)


direct = "--direct" in sys.argv
out = []
for d, _, fs in os.walk(root):
    for f in fs:
        if not f.lower().endswith(".bas"):
            continue
        p = os.path.join(d, f)
        if direct:
            if rc.category(p, []) == "dll":
                out.append(os.path.relpath(p, root))
            continue
        src = all_text(p, set())
        if ("--no-ole-dx" in sys.argv) and (rc.OLE_RE.search(src) or rc.DIRECTX_RE.search(src)):
            continue
        if rc.DLL_RE.search(src):
            out.append(os.path.relpath(p, root))
for r in sorted(out):
    print(r)
print(len(out), file=sys.stderr)
