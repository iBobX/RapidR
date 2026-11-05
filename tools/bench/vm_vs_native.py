#!/usr/bin/env python3
"""The programs in tools/bench/vm/ on the interpreter (`rapidr run`) and
built native, side by side (docs/speed-audit-2026-11.md): each one's CPU
time (the fastest of --runs runs: what the program itself costs, steady on
a machine busy with other work), its own `ms` line (TIMER around the work,
when it prints one), the interpreter / native ratio, and its answer (the
two must print the same).

  python3 tools/bench/vm_vs_native.py [--runs N] [--no-native] [--only a,b] [--json FILE]

Native builds go to tests/results/bench/native/<program>/ (rebuilt when the
program is newer); the build cache is RAPIDR_BUILD_CACHE's, else the
system's. Nothing reaches a printer or the user's registry: RAPIDR_PRINT_TO,
RAPIDR_REGISTRY and RAPIDR_AI_KEYSTORE point at tests/results/bench/.
"""
import json
import os
import re
import resource
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
PROGS = os.path.join(HERE, "vm")
WORK = os.path.join(ROOT, "tests", "results", "bench")
NATIVE = os.path.join(WORK, "native")
RAPIDR = os.environ.get("RAPIDR") or os.path.join(ROOT, "rapidr")

ENV = dict(os.environ)
ENV.setdefault("RAPIDR_REGISTRY", os.path.join(WORK, "registry.reg"))
ENV.setdefault("RAPIDR_PRINT_TO", os.path.join(WORK, "print"))
ENV.setdefault("RAPIDR_AI_KEYSTORE", "memory")

# (a window, or the start-up measure: vm_bench's, not timed here)
SKIP = {"idle_form", "hello"}


def run_cpu(cmd):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True, env=ENV, cwd=ROOT)
    wall = time.perf_counter() - t0
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    return wall, cpu, p.returncode, p.stdout.decode(errors="replace"), p.stderr.decode(errors="replace")


def own_ms(out):
    m = re.findall(r"^ms\s+(-?\d+)", out, re.M)
    return int(m[-1]) if m else None


def answer(out):
    return " | ".join(l for l in out.splitlines() if l.strip() and not l.startswith("ms "))[:80]


def measure(cmd, runs):
    best = None
    for _ in range(runs):
        wall, cpu, rc, out, err = run_cpu(cmd)
        if rc != 0:
            return {"error": f"exit {rc}: {err.strip()[:200]}"}
        r = {"cpu_ms": round(cpu * 1000, 1), "wall_ms": round(wall * 1000, 1), "own_ms": own_ms(out), "answer": answer(out)}
        if best is None or r["cpu_ms"] < best["cpu_ms"]:
            best = r
    return best


def build_native(name):
    out_dir = os.path.join(NATIVE, name)
    os.makedirs(out_dir, exist_ok=True)
    exe = os.path.join(out_dir, name + (".exe" if os.name == "nt" else ""))
    src = os.path.join(PROGS, name + ".rr")
    if not (os.path.exists(exe) and os.path.getmtime(exe) > os.path.getmtime(src)):
        r = subprocess.run([RAPIDR, "build", src, "--output", out_dir, "--no-bundle"], env=ENV, cwd=ROOT, capture_output=True, text=True)
        if r.returncode != 0 or not os.path.exists(exe):
            print(f"  native build of {name} failed: {r.stdout[-400:]} {r.stderr[-800:]}", file=sys.stderr)
            return None
    # (a fresh macOS binary's first launch costs ~0.2 s: once before timing)
    subprocess.run([exe], env=ENV, cwd=ROOT, capture_output=True)
    return exe


def main():
    args = sys.argv[1:]
    runs, native, only, json_out = 3, "--no-native" not in args, [], None
    i = 0
    while i < len(args):
        if args[i] == "--runs":
            runs = int(args[i + 1]); i += 1
        elif args[i] == "--only":
            only = args[i + 1].split(","); i += 1
        elif args[i] == "--json":
            json_out = args[i + 1]; i += 1
        i += 1
    os.makedirs(ENV["RAPIDR_PRINT_TO"], exist_ok=True)
    names = sorted(f[:-3] for f in os.listdir(PROGS) if f.endswith(".rr") and f[:-3] not in SKIP)
    if only:
        names = [n for n in names if n in only]
    results = {}
    print(f"{'program':14} {'vm cpu':>9} {'vm own':>8} {'nat cpu':>9} {'nat own':>8} {'vm/nat':>7}  answer   (ms; the fastest of {runs})")
    for n in names:
        vm = measure([RAPIDR, "run", os.path.join(PROGS, n + ".rr")], runs)
        nat = None
        if native:
            exe = build_native(n)
            nat = measure([exe], runs) if exe else {"error": "build failed"}
        results[n] = {"vm": vm, "native": nat}

        def cell(r, k):
            if r is None:
                return "-"
            if "error" in r:
                return "ERR"
            return "-" if r.get(k) is None else f"{r[k]}"

        ratio = "-"
        if nat and "error" not in nat and "error" not in vm:
            a = vm["own_ms"] if vm["own_ms"] is not None else vm["cpu_ms"]
            b = nat["own_ms"] if nat["own_ms"] is not None else nat["cpu_ms"]
            ratio = f"{a / max(b, 0.5):.1f}x"
        said = vm.get("answer", vm.get("error", ""))
        if nat and "error" not in nat and "error" not in vm and nat["answer"] != vm["answer"]:
            said += f"  (NATIVE DIFFERS: {nat['answer']})"
        if nat and "error" in nat:
            said += f"  (native: {nat['error'][:100]})"
        print(f"{n:14} {cell(vm, 'cpu_ms'):>9} {cell(vm, 'own_ms'):>8} {cell(nat, 'cpu_ms'):>9} {cell(nat, 'own_ms'):>8} {ratio:>7}  {said}", flush=True)
    if json_out:
        with open(json_out, "w") as fh:
            json.dump(results, fh, indent=1)


if __name__ == "__main__":
    main()
