#!/usr/bin/env python3
"""RArchive's libraries keep only the features the legal review cleared
(docs/legal/review-2026-10-29-rarchive.md, item 4): no AES (the zip
APPNOTE's strong encryption is patented, and `aes`, `cbc`, `hkdf` are
banned), no Deflate64 (a PKWARE mark), no code translated from another
project's sources (ruzstd's dictionary builder from facebook/zstd, PPMd), no
C libraries (bzip2, zstd, brotli, lz4 bindings). Turning one on needs a new
review first.

It reads the features cargo really resolves (`cargo metadata`), for the
desktop and for the web (wasm32-unknown-unknown), with the workspace's
default features and with all of them, and fails if a forbidden one is on
anywhere.

    python3 tools/check_archive_features.py       (repo root; tools/regress.sh legal)
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# package → features that must stay off (their aliases that turn them on
# too: a crate's `default`, sevenz-rust2's `default_wasm` …).
FORBIDDEN = {
    "zip": {"aes-crypto", "default", "deflate64", "bzip2", "bzip2-rs", "zstd", "ppmd", "deflate-zopfli"},
    "sevenz-rust2": {"aes256", "aes256_wasm", "default", "default_wasm", "ppmd", "bzip2", "zstd", "brotli", "lz4"},
    "ruzstd": {"dict_builder"},
}

# (the crates those features would bring: none may be in the graph)
FORBIDDEN_CRATES = {"aes", "cbc", "hkdf", "ppmd-rust", "zstd-sys", "bzip2-sys", "libbz2-rs-sys", "brotli", "lz4_flex", "deflate64", "zopfli"}


def metadata(platform, all_features):
    cmd = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", platform]
    if all_features:
        cmd.append("--all-features")
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        # (offline with a crate not downloaded yet: once more, online)
        cmd.remove("--offline")
        out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"check_archive_features: cargo metadata failed:\n{out.stderr[-2000:]}")
    return json.loads(out.stdout)


def host():
    out = subprocess.run(["rustc", "-vV"], capture_output=True, text=True, check=True).stdout
    return next(l.split(":", 1)[1].strip() for l in out.splitlines() if l.startswith("host:"))


def main():
    problems = []
    seen = set()
    for platform in [host(), "wasm32-unknown-unknown"]:
        for all_features in (False, True):
            m = metadata(platform, all_features)
            names = {p["id"]: (p["name"], p["version"]) for p in m["packages"]}
            where = f"{platform}{' --all-features' if all_features else ''}"
            for node in m["resolve"]["nodes"]:
                name, version = names[node["id"]]
                bad = FORBIDDEN.get(name, set()) & set(node.get("features", []))
                for f in sorted(bad):
                    problems.append(f"{name} {version}: feature `{f}` is on ({where})")
                if name in FORBIDDEN_CRATES:
                    problems.append(f"{name} {version} is in the graph ({where})")
                if name in FORBIDDEN:
                    seen.add(name)
    missing = set(FORBIDDEN) - seen
    if missing:
        # (a renamed or dropped crate: the guard would check nothing)
        problems.append(f"not in the graph any more, update this check: {', '.join(sorted(missing))}")
    if problems:
        print("check_archive_features: FAILED (each needs a new legal review first: docs/legal/review-2026-10-29-rarchive.md)")
        for p in sorted(set(problems)):
            print("  " + p)
        sys.exit(1)
    print(f"archive features: all ok ({', '.join(sorted(FORBIDDEN))}: no AES, Deflate64, PPMd, dictionary builder or C codecs; desktop and web)")


if __name__ == "__main__":
    main()
