#!/usr/bin/env python3
"""Check that every output RapidR builds carries its THIRD-PARTY-NOTICES.txt
and that the file lists every crate compiled into it (docs/licensing.md).

Builds a small program each way and looks at what comes out:

  interpreted executable (build --interp)   notices beside the executable
  web bundle (bundle-bc)                     notices in the zip's root, linked
                                             from index.html
  bytecode (build-bc)                        the .rrbc holds only the program
  native executable (build), --native        notices beside the executable
  native web build (build --web), --web      notices in <stem>_web/, linked

then, for every kind (`rapidr notices <kind>`: each desktop <os>-<arch>, the
web, RapidR's own tools), checks what `cargo tree` says that kind compiles in
— computed here on its own, so a mistake in the CLI's list of roots or
targets shows: every crate under a permissive licence (PERMISSIVE: no
copyleft, no data licences), none of the crates RapidR replaced (BANNED,
crates.io's KDE protocol bindings), and every crate in the file.

    python3 tools/check_notices.py [--rapidr PATH] [--native] [--web]

Programs run with RAPIDR_PRINT_TO / RAPIDR_REGISTRY pointed at scratch.
"""

import argparse
import os
import platform
import re
import subprocess
import sys
import tempfile
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NOTICES = "THIRD-PARTY-NOTICES.txt"

# What each kind compiles in: the workspace crates (and features) and the
# Rust targets (Windows: both toolchains). Kept apart from notices.rs on
# purpose: this is the check of it.
DESKTOP_ROOTS = (["rapidr-runtime-core", "rapidr-runner-stub"], [])
KINDS = {
    "macos-aarch64": (DESKTOP_ROOTS, ["aarch64-apple-darwin"]),
    "macos-x86_64": (DESKTOP_ROOTS, ["x86_64-apple-darwin"]),
    "windows-x86_64": (DESKTOP_ROOTS, ["x86_64-pc-windows-gnullvm", "x86_64-pc-windows-msvc"]),
    "windows-aarch64": (DESKTOP_ROOTS, ["aarch64-pc-windows-gnullvm", "aarch64-pc-windows-msvc"]),
    "linux-x86_64": (DESKTOP_ROOTS, ["x86_64-unknown-linux-gnu"]),
    "linux-aarch64": (DESKTOP_ROOTS, ["aarch64-unknown-linux-gnu"]),
    "web": ((["rapidr-vm-host-web", "rapidr-runtime-web"], ["rapidr-runtime-web/kernel"]), ["wasm32-unknown-unknown"]),
    "tools-macos": ((["rapidr-cli", "rapidr-launcher"], []), ["aarch64-apple-darwin", "x86_64-apple-darwin"]),
    "tools-windows": ((["rapidr-cli", "rapidr-launcher"], []), ["x86_64-pc-windows-gnullvm", "x86_64-pc-windows-msvc", "aarch64-pc-windows-gnullvm", "aarch64-pc-windows-msvc"]),
    "tools-linux": ((["rapidr-cli", "rapidr-launcher"], []), ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]),
}

# The only licences a crate compiled into a program may be used under
# (docs/licensing.md §4), listed here on their own: notices.rs' ALLOWED and
# deny.toml's allowlist must agree with it, and every kind's graph must keep
# to it.
PERMISSIVE = {"MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
              "0BSD", "BSL-1.0", "Unlicense", "Unicode-3.0", "CC0-1.0"}

# Crates that must never be in a program's graph again (what was replaced:
# docs/licensing.md).
BANNED = {"ring", "rustls", "aws-lc-rs", "aws-lc-sys", "openssl-src", "webpki-roots", "symphonia", "symphonia-core",
          "symphonia-bundle-mp3", "font-kit", "dwrote", "option-ext", "freetype-sys"}

failures = []


def check(cond, what):
    print(("  ok  " if cond else "  FAIL ") + what)
    if not cond:
        failures.append(what)


def host_target():
    os_name = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}[platform.system()]
    arch = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "AMD64": "x86_64"}[platform.machine()]
    return f"{os_name}-{arch}"


GRAPHS = {}


