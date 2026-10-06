"""Actions: the IDE's commands (docs/ide-plan.md §4, I1–I6). Monochrome
(`currentColor`) unless they carry a meaning in colour: run, stop, debug,
breakpoints, the execution point and the statuses."""

import math

from kit import icon
from motifs import arrowhead, page, folder, folder_open, window, magnifier, floppy, gear, star4, rot

A = "actions"


# ---- files -------------------------------------------------------------------

def plus_badge(g, cx=17.5, cy=17.5, arm=3.5, c="fg"):
    g.cut(lambda m: m.circle(cx, cy, arm + 0.5, c="fg", fill="fg"), gap=0.75)
    g.line(cx, cy - arm, cx, cy + arm, c=c)
    g.line(cx - arm, cy, cx + arm, cy, c=c)


@icon("new-file", A, "New file")
def new_file(g):
    page(g)
    plus_badge(g)


@icon("new-project", A, "New project")
def new_project(g):
    folder(g, 3, 4, 18, 14)
    plus_badge(g, 17.5, 17.5)


@icon("open", A, "Open")
def open_(g):
    folder_open(g, 2.5, 5)


@icon("save", A, "Save")
def save(g):
    floppy(g, 4, 4, 16, detail=g.size != 16 or True)


@icon("save-all", A, "Save all")
def save_all(g):
    g.path("M7 4 V3.5 H18 L21 6.5 V17 H20.5" if False else "M7.5 3 H17.5 L21 6.5 V16.5", c="fg")
    floppy(g, 3, 7, 14, detail=False)


@icon("close", A, "Close")
def close(g):
    g.line(6, 6, 18, 18)
    g.line(18, 6, 6, 18)


@icon("close-all", A, "Close all")
def close_all(g):
    g.rect(8, 3, 13, 13, r=2)
    g.cut(lambda m: m.rect(3, 8, 13, 13, r=2, fill="fg"))
    g.rect(3, 8, 13, 13, r=2)
    g.line(7, 12, 12, 17)
    g.line(12, 12, 7, 17)


@icon("print", A, "Print")
def print_(g):
    g.path("M7 8 V3 H17 V8")
    g.path("M7 17 H4 V9.5 A1.5 1.5 0 0 1 5.5 8 H18.5 A1.5 1.5 0 0 1 20 9.5 V17 H17")
    g.rect(7, 13.5, 10, 7.5, r=0.5)


@icon("import", A, "Import")
def import_(g):
    g.path("M4 14 V19 A1 1 0 0 0 5 20 H19 A1 1 0 0 0 20 19 V14")
    g.line(12, 3, 12, 14)
    arrowhead(g, 12, 14.5, 0, 1, 5)


@icon("export", A, "Export")
def export(g):
    g.path("M4 14 V19 A1 1 0 0 0 5 20 H19 A1 1 0 0 0 20 19 V14")
    g.line(12, 15, 12, 3.5)
    arrowhead(g, 12, 3, 0, -1, 5)


# ---- edit ---------------------------------------------------------------------

@icon("undo", A, "Undo")
def undo(g):
    g.path("M4.5 9.5 H14 A5.5 5.5 0 0 1 14 20.5 H9")
    arrowhead(g, 4.5, 9.5, -1, 0, 5)


@icon("redo", A, "Redo")
def redo(g):
    g.path("M19.5 9.5 H10 A5.5 5.5 0 0 0 10 20.5 H15")
    arrowhead(g, 19.5, 9.5, 1, 0, 5)


@icon("cut", A, "Cut")
def cut(g):
    g.circle(7, 17.5, 3)
    g.circle(17, 17.5, 3)
    g.line(9, 15, 17.5, 3)
    g.line(15, 15, 6.5, 3)


@icon("copy", A, "Copy")
def copy(g):
    g.rect(4, 3, 12, 14, r=2)
    g.cut(lambda m: m.rect(9, 8, 11, 13, r=2, fill="fg"))
    g.rect(9, 8, 11, 13, r=2)


@icon("paste", A, "Paste")
def paste(g):
    g.path("M8 5 H6 A2 2 0 0 0 4 7 V19 A2 2 0 0 0 6 21 H10")
    g.path("M16 5 H18 A2 2 0 0 1 20 7 V10")
    g.rect(8, 3, 8, 4, r=1)
    g.rect(12, 12, 9, 10, r=1.5)


