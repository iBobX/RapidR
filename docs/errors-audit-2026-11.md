# The compiler's messages: what each lacks (audit, 2026-11)

The ERRORS lane's round 1 (2026-11-05) fixed the silent failures of that
week. This page lists every other compile-time message of the front end —
preprocessor, lexer, parser, the AST checks, the bytecode compiler and the
language service's notes — that has **no fix hint** or **no
file:line:column**, ranked by what it costs a student, a teacher or an AI
agent (a mistake many make, with no way forward in the message, first).
Run-time errors are out of scope here (round 2).

How the inventory was made: every place that makes a diagnostic was read
(`error_at`, `missing_closer`, `describe_failure`, the AST checks' `(span,
message)` lists, `PreprocessError::new`, the lexer's errors, the language
service's notes), and each message counted as having a hint only when its
own text says the fix, or `rapidr_langsvc::foreign::for_error` gives one
for it (the habit of the line, `habits()`, doesn't count: it can say
something unrelated — see "Hints that don't fit" below).

Totals: about 105 message templates. 4 have no place, 58 no hint (13 more
only for some inputs), 3 neither; 54 have a catalog ID
(`crates/rapidr-diagnostics/src/messages.rs`), 51 none — and a message
without an ID is never translated.

## Fixed in round 1

- A block left open — SUB, FUNCTION, IF, FOR, WHILE, DO, SELECT, CREATE,
  TYPE, WITH — is an error at its opener naming the block, its file:line and
  the fix (RQ0018–RQ0020, RR0035–RR0038). A bare END where END SUB was
  meant is said so; a SUB that starts inside another; in a `.rr` program a
  block never runs on into the file that `$INCLUDE`d it.
- A global named like a builtin (`CREATE Timer AS RTimer`) is an error
  (RR0039–RR0043; RC.EXE's own words, RQ0007/RQ0021/RQ0022, in classic
  programs).
- A file that isn't UTF-8 (a `.rr`), or mixes UTF-8 with Windows-1252
  bytes (any file), is a warning at the first such byte (RR0044, RR0045).
- MCP `run_program` / `read_output` say "The program started and ended
  after 0.2 s (exit code 0), with no output" and what to try, not "The
  program isn't running".
- The duplicate global's hint: `Public Const` had "a routine is SUB Name(…)"
  under it; it has its own fix now ("hint.duplicate"), and its explanation
  is translated (RR0046).
- `rapidr check` read a Windows-1252 main file as an I/O error ("stream did
  not contain valid UTF-8"); it reads it as the compiler does.
- A template value starting with a character of two bytes or more (`ñ`)
  never matched, so such messages weren't translated
  (`rapidr_i18n::template::match_from`).

## Ranked: what to fix next

1. **`Syntax error in {WORD} statement`** (RR0002) — the most common
   message for a beginner's line, and only SET, FOR, END, CASE, ELSE,
   ELSEIF, IF and WHILE get a hint. DIM, PRINT, SELECT, DO, NEXT, SUB and
   the rest say nothing of what the statement looks like. Fix: one example
   of the statement per keyword, from the registry's syntax lines.
2. **`Syntax error near '{text}'`** (RR0003) and **`Syntax error: unexpected
   '{text}'`** (RR0004) — hints only for a handful of characters. Fix: say
   what was expected at that point (the parser knows: an operator, a `)`,
   `THEN`).
3. **`Member {PATH} not part of class {OBJECT}`** (RQ0001, parser and AST)
   — no "did you mean", though Studio has a quick fix that knows the
   nearest member (`compat.rs`): put the same nearest member and the list's
   size in the message ("— did you mean Caption? RButton has 52 members:
   rapidr_describe RButton").
4. **`Label '{label}' not found in {routine}`** (RR0007) — no did-you-mean
   (the labels of the routine are known), and no word that a label belongs
   to one routine.
5. **`Too many actual parameters for {name}`** (RQ0015) / **`Too few
   parameters for {name}`** (no ID) — neither shows the routine's
   signature or where it is declared. Fix: `— Pair takes (a AS INTEGER, b
   AS INTEGER), declared in util.rr:12` (a place mark, as RQ0007 does).
6. **`Unknown type {T}`** (RQ0008) / **`Unknown data type {T}`** (no ID) —
   the fix (BOOLEAN → INTEGER, a component's real name) exists in
   `foreign::for_message`, but only the AI tools use it: Problems and
   `rapidr check` should show it as the hint.
7. **`Type mismatch, expecting type {T}, but got STRING`** (no ID) — same:
   its hint exists for the AI tools only.
8. **`Undeclared identifier {name}`** (RQ0004) / **`Undefined symbol
   {NAME}`** (no ID) — with `$TYPECHECK ON`; no did-you-mean among the
   declared names, no "DIM it first".
9. **`Expected end-of-line but got {got}`** (RQ0005) — hints for `;`, `?`,
   `**`, `//` only; `L.Sorted = 1, 2` and `L.Clear = 1` (AST) say nothing
   of the property's or the method's form.
10. **`EXIT {kind} is not inside a {kind} loop`** (RR0009, RR0010) — no
    word of which blocks are open there.
11. **`Label '{l}' is defined more than once`** (RR0008) — no place of the
    first one (a place mark would give it).
