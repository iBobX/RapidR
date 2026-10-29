#!/usr/bin/env python3
"""RArchive's test archives, made by other tools (zip, 7zz, bsdtar,
gzip, xz, zstd) and by Python's zipfile / tarfile for the malicious ones —
so RapidR's reader is checked against archives it didn't write.

    python3 -I crates/rapidr-archive/tests/fixtures/make_fixtures.py

Needs zip, 7zz, bsdtar, gzip, xz and zstd on the PATH. Writes the fixtures beside this script; the files are
RapidR's own (a made-up 64-byte .z80 snapshot, a readme): no third-party
content. Times are fixed (2026-10-29 12:00 local) so zips list the same.
"""

import io
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
STAMP = "202610291200"  # touch -t
WHEN = (2026, 10, 29, 12, 0, 0)

# A made-up snapshot: "DI; XOR A; LD DE,..." then a counting pattern — RapidR's
# own bytes, not any real program.
Z80 = bytes([0xF3, 0xAF, 0x11, 0xFF, 0xFF, 0xC3, 0xCB, 0x11]) + bytes(range(56))
README = b"Hello from RapidR!\nThis archive holds one Z80 snapshot.\n"


def run(*args, cwd):
    subprocess.run(args, cwd=cwd, check=True, stdout=subprocess.DEVNULL)


def tree(root):
    os.makedirs(os.path.join(root, "roms"), exist_ok=True)
    with open(os.path.join(root, "readme.txt"), "wb") as f:
        f.write(README)
    with open(os.path.join(root, "roms", "hello.z80"), "wb") as f:
        f.write(Z80)
    for p in ["readme.txt", "roms/hello.z80", "roms"]:
        run("touch", "-t", STAMP, p, cwd=root)


def out(name):
    return os.path.join(HERE, name)


def main():
    for tool in ["zip", "7zz", "bsdtar", "gzip", "xz", "zstd"]:
        if not shutil.which(tool):
            sys.exit(f"{tool} not found")
    work = tempfile.mkdtemp(prefix="rarchive-fixtures-")
    src = os.path.join(work, "src")
    tree(src)
    files = ["readme.txt", "roms/hello.z80"]

    # The ordinary ones: the same two files in every format.
    for f in ["games.zip", "games.7z", "games.tar", "games.tar.gz", "games.tar.xz", "games.tar.zst"]:
        if os.path.exists(out(f)):
            os.remove(out(f))
    run("zip", "-X", "-q", "-9", out("games.zip"), "readme.txt", "roms/", "roms/hello.z80", cwd=src)
    run("7zz", "a", "-bd", "-mx=9", out("games.7z"), "readme.txt", "roms", cwd=src)
    run("bsdtar", "--format", "ustar", "--uid", "0", "--gid", "0", "--uname", "", "--gname", "", "-cf", out("games.tar"), *files, cwd=src)
    with open(out("games.tar"), "rb") as f:
        tar_bytes = f.read()
    with open(out("games.tar.gz"), "wb") as f:
        f.write(subprocess.run(["gzip", "-9", "-n", "-c"], input=tar_bytes, check=True, capture_output=True).stdout)
    with open(out("games.tar.xz"), "wb") as f:
        f.write(subprocess.run(["xz", "-6", "-c"], input=tar_bytes, check=True, capture_output=True).stdout)
    with open(out("games.tar.zst"), "wb") as f:
        f.write(subprocess.run(["zstd", "-q", "-19", "-c"], input=tar_bytes, check=True, capture_output=True).stdout)

    # One file compressed: the .gz keeps its name in its header.
    shutil.copy(os.path.join(src, "roms", "hello.z80"), os.path.join(work, "hello.z80"))
    run("touch", "-t", STAMP, "hello.z80", cwd=work)
    with open(out("hello.z80.gz"), "wb") as f:
        f.write(subprocess.run(["gzip", "-9", "-c", "hello.z80"], cwd=work, check=True, capture_output=True).stdout)
    with open(out("hello.z80.xz"), "wb") as f:
        f.write(subprocess.run(["xz", "-c"], input=Z80, check=True, capture_output=True).stdout)
    with open(out("hello.z80.zst"), "wb") as f:
        f.write(subprocess.run(["zstd", "-q", "-c"], input=Z80, check=True, capture_output=True).stdout)

    # Zip-slip: names that leave the folder, beside a good one.
    with zipfile.ZipFile(out("evil-slip.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        for name in ["ok.txt", "../evil.txt", "/tmp/rapidr-abs.txt", "sub/../../up.txt"]:
            info = zipfile.ZipInfo(name, WHEN)
            z.writestr(info, b"zip-slip test\n")
    # A tar with a link to outside and a file "through" it.
    with tarfile.open(out("evil-link.tar"), "w", format=tarfile.USTAR_FORMAT) as t:
        good = tarfile.TarInfo("ok.txt")
        good.size = 3
        good.mtime = 1_793_000_000
        t.addfile(good, io.BytesIO(b"ok\n"))
        link = tarfile.TarInfo("escape")
        link.type = tarfile.SYMTYPE
        link.linkname = "/tmp"
        link.mtime = 1_793_000_000
        t.addfile(link)
        through = tarfile.TarInfo("escape/rapidr-through-link.txt")
        through.size = 4
        through.mtime = 1_793_000_000
        t.addfile(through, io.BytesIO(b"bad\n"))

    # A zip bomb: 30 MB of zeros deflated to about 30 KB (≈ 1030 to 1).
    with zipfile.ZipFile(out("bomb.zip"), "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        z.writestr(zipfile.ZipInfo("zeros.bin", WHEN), bytes(30 << 20), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)

    # Encrypted: ZipCrypto (zip -P), a 7z with its files encrypted (its
    # list readable), and one with its list encrypted too.
    for f in ["encrypted.zip", "encrypted-files.7z", "encrypted-list.7z"]:
        if os.path.exists(out(f)):
            os.remove(out(f))
    run("zip", "-X", "-q", "-P", "secret", out("encrypted.zip"), "readme.txt", cwd=src)
    run("7zz", "a", "-bd", "-psecret", "-mhe=off", out("encrypted-files.7z"), "readme.txt", cwd=src)
    run("7zz", "a", "-bd", "-psecret", "-mhe=on", out("encrypted-list.7z"), "readme.txt", cwd=src)

    # An old zip whose names are in code page 437 (no UTF-8 flag): "café.txt"
    # with é as 0x82.
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_STORED) as z:
        z.writestr(zipfile.ZipInfo("caf_.txt", WHEN), b"cp437\n")
    data = buf.getvalue().replace(b"caf_.txt", b"caf\x82.txt")
    with open(out("cp437.zip"), "wb") as f:
        f.write(data)

    # A RAR's signature (not a real RAR: only its first bytes, which is all
    # RapidR looks at before refusing it).
    with open(out("not-open.rar"), "wb") as f:
        f.write(b"Rar!\x1a\x07\x01\x00" + bytes(24))

    shutil.rmtree(work)
    print("fixtures made in", HERE)


if __name__ == "__main__":
    main()
