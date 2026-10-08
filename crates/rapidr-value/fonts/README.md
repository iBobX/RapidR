# Fonts built into RapidR

The Liberation fonts 2.1.5 (Sans, Serif and Mono, Regular), unmodified, from
<https://github.com/liberationfonts/liberation-fonts> (release 2.1.5,
`liberation-fonts-ttf-2.1.5.tar.gz`, SHA-256
`7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`).

They're licensed under the SIL Open Font License 1.1 (`OFL-1.1.txt`;
authors in `AUTHORS`). RapidR draws text on bitmaps with them
(`src/objects/text.rs`): they have the same character widths as Arial,
Times New Roman and Courier New, the fonts RapidQ programs name.

## JetBrains Mono Italic (the code editor's comments)

`JetBrainsMono-Italic.ttf` is JetBrains Mono 2.211's italic by The
JetBrains Mono Project Authors (<https://github.com/JetBrains/JetBrainsMono>),
the variable font (weight axis 100–800) in the Latin subset Google Fonts
distributes, as packaged by Fontsource
(`@fontsource-variable/jetbrains-mono` 5.3.0,
`jetbrains-mono-latin-wght-italic.woff2` SHA-256
`a8afa085e9ca5e53434e2ee918ba6b65c7dd4dda56509976b36591478c99d62e`),
converted from WOFF2 to TrueType with fontTools (the glyphs and tables
unchanged). Licensed under the SIL Open Font License 1.1, which declares no
Reserved Font Name for it (`JetBrainsMono-OFL.txt`). The code editor
(RCODEEDITOR, RDIFFVIEW) draws comments with it where its scheme says
italic; upright and bold code comes from the 2.304 faces below.

## RapidR Sans

**RapidR Sans** (`RapidRSans-Regular.ttf`) is a Modified Version of
Liberation Sans 2.1.5 under the same licence (SIL Open Font License 1.1;
renamed, as the OFL asks: Liberation is a Reserved Font Name). It is the
face RapidR draws MS Sans Serif with — RapidQ's default font, every
component's — so a form laid out for RapidQ fits in RapidR as it did there:
each Windows-1252 character is exactly as wide as MS Sans Serif's at 8 pt
on a 96-dpi screen (its em is 11 pixels; the letters keep Liberation's
shapes — the widths come from their side bearings, a letter narrowed at
most 4 % or made a little smaller where it must), and a line is 13
pixels high with the baseline 11 pixels down, as Windows draws it. The widths are RapidQ's own `TextWidth` of each
character, measured with RapidQ's compiler on Windows 11; no Microsoft font
data is used. `tools/fonts/make_rapidr_sans.py` makes it from
`LiberationSans-Regular.ttf` (reproducibly) and explains the details.

**Inter** (`Inter-Regular.ttf`, `Inter-SemiBold.ttf`; Inter 4.1,
<https://github.com/rsms/inter>, `Inter-4.1.zip`, SHA-256
`9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`) and
**JetBrains Mono** (`JetBrainsMono-Regular.ttf`, `JetBrainsMono-Bold.ttf`;
2.304, <https://github.com/JetBrains/JetBrainsMono>,
`JetBrainsMono-2.304.zip`, SHA-256
`6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`) are
RapidR's own UI and code faces (docs/ide-plan.md decision D8: RapidR
Studio, the RapidR look's chrome). Both are under the SIL Open Font License
1.1 (`Inter-OFL.txt`, `JetBrainsMono-OFL.txt`, with their copyright lines),
neither with a Reserved Font Name. `tools/fonts/subset_ui_fonts.py` subsets
them to the Latin scripts without hinting (about 72 KB each); other
characters come from the fallback fonts. RapidQ's font names never map to
them: a program gets them by naming "Inter" or "JetBrains Mono".
