# Your program in every language

A RapidR program can show its text in English, Spanish, Portuguese, Chinese
or any other language. You write the program in one language (usually
English), wrap its text in `TR$`, and put the translations in small text
files beside the program. Nothing else changes: a program without
translation files shows its own text, exactly as before.

## A complete example

`hello.rr`:

```basic
' Run it in Spanish:  rapidr run hello.rr es
IF COMMAND$(1) <> "" THEN Application.Language = COMMAND$(1)

PRINT "Language: "; Application.Language
PRINT TR$("Save file")
PRINT TR$("Open", "menu")
PRINT TR$("Hello, {name}!", "name", "Ana")
PRINT TRN$("{n} file", "{n} files", 1)
PRINT TRN$("{n} file", "{n} files", 5)
```

`lang/es.po`, in a folder named `lang` beside the program:

```po
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

msgid "Save file"
msgstr "Guardar archivo"

msgctxt "menu"
msgid "Open"
msgstr "Abrir"

msgid "Hello, {name}!"
msgstr "Hola, {name}!"

msgid "{n} file"
msgid_plural "{n} files"
msgstr[0] "{n} archivo"
msgstr[1] "{n} archivos"
```

`rapidr run hello.rr es` prints:

```text
Language: es
Guardar archivo
Abrir
Hola, Ana!
1 archivo
5 archivos
```

`rapidr run hello.rr pt` (no `pt.po`) prints the program's own English
text. `rapidr run hello.rr es-AR` prints the Spanish: Argentina's Spanish
looks in `es-AR.po` first, then in `es.po`.

The same program is in [examples/translations](../../examples/translations/),
with Spanish, Portuguese and Chinese files; its canvas row draws the greeting
with `TextOut`, so the Chinese is on a label and on a canvas:

![The translations example in Chinese](images/translations/languages-chinese.png)

## Text in code

| You write | What it gives |
|---|---|
| `TR$("Save file")` | the translation of "Save file", or "Save file" itself |
| `TR$("Open", "menu")` | the translation of "Open" *as a menu item*: a **context** tells two meanings of the same word apart (the PO file's `msgctxt`) |
| `TR$("Hello, {name}!", "name", user$)` | placeholders by name: `{name}` becomes the value. Translators can move it anywhere in the sentence |
| `TR$("Open {file}", "menu", "file", f$)` | a context and placeholders together |
| `TRN$("{n} file", "{n} files", count)` | the right form for `count` in the language (one form in Chinese, two in Spanish and Portuguese, three in Polish); `{n}` is the count |

How `TR$` tells a context from placeholders: after the text, an **odd**
number of arguments starts with the context; an even number is only
placeholders (a name, then its value). `TRN$` takes them the same way after
the count.

`{{` and `}}` give the braces themselves (`TR$("Use {{ and }}")` shows
`Use { and }`). A `{name}` nobody gave stays as written.

## The language

- `Application.Language` is the system's language until the program sets
  it: on macOS and Windows the system's setting, on Linux the `LANGUAGE`,
  `LC_ALL`, `LC_MESSAGES` or `LANG` variable, in a browser the browser's
  language.
- `Application.Language = "pt-BR"` switches: the next `TR$` speaks
  Brazilian Portuguese. Any spelling works (`pt_br` reads back as `pt-BR`);
  `""` goes back to the system's language.
- Fallback: `es-AR` → `lang/es-AR.po` → `lang/es.po` → the program's own
  text. Chinese named by its region alone also takes its script's file
  (`zh-CN` reads `zh-Hans.po`, `zh-TW` reads `zh-Hant.po`).
- `Application.OnLanguageChange` fires after the program changes the
  language (at the next turn of the event loop, in a program with
  windows). Text you made with `TR$` is a plain string: make it again there.

```basic
SUB LanguageChanged
    SaveButton.Caption = TR$("Save file")
END SUB
Application.OnLanguageChange = LanguageChanged
```

## The translation files

- One file per language, named by the language: `lang/es.po`,
  `lang/pt-BR.po`, `lang/zh-Hans.po` (`es_AR.po` with an underscore works
  too).
- Where: in a `lang` folder beside the program file you run (`rapidr run
  hello.rr`, RapidR Studio's Run), beside a built executable, or beside a
  web page's files.
- A build takes them along: `rapidr build` (native or `--interp`), the
  web builds and RapidR Studio's Build copy the `lang` folder's `.po`
  files into what they make — a macOS app's `Contents/Resources/lang`,
  `lang/` beside an `.exe` or in a Linux AppDir's `usr/bin`, `lang/`
  beside a web page. A built app speaks every language you translated,
  wherever it's started ([Building apps](building-apps.md#the-programs-data-files-go-with-it)).
- They stay plain files in what you ship, so a user can add a language
  without rebuilding: put `lang/fr.po` beside the app's other files. On
  the web, also add its name to `lang/index.txt`: a page asks only for
  the files listed there, so a language it has no file for costs no
  request.
- The format is PO, the plain-text format translation tools and
  translators already know. RapidR reads it with its own small reader:
  - `msgid` is the program's text, `msgstr` its translation; `msgctxt` a
    context; `msgid_plural` and `msgstr[0]`, `msgstr[1]` … the plural forms.
  - Long text can go on several lines of `"…"`, which are joined; `\n` is a
    new line, `\t` a tab, `\"` a quote, `\\` a backslash.
  - `#` lines are comments. Entries marked `#, fuzzy` (a translation that
    still needs checking) are not used, as other tools do.
  - The first entry (`msgid ""`) is the header. Its `Plural-Forms` line says
    how the language counts:

    | Language | Plural-Forms |
    |---|---|
    | Spanish, English, German, Portugal's Portuguese | `nplurals=2; plural=(n != 1);` |
    | French, Brazilian Portuguese | `nplurals=2; plural=(n > 1);` |
    | Chinese, Japanese | `nplurals=1; plural=0;` |
    | Polish | `nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 \|\| n%100>=20) ? 1 : 2);` |

    Without one, the English rule is used.
- A missing folder, file or entry is never an error: the program shows its
  own text. A line RapidR can't read is skipped; the rest of the file still
  works. Save the files as UTF-8. A file that isn't UTF-8 (one an older
  program or editor saved as Windows-1252, an `é` as one byte) is read as
  Windows-1252, so no letter turns into `?` or `�`; a `?` already *in* the
  file (Chinese that an old tool lost when it wrote it) can't come back, so
  write the file again. To write a catalog from a program, use
  `WriteText` (or `WriteStr(Text, LEN(Text))`) on an `RFileStream`: see
  [Text and bytes](language.md#text-and-bytes-accents-chinese-emoji).
- Chinese, Japanese and Korean show on labels, buttons and canvases alike:
  the desktop uses the system's CJK font and the web the Noto fonts that
  ship with the build; nothing to install.
- A console program prints translations as they are when its file is a
  `.rr` file (or any file with a non-English letter in it). A `.bas` file
  of plain English letters is taken as a classic program, whose console
  shows the old code page: `¡` and `ó` would show as other signs
  there. Windows and their captions are not affected.

## Not yet

- Forms translating their own captions without code (coming next).
- Numbers, dates and money in each language's style (`FORMAT$` and the
  other functions stay exactly as they are).
- A translations editor in RapidR Studio.
