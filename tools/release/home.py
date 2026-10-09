#!/usr/bin/env python3
"""Assemble the source part of an installed RapidR's home (lib/rapidr/) for
one operating system: what native builds compile against, offline.

    python3 tools/release/home.py --os macos --src <the release's source> --out <dir>

The source is the release's commit as an archive makes it (prepare.sh's
src.tar, extracted): what ships is what was committed, nothing else a
checkout holds. It needs cargo and the network (or cargo's cache). It writes
(crates/rapidr-cli/src/home.rs reads this layout):

    Cargo.toml          a workspace of the runtime crates only
    Cargo.lock          the repository's, pruned to them (the versions RapidR
                        is tested with)
    crates/…            rapidr-runtime-core, rapidr-runtime-web and the RapidR
                        crates they use (their [dev-dependencies] removed: the
                        home is compiled against, not tested, and a dev-dependency
                        may name a crate the home doesn't have); crates/patches/… the crates.io crates
                        RapidR replaces (the [patch.crates-io] above)
    .cargo/config.toml  the web runtime's SQLite flags (rapidr build --web)
    design/…            the files the crates include from outside their own
                        folders (the program icon's masters, the icon inventory)
    tools/wasm-ar.sh
    vendor/             `cargo vendor` of their crates.io dependencies; a crate
                        no build on this OS (nor the web) compiles keeps only
                        its Cargo.toml, which resolution still reads
    release.toml        version, the Rust it was tested with, kind = "sdk"

The binaries (bin/), the runners (runners/), the web interpreter (web/) and
the IDE (ide/) are added by the platform scripts.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib

# The targets native builds on each OS compile for (host tools included):
# the OS' own, and the web (`rapidr build --web`).
TARGETS = {
    "macos": ["aarch64-apple-darwin", "x86_64-apple-darwin"],
    "linux": ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"],
    "windows": ["x86_64-pc-windows-gnullvm", "aarch64-pc-windows-gnullvm", "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
}
WEB = "wasm32-unknown-unknown"
RUNTIME_CRATES = ["rapidr-runtime-core", "rapidr-runtime-web"]


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, **kw)


def metadata(manifest, *extra):
    out = subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--manifest-path", manifest, *extra])
    return json.loads(out)


def runtime_crates(src):
    """The RapidR crates the runtimes use, as paths relative to the source."""
    meta = metadata(os.path.join(src, "Cargo.toml"), "--locked")
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    start = [p["id"] for p in meta["packages"] if p["name"] in RUNTIME_CRATES and p["source"] is None]
    seen, stack = set(), list(start)
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            if packages[dep["pkg"]]["source"] is None and any(k.get("kind") in (None, "build") for k in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    return sorted(os.path.relpath(os.path.dirname(packages[p]["manifest_path"]), src) for p in seen)


INCLUDE = re.compile(r'\binclude(?:_str|_bytes)?!\(\s*(?:concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*)?"([^"]+)"')


def outside_includes(src, crates):
    """The files the shipped crates' sources include from outside their own
    folders (include_str!("../../../design/…")), as paths relative to the
    source: the home keeps them where the crates expect them. Files inside
    another crate's folder are left (a test reading a neighbour's source)."""
    found = set()
    src = os.path.realpath(src)
    for c in crates:
        root = os.path.realpath(os.path.join(src, c))
        for base, _, files in os.walk(root):
            for name in files:
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(base, name)
                with open(path, errors="ignore") as f:
                    text = f.read()
                for m in INCLUDE.finditer(text):
                    rel = m.group(1)
                    full = os.path.normpath(root + rel if rel.startswith("/") else os.path.join(base, rel))
                    if full.startswith(root + os.sep) or not full.startswith(src + os.sep) or not os.path.isfile(full):
                        continue
                    d = os.path.dirname(full)
                    in_crate = False
                    while d != src and len(d) > len(src):
                        if os.path.exists(os.path.join(d, "Cargo.toml")):
                            in_crate = True
                            break
                        d = os.path.dirname(d)
                    if not in_crate:
                        found.add(os.path.relpath(full, src))
    return sorted(found)


DEV_SECTION = re.compile(r"^\s*\[(?:target\.[^\]]+\.)?dev-dependencies(?:\.[^\]]+)?\]\s*$")
ANY_SECTION = re.compile(r"^\s*\[")


def strip_dev_dependencies(manifest):
    """A shipped crate's Cargo.toml without its [dev-dependencies] sections
    (target-specific and per-crate tables too): the home is compiled
    against, never tested, and a dev-dependency may name a crate (or a
    workspace dependency) the home's workspace doesn't have — Cargo reads
    every member's manifest whole, and fails on it."""
    with open(manifest) as f:
        lines = f.read().splitlines(keepends=True)
    kept, skipping = [], False
    for line in lines:
        if ANY_SECTION.match(line):
            skipping = bool(DEV_SECTION.match(line))
        if not skipping:
            kept.append(line)
    text = "".join(kept)
    if tomllib.loads(text).get("dev-dependencies") or any("dev-dependencies" in v for v in tomllib.loads(text).get("target", {}).values()):
        sys.exit(f"{manifest}: dev-dependencies left after stripping (written as an inline table?)")
    with open(manifest, "w") as f:
        f.write(text)


