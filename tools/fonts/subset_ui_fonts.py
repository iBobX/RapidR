#!/usr/bin/env python3
"""RapidR Studio's fonts (docs/ide-plan.md decision D8): Inter for the
chrome, JetBrains Mono for code — both SIL Open Font License 1.1, neither
with a Reserved Font Name — subset to the Latin scripts so the web runtime
stays small (other characters come from the fallback fonts on demand,
fonts/fallback), without hinting (the UI kernel draws outlines unhinted).

    python3 tools/fonts/subset_ui_fonts.py <Inter-4.1.zip's folder> <JetBrainsMono-2.304.zip's folder>

Writes crates/rapidr-value/fonts/{Inter-Regular,Inter-SemiBold,
JetBrainsMono-Regular,JetBrainsMono-Bold}.ttf. Sources (fonts/README.md):
https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
(SHA-256 9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e),
https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip
(SHA-256 6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf).
Needs fontTools (MIT).
"""

import os
import sys

from fontTools import subset

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, "crates", "rapidr-value", "fonts")

# Basic Latin, Latin-1, Latin Extended-A, general punctuation, currency,
# letterlike (™), arrows, minus, the geometric shapes and box drawing
# (code and consoles).
LATIN = "U+0020-007E,U+00A0-017F,U+0192,U+0218-021B,U+02C6-02DD,U+2000-206F,U+20A0-20CF,U+2100-215F,U+2190-21FF,U+2212,U+2215,U+221E,U+2248,U+2260-2265,U+25A0-25FF,U+2500-257F,U+FFFD"


def run(src, name):
    args = [
        src,
        f"--unicodes={LATIN}",
        "--layout-features=kern,liga,calt,locl,mark,mkmk,ccmp,case,tnum",
        "--no-hinting",
        "--desubroutinize",
        "--name-IDs=*",
        "--name-languages=*",
        "--notdef-outline",
        f"--output-file={os.path.join(OUT, name)}",
    ]
    subset.main(args)
    print(f"{name}: {os.path.getsize(os.path.join(OUT, name)):,} bytes")


def main():
    inter, jbm = sys.argv[1], sys.argv[2]
    run(os.path.join(inter, "extras", "ttf", "Inter-Regular.ttf"), "Inter-Regular.ttf")
    run(os.path.join(inter, "extras", "ttf", "Inter-SemiBold.ttf"), "Inter-SemiBold.ttf")
    run(os.path.join(jbm, "fonts", "ttf", "JetBrainsMono-Regular.ttf"), "JetBrainsMono-Regular.ttf")
    run(os.path.join(jbm, "fonts", "ttf", "JetBrainsMono-Bold.ttf"), "JetBrainsMono-Bold.ttf")


if __name__ == "__main__":
    main()
