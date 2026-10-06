"""Components planned by the IDE plan (docs/ide-components.md): the IDE's
public components (§3), the linked data components of stage I7 (§4), RAI,
and RPLOT's chart kinds. Same rules as components.py."""

from kit import icon
from motifs import arrowhead, page, folder, star4, cylinder
from components import frame, bar_window, db_badge, edit, memo, checkbox, combobox, stringgrid, letter_a

C = "components"


def rglyph(g, x, y, h, fill="blue", counter=None):
    """The brand's R (design/brand/src/marks.py, run_r), `h` tall, its top
    left at (x, y), with its play-triangle counter cut out (or filled)."""
    k = h / 62

    def X(v):
        return f"{x + (v - 27) * k:.3f}"

    def Y(v):
        return f"{y + (v - 19) * k:.3f}"
    r = f"{20 * k:.3f}"
    counter_d = f"M{X(43)} {Y(31)} V{Y(47)} L{X(62)} {Y(39)} Z"
    d = (f"M{X(27)} {Y(19)} H{X(55)} A{r} {r} 0 0 1 {X(55)} {Y(59)} H{X(43)} V{Y(81)} H{X(27)} Z "
         f"{counter_d} M{X(43)} {Y(59)} H{X(59)} L{X(76)} {Y(81)} H{X(60)} Z")
    g.fpath(d, fill=fill, evenodd=True)
    if counter:
        g.fpath(counter_d, fill=counter)


# ---- the IDE's public components ---------------------------------------------------

@icon("dockmanager", C, "Dock manager")
def dockmanager(g):
    frame(g)
    g.path("M3 6 A2 2 0 0 1 5 4 H9 V20 H5 A2 2 0 0 1 3 18 Z", c="ink", fill="blue-tint")
    g.path("M9 15 H21 V18 A2 2 0 0 1 19 20 H9 Z", c="ink", fill="blue-tint")


@icon("formdesigner", C, "Form designer")
def formdesigner(g):
    bar_window(g, bar=4)
    g.rect(7.5, 10.5, 9, 5.5, c="blue", fill="blue-tint")
    for x in (7.5, 16.5):
        for y in (10.5, 16):
            g.frect(x - 1.25, y - 1.25, 2.5, 2.5, fill="blue")


@icon("propertyinspector", C, "Property inspector")
def propertyinspector(g):
    frame(g, 3, 3, 18, 18)
    g.path("M3 5 A2 2 0 0 1 5 3 H10 V21 H5 A2 2 0 0 1 3 19 Z", c="ink", fill="blue-tint")
    for y in (9, 15):
        g.line(3, y, 21, y, c="ink")
    if g.size != 16:
        g.line(12.5, 6, 18, 6, c="blue")
        g.line(12.5, 18, 17.5, 18, c="blue")
    g.line(12.5, 12, 16.5, 12, c="blue")


@icon("projecttree", C, "Project tree")
def projecttree(g):
    g.path("M3 8 V3 H6.5 L7.5 4 H11 V8 Z", c="blue", fill="blue-tint")
    g.path("M6 8 V17.5 H11 M6 12.5 H11", c="ink")
    g.rect(11, 10.5, 9, 4, r=1, c="ink", fill="paper")
    g.rect(11, 15.5, 9, 4, r=1, c="ink", fill="paper")


@icon("toolbox", C, "Toolbox")
def toolbox(g):
    g.path("M9 8 V5.5 A1.5 1.5 0 0 1 10.5 4 H13.5 A1.5 1.5 0 0 1 15 5.5 V8", c="ink")
    g.rect(2.5, 8, 19, 12.5, r=2, c="blue", fill="blue-tint")
    g.line(2.5, 13, 21.5, 13, c="blue")
    if g.size != 16:
        g.rect(10, 11.5, 4, 3, r=0.5, c="blue", fill="paper")


