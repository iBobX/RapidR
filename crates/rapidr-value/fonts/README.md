# Fonts built into RapidR

The Liberation fonts 2.1.5 (Sans, Serif and Mono, Regular), unmodified, from
<https://github.com/liberationfonts/liberation-fonts> (release 2.1.5,
`liberation-fonts-ttf-2.1.5.tar.gz`, SHA-256
`7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`).

They're licensed under the SIL Open Font License 1.1 (`OFL-1.1.txt`;
authors in `AUTHORS`). RapidR draws text on bitmaps with them
(`src/objects/text.rs`): they have the same character widths as Arial,
Times New Roman and Courier New, the fonts RapidQ programs name.

## JetBrains Mono (the code editor's font)

`JetBrainsMono-Regular.ttf` and `JetBrainsMono-Italic.ttf` are JetBrains
Mono 2.211 by The JetBrains Mono Project Authors
(<https://github.com/JetBrains/JetBrainsMono>), the variable fonts (weight
axis 100–800) in the Latin subset Google Fonts distributes (229 characters:
ASCII, Latin-1, common punctuation and symbols), as packaged by Fontsource
(`@fontsource-variable/jetbrains-mono` 5.3.0,
`jetbrains-mono-latin-wght-normal.woff2` SHA-256
`18be452724bfdc236c074ca94a249a7f41a86752c7d04ab258ce9ed5651f6a7e`,
`jetbrains-mono-latin-wght-italic.woff2` SHA-256
`a8afa085e9ca5e53434e2ee918ba6b65c7dd4dda56509976b36591478c99d62e`),
converted from WOFF2 to TrueType with fontTools (the glyphs and tables
unchanged). Licensed under the SIL Open Font License 1.1, which declares no
Reserved Font Name for it (`JetBrainsMono-OFL.txt`).

RCODEEDITOR and RDIFFVIEW draw code with it (docs/ide-plan.md, decision
D8); programs can name it too (`Font.Name = "JetBrains Mono"`). Characters
outside the subset fall back to Liberation Mono and the fallback fonts.
