"""RapidR's icon kit: icons drawn once on the 24 px master grid, hinted for
16, 24 and 32 px.

An icon is a Python function that draws with the primitives of `G` in
master coordinates (a 24 x 24 canvas, live area 2..22). The function runs
once per size; `G` maps the master live area onto that size's live area
and snaps every coordinate so that strokes and fills land on the pixel
grid (design/icons/README.md, "Hinting"):

    size  live area   scale   stroke  stroke centres on
    16    1..15        0.7     1       n + 0.5   (crisp)
    24    2..22        1.0     1.5     n + 0.75  (one edge crisp; crisp at 2x)
    32    2..30        1.4     2       n         (crisp)

Fill-only shapes snap their edges to whole pixels. A function may branch on
`g.size` to simplify a small size or add detail to a large one.

Colours are tokens (`TOKENS`), written into the SVG as their light-theme
values, which the renderers swap for the theme's (crates/rapidr-icons).
`fg` is `currentColor`: monochrome icons are drawn in it only.

Standard library only.
"""

import math
import re

SIZES = (16, 24, 32)
# size: (offset of the live area, scale from the master, stroke width)
SPEC = {16: (1.0, 0.7, 1.0), 24: (2.0, 1.0, 1.5), 32: (2.0, 1.4, 2.0)}

# The colour tokens and their light-theme values (the keys the renderers
# replace). The other themes' values are in crates/rapidr-icons/src/palette.rs.
TOKENS = {
    "ink": "#5B6478",         # neutral outlines (the brand's Slate)
    "paper": "#FFFFFF",       # neutral fills, surfaces
    "shade": "#E6EAF2",       # a neutral tint (headers, title bars)
    "blue": "#2F5BFF", "blue-tint": "#E3E9FF",
    "cyan": "#007F9C", "cyan-tint": "#D7F3F9",
    "teal": "#08805E", "teal-tint": "#D6F2E8",
    "amber": "#B86E00", "amber-tint": "#FFEDC7",
    "amber-solid": "#FFB224",  # Run Amber, a fill that needs no outline contrast
    "red": "#D92D36", "red-tint": "#FCE1E2",
    "violet": "#6F42E5", "violet-tint": "#EBE4FD",
    "spark": "#19C6E6",       # the brand gradient's end (with blue)
}


