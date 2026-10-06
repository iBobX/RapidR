#!/usr/bin/env python3
"""RapidR Sans: the face RapidR draws "MS Sans Serif" with — RapidQ's
default font (every component's, 8 pt) — so a form laid out for RapidQ fits
in RapidR as it did there.

MS Sans Serif is a Windows bitmap font; no open font has its widths
(Liberation Sans has Arial's, a few percent wider: "Password:" is 52 pixels
at 8 pt against MS Sans Serif's 49, and a label sized for one clips the
other). RapidR Sans is Liberation Sans (SIL Open Font License 1.1, its
outlines and character set) with each Windows-1252 character made exactly as
wide as MS Sans Serif's at 8 pt on a 96-dpi screen, where its em is 11
pixels — its letter keeping its shape (6 % larger than Liberation's, both
ways: an x-height of 6 pixels at 8 pt), narrowed or widened at most 4 %,
the rest from its side bearings (shared as they were), and where the ink
still doesn't fit, the letter made a little smaller in both directions;
descenders kept within the line — its upright stems moved onto whole
pixels at 8 pt, and MS Sans Serif's vertical metrics:
ascent 11 pixels, descent 2, so a line is 13 pixels high (TextHeight) with
the baseline 11 pixels down, as GDI draws it.

The widths are measurements, not Microsoft's data: RapidQ's own TextWidth of
each character in its default font, run by RC.EXE on Windows 11
(tests/visual/README.md, docs/rapidq-ground-truth.md). Other characters keep
Liberation's outlines and widths. The hinting instructions are dropped (they
were made for Liberation's outlines); the renderer hints automatically.

As a modified version under the OFL it is renamed (Liberation is a Reserved
Font Name) and stays under the OFL; crates/rapidr-value/fonts/README.md says
so.

    python3 tools/fonts/make_rapidr_sans.py

writes crates/rapidr-value/fonts/RapidRSans-Regular.ttf (needs fontTools).
"""
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
# The letters keep their shapes: a glyph's outline is narrowed or widened
# at most this much to meet its width (barely visible) …
SQUEEZE = (0.96, 1.04)
# … the rest comes from its side bearings, and a glyph whose ink still
# doesn't fit is made smaller in both directions, at most this much.
SHRINK = 0.88
# Descenders kept within the line's 2 pixels below the baseline
# (Liberation's g, p, y reach 0.212 em down; MS Sans Serif's line 2 of 11
# pixels), so nothing is cut off at the bottom of a 13-pixel line: only
# what lies below the baseline is shortened.
SHORT = (DESCENT_PX / EM_PX) / 0.212
# Every letter a little larger than Liberation's, the same in both
# directions: an x-height of 6 pixels at 8 pt (5.7 in Liberation), as MS Sans
# Serif's and Microsoft Sans Serif's — text as large as RapidQ's.
BIG = 1.06


class Fit(FilterPen):
    """x scaled by kx, both directions by u (what's below the baseline by
    SHORT too), then moved dx."""

    def __init__(self, out, kx=1.0, u=1.0, dx=0.0):
        super().__init__(out)
        self.kx, self.u, self.dx = kx, u, dx

    def _p(self, pt):
        x, y = pt
        u = self.u * BIG
        return (x * self.kx * u + self.dx, y * u * (1.0 if y > 0 else SHORT / BIG))

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


def stem_shift(g, glyf, px):
    """How far to move a glyph sideways (within half a pixel) so that its
    upright edges — the sides of l, i, n, H … — fall on pixel boundaries
    at 8 pt (one pixel `px` units), where a 1-pixel stem then shows as one
    black column instead of two grey ones."""
    if not g.numberOfContours:
        return 0
    coords, ends, flags = g.getCoordinates(glyf)
    edges = []
    start = 0
    for end in ends:
        pts = [(coords[i], flags[i] & 1) for i in range(start, end + 1)]
        for (a, on_a), (b, on_b) in zip(pts, pts[1:] + pts[:1]):
            if on_a and on_b and abs(a[0] - b[0]) <= 2 and abs(a[1] - b[1]) >= px * 3 // 4:
                edges.append(((a[0] + b[0]) / 2, abs(a[1] - b[1])))
        start = end + 1
    if not edges:
        return 0

    def cost(s):
        return sum(w * min((x + s) % px, px - (x + s) % px) for x, w in edges)

    best = min(range(-px // 2, px // 2 + 1, 2), key=lambda s: (round(cost(s)), abs(s)))
    return best if cost(best) < cost(0) else 0


def main():
    font = TTFont(SOURCE)
    # (an em of 2200 units: 200 a pixel at 11 pixels, so every width and
    # the line metrics are whole units — a line exactly 13 pixels high)
    scale_upem(font, EM_PX * 200)
    upm = font["head"].unitsPerEm
    glyf = font["glyf"]
    hmtx = font["hmtx"]
    cmap = font.getBestCmap()
    original = font.getGlyphSet()

    def outline(name, kx=1.0, u=1.0, dx=0.0):
        rec = DecomposingRecordingPen(original)
        original[name].draw(rec)
        pen = TTGlyphPen(None)
        rec.replay(Fit(pen, kx, u, dx))
        return pen.glyph()

    # Every glyph's descender kept in the line (composites decomposed from
    # the original outlines: a glyph outside the table keeps Liberation's
    # shape whatever its parts become).
    for name in font.getGlyphOrder():
        if glyf[name].isComposite() or glyf[name].numberOfContours > 0:
            glyf[name] = outline(name)

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
        advance, _ = hmtx[name]
        target = round(width * upm / EM_PX)
        if advance <= 0:
            hmtx[name] = (target, 0)
            continue
        base = glyf[name]
        base.recalcBounds(glyf)
        if not base.numberOfContours:
            hmtx[name] = (target, 0)
            continue
        # (its ink, and the space either side of it)
        ink = base.xMax - base.xMin
        lsb, rsb = base.xMin, advance - base.xMax
        kx = min(max(target / (advance * BIG), SQUEEZE[0]), SQUEEZE[1])
        u = 1.0 if ink * kx <= target else max(SHRINK, target / (ink * kx))
        # (the side bearings share what's left as they did)
        slack = target - ink * kx * u
        share = lsb / (lsb + rsb) if lsb + rsb > 0 else 0.5
        dx = slack * share - base.xMin * kx * u
        g = outline(name, kx, u, dx)
        # (its upright stems on whole pixels at 8 pt: crisp as a bitmap font's)
        s = stem_shift(g, glyf, upm // EM_PX)
        if s:
            g = outline(name, kx, u, dx + s)
        glyf[name] = g
        g.recalcBounds(glyf)
        hmtx[name] = (target, getattr(g, "xMin", 0) if g.numberOfContours else 0)

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
