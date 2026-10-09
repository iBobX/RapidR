"""Components: one icon per component type (rapidr_ast::COMPONENT_TYPES; a
RapidQ name and its R name share it) and per planned component of the IDE
plan (docs/ide-components.md). Two-tone: a neutral frame (`ink` outline on
`paper`) shows the control's shape, one hue marks its key part (README,
"Colour").

Hues by family: blue for controls, forms and the IDE's components; cyan for
dialogs, network and the web; teal for data; amber for time, values and
charts' lines; violet for media, DirectX and Direct3D; red only for errors,
breakpoints and the X axis.
"""

import math

from kit import icon
from motifs import arrowhead, page, folder, folder_open, window, magnifier, floppy, gear, star4, rot, cylinder

C = "components"


def frame(g, x=3, y=4, w=18, h=16, r=2, c="ink", fill="paper"):
    g.rect(x, y, w, h, r=r, c=c, fill=fill)


def bar_window(g, x=3, y=4, w=18, h=16, bar=4.5, hue="blue"):
    """A window: a frame with a tinted title bar."""
    g.rect(x, y, w, h, r=2, c="ink", fill="paper")
    g.path(f"M{x} {y + bar} V{y + 2} A2 2 0 0 1 {x + 2} {y} H{x + w - 2} A2 2 0 0 1 {x + w} {y + 2} V{y + bar} Z", c="ink", fill=f"{hue}-tint")


def db_badge(g, hue="teal"):
    """A small database (data-aware components) in the bottom-right corner."""
    def shape(m):
        m.path("M13.5 14.5 V20.5 A4.25 1.75 0 0 0 22 20.5 V14.5 A4.25 1.75 0 0 0 13.5 14.5 Z", c="fg", fill="fg")
    g.cut(shape, gap=1)
    g.path("M13.5 14.5 V20.5 A4.25 1.75 0 0 0 22 20.5 V14.5", c=hue, fill=f"{hue}-tint")
    g.ellipse(17.75, 14.5, 4.25, 1.75, c=hue, fill=f"{hue}-tint")


# ---- forms and containers ----------------------------------------------------

@icon("form", C, "Form (QFORM, RFORM)")
def form(g):
    bar_window(g)


@icon("formmdi", C, "MDI form (QFORMMDI)")
def formmdi(g):
    g.rect(2.5, 3, 19, 18, r=2, c="ink", fill="paper")
    g.line(2.5, 7, 21.5, 7, c="ink")
    bar_window(g, 6.5, 10, 12, 8, bar=3.5)


@icon("panel", C, "Panel")
def panel(g):
    frame(g)
    g.rect(6.5, 7.5, 11, 9, r=1, c="blue", fill="blue-tint")


@icon("groupbox", C, "Group box")
def groupbox(g):
    g.path("M7 6.5 H5 A2 2 0 0 0 3 8.5 V18 A2 2 0 0 0 5 20 H19 A2 2 0 0 0 21 18 V8.5 A2 2 0 0 0 19 6.5 H16.5", c="ink", fill="paper")
    g.line(9.5, 6.5, 14, 6.5, c="blue")
    g.line(7, 12, 17, 12, c="ink") if g.size != 16 else None
    g.line(7, 15.5, 13, 15.5, c="ink") if g.size != 16 else g.line(7, 13.5, 14, 13.5, c="ink")


@icon("tabcontrol", C, "Tab control")
def tabcontrol(g):
    # an inactive tab behind; the active tab and the page as one outline
    g.path("M11 8.5 V6 A1.5 1.5 0 0 1 12.5 4.5 H17 A1.5 1.5 0 0 1 18.5 6 V8.5", c="ink", fill="paper")
    g.path("M3 18 V6 A1.5 1.5 0 0 1 4.5 4.5 H9.5 A1.5 1.5 0 0 1 11 6 V8.5 H19 A2 2 0 0 1 21 10.5 V18 A2 2 0 0 1 19 20 H5 A2 2 0 0 1 3 18 Z", c="ink", fill="paper")
    g.path("M3 9 V6 A1.5 1.5 0 0 1 4.5 4.5 H9.5 A1.5 1.5 0 0 1 11 6 V9", c="blue", fill="blue-tint")
    g.line(6, 14, 17, 14, c="ink") if g.size != 16 else None


@icon("splitter", C, "Splitter")
def splitter(g):
    frame(g, 3, 4, 7, 16)
    frame(g, 14, 4, 7, 16)
    g.line(12, 7, 12, 17, c="blue")


@icon("scrollbox", C, "Scroll box")
def scrollbox(g):
    frame(g)
    g.line(16, 4, 16, 20, c="ink")
    g.line(18.5, 7.5, 18.5, 12, c="blue")


@icon("bevel", C, "Bevel (QBEVEL)")
def bevel(g):
    frame(g, r=1)
    g.rect(7, 8, 10, 8, c="ink", fill="blue-tint")
    for x0, y0, x1, y1 in ((3, 4, 7, 8), (21, 4, 17, 8), (3, 20, 7, 16), (21, 20, 17, 16)):
        g.line(x0, y0, x1, y1, c="ink")


@icon("glassframe", C, "Glass frame")
def glassframe(g):
    frame(g, c="cyan", fill="cyan-tint")
    g.line(7, 16, 12, 8, c="cyan")
    g.line(11, 16, 14, 11, c="cyan")


