"""RAI's actions (docs/rai-vision-and-screenshot.md and the RAI plans), and
the audio actions a program's own apps use. Monochrome (`currentColor`): the
brand's gradient stays on `actions/ai` alone. The four-pointed sparkle marks
what the AI does; the rest is the same ring, line and frame vocabulary as the
rest of the set."""

import math

from kit import icon
from motifs import arrowhead, star4, rot

A = "actions"


@icon("ai-plan", A, "Plan (RAI's steps)")
def ai_plan(g):
    # a checklist: the first step done
    g.rect(3.5, 4.5, 4.5, 4.5, r=1)
    g.poly([(4.75, 6.75), (5.75, 7.75), (7.25, 5.5)])
    g.rect(3.5, 14.5, 4.5, 4.5, r=1)
    g.line(11, 6.75, 20.5, 6.75)
    g.line(11, 16.75, 20.5, 16.75)


@icon("ai-ask", A, "Ask a question")
def ai_ask(g):
    g.path("M5.5 3.5 H18.5 A2 2 0 0 1 20.5 5.5 V13.5 A2 2 0 0 1 18.5 15.5 H11 L6.5 20 V15.5 H5.5 A2 2 0 0 1 3.5 13.5 V5.5 A2 2 0 0 1 5.5 3.5 Z")
    g.path("M9.75 8.25 A2.25 2.25 0 1 1 12.5 10.4 C12.1 10.7 12 11 12 11.5")
    g.dot(12, 13.4, 0.9)


@icon("ai-vision", A, "Vision (RAI sees the window)")
def ai_vision(g):
    # the corners of a frame, an eye inside it
    g.path("M3.5 8.5 V5.5 A2 2 0 0 1 5.5 3.5 H8.5")
    g.path("M15.5 3.5 H18.5 A2 2 0 0 1 20.5 5.5 V8.5")
    g.path("M20.5 15.5 V18.5 A2 2 0 0 1 18.5 20.5 H15.5")
    g.path("M8.5 20.5 H5.5 A2 2 0 0 1 3.5 18.5 V15.5")
    g.path("M7.5 12 C9 9.5 15 9.5 16.5 12 C15 14.5 9 14.5 7.5 12 Z")
    g.dot(12, 12, 1.1)


@icon("ai-record", A, "Record the window")
def ai_record(g):
    # a camera: the dot says it is recording
    g.rect(3, 6.5, 12.5, 11, r=2)
    g.path("M15.5 10.5 L21 7.5 V16.5 L15.5 13.5")
    g.dot(9.25, 12, 2.1)


@icon("ai-search", A, "Search the web")
def ai_search(g):
    # a lens with a globe in it
    g.circle(10.5, 10.5, 7)
    g.ellipse(10.5, 10.5, 2.6, 7)
    g.line(3.5, 10.5, 17.5, 10.5)
    g.line(16, 16, 20.5, 20.5)


@icon("ai-agent", A, "Agent working")
def ai_agent(g):
    # the way round the clock, a sparkle at its heart
    g.path("M12 3.5 A8.5 8.5 0 1 1 3.5 12")
    arrowhead(g, 3.5, 11.5, 0, -1, 4)
    star4(g, 12, 12, 4, fill="fg")


@icon("ai-tool", A, "Tool (a wrench)")
def ai_tool(g):
    # an open-end spanner at 45 degrees: a handle, a head with its jaw
    c, s_ = math.cos(math.radians(-45)), math.sin(math.radians(-45))
    ox, oy = 3.1, 20.9

    def P(x, y):
        return ox + x * c - y * s_, oy + x * s_ + y * c

    def f(x, y):
        a_, b_ = P(x, y)
        return f"{a_:.2f} {b_:.2f}"

    r = 4.6
    d = (f"M{f(2, -1.4)} L{f(12.02, -1.4)} A{r} {r} 0 0 1 {f(20.67, -1.7)} L{f(17.2, -1.7)} L{f(17.2, 1.7)} L{f(20.67, 1.7)} "
         f"A{r} {r} 0 0 1 {f(12.02, 1.4)} L{f(2, 1.4)} A1.4 1.4 0 0 1 {f(2, -1.4)} Z")
    g.path(d)


@icon("ai-local-model", A, "Local model (runs on this computer)")
def ai_local_model(g):
    # a chip with its pins, a sparkle on it
    g.rect(6.5, 6.5, 11, 11, r=2)
    for p in (9.5, 14.5):
        g.line(p, 3, p, 6.5)
        g.line(p, 17.5, p, 21)
        g.line(3, p, 6.5, p)
        g.line(17.5, p, 21, p)
    star4(g, 12, 12, 3, fill="fg")


@icon("ai-on-device", A, "On this device")
def ai_on_device(g):
    # a laptop with a sparkle: nothing leaves the computer
    g.rect(5, 4.5, 14, 10.5, r=2)
    g.path("M2.5 18.5 H21.5")
    star4(g, 12, 9.75, 3, fill="fg")


@icon("microphone", A, "Microphone")
def microphone(g):
    g.rect(9, 3, 6, 11, r=3)
    g.path("M5.5 11 V12 A6.5 6.5 0 0 0 18.5 12 V11")
    g.line(12, 18.5, 12, 21)
    g.line(8.5, 21, 15.5, 21)


def speaker(g):
    g.path("M3.5 9.5 H7.5 L12.5 5.5 V18.5 L7.5 14.5 H3.5 Z")


@icon("speaker-on", A, "Speaker on")
def speaker_on(g):
    speaker(g)
    g.path("M15.5 9 A4.5 4.5 0 0 1 15.5 15")
    g.path("M18.5 6.5 A8 8 0 0 1 18.5 17.5")


@icon("speaker-off", A, "Speaker off")
def speaker_off(g):
    speaker(g)
    g.line(16, 9.5, 21, 14.5)
    g.line(21, 9.5, 16, 14.5)
