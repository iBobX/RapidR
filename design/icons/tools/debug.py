"""The debugger's own icons (docs/preview4-plan.md S10): the kind of each
value Variables, Watch and the data tips show, and the sections they are
grouped in. Two-tone, like the symbols: a value is a tile in its family's
hue (the hue its text is drawn in — numbers teal, text amber, True / False
blue), a section a shape of its own.

- Values that already have an icon keep it (inventory.toml's
  [debug-kinds]): a constant is `symbols/constant`, a record (a TYPE) is
  `symbols/type`, a component is its own (`components/button` …).
- The breakpoints' states, the current line and the panes' commands are
  actions (actions.py); the changed-since-the-last-step mark is a glyph
  (glyphs.py), drawn in the theme's `debug.changed` colour."""

from kit import icon
from motifs import arrowhead

D = "debug"


def tile(g, hue):
    """A value's tile: the keyline square, rounded as a button."""
    g.rect(3, 3, 18, 18, r=3, c=hue, fill=f"{hue}-tint")


# ---- numbers: whole and decimal (teal) -------------------------------------------------
# A number is drawn as its digits' boxes, never as digits (README, "No
# text"): two tall rounded boxes, side by side for a whole number, a
# decimal point between them for a decimal one.

def digit(g, x, y, w, h, hue):
    g.rect(x, y, w, h, r=w / 2, c=hue)


def digit16(g, col, hue):
    """A digit's box at 16 px, by hand: three pixels wide, eight tall, its
    corners cut (a rounded box that narrow would lose its sides)."""
    c, top, bot = col, 4, 11
    g.native(16, f"M{c + 1} {top}H{c + 2}V{top + 1}H{c + 1}Z M{c} {top + 1}H{c + 1}V{bot}H{c}Z "
                 f"M{c + 2} {top + 1}H{c + 3}V{bot}H{c + 2}Z M{c + 1} {bot}H{c + 2}V{bot + 1}H{c + 1}Z", c=None, fill=hue)


@icon("integer", D, "Whole number (INTEGER, LONG, BYTE, WORD …)")
def integer(g):
    tile(g, "teal")
    if g.size == 16:
        digit16(g, 4, "teal")
        digit16(g, 9, "teal")
    else:
        digit(g, 6.5, 7, 4.5, 10, "teal")
        digit(g, 13, 7, 4.5, 10, "teal")


@icon("decimal", D, "Decimal number (SINGLE, DOUBLE)")
def decimal(g):
    tile(g, "teal")
    if g.size == 16:
        digit16(g, 3, "teal")
        g.native(16, "M7 10H9V12H7Z", c=None, fill="teal")
        digit16(g, 10, "teal")
    else:
        digit(g, 5.5, 7, 4.5, 10, "teal")
        g.dot(12, 16, 1.25, fill="teal")
        digit(g, 14, 7, 4.5, 10, "teal")


# ---- text (amber) --------------------------------------------------------------------------

def comma(g, x, y, r, hue):
    """A typographic quote's comma: a round head at (x, y), its tail
    rising to the upper right (an opening quote's “)."""
    g.dot(x, y, r, fill=hue)
    g.path(f"M{x - r + 0.75} {y} C{x - r + 0.75} {y - 3} {x} {y - 4.5} {x + r} {y - 5}", c=hue)


@icon("text", D, "Text (STRING)")
def text(g):
    tile(g, "amber")
    # a quotation mark (“): text, whatever it says
    if g.size == 16:
        for c in (4, 9):
            g.native(16, f"M{c + 2} 5H{c + 3}V6H{c + 2}Z M{c + 1} 6H{c + 2}V7H{c + 1}Z M{c} 7H{c + 2}V10H{c}Z",
                     c=None, fill="amber")
    else:
        comma(g, 9, 14.5, 2.25, "amber")
        comma(g, 15.5, 14.5, 2.25, "amber")


# ---- True / False (blue) ---------------------------------------------------------------------

