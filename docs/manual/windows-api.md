# One standard for the Windows API

If you have written programs that talk to Windows itself — a `DECLARE …
LIB "user32"` here, a `CONST WM_CLOSE` there — you probably know this
story. Everyone had a `WIN32API.INC`. It came from a friend or an old
example, and then it was fixed by hand: a constant added here, a
declaration corrected there. Over the years there were many
versions of it, and they didn't agree. Two of them in one program would
clash. A snippet that worked for someone else failed for you, because your
file had another name, another value, or nothing at all. Many of us lost
hours to that, and nobody did anything wrong: there simply was no common
file to share.

RapidR now has one: **`RWin32.inc`**, built in.

## First: nothing you have stops working

- **Your programs keep running as they are.** A program with its own
  `WIN32API.INC` (or any other include file) compiles and runs exactly as
  before, through RapidR's RapidQ compatibility layer. You don't have to
  change anything, today or later.
- **Your own include files stay yours.** RapidR never replaces or edits
  them. If you prefer your file, keep it.
- **Nothing is switched for you.** The importer only *offers* the change,
  and shows you every difference before anything happens (below).

## The change, if you want it: one line

```basic
$INCLUDE "RWin32.inc"
```

That's all. There is no file to download, find or copy: `RWin32.inc` is
part of RapidR, on every computer and every install (the Windows 7 and XP
downloads too). Any path or capitals work (`$INCLUDE
"C:\rapidq\include\RWIN32.INC"` reads the same built-in file), so an old
line only needs its file name changed.

**RapidR helps you switch.** When you import an older program
([Importing older programs](importing-rapidq.md)) that includes a
`WIN32API.INC`:

- the copy keeps your file, as always, and is checked to compile exactly
  like the original;
- the report then says that `RWin32.inc` can take its place, and **lists,
  for the names your program uses, everything that would differ**: a
  constant with another value, a function your file called by another DLL
  name or with another number of arguments, a name `RWin32.inc` doesn't
  have, and what your file did besides declaring (a `$DEFINE INTEGER SHORT`,
  an `$OPTION`);
- you decide. In RapidR Studio, File ▸ Import RapidQ Project or File… asks
  you after the import (**Keep my file** is the default answer). With the
  command line, `rapidr import-rapidq myprogram.bas --rwin32` makes the
  switch in the copy and checks that the copy still compiles. Your original
  program is never changed.

## Why one standard helps everyone

- **One file for all of us.** An example, an answer in a discussion or a
  snippet from the manual that uses `RWin32.inc` works on your computer the
  same way it works on the author's. RAI, RapidR's assistant, and other AI
  helpers can rely on the same names too, so their Windows code is right
  more often.
- **Complete and current.** `RWin32.inc` isn't typed by hand: RapidR's
  own tool generates it from the Win32 API metadata that Microsoft
  publishes. It has 55,710 constants, 3,675 structures (`TYPE`s) and
  6,506 functions of 29 Windows DLLs (kernel32, user32, gdi32, advapi32,
  shell32, comdlg32, comctl32, winmm, winspool, ole32, wininet, ws2_32 and
  more).
- **Right on 64-bit and 32-bit Windows.** Old include files were written
  for 32-bit Windows, where a pointer and a handle are 4 bytes. On today's
  64-bit Windows they are 8, so a structure such as `NOTIFYICONDATA`,
  `MSG` or `SYSTEM_INFO` needs another layout. `RWin32.inc` has both, and
  your program gets the right one for the computer it is built for.
- **The classic names are there.** `MessageBox`, `GetWindowText`,
  `FindWindow`, `SendMessage`, `ShellExecute`, `RECT`, `POINTAPI`,
  `LOGFONT` … mean what they meant: the unsuffixed name is the ANSI ("A")
  function or structure, as the classic files had them. 1,496 of the 1,514
  functions a widely used classic `WIN32API.INC` declares (98.8%) are in
  `RWin32.inc` under the same names; the modern A and W names
  (`MessageBoxA`, `MessageBoxW`) are there as well.
- **Fast.** A program gets only what it uses: a program with twenty
  Windows calls compiles in well under a second, and carries only those
  twenty declarations.

## It is your choice

