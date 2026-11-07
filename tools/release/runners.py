#!/usr/bin/env python3
"""The runner files a release publishes, and the list that covers them.

Building an app for another system needs that system's runner, and each
install ships only its own. A Studio fetches the others from the release's
host (docs/build-for-every-platform-plan.md, section 3): one `<target>.zst`
per target and a `runners-manifest.json` that lists them with their size and
SHA-256. Robert signs the manifest with minisign (finish.sh, with
RAPIDR_MINISIGN_KEY); the Studio checks that signature, the version and each
file's SHA-256 before it uses a runner (crates/rapidr-runners).

A `<target>.zst` is a zstd-compressed, deterministic ustar tar of:

    windows-*   rapidrintr-runner.exe                           THIRD-PARTY-NOTICES.txt
    macos       rapidrintr-runner (universal: arm64 + x86_64)   THIRD-PARTY-NOTICES.txt
    linux-*     rapidrintr-runner                               THIRD-PARTY-NOTICES.txt
    web         rapidrintr.js  rapidrintr_bg.wasm  fonts/*      THIRD-PARTY-NOTICES.txt

(the targets are an install's `runners/<target>/` names: windows-x86_64,
windows-aarch64, windows-x86, macos, linux-x86_64, linux-aarch64, web).
Windows has ONE runner: an install's console `rapidrintr-runner.exe` and windowed
`rapidrintr-runnerw.exe` differ in four bytes (the PE header's time stamp, checksum and
subsystem) and the packer sets the subsystem itself, so only the console one goes in
(the same download size as both would take with a long zstd window, half the unpacked size).

    runners.py tar --target T --from DIR --notices FILE --out FILE.tar
        the tar alone (stdlib only: runs in the Windows VM, which has no zstd)
    runners.py bundle --target T (--from DIR --notices FILE | --tar FILE.tar) --out DIR
        DIR/T.zst
    runners.py prefix PREFIX --out DIR [--only T…]
        every target an installed RapidR's lib/rapidr (PREFIX) has: runners/<T>/ with
        notices/<T>.txt, and the web interpreter (web/) with notices/web.txt
    runners.py manifest --dir DIR --out FILE [--version V]
        FILE (runners-manifest.json) listing every DIR/*.zst
    runners.py check --dir DIR --manifest FILE [--version V] [--key PUBLIC_KEY]
        the manifest against the files (size, SHA-256, contents, version) and,
        if FILE.minisig is there and minisign is installed, its signature

Needs python3 and the `zstd` program (brew install zstd). Never signs anything,
never uploads.
"""

import argparse
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# (the order a dialog lists them; crates/rapidr-runners/src/names.rs has the same list)
TARGETS = ["windows-x86_64", "windows-aarch64", "windows-x86", "macos", "linux-x86_64", "linux-aarch64", "web"]
NOTICES = "THIRD-PARTY-NOTICES.txt"
FORMAT = 1
# crates/rapidr-runners/src/manifest.rs: what a list may promise
MAX_FILE_SIZE = 300 << 20
MAX_UNPACKED = 600 << 20
RELEASE_KEY = "RWT4oRaArdWhjGXn5bIqoxGsqPvaLFJxHRpQqZXKUFDFuDZWgMPC3srG"
# (what a mac or a copy through a share leaves beside real files: never part of a runner)
JUNK = re.compile(r"(^|/)(\._[^/]*|\.DS_Store|Thumbs\.db|desktop\.ini)$")


def die(message):
    sys.exit(f"runners.py: {message}")


def version():
    with open(os.path.join(ROOT, "Cargo.toml")) as f:
        m = re.search(r'^version = "([^"]+)"', f.read(), re.M)
    if not m:
        die("no version in Cargo.toml")
    return m.group(1)


def required(target):
    if target == "web":
        files = ["rapidrintr.js", "rapidrintr_bg.wasm"]
    elif target.startswith("windows-"):
        files = ["rapidrintr-runner.exe"]
    else:
        files = ["rapidrintr-runner"]
    return files + [NOTICES]


def is_program(name):
    return name.startswith("rapidrintr-runner")


def check_target(target):
    if target not in TARGETS:
        die(f"{target!r} is not a target ({', '.join(TARGETS)})")


# ------------------------------------------------------------------ the tar