@icon("boolean", D, "True or False")
def boolean(g):
    tile(g, "blue")
    # a switch, on
    g.rect(6, 8.5, 12, 7, r=3.5, c="blue", fill="paper")
    g.dot(14.5, 12, 2 if g.size != 16 else 2.2, fill="blue")


# ---- containers -----------------------------------------------------------------------------

@icon("array", D, "Array")
def array(g):
    tile(g, "cyan")
    # a row of cells, the first one picked (an element)
    if g.size == 16:
        g.frect(5.5, 9.5, 3.5, 5, fill="cyan")
        g.rect(10.5, 9.5, 2.5, 5, c="cyan")
        g.rect(14.5, 9.5, 2.5, 5, c="cyan")
    else:
        g.frect(5.5, 9, 4, 6, fill="cyan")
        g.rect(11, 9.75, 2.5, 4.5, r=0.5, c="cyan")
        g.rect(16, 9.75, 2.5, 4.5, r=0.5, c="cyan")


@icon("handle", D, "Handle (a number that stands for something the system keeps: a window, a file, a picture)")
def handle(g):
    # a cloakroom ticket: you hand the thing over and keep its number (cyan:
    # what lives outside the program)
    g.path("M3 5 H21 V9.5 A2.5 2.5 0 0 0 21 14.5 V19 H3 V14.5 A2.5 2.5 0 0 0 3 9.5 Z", c="cyan", fill="cyan-tint")
    g.line(8.5, 9, 8.5, 15, c="cyan")


@icon("empty", D, "Empty (no value yet)")
def empty(g):
    # the empty set: a ring struck through
    g.circle(12, 12, 7, c="ink", fill="shade")
    g.line(18.5, 5.5, 5.5, 18.5, c="ink")


@icon("result", D, "A FUNCTION's result")
def result(g):
    tile(g, "violet")
    # return: down, then back to the left
    g.path("M16.5 6.5 V11 A2.5 2.5 0 0 1 14 13.5 H7.5", c="violet")
    arrowhead(g, 7, 13.5, -1, 0, 3.5, c="violet")
    g.line(7, 17.5, 17, 17.5, c="violet") if g.size != 16 else None


# ---- sections ---------------------------------------------------------------------------------

@icon("locals", D, "Locals (the variables of the SUB or FUNCTION the program is in)")
def locals_(g):
    # a pin: the variables where the program stopped
    g.path("M12 21 C8 16.5 5.5 13.5 5.5 9.5 A6.5 6.5 0 0 1 18.5 9.5 C18.5 13.5 16 16.5 12 21 Z", c="blue", fill="blue-tint")
    g.circle(12, 9.5, 2.5, c="blue", fill="paper")


@icon("globals", D, "Globals (the program's own variables)")
def globals_(g):
    g.circle(12, 12, 9, c="blue", fill="blue-tint")
    g.ellipse(12, 12, 4, 9, c="blue")
    g.line(3, 12, 21, 12, c="blue")


@icon("watch", D, "Watch (an expression you follow)")
def watch(g):
    g.path("M2.5 12 C5.5 6.5 9 4.5 12 4.5 C15 4.5 18.5 6.5 21.5 12 C18.5 17.5 15 19.5 12 19.5 C9 19.5 5.5 17.5 2.5 12 Z", c="ink", fill="paper")
    g.circle(12, 12, 3.5, c="blue", fill="blue-tint")
    g.dot(12, 12, 1.25, fill="blue")


@icon("frame", D, "A call on the stack (a SUB or FUNCTION running)")
def frame(g):
    # the call under it, then the call itself in front
    g.rect(6, 3, 15, 11, r=2, c="ink", fill="paper")
    g.cut(lambda m: m.rect(3, 9, 15, 12, r=2, fill="fg"))
    g.rect(3, 9, 15, 12, r=2, c="violet", fill="violet-tint")
    g.line(6.5, 15, 14.5, 15, c="violet")