@icon("find", A, "Find")
def find(g):
    magnifier(g)


@icon("search", A, "Search")
def search(g):
    magnifier(g)


@icon("replace", A, "Replace")
def replace(g):
    g.rect(3, 3.5, 9, 7, r=1.5)
    g.rect(12, 13.5, 9, 7, r=1.5)
    g.path("M12 7 H15 A2 2 0 0 1 17 9 V11")
    arrowhead(g, 17, 11.5, 0, 1, 3)
    g.path("M12 17 H9 A2 2 0 0 1 7 15 V13") if g.size != 16 else None


@icon("select-all", A, "Select all")
def select_all(g):
    # a dashed frame (corners and mid-sides) round a solid block
    for x, y, dx, dy in ((3.5, 3.5, 1, 1), (20.5, 3.5, -1, 1), (3.5, 20.5, 1, -1), (20.5, 20.5, -1, -1)):
        g.poly([(x, y + 3 * dy), (x, y), (x + 3 * dx, y)])
    for x0, y0, x1, y1 in ((10.5, 3.5, 13.5, 3.5), (10.5, 20.5, 13.5, 20.5), (3.5, 10.5, 3.5, 13.5), (20.5, 10.5, 20.5, 13.5)):
        g.line(x0, y0, x1, y1)
    g.frect(8, 8, 8, 8, r=1)


@icon("go-to-line", A, "Go to line")
def go_to_line(g):
    g.line(4, 6, 12, 6)
    g.line(4, 12, 20, 12)
    g.line(4, 18, 12, 18)
    arrowhead(g, 20, 12, 1, 0, 4)


@icon("comment", A, "Toggle comment")
def comment(g):
    g.line(10, 4, 7, 20)
    g.line(17, 4, 14, 20)


@icon("format", A, "Format document")
def format_(g):
    g.line(4, 5, 20, 5)
    g.line(8, 10, 20, 10)
    g.line(8, 15, 20, 15)
    g.line(4, 20, 20, 20)
    g.poly([(4, 10), (6, 12.5), (4, 15)])


# ---- run ------------------------------------------------------------------------

def play(g, x=5.5, y=4.5, w=14.5, h=15, c="teal", fill="teal"):
    """The run triangle, in the brand's counter proportions."""
    g.poly([(x, y), (x + w, y + h / 2), (x, y + h)], close=True, c=c, fill=fill)


@icon("run", A, "Run", mono=False)
def run(g):
    play(g)


@icon("run-without-debugging", A, "Run without debugging", mono=False)
def run_nodebug(g):
    play(g, c="teal", fill="teal-tint")


@icon("continue", A, "Continue", mono=False)
def continue_(g):
    g.frect(4, 4.5, 3, 15, fill="teal", r=1)
    play(g, 9.5, 4.5, 11, 15)


@icon("stop", A, "Stop", mono=False)
def stop(g):
    g.rect(5, 5, 14, 14, r=2, c="red", fill="red")


@icon("pause", A, "Pause")
def pause(g):
    g.frect(6, 4.5, 4, 15, r=1)
    g.frect(14, 4.5, 4, 15, r=1)


@icon("restart", A, "Restart", mono=False)
def restart(g):
    g.path("M19.5 12 A7.5 7.5 0 1 1 16.5 6", c="teal")
    g.poly([(17, 2.5), (17, 6.5), (13, 6.5)], c="teal")


@icon("build", A, "Build")
def build(g):
    # a hammer: the head across the top right, the handle down to the left
    head = rot([(8, 5.5), (21, 5.5), (21, 11), (8, 11)], 14.5, 8.25, 45)
    g.poly(head, close=True, fill="fg")
    g.line(12.5, 12.5, 4, 21)


@icon("rebuild", A, "Rebuild")
def rebuild(g):
    g.path("M4.5 12 A7.5 7.5 0 0 1 17.5 7")
    g.poly([(18, 3), (18, 7.5), (13.5, 7.5)])
    g.path("M19.5 12 A7.5 7.5 0 0 1 6.5 17")
    g.poly([(6, 21), (6, 16.5), (10.5, 16.5)])