def collect(folder):
    """{name: path} of every regular file below `folder` (symlinks refused, junk left out)."""
    files = {}
    for dirpath, dirnames, filenames in os.walk(folder):
        dirnames.sort()
        for name in sorted(filenames):
            path = os.path.join(dirpath, name)
            rel = os.path.relpath(path, folder).replace(os.sep, "/")
            if JUNK.search(rel):
                continue
            if os.path.islink(path):
                die(f"{path} is a link: a runner bundle has none")
            files[rel] = path
    return files


def make_tar(target, files):
    """The bundle's tar, the same bytes for the same files: names sorted, no owner, no time."""
    missing = [n for n in required(target) if n not in files]
    if missing:
        die(f"{target}: lacks {', '.join(missing)}")
    out = io.BytesIO()
    with tarfile.open(fileobj=out, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for name in sorted(files):
            data = files[name]
            if isinstance(data, str):
                with open(data, "rb") as f:
                    data = f.read()
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mtime = 0
            info.mode = 0o755 if is_program(name) else 0o644
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            tar.addfile(info, io.BytesIO(data))
    return out.getvalue()


def files_for(target, folder, notices):
    files = collect(folder)
    if target != "web":
        # (a runner folder holds the program and nothing else goes in: Windows' windowed twin, which
        # only differs in the subsystem the packer sets, stays out)
        files = {n: p for n, p in files.items() if n in required(target)}
    if notices:
        if not os.path.isfile(notices):
            die(f"{target}: no notices file at {notices}")
        files[NOTICES] = notices
    return files


# --------------------------------------------------------------- zstd

def need_zstd():
    if not shutil.which("zstd"):
        die("the zstd program is needed (brew install zstd)")


def zstd_compress(data, out_path):
    need_zstd()
    # Level 19, one frame, zstd's default window (8 MB): what the readers take with no extra memory
    # (`rapidr runners check dist/<ver>/out` reads every file back with the real reader).
    r = subprocess.run(["zstd", "-19", "-q", "-f", "-o", out_path], input=data, capture_output=True)
    if r.returncode != 0:
        die(f"zstd failed: {r.stderr.decode(errors='replace')}")


def zstd_decompress(path):
    need_zstd()
    r = subprocess.run(["zstd", "-dc", path], capture_output=True)
    if r.returncode != 0:
        die(f"{path}: not zstd ({r.stderr.decode(errors='replace')})")
    return r.stdout


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def megabytes(n):
    return f"{n / 1048576:.1f} MB"


# ------------------------------------------------------------- commands

def cmd_tar(a):
    check_target(a.target)
    data = make_tar(a.target, files_for(a.target, a.from_dir, a.notices))
    with open(a.out, "wb") as f:
        f.write(data)
    print(f"{a.target}: {a.out} ({megabytes(len(data))})")


def write_bundle(target, tar_bytes, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, f"{target}.zst")
    zstd_compress(tar_bytes, path)
    print(f"{target}: {path} ({megabytes(len(tar_bytes))} -> {megabytes(os.path.getsize(path))})")
    return path


def cmd_bundle(a):
    check_target(a.target)
    if a.tar:
        with open(a.tar, "rb") as f:
            data = f.read()
        # (it must be the bundle it says it is)
        names = [m.name for m in tarfile.open(fileobj=io.BytesIO(data)).getmembers() if m.isfile()]
        missing = [n for n in required(a.target) if n not in names]
        if missing:
            die(f"{a.tar}: lacks {', '.join(missing)}")
    elif a.from_dir:
        data = make_tar(a.target, files_for(a.target, a.from_dir, a.notices))
    else:
        die("bundle needs --from DIR (and --notices FILE) or --tar FILE")
    write_bundle(a.target, data, a.out)


def cmd_prefix(a):
    lib = a.prefix
    notices = os.path.join(lib, "notices")
    wanted = a.only or TARGETS
    made = 0
    for target in TARGETS:
        if target not in wanted:
            continue
        folder = os.path.join(lib, "web" if target == "web" else os.path.join("runners", target))
        if not os.path.isdir(folder):
            if a.only:
                die(f"{lib} has no {os.path.relpath(folder, lib)}")
            continue
        n = os.path.join(notices, f"{target}.txt")
        write_bundle(target, make_tar(target, files_for(target, folder, n)), a.out)
        made += 1
    if made == 0:
        die(f"{lib} has no runners")


def inspect(path):
    """(files in the bundle, unpacked bytes) of a `<target>.zst`."""
    tar = tarfile.open(fileobj=io.BytesIO(zstd_decompress(path)))
    members = [m for m in tar.getmembers() if m.isfile()]
    others = [m.name for m in tar.getmembers() if not m.isfile() and not m.isdir()]
    if others:
        die(f"{path}: has links or devices: {', '.join(others)}")
    return sorted(m.name for m in members), sum(m.size for m in members)


def cmd_manifest(a):
    v = a.version or version()
    runners = []
    for target in TARGETS:
        path = os.path.join(a.dir, f"{target}.zst")
        if not os.path.isfile(path):
            continue
        files, unpacked = inspect(path)
        missing = [n for n in required(target) if n not in files]
        if missing:
            die(f"{path}: lacks {', '.join(missing)}")
        size = os.path.getsize(path)
        if size > MAX_FILE_SIZE or unpacked > MAX_UNPACKED:
            die(f"{path}: bigger than a runner may be ({megabytes(size)} packed, {megabytes(unpacked)} unpacked)")
        runners.append({"target": target, "file": f"{target}.zst", "size": size, "sha256": sha256(path), "unpacked": unpacked, "files": files})
    if not runners:
        die(f"no <target>.zst in {a.dir}")
    text = json.dumps({"format": FORMAT, "version": v, "runners": runners}, indent=2) + "\n"
    with open(a.out, "w") as f:
        f.write(text)
    for r in runners:
        print(f"{r['target']:<16} {megabytes(r['size']):>10}  sha256 {r['sha256'][:16]}…")
    print(f"{a.out}: {len(runners)} runner(s) for RapidR {v}")


def cmd_check(a):
    v = a.version or version()
    with open(a.manifest, "rb") as f:
        manifest = json.loads(f.read())
    problems = []
    if manifest.get("format") != FORMAT:
        problems.append(f"format {manifest.get('format')}")
    if manifest.get("version") != v:
        problems.append(f"the list is for RapidR {manifest.get('version')}, not {v}")
    listed = []
    for r in manifest.get("runners", []):
        listed.append(r["target"])
        path = os.path.join(a.dir, r["file"])
        if not os.path.isfile(path):
            problems.append(f"{r['file']}: missing")
            continue
        if os.path.getsize(path) != r["size"]:
            problems.append(f"{r['file']}: size {os.path.getsize(path)}, the list says {r['size']}")
        if sha256(path) != r["sha256"]:
            problems.append(f"{r['file']}: SHA-256 differs from the list's")
        files, unpacked = inspect(path)
        if files != sorted(r["files"]) or unpacked != r["unpacked"]:
            problems.append(f"{r['file']}: its contents are not what the list says")
    for target in TARGETS:
        if target not in listed:
            problems.append(f"{target}: not in the list")
    sig = a.manifest + ".minisig"
    if os.path.isfile(sig):
        if shutil.which("minisign"):
            r = subprocess.run(["minisign", "-V", "-m", a.manifest, "-x", sig, "-P", a.key], capture_output=True, text=True)
            print(("signature: ok" if r.returncode == 0 else "signature: FAILED") + (f" ({r.stderr.strip() or r.stdout.strip()})" if r.returncode else ""))
            if r.returncode:
                problems.append("the signature does not verify")
        else:
            print("signature: not checked (minisign is not installed)")
    else:
        print("signature: none yet (finish.sh signs it with RAPIDR_MINISIGN_KEY)")
    for p in problems:
        print(f"  PROBLEM: {p}")
    print("ok" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("tar")
    p.add_argument("--target", required=True)
    p.add_argument("--from", dest="from_dir", required=True)
    p.add_argument("--notices")
    p.add_argument("--out", required=True)
    p.set_defaults(fn=cmd_tar)

    p = sub.add_parser("bundle")
    p.add_argument("--target", required=True)
    p.add_argument("--from", dest="from_dir")
    p.add_argument("--notices")
    p.add_argument("--tar")
    p.add_argument("--out", required=True)
    p.set_defaults(fn=cmd_bundle)

    p = sub.add_parser("prefix")
    p.add_argument("prefix")
    p.add_argument("--out", required=True)
    p.add_argument("--only", nargs="+")
    p.set_defaults(fn=cmd_prefix)

    p = sub.add_parser("manifest")
    p.add_argument("--dir", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--version")
    p.set_defaults(fn=cmd_manifest)

    p = sub.add_parser("check")
    p.add_argument("--dir", required=True)
    p.add_argument("--manifest", required=True)
    p.add_argument("--version")
    p.add_argument("--key", default=RELEASE_KEY)
    p.set_defaults(fn=cmd_check)

    a = ap.parse_args()
    sys.exit(a.fn(a) or 0)


if __name__ == "__main__":
    main()
