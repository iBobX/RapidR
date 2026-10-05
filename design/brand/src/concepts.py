"""Writes the three logo concepts (concepts/*.svg) and the comparison sheet
(concepts.svg -> concepts.png). Labels are outlined Inter, so the sheet
renders identically everywhere."""
import os
import subprocess

from brandkit import (BRAND, INK, PAPER, BLUE, CYAN, WHITE, SLATE, MIST, svg_doc,
                      text_path, transform, squircle, _num)
from marks import concept_a, concept_b_svg, run_r_centered

OUT = os.path.join(BRAND, "concepts")
os.makedirs(OUT, exist_ok=True)


def grad(gid, x1=10, y1=90, x2=90, y2=10, a=BLUE, b=CYAN):
    return (f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="{x1}" y1="{y1}" '
            f'x2="{x2}" y2="{y2}"><stop offset="0" stop-color="{a}"/><stop offset="1" '
            f'stop-color="{b}"/></linearGradient>')


TILE = squircle(0, 0, 100, 100, 32)


def mark(concept, style, gid):
    """Return SVG body (100 unit box) for a concept in a style:
    colour-light, colour-dark, mono-ink, mono-white."""
    colour = style.startswith("colour")
    fg = f"url(#{gid})" if colour else (INK if style == "mono-ink" else WHITE)
    if concept == "a":
        return f'<path fill="{fg}" d="{concept_a()}"/>'
    if concept == "b":
        return concept_b_svg(fg)
    # c: tile + knocked-out R
    if colour:
        tile, glyph = f"url(#{gid})", WHITE
    elif style == "mono-ink":
        tile, glyph = INK, PAPER
    else:
        tile, glyph = WHITE, INK
    return f'<path fill="{tile}" d="{TILE}"/><path fill="{glyph}" d="{run_r_centered()}"/>'


NAMES = {
    "a": ("A  Velocity", "A forward-leaning R trailing three speed bars."),
    "b": ("B  Prompt", "The R as a command prompt: chevron bowl, block cursor."),
    "c": ("C  Run", "A constructed R whose counter is a play triangle, on a tile."),
}
FILES = {"a": "concept-a-velocity", "b": "concept-b-prompt", "c": "concept-c-run"}


def write_concept_files():
    for c, base in FILES.items():
        # 4-up: colour/light, colour/dark, mono ink, mono white
        cells = []
        for i, (style, bg) in enumerate([("colour-light", PAPER), ("colour-dark", INK),
                                         ("mono-ink", PAPER), ("mono-white", INK)]):
            gid = f"g{i}"
            x = i * 120
            cells.append(f'<g transform="translate({x} 0)"><defs>{grad(gid)}</defs>'
                         f'<rect width="120" height="120" fill="{bg}"/>'
                         f'<g transform="translate(10 10)">{mark(c, style, gid)}</g></g>')
        open(os.path.join(OUT, base + ".svg"), "w").write(
            svg_doc("".join(cells), 480, 120, title=f"RapidR logo concept {NAMES[c][0]}"))


def label(text, size, x, y, fill, weight=600, tracking=0):
    d, w = text_path(text, size, x, y, weight=weight, tracking=tracking)
    return f'<path fill="{fill}" d="{d}"/>', w


def sheet():
    W, H = 1850, 1010
    col_w = 560
    x0 = 60
    body = [f'<rect width="{W}" height="{H}" fill="#E9EDF5"/>']
    t, _ = label("RapidR logo concepts", 40, x0, 86, INK, 700, -10)
    body.append(t)
    t, _ = label("Each concept in full colour and monochrome, on light and dark, and at real app-icon sizes "
                 "(64, 32 and 16 px, then the 16 px render magnified 3x).", 19, x0, 124, SLATE, 450)
    body.append(t)
    defs = []
    for ci, c in enumerate("abc"):
        cx = x0 + ci * (col_w + 30)
        top = 170
        t, _ = label(NAMES[c][0], 28, cx, top + 28, INK, 700, -5)
        body.append(t)
        t, _ = label(NAMES[c][1], 17, cx, top + 56, SLATE, 450)
        body.append(t)
        # 2x2 big tiles
        k = 0
        for r, row in enumerate([[("colour-light", PAPER), ("colour-dark", INK)],
                                 [("mono-ink", PAPER), ("mono-white", INK)]]):
            for q, (style, bg) in enumerate(row):
                gid = f"s{c}{k}"
                k += 1
                tx, ty = cx + q * 280, top + 80 + r * 280
                defs.append(grad(gid))
                body.append(f'<g transform="translate({tx} {ty})">'
                            f'<rect width="270" height="270" rx="18" fill="{bg}"/>'
                            f'<g transform="translate(35 35) scale(2)">{mark(c, style, gid)}</g></g>')
        # small sizes strip
        sy = top + 80 + 560 + 10
        for q, (style, bg) in enumerate([("colour-light", PAPER), ("colour-dark", INK)]):
            tx = cx + q * 280
            body.append(f'<rect x="{tx}" y="{sy}" width="270" height="100" rx="18" fill="{bg}"/>')
            gx = tx + 18
            for s in (64, 32, 16):
                gid = f"m{c}{q}{s}"
                defs.append(grad(gid))
                body.append(f'<g transform="translate({gx} {sy + 18}) scale({s / 100})">'
                            f'{mark(c, style, gid)}</g>')
                gx += s + 16
            body.append(f'<image x="{gx}" y="{sy + 18}" width="48" height="48" '
                        f'href="__ZOOM_{c}_{q}__"/>')
    note, _ = label("Recommended: C  Run. It is the clearest at 16 px, it is a ready-made app icon, and the play-"
                    "triangle counter says what RapidR does.", 19, x0, H - 40, INK, 550)
    body.append(note)
    return svg_doc("".join(body), W, H, defs="".join(defs), title="RapidR logo concepts")


def render(svg_path, png_path, w=None):
    args = ["rsvg-convert", svg_path, "-o", png_path]
    if w:
        args[1:1] = ["-w", str(w)]
    subprocess.run(args, check=True)


def main():
    import base64
    import io
    from PIL import Image
    write_concept_files()
    s = sheet()
    tmp = os.path.join(OUT, "_tmp")
    os.makedirs(tmp, exist_ok=True)
    # true 16 px renders, magnified 3x with nearest-neighbour, embedded in the sheet
    for c in "abc":
        for q, (style, bg) in enumerate([("colour-light", PAPER), ("colour-dark", INK)]):
            p = os.path.join(tmp, f"z{c}{q}.svg")
            open(p, "w").write(svg_doc(f'<defs>{grad("g")}</defs><rect width="100" height="100" fill="{bg}"/>'
                                       + mark(c, style, "g"), 100, 100))
            render(p, p + ".png", 16)
            im = Image.open(p + ".png").convert("RGB").resize((48, 48), Image.NEAREST)
            buf = io.BytesIO()
            im.save(buf, "PNG")
            s = s.replace(f"__ZOOM_{c}_{q}__", "data:image/png;base64," + base64.b64encode(buf.getvalue()).decode())
    sp = os.path.join(tmp, "concepts.svg")
    open(sp, "w").write(s)
    render(sp, os.path.join(BRAND, "concepts.png"))
    for f in os.listdir(tmp):
        os.remove(os.path.join(tmp, f))
    os.rmdir(tmp)


if __name__ == "__main__":
    main()
