"""The systems an app is built for (RapidR Studio's Build dialog,
design/build-dialog): our own drawings of a window as each system draws it,
never a system's logo (the logos are their makers' trademarks; the system's
name always stands beside the picture). Two-tone, as the components are: a
neutral frame (`ink` on `paper`) and one hue.

Told apart at 16 px, not only by colour:
- Windows: square corners, a solid title bar with the close cross at its
  right end;
- macOS: round corners, three dots at the left of the bar (the window
  buttons' place there), no bar fill;
- Linux: a terminal's window, its prompt;
- the web: the globe (the network's, as the HTTP components draw it);
- a phone and a board: what comes later, drawn greyed where they are shown.
"""

from kit import icon

A = "actions"


@icon("target-windows", A, "Target: Windows (a window, its close cross at the right of a solid title bar)", mono=False)
def target_windows(g):
    g.rect(3, 4, 18, 16, r=1, c="ink", fill="paper")
    if g.size == 16:
        g.frect(3, 4, 18, 5, fill="blue")
        g.line(16.5, 5.5, 18.5, 7.5, c="paper")
        g.line(18.5, 5.5, 16.5, 7.5, c="paper")
    else:
        g.frect(3, 4, 18, 5.5, fill="blue")
        g.line(16.25, 5.5, 18.75, 8, c="paper")
        g.line(18.75, 5.5, 16.25, 8, c="paper")
    g.rect(6.5, 12.5, 11, 4.5, r=0.5, c="blue", fill="blue-tint")


@icon("target-macos", A, "Target: macOS (a window with round corners, three dots at the left of its bar)", mono=False)
def target_macos(g):
    g.rect(3, 4, 18, 16, r=3.5, c="ink", fill="paper")
    g.line(3, 9, 21, 9, c="ink")
    if g.size == 16:
        # (three 2-pixel dots a pixel apart, drawn in the 16-pixel grid:
        # still three dots at 16 px)
        for x, hue in ((3, "red"), (6, "amber"), (9, "teal")):
            g.native(16, f"M{x} 3h2v2h-2z", c=None, fill=hue)
    else:
        for x, hue in ((6.75, "red"), (10, "amber"), (13.25, "teal")):
            g.circle(x, 6.5, 1.1, c=None, fill=hue)


@icon("target-linux", A, "Target: Linux (a terminal's window and its prompt)", mono=False)
def target_linux(g):
    g.rect(3, 4, 18, 16, r=2, c="ink", fill="paper")
    g.line(3, 8.5, 21, 8.5, c="ink")
    g.poly([(6.75, 11.5), (9.25, 13.75), (6.75, 16)], c="teal")
    g.line(11.25, 16, 16.25, 16, c="teal")


@icon("target-web", A, "Target: the web (the globe)", mono=False)
def target_web(g):
    g.circle(12, 12, 9, c="cyan", fill="cyan-tint")
    g.ellipse(12, 12, 4, 9, c="cyan")
    g.line(3, 12, 21, 12, c="cyan")


@icon("target-phone", A, "Target: a phone (Android, iOS; later)", mono=False)
def target_phone(g):
    g.rect(6.5, 2.5, 11, 19, r=2, c="ink", fill="paper")
    g.line(10.5, 18.5, 13.5, 18.5, c="ink")
    g.rect(9, 5.5, 6, 9.5, r=0.5, c="violet", fill="violet-tint")


@icon("target-board", A, "Target: a board (ESP32, micro:bit; later)", mono=False)
def target_board(g):
    g.rect(6, 6, 12, 12, r=1.5, c="teal", fill="teal-tint")
    g.rect(9.5, 9.5, 5, 5, r=0.5, c="teal")
    for v in (9, 12, 15) if g.size != 16 else (9.5, 14.5):
        g.line(v, 2.75, v, 6, c="ink")
        g.line(v, 18, v, 21.25, c="ink")
        g.line(2.75, v, 6, v, c="ink")
        g.line(18, v, 21.25, v, c="ink")