def graph_licences(kind):
    """{(name, version): licence expression} of every crates.io package
    `kind` compiles in, and the set of path packages (RapidR's own)."""
    if kind in GRAPHS:
        return GRAPHS[kind]
    (roots, features), triples = KINDS[kind]
    out, own = {}, set()
    for t in triples:
        cmd = ["cargo", "tree", "--quiet", "--locked", "-e", "normal", "--prefix", "none", "-f", "{p}|{l}", "--target", t]
        for r in roots:
            cmd += ["-p", r]
        for f in features:
            cmd += ["--features", f]
        for line in subprocess.check_output(cmd, cwd=ROOT, text=True).splitlines():
            package, _, licence = line.partition("|")
            parts = package.split()
            if len(parts) < 2:
                continue
            if "(/" in package or ":\\" in package:
                own.add(parts[0])
            else:
                out[(parts[0], parts[1].lstrip("v"))] = licence.replace(" (*)", "").strip()
    GRAPHS[kind] = (out, own)
    return out, own


def graph(kind):
    """(name, version) of every crates.io package `kind` compiles in."""
    return set(graph_licences(kind)[0])


def clarified():
    """deny.toml's [[licenses.clarify]]: the expression of crates that
    declare none."""
    out, crate = {}, None
    for line in open(os.path.join(ROOT, "deny.toml")):
        m = re.match(r'\s*(crate|expression)\s*=\s*"([^"]*)"', line)
        if m and m.group(1) == "crate":
            crate = m.group(2)
        elif m and crate:
            out[crate] = m.group(2)
            crate = None
    return out


def permitted(expr):
    """Whether an SPDX expression can be complied with using PERMISSIVE
    licences only (an OR needs one alternative, an AND all of them)."""
    tokens = expr.replace("/", " OR ").replace("(", " ( ").replace(")", " ) ").split()
    pos = 0

    def alt():
        nonlocal pos
        ok = conj()
        while pos < len(tokens) and tokens[pos] == "OR":
            pos += 1
            ok = conj() or ok
        return ok

    def conj():
        nonlocal pos
        ok = atom()
        while pos < len(tokens) and tokens[pos] == "AND":
            pos += 1
            ok = atom() and ok
        return ok

    def atom():
        nonlocal pos
        if tokens[pos] == "(":
            pos += 1
            ok = alt()
            pos += 1
            return ok
        ident = tokens[pos]
        pos += 1
        if pos < len(tokens) and tokens[pos] == "WITH":
            ident += " WITH " + tokens[pos + 1]
            pos += 2
        return ident in PERMISSIVE

    return bool(tokens) and alt()


def check_licences(kind):
    """Every crate in `kind`'s graph is under a permissive licence, none is
    banned, and winit's KDE bindings are RapidR's stand-in."""
    licences, own = graph_licences(kind)
    clar = clarified()
    bad = sorted(f"{n} {v} ({l or 'none'})" for (n, v), l in licences.items() if not permitted(l or clar.get(n, "")))
    check(not bad, f"{kind}: every crate is under {', '.join(sorted(PERMISSIVE))}" + (f" (not: {', '.join(bad[:10])})" if bad else ""))
    banned = sorted(f"{n} {v}" for (n, v) in licences if n in BANNED)
    check(not banned, f"{kind}: none of the replaced crates" + (f" (found: {', '.join(banned)})" if banned else ""))
    plasma = [f"{n} {v}" for (n, v) in licences if n == "wayland-protocols-plasma"]
    check(not plasma, f"{kind}: no crates.io wayland-protocols-plasma (KDE's LGPL protocol files)" + (f" (found: {plasma[0]}: is the [patch.crates-io] in Cargo.toml?)" if plasma else ""))


def listed(text):
    """The components PART 1 of a notices file names: (name, version)."""
    part1 = text.split("PART 1.", 1)[1].split("PART 2.", 1)[0]
    out = set()
    for line in part1.splitlines()[2:]:
        if line and not line.startswith(" "):
            parts = line.split()
            out.add((parts[0], parts[-1]) if len(parts) == 2 else (line, ""))
    return out


def check_lists(text, kind, label):
    want = graph(kind)
    have = listed(text)
    missing = sorted(want - have)
    check(not missing, f"{label}: lists all {len(want)} crates of {kind}" + (f" (missing: {', '.join(f'{n} {v}' for n, v in missing[:10])})" if missing else ""))
    check("RapidR (its runtime and libraries)" in text and "Rust standard library" in text, f"{label}: credits RapidR and Rust's standard library")


