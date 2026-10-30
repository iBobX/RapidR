# Spanish manual, batch 2: review sheet (ES-MANUAL-2)

For Robert, 2026-10-30. The other 28 pages of the manual in Spanish (español neutro, [the glossary](glossary-es.md), your 10 decisions, the choices of [batch 1](review-es-manual-1.md)). With batch 1 the manual is complete in Spanish: **37 of 37 pages** (`rapidr i18n coverage es`: 100.0%). Files: `docs/manual/es/<page>.md`; Studio's Help shows them when Studio is in Spanish (`rapidr ide --lang es`, then **Ayuda > Ayuda de RapidR**).

| Page | English words | Spanish words | `<!-- legal -->` marks |
|---|---|---|---|
| Let AI apps work in Studio (`ai-apps.md`) | 2,760 | 3,145 | 20 |
| A program's eyes and hands: RWindowCapture and RAppHands (`app-eyes-and-hands.md`) | 2,163 | 2,503 | 8 |
| Archives: zip, 7z, tar and gzip files (`archives.md`) | 1,261 | 1,401 | 1 |
| Building apps and their icons (`building-apps.md`) | 3,255 | 3,675 | 5 |
| The CLI and the RapidR Runtime (`cli-and-runtime.md`) | 1,998 | 2,349 | 3 |
| The code editor in your programs (`code-editor.md`) | 581 | 674 | 0 |
| Components and objects (`components.md`) | 5,313 | 6,147 | 0 |
| Data science with RapidR (`data-science-with-rapidr.md`) | 1,869 | 2,099 | 0 |
| Data science (`data-science.md`) | 2,899 | 3,185 | 0 |
| Databases (`databases.md`) | 2,227 | 2,536 | 13 |
| Designing forms in RapidR Studio (`designing-forms.md`) | 2,740 | 3,036 | 1 |
| Differences from RapidQ, and extensions (`differences.md`) | 1,515 | 1,923 | 3 |
| DirectX, media and devices (`directx-and-media.md`) | 1,991 | 2,312 | 2 |
| Drawing: canvases, bitmaps and pixels (`graphics.md`) | 1,639 | 1,828 | 0 |
| Importing RapidQ programs (`importing-rapidq.md`) | 1,353 | 1,508 | 0 |
| The language (`language.md`) | 5,547 | 6,033 | 2 |
| Markdown files and RMarkdownView (`markdown.md`) | 1,315 | 1,481 | 0 |
| Networking: sockets and a local web server (`network.md`) | 2,301 | 2,572 | 12 |
| RapidR in other editors (`other-editors.md`) | 1,009 | 1,271 | 0 |
| Recipes for systems programs: memory, bits, a CPU, a screen, a keyboard (`recipes-systems.md`) | 2,122 | 2,336 | 0 |
| Rich text: RRichEdit (`rich-text.md`) | 1,024 | 1,127 | 0 |
| Keeping secrets: RSecrets (`secrets.md`) | 715 | 861 | 16 |
| Serial ports and IoT boards (`serial-ports.md`) | 1,381 | 1,530 | 0 |
| Showcase: bigger programs (`showcase.md`) | 4,819 | 5,203 | 3 |
| Your own SVG icons (`svg-icons.md`) | 738 | 822 | 0 |
| Your program in every language (`translations.md`) | 1,492 | 1,651 | 0 |
| The web (`web.md`) | 2,052 | 2,388 | 6 |
| Why RapidR (`why-rapidr.md`) | 1,856 | 2,207 | 9 |
| **Total (28 pages)** | **59,935** | **67,803** | **104** |

Batch 1 had 31,890 English and 35,267 Spanish words (9 pages, 31 legal marks). The whole manual: **91,825 English words, 103,070 Spanish** (+12%), 135 legal marks (`grep -n "<!-- legal -->" docs/manual/es/*.md`).

## How it was checked

