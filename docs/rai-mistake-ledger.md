# The mistake ledger: what models get wrong in RapidR, and what we do about it

docs/preview4-plan.md K4. Every model that writes RapidR — a cloud model or
a small one on a laptop — makes some mistakes again and again. Each one we
see goes here, with its evidence and one decision:

- **(a) an extension** — when the habit is safe and unambiguous, RapidR
  takes it, additively: RapidQ programs never change meaning, and the
  extension works the same in the interpreter, native builds and the web;
- **(b) a message that says the fix** — the compiler (in RapidQ's words
  first, when RapidQ has them) or the editor's hints add what to write
  instead;
- **(c) a line on the language card** — the ~1,300-token card every RAI
  chat carries (`rapidr lang export --card`; crates/rapidr-lang/src/card.rs).
  Its rules are `[[rule]]` entries in crates/rapidr-lang/data/language.toml,
  and each rule's example is compiled and run by `rapidr lang truth`.

Some need more than one. The evidence is named, so a decision can be
checked again when models change.

## The ledger

| # | The mistake | Evidence | Decision | Where it lives, how it is guarded |
|---|---|---|---|---|
| 1 | `FOR i AS INTEGER = 1 TO n` (other BASICs' loop): "Syntax error in FOR statement" | Robert's evening with DeepSeek (deepseek-flash, 2026-10-20, the RapidRWebServer project); models trained on other BASICs write it constantly | **(a)** extension `FOR i AS INTEGER = 1 TO n [STEP s]`: declares the counter where it isn't yet (a local in a SUB, a global in the main program); the same type is the same variable; another type, an array, a constant or a non-number type is an error that says so. **(c)** card rule `for` | `rapidr_ast::for_as`, the first pass of both compilers (bcgen, codegen); parser `parse_for`. Conformance `for_as`, `for_as_errors` (VM, native, web); unit tests in for_as.rs; FOR's registry syntax `FOR variable [AS type] = …` (truth-checked) |
| 2 | The same global DIMmed in two modules; globals re-declared after a rewrite | DeepSeek evening: `Identifier siteCount already used, try another name` with no word of where the first one was (globals.rr vs a module) | **(b)** RapidQ's message, then where the first one is: `— it is declared in globals.rr:12 already: a global is declared once in the whole program …`; a file `$INCLUDE`d twice is said as such. **(c)** card rule `one-program`. RAI's `project_outline` lists every file's globals | `rapidr_ast::rapidq_checks` (place mark `rapidr_diagnostics::place_mark`, resolved by `PreprocessResult::resolve_places` in the CLI, the web compiler and the language service). Conformance `messages_say_the_fix`; langsvc test `messages_say_the_fix_across_files` |
| 3 | Calling helpers no file has (`NStr`, `srvOn`) | DeepSeek evening | **(b)** `Unknown SUB or FUNCTION 'NStr' — did you mean INSTR or STR$?` (the program's SUBs and FUNCTIONs and the registry's builtins, the nearest, ties all said), or `— no file of the program has a SUB or FUNCTION of that name …`. **(c)** card rule `known-names`; `project_outline` | `rapidr_ast::unknown_routine_fix`, used by bcgen. Conformance `messages_say_the_fix`; unit test in for_as.rs; langsvc golden `diagnostics` |
| 4 | "Type mismatch, expecting type DOUBLE, but got STRING" on lines with RServerSocket calls | DeepSeek evening. Cause found 2026-10-22: not the socket's arguments — the variable the text went into was never declared, and an undeclared name is a DOUBLE (RapidQ-exact) | **(c)** card rule `declare` (first on the card). **(b)** check_code's friendly hint under RapidQ's message: "the variable on the left is a DOUBLE — declared so, or never declared …: DIM it AS STRING". Exact method signatures: K2 | crates/rapidr-studio/src/friendly.rs (`for_message`, test); the rule's example truth-checked |
| 5 | `NOT FILEEXISTS(x)` — always true: FILEEXISTS gives 1 (RapidQ-exact) and NOT 1 is -2 | RAI's guesses about the "can't write" bug on the DeepSeek evening; found 2026-10-22 (K4b) | **(b)** a language-service warning, `FILEEXISTS gives 1, and NOT 1 is -2 (true): write FILEEXISTS(x) = 0`, with a quick fix (Ctrl+.) that writes it (in brackets inside a larger expression). Every builtin whose registry doc says "True (1)" (FILEEXISTS, DIREXISTS). **(c)** card rule `one-is-yes` | `rapidr_langsvc::hints::not_one`, `not_one_fix`. langsvc test `not_fileexists_is_always_true_and_fixed`; card test `every_one_for_yes_builtin_is_on_the_card` |
| 6 | `IF Check.Checked = True THEN` — never true in a `.rr` program: `True` is -1 there, `Checked` is 1 | The constants built into `.rr` (2026-10-22); gemma, Haiku | **(c)** card rule `one-is-yes` ("never `x = True`, never `NOT x`"); the existing friendly hint on `TRUE`/`FALSE` | card |
| 7 | `DIM x AS BOOLEAN` | gemma E4B (docs/small-models-plan.md), DeepSeek | **(c)** card rule `no-boolean` and the type list; **(b)** the existing friendly hint under `Unknown data type BOOLEAN` | card; friendly.rs |
| 8 | A file cut off mid-word (`END FUNCTI`), whole-file rewrites for a two-line fix | DeepSeek evening | Studio refuses the change and says why (R11, done 2026-10-22) — not a language matter | crates/rapidr-studio/src/ai_tools.rs `cut_off`, `needless_rewrite`; flows `ai-r11-*` |
| 9 | Wrong reference entries teach wrong calls: `RDataFrame.Iloc(start, end)`, `Select` takes one comma-separated string, `Cell(row, col)` | Database Studio (2026-10-21), K4b | The registry fixed and proven by `rapidr lang truth` (K7): every syntax line and example compiled, the `expect` ones run | crates/rapidr-lang/src/truth.rs; `regress.sh unit` |
| 10 | Other languages' habits: `!=`, `&&`, `[ … ]` lists, `print(…)`, `AS T AS T`, `OnChange = ()` | gemma E4B, MiMo 9B (2026-10-18) | **(b)** check_code's friendly hints (existing); **(c)** card rules `operators`, `arrays`, and the event SUBs list (each event's usual parameters, from the registry). `==` and `+=` compile in RapidR: no rule needed | friendly.rs; card |
| 11 | A window never shown (`Form.ShowModal` missing) | gemma E4B | **(c)** card rule `create`; **(b)** the existing friendly program check | card; friendly.rs |
| 12 | `REDIM a(20)` without its type turns a STRING array into numbers (its items are lost) | Found while writing the card (2026-10-22): `DIM a(2) AS STRING … REDIM a(5)` prints `0` for `a(1)` on the VM | **(c)** the card writes `REDIM a(20) AS STRING`. What RC.EXE does here is not yet checked: an open question for RapidQ ground truth, not changed | card rule `arrays` |

## Measured (live, 2026-10-22)

`claude-haiku-5-5` through `crates/rapidr-studio/examples/ai_eval.rs`,
before the card (development at 58a23ba5) and after it, the same scripts:

| Script | Before | After |
|---|---|---|
| `deepseek-evening.txt` (5 turns, 3 files): the program compiles; the 1-line fix is a targeted edit; the 4th answer Spanish, the 5th English | all ✓ | all ✓ |
| `apps/stopwatch.txt` | ✓ | ✓ |
| `apps/drawing-pad.txt` | ✓ | ✓ |
| `apps/web-server-manager.txt` | ✓ | ✓ |
| **Pass** | **4 / 4** | **4 / 4** |
| Tokens sent / received (all four) | 185,063 / 11,353 | 239,240 / 11,772 |

Haiku already passed these, so they can't show what the card adds; what
they show is its cost — about 2,400 more input tokens a request (most of it
cached by the provider: it is the same every turn). The card is for the
models that wrote the mistakes above (DeepSeek's flash model, the small
local ones); measuring them is K6. Only three of the ten apps were run, to
keep the live spend small.

## How a new mistake gets in

1. Note the mistake, a real example (a transcript, an eval run) and the
   model.
2. Decide: an extension only when it is safe and unambiguous and can't
   change a RapidQ program; a message when the compiler already stops
   there; a card line otherwise (the card has a 2,000-token budget, held by
   a test).
3. Add the guard: a conformance case for an extension or a message (both
   backends and the web), a `[[rule]]` with a `right` program (and its
   `expect`) for a card line, a langsvc test for a hint.