@icon("outputconsole", C, "Output console")
def outputconsole(g):
    frame(g)
    g.poly([(7, 9), (10, 12), (7, 15)], c="blue")
    g.line(12, 15, 17, 15, c="ink")


@icon("commandpalette", C, "Command palette")
def commandpalette(g):
    g.rect(2.5, 3, 19, 6.5, r=2, c="blue", fill="blue-tint")
    g.line(5.5, 6.25, 15, 6.25, c="blue")
    g.rect(2.5, 11.5, 19, 9.5, r=2, c="ink", fill="paper")
    g.line(5.5, 16.25, 15, 16.25, c="ink")


@icon("diffview", C, "Diff view")
def diffview(g):
    frame(g, 3, 3, 8, 18, r=1.5)
    frame(g, 13, 3, 8, 18, r=1.5)
    g.frect(3.75, 9.5, 6.5, 4, fill="red-tint")
    g.frect(13.75, 9.5, 6.5, 4, fill="teal-tint")
    g.line(5.5, 11.5, 8.5, 11.5, c="red")
    g.line(15.5, 11.5, 18.5, 11.5, c="teal")
    if g.size != 16:
        g.line(17, 10, 17, 13, c="teal")


@icon("componenttray", C, "Component tray")
def componenttray(g):
    g.path("M2.5 13 V18.5 A2 2 0 0 0 4.5 20.5 H19.5 A2 2 0 0 0 21.5 18.5 V13", c="ink")
    g.circle(6, 8.5, 3, c="amber", fill="amber-tint")
    g.rect(9.5, 5.5, 5.5, 6, r=1, c="teal", fill="teal-tint")
    g.poly([(18.5, 5.5), (21.5, 11.5), (15.5, 11.5)], close=True, c="blue", fill="blue-tint")


@icon("tabordereditor", C, "Tab order editor")
def tabordereditor(g):
    for i, y in enumerate((3, 9.5, 16)):
        g.rect(9, y, 12, 5, r=1.5, c="blue" if i == 1 else "ink", fill="blue-tint" if i == 1 else "paper")
    g.line(5, 5, 5, 18.5, c="ink")
    arrowhead(g, 5, 19, 0, 1, 3, c="ink")


@icon("menueditor", C, "Menu editor")
def menueditor(g):
    frame(g, 2.5, 3, 19, 5, r=1.5)
    g.frect(3.5, 3.5, 6.5, 4, fill="blue-tint")
    g.rect(4, 8, 9.5, 13, r=1.5, c="ink", fill="paper")
    g.line(6.5, 12, 11, 12, c="ink")
    pen = [(18.5, 10), (21, 12.5), (15, 18.5), (12.5, 19), (13, 16.5)]
    g.cut(lambda m: m.poly(pen, close=True, c="fg", fill="fg"), gap=0.75)
    g.poly(pen, close=True, c="blue", fill="blue-tint")


@icon("programview", C, "Program view")
def programview(g):
    bar_window(g)
    g.poly([(10, 10.5), (15.5, 13.5), (10, 16.5)], close=True, c="teal", fill="teal")


@icon("programsession", C, "Program session")
def programsession(g):
    g.line(4, 8, 18.5, 8, c="teal")
    arrowhead(g, 19, 8, 1, 0, 3.5, c="teal")
    g.line(20, 16, 5.5, 16, c="blue")
    arrowhead(g, 5, 16, -1, 0, 3.5, c="blue")


@icon("project", C, "Project")
def project(g):
    folder(g, 2.5, 4, 19, 15, c="blue", fill="blue-tint")
    rglyph(g, 9.5, 10.5, 7)


@icon("breakpointlist", C, "Breakpoint list")
def breakpointlist(g):
    frame(g, 3, 3, 18, 18)
    for y in ((7.5, 12, 16.5) if g.size != 16 else (8, 15)):
        g.dot(7, y, 1.6, fill="red")
        g.line(10.5, y, 17.5, y, c="ink")