@icon("toolbar", C, "Toolbar")
def toolbar(g):
    frame(g, 2.5, 7, 19, 10)
    for x in (5, 10, 15):
        g.rect(x, 9.5, 4, 5, r=1, c="blue", fill="blue-tint") if g.size != 16 else g.frect(x, 9.5, 4, 5, fill="blue")


@icon("statusbar", C, "Status bar")
def statusbar(g):
    frame(g)
    g.path("M3 15.5 H21 V18 A2 2 0 0 1 19 20 H5 A2 2 0 0 1 3 18 Z", c="ink", fill="blue-tint")
    g.line(10, 15.5, 10, 20, c="ink")


# ---- menus ----------------------------------------------------------------------

@icon("mainmenu", C, "Main menu")
def mainmenu(g):
    frame(g, 2.5, 3, 19, 5, r=1.5)
    g.rect(4, 3, 6, 5, r=1, c="blue", fill="blue-tint") if g.size != 16 else g.frect(3, 3.5, 7, 4, fill="blue-tint")
    g.rect(4, 8, 10, 13, r=1.5, c="ink", fill="paper")
    g.line(7, 12, 11, 12, c="ink")
    g.line(7, 16.5, 11, 16.5, c="ink")


@icon("menuitem", C, "Menu item")
def menuitem(g):
    g.rect(2.5, 7, 19, 10, r=2, c="blue", fill="blue-tint")
    g.line(6, 12, 13, 12, c="blue")
    g.poly([(16.5, 9.5), (19, 12), (16.5, 14.5)], c="blue")


@icon("popupmenu", C, "Popup menu")
def popupmenu(g):
    frame(g, 3, 3, 13, 15, r=1.5)
    g.frect(4.5, 8.25, 10, 4.5, fill="blue-tint")
    g.line(6, 6, 12, 6, c="ink")
    g.line(6, 10.5, 12, 10.5, c="blue")
    g.line(6, 15, 11, 15, c="ink")
    ptr = [(14, 12.5), (21, 17.5), (17.5, 18), (19, 21.5), (14.5, 21.5) if False else (17.5, 21.5)]
    ptr = [(14, 12), (21, 17.5), (17.75, 18), (19.5, 21.5), (17.5, 22), (16, 18.75), (14, 21)]
    g.cut(lambda m: m.poly(ptr, close=True, c="fg", fill="fg"), gap=0.75)
    g.poly(ptr, close=True, c="ink", fill="paper")


# ---- buttons and input -------------------------------------------------------------

@icon("button", C, "Button")
def button(g):
    g.rect(2.5, 7, 19, 10, r=3, c="blue", fill="blue-tint")
    g.line(8, 12, 16, 12, c="blue")


@icon("coolbtn", C, "Speed button (QCOOLBTN)")
def coolbtn(g):
    g.rect(4, 4, 16, 16, r=3, c="blue", fill="blue-tint")
    g.poly([(13, 7), (8.5, 12.5), (12, 12.5), (11, 17), (15.5, 11.5), (12, 11.5)], close=True, c="blue", fill="blue")


@icon("ovalbtn", C, "Oval button (QOVALBTN)")
def ovalbtn(g):
    g.ellipse(12, 12, 9.5, 6, c="blue", fill="blue-tint")
    g.line(8.5, 12, 15.5, 12, c="blue")


@icon("checkbox", C, "Check box")
def checkbox(g):
    g.rect(4, 4, 16, 16, r=3, c="blue", fill="blue-tint")
    g.poly([(8, 12.5), (11, 15.5), (16.5, 9)], c="blue")


@icon("radiobutton", C, "Radio button")
def radiobutton(g):
    g.circle(12, 12, 8.5, c="blue", fill="blue-tint")
    g.dot(12, 12, 3.5, fill="blue")


@icon("edit", C, "Edit box")
def edit(g):
    g.rect(2.5, 6.5, 19, 11, r=2, c="ink", fill="paper")
    g.line(6, 12, 10.5, 12, c="ink")
    g.line(13.5, 9, 13.5, 15, c="blue")
    if g.size != 16:
        g.line(12, 9, 15, 9, c="blue")
        g.line(12, 15, 15, 15, c="blue")


@icon("memo", C, "Memo")
def memo(g):
    frame(g, 3, 3, 18, 18)
    g.line(7, 7.5, 17, 7.5, c="ink")
    g.line(7, 12, 17, 12, c="ink")
    g.line(7, 16.5, 11, 16.5, c="ink")
    g.line(13.5, 14.5, 13.5, 18.5, c="blue")


@icon("richedit", C, "Rich edit")
def richedit(g):
    frame(g, 3, 3, 18, 18)
    g.line(7, 8, 17, 8, c="blue")
    g.line(7, 12, 13, 12, c="ink")
    g.line(15.5, 12, 17, 12, c="violet")
    g.line(7, 16, 14, 16, c="amber")


@icon("combobox", C, "Combo box")
def combobox(g):
    g.rect(2.5, 6.5, 19, 11, r=2, c="ink", fill="paper")
    g.path("M14.5 6.5 H19.5 A2 2 0 0 1 21.5 8.5 V15.5 A2 2 0 0 1 19.5 17.5 H14.5 Z", c="ink", fill="blue-tint")
    g.line(6, 12, 10.5, 12, c="ink")
    g.poly([(16, 11), (18, 13), (20, 11)], c="blue")


