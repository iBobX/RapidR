# Fonts built into RapidR

The Liberation fonts 2.1.5 (Sans, Serif and Mono, Regular), whole and unmodified, from
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

## Liberation Bold, Italic and Bold Italic; RapidR Sans

**Bold, Italic and Bold Italic** (`RapidRText{Sans,Serif,Mono}-{Bold,Italic,
BoldItalic}.ttf`, nine files) are Liberation 2.1.5's own designed faces
(`LiberationSans-Bold.ttf` … from the same release archive), cut to the Latin
scripts (Basic Latin, Latin-1, Latin Extended-A, punctuation, currency,
letterlike symbols, arrows, geometric shapes, box drawing) without hinting,
36 to 39 KB each instead of about 400 KB. A subset is a Modified Version
under the OFL, and Liberation (also Arimo, Tinos, Cousine) is a Reserved Font
Name, so the files are renamed **RapidR Text Sans / Serif / Mono**, their
copyright lines kept and the OFL named in each. Their glyphs are Liberation's,
untouched; so a bold "Arial" text has Arial Bold's widths (Liberation Sans
Bold's advances are Arial Bold's) and a bold "Times New Roman" has Times New
Roman Bold's, with no help from the renderer. `tools/fonts/make_liberation_styles.py
<unpacked archive folder>` makes them reproducibly. At run time they are
registered as members of the Liberation families (`rapidr_ui_kernel::text`),
so a bold or italic request for "Liberation Sans" finds them; a character
they lack (Greek, Cyrillic, Hebrew …) comes from the family's Regular face,
made bold by drawing it twice a pixel apart, or slanted — the fallback.

**RapidR Sans** (`RapidRSans-Regular.ttf`, `RapidRSans-Bold.ttf`) is a Modified Version of
Liberation Sans 2.1.5 under the same licence (SIL Open Font License 1.1;
renamed, as the OFL asks: Liberation is a Reserved Font Name). It is the
face RapidR draws MS Sans Serif with — RapidQ's default font, every
component's — so a form laid out for RapidQ fits in RapidR as it did there,
and its text reads clearly at every scale: each Windows-1252 character is as
wide as MS Sans Serif's at 8 pt on a 96-dpi screen (its em is 11 pixels),
and a line is 13 pixels high with the baseline 11 pixels down, as Windows
draws it. Its letters are Liberation's own shapes, all made 95 % as large
(never narrowed or squeezed), with at least 0.8 of a pixel between two
letters at 8 pt: where MS Sans Serif's width can't hold a letter and that
space (r, x, y, j, C, the brackets), the character is a pixel wider than in
RapidQ ("program" is 40 pixels, RapidQ's 38; "Password:" 50, RapidQ's 49).
MS Sans Serif's widths are RapidQ's own `TextWidth` of each character, measured with RapidQ's compiler on Windows 11; no Microsoft font
data is used. `tools/fonts/make_rapidr_sans.py` makes it from
`LiberationSans-Regular.ttf` (reproducibly) and explains the details. Its
**Bold** is the same from Liberation Sans Bold, each character exactly one
pixel wider than the regular's (what RC.EXE measures for MS Sans Serif Bold
at 8 pt) and 0.35 of a pixel (at 8 pt) heavier, so its stems are MS Sans
Serif Bold's two pixels instead of Liberation Bold's 1.6 (each outline
repeated beside itself, one shape for the renderer), cut to the Latin scripts
like the others; `make_rapidr_sans.py
<unpacked archive folder>` makes it.

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