@icon("callstackview", C, "Call stack")
def callstackview(g):
    g.rect(3, 3, 18, 5, r=1.5, c="blue", fill="blue-tint")
    g.rect(3, 9.5, 18, 5, r=1.5, c="ink", fill="paper")
    g.rect(3, 16, 18, 5, r=1.5, c="ink", fill="paper")


@icon("variablesview", C, "Variables")
def variablesview(g):
    g.path("M8 3.5 C6 3.5 5.5 4.5 5.5 6.5 V9.5 C5.5 11 4.5 12 3 12 C4.5 12 5.5 13 5.5 14.5 V17.5 C5.5 19.5 6 20.5 8 20.5", c="ink")
    g.path("M16 3.5 C18 3.5 18.5 4.5 18.5 6.5 V9.5 C18.5 11 19.5 12 21 12 C19.5 12 18.5 13 18.5 14.5 V17.5 C18.5 19.5 18 20.5 16 20.5", c="ink")
    g.line(9.5, 9, 14.5, 15, c="blue")
    g.line(14.5, 9, 9.5, 15, c="blue")


@icon("immediatewindow", C, "Immediate window")
def immediatewindow(g):
    frame(g)
    g.path("M9.25 9.5 A2.75 2.75 0 1 1 13 12 C12.4 12.3 12 12.8 12 13.6 V14", c="blue")
    g.dot(12, 17, 1.1, fill="blue")


@icon("datapreview", C, "Data preview")
def datapreview(g):
    g.rect(3, 3, 15, 15, r=2, c="teal", fill="paper")
    g.path("M3 7.5 V5 A2 2 0 0 1 5 3 H16 A2 2 0 0 1 18 5 V7.5 Z", c="teal", fill="teal-tint")
    g.line(10.5, 7.5, 10.5, 18, c="teal")
    g.cut(lambda m: m.circle(16.5, 16.5, 4, c="fg", fill="fg"), gap=0.75)
    g.circle(16.5, 16.5, 3.5, c="ink", fill="paper")
    g.line(19, 19, 21.5, 21.5, c="ink")


@icon("aichat", C, "AI chat")
def aichat(g):
    g.path("M4 5 A2 2 0 0 1 6 3 H18 A2 2 0 0 1 20 5 V14 A2 2 0 0 1 18 16 H10 L5.5 20 V16 H6 A2 2 0 0 1 4 14 Z", c="ink", fill="paper")
    grad = g.gradient(6, 14, 18, 5, [(0, "blue"), (1, "spark")])
    star4(g, 12, 9.5, 4.5, fill=grad)


@icon("ai", C, "RAI (AI)")
def ai_comp(g):
    grad = g.gradient(3, 21, 21, 3, [(0, "blue"), (1, "spark")])
    star4(g, 10, 13.5, 8, fill=grad)
    star4(g, 18.5, 5.5, 3.5, fill=grad)


# ---- linked data (stage I7) --------------------------------------------------------------

@icon("dbconnection", C, "Database connection")
def dbconnection(g):
    cylinder(g, 2.5, 3, 11, 14, c="teal", fill="teal-tint", bands=0)
    g.path("M8 17 V19.5 H13.5", c="ink")
    g.path("M13.5 16.5 H16.5 A3 3 0 0 1 16.5 22.5 H13.5 Z", c="teal", fill="teal-tint")
    if g.size != 16:
        g.line(19.5, 18, 21.5, 18, c="teal")
        g.line(19.5, 21, 21.5, 21, c="teal")


@icon("datafile", C, "Data file (CSV, JSON)")
def datafile(g):
    page(g, 4.5, 2, 15, 20, fold=5, c="ink", fill="paper")
    g.rect(7.5, 10, 9, 8.5, c="teal", fill="teal-tint")
    g.line(7.5, 14.25, 16.5, 14.25, c="teal")
    g.line(12, 10, 12, 18.5, c="teal")


