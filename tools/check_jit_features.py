#!/usr/bin/env python3
"""The JIT's code generator keeps to what the legal review cleared
(docs/legal/review-2026-11-05-cranelift.md, C2 and C4):

- one reviewed release, pinned (the five crates at the version below);
- its optional features off: `disas` (capstone), `incremental-cache`
  (sha2, postcard), `souper-harvest`, `enable-serde`, `pulley`, and
  cranelift-jit's `wasmtime-unwinder`;
- the crates it brings are the reviewed ones (a new one needs a review
  first), for every desktop target — those RapidR has anyway without it
  (serde, syn … through feature unification) are its own business;
- no source file of them carries a Mozilla Public License header, other
  than the one the review found (regalloc2's src/lib.rs: C1, Robert's
  decision — reported here until it is closed);
- never in the web runtime, the web host or rapidr-runtime-core (native
  builds never embed the VM), nor in a 32-bit x86 or wasm build.

It reads what cargo resolves (`cargo metadata`, offline when it can) and
the crates' sources in cargo's registry.

    python3 tools/check_jit_features.py      (repo root; tools/regress.sh legal)
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

PINNED = "0.137.1"
ROOTS = ["cranelift-codegen", "cranelift-frontend", "cranelift-jit", "cranelift-module", "cranelift-native"]

FORBIDDEN_FEATURES = {
    "cranelift-codegen": {"disas", "incremental-cache", "souper-harvest", "enable-serde", "pulley", "all-arch", "trace-log"},
    "cranelift-jit": {"wasmtime-unwinder"},
    "cranelift-module": {"enable-serde"},
    "regalloc2": {"enable-serde", "trace-log"},
}

# The review's section 1: what the code generator brings (shipped and
# build-time). Anything else needs a review first.
REVIEWED = {
    "cranelift-codegen", "cranelift-frontend", "cranelift-jit", "cranelift-module", "cranelift-native",
    "cranelift-entity", "cranelift-bforest", "cranelift-control", "cranelift-codegen-shared",
    "cranelift-assembler-x64", "cranelift-bitset", "regalloc2", "target-lexicon",
    "wasmtime-internal-core", "wasmtime-internal-jit-icache-coherence", "gimli", "region", "arbitrary",
    "memmap2", "mach2", "anyhow", "bumpalo", "allocator-api2", "equivalent", "fnv", "foldhash",
    "hashbrown", "indexmap", "libc", "libm", "log", "rustc-hash", "smallvec", "bitflags",
    "windows-sys", "windows-link", "windows-targets",
    "windows_aarch64_msvc", "windows_x86_64_msvc", "windows_aarch64_gnullvm", "windows_x86_64_gnu", "windows_x86_64_gnullvm",
    "windows_i686_msvc", "windows_i686_gnu", "windows_i686_gnullvm", "windows_aarch64_gnu",
    # (build time only: never in a program)
    "cranelift-codegen-meta", "cranelift-assembler-x64-meta", "cranelift-isle", "cranelift-srcgen", "heck",
}

TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
# (where the JIT must not be)
NO_JIT_TARGETS = ["wasm32-unknown-unknown", "i686-pc-windows-msvc"]
NO_JIT_PACKAGES = ["rapidr-runtime-web", "rapidr-vm-host-web", "rapidr-runtime-core", "rapidr-vm", "rapidr-bytecode", "rapidr-value"]
JIT_CRATES = re.compile(r"^(cranelift|regalloc2$|wasmtime-internal)")

MPL_HEADER = "subject to the terms of the Mozilla Public"
# (C1: known, Robert's decision pending — reported, not failed)
MPL_KNOWN = {("regalloc2", "src/lib.rs")}


def metadata(platform, all_features=False):
    cmd = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", platform]
    if all_features:
        cmd.append("--all-features")
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        cmd.remove("--offline")
        out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"check_jit_features: cargo metadata failed:\n{out.stderr[-2000:]}")
    return json.loads(out.stdout)


def reachable(m, roots, kinds=(None, "build"), blocked=()):
    """Packages reachable from `roots` (names) through normal and build
    dependencies (not through `blocked`)."""
    names = {p["id"]: p for p in m["packages"]}
    nodes = {n["id"]: n for n in m["resolve"]["nodes"]}
    start = [i for i, p in names.items() if p["name"] in roots]
    seen = set()
    while start:
        i = start.pop()
        if i in seen or (names[i]["name"] in blocked and names[i]["name"] not in roots):
            continue
        seen.add(i)
        for d in nodes[i].get("deps", []):
            if any(k.get("kind") in kinds for k in d.get("dep_kinds", [])):
                start.append(d["pkg"])
    return {i: names[i] for i in seen}, nodes


def main():
    problems, notes = [], []
    seen_roots = set()
    for platform in TARGETS:
        for all_features in (False, True):
            m = metadata(platform, all_features)
            where = f"{platform}{' --all-features' if all_features else ''}"
            pkgs, nodes = reachable(m, set(ROOTS))
            # (what the workspace has without the JIT)
            members = {p["name"] for p in m["packages"] if p["source"] is None}
            baseline, _ = reachable(m, members - {"rapidr-jit"}, blocked={"rapidr-jit", *ROOTS})
            for i, p in pkgs.items():
                name, version = p["name"], p["version"]
                if name in ROOTS:
                    seen_roots.add(name)
                    if version != PINNED:
                        problems.append(f"{name} {version}: not the reviewed release {PINNED} ({where})")
                if name not in REVIEWED and i not in baseline:
                    problems.append(f"{name} {version}: not in the review's list ({where})")
                bad = FORBIDDEN_FEATURES.get(name, set()) & set(nodes[i].get("features", []))
                for f in sorted(bad):
                    problems.append(f"{name} {version}: feature `{f}` is on ({where})")
            if not all_features and platform == TARGETS[0]:
                # (source headers: every crate of the tree, once)
                for p in pkgs.values():
                    src = os.path.dirname(p["manifest_path"])
                    for dirpath, _, files in os.walk(src):
                        if "/target" in dirpath:
                            continue
                        for fn in files:
                            if not fn.endswith(".rs"):
                                continue
                            path = os.path.join(dirpath, fn)
                            try:
                                text = open(path, encoding="utf-8", errors="replace").read(4000)
                            except OSError:
                                continue
                            if MPL_HEADER in text:
                                rel = os.path.relpath(path, src)
                                if (p["name"], rel) in MPL_KNOWN:
                                    notes.append(f"{p['name']} {p['version']} {rel}: MPL-2.0 header (C1, Robert's decision; not mergeable until closed)")
                                else:
                                    problems.append(f"{p['name']} {p['version']} {rel}: a Mozilla Public License header")
    missing = set(ROOTS) - seen_roots
    if missing:
        problems.append(f"not in the graph, update this check: {', '.join(sorted(missing))}")
    # (C4: where the JIT must never be)
    for platform in [TARGETS[0]] + NO_JIT_TARGETS:
        m = metadata(platform)
        roots = set(NO_JIT_PACKAGES) if platform == TARGETS[0] else {p["name"] for p in m["packages"] if p["source"] is None}
        pkgs, _ = reachable(m, roots, kinds=(None,))
        for p in pkgs.values():
            if not JIT_CRATES.match(p["name"]):
                continue
            if platform == TARGETS[0]:
                problems.append(f"{p['name']} is in the graph of {', '.join(sorted(NO_JIT_PACKAGES))} ({platform})")
            else:
                problems.append(f"{p['name']} is in a {platform} build")
    if problems:
        print("check_jit_features: FAILED (each needs a new legal review first: docs/legal/review-2026-11-05-cranelift.md)")
        for p in sorted(set(problems)):
            print("  " + p)
        for n in sorted(set(notes)):
            print("  note: " + n)
        sys.exit(1)
    for n in sorted(set(notes)):
        print("note: " + n)
    print(f"jit features: all ok (code generator {PINNED} pinned, optional features off, only the reviewed crates, desktop only: not in the web runtime, rapidr-runtime-core, 32-bit x86 or wasm)")


if __name__ == "__main__":
    main()
