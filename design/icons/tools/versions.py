"""Versions: RapidR Studio's built-in version control (docs/versions-plan.md).
Monochrome (`currentColor`). A version is a ring (a node) on a line; where
lines join or split, versions merge or branch. Nothing here borrows a
product's mark: rings, lines and arrows are the whole vocabulary."""

from kit import icon
from motifs import arrowhead

A = "actions"
R = 2.25      # a node's radius (2 at the 16 px size, whole pixels)


def node(g, x, y, r=R):
    g.circle(x, y, r)


@icon("save-version", A, "Save version")
def save_version(g):
    # a version: a ring on the line of versions
    g.circle(12, 12, 3.5)
    g.line(3, 12, 8.5, 12)
    g.line(15.5, 12, 21, 12)


@icon("timeline", A, "Version history")
def timeline(g):
    # versions down a line, each with its note
    for y, x2 in ((5.5, 20.5), (12, 20.5), (18.5, 17)):
        node(g, 6.5, y, 2)
        g.line(11, y, x2, y)
    g.line(6.5, 7.5, 6.5, 10)
    g.line(6.5, 14, 6.5, 16.5)


@icon("branch", A, "Branch")
def branch(g):
    # one line splits in two
    node(g, 12, 5.5)
    node(g, 6, 18.5)
    node(g, 18, 18.5)
    g.path("M12 7.75 V10 H9 A3 3 0 0 0 6 13 V16.25")
    g.path("M12 10 H15 A3 3 0 0 1 18 13 V16.25")


@icon("merge", A, "Merge")
def merge(g):
    # two lines become one
    node(g, 6, 5.5)
    node(g, 18, 5.5)
    node(g, 12, 18.5)
    g.path("M6 7.75 V10 A3 3 0 0 0 9 13 H12 V16.25")
    g.path("M18 7.75 V10 A3 3 0 0 1 15 13 H12")


@icon("compare", A, "Compare versions")
def compare(g):
    # two sides: what was taken out, what was put in
    g.rect(3, 4.5, 7.5, 15, r=1.5)
    g.rect(13.5, 4.5, 7.5, 15, r=1.5)
    g.line(5.25, 12, 8.25, 12)
    g.line(15.75, 12, 18.75, 12)
    g.line(17.25, 10.5, 17.25, 13.5)


@icon("line-history", A, "Line history (who changed it)")
def line_history(g):
    # lines of text, a clock on them: when each was written, and by whom
    g.line(3.5, 5.5, 20.5, 5.5)
    g.line(3.5, 10.5, 20.5, 10.5)
    g.line(3.5, 16, 9.5, 16)
    g.cut(lambda m: m.circle(16.5, 16.5, 4.5, c="fg", fill="fg"), gap=0.75)
    g.circle(16.5, 16.5, 4.5)
    g.path("M16.5 14.25 V16.5 L18.25 17.5")


@icon("push", A, "Push (send versions)")
def push(g):
    # from the version up to the remote
    node(g, 12, 18.5)
    g.line(12, 16.25, 12, 6)
    arrowhead(g, 12, 5.5, 0, -1, 4.5)
    g.line(5.5, 3, 18.5, 3)


@icon("pull", A, "Pull (fetch versions)")
def pull(g):
    # from the remote down to the version
    g.line(5.5, 21, 18.5, 21)
    g.line(12, 7.75, 12, 18)
    arrowhead(g, 12, 18.5, 0, 1, 4.5)
    node(g, 12, 5.5)


@icon("sync", A, "Sync (push and pull)")
def sync(g):
    g.line(8, 19.5, 8, 5.5)
    arrowhead(g, 8, 4.5, 0, -1, 4.5)
    g.line(16, 4.5, 16, 18.5)
    arrowhead(g, 16, 19.5, 0, 1, 4.5)


@icon("conflict", A, "Merge conflict")
def conflict(g):
    # two lines that meet and can't become one
    node(g, 6, 5.5)
    node(g, 18, 5.5)
    g.path("M6 7.75 V10 A3 3 0 0 0 9 13")
    g.path("M18 7.75 V10 A3 3 0 0 1 15 13")
    g.line(12, 14.5, 12, 17.5)
    g.dot(12, 20.5, 1.1)


@icon("restore-version", A, "Restore version")
def restore_version(g):
    # back to a version: the way round the clock, the version at its heart
    g.path("M4.5 12 A7.5 7.5 0 1 0 6.7 6.7")
    g.poly([(6.5, 3), (6.5, 7), (2.5, 7)])
    node(g, 12, 12)


@icon("pull-request", A, "Pull request")
def pull_request(g):
    # a line of versions asks to be taken into another
    node(g, 6, 5.5)
    node(g, 6, 18.5)
    node(g, 18, 18.5)
    g.line(6, 7.75, 6, 16.25)
    g.path("M18 16.25 V9.5 A3 3 0 0 0 15 6.5 H11.5")
    arrowhead(g, 10.5, 6.5, -1, 0, 3.5)