Use `RWin32.inc` in new programs if you like it, switch old ones when it
suits you, or keep your own file: all three are fine, and RapidR supports
each of them. If something in `RWin32.inc` is missing or looks wrong to
you, please tell us in [Discussions](https://github.com/iBobX/RapidR/discussions)
or open an [issue](https://github.com/iBobX/RapidR/issues): it is
generated, so a fix reaches everyone at once.

RapidR is free and open, made so that anybody can learn to program, and we
grow it together, people and AI alike. Thank you for being part of it.

---

## How it works

### Only what your program uses, and your own names first

`RWin32.inc` is large, so RapidR doesn't compile all of it into your
program. It reads your program, sees which names it uses, and gives it
those declarations (and the `TYPE`s they need) — nothing else.

**A name your program has of its own always wins**: a `CONST`, a
`DECLARE`, a `TYPE`, a `SUB` or `FUNCTION`, a `DIM`, a `CREATE`, a
`$DEFINE`, or a variable you store into. So an older program that declares
`CONST WM_MOUSEWHEEL = &H20A` itself, or its own `DECLARE FUNCTION
GetTickCount …`, compiles with `RWin32.inc` with no "already defined"
error: yours is used. (A variable and a `TYPE` may share a name: `DIM msg
AS MSG` works.)

### RAPIDQ.INC's numbers, and True and False

The Windows numbers that RAPIDQ.INC also has (`MB_OK`, `MB_ICONERROR`,
`VK_F1`, `SND_ASYNC`, the character sets …) aren't repeated in
`RWin32.inc`, so the two never clash, whichever comes first.

- In a `.bas` program that includes `RWin32.inc` **without**
  `$INCLUDE "RAPIDQ.INC"`, RapidR's RapidQ compatibility layer gives the
  ones it uses, with RAPIDQ.INC's numbers (the same as Windows'), and
  `True` = 1 and `False` = 0, as a classic `WIN32API.INC` did.
- In a RapidR program (`.rr`) they are built in already, and `True` stays
  RapidR's -1.

### Names RapidR has too: Sleep, Beep, MessageBox

A few Windows functions share their name with something RapidR has
(`Sleep`, `Beep`, `MessageBox`, `SendMessage`, `PlaySound`).

- In a classic program (`.bas`), `RWin32.inc` gives you the Windows
  function, as the old include did: `Sleep 500` waits 500 milliseconds,
  through kernel32.
- In a RapidR program (`.rr`), RapidR's own stays: `SLEEP 1` waits one
  second on every system. To call the Windows one there, declare it under
  a name of your own: `DECLARE SUB WinSleep LIB "kernel32" ALIAS "Sleep"
  (BYVAL ms AS LONG)`.

### 64-bit and 32-bit: `WIN64`

When a program is compiled for 64-bit (Windows on x64 and ARM64, macOS,
Linux, and the web), RapidR defines `WIN64`, and `$IFDEF WIN64` picks each
structure's 64-bit layout; for 32-bit Windows (the Windows 7 and XP
downloads) it isn't defined. Your own code can use it too:

```basic
$IFDEF WIN64
PRINT "64-bit"
$ELSE
PRINT "32-bit"
$ENDIF
```

(The web defines `WIN64` as well, so a program's structures have the same
sizes there as on 64-bit desktops; Windows' functions don't run on the web
in any case.)

### How the declarations are written

- **Handles, pointers and other pointer-sized numbers** are `LONG`
  parameters: RapidR passes them in a 64-bit slot and keeps a 64-bit
  pointer a DLL returns for you ([DLL calls](language.md#subs-and-functions)).
- **Text** (`LPSTR`, `LPCSTR`) is `STRING`: give it text, or a
  `SPACE$(n)` buffer the function fills. Wide text (the W functions) is a
  `LONG`: the address of a UTF-16 buffer. The classic names are the A
  functions, which take ordinary strings.
- **A structure** is passed by reference (`lprc AS RECT`). Give `0` for
  "none" (`NULL`): RapidR passes a null pointer.
- **In a TYPE**, a pointer or handle field is an `INT64` on 64-bit Windows
  and a `LONG` on 32-bit; a text pointer field is a `STRING` (and, on
  64-bit, a `…Hi` `LONG` after it for the pointer's high half, always 0);
  the gaps Windows' compiler leaves between fields are fields of their own
  (`rrPad1`, `rrPad2` …), so `SIZEOF` is the structure's real size.
  An unnamed union is its largest member.
- **A small structure passed by value** (`POINT` in `PtInRect`,
  `WindowFromPoint`) is one `INT64`: `x + y * 4294967296` (that is, `y` in
  the high 32 bits). That is how 64-bit Windows receives it; on 32-bit
  Windows the same `INT64` gives the same bytes.

### What isn't in it

GUID constants, COM interfaces, functions with a variable number of
arguments (`wsprintf`) and a few functions that take a large structure by
value. Some areas are left out for now (cryptography, GDI+, ODBC, WinHTTP,
IP Helper …): RapidR has components of its own for most of them
(`RSQLITE`, `RDB`, `RSOCKET` …), and you can always `DECLARE` what you
need yourself, next to the include.

### On macOS, Linux and the web

A program that includes `RWin32.inc` compiles everywhere; a call into a
Windows DLL runs on Windows and elsewhere stops with a clear message that
names the function. See [the example](../../examples/windows/rwin32_system_info.rr).

### The file itself

You can read it: [crates/rapidr-preprocessor/src/libraries/RWin32.inc](../../crates/rapidr-preprocessor/src/libraries/RWin32.inc)
in RapidR's source. Its header says where it comes from: generated by
RapidR's own tool (`tools/gen-win32`) from the Win32 API metadata
Microsoft publishes in the windows-rs project, under the MIT License, with
Microsoft's copyright notice ([provenance](../legal/win32-declarations-provenance.md)).
`$INCLUDE "RWin32.inc"` always reads RapidR's built-in file, never a file
of that name on your disk.
