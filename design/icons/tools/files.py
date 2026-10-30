"""File types and folders: the brand's page (design/brand: a sheet with its
top-right corner folded) in ink on paper, and an emblem in the type's hue.
RapidR's own files carry the brand's R: `.rr` (blue), `.rrbc` (its counter
lit in Run Amber, as the Runtime's icon), `.rrproj` (on a folder)."""

from kit import icon
from motifs import arrowhead, page, folder, folder_open, star4, cube_iso
from planned import rglyph

F = "files"


def sheet(g):
    page(g, 4.5, 2, 15, 20, fold=5, c="ink", fill="paper")


@icon("file", F, "File")
def file_(g):
    sheet(g)


@icon("folder", F, "Folder")
def folder_(g):
    folder(g, 2.5, 4, 19, 15, c="ink", fill="blue-tint")


@icon("folder-open", F, "Folder (open)")
def folder_open_(g):
    g.path("M2.5 19 V5 H8.5 L10.5 7 H17.5 V10", c="ink", fill="blue-tint")
    g.path("M2.5 19 L6 11 H22 L18.5 19 Z", c="ink", fill="blue-tint")


@icon("rr", F, "RapidR source (.rr)")
def rr(g):
    sheet(g)
    rglyph(g, 7.5, 9, 10.5)


@icon("rrbc", F, "RapidR program (.rrbc)")
def rrbc(g):
    sheet(g)
    rglyph(g, 7.5, 9, 10.5, counter="amber-solid")


@icon("rrproj", F, "RapidR project (.rrproj)")
def rrproj(g):
    folder(g, 2.5, 4, 19, 15, c="blue", fill="blue-tint")
    rglyph(g, 9.5, 10.5, 7)


@icon("rrext", F, "RapidR extension (.rrext)")
def rrext(g):
    # a puzzle piece
    g.path("M4 8 H8 A2.5 2.5 0 1 1 13 8 H17 V12 A2.5 2.5 0 1 1 17 17 V20.5 H4 Z", c="blue", fill="blue-tint")


@icon("bas", F, "BASIC source (.bas)")
def bas(g):
    sheet(g)
    for y in ((9.5, 13, 16.5) if g.size != 16 else (9, 13, 17)):
        g.frect(7.25, y - 1, 2, 2, fill="teal")
        g.line(11.5, y, 16 if y != 13 else 15, y, c="ink")


@icon("inc", F, "Include file (.inc)")
def inc(g):
    sheet(g)
    g.path("M8 9 V13.5 A2 2 0 0 0 10 15.5 H15.5", c="teal")
    arrowhead(g, 16, 15.5, 1, 0, 3, c="teal")


@icon("text", F, "Text")
def text(g):
    sheet(g)
    g.line(8, 10, 15.5, 10, c="ink")
    g.line(8, 13.5, 15.5, 13.5, c="ink")
    g.line(8, 17, 13, 17, c="ink")


@icon("markdown", F, "Markdown")
def markdown(g):
    sheet(g)
    g.path("M7.5 17.5 V10.5 L10 13.5 L12.5 10.5 V17.5", c="blue")
    g.line(15.5, 10.5, 15.5, 16, c="blue") if g.size != 16 else None
    arrowhead(g, 15.5, 17.5, 0, 1, 2.5, c="blue") if g.size != 16 else None


@icon("image", F, "Image")
def image(g):
    sheet(g)
    g.path("M4.5 19 L9 14 L12.5 17.5 L14.5 15.5 L19.5 20", c="blue")
    g.dot(14.5, 10.5, 1.6, fill="amber-solid")


@icon("audio", F, "Audio")
def audio(g):
    sheet(g)
    g.path("M11 17.5 V9.5 L16 8.5 V15.5", c="violet")
    g.dot(9.5, 17.5, 1.6, fill="violet")
    g.dot(14.5, 15.5, 1.6, fill="violet")


@icon("video", F, "Video")
def video(g):
    sheet(g)
    g.poly([(9.5, 10), (15.5, 13.5), (9.5, 17)], close=True, c="violet", fill="violet-tint")


