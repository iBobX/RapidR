#!/usr/bin/env python3
"""Seven-segment character bitmaps for QDigDisplay.inc (RapidQ's include
library draws characters 32..64 from 32.bmp .. 64.bmp, 12x24 each; the
originals weren't distributed). Usage: python3 tools/make_digit_bitmaps.py DIR"""
import struct, sys, os

W, H = 12, 24
BG, ON, OFF = (0, 0, 0), (0, 230, 0), (0, 40, 0)
# segments: a top, b top-right, c bottom-right, d bottom, e bottom-left, f top-left, g middle
SEG = {
    "0": "abcdef", "1": "bc", "2": "abged", "3": "abgcd", "4": "fgbc", "5": "afgcd",
    "6": "afgedc", "7": "abc", "8": "abcdefg", "9": "abcdfg", "-": "g", "=": "gd",
    "_": "d", "?": "abge", "@": "abcdeg", "'": "f", '"': "fb", "(": "adef", ")": "abcd",
    "[": "adef", "<": "ged", ">": "gcd", "*": "fbg", "+": "g", "/": "be", "\\": "fc", "$": "afgcd", "%": "fbge",
}
DOTS = {".": [(9, 21)], ",": [(9, 21), (8, 22)], ":": [(5, 8), (5, 16)], ";": [(5, 8), (5, 16), (4, 18)], "!": [(9, 21)]}

def rects(seg):
    t = 2  # thickness
    return {
        "a": (2, 1, 8, t), "d": (2, H - 1 - t, 8, t), "g": (2, H // 2 - 1, 8, t),
        "f": (1, 2, t, H // 2 - 3), "b": (W - 1 - t, 2, t, H // 2 - 3),
        "e": (1, H // 2 + 1, t, H // 2 - 3), "c": (W - 1 - t, H // 2 + 1, t, H // 2 - 3),
    }[seg]

def glyph(ch):
    px = [[BG] * W for _ in range(H)]
    def fill(x, y, w, h, c):
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                px[yy][xx] = c
    lit = SEG.get(ch, "b" if ch == "!" else "")
    for s in "abcdefg":
        fill(*rects(s), ON if s in lit else OFF)
    for (x, y) in DOTS.get(ch, []):
        fill(x, y, 2, 2, ON)
    return px

def bmp(px):
    row = (W * 3 + 3) & ~3
    data = b"".join(bytes(b for (r, g, bl) in line for b in (bl, g, r)) + b"\0" * (row - W * 3) for line in reversed(px))
    return b"BM" + struct.pack("<IHHI", 54 + len(data), 0, 0, 54) + struct.pack("<IiiHHIIiiII", 40, W, H, 1, 24, 0, len(data), 2835, 2835, 0, 0) + data

out = sys.argv[1] if len(sys.argv) > 1 else "."
os.makedirs(out, exist_ok=True)
for code in range(32, 65):
    with open(os.path.join(out, f"{code}.bmp"), "wb") as f:
        f.write(bmp(glyph(chr(code))))
