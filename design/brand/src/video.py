"""Writes the video and channel art into video/: the YouTube channel banner
(2560 x 1440, the logo and words inside the 1546 x 423 area every device
shows), the title and end cards (1920 x 1080) and, on demand, caption
cards (a lower third, transparent around it) for the demo videos.

    python3 design/brand/src/video.py                 # banner, title, end
    python3 design/brand/src/video.py caption "One sentence." out.png

All text is outlined (Inter), as the GitHub art is; PNGs come from the SVG
with rsvg-convert."""
import os
import subprocess
import sys

from brandkit import BRAND, WHITE, MIST, CYAN, svg_doc, rrect
from github import t, background, place_lockup, TXT
from logo import grad

OUT = os.path.join(BRAND, "video")
os.makedirs(OUT, exist_ok=True)


def png(svg, name, w, h):
    path = os.path.join(OUT, name + ".svg")
    open(path, "w").write(svg)
    subprocess.run(["rsvg-convert", "-w", str(w), "-h", str(h), "-o", os.path.join(OUT, name + ".png"), path], check=True)


def centred_lockup(cx, y, height):
    _, w = place_lockup("dark", 0, 0, height)
    lk, _ = place_lockup("dark", cx - w / 2, y, height)
    return lk


def youtube_banner():
    W, H = 2560, 1440
    bg, defs = background(W, H)
    parts = [bg, centred_lockup(W / 2, 560, 120)]
    p, _ = t("BASIC is powerful again.", 54, W / 2, 790, WHITE, 650, -10, anchor="middle")
    parts.append(p)
    p, _ = t("RapidR Studio and RAI  ·  macOS  ·  Windows  ·  Linux  ·  the browser", 30, W / 2, 850, MIST, 450, anchor="middle")
    parts.append(p)
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="RapidR")


def title_card(line1, line2):
    W, H = 1920, 1080
    bg, defs = background(W, H)
    parts = [bg, centred_lockup(W / 2, 300, 110)]
    p, _ = t(line1, 72, W / 2, 560, WHITE, 700, -15, anchor="middle")
    parts.append(p)
    p, _ = t(line2, 38, W / 2, 640, MIST, 450, anchor="middle")
    parts.append(p)
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="RapidR")


def end_card():
    W, H = 1920, 1080
    bg, defs = background(W, H)
    parts = [bg, centred_lockup(W / 2, 260, 120)]
    p, _ = t("Smarter together.", 76, W / 2, 540, WHITE, 700, -15, anchor="middle")
    parts.append(p)
    p, _ = t("rapidr.dev", 54, W / 2, 640, CYAN, 650, anchor="middle")
    parts.append(p)
    p, _ = t("Free and open source (MIT)  ·  github.com/iBobX/RapidR", 30, W / 2, 720, MIST, 450, anchor="middle")
    parts.append(p)
    p, _ = t("Also works with free local models for simple apps.", 26, W / 2, 900, MIST, 400, anchor="middle")
    parts.append(p)
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="RapidR")


def caption(text, sub=""):
    """A lower third: a rounded dark card near the bottom, centred."""
    W, H = 1920, 1080
    size = 46
    _, tw = t(text, size, 0, 0, WHITE, 650)
    sw = t(sub, 28, 0, 0, MIST, 450)[1] if sub else 0
    cw = max(tw, sw) + 150
    ch = 104 if not sub else 150
    x, y = (W - cw) / 2, H - ch - 70
    parts = [f'<path fill="#0B1222" fill-opacity="0.86" d="{rrect(x, y, cw, ch, 24)}"/>',
             f'<path fill="none" stroke="#FFFFFF" stroke-opacity="0.14" stroke-width="2" d="{rrect(x, y, cw, ch, 24)}"/>',
             f'<rect x="{x + 28}" y="{y + 26}" width="6" height="{ch - 52}" rx="3" fill="{CYAN}"/>']
    p, _ = t(text, size, W / 2, y + 68, WHITE, 650, anchor="middle")
    parts.append(p)
    if sub:
        p, _ = t(sub, 28, W / 2, y + 116, TXT, 450, anchor="middle")
        parts.append(p)
    return svg_doc("".join(parts), W, H, title="caption")


def main():
    if len(sys.argv) >= 4 and sys.argv[1] == "caption":
        sub = sys.argv[4] if len(sys.argv) > 4 else ""
        out = sys.argv[3]
        svg = caption(sys.argv[2], sub)
        tmp = out[:-4] + ".svg"
        open(tmp, "w").write(svg)
        subprocess.run(["rsvg-convert", "-w", "1920", "-h", "1080", "-o", out, tmp], check=True)
        os.remove(tmp)
        return
    png(youtube_banner(), "youtube-banner", 2560, 1440)
    png(title_card("Describe it. RAI builds it.", "RapidR Studio  ·  BASIC is powerful again"), "title", 1920, 1080)
    png(end_card(), "end", 1920, 1080)


if __name__ == "__main__":
    main()
