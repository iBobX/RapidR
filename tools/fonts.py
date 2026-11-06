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
                                        has which characters, the licence;
                                        and the Simplified Chinese subset
                                        (fonts.toml [subset]): the common
                                        characters in one file, for a page
                                        whose UI is Chinese
  python3 tools/fonts.py check [<dir>]  the legal rules for what `build` makes
                                        (docs/legal/review-2026-11-06-zh.md
                                        B-1 .. B-8); with <dir>, of that
                                        folder, else of a build in a scratch
                                        folder; skipped when the source font
                                        isn't cached

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
import tempfile
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


def subset_spec():
    """fonts.toml's [subset]: the Simplified Chinese subset."""
    with open(os.path.join(HERE, "fonts.toml"), "rb") as f:
        return tomllib.load(f)["subset"]


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


def rename(font, family, tag="chunk"):
    """The font's own family name: written into the file (name IDs 1, 3, 4, 6,
    16 and the CFF names), so a file lifted out of the bundle doesn't pass as
    the original. A chunk gets `RapidR Han 003`: fonts of one family name are
    one family to the text system, which would draw from the first chunk only.
    Kept as they are: the copyright (ID 0, Adobe's), the trademark notice
    (ID 7: "Noto is a trademark of Google Inc."), the designers, the licence
    and its URL (IDs 8 .. 14) — the OFL wants the notice and the licence in
    every copy, and a trademark notice stays."""
    ps = family.replace(" ", "")
    for rec in font["name"].names:
        if rec.nameID in (1, 16):
            rec.string = family
        elif rec.nameID == 4:
            rec.string = f"{family} Regular"
        elif rec.nameID == 6:
            rec.string = f"{ps}-Regular"
        elif rec.nameID == 3:
            rec.string = f"{ps}-Regular;{tag}"
    if "CFF " in font:
        cff = font["CFF "].cff
        cff.fontNames = [f"{ps}-Regular"]
        top = cff.topDictIndex[0]
        top.FamilyName = family
        top.FullName = f"{family} Regular"
        # (a CID-keyed font names its font dicts too — `NotoSansSC-Regular-Hangul`:
        # internal, never shown, but no "Noto" is left in the file)
        for fd in getattr(top, "FDArray", None) or []:
            name = getattr(fd, "FontName", None)
            if name and name.startswith("Noto"):
                fd.FontName = f"{ps}-Regular" + (name.split("-Regular", 1)[1] if "-Regular" in name else "")


def ranges(codepoints):
    """Sorted codepoints as [first, last] runs."""
    out = []
    for c in sorted(codepoints):
        if out and out[-1][1] == c - 1:
            out[-1][1] = c
        else:
            out.append([c, c])
    return out


def options(layout_features, desubroutinize):
    from fontTools import subset

    o = subset.Options()
    o.layout_features = layout_features
    o.name_IDs = ["*"]
    o.name_languages = ["*"]
    o.notdef_outline = True
    o.hinting = False
    o.glyph_names = False
    o.desubroutinize = desubroutinize
    return o


def cut(path, codepoints, opts):
    """A Modified Version of the font (OFL 1.1): only `codepoints`' glyphs."""
    from fontTools import subset
    from fontTools.ttLib import TTFont

    sub = subset.Subsetter(opts)
    f = TTFont(path)
    sub.populate(unicodes=list(codepoints))
    sub.subset(f)
    return f


def subset_chars(spec):
    """The characters of the Simplified Chinese subset (fonts.toml [subset]):
    the 3,755 hanzi of GB 2312 level 1 (rows 16-55, from Python's own `gb2312`
    codec: no downloaded word list), every non-ASCII character the Chinese
    catalogs show, and the CJK, full-width and general punctuation."""
    chars = set()
    for hi in range(0xB0, 0xD8):
        for lo in range(0xA1, 0xFF):
            try:
                c = bytes([hi, lo]).decode("gb2312")
            except UnicodeDecodeError:
                continue
            if len(c) == 1:
                chars.add(c)
    for catalog in spec["catalogs"]:
        with open(os.path.join(ROOT, catalog), encoding="utf-8") as f:
            for line in f:
                if line.startswith(("msgstr", '"')):
                    chars.update(ch for ch in line if ord(ch) > 0x7F)
    for first, last in ((0x3000, 0x303F), (0xFF00, 0xFFEF), (0x2010, 0x202F)):
        chars.update(chr(c) for c in range(first, last + 1))
    return chars


def build_subset(path, spec, taken):
    """The Simplified Chinese subset of the font at `path`: (the font, its
    codepoints), without what `taken` (an earlier font's) already draws. Its
    own family name, written in (`rename`): not Noto's."""
    from fontTools.ttLib import TTFont

    cmap = TTFont(path, lazy=True).getBestCmap()
    want = sorted(ord(c) for c in subset_chars(spec) if ord(c) in cmap and ord(c) not in taken)
    # (no layout features: the Han characters need none, and the file is a
    # third smaller)
    f = cut(path, want, options([], False))
    rename(f, spec["family"], "subset")
    return f, want