@icon("scrollbar", C, "Scroll bar")
def scrollbar(g):
    frame(g, 2.5, 8, 19, 8, r=2)
    g.frect(9, 10, 6, 4, fill="blue", r=1) if g.size != 16 else g.frect(9, 10.5, 6, 3, fill="blue")
    g.poly([(6.5, 10.5), (5, 12), (6.5, 13.5)], c="ink") if g.size != 16 else None
    g.poly([(17.5, 10.5), (19, 12), (17.5, 13.5)], c="ink") if g.size != 16 else None


@icon("trackbar", C, "Track bar")
def trackbar(g):
    g.line(3, 14, 21, 14, c="ink")
    for x in (3, 7.5, 12, 16.5, 21) if g.size != 16 else (3, 12, 21):
        g.line(x, 18.5, x, 20.5, c="ink")
    if g.size == 16:
        g.rect(7.5, 7, 4.5, 10, r=1, c="blue", fill="blue-tint")
    else:
        g.poly([(7.5, 5.5), (12.5, 5.5), (12.5, 12), (10, 14.5), (7.5, 12)], close=True, c="blue", fill="blue-tint")


@icon("updown", C, "Up-down")
def updown(g):
    g.rect(6, 3, 12, 8.5, r=2, c="ink", fill="paper")
    g.rect(6, 12.5, 12, 8.5, r=2, c="ink", fill="paper")
    g.poly([(9.5, 8.5), (12, 6), (14.5, 8.5)], c="blue")
    g.poly([(9.5, 15.5), (12, 18), (14.5, 15.5)], c="blue")


@icon("datetimepicker", C, "Date and time picker")
def datetimepicker(g):
    g.rect(3, 5, 18, 16, r=2, c="ink", fill="paper")
    g.path("M3 10 V7 A2 2 0 0 1 5 5 H19 A2 2 0 0 1 21 7 V10 Z", c="ink", fill="blue-tint")
    g.line(8, 3, 8, 6.5, c="ink")
    g.line(16, 3, 16, 6.5, c="ink")
    if g.size == 16:
        g.frect(13.5, 13, 4, 4, fill="blue")
    else:
        for x in (7.5, 12, 16.5):
            g.dot(x, 13.5, 1, fill="ink")
        g.dot(7.5, 17, 1, fill="ink")
        g.dot(12, 17, 1, fill="blue") if False else g.frect(10.5, 15.5, 3, 3, fill="blue", r=0.5)


@icon("codeeditor", C, "Code editor")
def codeeditor(g):
    frame(g, 3, 3, 18, 18)
    g.line(7.5, 3, 7.5, 21, c="ink")
    g.line(10.5, 8, 17, 8, c="blue")
    g.line(13, 12, 18, 12, c="amber")
    g.line(10.5, 16, 15, 16, c="ink")


@icon("markdownview", C, "Markdown view")
def markdownview(g):
    frame(g, 3, 3, 18, 18)
    g.path("M6.5 16.5 V9.5 L9.25 12.75 L12 9.5 V16.5", c="blue")
    g.line(16, 9.5, 16, 15.5, c="blue") if g.size != 16 else None
    arrowhead(g, 16, 16.5, 0, 1, 2.5, c="blue") if g.size != 16 else None


# ---- display and drawing ----------------------------------------------------------

def letter_a(g, x=6, y=19, w=12, h=14, c="blue"):
    g.poly([(x, y), (x + w / 2, y - h), (x + w, y)], c=c)
    g.line(x + w * 0.22, y - h * 0.32, x + w * 0.78, y - h * 0.32, c=c)


@icon("label", C, "Label")
def label(g):
    letter_a(g, 3, 15.5, 11, 12.5)
    g.line(3, 20, 21, 20, c="ink")
    g.line(16.5, 15.5, 21, 15.5, c="ink") if g.size != 16 else None


@icon("image", C, "Image")
def image(g):
    frame(g)
    g.path("M3 17 L8.5 11.5 L13 16 L15.5 13.5 L21 18.5", c="blue")
    g.circle(15.5, 8.5, 1.5, c="amber", fill="amber-tint") if g.size != 16 else g.dot(15.5, 8.5, 1.6, fill="amber")


@icon("canvas", C, "Canvas")
def canvas(g):
    frame(g)
    g.path("M6 16.5 C8 9 11 9 12 12 C13 15 15.5 15.5 18 8", c="blue")


@icon("progressbar", C, "Progress bar (QGAUGE)")
def progressbar(g):
    g.rect(2.5, 8.5, 19, 7, r=2, c="ink", fill="paper")
    g.frect(4.5, 10.5, 9, 3, fill="blue", r=0.5) if g.size != 16 else g.frect(4.5, 10.5, 9, 3, fill="blue")


@icon("digdisplay", C, "Digital display (QDIGDISPLAY)")
def digdisplay(g):
    segs = [((8.5, 3.5), (15.5, 3.5)), ((8.5, 12), (15.5, 12)), ((8.5, 20.5), (15.5, 20.5)),
            ((7, 5), (7, 10.5)), ((17, 5), (17, 10.5)), ((7, 13.5), (7, 19)), ((17, 13.5), (17, 19))]
    for (x0, y0), (x1, y1) in segs:
        g.line(x0, y0, x1, y1, c="red")
    g.dot(20.5, 20.5, 1, fill="red") if g.size != 16 else None


