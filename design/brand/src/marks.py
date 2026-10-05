"""The RapidR mark geometry, on a 100 x 100 design grid.

All shapes are filled outlines with nonzero winding (holes wound the other
way), so they import cleanly into any editor and need no transforms.
"""
from brandkit import skew_x, rrect, transform, _num

# ------------------------------------------------ Concept A: "Velocity" --
# A forward-leaning R whose motion trails off as three speed bars.
def concept_a():
    parts = [
        "M38 16 H52 V84 H38 Z",                                         # stem
        "M52 16 H61 A21 21 0 0 1 61 58 H52 Z M52 30 V44 H60 A7 7 0 0 0 60 30 Z",  # bowl
        "M54 58 H68 L83 84 H68 Z",                                      # leg
        rrect(6, 30, 26, 8, 4), rrect(14, 46, 18, 8, 4), rrect(22, 62, 10, 8, 4),
    ]
    d = " ".join(skew_x(p, 12, 50) for p in parts)
    return transform(d, (1, 0, 0, 1, 2, 0))


# -------------------------------------------------- Concept B: "Prompt" --
# The R drawn as a command prompt: the bowl is a ">" chevron, the leg steps
# down to a block cursor. BASIC's READY-and-cursor heritage. Stroke-based.
B_STROKE = 13
B_LINES = ["M27 20 V80", "M27 20 L60 39 L27 58", "M41 50 L56 76"]
B_CURSOR = "M64 74.5 H82 A2.5 2.5 0 0 1 84.5 77 V84 A2.5 2.5 0 0 1 82 86.5 H64 A2.5 2.5 0 0 1 61.5 84 V77 A2.5 2.5 0 0 1 64 74.5 Z"


def concept_b_svg(fill):
    lines = "".join(f'<path d="{d}"/>' for d in B_LINES)
    return (f'<g fill="none" stroke="{fill}" stroke-width="{B_STROKE}" stroke-linecap="round" '
            f'stroke-linejoin="round">{lines}</g><path fill="{fill}" d="{B_CURSOR}"/>')


# ----------------------------------------------------- Concept C: "Run" --
# A constructed geometric R whose counter is a play triangle: RapidR runs
# what you write. Lives on a squircle tile; the glyph also stands alone.
def run_r():
    return " ".join([
        # stem + bowl in one contour, counter (play triangle) wound backwards
        "M27 19 H55 A20 20 0 0 1 55 59 H43 V81 H27 Z",
        "M43 31 V47 L62 39 Z",
        # leg
        "M43 59 H59 L76 81 H60 Z",
    ])


def run_r_centered():
    """run_r shifted so its optical centre sits on the tile centre."""
    return transform(run_r(), (1, 0, 0, 1, -1.5, 0))