- **Structure**: for every page the same headings (number, level, order), tables (rows), list items, images, `<a id>` anchors and code blocks as the English (`esverify`, a throwaway script: 0 errors on the 28 pages; on the two batch-1 pages `ai-assistant.md` and `studio-editor.md` it counts one link more than the English, harmless and not touched here).
- **Code**: the manual has 138 BASIC blocks. In every one the code outside comments and text strings is identical to the English; 109 compile with `rapidr check` in English and in Spanish alike, and the other 29 (fragments with `...`) fail in both. A translated string is only shown text (a caption, a message, a comment): never a `TR$` key, a file name or a format. The 46 other blocks (sh, text, toml, po...) differ only in comments and shown text.
- **Links**: `tools/check_doc_links.py` passes on `docs/manual` (1,395 links) and on all of `docs/` (1,705). The pages were written with the English anchors and a script (`fixlinks`) turned them into the Spanish headings' anchors, and changed 125 links that pointed to English pages (`../x.md`) into links to the Spanish page (`x.md`), batch 1's included.
- **Words**: `tools/check_words.py` finds only the old `tests/studio_lang.mjs` hit. No Spain-only words (lint of *ordenador, fichero, ratón, pulsar, icono, agregar, librería, manejador, bucle*...), no *vos*/*vosotros*/*usted*; the English left in the Spanish prose is only quoted (an OS message in italics, RAI's own words).
- **Legal paragraphs** (consent, privacy, keys and secrets, costs, what is sent, licences): point by point, `<!-- legal -->` above each, for the lawyer.

## Rule from the coordinator: RapidQ is not named

The Spanish pages do not name RapidQ in their prose, except `importing-rapidq.md` (the importer's page) and one line on the compatibility page (`differences.md`, whose H1 is now **Notas de compatibilidad y extensiones**, a neutral page: *clásico* / *programas anteriores*). What stays is code: file, command and option names (`rapidr import-rapidq`, `RAPIDQ.INC`, `rapidqCompatible`, `rapidq-corpus-status.md`, the `rapidq` fence label). The English pages still say it (another lane is cleaning them); **after that lane merges, `rapidr i18n coverage` will flag the pages whose English changed** and each stamp must be renewed after a look. The sentences I rewrote are in `cli-and-runtime.md`, `components.md`, `markdown.md`, `other-editors.md` and `differences.md`.

## The reference pages

`docs/manual/reference/*.md` (7 pages, about 143,000 words, `members.md` alone 127,000) are **generated** by `rapidr lang export --manual` from the language registry ("Do not edit"), so they were not translated by hand: the ES-REFERENCE lane translates the registry's text, and the Spanish pages link to the English reference (`../reference/...`) until it lands. `rapidr i18n coverage` counts only the 37 guide pages.

## Help shows the pages (one change in `crates/`)

Studio's Help embeds the translated pages in the binary: the `TRANSLATED` table of `crates/rapidr-studio/src/manual_search.rs` (a test, `every_translated_page_is_built_in`, keeps it equal to `docs/manual/es/`). I added the 28 new lines (and nothing else in `crates/`), in its own commit, so a page of the manual in Spanish is shown by the Help pane. Without it the test fails and the Help says "Not translated yet" for the 28 pages.

## Where I (or the translators) had a real doubt

| English | Español | Why |
|---|---|---|
| Page titles that differ from the index's names (`differences`, `importing-rapidq`, `designing-forms`, `secrets`, `app-eyes-and-hands`, `directx-and-media`, `showcase`) | the English H1, translated | The README's rows are shorter than the H1s ("Diseñar formularios" / "Diseñar formularios en RapidR Studio"; "Galería" / "Galería: programas más grandes"; "DirectX y multimedia" / "DirectX, multimedia y dispositivos"). Change the H1 or the index row; `differences.md` and `importing-rapidq.md` also depend on the RapidQ cleaning. |
| the menu arrow `▸` (designing-forms, components) | kept as in the English | Batch 1 writes `>`. One `sed` if you prefer one. |
| grid (RDBGrid) | cuadrícula | The glossary says *tablas* (the group), but the data is also *tabla*. |
| frame (RDataFrame) | el DataFrame | No row; the name is kept. |
| stream (RFileStream, SSE) | flujo | No row; batch 1 says "en flujo". |
| score, speaker, snare/kick | puntaje, altavoz, caja / bombo | Latin-American picks (*puntuación*, Spain's; *altavoz*: *parlante* in the Río de la Plata, *bocina* in Mexico; *redoblante* in the Cono Sur). |
| squares of the snake board | casillas | *Casilla* is the check box, *cuadro* the animation frame; a native check welcome. |
| loop (a MIDI port; a CPU's cycles) | ciclo (loop); ciclos | The glossary's one word; the English kept beside where it would confuse. |
| language tag; resize handles; UI kernel; fire (an event) | etiqueta de idioma; puntos de control; núcleo de interfaz de usuario (*kernel de interfaz* in data); se dispara | No glossary rows. |
| Model Context Protocol® and MCP™ are marks of LF Projects, LLC... (`ai-apps.md`, `network.md`) | translated, marked legal | A trademark notice; the glossary says notices stay in English. The lawyer decides. |
| wiretapping (CIPA) | escucha ilegal (*wiretapping*) | A legal term with no neat Spanish word. |
| Windows, macOS, VS Code messages and menu names (Keychain Access, Screen Recording, SmartScreen, Increase Contrast, OnComError's *The system cannot find the file specified.*, VS Code's *Go to Definition*...) | my best Spanish with the English in italics | I could not check them on a Spanish Windows, Mac or VS Code: please compare in the VMs. |
| the example programs' own captions (New, Trigger, Load MIDI..., the recipes' Step / Run / Reset, csv_explorer's labels) | left in English | The programs show English (no `TR$` in them); the manual names what is on screen. |
| the `hint:` prefix of `rapidr check` (cli-and-runtime) | left `hint:` | The compiler prints *sugerencia:* in Spanish; one word to change. |
| `RRichEdit`'s first sample string | left in English | Its `SelStart` / `SelLength` numbers depend on its letters. |
| translations.md output `Hola, Ana!` | `¡Hola, Ana!` | The English output block lacks the opening `¡` that `lang/es.po` has: fixed in the Spanish. |
| five-digit numbers in the type tables (65 535, -2 147 483 648) | non-breaking space | Values; say if you prefer them unspaced. |
| The VS Code extension's own labels (*Use the installed RapidR*, *Choose...*, *Download*...) | English in bold, my Spanish in italics | The extension has no translation yet; one sentence in `other-editors.md` explains. |
| Import report words (*Include files*, *Not found*) | English with the Spanish beside | `rapidr-import-report.md` is generated in English (`crates/rapidr-import/src/report.rs`). |

## Texts the manual names in Spanish but Studio still shows in English

(In addition to batch 1's list.) Each is one `TR$` or `tr_ui` line in Studio's Rust or `.rr`:

- The activity line `an-ai-app: Reading main.rr` (`ai-apps.md`); the **Choose the file to open.** box with **Open...** / **Cancel** in a web program (`web.md`).
- **Type Here** in the menu editor (the manual says **Escribe aquí**); the Toolbox group names the designer page uses (*Estándar, Adicional, Cuadros de diálogo, Sistema*).
- The first-line comment Studio writes in a new `Form2.rr` (the manual shows it in Spanish).
- The `RSecrets`, `RHttpServer`, RAR and `RSerial` runtime messages the pages quote (left in English as the runtime's own words).

## Found in the English pages (not changed, only said)

- `databases.md` has code samples with no fence in four sections ("Background queries", "A query's rows in a grid", "Column names and types", "The SQL editor") and a stray tail after "Encrypted connections" ("...The connection is not encrypted...") that contradicts the section. The Spanish copies both as they are.
- The English H1s of `differences.md` and `importing-rapidq.md` name RapidQ (see above).
- `README.md` still says 2.119.0 (batch 1's note).
