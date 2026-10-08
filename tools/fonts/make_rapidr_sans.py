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

    python3 tools/fonts/make_rapidr_sans.py [<liberation-fonts-ttf-2.1.5 folder>]

writes crates/rapidr-value/fonts/RapidRSans-Regular.ttf (needs fontTools). Given
the folder of the Liberation fonts 2.1.5 release (the official
liberation-fonts-ttf-2.1.5.tar.gz, unpacked; fonts/README.md has its
SHA-256), it makes RapidRSans-Bold.ttf too: the same, from Liberation Sans
Bold, with each character one pixel wider than the regular's — what RC.EXE
measures for MS Sans Serif Bold at 8 pt (TextWidth of every character in
Windows-1252: regular + 1) — so a bold caption is as wide as RapidQ's, in a
real bold face instead of the regular one drawn heavier. Liberation Sans Bold's
stems are about 1.6 pixels at 8 pt where MS Sans Serif Bold's are 2, so each
letter is also drawn again EMBOLD (0.35) of a pixel to the right, the two
contours side by side (one shape under the nonzero fill rule). The bold face is
cut to the Latin scripts (the regular face has the rest, which a bold
caption then draws heavier, as before).
"""
import math
import os
import sys

from fontTools import subset
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
TARGET_BOLD = os.path.join(FONTS, "RapidRSans-Bold.ttf")
# The Latin scripts (what tools/fonts/make_liberation_styles.py keeps too):
# Basic Latin, Latin-1, Latin Extended-A, general punctuation, currency,
# letterlike, arrows, minus, geometric shapes and box drawing.
LATIN = "U+0020-007E,U+00A0-017F,U+0192,U+0218-021B,U+02C6-02DD,U+2000-206F,U+20A0-20CF,U+2100-215F,U+2190-21FF,U+2212,U+2215,U+221E,U+2248,U+2260-2265,U+25A0-25FF,U+2500-257F,U+FFFD"

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
# MS Sans Serif Bold 8 pt: RC.EXE's TextWidth of every character is the
# regular's plus one pixel (measured for 32 … 255, tests/visual/README.md).
BOLD_WIDTHS = [w + 1 if w else 0 for w in WIDTHS]
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
# The bold's extra weight, in pixels at 8 pt (Liberation Sans Bold's stems are
# about 1.6 pixels there; MS Sans Serif Bold's are 2).
EMBOLD = 0.35


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


def main(src=SOURCE, out=TARGET, widths=WIDTHS, bold=False):
    font = TTFont(src)
    # (an em of 2200 units: 200 a pixel at 11 pixels, so every width and
    # the line metrics are whole units — a line exactly 13 pixels high)
    scale_upem(font, EM_PX * 200)
    upm = font["head"].unitsPerEm
    glyf = font["glyf"]
    hmtx = font["hmtx"]
    cmap = font.getBestCmap()
    # (Liberation's outlines, from a copy of their own: a glyph set reads the
    # glyf table as it is, and the one being made changes as it goes)
    source = TTFont(src)
    scale_upem(source, upm)
    original = source.getGlyphSet()

    # (the bold: each letter drawn again EMBOLD of a pixel to the right, the
    # contours side by side — one shape under the nonzero rule — so a stem
    # is as heavy as MS Sans Serif Bold's two pixels, which Liberation Sans
    # Bold's 1.6 are not at this size)
    heavier = EMBOLD * upm / EM_PX if bold else 0.0

    def outline(name, dx=0.0):
        rec = DecomposingRecordingPen(original)
        original[name].draw(rec)
        pen = TTGlyphPen(None)
        rec.replay(Fit(pen, dx))
        if heavier:
            rec.replay(Fit(pen, dx + heavier))
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
    for i, width in enumerate(widths):
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

    style = "Bold" if bold else "Regular"
    names = {
        0: "Digitized data copyright (c) 2010 Google Corporation. Copyright (c) 2012 Red Hat, Inc. "
        "Modified for RapidR (2026): character widths and vertical metrics.",
        1: "RapidR Sans",
        2: style,
        3: f"RapidR Sans {style} (from Liberation Sans 2.1.5)",
        4: "RapidR Sans" if not bold else "RapidR Sans Bold",
        5: "Version 1.0 (Liberation Sans 2.1.5)",
        6: f"RapidRSans-{style}",
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
    font.save(out)
    if bold:
        # (the Latin scripts only; the regular face has the rest)
        subset.main([out, f"--unicodes={LATIN}", "--layout-features=kern,locl,mark,mkmk,ccmp,case", "--no-hinting",
                     "--name-IDs=*", "--name-languages=*", "--notdef-outline", f"--output-file={out}"])
    print(f"wrote {out}: {len(done)} characters fitted, {os.path.getsize(out)} bytes")


if __name__ == "__main__":
    main()
    if len(sys.argv) > 1:
        main(os.path.join(sys.argv[1], "LiberationSans-Bold.ttf"), TARGET_BOLD, BOLD_WIDTHS, bold=True)
