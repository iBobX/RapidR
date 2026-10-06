"""Glyphs: the small marks inside controls (a tree's twisty, a combo box's
arrow, sort arrows, grips, a check). Monochrome, drawn on the same grid as
every icon so they line up with them."""

from kit import icon
from motifs import arrowhead

GL = "glyphs"


@icon("chevron-right", GL, "Chevron right")
def chevron_right(g):
    g.poly([(9.5, 6), (15.5, 12), (9.5, 18)])


@icon("chevron-left", GL, "Chevron left")
def chevron_left(g):
    g.poly([(14.5, 6), (8.5, 12), (14.5, 18)])


@icon("chevron-down", GL, "Chevron down")
def chevron_down(g):
    g.poly([(6, 9.5), (12, 15.5), (18, 9.5)])


@icon("chevron-up", GL, "Chevron up")
def chevron_up(g):
    g.poly([(6, 14.5), (12, 8.5), (18, 14.5)])


@icon("caret-right", GL, "Caret right (solid)")
def caret_right(g):
    g.poly([(9.5, 7), (15.5, 12), (9.5, 17)], close=True, fill="fg")


@icon("caret-down", GL, "Caret down (solid)")
def caret_down(g):
    g.poly([(7, 9.5), (17, 9.5), (12, 15.5)], close=True, fill="fg")


@icon("check", GL, "Check")
def check(g):
    g.poly([(5, 12.5), (9.5, 17), (19, 7)])


@icon("close-small", GL, "Close (small)")
def close_small(g):
    g.line(7.5, 7.5, 16.5, 16.5)
    g.line(16.5, 7.5, 7.5, 16.5)


@icon("plus-small", GL, "Plus (small)")
def plus_small(g):
    g.line(12, 7, 12, 17)
    g.line(7, 12, 17, 12)


@icon("minus-small", GL, "Minus (small)")
def minus_small(g):
    g.line(7, 12, 17, 12)


@icon("dot", GL, "Dot")
def dot(g):
    g.dot(12, 12, 3)


@icon("sort-ascending", GL, "Sort ascending")
def sort_ascending(g):
    g.line(8, 19.5, 8, 5)
    arrowhead(g, 8, 4.5, 0, -1, 4)
    g.line(13.5, 8, 15.5, 8)
    g.line(13.5, 12.5, 18, 12.5)
    g.line(13.5, 17, 20.5, 17)


@icon("sort-descending", GL, "Sort descending")
def sort_descending(g):
    g.line(8, 4.5, 8, 19)
    arrowhead(g, 8, 19.5, 0, 1, 4)
    g.line(13.5, 7, 20.5, 7)
    g.line(13.5, 11.5, 18, 11.5)
    g.line(13.5, 16, 15.5, 16)


@icon("grip-vertical", GL, "Grip (drag handle)")
def grip_vertical(g):
    for x in (9.5, 14.5):
        for y in (6, 12, 18):
            g.dot(x, y, 1.4)


@icon("grip-horizontal", GL, "Grip (horizontal)")
def grip_horizontal(g):
    for y in (9.5, 14.5):
        for x in (6, 12, 18):
            g.dot(x, y, 1.4)


@icon("ellipsis-vertical", GL, "More (vertical)")
def ellipsis_vertical(g):
    for y in (5, 12, 19):
        g.dot(12, y, 1.75)


@icon("arrow-up", GL, "Arrow up")
def arrow_up(g):
    g.line(12, 19.5, 12, 5)
    arrowhead(g, 12, 4.5, 0, -1, 6)


@icon("arrow-down", GL, "Arrow down")
def arrow_down(g):
    g.line(12, 4.5, 12, 19)
    arrowhead(g, 12, 19.5, 0, 1, 6)


@icon("external", GL, "Opens elsewhere")
def external(g):
    g.path("M18.5 13 V18.5 A2 2 0 0 1 16.5 20.5 H5.5 A2 2 0 0 1 3.5 18.5 V7.5 A2 2 0 0 1 5.5 5.5 H11")
    g.path("M14.5 3.5 H20.5 V9.5")
    g.line(20.5, 3.5, 11.5, 12.5)


@icon("resize-grip", GL, "Size grip")
def resize_grip(g):
    g.line(20, 9, 9, 20)
    g.line(20, 14.5, 14.5, 20)


@icon("drop-target", GL, "Docking target")
def drop_target(g):
    g.rect(8.5, 8.5, 7, 7, r=1)
    g.poly([(10, 5), (12, 3), (14, 5)])
    g.poly([(10, 19), (12, 21), (14, 19)])
    g.poly([(5, 10), (3, 12), (5, 14)])
    g.poly([(19, 10), (21, 12), (19, 14)])


@icon("modified", GL, "Unsaved changes")
def modified(g):
    g.dot(12, 12, 4)
