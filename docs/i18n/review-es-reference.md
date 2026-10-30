# Spanish for the reference and the Rust-side texts: review sheet (ES-REFERENCE)

For Robert, 2026-10-30. Español neutro, [the glossary](glossary-es.md) and its decisions (compilar, aplicación, arreglo, controlador de eventos, añadir, biblioteca; tú; sentence case).

Coverage at the end of the lane (`rapidr i18n coverage es`): Studio's text **2,559 of 2,559**, the compiler's messages **120 of 120**, the registry's docs **5,377 of 5,377**, the manual **39 of 39 pages** (the manual is the other lane's). Studio's text and the compiler's still carry the `#, review` marks they had.

1. [Part 1: the registry's docs](#part-1-the-registrys-docs)
2. [Part 2: Rust-side texts](#part-2-rust-side-texts)
3. [Part 3: RAI in Studio's language](#part-3-rai-in-studios-language)
4. Part 2b: the sweep (further down)

## Part 1: the registry's docs

Every component, property, method and event, built-in, statement, keyword, directive, type and constant group has its doc in Spanish: **5,377 texts** in `crates/rapidr-lang/data/i18n/es/*.toml` (one file for each data file: `forms.toml`, `input.toml`, `builtins.toml`, `language.toml` …), the texts Studio shows in the hover, the completion list, the signature help, the Help pane and `rapidr lsp`. The 2,886 different English texts were translated once and shared by every entry that uses the same one, so a member that several components have (`Left`, `Visible`, `OnClick`) reads the same everywhere. Code stays as written: every `` `code` `` span, name and placeholder was checked against the English by a script (none differ); no `app`, `fichero`, `ordenador`, `pulsar`, `manejador`, `agregar` or `bucle` is left. A second pass replaced *el usuario* (208 times) with *la persona*, as the glossary's voice asks, where the text speaks of a person; *nombre de usuario* and the like stay. `rapidr i18n coverage es --missing` (new) lists the docs a language still lacks, with their English, for the next lane. The 53 docs that came with Coach mode, Local AI, School Lock and RSystemInfo (development, merged at the end) follow the words Studio's own catalog uses (*modo tutor*, *pista*, *bloqueo escolar*).

### Where I had a real doubt

| English | Español | Why |
|---|---|---|
| data frame (`RDataFrame`) | tabla de datos | The glossary has no entry. Studio's Welcome sample said *marco de datos*; now both say *tabla de datos* (plainer for a school; *DataFrame* stays code). |
| grid (`RStringGrid`, `RGrid`) | cuadrícula | 68 uses; the Toolbox group is *Listas, tablas y árboles* (glossary). *Tabla* is kept for a database table. |
| callback | devolución de llamada | No glossary row. |
| signature help | ayuda de firma | No glossary row; the editor's hover is *información flotante*. |
| handle (an operating-system handle) | identificador | As Studio's debugger says. |
| thumb of a scroll bar / slider | cuadro de desplazamiento / control deslizante | No glossary rows. |
| MDI child / parent window | ventana hija / ventana principal | *Secundaria* is the other option. |
| board (ESP32, micro:bit) | placa | *Tarjeta* is Mexico's; *placa* is the one most countries share. |
| archive (`RArchive`) | archivo comprimido | Plain *archivo* would clash with *file*. |
| digest of a hash (`RSecrets`) | huella | No glossary row. |
| user-defined type (UDT) | tipo propio (UDT) | Avoids *el usuario*; the established term is *tipo definido por el usuario*. |
| tick of a timer | tic | A loanword left as it is in many Spanish docs. |
| commit / rollback in SQL | confirmar / revertir | The Pro Git word *commit* stays English (Q7); these are the SQL transaction words. |
| rounds (PBKDF2) / salted hash | iteraciones / hash con sal | The usual technical terms. |
| quoted sample output and sample values ("Column 1", "Saved the version …") | left in English | They are what a program prints, not Studio's text (glossary 3.10). |
| the key name *Space* | Space | Key names stay as the English shows them (glossary 1.7); a Spanish keycap says *Espacio*. |

## Part 2: Rust-side texts

Every English text that was still hard-coded in Rust now goes through `tr_ui` / `tr_ui_fill` (or a template that finds a text built by code and translates it where a person reads it) and is translated: **500 new texts** in `crates/rapidr-value/lang/es.po` (`rapidr i18n runtime`; 17 marked `# legal`: keys, costs, what RAI sees of the running program) and 4 in `ide/lang/es.po` (the Theme values). English is unchanged.

What is covered: the Help pages' own words (*Propiedad de RButton*, *Insertar en el código*, *Propiedades / Métodos / Eventos*, *No hay entrada para …*, the start page, a program's own names), the hover's and the completion's kind words, the code editor's change box and its screen-reader line, the dock's drag words and screen-reader names, the diff view's hunk sentences, RAI's live strip, steps, cards, statuses and the provider errors explained in plain words, Studio's project / versions / local-history errors, Versions' Undo line, the Theme values, a program's run-time errors (13 common messages), and `rapidr lsp` in the client's language. See it: `rapidr ide --lang es`, then F1 on `RButton`, **Ayuda**, a hover, **Ver > Versiones**.

### Where I had a real doubt

| English | Español | Why |
|---|---|---|
| No entry for {word} | No hay entrada para {word} | *Entrada* is the word the manual already uses for a Help page ("la página de una entrada"); *Sin resultados* would suit a search, this is a page. |
| classic | clásico | The tag on a name that has been in the language since before RapidR (the View > Theme menu uses *Clásico* too). |
| RapidR's own | propio de RapidR | The other tag; the pair reads "clásico · propio de RapidR". |
| Keyword ({kind}) | Palabra clave ({kind}) | `{kind}` is the registry's English word (block, operator …): data, not translated here. |
| default {value} | valor predeterminado {value} | The glossary has *predeterminado* for default. |
| its handler: SUB Name{params} | su controlador: SUB Name{params} | Q4 (*controlador de eventos*). |
| Hunk {n} of {total} | Fragmento {n} de {total} | The diff view's changed piece (a screen reader says it); *bloque de cambios* is the other candidate. |
| Above of {pane} | Encima de {pane} | The English ("Above of X") is not grammatical; the drag preview of the dock. Left alone in English, fixed in Spanish. |
| Dock left / top / right / bottom | Acoplar a la izquierda / arriba / a la derecha / abajo | *Acoplar* for dock, as in the pane's menu. |
| Auto-hide left … | Ocultar automáticamente a la izquierda … | Long for a preview label: the preview is wide, it fits. |
| Side by Side | Lado a lado | The document tabs' split view. |
| Thought for {secs} s | Pensó durante {secs} s | The assistant speaks in the third person here, as in the English; past tense, like *Respondido.* for "Answered.". |
| You / Assistant (the chat's speakers) | Tú / Asistente | *Tú* is the one place the person is named; the glossary's voice rules say nothing on a transcript's label. |
| Asking {provider} for its models… | {provider}: consultando los modelos… | Word order on purpose: "The server" is a subject (*El servidor*), and "a El servidor" can't be formed (glossary 1.9). |
| The server | El servidor | Beginning of a sentence only; "Add credit on {name}'s website" can't take it, but a local server has no credit. |
| Fixing {n} problems | Corrigiendo {n} problemas | The strip says *problemas* (what Problems lists) and the card says *errores*; the English does the same. |
| {n} errors to fix | {n} errores por corregir | *Por corregir* over *a corregir*: reads as a to-do. |
| Read {file} · Looked up {target} · Wrote the program … | Se leyó {file} · Se consultó {target} · Se escribió el programa … | The steps under an answer are past facts; *Se …* (impersonal) keeps them short and without a subject, like the English. |
| Refused a change to {file}: {why} | Se rechazó un cambio a {file}: {why} | `{why}` is one of four short reasons, translated by the same table. |
| about ${cost} | unos US${cost} | Glossary 1.6: a bare $ is a peso in many countries (marked `# legal`). |
| RAI wants to click and type in your running program. It sees whatever the program shows, including private data. | RAI quiere hacer clic y escribir en tu programa en ejecución. Ve todo lo que el programa muestra, incluso datos privados. | The consent card; by meaning, row by row (marked `# legal`). |
| RapidR Dark / RapidR High Contrast (Properties > Theme) | RapidR oscuro / RapidR contraste alto | Same words as the View > Theme menu. At the default width of the Properties pane the value is cut ("RapidR osc…"); the full word is in the list. Marked `#. too long`. |
| run-time error: {message} | error de ejecución: {message} | The line a stopped program prints; `Division by zero` is *División por cero*, `Subscript out of range` is *Subíndice fuera de rango*. |
| global variable (implicit: made by its first use) | variable global (implícita: se crea con su primer uso) | RapidQ's implicit variables; the English says it the same way. |
| 1 more line / {n} more lines (change box) | 1 línea más / {n} líneas más | The English printed "1 more lines"; the singular is added in both (the one change to the English text). |

### What is not translated, and why

- The assistant's tool notes are English in the program (the code and the audit log read them) and are translated where shown: the steps and the refusal card. A note no template knows stays English.
- Texts that go to the model (the system prompt, the tool descriptions and results, the nudges): English, on purpose (part 3 handles the language of the answers).
- Names that are code: property names in the project's Properties (`Theme`, `Targets`, `MainFile`, `Grid`, `Snap`, `Guides`, `Arguments`), `{kind}` words from the registry, a file's path.
- The history's own words stored in a project's versions (the commit messages "Saved the version …", "Went back to …") stay English where they are stored; only Versions' **Deshacer** line translates them.
- Run-time errors: 13 common messages are in; the many messages specific to a component, and the line a debugged program shows at its error in Studio, still say English.

## Part 3: RAI in Studio's language

For Robert, 2026-10-30. The assistant now uses **Studio's language**: its greeting, statuses, cards and answers are in the language Studio speaks (English or Spanish today; a row in a table adds Portuguese or Chinese), with RapidR's words in it. If the person clearly writes in another language, it follows them. The new setting is **Tools > Settings > AI > RAI answers in:** with two choices, *El idioma de Studio* (the default) and *El idioma en que escribo*; an older settings file has neither and means Studio's language.

What reaches the model (English, on purpose; the glossary's Decisions win over its older rows): the stable instructions carry, in a Spanish Studio, one block of about 800 characters (Studio's panes and RapidR's terms as `English=Spanish` pairs, *tú*, code stays English), and the last line of every request says the language: `Reply in Spanish (español) unless the person writes clearly in another language: then use theirs, …`, or `The user writes in English: reply in English, …` for a message that is clearly English. The tools give the model the registry's docs in Studio's language (`rapidr_describe`, a word's reference hits) and the manual's pages in it (a page not translated yet is read in English, and the tool says so). English adds nothing to the prompt.

### Where I had a real doubt

| English | Español | Why |
|---|---|---|
| RAI answers in: | RAI respon&de en: | The mnemonic is *d*: *R* is Recordar's (web) and *e* Proveedor's on the same page. |
| Studio's language | El idioma de Studio | A choice in a list, next to the other: with the article, sentence case. |
| The language I write in | El idioma en que escribo | *En que* over *en el que*: shorter, and the row is narrow. |
| Waiting for you — {status} | Pendiente — {status} | Part 2 had *Esperando tu respuesta —*: *respuesta* is wrong for a change to review, and the strip is one line: "…propuso un cambio: revís" was cut. |
| Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good. | Se guardan en esta computadora, en la carpeta de Studio — nunca en el proyecto. Eliminar es definitivo. | The Chats list's note has two lines; part 2's text took three and the last was cut. Same meaning, marked `# legal`. |
| the macOS Keychain / Windows Credential Manager | el Llavero de macOS / el Administrador de credenciales de Windows | Where your key is kept ("se guarda en …"): these were English words inside Spanish sentences. Their own names, as the glossary says (2.10). `# legal`. |
| the Secret Service (GNOME Keyring or KWallet) | el Secret Service (GNOME Keyring o KWallet) | The product names stay. |
| this browser tab, until it closes | esta pestaña del navegador, hasta que se cierre | Studio on the web. |
| memory, for this session only | la memoria, solo durante esta sesión | What the tests' key store says. |
| an environment variable | una variable de entorno | "A key for {name} is already found ({hint}, {store})". |
| (prompt) click=clic, mouse=mouse | (to the model) | The glossary says *hacer clic* and *mouse*; the pair list keeps the noun, the model builds the verb. |
| (prompt) add=añadir | (to the model) | Decision Q8; the glossary's older row says *agregar*. |
| (prompt) Speak to the person as tú, never vos, usted or vosotros | (to the model) | Glossary 1.1. |
| (prompt) menu paths: Herramientas > Configuración | (to the model) | Quoted as Studio shows them, even in an English answer to a Spanish Studio. |

### What is not done, and why

- `docs/manual/es/ai-assistant.md` still says the old thing (the answer follows your last message; *Esperando tu respuesta — un cambio por revisar*): the manual lane's. The English page now describes the new setting, so `rapidr i18n coverage es` lists it as changed since it was translated.
- The registry's English doc of `RAI.ReplyLanguage` doesn't name the new value `"app"` (the manual's RAI section does): `crates/rapidr-lang/data/*.toml` is another lane's.
- The slash commands' one-line descriptions are cut at the pane's edge, in English too.
- The web page was not run for the new flows (they carry `lang` and `webFiles`: both hosts when the page is built).

## Part 2b: the sweep

For Robert, 2026-10-30. Goal: no English visible anywhere in Studio when it is in Spanish. `tests/studio_lang.mjs` now walks **54 scenes in the pseudo-language** and **84 in Spanish** (it was 13 and 40; 299 checks in all): every menu and submenu, the dialogs and bars, Settings' three pages with a list opened on each, the Set Up RAI pages that need no network, the debugging panes with a program stopped, a run-time error, Versions' history and changes, the external-changes bar, the code helpers' editors, tooltips, the context menus that exist, and the dialogs a program shows (colour, font, message box: run with `RAPIDR_LANG`). A scene fails on any visible English or any cut-off text.

**What was found and fixed**

- **Texts that were never translated** (found by the scenes, fixed at the root): the colour and font dialogs a program shows (style and colour lists, the editor's boxes, titles), a message box's buttons and titles, the code helpers' colour names ("&HEEF6FA · white") and the line help after a value (a registry doc shown in English), the language service's notes (RapidQ-compatibility, `&&`, `NOT`, `$INCLUDE`; 10 new compiler-catalog messages, RR0025 to RR0034), quick-fix titles, rename errors, the lines Run in Browser writes, and the line a stopped program prints (Output, the strip at the line, the status's "Paused, …").
- **Texts that did not fit** (the layout now follows the words; the English layout is unchanged where it fitted): the Breakpoints condition strip (*Cuando, Veces, Registro, Activado*), the Replace / All buttons of the find bar, the font dialog's style and size columns, the colour dialog's editor (boxes, preview label, add button), the caret's place in the status bar, the local history's list, its one-line description.
- **Theme in Properties**: the value column is narrow, so *RapidR oscuro* was cut. The three looks have their own words **only in that list** (*Claro, Oscuro, Contraste alto*; context `a look in the Properties pane's Theme row`); the View > Theme menu keeps *RapidR claro / oscuro / contraste alto*. The `#. too long` notes are gone.
- **Run-time errors**: 27 new templates (13 to 40): the END / evaluation messages, event loops, string length, DLL calls, hardware ports, `PEEK` / `POKE` / `VARPTR`, files that can't be opened or created, `UBOUND`, the native builds' variant of "does not refer to an object", and a message under an object (`Sales.At: …`, `DIM Grid: …`: the head stays, the message after it is translated). The debugger's stop line goes through the same templates (`Session.StopMessage`).
- Counts: runtime catalog 827 → 933 texts, compiler 110 → 120, Studio 1,482 (3 replaced, 1 reworded). `rapidr i18n studio|runtime|compiler --check`, `rapidr i18n check` and `coverage es` (2,367 / 120 / 5,324, all 100%) are clean.

**What is not there** (so nothing to translate): the dock has no context menu (its tabs and pane buttons have tooltips and drag previews, part 2); the toolbox, the project tree, the Properties rows and the designer have no context menu of their own (the toolbox's and Properties' search boxes and the tree's name editor have the edit menu, which is translated); Versions in this version has no Pro mode or merge tab.

**Not done**: run-time messages specific to one component (databases, sockets, printers, serial ports, streams, archives) and the `[WARN]` lines on stderr stay English; the Set Up RAI pages that need a server, the AI consent card and the import report need a model or a native file dialog; the web page was not run (`RAPIDR_LANG_WEB=0`).

### Where I had a real doubt

| English | Español | Why |
|---|---|---|
| RapidR Light / Dark / High Contrast (Properties > Theme) | Claro / Oscuro / Contraste alto | The value column is ~100 px: *RapidR oscuro* is cut. The list names the looks of one row, so the product word is not needed; the menu keeps it. |
| Write {name} / Change to {name} / Add $INCLUDE "{inc}" | Escribir / Cambiar a / Añadir | Quick fixes are menu items: infinitive; *añadir* (Q8). |
| '{name}' is already the name of {what} here | '{name}' ya está en uso aquí: es {what} | *{what}* carries its own article (*la variable global x*, *el campo y*); "ya es el nombre de el campo" would need *del*, so the sentence is built with *es*. |
| the SUB / the FUNCTION / the TYPE | la subrutina / la función / el tipo (TYPE) | Glossary: SUB = subrutina; a TYPE keeps its keyword. |
| '{name}' is a builtin's name | es el nombre de una función incorporada | *Builtin* is not a word a 12-year-old has; *incorporada* is what Studio's docs say. |
| RapidQ's compiler refuses it (this project is RapidQ-compatible) | el compilador de RapidQ la rechaza (este proyecto es compatible con RapidQ) | These notes only appear in a project that asks for RapidQ compatibility: they name RapidQ as the English does (the importer's world, not the RapidR-first text). |
| Regular (font style) | Normal | Windows' own word for the style list. |
| Maroon, Navy, Teal, Silver, Aqua, Lime | Granate, Azul marino, Verde azulado, Plateado, Aguamarina, Lima | The font dialog's sixteen colours, as Windows' Spanish calls them; *Plateado* over *Plata* so it reads as a colour. |
| &Abort | &Anular | Windows' Spanish for the button; *Abortar* is the other candidate. |
| &Yes / &No | &Sí / &No | The mnemonic is the letter, so *Sí* gets S. |
| Confirm (message box title) | Confirmar | The title of a question box. |
| Hu&e: / &Sat: / &Lum: | Ma&tiz: / &Sat: / &Lum: | The colour editor's boxes; *matiz* is the name Windows' dialog gives. |
| dark blue / light grey | azul oscuro / gris claro | The shade follows the colour in Spanish (template `{name} oscuro`). |
| brown | marrón | *Café* is regional. |
| Waiting for events | Esperando eventos | Studio puts it after "Pausado," in lower case. |
| A copy each time the file was saved, kept in the project's .rapidr folder (never in a version) | Una copia en cada guardado, en la carpeta .rapidr del proyecto (nunca en una versión) | The first Spanish was 12 characters longer than the English and was cut by the button beside it. |
| Cannot open file {path}. | No se puede abrir el archivo {path}. | RC.EXE's words: the classic English is kept beside a Spanish message only where the compiler catalog does it for compile errors, so a run-time error has none. |
