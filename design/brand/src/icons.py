"""Writes the icon SVG masters into icons/svg/.

Large masters (1024 grid) are drawn from the mark geometry. Small sizes
(16, 20, 22, 24, 32) are separate, hand-hinted drawings: every horizontal
and vertical edge sits on the pixel grid and the glyph is enlarged and
simplified for legibility, so they are not plain downscales.
"""
import os

from brandkit import (BRAND, INK, PAPER, BLUE, BLUE_DEEP, CYAN, AMBER, TEAL, TEAL_DEEP, WHITE, SLATE,
                      svg_doc, squircle, rrect, scale_translate, transform, text_path, _num)
from marks import run_r_centered

OUT = os.path.join(BRAND, "icons", "svg")
os.makedirs(OUT, exist_ok=True)

INK_HI = "#1A2644"     # top of the Runtime tile gradient
PAGE_EDGE = "#C9D1E3"
PAGE_FOLD = "#E3E8F2"
CODE_GREY = "#B9C2D4"


def write(name, body, w=1024, h=1024, defs="", title=None):
    open(os.path.join(OUT, name + ".svg"), "w").write(svg_doc(body, w, h, defs=defs, title=title))


# ------------------------------------------------------------ app icons --
def lin(gid, x1, y1, x2, y2, stops):
    s = "".join(f'<stop offset="{o}" stop-color="{c}"{"" if a is None else f" stop-opacity=\"{a}\""}/>'
                for o, c, a in stops)
    return (f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="{_num(x1)}" y1="{_num(y1)}" '
            f'x2="{_num(x2)}" y2="{_num(y2)}">{s}</linearGradient>')


def rad(gid, cx, cy, r, stops):
    s = "".join(f'<stop offset="{o}" stop-color="{c}" stop-opacity="{a}"/>' for o, c, a in stops)
    return (f'<radialGradient id="{gid}" gradientUnits="userSpaceOnUse" cx="{_num(cx)}" cy="{_num(cy)}" '
            f'r="{_num(r)}">{s}</radialGradient>')


def shadow(fid, dy, blur, opacity, color="#000000"):
    return (f'<filter id="{fid}" x="-20%" y="-20%" width="140%" height="140%" color-interpolation-filters="sRGB">'
            f'<feGaussianBlur in="SourceAlpha" stdDeviation="{_num(blur)}"/>'
            f'<feOffset dy="{_num(dy)}" result="b"/>'
            f'<feFlood flood-color="{color}" flood-opacity="{opacity}"/><feComposite in2="b" operator="in"/>'
            f'<feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge></filter>')


def r_parts(x, y, size):
    """The R mark (outer, counter, leg) placed on a tile at (x, y, size)."""
    k = size / 100
    d = scale_translate(run_r_centered(), k, x, y)
    return d


def r_counter(x, y, size):
    k = size / 100
    return scale_translate("M41.5 31 L41.5 47 L60.5 39 Z", k, x, y)


# The default icon of a compiled program (`rapidr build`): the Runtime's Ink
# tile with an app window drawn in white and the Run triangle in amber inside
# it, so a program reads as "made with RapidR" until it gets its own icon.
# On the 100 grid: the window's frame, its title bar and the triangle.
WIN_OUTER = (18, 22, 64, 56, 8)          # x, y, w, h, radius
WIN_HOLE = (24, 34, 52, 38, 3)           # the window's inside (frame 6, title bar 12)
WIN_DOTS = ((26.5, 28), (32.5, 28), (38.5, 28), 1.9)   # title-bar dots: centres, radius
WIN_PLAY = "M43.8 43 L43.8 63 L61.1 53 Z"


def window_parts(x, y, size):
    """(frame with its inside cut out, the title bar's dots, the triangle) on a tile at (x, y, size)."""
    k = size / 100
    ox, oy, ow, oh, orr = WIN_OUTER
    hx, hy, hw, hh, hr = WIN_HOLE
    frame = scale_translate(f"{rrect(ox, oy, ow, oh, orr)} {rrect(hx, hy, hw, hh, hr)}", k, x, y)
    (d1, d2, d3, r) = WIN_DOTS
    dots = " ".join(rrect(cx - r, cy - r, 2 * r, 2 * r, r) for cx, cy in (d1, d2, d3))
    return frame, scale_translate(dots, k, x, y), scale_translate(WIN_PLAY, k, x, y)


