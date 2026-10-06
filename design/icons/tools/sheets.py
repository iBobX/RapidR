#!/usr/bin/env python3
"""Review contact sheets (dev tool): every icon of a category at 16, 24 and
32 px, in the four themes side by side, at 1x or 2x.

    python3 design/icons/tools/sheets.py OUTDIR [--cats actions,components] [--scales 1,2]
                                                [--renderer rust|rsvg] [--names a,b]

With `--renderer rust` (the default) the pixels are crates/rapidr-icons'
own (resvg, through `cargo run -p rapidr-icons --example render`): the
variant the runtime picks for each device size, themed as the runtime does.
`rsvg` composes the sheet with rsvg-convert instead (no build needed).
Writes OUTDIR/<category>-<scale>x.png.
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import kit  # noqa: E402
import catalog  # noqa: E402,F401
from palette import THEMES, themed  # noqa: E402
from preview import variant_for, nested  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
THEME_ORDER = ("classic", "modern", "dark", "highcontrast")
LABEL = {"classic": "Classic", "modern": "Modern", "dark": "Dark", "highcontrast": "High contrast"}
LOGICAL = (16, 24, 32)


def text_color(theme):
    return {"classic": "#1B1B1B", "modern": "#1B1B1B", "dark": "#E8E8E8", "highcontrast": "#FFFFFF"}[theme]


def sub_color(theme):
    return {"classic": "#6B6B6B", "modern": "#6B6B6B", "dark": "#9A9A9A", "highcontrast": "#3FF23F"}[theme]


def layout(keys, scale, cols):
    cell_w = int(sum(s * scale for s in LOGICAL) + 2 * 8 * scale + 18 * scale)
    cell_w = max(cell_w, int(118 * scale))
    cell_h = int(32 * scale + 26 * scale)
    rows = (len(keys) + cols - 1) // cols
    panel_w = cols * cell_w + int(24 * scale)
    panel_h = rows * cell_h + int(56 * scale)
    return cell_w, cell_h, rows, panel_w, panel_h


def rust_pngs(keys, scale, out):
    """Renders every (icon, logical size, theme) through crates/rapidr-icons
    (its `render` example) into `out`."""
    p = os.path.join(out, "request.txt")
    with open(p, "w") as f:
        f.write(f"{scale}\n{' '.join(map(str, LOGICAL))}\n{' '.join(THEME_ORDER)}\n{out}\n" + "\n".join(keys) + "\n")
    subprocess.run(["cargo", "run", "-q", "--release", "-p", "rapidr-icons", "--example", "render", "--", p],
                   cwd=ROOT, check=True)


def sheet(cat, keys, scale, outdir, renderer):
    cols = 6 if cat != "glyphs" else 8
    cell_w, cell_h, rows, panel_w, panel_h = layout(keys, scale, cols)
    W, H = panel_w * len(THEME_ORDER), panel_h
    fs = 11 * scale
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{W}" height="{H}" viewBox="0 0 {W} {H}">']
    tmp = tempfile.mkdtemp()
    if renderer == "rust":
        rust_pngs(keys, scale, tmp)
    n = 0
    for ti, theme in enumerate(THEME_ORDER):
        tokens, fg, bgs = THEMES[theme]
        x0 = ti * panel_w
        parts.append(f'<rect x="{x0}" y="0" width="{panel_w}" height="{H}" fill="{bgs[0]}"/>')
        parts.append(f'<text x="{x0 + 12 * scale}" y="{24 * scale}" font-family="Inter, Helvetica, Arial" font-weight="600" '
                     f'font-size="{14 * scale}" fill="{text_color(theme)}">{LABEL[theme]} · {cat} · 16 / 24 / 32 px at {scale}x</text>')
        for i, key in enumerate(keys):
            cx = x0 + int(12 * scale) + (i % cols) * cell_w
            cy = int(40 * scale) + (i // cols) * cell_h
            x = cx
            for s in LOGICAL:
                device = int(round(s * scale))
                y = cy + int(32 * scale) - device
                if renderer == "rust":
                    png = os.path.join(tmp, f"{key.replace('/', '__')}_{s}_{theme}.png")
                    parts.append(f'<image x="{x}" y="{y}" width="{device}" height="{device}" xlink:href="file://{png}" style="image-rendering:pixelated"/>')
                else:
                    v, _ = variant_for(device)
                    n += 1
                    parts.append(nested(themed(kit.draw(key, v), theme), x, y, device, f"i{n}_", fg))
                x += device + int(8 * scale)
            name = key.split("/", 1)[1]
            parts.append(f'<text x="{cx}" y="{cy + 32 * scale + 15 * scale}" font-family="Inter, Helvetica, Arial" '
                         f'font-size="{fs}" fill="{sub_color(theme)}">{name}</text>')
    parts.append("</svg>")
    svg_path = os.path.join(tmp, "sheet.svg")
    open(svg_path, "w").write("".join(parts))
    out = os.path.join(outdir, f"{cat}-{int(scale) if scale == int(scale) else scale}x.png")
    subprocess.run(["rsvg-convert", svg_path, "-o", out], check=True)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("outdir")
    ap.add_argument("--cats", default=",".join(c for c in ("actions", "glyphs", "components", "files", "symbols", "groups")))
    ap.add_argument("--scales", default="1,2")
    ap.add_argument("--renderer", default="rust", choices=("rust", "rsvg"))
    ap.add_argument("--names")
    ap.add_argument("--tag", default="")
    a = ap.parse_args()
    os.makedirs(a.outdir, exist_ok=True)
    for cat in a.cats.split(","):
        keys = [k for k, s in kit.ICONS.items() if s["cat"] == cat]
        if a.names:
            keys = [k for k in keys if k.split("/", 1)[1] in a.names.split(",")]
        if not keys:
            continue
        for sc in a.scales.split(","):
            out = sheet_named(cat, a.tag, keys, float(sc) if "." in sc else int(sc), a.outdir, a.renderer)
            print(out)


def sheet_named(cat, tag, keys, scale, outdir, renderer):
    out = sheet(cat, keys, scale, outdir, renderer)
    if tag:
        new = out.replace(f"{cat}-", f"{cat}{tag}-")
        os.replace(out, new)
        return new
    return out


if __name__ == "__main__":
    main()