def run(cmd, cwd, env):
    r = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stdout[-2000:], r.stderr[-2000:], sep="\n")
    return r.returncode == 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rapidr", default=os.path.join(ROOT, "target", "debug", "rapidr"))
    ap.add_argument("--native", action="store_true", help="also a native build (compiles the runtime)")
    ap.add_argument("--web", action="store_true", help="also a native web build (needs wasm-bindgen)")
    args = ap.parse_args()
    rapidr = os.path.abspath(args.rapidr)
    host = host_target()

    with tempfile.TemporaryDirectory(prefix="rapidr-notices-") as tmp:
        env = dict(os.environ, RAPIDR_PRINT_TO=os.path.join(tmp, "prints"), RAPIDR_REGISTRY=os.path.join(tmp, "registry.reg"), RAPIDR_HOME=ROOT)
        src = os.path.join(tmp, "hello.bas")
        with open(src, "w") as f:
            f.write('PRINT "hello"\n')

        print("== interpreted executable")
        out = os.path.join(tmp, "interp")
        if run([rapidr, "build", src, out, "--interp"], tmp, env):
            exe = os.path.join(out, "hello" + (".exe" if host.startswith("windows") else ""))
            check(os.path.isfile(exe), "the executable is built")
            path = os.path.join(out, NOTICES)
            check(os.path.isfile(path), f"{NOTICES} beside it")
            if os.path.isfile(path):
                check_lists(open(path, encoding="utf-8").read(), host, "interpreted")
        else:
            check(False, "rapidr build --interp")

        print("== web bundle")
        zpath = os.path.join(tmp, "hello-web.zip")
        if run([rapidr, "bundle-bc", src, "-o", zpath], tmp, env):
            with zipfile.ZipFile(zpath) as z:
                names = z.namelist()
                check(NOTICES in names, f"{NOTICES} in the bundle's root")
                html = z.read("index.html").decode()
                check(f'rel="license" href="{NOTICES}"' in html, "index.html links it")
                if NOTICES in names:
                    check_lists(z.read(NOTICES).decode(), "web", "web bundle")
        else:
            check(False, "rapidr bundle-bc (needs tools/build_web_artifacts.sh)")

        print("== bytecode")
        rrbc = os.path.join(tmp, "hello.rrbc")
        check(run([rapidr, "build-bc", src, "-o", rrbc], tmp, env) and os.path.isfile(rrbc), "build-bc makes the .rrbc (only the program: run by an installed RapidR Runtime, which carries its own notices)")

        if args.native:
            print("== native executable")
            nsrc = os.path.join(tmp, "native", "hello.bas")
            os.makedirs(os.path.dirname(nsrc))
            with open(nsrc, "w") as f:
                f.write('PRINT "hello"\n')
            if run([rapidr, "build", nsrc], tmp, env):
                path = os.path.join(tmp, "native", NOTICES)
                check(os.path.isfile(path), f"{NOTICES} beside the native executable")
                if os.path.isfile(path):
                    check_lists(open(path, encoding="utf-8").read(), host, "native")
            else:
                check(False, "rapidr build (native)")

        if args.web:
            print("== native web build")
            wsrc = os.path.join(tmp, "webn", "hello.bas")
            os.makedirs(os.path.dirname(wsrc))
            with open(wsrc, "w") as f:
                f.write('PRINT "hello"\n')
            if run([rapidr, "build", wsrc, "--web"], tmp, env):
                d = os.path.join(tmp, "webn", "hello_web")
                check(os.path.isfile(os.path.join(d, NOTICES)), f"{NOTICES} in the web build's folder")
                check(NOTICES in open(os.path.join(d, "index.html")).read(), "index.html links it")
            else:
                check(False, "rapidr build --web")

        print("== what every kind may contain")
        deny = open(os.path.join(ROOT, "deny.toml")).read().split("\nallow = [", 1)[1].split("]", 1)[0]
        check(set(re.findall(r'^\s*"([^"]+)"', deny, re.M)) == PERMISSIVE, "deny.toml's allowlist is the permissive list")
        for kind in KINDS:
            check_licences(kind)

        print("== every kind (rapidr notices <kind>)")
        for kind in KINDS:
            r = subprocess.run([rapidr, "notices", kind], cwd=ROOT, env=env, capture_output=True, text=True)
            if r.returncode != 0:
                check(False, f"rapidr notices {kind}: {r.stderr.strip()[:300]}")
                continue
            check_lists(r.stdout, kind, f"notices {kind}")

    print(f"\n{'FAILED: ' + str(len(failures)) if failures else 'notices: all ok'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