12. **`{TYPE}` checks: `Create Method only works for QObjects, not {T}`,
    `{T} is not a valid data type for your FUNCTION`, `You can only extend
    QObjects`, `Datatype {T} not supported in STRUCT`, `Array of {Q} is not
    supported!`, `Component assignment is not yet supported.`** — RC.EXE's
    words with nothing after them; each wants one line of what to write
    instead.
13. **`{OBJ}.{MEMBER} is a read-only value.`** (RQ0002) / **`Property
    {prop} of {obj} is read-only.`** (RQ0003) — no word of how the value is
    set (often a method: `Items.Clear`).
14. **`Trying to assign return value while not in FUNCTION`** (no ID) — no
    word that a SUB has no result.
15. **Lowercase internal texts that reach a user** — `nested member-access
    store not yet supported`, `invalid assignment target`, `unsupported
    CALL target`, `unsupported function call target`, `unsupported method
    call object` (bcgen): no ID, no hint, a compiler's words. Each needs the
    statement as the user wrote it and what works instead.
16. **`Array initializers need constant bounds on every dimension but the
    first`**, **`Array field {field} in TYPE {type}: the dimensions aren't
    valid`**, **`Only PROPERTY SET blocks are supported inside a TYPE`**,
    **`Unexpected '{tok}' after field declaration`** (RR0015) — no example
    of the right form.
17. **Lexer: `Unterminated string literal`, `Invalid prefixed number
    literal`, `Unexpected character: {c}`** — no ID; the first only when the
    string runs to the end of the file (a line end closes one silently).
18. **Preprocessor: `Recursive include detected: '{f}'`, `Failed to include
    '{f}': {io}`, `Invalid $INCLUDE syntax`, `Invalid $RESOURCE syntax`,
    `Include file outside the workspace`** — no ID, no fix (the include
    chain for the first; the form of the directive for the others).
19. **No place: `Failed to read source file '{path}': {io}` (1:1),
    `$RESOURCE {name}: file not found` and the CLI's `$RESOURCE {name}:
    {file}: {io}`** — they don't name the `$RESOURCE` line. The resource
    knows its directive's line (`Resource::icon_directive` holds the
    `$OPTION ICON` one): keep the `$RESOURCE`'s too.
20. **`{file}:{line}:1: ICON file {x} does not exist.`** (preprocessor) —
    the place is written inside the text, so the catalog never matches it
    and it is never translated.
21. **The language service's notes** — RR0019–RR0026, RR0029 and the
    compatibility warnings have no fix (what to use on the other runtime;
    the classic way to write it).

## Hints that don't fit

`foreign::for_error` falls back to the habits of the source line for any
error that has no hint of its own. Round 1 stopped that for messages that
say their fix (a name declared twice, a builtin's name, a block left open),
and the routine habit now needs another language's routine (`public void`,
`private static`) — `Public Const` and `Private Sub` are VB's, which RapidR
and RC.EXE take. Still to check:

- `habit_above` for `Syntax error in END statement`: looks 40 lines up for
  any habit (`function`, `colon`, `braces`) — far enough to pick one of an
  unrelated routine.
- The fallback itself, for the remaining messages: a habit of the line
  under, say, `Member X not part of class Y` is rarely the fix. A positive
  list of the messages it may follow (the syntax errors) would be safer.

## Gaps left on purpose in round 1

- A SUB's own variables and parameters named like a builtin (`SUB S(Len AS
  INTEGER)` then `LEN(s)` inside: the parameter wins, LEN gives nothing).
  RC.EXE takes them and lets the builtin win; RapidR lets the local win.
  An error when the SUB also calls the builtin through the name would catch
  it.
- A classic program's `DIM Log AS STRING`: RC.EXE refuses it, RapidR keeps
  taking it (nothing reads LOG through it). Only names the program reads a
  builtin through are errors.
- `Left` hides `LEFT$` (RapidR reads a name with or without its `$` as one,
  RapidQ doesn't): `DIM Left AS INTEGER` then `LEFT$("abc", 2)` ran into
  "indexing a value that is not an array" in a classic program RC.EXE
  builds. In a `.rr` program round 1 makes it an error (the program reads
  LEFT$ through `Left`); a classic program still runs into it, since
  RC.EXE takes the name: the name resolution itself (bcgen) should tell
  `Left` from `LEFT$` there.
- `Timer.Enabled = 1` after the component was renamed names the builtin
  TIMER: a member of a builtin function should be RC.EXE's `Member ENABLED
  not part of class TIMER`.
- A classic CREATE's body takes any statement (`PRINT` inside a CREATE):
  RC.EXE says `Member PRINT not part of class T`.
- A FUNCTION called before it is declared, without a DECLARE: RC.EXE says
  `F identifier already in use, try another name`; RapidR takes it.
- Interleaved blocks (an `END IF` inside the FOR of that IF's body, as
  RapidQ's mysql/rqlibsql/mySQL_API.bas writes it): RC.EXE's flat block
  stack takes them; RapidR's parser reports the `END IF` as a stray.

## For the lead: the codes that want a manual page

RQ0018 `Missing {closer} statement`, RQ0019 `Missing END SUB or END
FUNCTION`, RQ0020 `Missing END STRUCT before end of file.`, RQ0021 `{name}
identifier already in use, try another name`, RQ0022 `CONST variable
{name} already defined`, RR0035–RR0038 (a block left open), RR0039–RR0043
(a builtin's name), RR0044–RR0045 (a file that isn't UTF-8), RR0046 (a
global declared twice).
