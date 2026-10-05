"""Writes the final logo files into logo/: the mark, its monochrome
versions, and the horizontal lockup (mark + outlined "RapidR" wordmark)
for light and dark backgrounds."""
import os
import sys

from brandkit import (BRAND, INK, PAPER, BLUE, CYAN, WHITE, svg_doc, squircle, text_path,
                      transform, _num)
from marks import run_r_centered

OUT = os.path.join(BRAND, "logo")
os.makedirs(OUT, exist_ok=True)

TILE = squircle(0, 0, 100, 100, 34)
BLUE_ON_DARK = "#6E93FF"   # the wordmark's final R on dark backgrounds (AA on INK)

# Wordmark: Inter Bold, tracked -1.5 %, outlined. Cap height 0.727 em.
WM_WEIGHT = 700
WM_SIZE = 66
WM_TRACK = -15
GAP = 26          # tile -> wordmark
BASELINE = 50 + 0.727 * WM_SIZE / 2


def grad(gid="rg"):
    return (f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="0" y1="100" x2="100" y2="0">'
            f'<stop offset="0" stop-color="{BLUE}"/><stop offset="1" stop-color="{CYAN}"/></linearGradient>')


def mark_body(style):
    """style: colour | mono-ink | mono-white. Mono marks knock the R out of
    the tile, so they work on any background in one colour."""
    if style == "colour":
        return f'<path fill="url(#rg)" d="{TILE}"/><path fill="{WHITE}" d="{run_r_centered()}"/>'
    # evenodd: the R becomes a hole in the tile, and its counter (inside the
    # R) flips back to filled, which is exactly the knocked-out look we want
    col = INK if style == "mono-ink" else WHITE
    return f'<path fill="{col}" fill-rule="evenodd" d="{TILE} {run_r_centered()}"/>'


def wordmark(x):
    rapid, w1 = text_path("Rapid", WM_SIZE, x, BASELINE, weight=WM_WEIGHT, tracking=WM_TRACK)
    # the final R follows "Rapid" with Inter's d-R spacing
    full, wfull = text_path("RapidR", WM_SIZE, x, BASELINE, weight=WM_WEIGHT, tracking=WM_TRACK)
    r_only, wr = text_path("R", WM_SIZE, 0, BASELINE, weight=WM_WEIGHT)
    r_only = transform(r_only, (1, 0, 0, 1, x + wfull - wr, 0))
    return rapid, r_only, wfull


def lockup(style):
    """style: light (for light backgrounds), dark, mono-ink, mono-white."""
    rapid, r_last, w = wordmark(100 + GAP)
    total_w = 100 + GAP + w
    if style == "light":
        body = mark_body("colour") + f'<path fill="{INK}" d="{rapid}"/><path fill="{BLUE}" d="{r_last}"/>'
    elif style == "dark":
        body = mark_body("colour") + f'<path fill="{WHITE}" d="{rapid}"/><path fill="{BLUE_ON_DARK}" d="{r_last}"/>'
    elif style == "mono-ink":
        body = mark_body("mono-ink") + f'<path fill="{INK}" d="{rapid} {r_last}"/>'
    else:
        body = mark_body("mono-white") + f'<path fill="{WHITE}" d="{rapid} {r_last}"/>'
    return body, total_w


def write(name, body, w, h, defs="", title="RapidR"):
    open(os.path.join(OUT, name + ".svg"), "w").write(svg_doc(body, w, h, defs=defs, title=title))


def main():
    write("rapidr-mark", mark_body("colour"), 100, 100, defs=grad(), title="RapidR mark")
    write("rapidr-mark-mono-ink", mark_body("mono-ink"), 100, 100, title="RapidR mark")
    write("rapidr-mark-mono-white", mark_body("mono-white"), 100, 100, title="RapidR mark")
    # the bare glyph, for places that already provide a coloured ground
    write("rapidr-glyph", f'<path fill="url(#rg)" d="{transform(run_r_centered(), (1, 0, 0, 1, -24, -19))}"/>',
          52, 62, defs=grad(), title="RapidR glyph")
    for style, name in [("light", "rapidr-lockup"), ("dark", "rapidr-lockup-dark"),
                        ("mono-ink", "rapidr-lockup-mono-ink"), ("mono-white", "rapidr-lockup-mono-white")]:
        body, w = lockup(style)
        write(name, body, round(w + 0.5), 100, defs=grad() if style in ("light", "dark") else "",
              title="RapidR")


if __name__ == "__main__":
    main()