@icon("data", F, "Data (CSV)")
def data(g):
    sheet(g)
    g.rect(7.5, 9.5, 9, 9, c="teal", fill="teal-tint")
    g.line(7.5, 14, 16.5, 14, c="teal")
    g.line(12, 9.5, 12, 18.5, c="teal")


@icon("sql", F, "SQL / database file")
def sql(g):
    sheet(g)
    g.path("M8 11 V17 A4 1.5 0 0 0 16 17 V11", c="teal", fill="teal-tint")
    g.ellipse(12, 11, 4, 1.5, c="teal", fill="teal-tint")


@icon("json", F, "JSON")
def json_(g):
    sheet(g)
    g.path("M10 9 C8.75 9 8.5 9.75 8.5 11 V12.25 C8.5 13 8 13.5 7 13.5 C8 13.5 8.5 14 8.5 14.75 V16 C8.5 17.25 8.75 18 10 18", c="amber")
    g.path("M14 9 C15.25 9 15.5 9.75 15.5 11 V12.25 C15.5 13 16 13.5 17 13.5 C16 13.5 15.5 14 15.5 14.75 V16 C15.5 17.25 15.25 18 14 18", c="amber")


@icon("config", F, "Settings file (.toml, .ini)")
def config(g):
    sheet(g)
    g.line(7.5, 11, 16.5, 11, c="ink")
    g.line(7.5, 16, 16.5, 16, c="ink")
    g.circle(10, 11, 1.75, c="blue", fill="paper")
    g.circle(14, 16, 1.75, c="blue", fill="paper")


@icon("font", F, "Font")
def font(g):
    sheet(g)
    g.poly([(8, 18), (12, 9), (16, 18)], c="blue")
    g.line(9.5, 15, 14.5, 15, c="blue")


@icon("archive", F, "Archive (.zip)")
def archive(g):
    sheet(g)
    for y in (4, 7, 10):
        g.line(10, y, 11.5, y, c="ink")
        g.line(12.5, y + 1.5, 14, y + 1.5, c="ink")
    g.rect(10, 13.5, 4, 5, r=1, c="ink", fill="shade")


@icon("html", F, "Web page (.html, .css, .js)")
def html(g):
    sheet(g)
    g.poly([(10, 10.5), (7.5, 13.5), (10, 16.5)], c="cyan")
    g.poly([(14, 10.5), (16.5, 13.5), (14, 16.5)], c="cyan")


@icon("rust", F, "Rust source (.rs)")
def rust(g):
    # a hex nut: native code
    sheet(g)
    import math
    pts = [(12 + 4.5 * math.cos(math.radians(30 + 60 * i)), 14 + 4.5 * math.sin(math.radians(30 + 60 * i))) for i in range(6)]
    g.poly(pts, close=True, c="violet", fill="violet-tint")
    g.circle(12, 14, 1.75, c="violet", fill="paper")


# ---- more file types (RAssets batch 1) ------------------------------------------

@icon("translation", F, "Translation file (.po, .pot)")
def translation(g):
    # a speech bubble with swap arrows: the text said in another language.
    # Amber: text values (the debugger's hue for text).
    sheet(g)
    if g.size == 16:
        # (hand-hinted: the arrows are barbs, as a 6 x 4 px room allows)
        g.native(16, "M5.5 5.5 H10.5 A1 1 0 0 1 11.5 6.5 V10.5 A1 1 0 0 1 10.5 11.5 H8.5 L6.5 13.5 V11.5 A1 1 0 0 1 4.5 10.5 V6.5 A1 1 0 0 1 5.5 5.5 Z", c="amber", fill="amber-tint")
        g.native(16, "M6.5 7.5 H9.5 M8.5 8.5 L9.5 7.5", c="amber")
        g.native(16, "M9.5 9.5 H6.5 M7.5 8.5 L6.5 9.5", c="amber")
        return
    g.path("M9 9.5 H15 A2 2 0 0 1 17 11.5 V15.25 A2 2 0 0 1 15 17.25 H11.75 L8.75 20 V17.25 H9 A2 2 0 0 1 7 15.25 V11.5 A2 2 0 0 1 9 9.5 Z", c="amber", fill="amber-tint")
    g.line(9.75, 12.25, 13.5, 12.25, c="amber")
    g.fpoly([(15.25, 12.25), (13.25, 10.95), (13.25, 13.55)], fill="amber")
    g.line(14.25, 14.75, 10.5, 14.75, c="amber")
    g.fpoly([(8.75, 14.75), (10.75, 13.45), (10.75, 16.05)], fill="amber")


