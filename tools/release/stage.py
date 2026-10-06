#!/usr/bin/env python3
"""Lay out an installed RapidR (a "prefix") from built pieces. Every
platform's package is made from this tree (crates/rapidr-cli/src/home.rs
finds its home from the executable by it):

    bin/rapidr[.exe]                    the CLI and the RapidR Runtime
    bin/rapidrw[.exe]                   Windows' and macOS' desktop launcher
    lib/rapidr/release.toml             version, Rust, kind (sdk | runtime)
    lib/rapidr/ide/rapidr-ide.rrbc      the IDE (`rapidr ide`)          sdk
    lib/rapidr/examples/                the example programs, by topic  sdk
                                        (`rapidr examples`: what git tracks of examples/)
    lib/rapidr/runners/<os>-<arch>/     rapidrintr-runner[w][.exe]      sdk
    lib/rapidr/web/                     rapidrintr.js, _bg.wasm         sdk
    lib/rapidr/notices/<os>-<arch>.txt, web.txt   the THIRD-PARTY-NOTICES.txt
                                        builds carry (`rapidr notices`) sdk
    lib/rapidr/{Cargo.*,crates,vendor,…} the runtime's sources (home.py) sdk
    lib/rapidr/toolchain/               LLVM-MinGW, trimmed (Windows)   sdk
    lib/rapidr/web/fonts/               the web's Noto fallback fonts   sdk (index.json, *.otf, OFL.txt)
    share/icons/                        the apps' and file types' .ico  Windows (design/brand/icons)
    share/doc/rapidr/                   LICENSE, LEGAL.md, LICENSES.md, THIRD_PARTY_NOTICES.md,
                                        THIRD-PARTY-NOTICES.txt (rapidr's own), the fonts' OFL, README.md

    python3 tools/release/stage.py --kind sdk --os macos --out STAGE \\
        --bin target/release --home dist/<ver>/home-macos \\
        --runner macos=<folder with the universal runner> … \\
        --web target/web --ide dist/<ver>/rapidr-ide.rrbc
"""

import argparse
import os
import shutil
import stat
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DOCS = ["LICENSE", "LEGAL.md", "LICENSES.md", "THIRD_PARTY_NOTICES.md", "README.md", "crates/rapidr-value/fonts/OFL-1.1.txt"]