def app_icon(kind, platform):
    """kind: 'ide' or 'runtime'. platform: 'macos' (Apple grid, 824 tile on
    1024, with shadow) or 'full' (Windows/Linux: 944 tile)."""
    if platform == "macos":
        x = y = 100
        size = 824
    else:
        x = y = 40
        size = 944
    ext = size * 0.34
    tile = squircle(x, y, size, size, ext)
    defs = []
    body = []
    if platform == "macos":
        defs.append(shadow("ts", 10, 14, 0.30))
    if kind == "ide":
        defs.append(lin("bg", x, y + size, x + size, y, [(0, BLUE, None), (1, CYAN, None)]))
        defs.append(rad("hl", x + size * 0.25, y + size * 0.1, size * 0.9,
                        [(0, WHITE, 0.22), (1, WHITE, 0)]))
        defs.append(lin("gl", 0, y + size * 0.19, 0, y + size * 0.81, [(0, WHITE, None), (1, "#E6EEFF", None)]))
        defs.append(shadow("gs", size * 0.014, size * 0.018, 0.28, "#0B2A8A"))
        body.append(f'<g filter="url(#ts)"><path fill="url(#bg)" d="{tile}"/></g>' if platform == "macos"
                    else f'<path fill="url(#bg)" d="{tile}"/>')
        body.append(f'<path fill="url(#hl)" d="{tile}"/>')
        body.append(f'<path fill="url(#gl)" filter="url(#gs)" d="{r_parts(x, y, size)}"/>')
    elif kind == "program":
        defs.append(lin("bg", 0, y, 0, y + size, [(0, INK_HI, None), (1, INK, None)]))
        defs.append(rad("hl", x + size * 0.25, y + size * 0.05, size * 0.8,
                        [(0, "#5B8CFF", 0.18), (1, "#5B8CFF", 0)]))
        defs.append(lin("gl", 0, y + size * 0.22, 0, y + size * 0.78, [(0, WHITE, None), (1, "#DCE5F7", None)]))
        defs.append(shadow("gs", size * 0.012, size * 0.02, 0.45, "#000000"))
        defs.append(lin("am", 0, y + size * 0.43, 0, y + size * 0.63, [(0, "#FFCB52", None), (1, "#FFA514", None)]))
        frame, dots, play = window_parts(x, y, size)
        body.append(f'<g filter="url(#ts)"><path fill="url(#bg)" d="{tile}"/></g>' if platform == "macos"
                    else f'<path fill="url(#bg)" d="{tile}"/>')
        body.append(f'<path fill="url(#hl)" d="{tile}"/>')
        body.append(f'<path fill="none" stroke="#FFFFFF" stroke-opacity="0.10" stroke-width="{_num(size * 0.006)}" d="{tile}"/>')
        body.append(f'<path fill="url(#gl)" fill-rule="evenodd" filter="url(#gs)" d="{frame}"/>')
        body.append(f'<path fill="{INK}" d="{dots}"/>')
        body.append(f'<path fill="url(#am)" d="{play}"/>')
    else:
        defs.append(lin("bg", 0, y, 0, y + size, [(0, INK_HI, None), (1, INK, None)]))
        defs.append(rad("hl", x + size * 0.25, y + size * 0.05, size * 0.8,
                        [(0, "#5B8CFF", 0.18), (1, "#5B8CFF", 0)]))
        defs.append(lin("gl", 0, y + size * 0.19, 0, y + size * 0.81, [(0, WHITE, None), (1, "#DCE5F7", None)]))
        defs.append(shadow("gs", size * 0.012, size * 0.02, 0.45, "#000000"))
        defs.append(lin("am", 0, y + size * 0.31, 0, y + size * 0.47, [(0, "#FFCB52", None), (1, "#FFA514", None)]))
        body.append(f'<g filter="url(#ts)"><path fill="url(#bg)" d="{tile}"/></g>' if platform == "macos"
                    else f'<path fill="url(#bg)" d="{tile}"/>')
        body.append(f'<path fill="url(#hl)" d="{tile}"/>')
        # a hairline rim so the dark tile holds its edge on dark docks/taskbars
        body.append(f'<path fill="none" stroke="#FFFFFF" stroke-opacity="0.10" stroke-width="{_num(size * 0.006)}" d="{tile}"/>')
        body.append(f'<path fill="url(#gl)" filter="url(#gs)" d="{r_parts(x, y, size)}"/>')
        body.append(f'<path fill="url(#am)" d="{r_counter(x, y, size)}"/>')
    name = {"ide": "RapidR", "runtime": "RapidR Runtime", "program": "RapidR program"}[kind]
    return "".join(body), "".join(defs), f"{name} app icon"


