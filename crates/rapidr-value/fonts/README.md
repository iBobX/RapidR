# Fonts built into RapidR

The Liberation fonts 2.1.5 (Sans, Serif and Mono, Regular), unmodified, from
<https://github.com/liberationfonts/liberation-fonts> (release 2.1.5,
`liberation-fonts-ttf-2.1.5.tar.gz`, SHA-256
`7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`).

They're licensed under the SIL Open Font License 1.1 (`OFL-1.1.txt`;
authors in `AUTHORS`). RapidR draws text on bitmaps with them
(`src/objects/text.rs`): they have the same character widths as Arial,
Times New Roman and Courier New, the fonts RapidQ programs name.

**RapidR Sans** (`RapidRSans-Regular.ttf`) is a Modified Version of
Liberation Sans 2.1.5 under the same licence (SIL Open Font License 1.1;
renamed, as the OFL asks: Liberation is a Reserved Font Name). It is the
face RapidR draws MS Sans Serif with — RapidQ's default font, every
component's — so a form laid out for RapidQ fits in RapidR as it did there:
each Windows-1252 character is exactly as wide as MS Sans Serif's at 8 pt
on a 96-dpi screen (its em is 11 pixels; the outline scaled horizontally to
that width), and a line is 13 pixels high with the baseline 11 pixels down,
as Windows draws it. The widths are RapidQ's own `TextWidth` of each
character, measured with RapidQ's compiler on Windows 11; no Microsoft font
data is used. `tools/fonts/make_rapidr_sans.py` makes it from
`LiberationSans-Regular.ttf` (reproducibly) and explains the details.
