"""Builds every binary asset from the SVG masters (the source of truth).

Needs only permissive tools: rsvg-convert (librsvg, LGPL tool; output
carries no obligation), macOS's built-in iconutil, and Pillow (HPND).
The .ico writer is our own (PNG-compressed entries, Windows Vista+).

    python3 design/brand/src/export.py
"""
import io
import os
import shutil
import struct
import subprocess
import tempfile

from PIL import Image

BRAND = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SVG = os.path.join(BRAND, "icons", "svg")
ICONS = os.path.join(BRAND, "icons")

# icon id -> (master for large sizes on macOS, master for large sizes elsewhere)
APPS = {
    "ide": ("app-ide-macos", "app-ide-full"),
    "runtime": ("app-runtime-macos", "app-runtime-full"),
    "program": ("app-program-macos", "app-program-full"),
}
FILES = ("rr", "bas", "rrbc")
HINTED = (16, 20, 22, 24, 32)

# Names the release packaging uses.
MAC_NAMES = {"ide": "RapidR", "runtime": "RapidR-Runtime", "program": "RapidR-App",
             "rr": "RapidR-Source", "bas": "BASIC-Source", "rrbc": "RapidR-Program"}
WIN_NAMES = {"ide": "rapidr-ide", "runtime": "rapidr-runtime", "program": "rapidr-app",
             "rr": "rapidr-source", "bas": "basic-source", "rrbc": "rapidr-program"}
LINUX = {"ide": ("apps", "rapidr-ide"), "runtime": ("apps", "rapidr-runtime"), "program": ("apps", "rapidr-app"),
         "rr": ("mimetypes", "text-x-rapidr"), "bas": ("mimetypes", "text-x-rapidq-basic"),
         "rrbc": ("mimetypes", "application-x-rapidr-bytecode")}


def render(svg_name, px, out=None):
    """Render an SVG master to an RGBA PIL image at px x px."""
    src = os.path.join(SVG, svg_name + ".svg")
    data = subprocess.run(["rsvg-convert", "-w", str(px), "-h", str(px), src],
                          check=True, capture_output=True).stdout
    im = Image.open(io.BytesIO(data)).convert("RGBA")
    if out:
        os.makedirs(os.path.dirname(out), exist_ok=True)
        save_png(im, out)
    return im


def save_png(im, path):
    im.save(path, "PNG", optimize=True)


def source_for(icon, px, platform):
    """Which master draws `icon` at px on `platform` (macos | other)."""
    if icon in APPS:
        if platform == "macos" and px == 32:
            return f"app-{icon}-mac32"
        if px in HINTED:
            return f"app-{icon}-{px}"
        return APPS[icon][0 if platform == "macos" else 1]
    if px in HINTED:
        return f"file-{icon}-{px}"
    return f"file-{icon}"


def build_icns(icon):
    tmp = tempfile.mkdtemp()
    iconset = os.path.join(tmp, "x.iconset")
    os.makedirs(iconset)
    for base in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            px = base * scale
            name = f"icon_{base}x{base}{'@2x' if scale == 2 else ''}.png"
            save_png(render(source_for(icon, px, "macos"), px), os.path.join(iconset, name))
    out = os.path.join(ICONS, "macos", MAC_NAMES[icon] + ".icns")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)
    shutil.rmtree(tmp)


def write_ico(images, path):
    """images: list of RGBA PIL images (distinct sizes). PNG-compressed entries."""
    blobs = []
    for im in images:
        b = io.BytesIO()
        im.save(b, "PNG", optimize=True)
        blobs.append(b.getvalue())
    n = len(images)
    header = struct.pack("<HHH", 0, 1, n)
    offset = 6 + 16 * n
    entries = b""
    for im, blob in zip(images, blobs):
        w, h = im.size
        entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(header + entries + b"".join(blobs))