def build(out_dir, fetch=True, only_first_chunk=False):
    """The folder a page loads its fallback fonts from. `only_first_chunk`:
    one chunk of each split font (for `check`: names, not the whole 5 MB)."""
    from fontTools.ttLib import TTFont

    os.makedirs(out_dir, exist_ok=True)
    for name in os.listdir(out_dir):
        if name.endswith((".otf", ".ttf", ".json")):
            os.remove(os.path.join(out_dir, name))
    index = {"version": 1, "chunks": []}
    covered = set()
    missing = []
    spec = subset_spec()
    for font in manifest():
        path = source(font, fetch)
        if path is None:
            missing.append(font["family"])
            continue
        cmap = TTFont(path, lazy=True).getBestCmap()
        # (the Simplified Chinese subset goes in the index before the chunks
        # it is cut from: the page takes the first file that has a
        # character, and asks for this one only when its UI is Chinese;
        # the chunks keep every character, the fallback for a rarer one)
        if font["family"] == spec["from"]:
            sub, want = build_subset(path, spec, covered)
            sub.save(os.path.join(out_dir, spec["file"]))
            index["chunks"].append({"family": spec["family"], "file": spec["file"], "ranges": ranges(want), "for": spec["for"]})
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
        # (each chunk a family of its own, a neutral one — `rename`; OFL 1.1
        # allows it, no Reserved Font Name being declared, and "Noto" is
        # Google's trademark, which the OFL doesn't license)
        for n, start in enumerate(range(0, len(mine), CHUNK)):
            if only_first_chunk and n > 0:
                break
            part = mine[start:start + CHUNK]
            name = f"{stem}.{n:03d}{ext}"
            f = cut(path, part, options(["*"], True))
            family = f"{font['chunk_family']} {n:03d}"
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


