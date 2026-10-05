# RapidQ's Q components and RapidR's R components: how they coexist

RapidR runs RapidQ programs as they are and adds its own components and extensions. A program — even a single form — can mix both freely: a QFORM holding a QBUTTON, a QSTRINGGRID, an RPLOT and an RDATAFRAME is ordinary. This page states the rules the language already follows, and how the IDE ([docs/ide-plan.md](ide-plan.md)) presents and writes them, including the optional "RapidQ-compatible project" setting for people who also compile with RapidQ's own compiler (RC.EXE).

Status: the language rules (§1–§2) describe today's behaviour (v2.116.0); the IDE rules (§3–§5) are planned with stages I0, I1, I3 and I4.

---

## 1. One component, two names

- RapidQ calls its components QFORM, QBUTTON, QSTRINGGRID …; RapidR calls the same components RFORM, RBUTTON, RSTRINGGRID …. **Both names are the same component**: same properties, methods, events, defaults, looks and behaviour, on every runtime.
- The mapping is `rapidr_ast::canonical_type_name`: a name `Q` + rest becomes `R` + rest when RapidR has that component; a few RapidQ names map elsewhere — QGAUGE → RPROGRESSBAR, QOUTLINE (Windows 3.1's tree) → RTREEVIEW, `COMPORT` (RAPIDQ2.INC's `$DEFINE QCOMPORT COMPORT`) → RCOMPORT — and the components RapidQ's include libraries define as TYPEs (QBEVEL, QDIGDISPLAY) are the program's own TYPE when it includes the library, RapidR's built-in otherwise (`component_type_reference`).
- Mixing is free at every level: a QFORM's child can be an RPLOT, an RFORM's child a QBUTTON; `DIM`, `CREATE`, `EXTENDS`, parameters (`Sender AS QBUTTON`), arrays of components all accept either name.
- **What a program reads never depends on the name it used** beyond what it read before: the IDE never renames anything a program wrote, so a program behaves exactly as it did.

## 2. Two families

