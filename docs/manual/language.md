# The language

RapidR's language is a RapidQ-compatible BASIC. This chapter is a working summary,
with the rules where RapidQ (and so RapidR) differs from QBasic, VB or other
BASICs — those are the ones that surprise people. Every example here was
run with RapidR 2.117.0; native builds, the interpreter and the web give the
same output.

RapidR also accepts some syntax RapidQ doesn't (VB's `#If`, `ON ERROR`,
`i++`, …): see [Differences and extensions](differences.md).

## Source files

- Extensions `.bas` and `.rr` are the same language. The other extensions
  RapidQ's editors saved programs with are accepted too: `.rqw`, `.rqb` and
  `.rq` (run, built, opened in RapidR Studio and `$INCLUDE`d like a `.bas`).
  A file is read as UTF-8; one that isn't valid UTF-8 is read as
  Windows-1252 (ANSI), as RapidQ's programs usually are.
- **Case doesn't matter** in keywords, names, components and builtins.
- `:` separates statements on a line; `_` at the end of a line continues it
  (a string may continue too).
- Comments: `'` or `REM`.
- `?` is short for `PRINT`.
- Labels: `Name:` on a line of their own, or a line number.
- A first line `#!/usr/bin/env rapidr` makes the file a script on macOS and
  Linux (`chmod +x prog.bas; ./prog.bas`); the line is ignored.

## Program kinds: `$APPTYPE`

