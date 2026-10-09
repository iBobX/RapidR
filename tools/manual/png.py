#!/usr/bin/env python3
"""A manual screenshot from a 2x capture (tools/manual/shots.mjs):

    png.py <capture.bmp> <out.png> [x y w h]

crops to the logical rectangle (x, y, w, h — doubled for the 2x capture),
reduces to a 256-colour palette (UI pictures keep their look) and writes an
optimized PNG; prints its size. Fails over 250 KB.
"""
import os
import sys

from PIL import Image

src, out = sys.argv[1], sys.argv[2]
im = Image.open(src).convert("RGB")
if len(sys.argv) >= 7:
    x, y, w, h = (int(v) * 2 for v in sys.argv[3:7])
    im = im.crop((x, y, min(x + w, im.width), min(y + h, im.height)))
im = im.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
im.save(out, optimize=True)
size = os.path.getsize(out)
print(f"{out} {im.width}x{im.height} {size // 1024} KB")
if size > 250 * 1024:
    sys.exit(f"{out} is {size // 1024} KB (over 250 KB): crop it")