# Hand-hinted small app icons. Coordinates are in pixels of the target size.
SMALL_R = {
    16: dict(tile=(1, 1, 14, 3),
             outer="M4 3 H8.5 A3.5 3.5 0 0 1 8.5 10 H6 V13 H4 Z",
             counter="M6 5 V8 H8 V7 H9 V6 H8 V5 Z",
             leg="M6 10 H8.6 L11.6 13 H9 Z"),
    20: dict(tile=(1, 1, 18, 4),
             outer="M5 4 H11 A4 4 0 0 1 11 12 H8 V16 H5 Z",
             counter="M8 6 V10 L11.6 8 Z",
             leg="M8 12 H11 L14.6 16 H11.6 Z"),
    24: dict(tile=(1, 1, 22, 5),
             outer="M6 5 H13 A5 5 0 0 1 13 15 H10 V19 H6 Z",
             counter="M10 8 V12 L14.4 10 Z",
             leg="M10 15 H13.6 L17.8 19 H14.2 Z"),
    32: dict(tile=(1, 1, 30, 6.5),
             outer="M8 7 H17 A6.5 6.5 0 0 1 17 20 H13 V25 H8 Z",
             counter="M13 10.5 V16.5 L19.2 13.5 Z",
             leg="M13 20 H17.8 L23 25 H18.2 Z"),
}
# macOS 32 px (and 16@2x): the Apple grid keeps the tile at ~80 % of the canvas
SMALL_R["mac32"] = dict(tile=(3, 3, 26, 6),
                        outer="M10 8 H17 A5 5 0 0 1 17 18 H14 V24 H10 Z",
                        counter="M14 11 V15 L18.4 13 Z",
                        leg="M14 18 H17.6 L22.6 24 H19 Z")
SMALL_R[22] = {k: (transform(v, (1, 0, 0, 1, 1, 1)) if isinstance(v, str) else v)
               for k, v in SMALL_R[20].items()}
SMALL_R[22]["tile"] = (1, 1, 20, 4.5)


# Hand-hinted small program icons (pixels of the target size): the window's
# outside and inside (x0, y0, x1, y1, radius) and the triangle.
SMALL_WIN = {
    16: dict(outer=(3, 3, 13, 13, 1.5), hole=(4, 6, 12, 12, 0), play="M7 7 V11 L10.4 9 Z"),
    20: dict(outer=(4, 4, 16, 16, 2), hole=(5, 7.5, 15, 15, 0), play="M8.5 9 V13.5 L12.5 11.25 Z"),
    22: dict(outer=(4, 4, 18, 18, 2), hole=(5, 8, 17, 17, 0.5), play="M9.5 9.5 V15 L14.2 12.25 Z"),
    24: dict(outer=(5, 5, 19, 19, 2.5), hole=(6, 9, 18, 18, 0.5), play="M10.5 10.5 V16 L15.2 13.25 Z"),
    32: dict(outer=(6, 7, 26, 25, 3), hole=(8, 11, 24, 23, 1), play="M13.5 13.5 V20.5 L19.8 17 Z"),
    "mac32": dict(outer=(8, 8, 24, 24, 2.5), hole=(9, 11.5, 23, 23, 0.5), play="M14 13.5 V20.5 L19.6 17 Z"),
}


