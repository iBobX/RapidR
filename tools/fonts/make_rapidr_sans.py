#!/usr/bin/env python3
"""RapidR Sans: the face RapidR draws "MS Sans Serif" with — RapidQ's
default font (every component's, 8 pt) — so a form laid out for RapidQ fits
in RapidR as it did there.

MS Sans Serif is a Windows bitmap font; no open font has its widths
(Liberation Sans has Arial's, a few percent wider: "Password:" is 52 pixels
at 8 pt against MS Sans Serif's 49, and a label sized for one clips the
other). RapidR Sans is Liberation Sans (SIL Open Font License 1.1, its
outlines and character set) with each Windows-1252 character as wide as MS
Sans Serif's at 8 pt on a 96-dpi screen, where its em is 11 pixels, and
MS Sans Serif's vertical metrics: ascent 11 pixels, descent 2, so a line is
13 pixels high (TextHeight) with the baseline 11 pixels down, as GDI draws
it.

Readable first: every letter is Liberation's own shape, all of them made
SIZE large in both directions (never narrowed, never one letter smaller
than the next), with at least GAP between two letters. A bitmap font's
letters are a pixel apart; anti-aliased letters closer than about that run
together ("Br", "pr", "ar" read as one shape). Where MS Sans Serif's width
can't hold the letter and that space — r, x, y, j, C, the brackets — the
character is a pixel wider than MS Sans Serif's ("program" 40 pixels
against RapidQ's 38); the rest keep their widths exactly. Within its width a
letter keeps Liberation's balance of space left and right, its upright
stems on whole pixels at 8 pt where that leaves space on both sides;
descenders are kept within the line.

The widths are measurements, not Microsoft's data: RapidQ's own TextWidth of
each character in its default font, run by RC.EXE on Windows 11
(tests/visual/README.md, docs/rapidq-ground-truth.md). Other characters keep
Liberation's outlines and widths, made SIZE large too. The hinting instructions are dropped (they
were made for Liberation's outlines); the renderer hints automatically.

As a modified version under the OFL it is renamed (Liberation is a Reserved
Font Name) and stays under the OFL; crates/rapidr-value/fonts/README.md says
so.

    python3 tools/fonts/make_rapidr_sans.py

writes crates/rapidr-value/fonts/RapidRSans-Regular.ttf (needs fontTools).
"""
import math
import os

from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.filterPen import FilterPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.scaleUpem import scale_upem
from fontTools.ttLib.tables import ttProgram

HERE = os.path.dirname(os.path.abspath(__file__))
FONTS = os.path.join(HERE, "..", "..", "crates", "rapidr-value", "fonts")
SOURCE = os.path.join(FONTS, "LiberationSans-Regular.ttf")
TARGET = os.path.join(FONTS, "RapidRSans-Regular.ttf")

# MS Sans Serif 8 pt at 96 dpi (em 11 pixels): RapidQ's TextWidth(CHR$(c))
# for c = 32 … 255 (0: no character there in Windows-1252).
WIDTHS = [int(w) for w in """
3 3 5 7 6 8 6 2 3 3 4 6 3 3 3 5 6 6 6 6 6 6 6 6 6 6 3 3 6 6 6 6
11 7 7 7 8 7 6 8 8 3 5 7 6 9 8 8 7 8 8 7 7 8 7 11 7 7 7 3 5 3 6 6
3 6 6 6 6 6 3 6 6 2 2 6 2 8 6 6 6 6 3 5 3 6 6 8 5 5 5 4 2 4 7 3
6 0 3 3 3 3 3 3 3 3 3 3 3 0 3 0 0 3 3 3 3 3 3 3 3 3 3 3 3 0 3 3
3 3 6 6 6 6 2 6 3 9 4 6 6 3 8 6 4 6 3 3 3 6 6 3 3 3 4 6 8 8 8 6
7 7 7 7 7 7 10 7 7 7 7 7 3 3 3 3 8 8 8 8 8 8 8 6 8 8 8 8 8 7 7 6
6 6 6 6 6 6 10 6 6 6 6 6 2 4 4 4 6 6 6 6 6 6 6 6 6 6 6 6 6 5 6 5
""".split()]
assert len(WIDTHS) == 224
EM_PX = 11
ASCENT_PX, DESCENT_PX = 11, 2
# The letters' size: Liberation's outlines, all of them scaled by this one
# factor, the same in both directions — every letter the same size as the
# others and its own shape (never narrowed, never one letter smaller than
# the next). An x-height of 5.5 pixels at 8 pt, which the renderer's
# hinting makes 6 at 1× (MS Sans Serif's), 8 at 1.5×, 11 at 2×.
SIZE = 0.95
# Between two letters, at least this much space (pixels at 8 pt): with
# less, anti-aliased letters run together — MS Sans Serif's bitmap letters
# have a blank pixel column between them. A letter Liberation sets closer
# than that (the pointed A, V, x, y) keeps its own spacing plus EXTRA.
GAP = 0.8
EXTRA = 0.3
# Where MS Sans Serif's width (RapidQ's TextWidth) leaves less than that
# by more than TOLERANCE, the character is a pixel wider (r, x, y, j, C, the
# brackets: text a pixel or two wider than in RapidQ). Less than that is
# left: a t's or an f's cross-bar, a pointed A or V, an s, as close to the
# next letter as Liberation sets them.
TOLERANCE = 0.38
# Space kept on each side of a letter when its stems are put on whole
# pixels, at least (or 40 % of what there is).
SIDE = 0.25
# Descenders kept within the line's 2 pixels below the baseline
# (Liberation's g, p, y reach 0.212 em down; MS Sans Serif's line 2 of 11
# pixels), so nothing is cut off at the bottom of a 13-pixel line: only
# what lies below the baseline is shortened.
SHORT = min(SIZE, (DESCENT_PX / EM_PX) / 0.212)


