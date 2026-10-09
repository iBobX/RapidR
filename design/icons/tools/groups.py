"""The toolbox's groups (RToolbox, docs/ide-plan.md I1), by purpose
(Standard, Additional, Dialogs, System, Network, Data, Data Science, Media,
DirectX, Direct3D, Web, AI, IDE, Templates), each in its family's hue; and
RapidR's own mark."""

import math

from kit import icon
from motifs import cylinder, star4, cube_iso
from planned import rglyph

GR = "groups"


@icon("rapidr", GR, "RapidR components")
def rapidr(g):
    grad = g.gradient(3, 21, 21, 3, [(0, "blue"), (1, "spark")])
    g.frect(2.5, 2.5, 19, 19, r=4.5, fill=grad)
    # the R knocked out of the tile, as the brand's one-colour mark
    g.cut(lambda m: rglyph(m, 7.5, 5.5, 13, fill="fg"), gap=0)


@icon("standard", GR, "Standard")
def standard(g):
    g.rect(2.5, 7, 19, 10, r=3, c="blue", fill="blue-tint")
    g.line(8, 12, 16, 12, c="blue")


@icon("additional", GR, "Additional")
def additional(g):
    for x, y, hue in ((3, 3, "blue"), (13, 3, None), (3, 13, None)):
        g.rect(x, y, 8, 8, r=2, c=hue or "ink", fill=(hue + "-tint") if hue else "paper")
    g.line(17, 13.5, 17, 20.5, c="blue")
    g.line(13.5, 17, 20.5, 17, c="blue")


@icon("dialogs", GR, "Dialogs")
def dialogs(g):
    g.rect(2.5, 3, 19, 18, r=2, c="ink", fill="paper")
    g.path("M2.5 7 V5 A2 2 0 0 1 4.5 3 H19.5 A2 2 0 0 1 21.5 5 V7 Z", c="ink", fill="cyan-tint")
    g.rect(12.5, 14.5, 6, 3.5, r=1, c="cyan", fill="cyan-tint")


@icon("system", GR, "System")
def system(g):
    g.rect(6, 6, 12, 12, r=2, c="ink", fill="paper")
    g.rect(9.5, 9.5, 5, 5, r=0.5, c="blue", fill="blue-tint")
    for v in ((9.5, 14.5) if g.size != 16 else (12,)):
        g.line(v, 2.5, v, 6, c="ink")
        g.line(v, 18, v, 21.5, c="ink")
        g.line(2.5, v, 6, v, c="ink")
        g.line(18, v, 21.5, v, c="ink")


@icon("media", GR, "Media")
def media(g):
    g.path("M9 18 V5.5 L19 3.5 V15.5", c="violet")
    g.circle(6.5, 18, 2.5, c="violet", fill="violet-tint")
    g.circle(16.5, 15.5, 2.5, c="violet", fill="violet-tint")


@icon("directx", GR, "DirectX")
def directx(g):
    g.path("M7 7 H17 A5 5 0 0 1 21.5 14 L20.5 17 A2.5 2.5 0 0 1 16.5 18 L14.5 16 H9.5 L7.5 18 A2.5 2.5 0 0 1 3.5 17 L2.5 14 A5 5 0 0 1 7 7 Z", c="violet", fill="violet-tint")
    g.line(6, 11.5, 9, 11.5, c="violet")
    g.line(7.5, 10, 7.5, 13, c="violet")
    g.dot(16.5, 11.5, 1.25, fill="violet")


@icon("direct3d", GR, "Direct3D")
def direct3d(g):
    cube_iso(g, c="violet", fill="violet-tint")


@icon("data", GR, "Data")
def data(g):
    cylinder(g, 4.5, 3, 15, 18, c="teal", fill="teal-tint", bands=1 if g.size != 16 else 0)


@icon("datascience", GR, "Data science")
def datascience(g):
    g.path("M3.5 3.5 V20.5 H20.5", c="ink")
    for x, h in ((8, 6), (12.5, 11), (17, 15)):
        if g.size == 16:
            g.frect(x - 1.5, 20.5 - h, 3, h, fill="teal")
        else:
            g.rect(x - 1.5, 20.5 - h, 3, h, c="teal", fill="teal-tint")


@icon("web", GR, "Web")
def web(g):
    g.circle(12, 12, 9, c="cyan", fill="cyan-tint")
    g.ellipse(12, 12, 4, 9, c="cyan")
    g.line(3, 12, 21, 12, c="cyan")


@icon("ai", GR, "AI")
def ai(g):
    grad = g.gradient(3, 21, 21, 3, [(0, "blue"), (1, "spark")])
    star4(g, 10, 13.5, 8, fill=grad)
    star4(g, 18.5, 5.5, 3.5, fill=grad)


@icon("ide", GR, "IDE")
def ide(g):
    g.rect(3, 4, 18, 16, r=2, c="ink", fill="paper")
    g.path("M3 6 A2 2 0 0 1 5 4 H9 V20 H5 A2 2 0 0 1 3 18 Z", c="ink", fill="blue-tint")
    g.line(11.5, 9, 18, 9, c="blue")
    g.line(13.5, 12.5, 18, 12.5, c="ink")
    g.line(11.5, 16, 16, 16, c="ink")


@icon("templates", GR, "Templates")
def templates(g):
    # a dashed sheet: a component made from a template
    for x0, y0, x1, y1 in ((5, 2.5, 8, 2.5), (11, 2.5, 13, 2.5), (5, 21.5, 8, 21.5), (11, 21.5, 14, 21.5),
                           (4.5, 6, 4.5, 9), (4.5, 13, 4.5, 16), (19.5, 13, 19.5, 16)):
        g.line(x0, y0, x1, y1, c="ink")
    g.path("M16 2.5 L19.5 6 V9", c="ink")
    g.line(4.5, 19, 4.5, 21.5, c="ink") if g.size != 16 else None
    g.line(17, 21.5, 19.5, 21.5, c="ink")
    g.line(19.5, 19, 19.5, 21.5, c="ink") if g.size != 16 else None
    g.rect(8.5, 9, 7, 7, r=1.5, c="blue", fill="blue-tint")


@icon("controls", GR, "Controls")
def controls(g):
    g.rect(2.5, 4, 19, 7, r=2, c="blue", fill="blue-tint")
    g.line(6, 7.5, 12, 7.5, c="blue")
    g.line(3, 17, 21, 17, c="ink")
    g.circle(15, 17, 2.5, c="blue", fill="paper")


@icon("network", GR, "Network")
def network(g):
    g.path("M12 7 V12 M12 12 L5.5 17.5 M12 12 L18.5 17.5", c="ink")
    g.circle(12, 5, 2.5, c="cyan", fill="cyan-tint")
    g.circle(5, 18.5, 2.5, c="cyan", fill="cyan-tint")
    g.circle(19, 18.5, 2.5, c="cyan", fill="cyan-tint")
