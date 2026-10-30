# Spanish for the reference and the Rust-side texts: review sheet (ES-REFERENCE)

For Robert, 2026-10-30. Español neutro, [the glossary](glossary-es.md) and its decisions (compilar, aplicación, arreglo, controlador de eventos, añadir, biblioteca; tú; sentence case).

Part 1 (the registry's docs in Spanish, `crates/rapidr-lang/data/i18n/es/`) is written by its own lane.

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