class Fit(FilterPen):
    """Scaled by SIZE (what's below the baseline by SHORT), then moved dx."""

    def __init__(self, out, dx=0.0):
        super().__init__(out)
        self.dx = dx

    def _p(self, pt):
        x, y = pt
        return (x * SIZE + self.dx, y * (SIZE if y > 0 else SHORT))

    def moveTo(self, pt):
        self._outPen.moveTo(self._p(pt))

    def lineTo(self, pt):
        self._outPen.lineTo(self._p(pt))

    def curveTo(self, *pts):
        self._outPen.curveTo(*[self._p(p) for p in pts])

    def qCurveTo(self, *pts):
        self._outPen.qCurveTo(*[None if p is None else self._p(p) for p in pts])

# (cp1252 0x80-0x9F: what Windows shows there, MS Sans Serif's 3-pixel
# boxes for most — those keep Liberation's glyphs and widths)
KEEP_LIBERATION = {c for c in range(0x80, 0xA0)}


def stem_shift(g, glyf, px, lo, hi):
    """How far to move a glyph sideways, between lo and hi (font units), so
    that its upright edges — the sides of l, i, n, H … — fall on pixel
    boundaries at 8 pt (one pixel `px` units), where a 1-pixel stem then
    shows as one black column instead of two grey ones. (The range keeps
    some space on both sides of the letter: a letter moved flush against
    its cell's edge touches the next one — B then r.)"""
    if lo > hi:
        return round((lo + hi) / 2)
    edges = []
    if g.numberOfContours:
        coords, ends, flags = g.getCoordinates(glyf)
        start = 0
        for end in ends:
            pts = [(coords[i], flags[i] & 1) for i in range(start, end + 1)]
            for (a, on_a), (b, on_b) in zip(pts, pts[1:] + pts[:1]):
                if on_a and on_b and abs(a[0] - b[0]) <= 2 and abs(a[1] - b[1]) >= px * 3 // 4:
                    edges.append(((a[0] + b[0]) / 2, abs(a[1] - b[1])))
            start = end + 1
    if not edges:
        return min(max(0, round(lo)), round(hi))

    def cost(s):
        return sum(w * min((x + s) % px, px - (x + s) % px) for x, w in edges)

    span = range(int(math.ceil(lo)), int(math.floor(hi)) + 1)
    return min(span, key=lambda s: (round(cost(s)), abs(s))) if span else round((lo + hi) / 2)


