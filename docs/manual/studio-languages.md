# Studio in your language

RapidR Studio speaks your language: its menus, dialogs, messages, the
compiler's errors and Help. **Español** comes first (Preview 5);
Portuguese and Chinese follow. Until a language is complete, what isn't
translated yet shows in English.

## Choose the language

**Tools > Settings > General > Language**:

- **System default** (the start): your computer's language when Studio has
  it, else English.
- **English**, **Español** … one for each language Studio has.

Studio changes at once, without starting again: the menus, the panes, the
open dialogs and the status bar. Your choice is kept for the next time.
**Cancel** in Settings goes back to the language you had.

![Settings > General > Language set to Español: the dialog in Spanish](images/studio-languages/settings-language.png)

![The menu bar in Spanish: Archivo, Edición, Ver, Proyecto, Ejecutar, Depurar](images/studio-languages/menus-spanish.png)

The keyboard shortcuts never change with the language (Ctrl+F finds,
F5 runs). Each menu's underlined letters are chosen for the language, so
Alt + a letter still opens a menu or picks an item.

On the command line, `rapidr ide --lang es` starts Studio in Spanish for
that session; in Studio for the web, add `?lang=es` to the page's address.

## The compiler's messages

Problems shows the compiler's errors in Studio's language:

![Problems in Spanish: Se esperaba el fin de la línea, pero se encontró THEN](images/studio-languages/problems-spanish.png)

The `rapidr` command speaks your system's language, or the one
`RAPIDR_LANG` names:

```
$ RAPIDR_LANG=es rapidr check hello.rr
hello.rr:3:11: error: Se esperaba el fin de la línea, pero se encontró THEN [Expected end-of-line but got THEN]
```

- A message that RapidQ's compiler also gives keeps RapidQ's own words
  beside the translation, in brackets, so the words you know (and search
  for) are still there.
- `rapidr check file.rr` compiles a program and lists its errors without
  running it.
- `RAPIDR_LANG=en` gives the English messages; tests and scripts that read
  them can set it.

## Help in your language

The Help pane (F1) shows the language's entries — RButton, Caption,
PRINT, MID$ — in Studio's language when they are translated, and the
manual's pages: **Help > RapidR Help**, then *Getting started*. A page with
no translation yet shows in English, with a note that says so. Code, names
and examples are never translated: `RButton`, `Caption`, `OnClick` stay as
you write them.

## Help translate RapidR

Translations are plain text files, in the same PO format your own programs
use ([Your program in every language](translations.md)). Each part of
RapidR has a template and one file per language:

| What | Template | A language's file |
|---|---|---|
| Studio's text | `ide/lang/studio.pot` | `ide/lang/es.po` |
| RapidR's own words (Studio's panes, dialogs, the code editor) | `crates/rapidr-value/lang/runtime.pot` | `crates/rapidr-value/lang/es.po` |
| The compiler's messages | `crates/rapidr-diagnostics/lang/compiler.pot` | `crates/rapidr-diagnostics/lang/es.po` |
| The manual | `docs/manual/<page>.md` | `docs/manual/es/<page>.md` |
| The language's entries (Help, hover) | `crates/rapidr-lang/data/**/*.toml` | `crates/rapidr-lang/data/i18n/es/*.toml` |

1. **Follow the glossary**: [docs/i18n/glossary-es.md](../i18n/glossary-es.md)
   for Spanish (the words, *tú*, sentence case, the menus' letters).
2. **Start the file from its template** (or bring it up to date):
   `rapidr i18n update ide/lang/studio.pot ide/lang/pt.po`. Each text is a
   `msgid`; write its translation in `msgstr`.
3. **Keep the markers**: `{name}` placeholders (they may move, never
   change), `` `code` ``, one `&` before the menu letter, and a menu's
   `...` as three dots. `rapidr i18n check ide/lang/es.po` finds any lost.
4. **Mark a draft**: `#, review` above an entry says a person still has to
   check it.
5. **See it**: Settings > Language, or `rapidr ide --lang es`.
6. **Count what's left**: `rapidr i18n coverage`.
7. **A manual page**: copy it to `docs/manual/es/` and translate it; then
   `rapidr i18n manual-stamp es <page>.md` records which English it
   translates, and `rapidr i18n coverage` says when that English changes.
8. **The language's entries**: a `[docs]` table, each entry by its name in
   capitals — `"RBUTTON" = "…"`, `"RBUTTON.CAPTION" = "…"`, `"MID$" = "…"`.

A new language for Studio is a new `ide/lang/<language>.po`: Settings lists
it by its own name (*Português*, *中文（简体）*) as soon as it is there.

### The pseudo-language

`rapidr i18n studio` also makes `ide/lang/xx.po`, the pseudo-language: every
text Studio translates, accented and a third longer. Studio in it
(`rapidr ide --lang xx`) shows at a glance any text that isn't translatable
(plain English) and any place a longer language won't fit (text cut off).
RapidR's tests run Studio's main scenes in it (`tests/studio_lang.mjs`).

![Studio in the pseudo-language: every text accented and longer](images/studio-languages/pseudo-language.png)

## Not yet

- Studio's text is a sample in Spanish today (the menu bar, part of the
  File menu, Settings' Language): the rest comes with the translation
  work, then Portuguese and Chinese.
- The run-time error messages of a running program, and the Help pane's own
  words around an entry ("Property of RButton"), are in English.
- The manual in Spanish starts with a sample of *Getting started*.
