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


def end_card(line1="Smarter together.", line3="Free and open source (MIT)  ·  github.com/iBobX/RapidR", line4="Also works with free local models for simple apps."):
    W, H = 1920, 1080
    bg, defs = background(W, H)
    parts = [bg, centred_lockup(W / 2, 260, 120)]
    p, _ = t(line1, 76, W / 2, 540, WHITE, 700, -15, anchor="middle")
    parts.append(p)
    p, _ = t("rapidr.dev", 54, W / 2, 640, CYAN, 650, anchor="middle")
    parts.append(p)
    p, _ = t(line3, 30, W / 2, 720, MIST, 450, anchor="middle")
    parts.append(p)
    p, _ = t(line4, 26, W / 2, 900, MIST, 400, anchor="middle")
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


def badge(text):
    """A small speed badge, top right ("8x faster")."""
    W, H = 1920, 1080
    _, tw = t(text, 30, 0, 0, WHITE, 650)
    bw, bh = tw + 56, 58
    x, y = W - bw - 48, 40
    parts = [f'<path fill="#0B1222" fill-opacity="0.82" d="{rrect(x, y, bw, bh, 29)}"/>']
    p, _ = t(text, 30, x + bw / 2, y + 39, WHITE, 650, anchor="middle")
    parts.append(p)
    return svg_doc("".join(parts), W, H, title="badge")


def word_card(text, size=150, accent=False):
    """One big word, centred, on the brand background (the intro's beats)."""
    W, H = 1920, 1080
    bg, defs = background(W, H)
    p, _ = t(text, size, W / 2, H / 2 + size * 0.35, CYAN if accent else WHITE, 750, -20, anchor="middle")
    return svg_doc(bg + p, W, H, defs=defs + grad(), title="word")


def blueprint(code_path, W=3000, H=1700):
    """The intro's backdrop: a blueprint grid with real RapidR code, faint."""
    from brandkit import MONO
    lines = [l.rstrip() for l in open(code_path).read().splitlines()][:120]
    parts = [f'<rect width="{W}" height="{H}" fill="#071530"/>']
    for x in range(0, W, 40):
        a = 0.10 if x % 200 == 0 else 0.04
        parts.append(f'<rect x="{x}" y="0" width="1.5" height="{H}" fill="#5FB4FF" fill-opacity="{a}"/>')
    for y in range(0, H, 40):
        a = 0.10 if y % 200 == 0 else 0.04
        parts.append(f'<rect x="0" y="{y}" width="{W}" height="1.5" fill="#5FB4FF" fill-opacity="{a}"/>')
    col_w, lh, size = 1000, 34, 22
    per_col = int((H - 120) / lh)
    for i, line in enumerate(lines[: per_col * 3]):
        if not line.strip():
            continue
        c, r = divmod(i, per_col)
        p, _ = t(line[:70], size, 90 + c * col_w, 110 + r * lh, "#9FD0FF", 400, font=MONO)
        parts.append(p.replace('<path ', '<path fill-opacity="0.16" '))
    defs = (f'<radialGradient id="vig" cx="0.5" cy="0.5" r="0.7"><stop offset="0.55" stop-color="#000" stop-opacity="0"/>'
            f'<stop offset="1" stop-color="#000" stop-opacity="0.65"/></radialGradient>')
    parts.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg_doc("".join(parts), W, H, defs=defs, title="blueprint")


def thumbnail_overlay(l1="Describe it.", l2="RAI builds it.", sub="3 apps · 3 sentences · real code", size=92):
    """YouTube thumbnail text (1280 x 720, transparent right side for a frame)."""
    W, H = 1280, 720
    defs = ('<linearGradient id="fadeL" x1="0" x2="1" y1="0" y2="0">'
            '<stop offset="0" stop-color="#050C1C" stop-opacity="0.97"/><stop offset="0.55" stop-color="#050C1C" stop-opacity="0.85"/>'
            '<stop offset="1" stop-color="#050C1C" stop-opacity="0"/></linearGradient>')
    parts = [f'<rect width="{W * 0.72}" height="{H}" fill="url(#fadeL)"/>']
    lk, _ = place_lockup("dark", 64, 56, 64)
    parts.append(lk)
    for i, (line, col) in enumerate(((l1, WHITE), (l2, CYAN))):
        p, _ = t(line, size, 60, 300 + i * 112, col, 800, -25)
        parts.append(p)
    p, _ = t(sub, 34, 64, 560, MIST, 500)
    parts.append(p)
    return svg_doc("".join(parts), W, H, defs=defs + grad(), title="thumbnail")


def main():
    if len(sys.argv) >= 4 and sys.argv[1] == "word":
        out = sys.argv[3]
        tmp = out[:-4] + ".svg"
        open(tmp, "w").write(word_card(sys.argv[2], accent=len(sys.argv) > 4))
        subprocess.run(["rsvg-convert", "-w", "1920", "-h", "1080", "-o", out, tmp], check=True)
        os.remove(tmp)
        return
    if len(sys.argv) >= 4 and sys.argv[1] == "badge":
        out = sys.argv[3]
        tmp = out[:-4] + ".svg"
        open(tmp, "w").write(badge(sys.argv[2]))
        subprocess.run(["rsvg-convert", "-w", "1920", "-h", "1080", "-o", out, tmp], check=True)
        os.remove(tmp)
        return
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