@icon("header", C, "Header")
def header(g):
    g.rect(2.5, 4, 19, 6, r=1.5, c="ink", fill="blue-tint")
    g.line(9, 4, 9, 10, c="ink")
    g.line(15, 4, 15, 10, c="ink")
    g.line(4, 14.5, 20, 14.5, c="ink")
    g.line(4, 19, 20, 19, c="ink")


@icon("designsurface", C, "Design surface")
def designsurface(g):
    frame(g)
    if g.size != 16:
        for x in (6, 18):
            for y in (7, 17):
                g.dot(x, y, 0.75, fill="ink")
    g.rect(8, 9, 8, 6, c="blue", fill="blue-tint")
    for x in (8, 16):
        for y in (9, 15):
            g.frect(x - 1.25, y - 1.25, 2.5, 2.5, fill="blue")


@icon("rect", C, "Rectangle (QRECT)")
def rect_(g):
    g.rect(5, 6, 14, 12, c="ink")
    for x in (5, 19):
        for y in (6, 18):
            g.frect(x - 1.5, y - 1.5, 3, 3, fill="blue")


# ---- lists, grids and trees ---------------------------------------------------------

@icon("listbox", C, "List box")
def listbox(g):
    frame(g, 3, 3, 18, 18)
    g.frect(4.5, 9.75, 15, 4.5, fill="blue-tint")
    g.line(7, 7.5, 17, 7.5, c="ink")
    g.line(7, 12, 17, 12, c="blue")
    g.line(7, 16.5, 14, 16.5, c="ink")


@icon("filelistbox", C, "File list box")
def filelistbox(g):
    frame(g, 3, 3, 18, 18)
    for y in (7.5, 12, 16.5):
        g.path(f"M6.5 {y - 1.75} H8.5 L9.5 {y - 0.75} V{y + 1.75} H6.5 Z", c="blue", fill="blue-tint") if g.size != 16 else g.frect(6, y - 1, 2, 2, fill="blue")
        g.line(12, y, 17.5, y, c="ink")


@icon("dirtree", C, "Directory tree")
def dirtree(g):
    g.path("M6.5 8 V17.5 H10", c="ink")
    g.line(6.5, 12.5, 10, 12.5, c="ink") if g.size != 16 else g.line(6.5, 12.5, 10, 12.5, c="ink")
    for (x, y) in ((3, 3), (10, 10), (10, 15)):
        g.path(f"M{x} {y + 5} V{y} H{x + 3.5} L{x + 4.5} {y + 1} H{x + 8} V{y + 5} Z", c="blue", fill="blue-tint")


@icon("treeview", C, "Tree view (QOUTLINE)")
def treeview(g):
    g.path("M6 8 V17.5 H11", c="ink")
    g.line(6, 12.5, 11, 12.5, c="ink")
    g.rect(3, 3, 6, 5, r=1, c="ink", fill="paper")
    g.rect(11, 10, 9, 5, r=1, c="blue", fill="blue-tint")
    g.rect(11, 15, 9, 5, r=1, c="ink", fill="paper") if False else g.rect(11, 16, 9, 3.5, r=1, c="ink", fill="paper") if False else None
    g.line(11, 17.5, 19, 17.5, c="ink") if False else g.rect(11, 15.5, 9, 4.5, r=1, c="ink", fill="paper")


@icon("listview", C, "List view")
def listview(g):
    g.rect(3, 3, 7.5, 7.5, r=1.5, c="blue", fill="blue-tint")
    for x, y in ((13.5, 3), (3, 13.5), (13.5, 13.5)):
        g.rect(x, y, 7.5, 7.5, r=1.5, c="ink", fill="paper")


@icon("stringgrid", C, "String grid")
def stringgrid(g):
    frame(g)
    g.path("M3 8.5 V6 A2 2 0 0 1 5 4 H19 A2 2 0 0 1 21 6 V8.5 Z", c="ink", fill="blue-tint")
    g.line(3, 14.25, 21, 14.25, c="ink")
    g.line(9, 4, 9, 20, c="ink")
    g.line(15, 4, 15, 20, c="ink")


# ---- dialogs ---------------------------------------------------------------------------

def dialog(g):
    g.rect(2.5, 3, 19, 18, r=2, c="ink", fill="paper")
    g.path("M2.5 7 V5 A2 2 0 0 1 4.5 3 H19.5 A2 2 0 0 1 21.5 5 V7 Z", c="ink", fill="cyan-tint")


@icon("opendialog", C, "Open dialog")
def opendialog(g):
    dialog(g)
    g.path("M7 18 V10.5 H10 L11 11.5 H15.5 V13", c="cyan")
    g.path("M7 18 L8.5 14 H17.5 L16 18 Z", c="cyan", fill="cyan-tint")


@icon("savedialog", C, "Save dialog")
def savedialog(g):
    dialog(g)
    g.path("M7.5 10.5 H14.5 L16.5 12.5 V18.5 H7.5 Z", c="cyan", fill="cyan-tint")
    g.path("M10 18.5 V15.5 H14 V18.5", c="cyan") if g.size != 16 else None


@icon("filedialog", C, "File dialog")
def filedialog(g):
    dialog(g)
    g.path("M8.5 10.5 H13 L15.5 13 V18.5 H8.5 Z", c="cyan", fill="cyan-tint")


@icon("colordialog", C, "Colour dialog")
def colordialog(g):
    dialog(g)
    g.dot(8.5, 13, 2, fill="red")
    g.dot(15.5, 13, 2, fill="blue")
    g.dot(12, 17.5, 2, fill="teal")


