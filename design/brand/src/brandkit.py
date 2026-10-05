"""Small vector toolkit for the RapidR brand masters.

Everything here is plain Python (stdlib + fontTools for outlining the OFL
Inter font). It parses the simple absolute SVG paths the masters are written
in, applies affine transforms (so skews and scales are baked into the
geometry instead of living in transform attributes), builds the continuous-
corner "squircle" tile, and turns text into outlines so no SVG needs a font.
"""
from __future__ import annotations

import math
import os
import re
from functools import lru_cache

HERE = os.path.dirname(os.path.abspath(__file__))
BRAND = os.path.dirname(HERE)

# ---------------------------------------------------------------- palette --
INK = "#0E1525"        # near-black navy: text, mono mark, dark tiles
PAPER = "#F6F8FC"      # off-white backgrounds
BLUE = "#2F5BFF"       # RapidR Blue: primary
BLUE_DEEP = "#1E3FD8"  # pressed / text-on-light blue (AA on white)
CYAN = "#19C6E6"       # Spark Cyan: gradient end, accents on dark
AMBER = "#FFB224"      # Run Amber: the Runtime / compiled-program accent
TEAL = "#12B48A"       # BASIC Teal: the generic .bas source accent
TEAL_DEEP = "#0B7B5E"  # BASIC Teal for text on light (AA)
SLATE = "#5B6478"      # secondary text on light
MIST = "#C9D1E3"       # hairlines, secondary text on dark
WHITE = "#FFFFFF"

# ------------------------------------------------------------- path utils --
_TOK = re.compile(r"[MLHVCQAZmlhvcqaz]|-?\d*\.?\d+(?:e-?\d+)?")


def _num(v: float) -> str:
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def parse(d: str):
    """Absolute M/L/H/V/C/Q/A/Z path -> list of (cmd, points) with only M/L/C/Z.
    Arcs (circular, no rotation) become cubic Beziers."""
    toks = _TOK.findall(d)
    i = 0
    out = []
    cx = cy = sx = sy = 0.0
    cmd = None

    def nums(n):
        nonlocal i
        vals = [float(t) for t in toks[i:i + n]]
        i += n
        return vals

    while i < len(toks):
        t = toks[i]
        if t.isalpha():
            cmd = t
            i += 1
            if cmd in "Zz":
                out.append(("Z", []))
                cx, cy = sx, sy
                continue
        if cmd == "M":
            cx, cy = nums(2)
            sx, sy = cx, cy
            out.append(("M", [(cx, cy)]))
            cmd = "L"
        elif cmd == "L":
            cx, cy = nums(2)
            out.append(("L", [(cx, cy)]))
        elif cmd == "H":
            (cx,) = nums(1)
            out.append(("L", [(cx, cy)]))
        elif cmd == "V":
            (cy,) = nums(1)
            out.append(("L", [(cx, cy)]))
        elif cmd == "C":
            a = nums(6)
            out.append(("C", [(a[0], a[1]), (a[2], a[3]), (a[4], a[5])]))
            cx, cy = a[4], a[5]
        elif cmd == "Q":
            qx, qy, x, y = nums(4)
            out.append(("C", [(cx + 2 / 3 * (qx - cx), cy + 2 / 3 * (qy - cy)),
                              (x + 2 / 3 * (qx - x), y + 2 / 3 * (qy - y)), (x, y)]))
            cx, cy = x, y
        elif cmd == "A":
            rx, ry, _rot, large, sweep, x, y = nums(7)
            for seg in _arc_to_cubic(cx, cy, rx, large, sweep, x, y):
                out.append(("C", seg))
            cx, cy = x, y
        else:
            raise ValueError(f"unsupported path command {cmd!r} in {d!r}")
    return out