def toml_value(v):
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {toml_value(x)}" for k, x in v.items()) + " }"
    raise TypeError(v)


def write_workspace(src, out, crates):
    with open(os.path.join(src, "Cargo.toml"), "rb") as f:
        root = tomllib.load(f)
    ws = root["workspace"]
    names = {}
    for c in crates:
        with open(os.path.join(src, c, "Cargo.toml"), "rb") as f:
            names[tomllib.load(f)["package"]["name"]] = c
    lines = [
        "# RapidR's runtime crates, as an installed RapidR ships them: native",
        "# builds compile against these (tools/release/home.py made this file).",
        "[workspace]",
        'resolver = "2"',
        "members = [" + ", ".join(json.dumps(c) for c in crates) + "]",
        "",
        "[workspace.package]",
    ]
    lines += [f"{k} = {toml_value(v)}" for k, v in ws["package"].items()]
    lines += ["", "[workspace.dependencies]"]
    for name, spec in ws.get("dependencies", {}).items():
        if name in names or not (isinstance(spec, dict) and "path" in spec):
            lines.append(f"{name} = {toml_value(spec)}")
    # (the crates.io crates RapidR replaces: what native builds compile too)
    lines += ["", "[patch.crates-io]"]
    lines += [f"{name} = {toml_value(spec)}" for name, spec in patches(src).items()]
    with open(os.path.join(out, "Cargo.toml"), "w") as f:
        f.write("\n".join(lines) + "\n")


def patches(src):
    """The workspace's [patch.crates-io]: {crate: {path = …}} — RapidR's
    own replacements (crates/patches/), shipped with the runtime's crates."""
    with open(os.path.join(src, "Cargo.toml"), "rb") as f:
        return tomllib.load(f).get("patch", {}).get("crates-io", {})


def needed_packages(out, targets):
    """(name, version) of every package some target's build resolves to."""
    needed = set()
    for t in targets:
        meta = metadata(os.path.join(out, "Cargo.toml"), "--filter-platform", t, "--offline")
        packages = {p["id"]: p for p in meta["packages"]}
        for node in meta["resolve"]["nodes"]:
            p = packages[node["id"]]
            needed.add((p["name"], p["version"]))
    return needed


def stub_targets(manifest):
    """The files a crate's manifest names as its targets (a library at
    src/lib.rs when it names none)."""
    paths = {manifest.get("lib", {}).get("path", "src/lib.rs")}
    for kind in ("bin", "example", "test", "bench"):
        paths.update(t["path"] for t in manifest.get(kind, []) if "path" in t)
    package = manifest.get("package", {})
    build = package.get("build")
    if isinstance(build, str):
        paths.add(build)
    elif build is None and "links" in package:
        # (a `links` crate must have a build script: found as build.rs)
        paths.add("build.rs")
    return sorted(paths)