@icon("dbquery", C, "Database query")
def dbquery(g):
    cylinder(g, 3, 3, 14, 16, c="teal", fill="teal-tint", bands=1 if g.size != 16 else 0)
    g.cut(lambda m: m.circle(16.5, 16.5, 4, c="fg", fill="fg"), gap=0.75)
    g.circle(16.5, 16.5, 3.5, c="ink", fill="paper")
    g.line(19, 19, 21.5, 21.5, c="ink")


@icon("dbtable", C, "Database table")
def dbtable(g):
    frame(g, c="teal")
    g.path("M3 8.5 V6 A2 2 0 0 1 5 4 H19 A2 2 0 0 1 21 6 V8.5 Z", c="teal", fill="teal-tint")
    g.line(3, 14.25, 21, 14.25, c="teal")
    g.line(10, 8.5, 10, 20, c="teal")
    db_badge(g)


@icon("dffilter", C, "Data frame: filter")
def dffilter(g):
    g.path("M3.5 4 H20.5 L14 12 V18.5 L10 20.5 V12 Z", c="teal", fill="teal-tint")


@icon("dfsort", C, "Data frame: sort")
def dfsort(g):
    for i, w in enumerate((10, 7, 4)):
        y = 5 + i * 6.5
        g.line(3, y, 3 + w, y, c="teal")
    g.line(18, 3.5, 18, 19.5, c="ink")
    arrowhead(g, 18, 20, 0, 1, 3.5, c="ink")


@icon("dfgroup", C, "Data frame: group and aggregate")
def dfgroup(g):
    g.path("M18 4.5 H5.5 L12 12 L5.5 19.5 H18", c="teal")


@icon("dfjoin", C, "Data frame: join")
def dfjoin(g):
    g.circle(9, 12, 6, c="teal", fill="teal-tint")
    g.circle(15, 12, 6, c="teal")


@icon("dfcompute", C, "Data frame: computed column")
def dfcompute(g):
    g.path("M13 4 C10.5 3.5 9.5 4.5 9 7 L7 17 C6.5 19.5 5.5 20.5 3 20", c="teal")
    g.line(5.5, 9.5, 12, 9.5, c="teal")
    g.line(14, 12, 20.5, 19, c="ink")
    g.line(20.5, 12, 14, 19, c="ink")


@icon("dfselect", C, "Data frame: select columns")
def dfselect(g):
    g.rect(3, 4, 5, 16, r=1.5, c="teal", fill="teal-tint")
    g.rect(9.5, 4, 5, 16, r=1.5, c="ink", fill="paper")
    g.rect(16, 4, 5, 16, r=1.5, c="teal", fill="teal-tint")


@icon("dflimit", C, "Data frame: limit rows")
def dflimit(g):
    g.line(3, 4.5, 21, 4.5, c="teal")
    g.line(3, 9, 21, 9, c="teal")
    g.line(3, 13.5, 21, 13.5, c="teal")
    for x in ((3, 8, 13, 18) if g.size != 16 else (3, 9.5, 16)):
        g.line(x, 19, x + 2.5, 19, c="ink")


@icon("datasource", C, "Data source")
def datasource(g):
    g.line(12, 12, 5, 5, c="ink")
    g.line(12, 12, 19, 5, c="ink")
    g.line(12, 12, 12, 19.5, c="ink")
    g.circle(12, 12, 4, c="teal", fill="teal-tint")
    for x, y in ((5, 5), (19, 5), (12, 19.5)):
        g.dot(x, y, 2, fill="teal")


@icon("dbgrid", C, "Data-aware grid")
def dbgrid(g):
    stringgrid(g)
    db_badge(g)


@icon("dbedit", C, "Data-aware edit")
def dbedit(g):
    edit(g)
    db_badge(g)