@icon("xml", F, "XML (.xml, .xsd, .plist)")
def xml(g):
    # a tree: a root and two children (structured, marked-up data)
    sheet(g)
    g.path("M12 13 V14.75 M9.25 16 V14.75 H14.75 V16", c="teal")
    g.frect(10.25, 9.5, 3.5, 3.5, fill="teal")
    g.frect(7.5, 16, 3.5, 3.5, fill="teal")
    g.frect(13, 16, 3.5, 3.5, fill="teal")


@icon("log", F, "Log (.log)")
def log(g):
    # lines that start with a time stamp
    sheet(g)
    for y in ((9.5, 13, 16.5) if g.size != 16 else (9, 13, 17)):
        g.line(7.5, y, 9.5, y, c="blue")
        g.line(11.5, y, 16.5, y, c="ink")


@icon("pdf", F, "PDF document (.pdf)")
def pdf(g):
    # a page with a ribbon bookmark: a finished, fixed document
    sheet(g)
    g.path("M7.75 2 V10 L9.5 8.5 L11.25 10 V2", c="red", fill="red-tint")
    g.line(7.5, 13.5, 16.5, 13.5, c="ink")
    g.line(7.5, 17, 13.5, 17, c="ink")


@icon("svg", F, "Vector image (.svg)")
def svg(g):
    # a curve between two anchor points
    sheet(g)
    g.path("M8.5 17.5 C8.5 10.5 15.5 17.5 15.5 10.5", c="violet")
    g.rect(6.5, 16.5, 3, 3, c="violet", fill="paper")
    g.rect(14.5, 9.5, 3, 3, c="violet", fill="paper")


@icon("midi", F, "MIDI music (.mid, .midi)")
def midi_file(g):
    # three keys of a piano
    sheet(g)
    g.rect(7.5, 10, 9, 8.5, r=1, c="violet", fill="paper")
    for x in (10.5, 13.5):
        g.line(x, 14.5, x, 18.5, c="violet")
    for x in (9.5, 12.5):
        g.frect(x, 10, 2, 4.5, fill="violet")


@icon("model", F, "3D model (.obj, .stl, .gltf)")
def model(g):
    # a cube, drawn isometric as true 3D objects are
    sheet(g)
    with g.sub(5.5, 8, 0.5):
        cube_iso(g, c="violet", fill="violet-tint")


@icon("certificate", F, "Certificate (.crt, .cer, .p12)")
def certificate(g):
    # a seal with ribbons
    sheet(g)
    g.line(10.25, 15.25, 9.25, 19.5, c="teal")
    g.line(13.75, 15.25, 14.75, 19.5, c="teal")
    g.circle(12, 12.5, 3.5, c="teal", fill="teal-tint")


@icon("key", F, "Key (.key, .pem, .pub)")
def key_file(g):
    # a key: bow, shaft and two teeth
    sheet(g)
    g.circle(9.5, 13.75, 2.5, c="amber", fill="amber-tint")
    g.line(12, 13.75, 17, 13.75, c="amber")
    g.line(14.5, 13.75, 14.5, 16.5, c="amber")
    if g.size != 16:
        g.line(17, 13.75, 17, 16.5, c="amber")


@icon("env", F, "Secrets (.env)")
def env(g):
    # a padlock: settings kept secret (as the Secrets component)
    sheet(g)
    g.path("M9.75 12 V10.5 A2.25 2.25 0 0 1 14.25 10.5 V12", c="ink")
    g.rect(8, 12, 8, 6.5, r=1.5, c="amber", fill="amber-tint")
    g.frect(11.25, 14, 1.5, 2.5, fill="amber")