def stub_unneeded(vendor, needed):
    """A crate no build here compiles: its Cargo.toml only (resolution reads
    it), and the checksum file saying so. Returns the bytes saved."""
    saved = 0
    for d in sorted(os.listdir(vendor)):
        path = os.path.join(vendor, d)
        with open(os.path.join(path, "Cargo.toml"), "rb") as f:
            pkg = tomllib.load(f)["package"]
        if (pkg["name"], pkg["version"]) in needed:
            continue
        with open(os.path.join(path, ".cargo-checksum.json")) as f:
            checksum = json.load(f)
        for base, dirs, files in os.walk(path):
            for name in files:
                saved += os.path.getsize(os.path.join(base, name))
        manifest = open(os.path.join(path, "Cargo.toml"), "rb").read()
        # (its licence files stay with what is left of it: docs/licensing.md)
        licences = {n: open(os.path.join(path, n), "rb").read() for n in os.listdir(path)
                    if os.path.isfile(os.path.join(path, n)) and n.lower().startswith(("licen", "copying", "copyright", "notice", "unlicense"))}
        shutil.rmtree(path)
        os.makedirs(path)
        open(os.path.join(path, "Cargo.toml"), "wb").write(manifest)
        files = {"Cargo.toml": hashlib.sha256(manifest).hexdigest()}
        for name, data in licences.items():
            open(os.path.join(path, name), "wb").write(data)
            files[name] = hashlib.sha256(data).hexdigest()
            saved -= len(data)
        # (its targets as empty files: a manifest with none doesn't load)
        for target in stub_targets(tomllib.loads(manifest.decode())):
            dest = os.path.join(path, target)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            open(dest, "wb").close()
            files[target] = hashlib.sha256(b"").hexdigest()
        with open(os.path.join(path, ".cargo-checksum.json"), "w") as f:
            json.dump({"files": files, "package": checksum.get("package")}, f)
    return saved


# What a compiled-against crate doesn't need: its tests, benchmarks, examples,
# fuzz targets and test data, and any prebuilt binary (.dll/.exe/.so/.dylib/…)
# inside it. Dropped from the vendored sources the packages ship, unless a
# source file of the crate includes it (include_str! and friends).
DROP_DIRS = {"tests", "test-data", "testdata", "test_data", "benches", "fuzz"}
# (examples/ stays: a crate's docs may include_str! them, as winnow's do)
INCLUDE_ANY = re.compile(r"\binclude(?:_str|_bytes)?!")
INCLUDE_RE = re.compile(r'include(?:_str|_bytes)?!\s*\(\s*"([^"]+)"')
DROP_SUFFIXES = (".dll", ".exe", ".so", ".dylib", ".lib", ".a", ".o", ".obj")


def trim_needed(vendor, needed):
    """Of the crates a build compiles, remove their tests, benches, fuzz
    targets, test data and prebuilt binaries (a file the manifest names as a target
    stays, empty: a manifest naming a missing file doesn't load). The checksum
    file is rewritten to match. Returns the bytes saved."""
    saved = 0
    for d in sorted(os.listdir(vendor)):
        path = os.path.join(vendor, d)
        with open(os.path.join(path, "Cargo.toml"), "rb") as f:
            manifest = tomllib.load(f)
        pkg = manifest["package"]
        if (pkg["name"], pkg["version"]) not in needed:
            continue
        keep = {os.path.normpath(t) for t in stub_targets(manifest)}
        # (files the crate's sources include stay, whatever their folder)
        included = set()
        kept_dirs = set()
        for base, dirs, names in os.walk(path):
            for name in names:
                if name.endswith(".rs"):
                    text = open(os.path.join(base, name), encoding="utf-8", errors="replace").read()
                    if INCLUDE_ANY.search(text):
                        # (a folder an including source names stays whole: zerocopy's docs
                        # build the file names with concat!)
                        kept_dirs.update(d for d in DROP_DIRS if re.search(r'["/]' + d + r'/', text))
                    for inc in INCLUDE_RE.findall(text):
                        included.add(os.path.normpath(os.path.relpath(os.path.join(base, inc), path)))
        with open(os.path.join(path, ".cargo-checksum.json")) as f:
            checksum = json.load(f)
        files = checksum["files"]
        for rel in sorted(files):
            parts = os.path.normpath(rel).split(os.sep)
            if not (any(p in DROP_DIRS for p in parts[:-1]) or rel.lower().endswith(DROP_SUFFIXES)):
                continue
            full = os.path.join(path, rel)
            if not os.path.isfile(full) or os.path.normpath(rel) in included or any(p in kept_dirs for p in parts[:-1]):
                continue
            saved += os.path.getsize(full)
            if os.path.normpath(rel) in keep:
                open(full, "wb").close()
                files[rel] = hashlib.sha256(b"").hexdigest()
            else:
                os.remove(full)
                del files[rel]
        for base, dirs, names in os.walk(path, topdown=False):
            if base != path and not os.listdir(base):
                os.rmdir(base)
        with open(os.path.join(path, ".cargo-checksum.json"), "w") as f:
            json.dump(checksum, f)
    return saved


