#!/usr/bin/env python3
"""A contact sheet of a folder of screenshots (the use-audit's, for the release's look-through).

  python3 tools/release/contact_sheet.py <folder of PNGs> <out.png> [--match REGEX] [--cols N] [--thumb WIDTH]
         [--title TEXT]

Every PNG under the folder whose name matches REGEX (default: all), sorted by name, scaled to WIDTH
pixels (default 480), in a grid of N columns (default 4), each under its file name. Needs Pillow
(`python3 -m pip install pillow`). Made to be looked at: one sheet per feature group, then the lot.
"""
import argparse
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("folder")
    ap.add_argument("out")
    ap.add_argument("--match", default=".")
    ap.add_argument("--cols", type=int, default=4)
    ap.add_argument("--thumb", type=int, default=480)
    ap.add_argument("--title", default="")
    a = ap.parse_args()
    rx = re.compile(a.match)
    files = sorted(p for p in Path(a.folder).rglob("*.png") if rx.search(p.name))
    if not files:
        print("no pictures match", file=sys.stderr)
        return 1
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 13)
        big = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 20)
    except OSError:
        font = big = ImageFont.load_default()
    thumbs = []
    for p in files:
        im = Image.open(p).convert("RGB")
        h = max(1, round(im.height * a.thumb / im.width))
        thumbs.append((p.name, im.resize((a.thumb, h), Image.LANCZOS)))
    pad, label = 10, 20
    cols = min(a.cols, len(thumbs))
    rows = [thumbs[i : i + cols] for i in range(0, len(thumbs), cols)]
    top = 36 if a.title else 0
    heights = [max(t.height for _, t in row) + label for row in rows]
    sheet = Image.new("RGB", (cols * (a.thumb + pad) + pad, sum(heights) + pad * (len(rows) + 1) + top), (236, 236, 240))
    d = ImageDraw.Draw(sheet)
    if a.title:
        d.text((pad, 8), a.title, fill=(20, 20, 30), font=big)
    y = pad + top
    for row, hgt in zip(rows, heights):
        for c, (name, t) in enumerate(row):
            x = pad + c * (a.thumb + pad)
            d.text((x, y), name[:70], fill=(40, 40, 60), font=font)
            sheet.paste(t, (x, y + label))
            d.rectangle((x - 1, y + label - 1, x + t.width, y + label + t.height), outline=(180, 180, 195))
        y += hgt + pad
    Path(a.out).parent.mkdir(parents=True, exist_ok=True)
    sheet.save(a.out)
    print(f"{a.out}: {len(files)} pictures, {sheet.width} x {sheet.height}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