@icon("fontdialog", C, "Font dialog")
def fontdialog(g):
    dialog(g)
    letter_a(g, 8, 18.5, 8, 8.5, c="cyan")


# ---- non-visual objects ----------------------------------------------------------------

@icon("timer", C, "Timer")
def timer(g):
    g.circle(12, 13.5, 7.5, c="amber", fill="amber-tint")
    g.line(10, 3, 14, 3, c="amber")
    g.line(12, 3, 12, 6, c="amber")
    g.line(12, 13.5, 12, 9.5, c="amber")
    g.line(12, 13.5, 14.5, 13.5, c="amber") if g.size != 16 else None


@icon("font", C, "Font")
def font(g):
    letter_a(g, 2.5, 19.5, 12, 15, c="ink")
    g.circle(17.5, 16, 3, c="blue") if g.size != 16 else g.circle(17.5, 16.5, 3, c="blue")
    g.line(20.5, 13, 20.5, 19.5, c="blue")


@icon("bitmap", C, "Bitmap")
def bitmap(g):
    frame(g, 3, 3, 18, 18, r=1)
    for (i, j, f) in ((0, 0, "blue"), (2, 0, "teal"), (1, 1, "amber-solid"), (0, 2, "violet"), (2, 2, "blue")):
        g.frect(3 + i * 6, 3 + j * 6, 6, 6, fill=f)
    g.rect(3, 3, 18, 18, r=1, c="ink")


@icon("imagelist", C, "Image list")
def imagelist(g):
    g.path("M3 15 V5 A2 2 0 0 1 5 3 H15", c="ink")
    g.path("M6 17.5 V8.5 A2 2 0 0 1 8 6.5 H17.5", c="ink")
    frame(g, 9, 9.5, 12, 11.5)
    g.path("M9 18.5 L13 14.5 L17 18 L21 15", c="blue")


@icon("stringlist", C, "String list")
def stringlist(g):
    for y in (5, 10, 15, 20) if g.size != 16 else (5, 11, 17):
        g.dot(4.5, y, 1.25, fill="amber")
        g.line(8.5, y, 20 if y != 15 else 16, y, c="ink")


@icon("filestream", C, "File stream")
def filestream(g):
    page(g, 4, 2, 13, 18, fold=4.5, c="ink", fill="paper")
    g.cut(lambda m: m.line(9, 17, 21, 17, c="fg"), gap=1)
    g.line(9, 17, 20.5, 17, c="blue")
    arrowhead(g, 21, 17, 1, 0, 3.5, c="blue")


@icon("memorystream", C, "Memory stream")
def memorystream(g):
    g.rect(2.5, 6, 19, 9, r=1.5, c="ink", fill="paper")
    for x in (5, 10, 15):
        g.rect(x, 8, 4, 5, r=0.5, c="blue", fill="blue-tint") if g.size != 16 else g.frect(x, 8.5, 4, 4, fill="blue")
    for x in (5, 9, 13, 17) if g.size != 16 else (5.5, 12, 18.5):
        g.line(x, 15, x, 18, c="ink")


@icon("registry", C, "Registry")
def registry(g):
    g.circle(8, 9, 5, c="amber", fill="amber-tint")
    g.line(11.5, 12.5, 20, 21, c="amber")
    g.line(16.5, 17.5, 18.5, 15.5, c="amber")
    g.line(19, 20, 21, 18, c="amber") if g.size != 16 else None


@icon("printer", C, "Printer")
def printer(g):
    g.path("M7 8 V3 H17 V8", c="ink", fill="paper")
    g.path("M7 17 H4 V9.5 A1.5 1.5 0 0 1 5.5 8 H18.5 A1.5 1.5 0 0 1 20 9.5 V17 H17", c="ink", fill="paper")
    g.rect(7, 13.5, 10, 7.5, r=0.5, c="blue", fill="blue-tint")


@icon("notifyicondata", C, "Tray icon (QNOTIFYICONDATA)")
def notifyicondata(g):
    g.rect(2.5, 15, 19, 6, r=1.5, c="ink", fill="paper")
    g.frect(15, 16.5, 3.5, 3, fill="blue") if g.size == 16 else g.rect(15, 16.5, 4, 3, r=0.5, c="blue", fill="blue")
    g.path("M9 3 H19 A2 2 0 0 1 21 5 V9 A2 2 0 0 1 19 11 H17 L15.5 13 L14 11 H9 A2 2 0 0 1 7 9 V5 A2 2 0 0 1 9 3 Z", c="blue", fill="blue-tint")


@icon("json", C, "JSON")
def json_(g):
    g.path("M9.5 3.5 C7.5 3.5 7 4.5 7 6.5 V9.5 C7 11 6 12 4.5 12 C6 12 7 13 7 14.5 V17.5 C7 19.5 7.5 20.5 9.5 20.5", c="amber")
    g.path("M14.5 3.5 C16.5 3.5 17 4.5 17 6.5 V9.5 C17 11 18 12 19.5 12 C18 12 17 13 17 14.5 V17.5 C17 19.5 16.5 20.5 14.5 20.5", c="amber")


# ---- databases ---------------------------------------------------------------------------

@icon("sqlite", C, "SQLite database")
def sqlite(g):
    cylinder(g, 4.5, 3, 15, 18, c="teal", fill="teal-tint", bands=1 if g.size != 16 else 0)