def check(out_dir=None):
    """The legal rules for the fonts `build` makes — the Simplified Chinese
    subset above all (docs/legal/review-2026-11-06-zh.md, conditions B-1 ..
    B-8). Exit 1 when one fails. The built folder is `out_dir`, else a build
    of the subset and one chunk of each CJK font, in a scratch folder (skipped,
    and said, when the source font isn't cached or fontTools isn't there)."""
    problems = []

    def need(ok, what):
        if not ok:
            problems.append(what)

    def text(rel):
        with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
            return f.read()

    spec = subset_spec()
    fonts = manifest()
    # B-2: a family name of its own, no "Noto" in it
    need("noto" not in spec["family"].lower(), f"the subset's family name `{spec['family']}` has \"Noto\" in it (Google's trademark)")
    for font in fonts:
        if not font["committed"] and "chunk_family" in font:
            need("noto" not in font["chunk_family"].lower(), f"{font['family']}'s chunk family `{font['chunk_family']}` has \"Noto\" in it")
    need(any(f["family"] == spec["from"] for f in fonts), f"the subset is cut from `{spec['from']}`, which fonts.toml doesn't list")
    # B-8: cut from the pinned file only
    for f in fonts:
        if f["family"] == spec["from"]:
            need("release_sha256" in f and len(f["release_sha256"]) == 64, f"{f['family']} isn't pinned by SHA-256")
    # B-1, B-4: the licence and Adobe's copyright travel with it, OFL only
    ofl = text("fonts/fallback/OFL.txt")
    need("(c) 2014-2021 Adobe" in ofl, "fonts/fallback/OFL.txt lost Adobe's copyright line")
    need("SIL OPEN FONT LICENSE Version 1.1" in ofl and "PERMISSION & CONDITIONS" in ofl, "fonts/fallback/OFL.txt isn't the whole OFL 1.1")
    need(spec["family"] in ofl, f"fonts/fallback/OFL.txt doesn't credit the subset `{spec['family']}`")
    # B-6: the notices name it (the shipped ones: notices.rs; the repo's)
    for rel in ("LICENSES.md", "THIRD_PARTY_NOTICES.md", "crates/rapidr-cli/src/notices.rs", "tools/third_party_notices.py", "docs/legal/i18n-zh-font.md"):
        need(spec["family"] in text(rel), f"{rel} doesn't name the subset `{spec['family']}`")
    # B-7: same origin only; and the subset only for a Chinese UI
    web = text("crates/rapidr-runtime-web/src/fonts_web.rs")
    code = [l for l in web.splitlines() if not l.lstrip().startswith("//")]
    need(not any("http://" in l or "https://" in l for l in code), "fonts_web.rs names a URL: the fallback fonts come from the page's own origin only")
    need("chinese_ui()" in web and '"for"' in web, "fonts_web.rs doesn't ask for the subset only when the UI is Chinese")
    for f in fonts:
        for key in ("release", "url"):
            need(key not in f or f[key].startswith(("https://github.com/", "https://raw.githubusercontent.com/")), f"{f['family']}: {f.get(key)} isn't a pinned GitHub asset")

    skipped = None
    try:
        from fontTools.ttLib import TTFont
    except ImportError:
        TTFont = None
        skipped = "fontTools isn't installed (pip install fonttools)"
    tmp = None
    if TTFont and out_dir is None:
        sc = next(f for f in fonts if f["family"] == spec["from"])
        if source(sc, fetch=False) is None:
            skipped = "the source fonts aren't cached (python3 tools/fonts.py fetch)"
        else:
            tmp = tempfile.mkdtemp(prefix="fonts-check-")
            out_dir = tmp
            build(out_dir, fetch=False, only_first_chunk=True)
    if TTFont and out_dir is not None:
        with open(os.path.join(out_dir, "index.json")) as f:
            index = json.load(f)
        need(os.path.exists(os.path.join(out_dir, "OFL.txt")), "the fonts folder has fonts but no OFL.txt (B-1)")
        names = {c["file"] for c in index["chunks"]}
        for name in os.listdir(out_dir):
            if name.endswith((".otf", ".ttf")):
                need(name in names, f"{name} is in the fonts folder but not in index.json")
        subs = [c for c in index["chunks"] if c["family"] == spec["family"]]
        need(len(subs) == 1, "index.json lists the subset not exactly once")
        if subs:
            first_han = next((i for i, c in enumerate(index["chunks"]) if any(a <= 0x4E00 <= b for a, b in c["ranges"])), None)
            need(first_han is not None and index["chunks"][first_han]["family"] == spec["family"], "the subset isn't the first file in index.json with Han characters")
            need(subs[0].get("for") == spec["for"], f"the subset isn't marked for {spec['for']} only")
            sub_path = os.path.join(out_dir, subs[0]["file"])
            need(0x20000 < os.path.getsize(sub_path) < 0x180000, "the subset's size is off (expected ~0.4 to 1.5 MB)")
            # (every Han character of the Chinese catalogs is in it: one file for a Chinese UI)
            used = {ord(ch) for ch in subset_chars(spec) if 0x4E00 <= ord(ch) <= 0x9FFF}
            have = set(TTFont(sub_path, lazy=True).getBestCmap())
            for f in fonts:
                src = source(f, fetch=False) if f["family"] == spec["from"] else None
                if src:
                    need(not ((used & set(TTFont(src, lazy=True).getBestCmap())) - have), "the subset lacks Han characters the catalogs use")
        # B-1, B-2: in every font file made from Noto: the family names are
        # neutral, Adobe's copyright, the trademark notice and the licence are kept
        for c in index["chunks"]:
            if c["file"].startswith(("NotoSans-", "NotoSansSymbols", "Noto-COLR")):
                continue
            nm = TTFont(os.path.join(out_dir, c["file"]), lazy=True)["name"]
            by = {}
            for rec in nm.names:
                by.setdefault(rec.nameID, []).append(rec.toUnicode())
            for i in (1, 3, 4, 6, 16):
                need(all("noto" not in v.lower() for v in by.get(i, [])), f"{c['file']}: name ID {i} has \"Noto\" in it")
            need(all(c["family"] in v for v in by.get(1, [])), f"{c['file']}: family name ID 1 isn't `{c['family']}`")
            need(any("Adobe" in v for v in by.get(0, [])), f"{c['file']}: the copyright (name ID 0) lost Adobe")
            need(any("trademark" in v.lower() for v in by.get(7, [])), f"{c['file']}: the trademark notice (name ID 7) is gone")
            need(any("SIL Open Font License" in v for v in by.get(13, [])), f"{c['file']}: the licence (name ID 13) is gone")
            # (nowhere else in the file either — but for the trademark notice itself)
            with open(os.path.join(out_dir, c["file"]), "rb") as fh:
                raw = fh.read()
            notice = "Noto is a trademark of Google Inc."
            for enc in ("ascii", "utf-16-be", "mac-roman"):
                raw = raw.replace(notice.encode(enc), b"")
            need(b"Noto" not in raw and "Noto".encode("utf-16-be") not in raw, f"{c['file']}: \"Noto\" is still in the file outside the trademark notice")
    if tmp:
        shutil.rmtree(tmp, ignore_errors=True)
    for p in problems:
        print(f"fonts: FAIL: {p}")
    if problems:
        sys.exit(1)
    print("fonts: ok" + (f" (not checked on a build: {skipped})" if skipped else ""))


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "fetch":
        for font in manifest():
            if source(font) is None:
                sys.exit(1)
        print("fallback fonts fetched into target/fonts-src")
    elif len(sys.argv) == 3 and sys.argv[1] == "build":
        build(os.path.abspath(sys.argv[2]))
    elif sys.argv[1:2] == ["check"] and len(sys.argv) <= 3:
        check(os.path.abspath(sys.argv[2]) if len(sys.argv) == 3 else None)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