def program_icon_small(s, tile):
    tx, ty, tw, tr = tile
    p = SMALL_WIN[s]
    ox0, oy0, ox1, oy1, orr = p["outer"]
    hx0, hy0, hx1, hy1, hr = p["hole"]
    hole = (f"M{_num(hx0)} {_num(hy0)} H{_num(hx1)} V{_num(hy1)} H{_num(hx0)} Z" if hr == 0
            else rrect(hx0, hy0, hx1 - hx0, hy1 - hy0, hr))
    frame = f"{rrect(ox0, oy0, ox1 - ox0, oy1 - oy0, orr)} {hole}"
    defs = lin("bg", 0, ty, 0, ty + tw, [(0, INK_HI, None), (1, INK, None)])
    body = (f'<path fill="url(#bg)" d="{rrect(tx, ty, tw, tw, tr)}"/>'
            f'<path fill="{WHITE}" fill-rule="evenodd" d="{frame}"/>'
            f'<path fill="{AMBER}" d="{p["play"]}"/>')
    return body, defs


def app_icon_small(kind, s):
    p = SMALL_R[s]
    tx, ty, tw, tr = p["tile"]
    if kind == "program":
        return program_icon_small(s, p["tile"])
    tile = rrect(tx, ty, tw, tw, tr)
    if kind == "ide":
        defs = lin("bg", tx, ty + tw, tx + tw, ty, [(0, BLUE, None), (1, CYAN, None)])
        body = (f'<path fill="url(#bg)" d="{tile}"/>'
                f'<path fill="{WHITE}" fill-rule="evenodd" d="{p["outer"]} {p["counter"]} {p["leg"]}"/>')
    else:
        defs = lin("bg", 0, ty, 0, ty + tw, [(0, INK_HI, None), (1, INK, None)])
        body = (f'<path fill="url(#bg)" d="{tile}"/>'
                f'<path fill="{WHITE}" fill-rule="evenodd" d="{p["outer"]} {p["counter"]} {p["leg"]}"/>'
                f'<path fill="{AMBER}" d="{p["counter"]}"/>')
    return body, defs


# ----------------------------------------------------------- file icons --
# Page on the 1024 grid: 704 x 880, folded top-right corner.
PX, PY, PW, PH, FOLD, PR = 160, 72, 704, 880, 200, 40


def page_paths():
    x2, y2 = PX + PW, PY + PH
    outline = (f"M{PX + PR} {PY} H{x2 - FOLD} L{x2} {PY + FOLD} V{y2 - PR} "
               f"A{PR} {PR} 0 0 1 {x2 - PR} {y2} H{PX + PR} A{PR} {PR} 0 0 1 {PX} {y2 - PR} "
               f"V{PY + PR} A{PR} {PR} 0 0 1 {PX + PR} {PY} Z")
    fold = f"M{x2 - FOLD} {PY} V{PY + FOLD - 24} A24 24 0 0 0 {x2 - FOLD + 24} {PY + FOLD} H{x2} Z"
    return outline, fold


def label_path(text, cx, baseline, size, weight=800):
    d, w = text_path(text, size, 0, baseline, weight=weight, tracking=20)
    return transform(d, (1, 0, 0, 1, cx - w / 2, 0))