def _arc_to_cubic(x1, y1, r, large, sweep, x2, y2):
    dx, dy = (x2 - x1) / 2, (y2 - y1) / 2
    d2 = dx * dx + dy * dy
    r = max(r, math.sqrt(d2))
    h = math.sqrt(max(r * r - d2, 0))
    mx, my = (x1 + x2) / 2, (y1 + y2) / 2
    ln = math.sqrt(d2) or 1
    ux, uy = -dy / ln, dx / ln
    sign = 1 if large != sweep else -1
    ccx, ccy = mx + sign * h * ux, my + sign * h * uy
    a1 = math.atan2(y1 - ccy, x1 - ccx)
    a2 = math.atan2(y2 - ccy, x2 - ccx)
    da = a2 - a1
    if sweep and da < 0:
        da += 2 * math.pi
    if not sweep and da > 0:
        da -= 2 * math.pi
    n = max(1, int(math.ceil(abs(da) / (math.pi / 2) - 1e-9)))
    step = da / n
    k = 4 / 3 * math.tan(step / 4)
    segs = []
    a = a1
    for _ in range(n):
        b = a + step
        p0 = (ccx + r * math.cos(a), ccy + r * math.sin(a))
        p3 = (ccx + r * math.cos(b), ccy + r * math.sin(b))
        c1 = (p0[0] - k * r * math.sin(a), p0[1] + k * r * math.cos(a))
        c2 = (p3[0] + k * r * math.sin(b), p3[1] - k * r * math.cos(b))
        segs.append([c1, c2, p3])
        a = b
    return segs


def transform(d: str, m) -> str:
    """Apply affine m=(a,b,c,d,e,f) to path d."""
    a, b, c, dd, e, f = m
    out = []
    for cmd, pts in parse(d):
        tp = [(a * x + c * y + e, b * x + dd * y + f) for x, y in pts]
        out.append(cmd + " ".join(f"{_num(x)} {_num(y)}" for x, y in tp))
    return " ".join(out)


def scale_translate(d, s, tx=0.0, ty=0.0):
    return transform(d, (s, 0, 0, s, tx, ty))


def skew_x(d, deg, cy):
    """Skew horizontally around the horizontal line y=cy (positive = lean right)."""
    k = math.tan(math.radians(deg))
    return transform(d, (1, 0, -k, 1, k * cy, 0))


def squircle(x, y, w, h, ext, n=48):
    """Continuous-curvature rounded rectangle (Apple-style "squircle").

    Each corner is a superellipse quadrant reaching `ext` along both edges;
    sampled as a polyline that is smooth at any icon size."""
    ext = min(ext, w / 2, h / 2)
    p = 4.6  # superellipse exponent, close to the macOS Big Sur template
    pts = []
    corners = [
        (x + w - ext, y + ext, 0),        # top-right: angle -90..0
        (x + w - ext, y + h - ext, 1),    # bottom-right
        (x + ext, y + h - ext, 2),        # bottom-left
        (x + ext, y + ext, 3),            # top-left
    ]
    for cx, cy, q in corners:
        for i in range(n + 1):
            t = (q - 1) * math.pi / 2 + (i / n) * math.pi / 2
            ct, st = math.cos(t), math.sin(t)
            px = cx + ext * math.copysign(abs(ct) ** (2 / p), ct)
            py = cy + ext * math.copysign(abs(st) ** (2 / p), st)
            pts.append((px, py))
    return "M" + " L".join(f"{_num(px)} {_num(py)}" for px, py in pts) + " Z"


def rrect(x, y, w, h, r):
    return (f"M{_num(x + r)} {_num(y)} H{_num(x + w - r)} A{_num(r)} {_num(r)} 0 0 1 {_num(x + w)} {_num(y + r)} "
            f"V{_num(y + h - r)} A{_num(r)} {_num(r)} 0 0 1 {_num(x + w - r)} {_num(y + h)} H{_num(x + r)} "
            f"A{_num(r)} {_num(r)} 0 0 1 {_num(x)} {_num(y + h - r)} V{_num(y + r)} A{_num(r)} {_num(r)} 0 0 1 {_num(x + r)} {_num(y)} Z")


