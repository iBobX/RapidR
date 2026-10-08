# RapidR's component names, and RapidQ's

RapidR is compatible with RapidQ: it runs RapidQ programs as they are, and adds its own components and extensions. Its components have **RapidR's names** — `RForm`, `RButton`, `RLabel`, `RStringGrid`, `RPlot` — and the ones RapidQ has too are also known by **RapidQ's names** (`QFORM`, `QBUTTON` …), so RapidQ programs keep working. This page states the rules the language follows and how the tools present and write names.

Decided 2026-10-08 (R-NAMES): RapidR's names are the default everywhere — the docs, the examples, the templates, completion and hovers, the manual's tables — and RapidQ's names appear as a note ("RapidQ name: QBUTTON") or where compatibility is the point. Being visibly an original system, compatible with RapidQ and not a copy of it, is one of the reasons.

Status: §1–§3 and §6 describe RapidR today (v2.118); §4's RapidR Studio rules are phase 2 of R-NAMES (docs/ide-plan.md, "R-NAMES").

---

## 1. One component, two names

- Each component has RapidR's name, written in mixed case: `R` + the component's name as RapidQ's documentation spells its classes (`RCheckBox`, `RStringGrid`, `RDXScreen`; the registry's `display`, `Component::spelling`). Names are not case-sensitive: `RBUTTON`, `rbutton` and `RButton` are the same.
- The components RapidQ has are also known by RapidQ's names: `QBUTTON` is `RButton`, `QFORM` is `RForm`. A few RapidQ names map elsewhere — `QGAUGE` is `RProgressBar`, `QOUTLINE` (Windows 3.1's tree) is `RTreeView`, `COMPORT` (RAPIDQ2.INC's `$DEFINE QCOMPORT COMPORT`) is `RComPort` — and RapidQ's empty base object `QOBJECT` (`TYPE T EXTENDS QOBJECT`) is `RObject`.
- **The compilers always accept both names**, silently, with no flag or setting: `DIM`, `CREATE`, `EXTENDS`, parameters (`Sender AS QBUTTON`), TYPE fields, results and arrays of components take either, and a program may mix them. Both are the same component: same properties, methods, events, defaults, looks and behaviour on every runtime (`rapidr_ast::canonical_type_name` reads both as the compilers' one name, `RBUTTON`).
- The components RapidQ's include libraries define as TYPEs (QBEVEL, QDIGDISPLAY) are the program's own TYPE when it includes the library, RapidR's built-in (`RBevel`, `RDigDisplay`) otherwise (`component_type_reference`). RapidR's BASIC implementations of RapidQ community libraries keep their names (`QDockForm`, `QDirListView`).
- A quirk kept for compatibility: a Q-prefixed name RapidQ doesn't have is read as RapidR's component of the same name (`QPLOT` is `RPlot`). RapidR keeps accepting it; tools never write it, and a RapidQ-compatible project gets a warning for it (§5).

## 2. Two families

- **Components RapidQ has too** (RapidQ's manual's list, checked against RC.EXE): `RForm`, `RButton`, `REdit`, `RStringGrid`, `RListView`, `RTimer`, `RMySQL`, `RCanvas` … Each has a RapidQ name.
- **RapidR's own components**: `RPlot`, `RJson`, `RDataFrame`, `RNum`, `RSQLite`, `RCodeEditor`, `RDesignSurface`, the web components (`RWebView`, `RDom` …) and RapidR Studio's components. They have no RapidQ name.
- The language registry (`crates/rapidr-lang`) records for each component its RapidQ name or none, and for every member, builtin, statement and directive whether it is RapidQ's or RapidR's. Every rule here comes from that record.

## 3. RapidR's extras on RapidQ's components

RapidR adds members to the components RapidQ has — `Anchors` and `Constraints` (responsive layout), `AccessibleName` and `AccessibleDescription` (screen readers), `Scale` and `OnScaleChanged` (high-DPI), autofill hints on edits for the web …

- They are **additive**: a program that doesn't use them behaves exactly as under RapidQ; their defaults are RapidQ's behaviour.
- They work under **either name**: `QBUTTON.Anchors` is `RButton.Anchors`.
- They make a program **RapidR-dependent** (RapidQ's compiler refuses them): the tools mark them, and a RapidQ-compatible project warns about each use (§5). `$IFDEF RAPIDR` guards them for programs that also compile with RC.EXE (decision D12).

## 4. In the tools

- **Docs, examples, templates**: RapidR's names, with "RapidQ name: QBUTTON" where it helps (the manual's component tables have a *RapidQ name* column). `examples/rapidq/` keeps RapidQ's own style on purpose, to show that RapidQ programs run unchanged.
- **Language service** (VS Code, RapidR Studio): completion after `AS` lists RapidR's names with "RapidQ name: QBUTTON" as the detail — RapidQ's names in a RapidQ-compatible project, with "RapidR name: RButton"; hovers show "RButton — RapidQ name: QBUTTON"; members are the same list under both names, RapidR's extensions marked.
- **RapidR Studio** (R-NAMES phase 2): the toolbox and the inspector show RapidR's names; the designer and completion follow each file's own style (`rapidr_import::NameStyle`): a file written with RapidQ's names gets RapidQ's names for new components, any other file RapidR's — a file never gets both; a name already in the source is never changed. File > "Import RapidQ Project or File…" and "Upgrade this file to RapidR names" run the importer (§6).
- **New-project templates**: RapidR's names (`CREATE Form1 AS RForm`); a "RapidQ program" template, when Studio has it, writes RapidQ's names with the compatibility setting on.

## 5. The "RapidQ-compatible project" setting

For people who also compile with RapidQ's own compiler (RC.EXE). Off by default; `[compat] rapidq_compatible = true` in the project file. It **never changes how RapidR compiles or runs a program** — RapidR always accepts both names — it only adds warnings (or errors with `level = "error"`) for everything RC.EXE would refuse:

| Diagnostic | Example | Fix |
|---|---|---|
| RapidR's own component | `CREATE Chart1 AS RPlot` — "RPlot is RapidR's own component: RapidQ doesn't have it" | Guard with `$IFDEF RAPIDR` |
| RapidR's name of a RapidQ component | `DIM b AS RButton` — "RButton is RapidR's name: RapidQ's compiler knows it as QBUTTON" | Write QBUTTON |
| A Q name RapidQ doesn't have | `AS QPLOT` — "RapidQ has no QPLOT: RapidR reads it as RPlot" | Write RPlot |
| RapidR's extension of a RapidQ component | `Button1.Anchors = …` — "QButton.Anchors is a RapidR extension" | Guard; remove |
| RapidR's builtin, statement, directive, type or constant | `$THEME modern`, `BEEP`, `akLeft` | Guard; remove |

## 6. Converting a program: `rapidr import-rapidq` and `rapidr upgrade-names`

- `rapidr import-rapidq <file | folder | project.rrproj> [-o out_dir] [--include DIR]` writes a **copy** with RapidR's names and a report (`rapidr-import-report.md`); the original is never touched. Sources may be `.bas`, `.rqw` (RapidQ's window programs), `.rqb` (libraries of functions), `.rq` (libraries of TYPEs), `.inc` or `.rr`; each keeps its extension. RapidQ IDE templates (`.tpl`) aren't copied.
- `rapidr upgrade-names <file> [--dry-run]` does the same to one of your own files in place (`--dry-run`: the diff only).
- **What changes**: exactly the type names the compiler's parser reads (after `AS` in `DIM` / `CREATE` / parameters / TYPE fields / FUNCTION results, after `EXTENDS`) that mean one of RapidR's components. Never strings, comments, the program's own names (`QButtonCount`), its own TYPEs, a name a `$DEFINE` makes, or code in an `$IFDEF` branch the build doesn't compile — the report lists those.
- **Includes** are followed into the copy (the program's own and RapidQ's include folder's). `$INCLUDE "RAPIDQ.INC"` stays: RapidR supplies RAPIDQ.INC's constants through that line (no file needed), as RapidQ gives them through it; without it, `clRed` would be an undeclared name. RapidQ's RAPIDQ.INC file itself isn't copied, unless the program uses something of it RapidR's constants don't have (RapidQ's `QBColor` array).
- **Proved**: every program the original compiles to is compared with what its copy compiles to, byte for byte, and the report says so for each. On RapidQ's own 428 example programs (`.bas`, `.rqw`, `.rqb`, `.rq`): every one that RapidR compiles today (175) compiles to identical bytecode after the conversion.
- The engine is `crates/rapidr-import`; RapidR Studio uses the same.

## 7. Summary

1. A RapidQ name and RapidR's name are one component; RapidR always accepts both, mixed, in any case.
2. RapidR writes and shows RapidR's names (`RButton`); RapidQ's names are notes, or a RapidQ program's own style.
3. Old programs behave exactly as before; nothing is renamed in a file unless you ask (`import-rapidq` makes a copy; `upgrade-names` changes one file).
4. RapidR's extras are additive, work under both names, and make a program RapidR-dependent.
5. A RapidQ-compatible project lists every RapidR dependency (RapidR's names of RapidQ's components included) and changes nothing else.