def file_icon(kind):
    outline, fold = page_paths()
    dark = kind == "rrbc"
    defs = [shadow("ps", 12, 16, 0.22)]
    body = []
    page_fill = "url(#pg)" if dark else WHITE
    if dark:
        defs.append(lin("pg", 0, PY, 0, PY + PH, [(0, INK_HI, None), (1, INK, None)]))
    body.append(f'<g filter="url(#ps)"><path fill="{page_fill}" d="{outline}"/></g>')
    if not dark:
        body.append(f'<path fill="none" stroke="{PAGE_EDGE}" stroke-width="8" d="{outline}"/>')
    body.append(f'<path fill="{"#2A3657" if dark else PAGE_FOLD}" d="{fold}"/>')
    if not dark:
        body.append(f'<path fill="none" stroke="{PAGE_EDGE}" stroke-width="8" stroke-linejoin="round" d="{fold}"/>')
    cx = PX + PW / 2
    if kind == "rr":
        # the app tile, centred in the upper page
        ts, tx, ty = 400, cx - 200, 250
        defs.append(lin("bg", tx, ty + ts, tx + ts, ty, [(0, BLUE, None), (1, CYAN, None)]))
        defs.append(shadow("gs", 6, 8, 0.25, "#0B2A8A"))
        body.append(f'<path fill="url(#bg)" d="{squircle(tx, ty, ts, ts, ts * 0.34)}"/>')
        body.append(f'<path fill="{WHITE}" filter="url(#gs)" d="{r_parts(tx, ty, ts)}"/>')
        body.append(f'<path fill="{BLUE_DEEP}" d="{label_path("RR", cx, 832, 150)}"/>')
    elif kind == "bas":
        # BASIC listing: line numbers in teal, statements in grey
        y = 236
        rows = [(3, 300), (3, 220), (3, 360), (3, 180), (3, 260)]
        for i, (_, w) in enumerate(rows):
            ly = y + i * 92
            body.append(f'<path fill="{TEAL}" d="{rrect(PX + 96, ly, 84, 44, 22)}"/>')
            body.append(f'<path fill="{CODE_GREY}" d="{rrect(PX + 204, ly, w, 44, 22)}"/>')
        body.append(f'<path fill="{TEAL_DEEP}" d="{label_path("BAS", cx, 832, 150)}"/>')
    else:  # rrbc: compiled program, runs in the Runtime
        ts, tx, ty = 440, cx - 220, 230
        defs.append(lin("gl", tx, ty + ts, tx + ts, ty, [(0, BLUE, None), (1, CYAN, None)]))
        defs.append(lin("am", 0, ty + ts * 0.31, 0, ty + ts * 0.47, [(0, "#FFCB52", None), (1, "#FFA514", None)]))
        body.append(f'<path fill="url(#gl)" d="{r_parts(tx, ty, ts)}"/>')
        body.append(f'<path fill="url(#am)" d="{r_counter(tx, ty, ts)}"/>')
        body.append(f'<path fill="{AMBER}" d="{label_path("RRBC", cx, 832, 132)}"/>')
    names = {"rr": "RapidR source (.rr)", "bas": "BASIC source (.bas)", "rrbc": "RapidR program (.rrbc)"}
    return "".join(body), "".join(defs), names[kind] + " file icon"


# Small file icons: page = x 2.5..13.5 style outlines, all hinted.
def small_page(s, dark):
    """Pixel-snapped page with folded corner for size s. Returns (body, content box)."""
    if s == 16:
        x0, y0, x1, y1, f = 2, 0, 14, 16, 4
    elif s == 20:
        x0, y0, x1, y1, f = 3, 1, 17, 19, 5
    elif s == 22:
        x0, y0, x1, y1, f = 3, 1, 19, 21, 5
    elif s == 24:
        x0, y0, x1, y1, f = 3, 1, 21, 23, 6
    else:  # 32
        x0, y0, x1, y1, f = 4, 1, 28, 31, 8
    # outline drawn as a 1px inset stroke via two fills (crisp, no AA on straight edges)
    outer = f"M{x0} {y0} H{x1 - f} L{x1} {y0 + f} V{y1} H{x0} Z"
    inner = f"M{x0 + 1} {y0 + 1} V{y1 - 1} H{x1 - 1} V{y0 + f + 0.414} L{x1 - f - 0.414} {y0 + 1} Z"
    fold = f"M{x1 - f} {y0} V{y0 + f} H{x1} Z"
    edge = "#3A4766" if dark else "#8A94AA"
    fillc = INK if dark else WHITE
    foldc = "#3A4766" if dark else "#D5DBE8"
    body = (f'<path fill="{edge}" d="{outer}"/><path fill="{fillc}" d="{inner}"/>'
            f'<path fill="{foldc}" d="{fold}"/>')
    return body, (x0 + 1, y0 + 1, x1 - 1, y1 - 1, f)