- **RapidQ components**: the components RapidQ has (its manual's list, checked against RC.EXE). They have a Q name and an R name. Examples: QFORM, QBUTTON, QEDIT, QSTRINGGRID, QLISTVIEW, QTIMER, QMYSQL, QCANVAS.
- **RapidR-only components**: the ones RapidQ doesn't have. They have only an R name. Examples: RPLOT, RJSON, RDATAFRAME, RNUM, RSQLITE (RapidQ has QMYSQL, not SQLite), RCODEEDITOR, RDESIGNSURFACE, the web components (RWEBVIEW, RDOM …), and the planned IDE components and data components (RDBCONNECTION, RDATAFILE, RDFGROUP, RDBGRID …).
- **A quirk to know**: because the mapping replaces any leading Q with R when the R component exists, `DIM p AS QPLOT` is accepted today and means RPLOT, though RapidQ has no QPLOT. RapidR keeps accepting it (no behaviour change), the IDE never writes it, and a RapidQ-compatible project gets a warning for it (§5; decision D13 in the IDE plan).
- The language registry (IDE plan, stage I0) records for every component its RapidQ name or none, and for every property, method, event, builtin, statement and directive whether it is RapidQ's or RapidR's. That record is the source of every rule below.

## 3. RapidR's extra members on RapidQ components

RapidR adds members to RapidQ components — for example `Anchors` and `Constraints` (responsive layout), `AccessibleName` and `AccessibleDescription` (screen readers), `Scale` and `OnScaleChanged` (high-DPI), autofill hints on edits for the web — and data binding properties on some (e.g. RPlot's, which is R-only anyway).

- They are **additive**: a program that doesn't set them behaves exactly as under RapidQ (rule: RapidQ-exact; extensions never change old behaviour). Their defaults are RapidQ's behaviour (no anchors beyond RapidQ's `Align`, no accessible name beyond the caption …).
- They work **under either name**: `QBUTTON.Anchors` and `RBUTTON.Anchors` are the same property. Using one doesn't change the component's name or family.
- **But they make the program RapidR-dependent**: RC.EXE doesn't know them and will refuse the program. The IDE marks them (§4) and, in a RapidQ-compatible project, warns about each use (§5).
- Where a program needs both — RapidR's extras when run on RapidR, still compiling with RC.EXE — the extras can be guarded by a preprocessor block RC.EXE skips. This needs RapidR to predefine a symbol (`$IFDEF RAPIDR`), which it doesn't yet: decision D12 in the IDE plan.

## 4. In the IDE

- **Toolbox**: two groups, **"RapidQ"** and **"RapidR"**, each with its usual categories. RapidQ components are listed under their Q names (QBUTTON), RapidR-only ones under R names (RPLOT) — the names the designer will write.
- **Designer** (what it writes):
  - a **new RapidQ component** is written with its **Q name** (`CREATE Button1 AS QBUTTON`), so a program that uses only RapidQ components stays a plain RapidQ program;
  - a **new RapidR-only component** is written with its **R name** (`CREATE Chart1 AS RPLOT`);
  - a component **already in the source keeps the name it was written with** (an existing `AS RBUTTON` stays RBUTTON; the designer never renames types);
  - a new form is a QFORM; property values use RapidQ's spellings (colours as `&H` BGR numbers or RapidQ's constants);
  - a RapidR-only property set from the inspector is written like any other property (it is one), and is marked in the inspector.
- **Property inspector**: Q and R components are treated the same (one model). RapidR-only members of a RapidQ component appear under a **"RapidR extensions"** category with a small badge, so it's visible at a glance what goes beyond RapidQ.
- **IntelliSense**: completion after `AS` lists RapidQ components with Q names and RapidR-only ones with R names (typing `AS R` lists every R name, for people who prefer them); member completion is the same for both names, RapidR-only members marked; hover shows both names ("QBUTTON — RapidR: RBUTTON") and the origin of each member.
- **New-project templates**: "RapidQ program" (`.bas`, Q names, compat setting on) and "RapidR program" (`.rr`, both families, compat setting off).

## 5. The "RapidQ-compatible project" setting

For users who also need RC.EXE compatibility. Off by default; on in the "RapidQ program" template; `[compat] rapidq_compatible = true` in the project file (and a status-bar indicator). When on, the language service reports, as warnings (or errors with `level = "error"`), every place the program depends on RapidR:

| Diagnostic | Example | Code actions |
|---|---|---|
| RapidR-only component | `CREATE Chart1 AS RPLOT` — "RPLOT is a RapidR component; RapidQ has no equivalent" | Guard with `$IFDEF RAPIDR` (if D12 is accepted); show alternatives when one exists (e.g. QCANVAS drawing for a simple chart) |
| A Q name RapidQ doesn't have | `AS QPLOT` — "RapidQ has no QPLOT; RapidR reads it as RPLOT" | Rename to RPLOT |
| RapidR-only member of a RapidQ component | `Button1.Anchors = akLeft OR akRight` — "Anchors is a RapidR extension of QBUTTON" | Guard; remove |
| RapidR-only builtin, statement or directive | `$THEME modern`, a RapidR builtin, an R-only constant (`akLeft`) | Guard; remove |
| RapidR-only syntax | constructs RapidR accepts that RC.EXE doesn't (the registry lists them) | Rewrite where an equivalent exists |
| RapidR-only include library | QDIRLISTVIEW / QDOCKFORM are RapidR's own implementations of RapidQ community libraries — fine for RC.EXE only with the original include files | Note only |

- The **Problems** panel can filter to "RapidR dependencies", giving the exact list of places a program relies on RapidR.
- The setting **never changes how RapidR compiles or runs the program** — only diagnostics.
- Optional ground truth: on Windows (or a Windows VM), a user who has RapidQ can point the IDE at their RC.EXE to compile the project with it as an extra check (RapidR never ships RC.EXE).

## 6. Summary of the rules

1. A Q name and its R name are one component; mix families freely.
2. Old programs behave exactly as before, under any name; nothing is renamed for you.
3. RapidR's extras are additive, work under both names, and make a program RapidR-dependent.
4. The designer writes Q names for RapidQ components and R names for RapidR-only ones; it keeps existing names.
5. A RapidQ-compatible project lists every RapidR dependency, and changes nothing else.