@icon("mysql", C, "MySQL database")
def mysql(g):
    cylinder(g, 3, 3, 14, 16, c="teal", fill="teal-tint", bands=1 if g.size != 16 else 0)
    g.cut(lambda m: m.rect(13.5, 13.5, 8.5, 8.5, r=2, c="fg", fill="fg"), gap=0.75)
    # a network: three linked nodes (a database served over the network)
    g.path("M17.75 15.5 V18.25 M15 21 L17.75 18.25 L20.5 21", c="cyan")
    g.dot(17.75, 15, 1.5, fill="cyan")
    g.dot(15, 21, 1.5, fill="cyan")
    g.dot(20.5, 21, 1.5, fill="cyan")


# ---- network, devices and CGI ---------------------------------------------------------------

@icon("socket", C, "Socket")
def socket(g):
    g.path("M7 9 H17 V13 A5 5 0 0 1 7 13 Z", c="cyan", fill="cyan-tint")
    g.line(9.5, 3.5, 9.5, 9, c="cyan")
    g.line(14.5, 3.5, 14.5, 9, c="cyan")
    g.line(12, 18, 12, 21.5, c="cyan")


@icon("serversocket", C, "Server socket")
def serversocket(g):
    g.rect(3, 3, 18, 8, r=2, c="cyan", fill="cyan-tint")
    g.rect(3, 13, 18, 8, r=2, c="cyan", fill="cyan-tint")
    g.dot(7, 7, 1.25, fill="cyan")
    g.dot(7, 17, 1.25, fill="cyan")
    g.line(11.5, 7, 17, 7, c="cyan") if g.size != 16 else None
    g.line(11.5, 17, 17, 17, c="cyan") if g.size != 16 else None


def globe(g, cx=12, cy=12, r=9, c="cyan", fill="cyan-tint"):
    g.circle(cx, cy, r, c=c, fill=fill)
    g.ellipse(cx, cy, r * 0.45, r, c=c)
    g.line(cx - r, cy, cx + r, cy, c=c)


@icon("http", C, "HTTP client")
def http(g):
    globe(g)


@icon("download", C, "Download")
def download(g):
    g.path("M4 14 V19 A1 1 0 0 0 5 20 H19 A1 1 0 0 0 20 19 V14", c="ink")
    g.line(12, 3, 12, 14, c="cyan")
    arrowhead(g, 12, 14.5, 0, 1, 5, c="cyan")


@icon("comport", C, "Serial port (QCOMPORT)")
def comport(g):
    g.path("M3 7 H21 L18.5 17 H5.5 Z", c="cyan", fill="cyan-tint")
    if g.size == 16:
        for x in (7, 10.5, 14, 17.5):
            g.frect(x - 0.75, 9.5, 1, 1, fill="cyan")
        for x in (9, 12.5, 16):
            g.frect(x - 0.75, 13.5, 1, 1, fill="cyan")
    else:
        for x in (7, 9.5, 12, 14.5, 17):
            g.dot(x, 10, 0.9, fill="cyan")
        for x in (8.25, 10.75, 13.25, 15.75):
            g.dot(x, 14, 0.9, fill="cyan")


@icon("cgi", C, "CGI")
def cgi(g):
    page(g, 4.5, 2, 15, 20, fold=5, c="ink", fill="paper")
    g.poly([(8, 10.5), (11, 13), (8, 15.5)], c="cyan")
    g.line(12.5, 17, 16, 17, c="cyan")


# ---- media ---------------------------------------------------------------------------------

@icon("midi", C, "MIDI")
def midi(g):
    g.rect(2.5, 4, 19, 16, r=2, c="ink", fill="paper")
    for x in (8.5, 15.5):
        g.line(x, 13.5, x, 20, c="ink")
    for x in (8.5, 15.5):
        g.frect(x - 2, 4, 4, 9.5, fill="violet")


@icon("wave", C, "Wave sound (QWAVE)")
def wave(g):
    hs = ((3.5, 3), (7, 9), (10.5, 15), (14, 7), (17.5, 11), (21, 4)) if g.size != 16 else ((3.5, 4), (8, 12), (12.5, 16), (17, 8), (21, 4))
    for x, h in hs:
        g.line(x, 12 - h / 2, x, 12 + h / 2, c="violet")


@icon("video", C, "Video")
def video(g):
    g.rect(3, 4, 18, 16, r=2, c="violet", fill="violet-tint")
    g.line(7, 4, 7, 20, c="violet")
    g.line(17, 4, 17, 20, c="violet")
    if g.size != 16:
        for y in (8, 12, 16):
            g.line(3, y, 7, y, c="violet")
            g.line(17, y, 21, y, c="violet")


@icon("cdaudio", C, "CD audio")
def cdaudio(g):
    g.circle(12, 12, 9.5, c="violet", fill="violet-tint")
    g.circle(12, 12, 2.5, c="violet", fill="paper")
    g.path("M12 5.5 A6.5 6.5 0 0 1 18.5 12", c="violet") if g.size != 16 else None


# ---- DirectX 2D ------------------------------------------------------------------------

@icon("dxscreen", C, "DirectX screen")
def dxscreen(g):
    g.rect(2.5, 3.5, 19, 13.5, r=2, c="ink", fill="paper")
    g.line(9, 20.5, 15, 20.5, c="ink")
    g.line(12, 17, 12, 20.5, c="ink")
    star4(g, 12, 10.25, 4.5, fill="violet", pinch=0.25)