@icon("clean", A, "Clean")
def clean(g):
    # a broom
    g.line(19.5, 3.5, 12.5, 10.5)
    g.path("M9 9 L15 15 L11 21 C8 19.5 5 16.5 3 13 Z")
    g.line(8.5, 18.5, 10.5, 15.5) if g.size != 16 else None


def bug(g, c="fg", fill=None):
    g.path("M9 7.5 A3 3 0 0 1 15 7.5", c=c)
    g.rect(7, 8, 10, 13, r=5, c=c, fill=fill)
    g.line(12, 12, 12, 21, c=c)
    legs = ((11.5, 10), (15, 15), (18.5, 20)) if g.size != 16 else ((12, 11), (17.5, 19))
    for y0, y1 in legs:
        g.line(7, y0, 3.5, y1, c=c)
        g.line(17, y0, 20.5, y1, c=c)
    if g.size != 16:
        g.line(10, 4.5, 9, 3, c=c)
        g.line(14, 4.5, 15, 3, c=c)


@icon("debug", A, "Debug", mono=False)
def debug(g):
    bug(g, c="teal", fill="teal-tint")


@icon("step-into", A, "Step into")
def step_into(g):
    g.line(12, 2.5, 12, 12.5)
    arrowhead(g, 12, 13, 0, 1, 5.5)
    g.dot(12, 19.5, 2.5)


@icon("step-over", A, "Step over")
def step_over(g):
    g.path("M4 14 A8 7.5 0 0 1 19 11")
    g.poly([(15, 12), (19.5, 12), (19.5, 7.5)])
    g.dot(12, 19, 2.5)


@icon("step-out", A, "Step out")
def step_out(g):
    g.line(12, 14, 12, 3)
    arrowhead(g, 12, 2.5, 0, -1, 5.5)
    g.dot(12, 19.5, 2.5)


@icon("step-back", A, "Step back")
def step_back(g):
    g.path("M20 14 A8 7.5 0 0 0 5 11")
    g.poly([(9, 12), (4.5, 12), (4.5, 7.5)])
    g.dot(12, 19, 2.5)


@icon("run-to-cursor", A, "Run to cursor")
def run_to_cursor(g):
    g.line(3, 12, 14, 12)
    arrowhead(g, 14.5, 12, 1, 0, 4.5)
    g.line(19.5, 5, 19.5, 19)
    g.line(17.5, 5, 21.5, 5) if False else None


# breakpoints and the execution point (the gutter's glyphs; colour)

@icon("breakpoint", A, "Breakpoint", mono=False)
def breakpoint(g):
    g.circle(12, 12, 6.5, c="red", fill="red")


@icon("breakpoint-disabled", A, "Breakpoint (disabled)", mono=False)
def breakpoint_disabled(g):
    g.circle(12, 12, 6.5, c="ink", fill="shade")


@icon("breakpoint-unverified", A, "Breakpoint (not yet bound)", mono=False)
def breakpoint_unverified(g):
    g.circle(12, 12, 6.5, c="red", fill="red-tint")


@icon("breakpoint-conditional", A, "Conditional breakpoint", mono=False)
def breakpoint_conditional(g):
    g.circle(12, 12, 6.5, c="red", fill="red")
    if g.size == 16:
        g.cut(lambda m: (m.frect(8.5, 9.5, 7, 1.4), m.frect(8.5, 13.6, 7, 1.4)), gap=0)
    else:
        g.cut(lambda m: (m.frect(8, 9, 8, 1.5), m.frect(8, 13.5, 8, 1.5)), gap=0)


@icon("logpoint", A, "Logpoint", mono=False)
def logpoint(g):
    g.poly([(12, 4.5), (19.5, 12), (12, 19.5), (4.5, 12)], close=True, c="red", fill="red")


@icon("breakpoint-hit", A, "Breakpoint, stopped here", mono=False)
def breakpoint_hit(g):
    g.circle(10, 12, 6.5, c="red", fill="red")
    pts = [(9, 8.5), (15, 8.5), (15, 5), (21.5, 12), (15, 19), (15, 15.5), (9, 15.5)]
    g.cut(lambda m: m.poly(pts, close=True, c="fg", fill="fg"))
    g.poly(pts, close=True, c="amber", fill="amber-solid")