def main():
    font = TTFont(SOURCE)
    # (an em of 2200 units: 200 a pixel at 11 pixels, so every width and
    # the line metrics are whole units — a line exactly 13 pixels high)
    scale_upem(font, EM_PX * 200)
    upm = font["head"].unitsPerEm
    glyf = font["glyf"]
    hmtx = font["hmtx"]
    cmap = font.getBestCmap()
    # (Liberation's outlines, from a copy of their own: a glyph set reads the
    # glyf table as it is, and the one being made changes as it goes)
    source = TTFont(SOURCE)
    scale_upem(source, upm)
    original = source.getGlyphSet()

    def outline(name, dx=0.0):
        rec = DecomposingRecordingPen(original)
        original[name].draw(rec)
        pen = TTGlyphPen(None)
        rec.replay(Fit(pen, dx))
        return pen.glyph()

    # Every glyph SIZE large, its width too (composites decomposed from the
    # original outlines: a glyph outside the table keeps Liberation's shape
    # whatever its parts become).
    for name in font.getGlyphOrder():
        if glyf[name].isComposite() or glyf[name].numberOfContours > 0:
            glyf[name] = outline(name)
            glyf[name].recalcBounds(glyf)
        advance, lsb = hmtx[name]
        hmtx[name] = (round(advance * SIZE), getattr(glyf[name], "xMin", 0) if glyf[name].numberOfContours else 0)

    px = upm / EM_PX
    report = []
    done = set()
    for i, width in enumerate(WIDTHS):
        code = 32 + i
        if width == 0 or code in KEEP_LIBERATION:
            continue
        ch = bytes([code]).decode("cp1252", errors="ignore")
        name = cmap.get(ord(ch)) if ch else None
        if not name or name in done:
            continue
        done.add(name)
        # (Liberation's width, ink and side bearings, made SIZE large)
        advance = source["hmtx"][name][0] * SIZE
        g = glyf[name]
        if advance <= 0 or not g.numberOfContours:
            hmtx[name] = (round(width * px), 0)
            continue
        ink = g.xMax - g.xMin
        lsb, rsb = g.xMin, advance - g.xMax
        # (the space it needs beside it: GAP, or Liberation's own plus EXTRA)
        gap = min(GAP * px, lsb + rsb + EXTRA * px)
        cells = max(width, math.ceil((ink + gap) / px - TOLERANCE))
        if cells != width:
            report.append(f"{ch} {width}->{cells}")
        target = cells * px
        # (the side bearings share what's left as they did)
        slack = target - ink
        share = lsb / (lsb + rsb) if lsb + rsb > 0 else 0.5
        dx = slack * share - g.xMin
        # (its upright stems on whole pixels at 8 pt: crisp as a bitmap
        # font's; at least SIDE on either side of it, or 40 % of what there is)
        side = min(SIDE * px, max(slack, 0) * 0.4) if slack > 0 else slack / 2
        lsb0 = slack * share
        moved = outline(name, dx)
        s = stem_shift(moved, glyf, upm // EM_PX, side - lsb0, slack - side - lsb0)
        g = outline(name, dx + s) if s else moved
        glyf[name] = g
        g.recalcBounds(glyf)
        hmtx[name] = (round(target), g.xMin)
    print("wider than MS Sans Serif:", " ".join(report) or "none")

    # (the hinting instructions were made for Liberation's outlines; the
    # renderer hints vertically on its own)
    for tag in ("fpgm", "prep", "cvt ", "hdmx", "LTSH", "VDMX", "kern"):
        if tag in font:
            del font[tag]
    for name in font.getGlyphOrder():
        g = glyf[name]
        if g.numberOfContours:
            g.program = ttProgram.Program()
            g.program.fromBytecode(b"")
    font["gasp"].gaspRange = {0xFFFF: 0x000F}

    ascent = round(ASCENT_PX * upm / EM_PX)
    descent = round(DESCENT_PX * upm / EM_PX)
    hhea, os2 = font["hhea"], font["OS/2"]
    hhea.ascent, hhea.descent, hhea.lineGap = ascent, -descent, 0
    os2.sTypoAscender, os2.sTypoDescender, os2.sTypoLineGap = ascent, -descent, 0
    os2.usWinAscent, os2.usWinDescent = ascent, descent
    os2.version = max(os2.version, 4)
    os2.fsSelection |= 1 << 7  # USE_TYPO_METRICS
    os2.xAvgCharWidth = round(sum(hmtx[n][0] for n in done) / max(1, len(done)))

    names = {
        0: "Digitized data copyright (c) 2010 Google Corporation. Copyright (c) 2012 Red Hat, Inc. "
        "Modified for RapidR (2026): character widths and vertical metrics.",
        1: "RapidR Sans",
        2: "Regular",
        3: "RapidR Sans Regular (from Liberation Sans 2.1.5)",
        4: "RapidR Sans",
        5: "Version 1.0 (Liberation Sans 2.1.5)",
        6: "RapidRSans-Regular",
        10: "Liberation Sans (SIL OFL 1.1) with MS Sans Serif 8 pt's character widths and line metrics, "
        "so forms laid out for RapidQ's default font fit.",
        13: "Licensed under the SIL Open Font License, Version 1.1",
        14: "https://openfontlicense.org",
    }
    table = font["name"]
    table.names = [r for r in table.names if r.nameID not in names and r.nameID not in (7, 16, 17, 18, 19)]
    for nid, text in names.items():
        table.setName(text, nid, 3, 1, 0x409)
        table.setName(text, nid, 1, 0, 0)
    font["head"].fontRevision = 1.0
    # (reproducible: the source's dates)
    font.recalcTimestamp = False
    font.save(TARGET)
    print(f"wrote {TARGET}: {len(done)} characters fitted, {os.path.getsize(TARGET)} bytes")


if __name__ == "__main__":
    main()
