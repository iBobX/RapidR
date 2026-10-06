"""The icon colour tokens per theme: the one source the Rust crate's table
(crates/rapidr-icons/src/generated.rs, written by build.py), the previews
and the docs export read.

Each theme gives every token (kit.TOKENS' keys) a value, plus `fg` (what
`currentColor` is by default: monochrome icons' ink) and the backgrounds
icons are checked against (design/icons/README.md, "Contrast"). The light
themes' values are the SVG sources' own (kit.TOKENS).
"""

from kit import TOKENS

LIGHT = dict(TOKENS)

DARK = {
    "ink": "#C9D1E3",        # the brand's Mist
    "paper": "#373A42",
    "shade": "#4A4F5A",
    "blue": "#7398FF", "blue-tint": "#24315C",
    "cyan": "#2CC9E9", "cyan-tint": "#123C47",
    "teal": "#34CF98", "teal-tint": "#123D30",
    "amber": "#FFB224", "amber-tint": "#45340F",
    "amber-solid": "#FFB224",
    "red": "#FF6B70", "red-tint": "#4D1E22",
    "violet": "#AC8FFF", "violet-tint": "#33295E",
    "spark": "#19C6E6",
}

# Windows' High Contrast Black: monochrome — every outline and solid shape
# white, every tint the black ground (design/icons/README.md).
HIGH_CONTRAST = {t: ("#000000" if t.endswith("-tint") or t in ("paper", "shade") else "#FFFFFF") for t in TOKENS}

THEMES = {
    # name: (tokens, fg, backgrounds the icons sit on)
    "classic": (LIGHT, "#5B6478", ["#F0F0F0", "#FFFFFF"]),
    "modern": (LIGHT, "#5B6478", ["#F3F3F3", "#FFFFFF", "#F9F9F9"]),
    "dark": (DARK, "#C9D1E3", ["#202020", "#2B2B2B", "#2C2C2C"]),
    "highcontrast": (HIGH_CONTRAST, "#FFFFFF", ["#000000"]),
}

# A disabled icon's ink (its outlines and solid shapes; tints become paper),
# after each theme's disabled text (rapidr_value::theme, gray_text).
DISABLED = {"classic": "#8D93A0", "modern": "#8D93A0", "dark": "#7C8290", "highcontrast": "#3FF23F"}

# What each tint's outline colour is (contrast checks: an outline on its tint).
PAIRS = [(h, h + "-tint") for h in ("blue", "cyan", "teal", "amber", "red", "violet")]


def themed(svg, theme):
    """An SVG source in `theme`'s colours (`currentColor` stays, for the
    caller's `color`)."""
    tokens, fg, _ = THEMES[theme]
    out = svg
    # (each light value is unique: kit.TOKENS; swap through placeholders so
    # a value equal to another token's light value isn't swapped twice)
    for i, (t, light) in enumerate(TOKENS.items()):
        out = out.replace(f'"{light}"', f'"@@{i}@@"')
    for i, t in enumerate(TOKENS):
        out = out.replace(f'"@@{i}@@"', f'"{tokens[t]}"')
    return out


def luminance(hex_):
    v = int(hex_[1:], 16)
    def ch(c):
        c = c / 255
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * ch(v >> 16 & 255) + 0.7152 * ch(v >> 8 & 255) + 0.0722 * ch(v & 255)


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


if __name__ == "__main__":
    for name, (tok, fg, bgs) in THEMES.items():
        print(f"== {name}")
        for h in ("ink", "blue", "cyan", "teal", "amber", "red", "violet"):
            vals = [contrast(tok[h], bg) for bg in bgs + [tok["paper"]]]
            tint = contrast(tok[h], tok.get(h + "-tint", tok["paper"]))
            flag = "" if min(vals + [tint]) >= 3 else "  <-- below 3:1"
            print(f"  {h:7} {tok[h]}  on bgs/paper {' '.join(f'{v:4.1f}' for v in vals)}  on tint {tint:4.1f}{flag}")
