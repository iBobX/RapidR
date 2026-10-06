#!/usr/bin/env python3
"""For each toolchain object a link kept (link_audit.sh's list), its source
and the licence its header states: mingw-w64's archive members from the
mingw-w64-crt source, LLVM-MinGW's start-up objects and libunwind by name.
Cephes-derived, Wine-imported and (L)GPL code is flagged, and so is any
licence the Windows notices don't carry (notices.rs, "mingw-w64 runtime"
and "LLVM libunwind"); either makes the exit status 1.

    python3 member_licences.py <list file> <mingw-w64-crt source folder>
"""
import os, re, sys

list_file, crt = sys.argv[1], sys.argv[2]
# what the Windows notices carry: change both together
NOTICED = {"public domain", "ZPL-2.1", "HPND (gdtoa)", "Apache-2.0 WITH LLVM-exception"}
# mingw-w64's start-up objects (crt2.o is crt/crtexe.c) and LLVM's libunwind
STARTUP = {"crt2.o": "crtexe", "crtbegin.o": "crtbegin", "crtend.o": "crtend"}
LIBUNWIND = re.compile(r"^(libunwind|Unwind[\w-]*)\.(c|cpp|S)\.obj$")

sources = {}
for base, dirs, files in os.walk(crt):
    if "testcases" in base:
        continue
    for f in files:
        stem, ext = os.path.splitext(f)
        if ext in (".c", ".S", ".s", ".cpp"):
            sources.setdefault(stem, os.path.join(base, f))
flags = re.compile(r"cephes|moshier|\bwine\b|lesser general|gnu general|\bl?gpl\b", re.I)


def licence(head):
    if re.search(r"public domain|without restriction of copyright", head, re.I):
        return "public domain"
    if "David M. Gay" in head:
        return "HPND (gdtoa)"
    if re.search(r"copyright|license|licence|permission", head, re.I):
        return "header: see the file"
    return "ZPL-2.1"   # no notice of its own: mingw-w64-crt's COPYING (ZPL-2.1)


bad = 0
for line in open(list_file):
    name = os.path.basename(line.strip())
    if not name:
        continue
    if LIBUNWIND.match(name):
        print(f"libunwind: {name}: LLVM: Apache-2.0 WITH LLVM-exception")
        continue
    m = re.match(r"lib\w+?_lib(\w+?)_a-(.+)\.o$", name)
    if m:
        lib, stem = "lib" + m.group(1), m.group(2)
    elif name in STARTUP:
        lib, stem = "start-up object " + name, STARTUP[name]
    else:
        bad += 1
        print(f"{name}: not mingw-w64's, not libunwind's (compiler-rt? another toolchain library?)  FLAG: unknown")
        continue
    src = sources.get(stem)
    if not src:
        bad += 1
        print(f"{lib}: {stem}: source not found  FLAG: unknown")
        continue
    text = open(src, errors="replace").read()
    kind = licence(text[:3000])
    hit = sorted(set(h.lower() for h in flags.findall(text))) + (["cephes_mconf.h"] if "cephes_mconf.h" in text else [])
    if kind not in NOTICED:
        hit.append(f"licence not in the notices ({kind})")
    if hit:
        bad += 1
    print(f"{lib}: {stem}: {os.path.relpath(src, crt)}: {kind}{'  FLAG: ' + ', '.join(hit) if hit else ''}")
print(f"{bad} flagged")
sys.exit(1 if bad else 0)