@icon("execution-point", A, "Current statement", mono=False)
def execution_point(g):
    if g.size == 16:
        g.poly([(4, 9.5), (12, 9.5), (12, 5.5), (19.5, 12), (12, 18.5), (12, 14.5), (4, 14.5)], close=True, c="amber", fill="amber-solid")
    else:
        g.poly([(3.5, 8.5), (12, 8.5), (12, 4), (20.5, 12), (12, 20), (12, 15.5), (3.5, 15.5)], close=True, c="amber", fill="amber-solid")


@icon("bookmark", A, "Bookmark")
def bookmark(g):
    g.path("M6 3 H18 V21 L12 16 L6 21 Z")


@icon("watch", A, "Add watch")
def watch(g):
    g.path("M2.5 12 C5.5 6.5 9 4.5 12 4.5 C15 4.5 18.5 6.5 21.5 12 C18.5 17.5 15 19.5 12 19.5 C9 19.5 5.5 17.5 2.5 12 Z")
    g.circle(12, 12, 3)


# ---- designer --------------------------------------------------------------------

def bar_h(g, x, y, w, h=5):
    g.rect(x, y, w, h, r=1.5)


@icon("align-left", A, "Align left edges")
def align_left(g):
    g.line(3.5, 3, 3.5, 21)
    bar_h(g, 7, 5, 13)
    bar_h(g, 7, 14, 8)


@icon("align-center", A, "Align centres (horizontally)")
def align_center(g):
    g.line(12, 3, 12, 21)
    bar_h(g, 4.5, 5, 15)
    bar_h(g, 7.5, 14, 9)


@icon("align-right", A, "Align right edges")
def align_right(g):
    g.line(20.5, 3, 20.5, 21)
    bar_h(g, 4, 5, 13)
    bar_h(g, 9, 14, 8)


def bar_v(g, x, y, h, w=5):
    g.rect(x, y, w, h, r=1.5)


@icon("align-top", A, "Align top edges")
def align_top(g):
    g.line(3, 3.5, 21, 3.5)
    bar_v(g, 5, 7, 13)
    bar_v(g, 14, 7, 8)


@icon("align-middle", A, "Align middles (vertically)")
def align_middle(g):
    g.line(3, 12, 21, 12)
    bar_v(g, 5, 4.5, 15)
    bar_v(g, 14, 7.5, 9)


@icon("align-bottom", A, "Align bottom edges")
def align_bottom(g):
    g.line(3, 20.5, 21, 20.5)
    bar_v(g, 5, 4, 13)
    bar_v(g, 14, 9, 8)


@icon("distribute-horizontal", A, "Distribute horizontally")
def distribute_h(g):
    g.line(3.5, 3, 3.5, 21)
    g.line(20.5, 3, 20.5, 21)
    g.rect(9.5, 6, 5, 12, r=1.5)


@icon("distribute-vertical", A, "Distribute vertically")
def distribute_v(g):
    g.line(3, 3.5, 21, 3.5)
    g.line(3, 20.5, 21, 20.5)
    g.rect(6, 9.5, 12, 5, r=1.5)


def dbl_arrow_h(g, x0, x1, y, size=3):
    g.line(x0, y, x1, y)
    arrowhead(g, x0, y, -1, 0, size)
    arrowhead(g, x1, y, 1, 0, size)


def dbl_arrow_v(g, x, y0, y1, size=3):
    g.line(x, y0, x, y1)
    arrowhead(g, x, y0, 0, -1, size)
    arrowhead(g, x, y1, 0, 1, size)


@icon("same-width", A, "Make same width")
def same_width(g):
    dbl_arrow_h(g, 4, 20, 5)
    g.rect(4, 10, 16, 10, r=2)


@icon("same-height", A, "Make same height")
def same_height(g):
    dbl_arrow_v(g, 5, 4, 20)
    g.rect(10, 4, 10, 16, r=2)


@icon("same-size", A, "Make same size")
def same_size(g):
    dbl_arrow_h(g, 10, 20.5, 4.5)
    dbl_arrow_v(g, 4.5, 10, 20.5)
    g.rect(10, 10, 10.5, 10.5, r=2)


