#!/usr/bin/env python3
"""tests/visual/cases/words.bas word by word, zoomed, beside RapidQ's: one
PNG per word, its rows the scales (1×, 1.5×, 2×), its columns each RapidR
build given and RC.EXE (tests/visual/rapidq/words@1x / @2x; at 1.5× RC.EXE's
1× capture enlarged smoothly by 1.5, as Windows shows a program that isn't
DPI-aware at 150 %), regular and bold.

    tools/visual/words.py OUT_DIR [NAME=RAPIDR_BINARY …]

(default: new=./rapidr). Programs run with RAPIDR_PRINT_TO and
RAPIDR_REGISTRY in OUT_DIR. Needs Pillow.
"""
import glob
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
CASE = os.path.join(ROOT, "tests", "visual", "cases", "words.bas")
RAPIDQ = os.path.join(ROOT, "tests", "visual", "rapidq")
WORDS = ["program", "start", "Bread", "Price", "Right-click", "Notepad - untitled"]
SCALES = [1, 1.5, 2]
ZOOM = {1: 4, 1.5: 3, 2: 2}


def capture(binary, scale, work):
    """The words window at `scale`, as an image (RAPIDR_CAPTURE)."""
    tmp = tempfile.mkdtemp(dir=work)
    rrbc = os.path.join(tmp, "words.rrbc")
    subprocess.run([binary, "build-bc", CASE, "-o", rrbc], check=True, capture_output=True)
    env = dict(os.environ, RAPIDR_CAPTURE=os.path.join(tmp, "cap"), RAPIDR_SCALE=str(scale),
               RAPIDR_THEME="classic", RAPIDR_PRINT_TO=os.path.join(work, "prints"),
               RAPIDR_REGISTRY=os.path.join(tmp, "registry.reg"))
    try:
        subprocess.run([binary, "run-bc", rrbc], env=env, capture_output=True, timeout=40)
    except subprocess.TimeoutExpired:
        pass
    shots = sorted(glob.glob(os.path.join(tmp, "cap-*.bmp")))
    return Image.open(shots[0]).convert("RGB") if shots else None


def rapidq(scale):
    if scale == 1.5:
        im = Image.open(os.path.join(RAPIDQ, "words@1x-1.png")).convert("RGB")
        return im.resize((round(im.width * 1.5), round(im.height * 1.5)), Image.BILINEAR)
    return Image.open(os.path.join(RAPIDQ, f"words@{scale}x-1.png")).convert("RGB")


def crop(im, scale, row, bold):
    """Label `row`'s text (logical: x 8 or 140, y 8 + 18 row, 13 high)."""
    x0, w = (140, 130) if bold else (8, 120)
    y0 = 8 + 18 * row - 2
    box = tuple(round(v * scale) for v in (x0 - 2, y0, x0 + w, y0 + 17))
    c = im.crop(box)
    z = ZOOM[scale]
    return c.resize((c.width * z, c.height * z), Image.NEAREST)


def main():
    out = sys.argv[1]
    builds = [a.split("=", 1) for a in sys.argv[2:]] or [["new", os.path.join(ROOT, "rapidr")]]
    os.makedirs(out, exist_ok=True)
    work = tempfile.mkdtemp(prefix="words-", dir=out)
    shots = {}
    for name, binary in builds:
        for s in SCALES:
            shots[(name, s)] = capture(binary, s, work)
            if shots[(name, s)]:
                shots[(name, s)].save(os.path.join(out, f"words-{name}@{s}x.png"))
    for s in SCALES:
        shots[("RC.EXE", s)] = rapidq(s)
    columns = [n for n, _ in builds] + ["RC.EXE"]
    for row, word in enumerate(WORDS):
        tiles = []
        for s in SCALES:
            for bold in (False, True):
                tiles.append((f"{s}x{' bold' if bold else ''}", [crop(shots[(c, s)], s, row, bold) for c in columns]))
        cw = max(t.width for _, ts in tiles for t in ts) + 12
        rh = [max(t.height for t in ts) + 6 for _, ts in tiles]
        sheet = Image.new("RGB", (70 + cw * len(columns), 22 + sum(rh)), "white")
        d = ImageDraw.Draw(sheet)
        for i, c in enumerate(columns):
            d.text((70 + cw * i, 4), c, fill=(0, 0, 0))
        y = 22
        for (label, ts), h in zip(tiles, rh):
            d.text((4, y + 4), label, fill=(0, 0, 0))
            for i, t in enumerate(ts):
                sheet.paste(t, (70 + cw * i, y))
            y += h
        sheet.save(os.path.join(out, f"word-{word.replace(' ', '')}.png"))
    print("wrote", out)


if __name__ == "__main__":
    main()