# ------------------------------------------------------------- outlining --
INTER = os.environ.get("RAPIDR_BRAND_FONT", os.path.join(HERE, "fonts", "Inter-V.ttf"))
# Liberation Mono (OFL-1.1) ships in the repo for the runtime already.
MONO = os.path.join(os.path.dirname(os.path.dirname(BRAND)), "crates", "rapidr-value", "fonts",
                    "LiberationMono-Regular.ttf")


@lru_cache(maxsize=None)
def _font(weight: int, path: str = INTER):
    from fontTools.ttLib import TTFont
    f = TTFont(path)
    if "fvar" not in f:
        return f
    from fontTools.varLib import instancer
    return instancer.instantiateVariableFont(f, {"wght": weight, "slnt": 0})


def text_path(text, size, x=0.0, y=0.0, weight=700, tracking=0.0, kern=True, font=INTER):
    """Outline `text` with Inter at `weight`. (x, y) is the left baseline.
    tracking is in em/1000. Returns (path d, advance width)."""
    from fontTools.pens.svgPathPen import SVGPathPen
    from fontTools.pens.transformPen import TransformPen
    f = _font(weight, font)
    upm = f["head"].unitsPerEm
    cmap = f.getBestCmap()
    gs = f.getGlyphSet()
    hmtx = f["hmtx"]
    s = size / upm
    pen_x = 0.0
    parts = []
    names = [cmap[ord(ch)] for ch in text]
    kerns = _kerning(weight, font) if kern else {}
    for i, gn in enumerate(names):
        sp = SVGPathPen(gs)
        tp = TransformPen(sp, (s, 0, 0, -s, x + pen_x * s, y))
        gs[gn].draw(tp)
        parts.append(sp.getCommands())
        adv = hmtx[gn][0] + tracking * upm / 1000
        if i + 1 < len(names):
            adv += kerns.get((gn, names[i + 1]), 0)
        pen_x += adv
    d = " ".join(p for p in parts if p)
    return transform(d, (1, 0, 0, 1, 0, 0)), (pen_x - tracking * upm / 1000) * s


@lru_cache(maxsize=None)
def _kerning(weight, font=INTER):
    """Pair kerning from GPOS PairPos (formats 1 and 2)."""
    f = _font(weight, font)
    cache = {}
    if "GPOS" not in f:
        return cache
    gpos = f["GPOS"].table
    for lookup in gpos.LookupList.Lookup:
        subs = lookup.SubTable
        for st in subs:
            if lookup.LookupType == 9:
                st = st.ExtSubTable
            if getattr(st, "LookupType", lookup.LookupType) != 2 and lookup.LookupType not in (2, 9):
                continue
            if not hasattr(st, "Format"):
                continue
            if st.Format == 1 and hasattr(st, "PairSet"):
                cov = st.Coverage.glyphs
                for g1, ps in zip(cov, st.PairSet):
                    for pvr in ps.PairValueRecord:
                        v = getattr(pvr.Value1, "XAdvance", 0) if pvr.Value1 else 0
                        if v:
                            cache.setdefault((g1, pvr.SecondGlyph), v)
            elif st.Format == 2 and hasattr(st, "Class1Record"):
                cov = st.Coverage.glyphs
                c1 = st.ClassDef1.classDefs
                c2 = st.ClassDef2.classDefs
                second_by_class = {}
                for g, c in c2.items():
                    second_by_class.setdefault(c, []).append(g)
                for g1 in cov:
                    k1 = c1.get(g1, 0)
                    rec = st.Class1Record[k1]
                    for k2, r2 in enumerate(rec.Class2Record):
                        v = getattr(r2.Value1, "XAdvance", 0) if r2.Value1 else 0
                        if not v:
                            continue
                        for g2 in second_by_class.get(k2, []):
                            cache.setdefault((g1, g2), v)
    return cache


def svg_doc(body, w, h, defs="", title=None, px_w=None, px_h=None):
    t = f"<title>{title}</title>" if title else ""
    size = f' width="{px_w}" height="{px_h}"' if px_w else ""
    d = f"<defs>{defs}</defs>" if defs else ""
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {_num(w)} {_num(h)}"{size}>'
            f"{t}{d}{body}</svg>\n")
