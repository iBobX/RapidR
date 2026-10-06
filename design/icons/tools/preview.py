"""Quick previews while drawing (dev tool; needs rsvg-convert):

    python3 design/icons/tools/preview.py OUT.png [--cat actions] [--names a,b] [--theme modern]
                                          [--scale 1] [--zoom 4]

One row per icon set, each icon at 16, 24 and 32 px (the hinted variants
the renderers pick at that device size), optionally magnified with
nearest-neighbour pixels (`--zoom`) to check the hinting.
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kit  # noqa: E402
import catalog  # noqa: E402,F401  (registers every icon)
from palette import THEMES, themed  # noqa: E402


def variant_for(device):
    """The hinted size the renderers draw for `device` pixels, and its scale
    (crates/rapidr-icons: exact size, else a whole multiple, else the
    nearest to a whole multiple)."""
    if device in kit.SIZES:
        return device, 1.0
    best = None
    for v in sorted(kit.SIZES, reverse=True):
        r = device / v
        err = abs(r - round(r)) if r >= 1 else 1 + (1 - r)
        if best is None or err < best[0] - 1e-9:
            best = (err, v)
    return best[1], device / best[1]


def nested(svg, x, y, px, prefix, fg):
    """`svg` placed at (x, y), `px` pixels wide, its ids made unique."""
    svg = re.sub(r'id="([^"]+)"', lambda m: f'id="{prefix}{m.group(1)}"', svg)
    svg = re.sub(r'url\(#([^)]+)\)', lambda m: f'url(#{prefix}{m.group(1)})', svg)
    svg = re.sub(r"<title>.*?</title>", "", svg, flags=re.S)
    vb = re.search(r'viewBox="0 0 (\d+) (\d+)"', svg).group(1)
    return svg.replace(f'width="{vb}" height="{vb}"', f'x="{x}" y="{y}" width="{px}" height="{px}" color="{fg}"', 1)


def sheet(names, theme, scale, zoom, cols=None, label=True):
    tokens, fg, bgs = THEMES[theme]
    bg = bgs[0]
    text = "#1B1B1B" if theme in ("classic", "modern") else "#FFFFFF"
    logical = (16, 24, 32)
    cell_w = int(sum(int(s * scale) for s in logical) * zoom + 40)
    cell_h = int(32 * scale * zoom + (22 if label else 6))
    cols = cols or max(1, 1500 // cell_w)
    rows = (len(names) + cols - 1) // cols
    W, H = cols * cell_w + 20, rows * cell_h + 20
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
             f'<rect width="{W}" height="{H}" fill="{bg}"/>']
    n = 0
    for i, name in enumerate(names):
        cx, cy = 10 + (i % cols) * cell_w, 10 + (i // cols) * cell_h
        x = cx
        for s in logical:
            device = int(round(s * scale))
            v, k = variant_for(device)
            src = themed(kit.draw(name, v), theme)
            n += 1
            parts.append(nested(src, x * 1, cy, device, f"i{n}_", fg))
            x += device * zoom + 6
        if label:
            parts.append(f'<text x="{cx}" y="{cy + 32 * scale * zoom + 14}" font-family="Inter, Helvetica" font-size="11" fill="{text}">{name.split("/", 1)[1]}</text>')
    parts.append("</svg>")
    return "".join(parts), W, H


def render(svg_text, out, zoom=1):
    with tempfile.NamedTemporaryFile("w", suffix=".svg", delete=False) as f:
        f.write(svg_text)
        p = f.name
    subprocess.run(["rsvg-convert", p, "-o", out], check=True)
    os.unlink(p)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--cat")
    ap.add_argument("--names")
    ap.add_argument("--theme", default="modern")
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--zoom", type=int, default=1)
    ap.add_argument("--cols", type=int)
    a = ap.parse_args()
    names = [n for n, s in kit.ICONS.items() if (not a.cat or s["cat"] == a.cat)]
    if a.names:
        names = [n if "/" in n else next(k for k in kit.ICONS if k.endswith("/" + n)) for n in a.names.split(",") if n]
    svg, W, H = sheet(names, a.theme, a.scale, 1, a.cols)
    render(svg, a.out)
    if a.zoom > 1:
        from PIL import Image
        im = Image.open(a.out)
        im = im.resize((im.width * a.zoom, im.height * a.zoom), Image.NEAREST)
        im.save(a.out)
    print(a.out, len(names), "icons")


if __name__ == "__main__":
    main()
