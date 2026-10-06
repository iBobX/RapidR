"""Shapes shared by many icons, in master coordinates (design/icons/README.md,
"Metaphors"): the page, the folder, the window, the cylinder, the gear …
Each draws with a `kit.G`; colours are passed in (`c` outline, `fill`)."""

import math

# The keylines (master px): live area 2..22; a square 3..21; a circle of
# radius 9.5 round (12, 12); a page 5..19 x 2..22; a window 3..21 x 4..20.
PAGE = (5, 2, 14, 20)
WINDOW = (3, 4, 18, 16)


def arrowhead(g, x, y, dx, dy, size=4, c="fg"):
    """A chevron arrowhead with its tip at (x, y), pointing along (dx, dy)."""
    n = math.hypot(dx, dy)
    ux, uy = dx / n, dy / n
    a = []
    for sgn in (1, -1):
        # the arm: back along the direction, turned 45 degrees
        rx = -ux * math.cos(math.pi / 4) - sgn * -uy * math.sin(math.pi / 4)
        ry = -uy * math.cos(math.pi / 4) - sgn * ux * math.sin(math.pi / 4)
        a.append((x + rx * size, y + ry * size))
    g.poly([a[0], (x, y), a[1]], c=c)


def page(g, x=5, y=2, w=14, h=20, fold=5, c="fg", fill=None, fold_fill=None):
    """A sheet with its top-right corner folded (the brand's file shape)."""
    g.path(f"M{x} {y} H{x + w - fold} L{x + w} {y + fold} V{y + h} H{x} Z", c=c, fill=fill)
    g.path(f"M{x + w - fold} {y} V{y + fold} H{x + w}", c=c, fill=fold_fill)


def folder(g, x=3, y=5, w=18, h=14, c="fg", fill=None, tab_fill=None):
    """A closed folder: a tab top-left, the body."""
    g.path(f"M{x} {y + h} V{y} H{x + 6} L{x + 8} {y + 2} H{x + w} V{y + h} Z", c=c, fill=fill)
    g.line(x, y + 5, x + w, y + 5, c=c)


def folder_open(g, x=3, y=5, c="fg", fill=None, front_fill=None):
    g.path(f"M{x} {y + 14} V{y} H{x + 6} L{x + 8} {y + 2} H{x + 15} V{y + 5}", c=c, fill=fill)
    g.path(f"M{x} {y + 14} L{x + 3.5} {y + 6} H{x + 19} L{x + 15.5} {y + 14} Z", c=c, fill=front_fill)


def window(g, x=3, y=4, w=18, h=16, c="fg", fill=None, bar_fill=None, bar=4.5):
    """A window: a rounded frame with a title bar."""
    if bar_fill:
        g.rect(x, y, w, h, r=2, c=c, fill=fill)
        g.path(f"M{x} {y + bar} V{y + 2} A2 2 0 0 1 {x + 2} {y} H{x + w - 2} A2 2 0 0 1 {x + w} {y + 2} V{y + bar} Z", c=c, fill=bar_fill)
    else:
        g.rect(x, y, w, h, r=2, c=c, fill=fill)
        g.line(x, y + bar, x + w, y + bar, c=c)


def magnifier(g, cx=10.5, cy=10.5, r=6.5, c="fg", fill=None, handle=5.5):
    g.circle(cx, cy, r, c=c, fill=fill)
    d = r / math.sqrt(2)
    g.line(cx + d + 0.6, cy + d + 0.6, cx + d + handle, cy + d + handle, c=c)


def cylinder(g, x=5, y=3, w=14, h=18, c="fg", fill=None, bands=1, top_fill=None):
    """A database: a cylinder with `bands` rings."""
    rx, ry = w / 2, (2.5 if g.size != 16 else 2.2)
    cx = x + rx
    g.path(f"M{x} {y + ry} V{y + h - ry} A{rx} {ry} 0 0 0 {x + w} {y + h - ry} V{y + ry}", c=c, fill=fill)
    g.ellipse(cx, y + ry, rx, ry, c=c, fill=top_fill if top_fill else fill)
    for i in range(1, bands + 1):
        yy = y + ry + (h - 2 * ry) * i / (bands + 1)
        g.path(f"M{x} {yy} A{rx} {ry} 0 0 0 {x + w} {yy}", c=c)


def gear(g, cx=12, cy=12, r_out=9, r_in=6.6, teeth=8, hole=2.6, c="fg", fill=None, tip=0.22, root=0.30):
    """A gear: `teeth` square teeth, a round hole."""
    pts = []
    step = 2 * math.pi / teeth
    tw = step * tip    # half the tooth's angle at the tip
    bw = step * root   # … at the root
    for i in range(teeth):
        a = -math.pi / 2 + i * step
        for ang, rad in ((a - bw, r_in), (a - tw, r_out), (a + tw, r_out), (a + bw, r_in)):
            pts.append((cx + rad * math.cos(ang), cy + rad * math.sin(ang)))
    d = "M" + " L".join(f"{x:.3f} {y:.3f}" for x, y in pts) + " Z"
    g.path(d, c=c, fill=fill, snap=g.size != 24 and False)
    if hole:
        g.circle(cx, cy, hole, c=c)


def floppy(g, x=4, y=4, s=16, c="fg", fill=None, detail=True):
    """A diskette (Save): a square body with a cut corner, the shutter, the label."""
    g.path(f"M{x} {y} H{x + s - 3.5} L{x + s} {y + 3.5} V{y + s} H{x} Z", c=c, fill=fill)
    if detail:
        g.path(f"M{x + 4} {y} V{y + 4} H{x + s - 6} V{y}", c=c)
    g.path(f"M{x + 3.5} {y + s} V{y + s - 6} H{x + s - 3.5} V{y + s}", c=c)


def star4(g, cx, cy, r, fill="fg", pinch=0.18):
    """A four-pointed sparkle (concave sides)."""
    p = r * pinch
    g.fpath(f"M{cx} {cy - r} Q{cx + p} {cy - p} {cx + r} {cy} Q{cx + p} {cy + p} {cx} {cy + r} "
            f"Q{cx - p} {cy + p} {cx - r} {cy} Q{cx - p} {cy - p} {cx} {cy - r} Z", fill=fill, snap=False)


def rot(points, cx, cy, deg):
    a = math.radians(deg)
    ca, sa = math.cos(a), math.sin(a)
    return [(cx + (x - cx) * ca - (y - cy) * sa, cy + (x - cx) * sa + (y - cy) * ca) for x, y in points]


def cube_iso(g, c="fg", fill=None):
    """An isometric cube round (12, 12): a SUB, a module, a mesh."""
    g.path("M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z", c=c, fill=fill)
    g.path("M4 7.5 L12 12 L20 7.5 M12 12 V21", c=c if fill != c else "paper")
