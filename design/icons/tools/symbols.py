"""Symbol kinds: the language service's outline and completion items
(docs/ide-plan.md I3). One shape per kind, its hue telling the family:
violet for code you call (SUB, FUNCTION, method, parameter), blue for
values (variable, constant, field, property), amber for types, events and
labels, teal for directives and resources, ink for keywords and snippets."""

import math

from kit import icon
from motifs import cube_iso

S = "symbols"


@icon("sub", S, "SUB")
def sub(g):
    cube_iso(g, c="violet", fill="violet-tint")


@icon("method", S, "Method")
def method(g):
    cube_iso(g, c="violet", fill="violet")


@icon("function", S, "FUNCTION")
def function(g):
    g.rect(3, 3, 18, 18, r=3, c="violet", fill="violet-tint")
    g.path("M15.5 6.5 C13.25 6 12.4 7 12 9 L10.75 15.25 C10.4 17.25 9.5 18 7.5 17.5", c="violet")
    g.line(9, 10.75, 14.5, 10.75, c="violet")


@icon("parameter", S, "Parameter")
def parameter(g):
    g.path("M8 3.5 C4.5 7 4.5 17 8 20.5", c="violet")
    g.path("M16 3.5 C19.5 7 19.5 17 16 20.5", c="violet")
    g.dot(12, 12, 2, fill="violet")


@icon("variable", S, "Variable")
def variable(g):
    g.path("M7 6.5 H4 V17.5 H7", c="blue")
    g.path("M17 6.5 H20 V17.5 H17", c="blue")
    g.rect(8.5, 9.5, 7, 5, r=1, c="blue", fill="blue-tint")


@icon("constant", S, "Constant")
def constant(g):
    g.rect(3, 5, 18, 14, r=2, c="blue", fill="blue-tint")
    g.line(8, 10, 16, 10, c="blue")
    g.line(8, 14, 16, 14, c="blue")


@icon("field", S, "Field")
def field(g):
    g.path("M3.5 12 L10 5.5 H20.5 V18.5 H10 Z", c="blue", fill="blue-tint")
    g.dot(9, 12, 1.5, fill="blue")


@icon("property", S, "Property")
def property_(g):
    # a wrench
    g.path("M14.5 3.5 A5 5 0 0 0 10.6 11.5 L4 18 A1.75 1.75 0 0 0 6.5 20.5 L13 14 A5 5 0 0 0 20.5 9.5 L17 11 L14 10 L13 7 Z", c="blue", fill="blue-tint")


@icon("type", S, "TYPE")
def type_(g):
    g.rect(3, 3, 18, 18, r=2, c="amber", fill="amber-tint")
    g.line(3, 9, 21, 9, c="amber")
    g.line(7, 13, 17, 13, c="amber") if g.size != 16 else None
    g.line(7, 16.5, 13, 16.5, c="amber") if g.size != 16 else g.line(6, 15, 15, 15, c="amber")


@icon("event", S, "Event")
def event(g):
    g.poly([(14, 2.5), (5.5, 13.5), (11.5, 13.5), (10, 21.5), (18.5, 10.5), (12.5, 10.5)], close=True, c="amber", fill="amber-tint")


@icon("label", S, "Label (GOTO target)")
def label(g):
    g.path("M3.5 6 H15 L20.5 12 L15 18 H3.5 Z", c="amber", fill="amber-tint")
    g.dot(14.5, 12, 1.5, fill="amber")


@icon("keyword", S, "Keyword")
def keyword(g):
    g.rect(3, 5, 18, 14, r=2, c="ink", fill="shade")
    g.line(7, 12, 17, 12, c="ink")


@icon("snippet", S, "Snippet")
def snippet(g):
    for x0, y0, x1, y1 in ((3, 6, 3, 9), (3, 15, 3, 18), (21, 6, 21, 9), (21, 15, 21, 18)):
        g.line(x0, y0, x1, y1, c="ink")
    g.path("M3 6 V5 A2 2 0 0 1 5 3 H8 M16 3 H19 A2 2 0 0 1 21 5 V6 M21 18 V19 A2 2 0 0 1 19 21 H16 M8 21 H5 A2 2 0 0 1 3 19 V18", c="ink")
    g.line(11, 3, 13, 3, c="ink")
    g.line(11, 21, 13, 21, c="ink")
    g.line(7.5, 12, 16.5, 12, c="ink")


@icon("component", S, "Component")
def component(g):
    g.rect(3, 3, 18, 18, r=2, c="blue", fill="blue-tint")
    g.rect(7.5, 7.5, 9, 9, r=1, c="blue", fill="paper")


@icon("directive", S, "Directive ($INCLUDE …)")
def directive(g):
    g.line(9.5, 4, 7.5, 20, c="teal")
    g.line(16.5, 4, 14.5, 20, c="teal")
    g.line(4.5, 9, 19.5, 9, c="teal")
    g.line(4, 15, 19, 15, c="teal")


@icon("module", S, "Module (file)")
def module(g):
    g.path("M3.5 7.5 L12 3 L20.5 7.5 V16.5 L12 21 L3.5 16.5 Z", c="ink", fill="paper")
    g.path("M3.5 7.5 L12 12 L20.5 7.5 M12 12 V21", c="ink")


@icon("resource", S, "Resource ($RESOURCE)")
def resource(g):
    g.rect(3, 3, 18, 18, r=2, c="teal", fill="teal-tint")
    g.path("M3 16 L8.5 11 L12.5 15 L15 12.5 L21 18", c="teal")
    g.dot(15.5, 7.5, 1.5, fill="teal")
