"""Pixel check (dev tool; needs rsvg-convert and Pillow): each icon's
hinted variants at their own size, magnified with hard pixels.

    python3 design/icons/tools/pixels.py OUT.png name[,name…] [--theme modern] [--mag 8]
"""

import argparse
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kit  # noqa: E402
import catalog  # noqa: E402,F401
from palette import THEMES, themed  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402


def raster(svg, px, fg):
    svg = svg.replace("<svg ", f'<svg color="{fg}" ', 1)
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "i.svg")
        o = os.path.join(d, "i.png")
        open(p, "w").write(svg)
        subprocess.run(["rsvg-convert", "-w", str(px), "-h", str(px), p, "-o", o], check=True)
        return Image.open(o).convert("RGBA")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("names")
    ap.add_argument("--theme", default="modern")
    ap.add_argument("--mag", type=int, default=8)
    a = ap.parse_args()
    names = [n if "/" in n else next(k for k in kit.ICONS if k.endswith("/" + n)) for n in a.names.split(",")] if a.names != "*" else list(kit.ICONS)
    tokens, fg, bgs = THEMES[a.theme]
    mag = a.mag
    row_h = 32 * mag + 30
    W = (16 + 24 + 32) * mag + 80
    im = Image.new("RGB", (W, row_h * len(names) + 10), bgs[0])
    dr = ImageDraw.Draw(im)
    grid = (205, 205, 205) if a.theme in ("classic", "modern") else (60, 60, 60)
    for r, name in enumerate(names):
        x = 10
        y = 10 + r * row_h
        dr.text((x, y), name, fill=(120, 120, 120))
        y += 16
        for s in kit.SIZES:
            ic = raster(themed(kit.draw(name, s), a.theme), s, fg)
            big = ic.resize((s * mag, s * mag), Image.NEAREST)
            im.paste(big, (x, y), big)
            for i in range(s + 1):
                dr.line([(x + i * mag, y), (x + i * mag, y + s * mag)], fill=grid)
                dr.line([(x, y + i * mag), (x + s * mag, y + i * mag)], fill=grid)
            x += s * mag + 30
    im.save(a.out)
    print(a.out)


if __name__ == "__main__":
    main()
