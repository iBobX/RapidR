"""Writes the GitHub art into github/: the README banner (1280 x 400) and
the repository social preview (1280 x 640). All text is outlined (Inter
for prose, Liberation Mono for code), so the SVGs render without fonts."""
import os

from brandkit import (BRAND, INK, BLUE, CYAN, AMBER, WHITE, MIST, MONO, svg_doc, text_path,
                      transform, rrect, squircle, _num)
from logo import lockup, grad

OUT = os.path.join(BRAND, "github")
os.makedirs(OUT, exist_ok=True)

CARD = "#141E33"
KW = "#7FB2FF"
CMT = "#8792A8"
TXT = "#E6ECF8"
LN = "#6B7590"

CODE = [
    [("' my first RapidR program", CMT)],
    [("CREATE", KW), (" Form ", TXT), ("AS", KW), (" RForm", TXT)],
    [("    Caption = ", TXT), ('"Hello, RapidR"', AMBER)],
    [("    Width = ", TXT), ("320", CYAN)],
    [("END CREATE", KW)],
    [("Form.Center", TXT)],
    [("Form.ShowModal", TXT)],
]


def t(text, size, x, y, fill, weight=400, tracking=0, anchor="start", font=None):
    kw = {"font": font} if font else {}
    d, w = text_path(text, size, 0, y, weight=weight, tracking=tracking, **kw)
    if anchor == "middle":
        x -= w / 2
    d = transform(d, (1, 0, 0, 1, x, 0))
    return f'<path fill="{fill}" d="{d}"/>', w


def background(w, h):
    defs = (f'<radialGradient id="g1" gradientUnits="userSpaceOnUse" cx="{w * 0.82}" cy="{h * 0.1}" r="{w * 0.45}">'
            f'<stop offset="0" stop-color="{BLUE}" stop-opacity="0.38"/><stop offset="1" stop-color="{BLUE}" stop-opacity="0"/></radialGradient>'
            f'<radialGradient id="g2" gradientUnits="userSpaceOnUse" cx="{w * 0.98}" cy="{h}" r="{w * 0.3}">'
            f'<stop offset="0" stop-color="{CYAN}" stop-opacity="0.22"/><stop offset="1" stop-color="{CYAN}" stop-opacity="0"/></radialGradient>'
            f'<pattern id="dots" width="24" height="24" patternUnits="userSpaceOnUse">'
            f'<circle cx="2" cy="2" r="1.2" fill="#FFFFFF" fill-opacity="0.07"/></pattern>')
    body = (f'<rect width="{w}" height="{h}" fill="{INK}"/><rect width="{w}" height="{h}" fill="url(#dots)"/>'
            f'<rect width="{w}" height="{h}" fill="url(#g1)"/><rect width="{w}" height="{h}" fill="url(#g2)"/>')
    return body, defs


def code_card(x, y, w, size=16, lh=27):
    pad = 26
    head = 46
    h = head + pad * 0.6 + len(CODE) * lh + pad * 0.7
    out = [f'<path fill="{CARD}" fill-opacity="0.92" d="{rrect(x, y, w, h, 16)}"/>',
           f'<path fill="none" stroke="#FFFFFF" stroke-opacity="0.09" stroke-width="1.5" d="{rrect(x, y, w, h, 16)}"/>',
           f'<rect x="{x}" y="{y + head}" width="{w}" height="1.5" fill="#FFFFFF" fill-opacity="0.07"/>']
    p, _ = t("hello.rr", 15, x + pad, y + 29, MIST, 500)
    out.append(p)
    # Run pill
    pw, ph = 74, 28
    px, py = x + w - pad - pw, y + (head - ph) / 2
    out.append(f'<path fill="{AMBER}" d="{rrect(px, py, pw, ph, 14)}"/>')
    out.append(f'<path fill="{INK}" d="M{_num(px + 16)} {_num(py + 8)} V{_num(py + 20)} L{_num(px + 26)} {_num(py + 14)} Z"/>')
    p, _ = t("Run", 14, px + 33, py + 19, INK, 700)
    out.append(p)
    adv = 0.6 * size
    cy = y + head + pad * 0.6 + lh * 0.72
    for i, segs in enumerate(CODE):
        p, _ = t(str(i + 1), size, x + pad + adv, cy, LN, font=MONO, anchor="middle")
        out.append(p)
        cx = x + pad + adv * 3
        for text, col in segs:
            if text.strip():
                p, _ = t(text, size, cx, cy, col, font=MONO)
                out.append(p)
            cx += adv * len(text)
        cy += lh
    return "".join(out), h


def place_lockup(style, x, y, height):
    body, w = lockup(style)
    k = height / 100
    return f'<g transform="translate({_num(x)} {_num(y)}) scale({_num(k)})">{body}</g>', w * k


def banner():
    W, H = 1280, 400
    bg, defs = background(W, H)
    parts = [bg]
    lk, _ = place_lockup("dark", 80, 92, 92)
    parts.append(lk)
    p, _ = t("RapidQ-compatible BASIC, built new in Rust.", 32, 82, 252, WHITE, 650, -10)
    parts.append(p)
    p, _ = t("Native compiler, bytecode runtime, web output and an IDE.", 19, 82, 292, MIST, 450)
    parts.append(p)
    p, _ = t("macOS  ·  Windows  ·  Linux  ·  the browser", 19, 82, 322, MIST, 450)
    parts.append(p)
    card, ch = code_card(760, 0, 440)
    parts.append(f'<g transform="translate(0 {_num((H - ch) / 2)})">{card}</g>')
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="RapidR")


def social():
    W, H = 1280, 640
    bg, defs = background(W, H)
    parts = [bg]
    lk, _ = place_lockup("dark", 96, 132, 112)
    parts.append(lk)
    p, _ = t("RapidQ-compatible BASIC,", 46, 98, 340, WHITE, 650, -15)
    parts.append(p)
    p, _ = t("built new in Rust.", 46, 98, 396, WHITE, 650, -15)
    parts.append(p)
    p, _ = t("Native compiler · bytecode runtime · web · IDE", 21, 98, 446, MIST, 450)
    parts.append(p)
    # platform chips
    x = 98
    for label in ("macOS", "Windows", "Linux", "Web"):
        d, w = text_path(label, 17, 0, 0, weight=600)
        cw = w + 36
        parts.append(f'<path fill="#FFFFFF" fill-opacity="0.08" d="{rrect(x, 482, cw, 36, 18)}"/>')
        parts.append(f'<path fill="none" stroke="#FFFFFF" stroke-opacity="0.16" stroke-width="1.5" d="{rrect(x, 482, cw, 36, 18)}"/>')
        p, _ = t(label, 17, x + 18, 506, TXT, 600)
        parts.append(p)
        x += cw + 12
    card, ch = code_card(716, 0, 468, size=17, lh=29)
    parts.append(f'<g transform="translate(0 {_num((H - ch) / 2)})">{card}</g>')
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="RapidR")


def main():
    open(os.path.join(OUT, "banner.svg"), "w").write(banner())
    open(os.path.join(OUT, "social-preview.svg"), "w").write(social())


if __name__ == "__main__":
    main()
