#!/usr/bin/env python3
"""Profiles with macOS `sample` (docs/speed-audit-2026-11.md): start a
command, wait WARM seconds, sample its process for SECS seconds, stop it (by
its PID only), print the top-of-stack table and keep the whole report
(OUT_PREFIX.sample.txt) — then read a report back: the inclusive share of
named functions, what a function's samples are spent in, or the self time
inside a function's calls, its names demangled (c++filt).

  python3 tools/bench/sample_profile.py OUT_PREFIX WARM SECS -- cmd args...
  python3 tools/bench/sample_profile.py --pid PID OUT_PREFIX SECS
  python3 tools/bench/sample_profile.py --summary REPORT [N]
  python3 tools/bench/sample_profile.py --incl REPORT name-part...
  python3 tools/bench/sample_profile.py --children REPORT name-part [N]
  python3 tools/bench/sample_profile.py --flat REPORT name-part [N]

A release `rapidr` keeps its symbols; a native build is stripped (build the
generated project with CARGO_PROFILE_RELEASE_STRIP=none to read it). The
command runs from the repository's folder with RAPIDR_PRINT_TO,
RAPIDR_REGISTRY and RAPIDR_AI_KEYSTORE pointing at tests/results/bench/.
"""
import os
import re
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
WT = os.path.dirname(os.path.dirname(HERE))
WORK = os.path.join(WT, "tests", "results", "bench")
ENV = dict(os.environ)
ENV.setdefault("RAPIDR_REGISTRY", os.path.join(WORK, "registry.reg"))
ENV.setdefault("RAPIDR_AI_KEYSTORE", "memory")
ENV.setdefault("RAPIDR_PRINT_TO", os.path.join(WORK, "print"))


def top_of_stack(report, n=30):
    lines = report.splitlines()
    try:
        i = next(k for k, l in enumerate(lines) if l.startswith("Sort by top of stack"))
    except StopIteration:
        return []
    out = []
    for l in lines[i + 1:]:
        if not l.strip():
            if out:
                break
            continue
        m = re.match(r"\s+(.*?)\s+(\d+)\s*$", l)
        if m:
            out.append((m.group(1), int(m.group(2))))
        if len(out) >= n:
            break
    return out


def demangle(names):
    syms = [re.sub(r"\s+\(in .*\)$", "", n) for n in names]
    r = subprocess.run(["c++filt"], input="\n".join(syms), capture_output=True, text=True)
    out = r.stdout.splitlines()
    return out if len(out) == len(names) else syms


def summarize(path, n=30):
    report = open(path, errors="replace").read()
    rows = top_of_stack(report, n)
    total = sum(c for _, c in top_of_stack(report, 100000))
    print(f"-- top of stack ({total} samples) from {path}")
    names = demangle([r[0] for r in rows])
    for (name, c), d in zip(rows, names):
        print(f"{c:7d} {100.0 * c / max(total, 1):5.1f}%  {d[:160]}")


def inclusive(path, patterns, thread_filter="Main Thread"):
    """Inclusive samples of the call-graph nodes whose (demangled) name
    contains each pattern, not counted twice inside themselves."""
    report = open(path, errors="replace").read()
    lines = report.splitlines()
    try:
        i = next(k for k, l in enumerate(lines) if l.startswith("Call graph:"))
    except StopIteration:
        return
    nodes = []
    for l in lines[i + 1:]:
        if l.startswith("Total number in stack") or l.startswith("Sort by top"):
            break
        m = re.match(r"^([\s+!:|]*)(\d+) (.*)$", l)
        if not m:
            continue
        nodes.append((len(m.group(1)), int(m.group(2)), m.group(3)))
    # demangle every distinct symbol once
    syms = sorted({re.sub(r"\s+\(in .*$", "", n[2]) for n in nodes})
    r = subprocess.run(["c++filt"], input="\n".join(syms), capture_output=True, text=True)
    dm = dict(zip(syms, r.stdout.splitlines()))
    total_main = sum(c for d, c, n in nodes if d == min(x[0] for x in nodes) and "com.apple.main-thread" in n) or 1
    for pat in patterns:
        tot = 0
        stack = []  # (depth, matched)
        for d, c, n in nodes:
            while stack and stack[-1][0] >= d:
                stack.pop()
            name = dm.get(re.sub(r"\s+\(in .*$", "", n), n)
            hit = pat in name
            if hit and not any(s[1] for s in stack):
                tot += c
            stack.append((d, hit))
        print(f"{tot:7d} {100.0 * tot / total_main:5.1f}%  incl. {pat}")