def num(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def color(c):
    if c is None:
        return "none"
    if c == "fg":
        return "currentColor"
    if c.startswith("url(") or c in ("white", "black"):
        return c
    return TOKENS[c]


PATH_TOKEN = re.compile(r"[MLHVACQZmlhvacqz]|-?\d*\.?\d+(?:e-?\d+)?")


class G:
    """One icon at one size."""

    def __init__(self, size, mono):
        self.size = size
        self.mono = mono
        self.off, self.k, self.w = SPEC[size]
        self.phase = (self.w / 2) % 1
        self.items = []      # SVG elements (strings) in drawing order
        self.defs = []
        self.ids = 0
        self.xf = (0.0, 0.0, 1.0)   # sub-transform in master space: v -> d + v * s

    # ---- coordinates ----------------------------------------------------
    def m(self, v, axis=0):
        """A master coordinate on the target canvas, unsnapped."""
        dx, dy, s = self.xf
        v = (dx if axis == 0 else dy) + v * s
        return self.off + (v - 2) * self.k

    def snap(self, t, phase):
        """`t` on the grid n + phase; a tie goes away from the centre (so
        mirror-symmetric designs stay symmetric)."""
        n = t - phase
        f = math.floor(n)
        fr = n - f
        if abs(fr - 0.5) < 1e-6:
            r = f if t < self.size / 2 else f + 1
        else:
            r = math.floor(n + 0.5)
        return r + phase

    def X(self, v, fill=False):
        return self.snap(self.m(v, 0), 0 if fill else self.phase)

    def Y(self, v, fill=False):
        return self.snap(self.m(v, 1), 0 if fill else self.phase)

    def L(self, v):
        """A length (radius), scaled, in half pixels."""
        return max(0.0, round(v * self.xf[2] * self.k * 2) / 2)

    def R(self, v):
        """A circle's radius, in whole pixels (its edges stay on the grid)."""
        return max(1.0, round(v * self.xf[2] * self.k))

    # ---- output -----------------------------------------------------------
    def _paint(self, c, fill, stroke=True):
        a = []
        if fill is not None:
            a.append(f'fill="{color(fill)}"')
        if stroke and c is not None:
            a.append(f'stroke="{color(c)}"')
        return " ".join(a)

    def add(self, el):
        self.items.append(el)

    def new_id(self, p):
        self.ids += 1
        return f"{p}{self.ids}"

    # ---- stroked primitives (centre-line coordinates) --------------------
    def line(self, x1, y1, x2, y2, c="fg"):
        self.add(f'<path d="M{num(self.X(x1))} {num(self.Y(y1))}L{num(self.X(x2))} {num(self.Y(y2))}" {self._paint(c, None)}/>')

    def poly(self, pts, close=False, c="fg", fill=None):
        if close:
            pts = list(pts) + [pts[0]]   # (an explicit close: see _path)
        d = "M" + "L".join(f"{num(self.X(x, fill and c is None))} {num(self.Y(y, fill and c is None))}" for x, y in pts)
        if close:
            d += "Z"
        self.add(f'<path d="{d}" {self._paint(c, fill)}/>')

    def rect(self, x, y, w, h, r=0, c="fg", fill=None):
        """A rectangle's outline (as a path that closes explicitly: see
        `_path`), corners rounded by `r`."""
        x0, y0, x1, y1 = self.X(x), self.Y(y), self.X(x + w), self.Y(y + h)
        rr = min(self.L(r), (x1 - x0) / 2, (y1 - y0) / 2) if r else 0
        if self.size == 16:
            # (a short side keeps a straight run: rounding eats small boxes)
            rr = min(rr, math.floor(min(x1 - x0, y1 - y0) / 4 * 2) / 2)
        if rr:
            a = f"A{num(rr)} {num(rr)} 0 0 1"
            d = (f"M{num(x0 + rr)} {num(y0)}H{num(x1 - rr)}{a} {num(x1)} {num(y0 + rr)}V{num(y1 - rr)}"
                 f"{a} {num(x1 - rr)} {num(y1)}H{num(x0 + rr)}{a} {num(x0)} {num(y1 - rr)}V{num(y0 + rr)}"
                 f"{a} {num(x0 + rr)} {num(y0)}Z")
        else:
            d = f"M{num(x0)} {num(y0)}H{num(x1)}V{num(y1)}H{num(x0)}V{num(y0)}Z"
        self.add(f'<path d="{d}" {self._paint(c, fill)}/>')

    def _centre(self, v, axis, r):
        """A circle's centre: its edges (centre +- r) on the stroke grid."""
        return self.snap(self.m(v, axis), (self.phase + r) % 1)

    def circle(self, cx, cy, r, c="fg", fill=None):
        r = max(1.0, self.L(r))
        self.add(f'<circle cx="{num(self._centre(cx, 0, r))}" cy="{num(self._centre(cy, 1, r))}" r="{num(r)}" {self._paint(c, fill)}/>')

    def ellipse(self, cx, cy, rx, ry, c="fg", fill=None):
        rx, ry = max(1.0, self.L(rx)), max(0.5, self.L(ry))
        self.add(f'<ellipse cx="{num(self._centre(cx, 0, rx))}" cy="{num(self._centre(cy, 1, ry))}" rx="{num(rx)}" ry="{num(ry)}" {self._paint(c, fill)}/>')

    def native(self, size, d, c="fg", fill=None):
        """A path in this size's own pixels (hand-hinted), when `self.size`
        is `size`; returns whether it drew."""
        if self.size != size:
            return False
        self.add(f'<path d="{d}" {self._paint(c, fill)}/>')
        return True

    def path(self, d, c="fg", fill=None, snap=True):
        self.add(f'<path d="{self._path(d, fill=(c is None), snap=snap)}" {self._paint(c, fill)}/>')

    # ---- fill-only primitives (outer edges) --------------------------------
    def frect(self, x, y, w, h, fill="fg", r=0):
        x0, y0 = self.X(x, True), self.Y(y, True)
        x1, y1 = max(self.X(x + w, True), x0 + 1), max(self.Y(y + h, True), y0 + 1)
        rr = min(self.L(r), (x1 - x0) / 2, (y1 - y0) / 2) if r else 0
        ra = f' rx="{num(rr)}"' if rr else ""
        self.add(f'<rect x="{num(x0)}" y="{num(y0)}" width="{num(x1 - x0)}" height="{num(y1 - y0)}"{ra} fill="{color(fill)}"/>')

    def dot(self, cx, cy, r, fill="fg"):
        """A filled dot; two pixels or less across it's a square (crisp)."""
        d = max(1, round(2 * r * self.xf[2] * self.k))
        ph = (d / 2) % 1
        x, y = self.snap(self.m(cx, 0), ph), self.snap(self.m(cy, 1), ph)
        if d <= 2:
            self.add(f'<rect x="{num(x - d / 2)}" y="{num(y - d / 2)}" width="{d}" height="{d}" fill="{color(fill)}"/>')
        else:
            self.add(f'<circle cx="{num(x)}" cy="{num(y)}" r="{num(d / 2)}" fill="{color(fill)}"/>')

    def fpoly(self, pts, fill="fg"):
        d = "M" + "L".join(f"{num(self.X(x, True))} {num(self.Y(y, True))}" for x, y in pts) + "Z"
        self.add(f'<path d="{d}" fill="{color(fill)}"/>')

    def fpath(self, d, fill="fg", snap=True, evenodd=False):
        rule = ' fill-rule="evenodd"' if evenodd else ""
        self.add(f'<path d="{self._path(d, fill=True, snap=snap)}" fill="{color(fill)}"{rule}/>')

    def _path(self, d, fill, snap):
        toks = PATH_TOKEN.findall(d)
        out = []
        i = 0
        cmd = None
        # absolute commands only (M L H V A C Q Z)
        def pt(x, y):
            if snap:
                return f"{num(self.X(x, fill))} {num(self.Y(y, fill))}"
            return f"{num(self.m(x, 0))} {num(self.m(y, 1))}"
        # (resvg's rasterizer, tiny-skia, strokes a subpath's implicit
        # closing segment half a pixel soft: a Z that would draw one gets an
        # explicit segment back to the start before it)
        start = cur = None
        def X_(v):
            return self.X(v, fill) if snap else self.m(v, 0)
        def Y_(v):
            return self.Y(v, fill) if snap else self.m(v, 1)
        while i < len(toks):
            t = toks[i]
            if t.isalpha():
                cmd = t
                i += 1
                if cmd in "Zz":
                    if start and cur and (abs(start[0] - cur[0]) > 1e-6 or abs(start[1] - cur[1]) > 1e-6):
                        out.append(f"L{num(start[0])} {num(start[1])}")
                    out.append("Z")
                    cur = start
                    continue
            vals = toks[i:]
            if cmd == "M" or cmd == "L":
                p = (X_(float(vals[0])), Y_(float(vals[1])))
                out.append(f"{cmd}{num(p[0])} {num(p[1])}")
                if cmd == "M":
                    start = p
                cur = p
                i += 2
            elif cmd == "H":
                cur = (X_(float(vals[0])), cur[1])
                out.append(f"H{num(cur[0])}")
                i += 1
            elif cmd == "V":
                cur = (cur[0], Y_(float(vals[0])))
                out.append(f"V{num(cur[1])}")
                i += 1
            elif cmd == "A":
                rx, ry, rot, large, sweep, x, y = (float(v) for v in vals[:7])
                cur = (X_(x), Y_(y))
                out.append(f"A{num(self.L(rx))} {num(self.L(ry))} {num(rot)} {int(large)} {int(sweep)} {num(cur[0])} {num(cur[1])}")
                i += 7
            elif cmd == "C":
                v = [float(x) for x in vals[:6]]
                cur = (X_(v[4]), Y_(v[5]))
                out.append(f"C{pt(v[0], v[1])} {pt(v[2], v[3])} {num(cur[0])} {num(cur[1])}")
                i += 6
            elif cmd == "Q":
                v = [float(x) for x in vals[:4]]
                cur = (X_(v[2]), Y_(v[3]))
                out.append(f"Q{pt(v[0], v[1])} {num(cur[0])} {num(cur[1])}")
                i += 4
            else:
                raise ValueError(f"path command {cmd!r} unsupported (absolute M L H V A C Q Z only)")
        return "".join(out)

    # ---- structure ----------------------------------------------------------
    def cut(self, shape, gap=None):
        """Cuts what `shape(g)` draws (filled, grown by `gap` master px —
        default one stroke width) out of everything drawn so far: what is
        drawn next sits in the gap (a badge, an overlapping sheet)."""
        mid = self.new_id("m")
        inner = G(self.size, self.mono)
        inner.xf = self.xf
        shape(inner)
        gap_px = (self.w if gap is None else gap * self.k)
        grown = []
        for el in inner.items:
            stroked = 'stroke="' in el
            el = re.sub(r' (fill|stroke)="[^"]*"', "", el)
            width = (self.w if stroked else 0) + 2 * gap_px
            el = el.replace("/>", f' fill="black" stroke="black" stroke-width="{num(width)}"/>')
            grown.append(el)
        s = self.size
        self.defs.append(f'<mask id="{mid}" maskUnits="userSpaceOnUse" x="0" y="0" width="{s}" height="{s}">'
                         f'<rect width="{s}" height="{s}" fill="white"/>{"".join(grown)}</mask>')
        self.items = [f'<g mask="url(#{mid})">{"".join(self.items)}</g>']

    def sub(self, dx, dy, s=1.0):
        """Draws in a transformed master space: `with g.sub(…):`."""
        g = self

        class _Ctx:
            def __enter__(self_):
                self_.was = g.xf
                odx, ody, os_ = g.xf
                g.xf = (odx + dx * os_, ody + dy * os_, os_ * s)
                return g

            def __exit__(self_, *a):
                g.xf = self_.was

        return _Ctx()

    def gradient(self, x1, y1, x2, y2, stops):
        gid = self.new_id("g")
        st = "".join(f'<stop offset="{num(o)}" stop-color="{color(c)}"/>' for o, c in stops)
        self.defs.append(f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="{num(self.m(x1, 0))}" '
                         f'y1="{num(self.m(y1, 1))}" x2="{num(self.m(x2, 0))}" y2="{num(self.m(y2, 1))}">{st}</linearGradient>')
        return f"url(#{gid})"

    def svg(self, title=None, desc=None):
        s = self.size
        head = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="0 0 {s} {s}" '
                f'fill="none" stroke-width="{num(self.w)}" stroke-linecap="round" stroke-linejoin="round">')
        meta = ""
        if title:
            meta += f"\n  <title>{title}</title>"
        if desc:
            meta += f"\n  <desc>{desc}</desc>"
        defs = f"\n  <defs>{''.join(self.defs)}</defs>" if self.defs else ""
        body = "".join(f"\n  {el}" for el in self.items)
        return f"{head}{meta}{defs}{body}\n</svg>\n"


# ---- the registry -------------------------------------------------------------

ICONS = {}   # "category/name" -> dict(fn, cat, name, mono, title)


def icon(name, cat, title, mono=None):
    """Registers an icon as `cat/name` (its id): `cat` is its category
    (actions, components …); `mono` defaults to True for actions and
    glyphs."""
    def deco(fn):
        key = f"{cat}/{name}"
        if key in ICONS:
            raise ValueError(f"icon {key} defined twice")
        m = mono if mono is not None else cat in ("actions", "glyphs")
        ICONS[key] = dict(fn=fn, cat=cat, name=name, mono=m, title=title)
        return fn
    return deco


def draw(key, size):
    """Icon `key` ("category/name") drawn at `size`: its SVG source."""
    spec = ICONS[key]
    g = G(size, spec["mono"])
    spec["fn"](g)
    return g.svg(title=spec["title"])