`$APPTYPE CONSOLE`, `$APPTYPE GUI`, `$APPTYPE CGI` (RapidQ's) and `$APPTYPE
WEB` (RapidR's). Without one, a program that creates components is a GUI
program and any other a console program — as RapidQ decides. The kind
decides, for example, whether a Windows executable opens a console window,
and whether a program double-clicked on macOS or Linux gets a terminal.

## Variables and types

| Type | Suffix | Holds |
|---|---|---|
| `BYTE` | `?` | 0 … 255 (a store wraps: `b? = 300` holds 44) |
| `WORD` | `??` | 0 … 65535 |
| `SHORT` | `%` | -32768 … 32767 (`q% = 40000` holds -25536) |
| `INTEGER`, `LONG` | `&` | 32-bit signed |
| `DWORD` | `???` | 32-bit (signed, as RapidQ has it) |
| `SINGLE` | `!` | 32-bit float |
| `DOUBLE` | `#` | 64-bit float |
| `STRING` | `$` | text; `STRING * n` is always n characters |
| `VARIANT` | | whatever is stored |

```basic
DIM count AS INTEGER, title AS STRING
DIM x AS INTEGER = 5
DIM code AS STRING * 3        ' "abcdef" stored gives "abc"; "a" gives "a  "
DIM (a, b, c)(5) AS LONG      ' three arrays
DIM m                         ' no AS: a DOUBLE
DIM p, q AS LONG              ' only q is a LONG; p is a VARIANT
CONST MaxItems = 3
DEFINT k(1 TO 3) = {7, 8, 9}  ' DEFINT, DEFSTR, DEFLNG, DEFDBL, …
```

- Variables needn't be declared; an undeclared name is a VARIANT. A name
  with a suffix (`n%`, `s$`) has the suffix's type.
- **A store into an integer variable truncates** (`i = 2.7` gives 2);
  beyond 32 bits it gives -2147483648. A `BYVAL` parameter rounds half to
  even instead, as RapidQ does.
- Inside a SUB or FUNCTION, `DIM` makes a local; `STATIC` keeps its value
  between calls. An undeclared variable a SUB uses before the main program
  does is the SUB's own (kept between calls) — RapidQ's one-pass rule.
- `TRUE` and `FALSE`: a comparison gives -1 or 0 (`PRINT 1 = 1` prints
  -1). With `$INCLUDE "RAPIDQ.INC"`, `True` is 1, as in RapidQ's file.

## Arrays

```basic
DIM a(1 TO 3) AS INTEGER      ' bounds 1 … 3
DIM grid(2, 2) AS STRING      ' 0 … 2 in each dimension
REDIM a(1 TO 5) AS INTEGER    ' resized, its values kept
PRINT LBOUND(a), UBOUND(a)
SWAP a(1), a(3)
QUICKSORT a(1), a(5), 0       ' RapidQ's sort of a(1) … a(5); 0 ascending
```

Whole arrays can be passed (`Fill M()`, `SUB Fill (list() AS STRING)`), and
TYPEs can hold arrays (`Names(2) AS STRING`, `vertex(9, 2) AS SINGLE`).

## Operators

From the loosest: `OR`, `XOR` · `AND` · `NOT` · comparisons (`=`, `<>`,
`<`, `>`, `<=`, `>=`, `NOT=`) · `+`, `-`, `&` · `*`, `/`, `\`, `MOD`,
`SHL`, `SHR`, `INV` · `^` · unary `-`.

- `AND`, `OR`, `XOR`, `NOT` are **bitwise** on 32-bit integers (`7 AND 3`
  is 3, `NOT 0` is -1). `NOT` binds looser than comparisons.
- `/` divides as reals (`7 / 2` is 3.5); `\` is integer division, its
  operands rounded first; `MOD`'s operands round half to even (`7.5 MOD 2`
  is 0).
- `\` and `MOD` by zero stop the program with "Division by zero"; `/` by
  zero gives an infinity.
- `a INV m` is the modular inverse (`3 INV 26` is 9).
- Strings: `+` or `&` join; `-` removes every occurrence (`"jello" - "l"`
  is `"jeo"`); `s$[i]` is the i-th character.
- `i++`, `i--`, `x += y` (and `-=`, `*=`, `/=`, `&=`); `INC x [, n]`,
  `DEC x [, n]`.
- `@var` passes a variable by reference.

## Printing numbers: the rules RapidQ has

RapidR prints numbers exactly as RapidQ does (checked against RapidQ's
compiler, RC.EXE):

```basic
PRINT 1 / 3         ' 0.333333333
PRINT 3.5           ' 3.500000000   (fractions: 9 decimals)
PRINT 10 / 5        ' 2             (whole numbers: as integers)
PRINT 1E10          ' -2147483648   (a whole number beyond 32 bits)
PRINT STR$(1 / 3)   ' 0.333333333   (STR$: 9 significant digits)
```

**PRINT's comma is a semicolon** in RapidQ: `PRINT 1, 2` prints `12`, with
no tab zones. Use `;` with explicit spaces, `TAB(n)`, `SPACE$(n)`,
`FORMAT$` or `STRF$` to lay out columns:

```basic
PRINT FORMAT$("%5d|%.2f|%s", 42, 3.14159, "x")    '    42|3.14|x
PRINT STRF$(1234.5678, 2, 8, 2)                    ' 1234.57 (ffFixed)
```

Other numeric rules: `INT` and `FIX` truncate toward zero; `ROUND`, `CINT`
and `CLNG` are `INT(x + 0.5)` (2.5 → 3, -2.5 → -2); `VAL` reads the number
at the start of the text (`VAL("12abc")` is 12); `HEX$` has 8 digits.

## Control flow

```basic
IF x > 10 THEN
    PRINT "large"
ELSEIF x > 5 THEN
    PRINT "medium"
ELSE
    PRINT "small"
END IF
IF ok THEN PRINT "yes" ELSE PRINT "no"

FOR i = 1 TO 10 STEP 2 : PRINT i : NEXT i
WHILE k < 5 : k = k + 1 : WEND
DO
    k = k + 1
LOOP UNTIL k >= 3                 ' also DO WHILE / DO UNTIL / LOOP WHILE

SELECT CASE n
    CASE 1 TO 10
        PRINT "small"
    CASE 11, 12
        PRINT "eleven or twelve"
    CASE IS > 12
        PRINT "big"
    CASE ELSE
        PRINT "?"
END SELECT

GOSUB Shout                       ' … Shout: PRINT "!" : RETURN
GOTO Done
EXIT FOR / EXIT DO / EXIT WHILE / EXIT SUB / EXIT FUNCTION
END
```

`DATA`, `READ` and `RESTORE [label]` work as in other BASICs.

## SUBs and FUNCTIONs

```basic
DECLARE FUNCTION Square (x AS DOUBLE) AS DOUBLE   ' optional, as in RapidQ

SUB Greet (Who AS STRING)
    PRINT "Hello, "; Who
END SUB

FUNCTION Square (x AS DOUBLE) AS DOUBLE
    Square = x * x            ' or: Result = x * x
END FUNCTION

FUNCTION Twice% (n%)          ' a suffix gives the result's type
    Twice% = n% * 2
END FUNCTION

SUB Change (BYREF v AS INTEGER)
    v = 99
END SUB

Greet "RapidR"                ' a SUB is called without parentheses
PRINT Square(1.5)             ' 2.250000000
```

- Parameters are by value unless `BYREF` (or passed as `@var`).
- `SUBI` / `FUNCTIONI` take any number of arguments, read with
  `ParamVal(i)`, `ParamValCount`, `ParamStr$(i)`, `ParamStrCount`.
- Function pointers: `BIND p TO MySub`, `CALLFUNC p, args`, `CODEPTR(MySub)`.
- `DECLARE … LIB "file"` calls a function in a shared library (`.dll`,
  `.so`, `.dylib`) in native builds; the interpreter and the web refuse it
  with a clear message. Windows API calls (`LIB "kernel32"`, `"user32"`,
  …) are compile errors that name RapidR's portable alternative: RapidR
  runs on every platform and doesn't emulate Windows.

## TYPEs and objects

A `TYPE` is a record, and — as in RapidQ — can have methods, a constructor,
events and inheritance:

```basic
TYPE TCounter
    Count AS INTEGER
    Name AS STRING
    SUB Bump (n AS INTEGER)
        This.Count = This.Count + n
    END SUB
    FUNCTION Describe AS STRING
        Describe = This.Name + "=" + STR$(This.Count)
    END FUNCTION
    CONSTRUCTOR
        Name = "counter"
        Count = 100
    END CONSTRUCTOR
END TYPE

TYPE TLoud EXTENDS TCounter
    FUNCTION Shout AS STRING
        Shout = UCASE$(This.Describe)
    END FUNCTION
END TYPE

DIM c AS TCounter
c.Bump 5
PRINT c.Describe              ' counter=105
DIM l AS TLoud
l.Bump 1
PRINT l.Shout                 ' COUNTER=101
```

- `TYPE TMyButton EXTENDS RButton` makes a component of your own, with
  `EVENT OnSomething … END EVENT` blocks.
- `WITH obj … END WITH` and `.Member` inside it.
- Arrays of TYPEs and of components: `DIM lbl(1 TO 3) AS RLabel`.

## Components

```basic
CREATE Form AS RForm
    Caption = "Title"
    CREATE Btn AS RButton
        Caption = "OK"
        OnClick = OkClick
    END CREATE
END CREATE
DIM Font AS RFont             ' a non-visual object
```

See [Components and objects](components.md).

## Strings

The builtins are the ones RapidQ has: `LEFT$`, `MID$`, `RIGHT$`, `INSTR`, `RINSTR`,
`UCASE$`, `LCASE$`, `TRIM$`, `REPLACE$` (overwrites at a position),
`REPLACESUBSTR$` (find and replace), `INSERT$`, `DELETE$`, `REVERSE$`,
`FIELD$`, `TALLY`, `STRING$`, `SPACE$`, `CHR$`, `ASC`, `HEX$`, `BIN$`,
`OCT$`, `CONVBASE$`, `FORMAT$`, `STRF$`, … — the full list is in
[the builtins reference](reference/builtins.md). String functions count
characters, not bytes.

- `""` inside a string is **not** an escaped quote (RapidQ reads two strings
  side by side). Use `CHR$(34)`, or turn on `$ESCAPECHARS ON` for `\"`,
  `\n`, `\t`, `\\`, `\x41`.

## Files

```basic
OPEN "notes.txt" FOR OUTPUT AS #1     ' also INPUT, APPEND, BINARY, RANDOM
PRINT #1, "first line"
CLOSE #1
f = FREEFILE
OPEN "notes.txt" FOR INPUT AS #f
WHILE NOT EOF(f)
    LINE INPUT #f, s$
    PRINT s$
WEND
CLOSE #f
PRINT FILEEXISTS("notes.txt"), FILELEN("notes.txt")
```

The stream objects do the same with methods: `RFileStream` (RapidQ's
`QFILESTREAM`: `Open`,
`ReadLine`, `WriteLine`, `ReadNum`, `WriteNum`, `Read`, `Write`, `Seek`,
`Size`, `Position`, `SaveArray`, `LoadArray`, …), `RMemoryStream` and
`RStringList` (`LoadFromFile`, `SaveToFile`). `DIR$`, `KILL`, `MKDIR`,
`RMDIR`, `RENAME`, `CHDIR`, `CURDIR$` and `DIREXISTS` work with folders.
In a browser, files are the page's own (see [The web](web.md)).

## The console

`CLS`, `COLOR fg, bg`, `LOCATE row, col`, `CSRLIN`, `POS(0)`, `INPUT
[prompt,] var`, `INKEY$`, `INPUT$(n)`, `SLEEP seconds` (`SLEEP 0.5`), `BEEP`,
`SOUND`. Colours and positions are written as ANSI sequences, so they work
in any modern terminal and in the IDE's output panel.

## The preprocessor

| Directive | |
|---|---|
| `$INCLUDE "file"` | include a file (RapidQ's include folders and `\` paths work; `RAPIDQ.INC` is built in) |
| `$DEFINE NAME [text]`, `$UNDEF NAME` | text substitution |
| `$IFDEF` / `$IFNDEF` / `$ELSE` / `$ENDIF` | conditional compilation |
| `$MACRO NAME(x) = text` | a macro with parameters |
| `$APPTYPE CONSOLE \| GUI \| CGI \| WEB` | the program's kind |
| `$RESOURCE NAME AS "file"` | a file built into the program (`RESOURCE(n)`, `EXTRACTRESOURCE`, pictures) |
| `$OPTION ICON "file.ico"` | the program's icon: its windows' and the built app's ([Building apps](building-apps.md)) |
| `$ESCAPECHARS ON` | escape sequences in this file's strings |
| `$TYPECHECK ON` / `OFF` (`$OPTION EXPLICIT`: ON) | every variable must be declared (DIM, CONST, a parameter): an undeclared one stored into is RapidQ's `Undeclared identifier x`, read is its `Undefined symbol X` |
| `$OPTIMIZE` | accepted |
| `$THEME name` | RapidR's: the desktop look ([Components](components.md#themes)) |
| `#If … Then` / `#ElseIf` / `#Else` / `#End If`, `#Const` | VB's conditional compilation |

An error in included code points at the include file and line.

## Errors

- Compile errors name the file, line and column, and every bad line is
  reported, not just the first: `syn.bas:3:1: error: Syntax error in FOR
  statement`.
- A run-time error stops the program with its message and line: `run-time
  error: Division by zero (at err.bas line 4)`.
- `ON ERROR GOTO` / `ON ERROR RESUME NEXT` (VB, not RapidQ) are accepted
  so that such programs compile, and **ignored**: a run-time error still
  ends the program.

## Native code: `RUSTSTART`

In native builds only, `RUSTSTART … RUSTEND` puts Rust code in the
program. The interpreter and the web refuse it with a clear message.
