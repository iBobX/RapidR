# Spanish manual, batch 1: review sheet (ES-MANUAL-1)

For Robert, 2026-10-30. The nine most-read pages of the manual in Spanish (español neutro, [the glossary](glossary-es.md), your 10 decisions). Files: `docs/manual/es/<page>.md`; Studio's Help shows them when Studio is in Spanish (`rapidr ide --lang es`, then **Ayuda > Ayuda de RapidR**).

| Page | English words | Spanish words |
|---|---|---|
| Getting started (`getting-started.md`) | 3,301 | 3,727 |
| The manual's index (`README.md`) | 1,631 | 1,867 |
| The code editor (`studio-editor.md`) | 4,275 | 4,595 |
| Projects (`studio-projects.md`) | 2,513 | 2,768 |
| Studio in your language (`studio-languages.md`) | 837 | 955 |
| Versions (`versions.md`) | 2,141 | 2,398 |
| Debugging (`debugging.md`) | 7,174 | 7,916 |
| The AI assistant and RAI (`ai-assistant.md`) | 7,598 | 8,270 |
| Troubleshooting (`troubleshooting.md`) | 2,420 | 2,771 |
| **Total** | **31,890** | **35,267** |

Every code block was checked with `rapidr check` (10 of the 14 BASIC blocks compile, in English and Spanish alike; the other 4 are fragments with `...` that do not compile in English either). Comments and shown text are translated, never a keyword or a name; the translated samples were run once where they run (hello, counter, the diff view, the division by zero) and print what the page says. The 31 paragraphs that touch consent, privacy, keys, costs, licences or what is sent carry `<!-- legal -->` above them, for the lawyer (`grep -n "<!-- legal -->" docs/manual/es/*.md`). Pictures are the English ones (this lane makes none).

## Where I had a real doubt

| English | Español | Why |
|---|---|---|
| Showcase (page title in the index) | Galería | *Vitrina* is Mexico/Colombia, *vidriera* Argentina, *escaparate* Spain. *Galería* is the one word all share. |
| completion (the editor's) | autocompletado | The glossary has no row; *completado* reads oddly. The menu command stays *Completar palabra* (Studio's own). |
| data tip | sugerencia de datos | Glossary: tooltip = *sugerencia*. |
| a watch (noun) | una expresión observada | From the Studio sheet (*Expresiones observadas*). The pane is **Observar**. |
| Hover | información flotante | Studio's *Mostrar información flotante*. In prose I also say "deja el mouse sobre". |
| Longest answer (Settings) | Máximo (la respuesta más larga) | The label in Studio's Spanish is "Máximo:" (it did not fit longer); the manual says both. |
| Workspace ID | ID del espacio | Studio's label; the text says *espacio de trabajo* for Anthropic's own word. |
| Build (the Output's page) | Compilación | The page's name has no catalog entry; *Compilar* is the command, *Compilación* the noun the status line already uses. |
| Step Into Specific / Set Next Statement | Entrar en llamada específica / Establecer la siguiente instrucción | Studio's menu words (Q6). |
| Build me… (a prompt) | Hazme… | Studio's own sample: "Hazme una calculadora de propinas". |
| array, first mention | arreglo (array) | Q3 recommendation: say "(array)" once per page so searching works. |
| Windows and macOS messages (SmartScreen, Gatekeeper, System Settings) | my best Spanish with the English in italics beside | I wrote *Windows protegió su PC*, *Más información*, *Ejecutar de todos modos*, *Configuración del Sistema > Privacidad y seguridad*, *Abrir de todos modos*. I could not check them on a Spanish Windows or Mac: please compare in the VMs. (macOS says *Ajustes del Sistema* and *Abrir igualmente* in Spain.) |
| the Windows installer's pages | kept in English with the Spanish beside | The installer (`tools/release/windows/rapidr.iss`) has no Spanish yet. |
| Alt+F, then R (File > Open Recent) | Alt+A, luego R | The menu letters change with the language (`&Archivo`); the same for Changed Outside Studio (Alt+A, F). |
| the Theme values (RapidR Light, RapidR Dark, RapidR High Contrast, Classic) | left as the Properties pane shows them | The dropdown's values are English words in Studio (`LOOKS` in `ide/panels.inc`); the *View > Theme* menu says *RapidR claro / oscuro / contraste alto*. |
| M, N, D (file badges) | M cambiado, N nuevo, D quitado | The letters are fixed in Studio; *D* does not match *quitado*. |
| thousands, decimals | 32 000, 4096, 0.61 s | Non-breaking space from five digits, none for four; the point for decimals (your open ⚑ in the glossary). |
| Output lines a program or the runtime prints (`Row 1 (Id 1): Score 10 → 12`, `Handle can't be changed…`, run-time errors) | left in English | They are the runtime's words and are not translated yet (the "Not yet" list of *Studio en tu idioma*). |

## Texts the manual names in Spanish but Studio still shows in English

The manual uses the Spanish the catalog will have; until these are in `es.po` a reader sees the English on screen. Each is one `TR$` or `tr_ui` line.

- **The code editor's change box** (`Undo This Change`, `Close`, from the gutter mark): hard-coded in `crates/rapidr-ui-kernel/src/components/codeeditor/popup.rs` (lines 298 and 363); not in any catalog. The manual says **Deshacer este cambio**.
- **The Help pane's own words** (`Insert into the code`, `Properties`, `Methods`, `Events`, `No entry for`): `crates/rapidr-studio/src/help.rs`. Known in the "Not yet" list.
- **The assistant's cards and live strip** (`Created`, `Proposed a change to`, `Started from the recipe`, `Fixed`, `Refused`; `Reading the help of …`, `Looking at the form…`, `Writing main.rr (+318 lines)…`, `Checking (compiling)…`, `Thought for 12 s`, `Answer was long: continuing (part 2 of up to 5)`, `You applied the change to …`): written by Studio in Rust, not in `es.po`.
- **The Search pane's `In`, `Case`, `Word`** (the manual says *Buscar en*, *Mayús.*, *Palabra*, as the Studio sheet decided) and the Output's `Build` page.

## Found in the English pages (not changed, only said)

- `README.md` says **2.119.0 (Preview 2)**; Studio is at 2.121.x. The Spanish copies it; when the English is updated, `rapidr i18n coverage` flags the page.
- `README.md` lists **Databases** twice (two rows, different texts). Copied as is.
- The one English change I made: *Studio in your language*, "Not yet", last bullet (it said the Spanish manual was a sample of Getting started; it is now the nine pages).

## What remains

The manual has 37 pages; 9 are in Spanish (`rapidr i18n coverage`: 24.3%). The next batch is the language chapter, the designer, components, translations, the AI apps page and the web.
