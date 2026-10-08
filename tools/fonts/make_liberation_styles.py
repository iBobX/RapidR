#!/usr/bin/env python3
"""The real bold, italic and bold italic faces of RapidR's Liberation text
fonts (Sans, Serif, Mono), cut from the Liberation fonts 2.1.5 release.

RapidR draws Arial, Times New Roman and Courier New (RapidQ programs name
them) with Liberation Sans, Serif and Mono, which have those fonts' character
widths. Liberation's Regular faces are built in whole and unmodified; their
Bold, Italic and Bold Italic come from here, so bold text is a designed bold
face (as wide as Arial Bold's, naturally) instead of the regular letters
drawn heavier and spaced out, and italic is a designed italic instead of
slanted regular letters.

The nine faces are subset to the Latin scripts (the same set as RapidR Sans
Bold and the Inter / JetBrains Mono subsets: Basic Latin, Latin-1, Latin
Extended-A, general punctuation, currency, letterlike symbols, arrows, minus,
geometric shapes, box drawing), without hinting (RapidR draws outlines
unhinted): about 36 KB instead of 400 KB each, which matters for the web
runtime. A character outside that set (Greek, Cyrillic, Hebrew …) is drawn
from the Regular face, made bold or slanted by the renderer, as before.

A subset is a Modified Version under the SIL Open Font License 1.1, and the
license makes Liberation (and Arimo, Tinos, Cousine) Reserved Font Names, so
the faces are renamed "RapidR Text Sans", "RapidR Text Serif" and "RapidR
Text Mono" (their copyright lines kept, the license named in each file). The
UI kernel registers them as members of the Liberation families
(`rapidr_ui_kernel::text`), so a bold request for "Liberation Sans" finds
them.

    python3 tools/fonts/make_liberation_styles.py <liberation-fonts-ttf-2.1.5 folder>

where the folder is the official release archive
(https://github.com/liberationfonts/liberation-fonts/releases/tag/2.1.5,
`liberation-fonts-ttf-2.1.5.tar.gz`, SHA-256
7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0) unpacked.
Writes crates/rapidr-value/fonts/RapidRText{Sans,Serif,Mono}-{Bold,Italic,
BoldItalic}.ttf, reproducibly. Needs fontTools (MIT).
"""
import os
import sys

from fontTools import subset
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "crates", "rapidr-value", "fonts")

# (kept in step with tools/fonts/make_rapidr_sans.py)
LATIN = "U+0020-007E,U+00A0-017F,U+0192,U+0218-021B,U+02C6-02DD,U+2000-206F,U+20A0-20CF,U+2100-215F,U+2190-21FF,U+2212,U+2215,U+221E,U+2248,U+2260-2265,U+25A0-25FF,U+2500-257F,U+FFFD"

FAMILIES = {"Sans": "Liberation Sans", "Serif": "Liberation Serif", "Mono": "Liberation Mono"}
STYLES = {"Bold": "Bold", "Italic": "Italic", "BoldItalic": "Bold Italic"}


def make(src_dir, kind, style):
    source = os.path.join(src_dir, f"Liberation{kind}-{style}.ttf")
    target = os.path.join(OUT, f"RapidRText{kind}-{style}.ttf")
    family = f"RapidR Text {kind}"
    sub = STYLES[style]
    # (the subsetter reads and writes the file: the renaming goes on the
    # subset after)
    subset.main([
        source,
        f"--unicodes={LATIN}",
        "--layout-features=kern,locl,mark,mkmk,ccmp,case",
        "--no-hinting",
        "--name-IDs=*",
        "--name-languages=*",
        "--notdef-outline",
        f"--output-file={target}",
    ])
    font = TTFont(target)
    font.recalcTimestamp = False
    names = {
        0: "Digitized data copyright (c) 2010 Google Corporation. Copyright (c) 2012 Red Hat, Inc. "
        "Modified for RapidR (2026): subset to the Latin scripts, without hinting, renamed.",
        1: family,
        2: sub,
        3: f"{family} {sub} (from {FAMILIES[kind]} {sub} 2.1.5)",
        4: f"{family} {sub}",
        5: f"Version 2.1.5 ({FAMILIES[kind]} 2.1.5, subset)",
        6: f"RapidRText{kind}-{style}",
        10: f"{FAMILIES[kind]} {sub} (SIL OFL 1.1), cut to the Latin scripts for RapidR.",
        13: "Licensed under the SIL Open Font License, Version 1.1",
        14: "https://openfontlicense.org",
    }
    table = font["name"]
    table.names = [r for r in table.names if r.nameID not in names and r.nameID not in (7, 16, 17, 18, 19)]
    for nid, text in names.items():
        table.setName(text, nid, 3, 1, 0x409)
        table.setName(text, nid, 1, 0, 0)
    font.save(target)
    print(f"{os.path.basename(target)}: {os.path.getsize(target):,} bytes")


def main():
    src_dir = sys.argv[1]
    for kind in FAMILIES:
        for style in STYLES:
            make(src_dir, kind, style)


if __name__ == "__main__":
    main()