# where the hinted small R sits on each small page (dx, dy)
SMALL_FILE_SHIFT = {16: (0, 1), 20: (0, 2), 22: (0, 2), 24: (0, 2), 32: (0, 3)}


def file_icon_small(kind, s):
    dark = kind == "rrbc"
    page, (cx0, cy0, cx1, cy1, f) = small_page(s, dark)
    body = [page]
    w = cx1 - cx0
    p = SMALL_R[s]
    dx, dy = SMALL_FILE_SHIFT[s]
    m = (1, 0, 0, 1, dx, dy)
    glyph = transform(f'{p["outer"]} {p["counter"]} {p["leg"]}', m)
    if kind == "rr":
        body.append(f'<path fill="url(#gl)" fill-rule="evenodd" d="{glyph}"/>')
        defs = lin("gl", 0, cy1, w, cy0, [(0, BLUE_DEEP, None), (1, BLUE, None)])
    elif kind == "bas":
        lines = {16: [(4, 4, 5), (7, 6, 4), (10, 4, 6)],
                 20: [(5, 6, 6), (8, 6, 5), (11, 6, 7), (14, 6, 4)],
                 22: [(6, 7, 6), (9, 7, 5), (12, 7, 8), (15, 7, 4)],
                 24: [(6, 7, 7), (9, 7, 5), (12, 7, 9), (15, 7, 4), (18, 7, 6)],
                 32: [(8, 9, 9), (12, 9, 7), (16, 9, 12), (20, 9, 6), (24, 9, 9)]}[s]
        lh = 2 if s >= 24 else (2 if s >= 20 else 2)
        for (y, xs, ln) in lines:
            nx = cx0 + 1 if s < 24 else cx0 + 2
            nw = 2 if s < 32 else 3
            body.append(f'<rect x="{nx}" y="{y}" width="{nw}" height="{lh}" fill="{TEAL}"/>')
            body.append(f'<rect x="{nx + nw + 1}" y="{y}" width="{min(ln, cx1 - (nx + nw + 1) - 1)}" '
                        f'height="{lh}" fill="#97A2B8"/>')
        defs = ""
    else:
        body.append(f'<path fill="url(#gl)" fill-rule="evenodd" d="{glyph}"/>')
        body.append(f'<path fill="{AMBER}" d="{transform(p["counter"], m)}"/>')
        defs = lin("gl", 0, cy1, w, cy0, [(0, "#4A78FF", None), (1, CYAN, None)])
    return "".join(body), defs


SMALL = (16, 20, 22, 24, 32)


def touch_icon():
    """apple-touch-icon master: full-bleed and opaque (iOS applies its own mask)."""
    defs = (lin("bg", 0, 1024, 1024, 0, [(0, BLUE, None), (1, CYAN, None)])
            + rad("hl", 256, 102, 920, [(0, WHITE, 0.22), (1, WHITE, 0)])
            + lin("gl", 0, 195, 0, 829, [(0, WHITE, None), (1, "#E6EEFF", None)])
            + shadow("gs", 14, 18, 0.28, "#0B2A8A"))
    body = ('<rect width="1024" height="1024" fill="url(#bg)"/><rect width="1024" height="1024" fill="url(#hl)"/>'
            f'<path fill="url(#gl)" filter="url(#gs)" d="{r_parts(102, 102, 820)}"/>')
    return body, defs


def main():
    for kind in ("ide", "runtime", "program"):
        for plat in ("macos", "full"):
            b, d, t = app_icon(kind, plat)
            write(f"app-{kind}-{plat}", b, defs=d, title=t)
        for s in SMALL + ("mac32",):
            b, d = app_icon_small(kind, s)
            px = 32 if s == "mac32" else s
            write(f"app-{kind}-{s}", b, px, px, defs=d)
    b, d = touch_icon()
    write("web-touch-icon", b, defs=d, title="RapidR")
    for kind in ("rr", "bas", "rrbc"):
        b, d, t = file_icon(kind)
        write(f"file-{kind}", b, defs=d, title=t)
        for s in SMALL:
            b, d = file_icon_small(kind, s)
            write(f"file-{kind}-{s}", b, s, s, defs=d)


if __name__ == "__main__":
    main()