@icon("center-horizontally", A, "Centre horizontally in parent")
def center_h(g):
    g.rect(3, 4, 18, 16, r=2)
    g.rect(8.5, 9, 7, 6, r=1)
    g.line(12, 4, 12, 9)
    g.line(12, 15, 12, 20)


@icon("center-vertically", A, "Centre vertically in parent")
def center_v(g):
    g.rect(3, 4, 18, 16, r=2)
    g.rect(9, 8.5, 6, 7, r=1)
    g.line(3, 12, 9, 12)
    g.line(15, 12, 21, 12)


@icon("bring-to-front", A, "Bring to front")
def bring_front(g):
    g.rect(3, 3, 11, 11, r=2)
    g.cut(lambda m: m.rect(10, 10, 11, 11, r=2, fill="fg"))
    g.rect(10, 10, 11, 11, r=2, fill="fg")


@icon("send-to-back", A, "Send to back")
def send_back(g):
    g.rect(10, 10, 11, 11, r=2, fill="fg")
    g.cut(lambda m: m.rect(3, 3, 11, 11, r=2, fill="fg"))
    g.rect(3, 3, 11, 11, r=2)


@icon("tab-order", A, "Tab order")
def tab_order(g):
    g.line(3, 12, 16.5, 12)
    arrowhead(g, 17, 12, 1, 0, 5)
    g.line(21, 6, 21, 18)


@icon("menu-editor", A, "Menu editor")
def menu_editor(g):
    g.line(3, 4.5, 21, 4.5)
    g.rect(5, 8, 12, 13, r=1.5)
    g.line(8, 12, 14, 12)
    g.line(8, 16.5, 12, 16.5)


@icon("grid", A, "Show grid")
def grid(g):
    for x in (5, 12, 19):
        for y in (5, 12, 19):
            g.dot(x, y, 1.25)


@icon("snap", A, "Snap to grid")
def snap(g):
    g.path("M4 3 H9 V12 A3 3 0 0 0 15 12 V3 H20 V12 A8 8 0 0 1 4 12 Z")
    g.line(4, 7.5, 9, 7.5)
    g.line(15, 7.5, 20, 7.5)


@icon("guides", A, "Smart guides")
def guides(g):
    g.rect(8, 8, 9, 9, r=1.5)
    # the guides: dashed, along its top edge and its left edge
    for x in (3, 18.5):
        g.line(x, 8, x + 2.5, 8)
    for y in (3, 18.5):
        g.line(8, y, 8, y + 2.5)
    g.line(3, 4, 21, 4) if False else None


@icon("zoom-in", A, "Zoom in")
def zoom_in(g):
    magnifier(g)
    g.line(10.5, 7.5, 10.5, 13.5)
    g.line(7.5, 10.5, 13.5, 10.5)


@icon("zoom-out", A, "Zoom out")
def zoom_out(g):
    magnifier(g)
    g.line(7.5, 10.5, 13.5, 10.5)


@icon("zoom-fit", A, "Zoom to fit")
def zoom_fit(g):
    for (x, y, dx, dy) in ((3, 3, 1, 1), (21, 3, -1, 1), (3, 21, 1, -1), (21, 21, -1, -1)):
        g.poly([(x, y + 5 * dy), (x, y), (x + 5 * dx, y)])
    g.rect(8, 8, 8, 8, r=1)


# ---- docking and windows --------------------------------------------------------

@icon("dock", A, "Dock")
def dock(g):
    g.rect(3, 4, 18, 16, r=2)
    g.frect(3, 4, 7, 16, r=2)


@icon("float", A, "Float")
def float_(g):
    g.rect(3, 8, 14, 12, r=2)
    g.cut(lambda m: m.rect(9, 3, 12, 10, r=2, fill="fg"))
    window(g, 9, 3, 12, 10, bar=3.5)


@icon("pin", A, "Pin")
def pin(g):
    g.path("M8 3.5 H16 M9.5 3.5 V9.5 L6.5 14 H17.5 L14.5 9.5 V3.5")
    g.line(12, 14, 12, 21)


