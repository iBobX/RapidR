#!/usr/bin/env python3
"""A manual screenshot from a 2x capture (tools/manual/shots.mjs):

    png.py <capture.bmp> <out.png> [x y w h] [--smooth]

crops to the logical rectangle (x, y, w, h — doubled for the 2x capture),
reduces to a 256-colour palette (UI pictures keep their look; --smooth
dithers it, for a picture with gradients — a colour wheel — that would
otherwise break into bands) and writes an optimized PNG; prints its size.
Fails over 250 KB.
"""
import os
import sys

from PIL import Image

args = [a for a in sys.argv[1:] if a != "--smooth"]
smooth = "--smooth" in sys.argv
src, out = args[0], args[1]
im = Image.open(src).convert("RGB")
if len(args) >= 6:
    x, y, w, h = (int(v) * 2 for v in args[2:6])
    im = im.crop((x, y, min(x + w, im.width), min(y + h, im.height)))
im_p = im.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
# (Pillow dithers only against a palette it is given: the one just made)
im = im.quantize(palette=im_p, dither=Image.Dither.FLOYDSTEINBERG) if smooth else im_p
im.save(out, optimize=True)
size = os.path.getsize(out)
print(f"{out} {im.width}x{im.height} {size // 1024} KB")
if size > 250 * 1024:
    sys.exit(f"{out} is {size // 1024} KB (over 250 KB): crop it")
