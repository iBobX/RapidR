# RapidQ import: greeter.rqw

A copy of `greeter/greeter.rqw` in `greeter/greeter-rapidr`, with RapidR's component names (`QBUTTON` → `RButton`). The original is unchanged. RapidR runs both alike: it reads RapidQ's names and RapidR's as the same components.

**Summary:** 1 program(s), 2 source file(s) (2 changed, 7 name(s)), 0 other file(s) copied; compiled as the original: 1 identical, 0 different, 0 copy failed, 0 original doesn't compile in RapidR today.

Source files: 1 `.inc`, 1 `.rqw` (RapidQ's `.rqw`, `.rqb` and `.rq` programs and `.inc` includes keep their extensions).

## Programs

| Program | Compiled as the original |
|---|---|
| `greeter.rqw` | yes: identical bytecode |

## RAPIDQ.INC

`$INCLUDE "RAPIDQ.INC"` is kept: RapidR supplies RAPIDQ.INC's constants (`clRed`, `MB_OK` …) through that line without needing the file, as RapidQ gives them through it. Without the line those names would be undeclared (1 program(s) include it).

## Changes

### `greeter.rqw`

- line 9, column 33: `QBUTTON` → `RButton`
- line 11, column 16: `QFORM` → `RForm`
- line 16, column 24: `QEDIT` → `REdit`
- line 20, column 24: `QBUTTON` → `RButton`
- line 25, column 22: `QLABEL` → `RLabel`
- line 31, column 25: `QBUTTON` → `RButton`

### `include/shapes.inc`

- line 2, column 18: `QFONT` → `RFont`

## Not converted

Nothing.
