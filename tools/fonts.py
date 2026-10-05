#!/usr/bin/env python3
"""The fallback fonts (fonts/fallback/fonts.toml, docs/web-host-plan.md §3.7):

  python3 tools/fonts.py fetch          the fonts not in the repo (Noto Sans SC,
                                        KR …) from their pinned Noto release
                                        assets into target/fonts-src (cached;
                                        each archive's SHA-256 checked)
  python3 tools/fonts.py build <dir>    the chunks a page loads on demand into
                                        <dir> (target/web/fonts): each font
                                        split by Unicode range into files of
                                        ~200 KB, index.json saying which file
                                        has which characters, the licence

`build` fetches what isn't cached first; without the network and without a
cache it builds the committed fonts only and says how to get the rest. An
installed RapidR ships the built chunks and never runs this.

Splitting needs fontTools (MIT): pip install fonttools.
"""

import hashlib
import io
import json
import os
import shutil
import sys
import tomllib
import urllib.request
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HERE = os.path.join(ROOT, "fonts", "fallback")
CACHE = os.path.join(ROOT, "target", "fonts-src")
# (a chunk's glyphs: ~200 KB of CJK outlines)
CHUNK = 900


def manifest():
    with open(os.path.join(HERE, "fonts.toml"), "rb") as f:
        return tomllib.load(f)["font"]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def source(font, fetch=True):
    """The font's file: the committed one, else the cached one (fetched
    first when allowed). None when it isn't there and can't be had."""
    if font["committed"]:
        return os.path.join(HERE, font["file"])
    path = os.path.join(CACHE, font["file"])
    if os.path.exists(path):
        if "sha256" in font and sha256(open(path, "rb").read()) != font["sha256"]:
            sys.exit(f"error: {path}: SHA-256 isn't the pinned {font['sha256']} (delete it to fetch again)")
        return path
    if not fetch:
        return None
    os.makedirs(CACHE, exist_ok=True)
    # (a font file itself, pinned by its SHA-256)
    if "url" in font:
        print(f"fetching {font['url']} …", flush=True)
        try:
            req = urllib.request.Request(font["url"], headers={"User-Agent": "rapidr-fonts"})
            data = urllib.request.urlopen(req, timeout=600).read()
        except OSError as e:
            print(f"  could not fetch it ({e}); offline, {font['family']} is left out — "
                  f"run `python3 tools/fonts.py fetch` with the network, then build again", file=sys.stderr)
            return None
        if sha256(data) != font["sha256"]:
            sys.exit(f"error: {font['url']}: SHA-256 {sha256(data)} isn't the pinned {font['sha256']}")
        with open(path, "wb") as f:
            f.write(data)
        return path
    archive = os.path.join(CACHE, font["release"].rsplit("/", 1)[1])
    if not os.path.exists(archive):
        print(f"fetching {font['release']} …", flush=True)
        try:
            req = urllib.request.Request(font["release"], headers={"User-Agent": "rapidr-fonts"})
            data = urllib.request.urlopen(req, timeout=600).read()
        except OSError as e:
            print(f"  could not fetch it ({e}); offline, {font['family']} is left out — "
                  f"run `python3 tools/fonts.py fetch` with the network, then build again", file=sys.stderr)
            return None
        if sha256(data) != font["release_sha256"]:
            sys.exit(f"error: {archive}: SHA-256 {sha256(data)} isn't the pinned {font['release_sha256']}")
        with open(archive, "wb") as f:
            f.write(data)
    with open(archive, "rb") as f:
        data = f.read()
    if sha256(data) != font["release_sha256"]:
        sys.exit(f"error: {archive}: SHA-256 {sha256(data)} isn't the pinned {font['release_sha256']} (delete it to fetch again)")
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        with open(path, "wb") as f:
            f.write(z.read(font["member"]))
    return path


def rename(font, family):
    """The chunk's own family name (`Noto Sans SC 003`): fonts of one
    family name are one family to the text system, which would draw from
    the first chunk only."""
    ps = family.replace(" ", "")
    for rec in font["name"].names:
        if rec.nameID in (1, 16):
            rec.string = family
        elif rec.nameID == 4:
            rec.string = f"{family} Regular"
        elif rec.nameID == 6:
            rec.string = f"{ps}-Regular"
        elif rec.nameID == 3:
            rec.string = f"{ps}-Regular;chunk"
    if "CFF " in font:
        cff = font["CFF "].cff
        cff.fontNames = [f"{ps}-Regular"]
        top = cff.topDictIndex[0]
        top.FamilyName = family
        top.FullName = f"{family} Regular"


def ranges(codepoints):
    """Sorted codepoints as [first, last] runs."""
    out = []
    for c in sorted(codepoints):
        if out and out[-1][1] == c - 1:
            out[-1][1] = c
        else:
            out.append([c, c])
    return out


def build(out_dir):
    from fontTools import subset
    from fontTools.ttLib import TTFont

    os.makedirs(out_dir, exist_ok=True)
    for name in os.listdir(out_dir):
        if name.endswith((".otf", ".ttf", ".json")):
            os.remove(os.path.join(out_dir, name))
    index = {"version": 1, "chunks": []}
    covered = set()
    missing = []
    for font in manifest():
        path = source(font)
        if path is None:
            missing.append(font["family"])
            continue
        cmap = TTFont(path, lazy=True).getBestCmap()
        # (what an earlier font has stays its: the first in the list draws it)
        mine = sorted(set(cmap) - covered)
        covered |= set(mine)
        stem, ext = os.path.splitext(font["file"])
        if font["committed"] or not font.get("split", True):
            # (small, or not to be split: the whole file, one chunk)
            name = font["file"]
            shutil.copyfile(path, os.path.join(out_dir, name))
            index["chunks"].append({"family": font["family"], "file": name, "ranges": ranges(mine)})
            continue
        # (each chunk a family of its own: `rename`; OFL 1.1 allows it, no
        # Reserved Font Name being declared)
        for n, start in enumerate(range(0, len(mine), CHUNK)):
            part = mine[start:start + CHUNK]
            name = f"{stem}.{n:03d}{ext}"
            options = subset.Options()
            options.layout_features = ["*"]
            options.name_IDs = ["*"]
            options.name_languages = ["*"]
            options.notdef_outline = True
            options.hinting = False
            options.desubroutinize = True
            sub = subset.Subsetter(options)
            f = TTFont(path)
            sub.populate(unicodes=part)
            sub.subset(f)
            family = f"{font['family']} {n:03d}"
            rename(f, family)
            f.save(os.path.join(out_dir, name))
            index["chunks"].append({"family": family, "file": name, "ranges": ranges(part)})
    shutil.copyfile(os.path.join(HERE, "OFL.txt"), os.path.join(out_dir, "OFL.txt"))
    with open(os.path.join(out_dir, "index.json"), "w") as f:
        json.dump(index, f, separators=(",", ":"))
    size = sum(os.path.getsize(os.path.join(out_dir, c["file"])) for c in index["chunks"])
    print(f"fallback fonts: {len(index['chunks'])} chunks, {size / 1e6:.1f} MB, into {out_dir}")
    if missing:
        print(f"  left out (not fetched): {', '.join(missing)} — run `python3 tools/fonts.py fetch`", file=sys.stderr)


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "fetch":
        for font in manifest():
            if source(font) is None:
                sys.exit(1)
        print("fallback fonts fetched into target/fonts-src")
    elif len(sys.argv) == 3 and sys.argv[1] == "build":
        build(os.path.abspath(sys.argv[2]))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