def copy_exe(src, dest):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    shutil.copy2(src, dest)
    os.chmod(dest, os.stat(dest).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def notices(kind, dest):
    """The THIRD-PARTY-NOTICES.txt for `kind` (an <os>-<arch>, web,
    tools-<os>), made from this checkout's dependency graph by the CLI's
    generator (crates/rapidr-cli/src/notices.rs, docs/licensing.md)."""
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    subprocess.run(["cargo", "run", "--quiet", "--release", "-p", "rapidr-cli", "--", "notices", kind, "-o", dest],
                   cwd=ROOT, env=dict(os.environ, RAPIDR_HOME=ROOT), check=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--kind", required=True, choices=["sdk", "runtime"])
    ap.add_argument("--os", required=True, choices=["macos", "linux", "windows"])
    ap.add_argument("--out", required=True)
    ap.add_argument("--bin", required=True, help="the folder with rapidr (and rapidrw)")
    ap.add_argument("--home", help="home.py's output (sdk)")
    ap.add_argument("--runner", action="append", default=[], help="<os>-<arch>=<folder with rapidrintr-runner[w]> (sdk)")
    ap.add_argument("--web", help="the folder with rapidrintr.js and rapidrintr_bg.wasm (sdk)")
    ap.add_argument("--ide", help="the IDE's bytecode (sdk)")
    ap.add_argument("--toolchain", help="Windows: the trimmed LLVM-MinGW native builds link with (sdk)")
    ap.add_argument("--version", help="for a runtime's release.toml (default: home's)")
    ap.add_argument("--rust", default="", help="for a runtime's release.toml")
    args = ap.parse_args()

    exe = ".exe" if args.os == "windows" else ""
    out = os.path.abspath(args.out)
    if os.path.exists(out):
        shutil.rmtree(out)
    lib = os.path.join(out, "lib", "rapidr")

    copy_exe(os.path.join(args.bin, f"rapidr{exe}"), os.path.join(out, "bin", f"rapidr{exe}"))
    if args.os in ("windows", "macos"):
        copy_exe(os.path.join(args.bin, f"rapidrw{exe}"), os.path.join(out, "bin", f"rapidrw{exe}"))

    if args.kind == "sdk":
        if not (args.home and args.runner and args.web and args.ide):
            sys.exit("an sdk needs --home, --runner, --web and --ide")
        shutil.copytree(args.home, lib, symlinks=False)
        for spec in args.runner:
            target, folder = spec.split("=", 1)
            names = ["rapidrintr-runner"] + (["rapidrintr-runnerw"] if target.startswith("windows-") else [])
            for name in names:
                copy_exe(os.path.join(folder, f"{name}{exe}"), os.path.join(lib, "runners", target, f"{name}{exe}"))
        os.makedirs(os.path.join(lib, "web"))
        for f in ["rapidrintr.js", "rapidrintr_bg.wasm"]:
            shutil.copy2(os.path.join(args.web, f), os.path.join(lib, "web", f))
        # (the web's fallback fonts, beside the interpreter: `rapidr build --web`
        # and `bundle-bc` copy them, never download — when the web build makes them)
        if os.path.isdir(os.path.join(args.web, "fonts")):
            shutil.copytree(os.path.join(args.web, "fonts"), os.path.join(lib, "web", "fonts"))
        elif os.path.exists(os.path.join(ROOT, "tools", "fonts.py")):
            sys.exit(f"no fonts/ beside the web interpreter in {args.web}: prepare.sh makes them")
        if args.toolchain:
            shutil.copytree(args.toolchain, os.path.join(lib, "toolchain"))
        os.makedirs(os.path.join(lib, "ide"))
        shutil.copy2(args.ide, os.path.join(lib, "ide", "rapidr-ide.rrbc"))
        # (a checkout's tracked files — no builds, nothing a run left; a
        # release archive's examples/ is only those)
        tracked = subprocess.run(["git", "ls-files", "examples"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines() \
            if os.path.exists(os.path.join(ROOT, ".git")) else \
            [os.path.relpath(os.path.join(d, f), ROOT) for d, _, fs in os.walk(os.path.join(ROOT, "examples")) for f in fs]
        for f in tracked:
            os.makedirs(os.path.dirname(os.path.join(lib, f)), exist_ok=True)
            shutil.copy2(os.path.join(ROOT, f), os.path.join(lib, f))
    else:
        os.makedirs(lib)
        version = args.version
        if not version:
            sys.exit("a runtime needs --version")
        with open(os.path.join(lib, "release.toml"), "w") as f:
            f.write("# An installed RapidR's home (crates/rapidr-cli/src/home.rs).\n")
            f.write(f'version = "{version}"\nrust = "{args.rust}"\nkind = "runtime"\n')

    if args.os == "windows":
        # (the file types', the Start menu's and the uninstaller's icons)
        icons = os.path.join(out, "share", "icons")
        os.makedirs(icons)
        for f in ["rapidr-ide.ico", "rapidr-runtime.ico", "rapidr-source.ico", "basic-source.ico", "rapidr-program.ico"]:
            shutil.copy2(os.path.join(ROOT, "design", "brand", "icons", "windows", f), os.path.join(icons, f))
    doc = os.path.join(out, "share", "doc", "rapidr")
    os.makedirs(doc)
    for f in DOCS:
        shutil.copy2(os.path.join(ROOT, f), os.path.join(doc, os.path.basename(f)))
    # The notices: rapidr's own, and (SDK) those every build carries
    notices(f"tools-{args.os}", os.path.join(doc, "THIRD-PARTY-NOTICES.txt"))
    if args.kind == "sdk":
        for spec in args.runner:
            target = spec.split("=", 1)[0]
            # (macOS' universal runner also makes either slice: --target macos-arm64 / macos-x86_64)
            for t in [target, "macos-arm64", "macos-x86_64"] if target == "macos" else [target]:
                notices(t, os.path.join(lib, "notices", f"{t}.txt"))
        notices("web", os.path.join(lib, "notices", "web.txt"))
    print(f"staged {args.kind} for {args.os} in {out}")


if __name__ == "__main__":
    main()