def rust_version():
    out = subprocess.check_output(["rustc", "--version"]).decode()
    return out.split()[1]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--os", required=True, choices=sorted(TARGETS))
    ap.add_argument("--src", required=True, help="the release's source (its archive, extracted)")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    src = os.path.abspath(args.src)
    if os.path.exists(os.path.join(src, ".git")):
        sys.exit(f"{src} is a checkout: give the release's source archive, extracted (prepare.sh's src.tar)")
    out = os.path.abspath(args.out)
    if os.path.exists(out):
        shutil.rmtree(out)
    os.makedirs(out)

    crates = runtime_crates(src)
    for c in crates + [spec["path"] for spec in patches(src).values()]:
        shutil.copytree(os.path.join(src, c), os.path.join(out, c), ignore=shutil.ignore_patterns("target"))
        if c in crates:
            strip_dev_dependencies(os.path.join(out, c, "Cargo.toml"))
    # (the files they include from outside: the icon masters, the manual's icon inventory)
    for f in outside_includes(src, crates):
        os.makedirs(os.path.dirname(os.path.join(out, f)), exist_ok=True)
        shutil.copy2(os.path.join(src, f), os.path.join(out, f))
    write_workspace(src, out, crates)
    shutil.copy2(os.path.join(src, "Cargo.lock"), os.path.join(out, "Cargo.lock"))
    for f in [".cargo/config.toml", "tools/wasm-ar.sh"]:
        os.makedirs(os.path.dirname(os.path.join(out, f)), exist_ok=True)
        shutil.copy2(os.path.join(src, f), os.path.join(out, f))

    # The crates.io sources (the lockfile is pruned to these crates, its
    # versions kept), then those no build on this OS compiles left as stubs.
    print(f"vendoring for {args.os} …", file=sys.stderr)
    run(["cargo", "vendor", "--quiet", "--versioned-dirs", "--manifest-path", os.path.join(out, "Cargo.toml"), os.path.join(out, "vendor")], stdout=subprocess.DEVNULL)
    needed = needed_packages(out, TARGETS[args.os] + [WEB])
    saved = stub_unneeded(os.path.join(out, "vendor"), needed)
    print(f"vendor: {len(needed)} crates for {args.os} + web; {saved / 1e6:.0f} MB of others' sources left out", file=sys.stderr)
    trimmed = trim_needed(os.path.join(out, "vendor"), needed)
    print(f"vendor: {trimmed / 1e6:.0f} MB of the needed crates' tests, benches, test data and prebuilt binaries left out", file=sys.stderr)

    version = tomllib.load(open(os.path.join(src, "Cargo.toml"), "rb"))["workspace"]["package"]["version"]
    with open(os.path.join(out, "release.toml"), "w") as f:
        f.write("# An installed RapidR's home (crates/rapidr-cli/src/home.rs).\n")
        f.write(f'version = "{version}"\nrust = "{rust_version()}"\nkind = "sdk"\n')


if __name__ == "__main__":
    main()
