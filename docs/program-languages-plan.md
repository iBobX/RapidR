# Your program in every language: translation for RapidR programs (plan, 2026-10-21)

Robert (2026-10-21): RapidR programs, such as RapidR Database Studio, must speak English, Spanish, Portuguese and Chinese. RapidR's languages plan (ROADMAP "Languages", master-plan M5) covers Studio, the manual and the messages. This plan adds what **programs** need. Today they have no translation layer.

## 1. Writing a translatable program
- **Text in code:**
  - `TR$("Save file")`: the English text is its own key, so a beginner needs nothing else.
  - A context tells two meanings apart: `TR$("Open", "menu")`.
  - Placeholders by name: `TR$("Hello, {name}!", "name", user)`. Translators can reorder them.
  - Plurals: `TRN$("{n} file", "{n} files", count)`, with each language's plural rules from Unicode CLDR (Chinese has one form; Spanish and Portuguese have two).
- **Text in forms translates itself.**
  - Properties marked `translatable` in the registry (Caption, Hint, Text of labels, menu items, column titles) are looked up when the form is created. They are looked up again when the language changes.
  - A program translates without changing a line.
  - It is active only when the project has translation files, so RapidQ programs behave exactly as before.
- **At run time:**
  - `App.Language` follows the system by default, and the program can set it (`App.Language = "pt-BR"`).
  - `OnLanguageChange` lets code-made text refresh.
  - Fallback: `es-AR` → `es` → the source text.
- **Numbers, dates and money per language:**
  - New functions: `LFORMAT$`, `LDATE$`, `LMONEY$`.
  - `FORMAT$` and the other RapidQ functions stay exactly as they are.

## 2. The files
- **One file per language:** `lang/es.po`, `lang/pt-BR.po`, `lang/zh-Hans.po`.
  - The plain-text PO format: every translation tool and translator already knows it.
  - RapidR reads it with its own small parser. No gettext library (GPL/LGPL).
- **Plural rules and locale formats:** the Unicode ICU4X crates (Unicode-3.0, already allowed in `deny.toml`).
- **Bundling:**
  - Builds embed the files: native, interpreted and web alike.
  - Or keep them beside the program, so a user can add a language without rebuilding.
  - Done (T1b, 2026-10-22): every build copies `lang/*.po` beside what it makes, as files (a macOS app's `Contents/Resources/lang`, `lang/` beside an executable or a web page); a web page also gets `lang/index.txt` and asks only for the catalogs it lists.

## 3. In Studio
- **A Translations editor:**
  - columns: source | es | pt-BR | zh-Hans;
  - each row has a status (new, translated, needs review, outdated) and shows where the text is used, with the form's picture;
  - **Find untranslated**.
- **Extraction is automatic.** The language service knows every `TR$`/`TRN$` and every translatable property. When the English changes, its translations are marked *outdated*.
- **RAI drafts translations:**
  - with any provider or a local model, marked *needs review* until a person accepts them;
  - it keeps code names (RButton, Caption) and placeholders untouched, checked by a lint;
  - no system-specific translation service, so the result is the same on every OS and the web.
- **Preview a form in any language** in the designer, with a "longest language" check for text that won't fit.

## 4. Chinese, and later right-to-left
- **Typing Chinese:** the kernel already receives the input method's composition (winit IME events and preedit text in the kernel). It gets a walk-through with real Pinyin input on macOS, Windows and Linux.
- **Fonts:**
  - The desktop uses the system's CJK fonts.
  - The web needs a font file. RapidR already ships OFL-1.1 Noto Sans SC and KR in the web fallback (LICENSES.md §Noto; legal review 2026-10-22, item 2c). The OFL covers font files only, never code. Its duties: ship the copyright line and the OFL text with the font; never sell the font alone; subsets can't use a reserved name (Noto CJK has none); serve the font from our own origin.
- **Right-to-left** (Arabic, Hebrew) comes later: the kernel's layout mirroring.

## 5. Lanes (master-plan M5, plus what RapidR Database Studio asks for first)
| Lane | Work | Model, size |
|---|---|---|
| T1 | `TR$`, `TRN$`, placeholders, `App.Language`, `OnLanguageChange`, PO loading, fallback; both backends and the web; conformance cases | Opus M |
| T2 | `translatable` properties, applied by the runtime to forms | Opus M |
| T3 | `LFORMAT$` / `LDATE$` / `LMONEY$` with ICU4X | Sonnet M |
| T4 | Studio's Translations editor, automatic extraction, outdated marks, designer preview | Opus M (visual) |
| T5 | RAI translation drafts and the placeholder lint | Sonnet M |
| T6 | A Chinese walk-through (IME, fonts); the web CJK font decision | Opus S + Robert |