def build_ico(icon):
    sizes = (16, 20, 24, 32, 40, 48, 64, 256)
    ims = [render(source_for(icon, s, "windows"), s) for s in sizes]
    write_ico(ims, os.path.join(ICONS, "windows", WIN_NAMES[icon] + ".ico"))


def build_linux(icon):
    ctx, name = LINUX[icon]
    root = os.path.join(ICONS, "linux", "hicolor")
    for s in (16, 22, 24, 32, 48, 64, 128, 256, 512):
        render(source_for(icon, s, "linux"), s, os.path.join(root, f"{s}x{s}", ctx, name + ".png"))
    sc = os.path.join(root, "scalable", ctx)
    os.makedirs(sc, exist_ok=True)
    master = APPS[icon][1] if icon in APPS else f"file-{icon}"
    shutil.copyfile(os.path.join(SVG, master + ".svg"), os.path.join(sc, name + ".svg"))


def build_web():
    web = os.path.join(ICONS, "web")
    os.makedirs(web, exist_ok=True)
    shutil.copyfile(os.path.join(SVG, "app-ide-32.svg"), os.path.join(web, "favicon.svg"))
    render("app-ide-32", 32, os.path.join(web, "favicon-32.png"))
    # iOS masks it itself and wants it opaque
    save_png(render("web-touch-icon", 180).convert("RGB"), os.path.join(web, "apple-touch-icon.png"))
    render("app-ide-full", 512, os.path.join(web, "icon-512.png"))
    # the web IDE serves its own copies (web.sh bundles web-ide's tracked files)
    site = os.path.join(os.path.dirname(os.path.dirname(BRAND)), "web-ide", "icons")
    os.makedirs(site, exist_ok=True)
    for f in ("favicon.svg", "favicon-32.png", "apple-touch-icon.png"):
        shutil.copyfile(os.path.join(web, f), os.path.join(site, f))


def build_logo_pngs():
    logo = os.path.join(BRAND, "logo")
    out = os.path.join(logo, "png")
    os.makedirs(out, exist_ok=True)
    for name, h in [("rapidr-mark", 512), ("rapidr-lockup", 160), ("rapidr-lockup-dark", 160)]:
        subprocess.run(["rsvg-convert", "-h", str(h), os.path.join(logo, name + ".svg"),
                        "-o", os.path.join(out, f"{name}-{h}.png")], check=True)


def build_github():
    gh = os.path.join(BRAND, "github")
    for name in ("banner", "social-preview"):
        subprocess.run(["rsvg-convert", os.path.join(gh, name + ".svg"), "-o",
                        os.path.join(gh, name + ".png")], check=True)


def build_preview():
    """icons/preview.png: every icon at every size, hinted sizes magnified."""
    rows = list(APPS) + list(FILES)
    Z = 4
    sizes = (16, 20, 22, 24, 32, 48, 64, 128, 256)
    W = 40 + sum(sizes) + 20 * len(sizes) + 5 * (32 * Z + 20) - 20
    row_h = 256 + 40
    sheet = Image.new("RGBA", (W + 40, 40 + row_h * len(rows)), "#E9EDF5")
    y = 30
    for icon in rows:
        x = 30
        for s in HINTED:
            im = render(source_for(icon, s, "other"), s)
            z = im.resize((s * Z, s * Z), Image.NEAREST)
            sheet.alpha_composite(z, (x, y + (256 - s * Z) // 2))
            x += 32 * Z + 20
        for s in sizes:
            im = render(source_for(icon, s, "other"), s)
            sheet.alpha_composite(im, (x, y + (256 - s) // 2))
            x += s + 20
        y += row_h
    save_png(sheet.convert("RGB"), os.path.join(ICONS, "preview.png"))


def main():
    for icon in list(APPS) + list(FILES):
        build_icns(icon)
        build_ico(icon)
        build_linux(icon)
    build_web()
    build_logo_pngs()
    build_github()
    build_preview()


if __name__ == "__main__":
    main()