@icon("dximagelist", C, "DirectX image list")
def dximagelist(g):
    frame(g, 3, 3, 18, 18, r=1.5)
    g.line(12, 3, 12, 21, c="ink")
    g.line(3, 12, 21, 12, c="ink")
    g.rect(12, 12, 9, 9, r=0, c="violet", fill="violet-tint") if False else g.path("M12 12 H21 V19 A2 2 0 0 1 19 21 H12 Z", c="violet", fill="violet-tint")


@icon("dxtimer", C, "DirectX timer")
def dxtimer(g):
    g.circle(12, 13.5, 7.5, c="violet", fill="violet-tint")
    g.line(10, 3, 14, 3, c="violet")
    g.line(12, 3, 12, 6, c="violet")
    g.line(12, 13.5, 12, 9.5, c="violet")
    g.line(12, 13.5, 14.5, 13.5, c="violet") if g.size != 16 else None


def speaker(g, c="violet", fill="violet-tint", waves=2):
    g.path("M3.5 9 H7 L12 4.5 V19.5 L7 15 H3.5 Z", c=c, fill=fill)
    g.path("M15 9 A4 4 0 0 1 15 15", c=c)
    if waves > 1 and g.size != 16:
        g.path("M17.5 6 A8 8 0 0 1 17.5 18", c=c)


@icon("dxsound", C, "DirectX sound")
def dxsound(g):
    speaker(g)


@icon("dxjoystick", C, "DirectX joystick")
def dxjoystick(g):
    g.path("M4 16.5 H20 V18.5 A2 2 0 0 1 18 20.5 H6 A2 2 0 0 1 4 18.5 Z", c="ink", fill="paper")
    g.line(12, 10, 12, 16.5, c="ink")
    g.circle(12, 7, 3.5, c="violet", fill="violet-tint")


# ---- Direct3D -------------------------------------------------------------------------

def cube(g, c="violet", fill="violet-tint", hidden=False):
    # an isometric cube round (12, 12)
    top = [(12, 3), (20, 7.5), (12, 12), (4, 7.5)]
    g.path("M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z", c=c, fill=fill)
    g.path("M4 7.5 L12 12 L20 7.5 M12 12 V21", c=c)


@icon("d3dframe", C, "Direct3D frame")
def d3dframe(g):
    ox, oy = 9, 15
    g.line(ox, oy, 20.5, oy, c="red")
    arrowhead(g, 21, oy, 1, 0, 3, c="red")
    g.line(ox, oy, ox, 3.5, c="teal")
    arrowhead(g, ox, 3, 0, -1, 3, c="teal")
    g.line(ox, oy, 3.75, 20.25, c="blue")
    arrowhead(g, 3.5, 20.5, -1, 1, 3, c="blue")


@icon("d3dmeshbuilder", C, "Direct3D mesh builder")
def d3dmeshbuilder(g):
    cube(g)
    g.cut(lambda m: m.circle(18, 18, 4.5, c="fg", fill="fg"), gap=0.5)
    g.line(18, 15, 18, 21, c="violet")
    g.line(15, 18, 21, 18, c="violet")


@icon("d3dmesh", C, "Direct3D mesh")
def d3dmesh(g):
    cube(g)
    if g.size != 16:
        g.line(4, 7.5, 12, 21, c="violet")
        g.line(12, 12, 20, 16.5, c="violet")


@icon("d3dface", C, "Direct3D face")
def d3dface(g):
    pts = [(12, 4), (20.5, 19), (3.5, 19)]
    g.poly(pts, close=True, c="violet", fill="violet-tint")
    for x, y in pts:
        g.dot(x, y, 1.75, fill="violet")


@icon("d3dlight", C, "Direct3D light")
def d3dlight(g):
    g.circle(12, 12, 4, c="amber", fill="amber-tint")
    for i in range(8):
        a = i * math.pi / 4
        g.line(12 + 6.5 * math.cos(a), 12 + 6.5 * math.sin(a), 12 + 9 * math.cos(a), 12 + 9 * math.sin(a), c="amber")


@icon("d3dtexture", C, "Direct3D texture")
def d3dtexture(g):
    g.path("M6 6 H21 L18 18 H3 Z", c="ink", fill="paper") if False else None
    frame(g, 3, 3, 18, 18, r=1, c="violet", fill="paper")
    for (i, j) in ((0, 0), (2, 0), (1, 1), (0, 2), (2, 2)):
        g.frect(3 + i * 6, 3 + j * 6, 6, 6, fill="violet-tint")
    g.rect(3, 3, 18, 18, r=1, c="violet")


@icon("d3dvisual", C, "Direct3D visual")
def d3dvisual(g):
    g.circle(12, 12, 9, c="violet", fill="violet-tint")
    g.path("M3 12 A9 3.5 0 0 0 21 12", c="violet")


@icon("d3dwrap", C, "Direct3D wrap")
def d3dwrap(g):
    g.circle(12, 12, 9, c="violet", fill="violet-tint")
    g.ellipse(12, 12, 4, 9, c="violet")
    g.path("M3 12 A9 3.5 0 0 0 21 12", c="violet") if g.size != 16 else g.line(3, 12, 21, 12, c="violet")


@icon("d3dvector", C, "Direct3D vector")
def d3dvector(g):
    g.dot(5, 19, 2, fill="ink")
    g.line(5, 19, 18.5, 5.5, c="violet")
    g.poly([(11.5, 5), (19, 5), (19, 12.5)], c="violet")