@icon("unpin", A, "Unpin")
def unpin(g):
    pts = rot([(8, 3.5), (16, 3.5)], 12, 12, 45)
    body = rot([(9.5, 3.5), (9.5, 9.5), (6.5, 14), (17.5, 14), (14.5, 9.5), (14.5, 3.5)], 12, 12, 45)
    g.line(*pts[0], *pts[1])
    g.poly(body)
    n = rot([(12, 14), (12, 21)], 12, 12, 45)
    g.line(*n[0], *n[1])


@icon("auto-hide", A, "Auto-hide")
def auto_hide(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(8, 4, 8, 20)
    g.line(17, 12, 11.5, 12)
    arrowhead(g, 11, 12, -1, 0, 3.5)


@icon("split-horizontal", A, "Split horizontally")
def split_h(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(3, 12, 21, 12)


@icon("split-vertical", A, "Split vertically")
def split_v(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(12, 4, 12, 20)


@icon("maximize", A, "Maximize")
def maximize(g):
    g.rect(4, 4, 16, 16, r=2)


@icon("restore", A, "Restore")
def restore(g):
    g.path("M8 6 V5.5 A2 2 0 0 1 10 3.5 H18.5 A2 2 0 0 1 20.5 5.5 V14 A2 2 0 0 1 18.5 16 H18")
    g.rect(3.5, 8, 12.5, 12.5, r=2)


@icon("minimize", A, "Minimize")
def minimize(g):
    g.line(5, 12, 19, 12)


@icon("cascade", A, "Cascade windows")
def cascade(g):
    g.path("M3 13 V5 A2 2 0 0 1 5 3 H13")
    g.path("M6.5 16.5 V8.5 A2 2 0 0 1 8.5 6.5 H16.5")
    window(g, 10, 10, 11, 11, bar=3.5)


@icon("tile-horizontal", A, "Tile horizontally")
def tile_h(g):
    g.rect(3, 3, 18, 8, r=2)
    g.rect(3, 13, 18, 8, r=2)


@icon("tile-vertical", A, "Tile vertically")
def tile_v(g):
    g.rect(3, 3, 8, 18, r=2)
    g.rect(13, 3, 8, 18, r=2)


@icon("layout", A, "Layout")
def layout(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(9, 4, 9, 20)
    g.line(9, 14, 21, 14)


# ---- general ----------------------------------------------------------------------

@icon("settings", A, "Settings")
def settings(g):
    if g.size == 16:
        gear(g, r_out=10, r_in=7.6, teeth=6, hole=3.6, tip=0.20, root=0.30)
    else:
        gear(g)


@icon("filter", A, "Filter")
def filter_(g):
    g.path("M3.5 4 H20.5 L14 12 V18.5 L10 20.5 V12 Z")


@icon("refresh", A, "Refresh")
def refresh(g):
    g.path("M19.5 12 A7.5 7.5 0 1 1 16.5 6")
    g.poly([(17, 2.5), (17, 6.5), (13, 6.5)])


@icon("expand", A, "Expand")
def expand(g):
    g.poly([(7, 9), (12, 4), (17, 9)])
    g.poly([(7, 15), (12, 20), (17, 15)])


@icon("collapse", A, "Collapse")
def collapse(g):
    g.poly([(7, 4), (12, 9), (17, 4)])
    g.poly([(7, 20), (12, 15), (17, 20)])


@icon("expand-all", A, "Expand all")
def expand_all(g):
    g.rect(8, 8, 13, 13, r=2)
    g.path("M4 15 V5 A2 2 0 0 1 6 3 H16")
    g.line(14.5, 11, 14.5, 18)
    g.line(11, 14.5, 18, 14.5)


@icon("collapse-all", A, "Collapse all")
def collapse_all(g):
    g.rect(8, 8, 13, 13, r=2)
    g.path("M4 15 V5 A2 2 0 0 1 6 3 H16")
    g.line(11, 14.5, 18, 14.5)


@icon("add", A, "Add")
def add(g):
    g.line(12, 4.5, 12, 19.5)
    g.line(4.5, 12, 19.5, 12)


@icon("remove", A, "Remove")
def remove(g):
    g.line(4.5, 12, 19.5, 12)


@icon("delete", A, "Delete")
def delete(g):
    g.line(3.5, 6, 20.5, 6)
    g.path("M9 6 V3.5 H15 V6")
    g.path("M5.5 6 L6.5 20.5 H17.5 L18.5 6")
    g.line(10, 10, 10, 16.5)
    g.line(14, 10, 14, 16.5)


@icon("edit", A, "Edit")
def edit(g):
    g.path("M15 4.5 L19.5 9 L9 19.5 L3.5 20.5 L4.5 15 Z")
    g.line(12.5, 7, 17, 11.5)


@icon("rename", A, "Rename")
def rename(g):
    g.rect(3, 7, 18, 10, r=2)
    g.line(14, 4, 14, 20)
    g.line(12, 4, 16, 4)
    g.line(12, 20, 16, 20)
    g.line(6.5, 12, 10, 12)


@icon("more", A, "More")
def more(g):
    for x in (5, 12, 19):
        g.dot(x, 12, 1.75)


@icon("menu", A, "Menu")
def menu(g):
    g.line(4, 6, 20, 6)
    g.line(4, 12, 20, 12)
    g.line(4, 18, 20, 18)


@icon("help", A, "Help")
def help_(g):
    g.circle(12, 12, 9.5)
    g.path("M9.25 9.5 A2.75 2.75 0 1 1 13 12 C12.4 12.3 12 12.8 12 13.6 V14")
    g.dot(12, 17.25, 1.25)


@icon("history", A, "History")
def history(g):
    g.path("M4.5 12 A7.5 7.5 0 1 0 6.7 6.7")
    g.poly([(3, 4.5), (3, 8.5), (7, 8.5)]) if False else g.poly([(6.5, 3), (6.5, 7), (2.5, 7)])
    g.path("M12 8 V12 L15 14")


@icon("link", A, "Link")
def link(g):
    g.path("M10.5 13.5 A3.75 3.75 0 0 0 16 13.7 L19 10.7 A3.9 3.9 0 0 0 13.3 5 L12 6.3")
    g.path("M13.5 10.5 A3.75 3.75 0 0 0 8 10.3 L5 13.3 A3.9 3.9 0 0 0 10.7 19 L12 17.7")


@icon("unlink", A, "Unlink")
def unlink(g):
    g.path("M16 13.7 L19 10.7 A3.9 3.9 0 0 0 13.3 5 L12 6.3")
    g.path("M8 10.3 L5 13.3 A3.9 3.9 0 0 0 10.7 19 L12 17.7")
    g.line(3.5, 3.5, 6, 6)
    g.line(18, 18, 20.5, 20.5)


@icon("visible", A, "Visible")
def visible(g):
    watch(g)


@icon("hidden", A, "Hidden")
def hidden(g):
    g.path("M2.5 12 C5.5 6.5 9 4.5 12 4.5 C15 4.5 18.5 6.5 21.5 12 C18.5 17.5 15 19.5 12 19.5 C9 19.5 5.5 17.5 2.5 12 Z")
    g.circle(12, 12, 3)
    g.cut(lambda m: m.line(4, 4, 20, 20, c="fg"))
    g.line(4, 4, 20, 20)


@icon("lock", A, "Locked")
def lock(g):
    g.rect(5, 10.5, 14, 10, r=2)
    g.path("M8 10.5 V8 A4 4 0 0 1 16 8 V10.5")


@icon("unlock", A, "Unlocked")
def unlock(g):
    g.rect(5, 10.5, 14, 10, r=2)
    g.path("M8 10.5 V8 A4 4 0 0 1 15.7 6.5")


@icon("back", A, "Back")
def back(g):
    g.line(20, 12, 4.5, 12)
    arrowhead(g, 4, 12, -1, 0, 6)


@icon("forward", A, "Forward")
def forward(g):
    g.line(4, 12, 19.5, 12)
    arrowhead(g, 20, 12, 1, 0, 6)


@icon("terminal", A, "Terminal")
def terminal(g):
    g.rect(3, 4, 18, 16, r=2)
    g.poly([(7, 9), (10, 12), (7, 15)])
    g.line(12, 15, 16, 15)


@icon("command-palette", A, "Command palette")
def command_palette(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(3, 9, 21, 9)
    g.poly([(6.5, 12.5), (9, 14.75), (6.5, 17)]) if g.size != 16 else g.poly([(6.5, 12), (9, 14.5), (6.5, 17)])
    g.line(11, 17, 17, 17) if False else g.line(11.5, 16.5, 17, 16.5)


@icon("keyboard", A, "Keyboard shortcuts")
def keyboard(g):
    g.rect(2.5, 6, 19, 12, r=2)
    for x in (6, 9.5, 13, 16.5) if g.size != 16 else (6, 10, 14):
        g.dot(x if g.size != 16 else x + 0.5, 10, 0.9)
    g.line(7.5, 14.5, 16.5, 14.5)


@icon("clear", A, "Clear")
def clear(g):
    # an eraser at 45 degrees and the line it leaves
    body = rot([(6, 9), (18, 9), (18, 15), (6, 15)], 12, 12, -45)
    g.poly(body, close=True)
    mid = rot([(12, 9), (12, 15)], 12, 12, -45)
    g.line(*mid[0], *mid[1])
    g.line(13, 21, 21, 21)


@icon("notifications", A, "Notifications")
def notifications(g):
    g.path("M6 17 V10.5 A6 6 0 0 1 18 10.5 V17 L19.5 18.5 H4.5 Z")
    g.path("M10 21 H14")


@icon("output", A, "Output")
def output(g):
    g.rect(3, 4, 18, 16, r=2)
    g.line(7, 9, 17, 9)
    g.line(7, 12.5, 14, 12.5)
    g.line(7, 16, 16, 16)


@icon("problems", A, "Problems", mono=False)
def problems(g):
    g.circle(9, 9, 6, c="red", fill="red-tint")
    g.line(7, 7, 11, 11, c="red") if g.size != 16 else g.line(7.5, 7.5, 10.5, 10.5, c="red")
    g.line(11, 7, 7, 11, c="red") if g.size != 16 else g.line(10.5, 7.5, 7.5, 10.5, c="red")
    tri = [(16, 11), (21.5, 20.5), (10.5, 20.5)]
    g.cut(lambda m: m.poly(tri, close=True, c="fg", fill="fg"))
    g.poly(tri, close=True, c="amber", fill="amber-tint")


# ---- statuses ---------------------------------------------------------------------

@icon("error", A, "Error", mono=False)
def error(g):
    g.circle(12, 12, 9, c="red", fill="red-tint")
    g.line(8.5, 8.5, 15.5, 15.5, c="red")
    g.line(15.5, 8.5, 8.5, 15.5, c="red")


@icon("warning", A, "Warning", mono=False)
def warning(g):
    g.poly([(12, 3), (21.5, 20), (2.5, 20)], close=True, c="amber", fill="amber-tint")
    g.line(12, 9, 12, 13.5, c="amber")
    g.dot(12, 16.75, 1, fill="amber")


@icon("info", A, "Information", mono=False)
def info(g):
    g.circle(12, 12, 9, c="blue", fill="blue-tint")
    g.line(12, 11, 12, 16.5, c="blue")
    g.dot(12, 7.75, 1, fill="blue")


@icon("hint", A, "Hint", mono=False)
def hint(g):
    g.path("M9 16.5 C9 14 6 12.5 6 9 A6 6 0 0 1 18 9 C18 12.5 15 14 15 16.5 Z", c="cyan", fill="cyan-tint")
    g.line(9.5, 20, 14.5, 20, c="cyan")


@icon("success", A, "Success", mono=False)
def success(g):
    g.circle(12, 12, 9, c="teal", fill="teal-tint")
    g.poly([(7.5, 12.5), (10.5, 15.5), (16.5, 9)], c="teal")


@icon("ai", A, "AI assistant", mono=False)
def ai(g):
    grad = g.gradient(3, 21, 21, 3, [(0, "blue"), (1, "spark")])
    star4(g, 10, 13.5, 8, fill=grad)
    star4(g, 18.5, 5.5, 3.5, fill=grad)


@icon("ai-chat", A, "Ask the AI", mono=False)
def ai_chat(g):
    g.path("M4 5 A2 2 0 0 1 6 3 H18 A2 2 0 0 1 20 5 V14 A2 2 0 0 1 18 16 H10 L5.5 20 V16 H6 A2 2 0 0 1 4 14 Z", c="ink")
    grad = g.gradient(6, 14, 18, 5, [(0, "blue"), (1, "spark")])
    star4(g, 12, 9.5, 4.5, fill=grad)
