"""Regenerate everything in design/brand.

    python3 design/brand/src/make.py           # SVG masters + all exports
    python3 design/brand/src/make.py export    # exports only, from the committed SVGs

Writing the masters needs Inter (OFL-1.1) at src/fonts/Inter-V.ttf (or
RAPIDR_BRAND_FONT) to outline the labels; exporting needs only
rsvg-convert, iconutil (macOS) and Pillow.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import export  # noqa: E402

if __name__ == "__main__":
    if sys.argv[1:] != ["export"]:
        import concepts
        import github
        import icons
        import logo
        concepts.main()
        logo.main()
        icons.main()
        github.main()
    export.main()