@icon("d3danimation", C, "Direct3D animation")
def d3danimation(g):
    # a cube (the animated frame) under a motion arc with its keyframes
    with g.sub(-0.4, 6.4, 0.72):
        cube(g)
    g.path("M9 5 Q18 0.5 20.5 10.5", c="violet")
    arrowhead(g, 20.5, 11, 0.25, 1, 3, c="violet")
    g.dot(9, 5, 1.5, fill="ink")


@icon("d3danimationset", C, "Direct3D animation set")
def d3danimationset(g):
    # two cubes, one behind the other: a set of animations
    with g.sub(7, 0.9, 0.7):
        cube(g)
    with g.sub(0.2, 6.8, 0.7):
        g.cut(lambda m: m.path("M12 3 L20 7.5 V16.5 L12 21 L4 16.5 V7.5 Z", c="fg", fill="fg"), gap=0.5)
        cube(g)


# ---- data science ------------------------------------------------------------------------

@icon("num", C, "RNUM (numeric array)")
def num_(g):
    g.path("M7 5 H4 V19 H7", c="ink")
    g.path("M17 5 H20 V19 H17", c="ink")
    for x in (8.5, 12, 15.5) if g.size != 16 else (8, 13):
        g.dot(x, 12, 1.6, fill="teal")


@icon("dataframe", C, "RDATAFRAME (data frame)")
def dataframe(g):
    frame(g, c="teal")
    g.path("M3 8.5 V6 A2 2 0 0 1 5 4 H19 A2 2 0 0 1 21 6 V8.5 Z", c="teal", fill="teal-tint")
    g.path("M3 8.5 H8.5 V20 H5 A2 2 0 0 1 3 18 Z", c="teal", fill="teal-tint")
    g.line(8.5, 14.25, 21, 14.25, c="teal")
    g.line(14.75, 8.5, 14.75, 20, c="teal")


@icon("plot", C, "RPLOT (chart)")
def plot(g):
    g.path("M3.5 3.5 V20.5 H20.5", c="ink")
    g.path("M6.5 16 L10.5 10.5 L14 13.5 L19.5 6", c="amber")
    for x, y in ((10.5, 10.5), (14, 13.5)) if g.size != 16 else ():
        g.dot(x, y, 1.5, fill="amber")


# ---- web only -----------------------------------------------------------------------------

@icon("webview", C, "Web view")
def webview(g):
    frame(g, 2.5, 4, 19, 16)
    g.line(2.5, 9, 21.5, 9, c="ink")
    g.dot(5.5, 6.5, 0.9, fill="ink") if g.size != 16 else None
    g.line(8.5, 6.5, 18.5, 6.5, c="cyan") if g.size != 16 else g.line(6, 6.5, 18, 6.5, c="cyan")
    globe(g, 12, 14.5, 3.5) if g.size != 16 else g.circle(12, 14.5, 3, c="cyan", fill="cyan-tint")


@icon("dom", C, "DOM")
def dom(g):
    g.poly([(8, 6.5), (2.5, 12), (8, 17.5)], c="cyan")
    g.poly([(16, 6.5), (21.5, 12), (16, 17.5)], c="cyan")
    g.line(13.5, 5, 10.5, 19, c="cyan")


@icon("javascript", C, "JavaScript")
def javascript(g):
    g.rect(3, 3, 18, 18, r=3, c="cyan", fill="cyan-tint")
    g.path("M16 7 C13.5 6.5 12.5 7.5 12 10 L11 15.5 C10.5 17.5 9.5 18 7.5 17.5", c="cyan")
    g.line(9, 11, 15, 11, c="cyan")


@icon("webstorage", C, "Web storage")
def webstorage(g):
    g.rect(3.5, 3, 17, 18, r=2, c="cyan", fill="cyan-tint")
    g.line(3.5, 12, 20.5, 12, c="cyan")
    g.line(9.5, 7.5, 14.5, 7.5, c="cyan")
    g.line(9.5, 16.5, 14.5, 16.5, c="cyan")


@icon("webaudio", C, "Web audio")
def webaudio(g):
    speaker(g, c="cyan", fill="cyan-tint")


@icon("webvideo", C, "Web video")
def webvideo(g):
    g.rect(2.5, 6, 13, 12, r=2, c="cyan", fill="cyan-tint")
    g.path("M15.5 10.5 L21.5 7 V17 L15.5 13.5 Z", c="cyan", fill="cyan-tint")


@icon("webnotification", C, "Web notification")
def webnotification(g):
    g.path("M6 17 V10.5 A6 6 0 0 1 18 10.5 V17 L19.5 18.5 H4.5 Z", c="cyan", fill="cyan-tint")
    g.path("M10 21 H14", c="cyan")


@icon("webgeolocation", C, "Web geolocation")
def webgeolocation(g):
    g.path("M12 21.5 C8 17 5 13.5 5 9.5 A7 7 0 0 1 19 9.5 C19 13.5 16 17 12 21.5 Z", c="cyan", fill="cyan-tint")
    g.circle(12, 9.5, 2.5, c="cyan", fill="paper")


@icon("router", C, "Router")
def router(g):
    g.line(12, 3, 12, 21, c="ink")
    g.path("M12 5 H18.5 L21 7.5 L18.5 10 H12 Z", c="cyan", fill="cyan-tint")
    g.path("M12 12 H5.5 L3 14.5 L5.5 17 H12 Z", c="cyan", fill="cyan-tint")