def parse_graph(path):
    lines = open(path, errors="replace").read().splitlines()
    i = next(k for k, l in enumerate(lines) if l.startswith("Call graph:"))
    nodes = []
    for l in lines[i + 1:]:
        if l.startswith("Total number in stack") or l.startswith("Sort by top"):
            break
        m = re.match(r"^([\s+!:|]*)(\d+) (.*)$", l)
        if m:
            nodes.append((len(m.group(1)), int(m.group(2)), m.group(3)))
    syms = sorted({re.sub(r"\s+\(in .*$", "", n[2]) for n in nodes})
    r = subprocess.run(["c++filt"], input="\n".join(syms), capture_output=True, text=True)
    dm = dict(zip(syms, r.stdout.splitlines()))
    return [(d, c, dm.get(re.sub(r"\s+\(in .*$", "", n), n)) for d, c, n in nodes]


def children(path, pat, n=25):
    """What the outermost nodes matching `pat` spend their samples in: their
    direct children aggregated by name (self time as '<self>')."""
    nodes = parse_graph(path)
    agg = {}
    total = 0
    k = 0
    while k < len(nodes):
        d, c, name = nodes[k]
        if pat in name:
            total += c
            kids = 0
            j = k + 1
            child_depth = None
            while j < len(nodes) and nodes[j][0] > d:
                if child_depth is None:
                    child_depth = nodes[j][0]
                if nodes[j][0] == child_depth:
                    short = re.sub(r"\s+\+ \d+.*$", "", nodes[j][2])
                    agg[short] = agg.get(short, 0) + nodes[j][1]
                    kids += nodes[j][1]
                j += 1
            agg["<self>"] = agg.get("<self>", 0) + c - kids
            k = j
        else:
            k += 1
    print(f"-- under {pat}: {total} samples")
    for name, c in sorted(agg.items(), key=lambda x: -x[1])[:n]:
        print(f"{c:7d} {100.0 * c / max(total, 1):5.1f}%  {name[:170]}")


def flat(path, pat, n=30, also=None):
    """Self time by function inside the subtrees of the outermost nodes
    matching `pat`; and inclusive time of `also` names inside them."""
    nodes = parse_graph(path)
    selfs = {}
    incl = {}
    total = 0
    k = 0
    while k < len(nodes):
        d, c, name = nodes[k]
        if pat in name:
            total += c
            j = k
            end = k + 1
            while end < len(nodes) and nodes[end][0] > d:
                end += 1
            for x in range(k, end):
                dx, cx, nx = nodes[x]
                kids = 0
                y = x + 1
                cd = None
                while y < end and nodes[y][0] > dx:
                    if cd is None:
                        cd = nodes[y][0]
                    if nodes[y][0] == cd:
                        kids += nodes[y][1]
                    y += 1
                short = re.sub(r"\s+\+ \d+.*$", "", nx)
                short = re.sub(r"::<.*$", "", short)
                selfs[short] = selfs.get(short, 0) + cx - kids
            k = end
        else:
            k += 1
    print(f"-- self time inside {pat}: {total} samples")
    for name, c in sorted(selfs.items(), key=lambda x: -x[1])[:n]:
        print(f"{c:7d} {100.0 * c / max(total, 1):5.1f}%  {name[:170]}")


def main():
    a = sys.argv[1:]
    if a[0] == "--flat":
        flat(a[1], a[2], int(a[3]) if len(a) > 3 else 30)
        return
    if a[0] == "--children":
        children(a[1], a[2], int(a[3]) if len(a) > 3 else 25)
        return
    if a[0] == "--incl":
        inclusive(a[1], a[2:])
        return
    if a[0] == "--summary":
        summarize(a[1], int(a[2]) if len(a) > 2 else 30)
        return
    if a[0] == "--pid":
        pid, prefix, secs = int(a[1]), a[2], a[3]
        out = prefix + ".sample.txt"
        subprocess.run(["sample", str(pid), str(secs), "-mayDie", "-file", out], capture_output=True)
        summarize(out)
        return
    prefix, warm, secs = a[0], float(a[1]), a[2]
    cmd = a[a.index("--") + 1:]
    os.makedirs(ENV["RAPIDR_PRINT_TO"], exist_ok=True)
    p = subprocess.Popen(cmd, cwd=WT, env=ENV, stdout=open(prefix + ".stdout.txt", "w"), stderr=subprocess.STDOUT)
    time.sleep(warm)
    out = prefix + ".sample.txt"
    subprocess.run(["sample", str(p.pid), str(secs), "-mayDie", "-file", out], capture_output=True)
    if p.poll() is None:
        os.kill(p.pid, signal.SIGTERM)
        try:
            p.wait(5)
        except subprocess.TimeoutExpired:
            os.kill(p.pid, signal.SIGKILL)
            p.wait()
    summarize(out)


if __name__ == "__main__":
    main()