@icon("dblabel", C, "Data-aware label")
def dblabel(g):
    letter_a(g, 3.5, 16, 10.5, 12)
    g.line(3.5, 20, 11, 20, c="ink")
    db_badge(g)


@icon("dbmemo", C, "Data-aware memo")
def dbmemo(g):
    memo(g)
    db_badge(g)


@icon("dbcheckbox", C, "Data-aware check box")
def dbcheckbox(g):
    checkbox(g)
    db_badge(g)


@icon("dbcombobox", C, "Data-aware combo box")
def dbcombobox(g):
    combobox(g)
    db_badge(g)


@icon("dblookupcombo", C, "Data-aware lookup combo box")
def dblookupcombo(g):
    g.rect(2.5, 6.5, 19, 11, r=2, c="ink", fill="paper")
    g.circle(8, 11.5, 2.5, c="blue")
    g.line(9.75, 13.25, 11.5, 15, c="blue")
    db_badge(g)


@icon("dbnavigator", C, "Data navigator")
def dbnavigator(g):
    # first and last: a bar and a solid arrow each
    g.rect(2.5, 6, 19, 12, r=2, c="ink", fill="paper")
    g.frect(5.5, 9, 1.5, 6, fill="teal")
    g.fpoly([(11.5, 9), (7.5, 12), (11.5, 15)], fill="teal")
    g.fpoly([(12.5, 9), (16.5, 12), (12.5, 15)], fill="teal")
    g.frect(17, 9, 1.5, 6, fill="teal")


# ---- chart kinds (RPLOT's Kind) ---------------------------------------------------------------

def axes(g):
    g.path("M3.5 3.5 V20.5 H20.5", c="ink")


@icon("chart-line", C, "Chart: line")
def chart_line(g):
    axes(g)
    g.path("M6.5 16 L10.5 10.5 L14 13.5 L19.5 6", c="amber")


@icon("chart-bar", C, "Chart: bar")
def chart_bar(g):
    axes(g)
    for x, h in ((7.5, 7), (12.5, 12.5), (17.5, 9)):
        if g.size == 16:
            g.frect(x - 1.5, 20.5 - h, 3, h, fill="teal")
        else:
            g.rect(x - 1.5, 20.5 - h, 3, h, c="teal", fill="teal-tint")


@icon("chart-barh", C, "Chart: horizontal bar")
def chart_barh(g):
    axes(g)
    for y, w in ((6.5, 9), (11.5, 14), (16.5, 6)):
        if g.size == 16:
            g.frect(3.5, y - 1.5, w, 3, fill="teal")
        else:
            g.rect(3.5, y - 1.5, w, 3, c="teal", fill="teal-tint")


@icon("chart-scatter", C, "Chart: scatter")
def chart_scatter(g):
    axes(g)
    for x, y in ((7.5, 15.5), (11, 10.5), (15, 14), (18, 7)):
        g.dot(x, y, 1.6, fill="amber")


@icon("chart-area", C, "Chart: area")
def chart_area(g):
    axes(g)
    g.path("M3.5 17 L9 11 L13.5 14 L20.5 6 V20.5 H3.5 Z", c="amber", fill="amber-tint")


@icon("chart-step", C, "Chart: step")
def chart_step(g):
    axes(g)
    g.path("M6 17 H10 V12 H14 V14.5 H17 V7 H20", c="amber")


@icon("chart-hist", C, "Chart: histogram")
def chart_hist(g):
    axes(g)
    g.path("M5.5 20.5 V15 H9 V9.5 H12.5 V6 H16 V11 H19.5 V20.5", c="teal", fill="teal-tint")


@icon("chart-pie", C, "Chart: pie")
def chart_pie(g):
    g.circle(12, 12, 9, c="teal", fill="teal-tint")
    g.path("M12 3 V12 H21", c="teal")
    g.path("M12 3 A9 9 0 0 1 21 12 H12 Z", c="teal", fill="teal")
