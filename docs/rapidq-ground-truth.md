# RapidQ's own compiler as the ground truth

When the manual, the phatcode mirror and the corpus leave a question open — how RapidQ parses something, what a built-in returns, how PRINT formats a number — the answer is what RapidQ itself does. RapidQ's compiler (`RC.EXE`, Rapid-Q 2006, William Yu) still runs: compile a small program with it, run the program, read what it prints. RapidR's behaviour is then made to match, and the program becomes a conformance case whose `.expected` is RapidQ's output (`tests/conformance/cases/juxtaposed_operands.bas` was made this way).

## Running it: `tools/rc_probe.sh`

```sh
tools/rc_probe.sh <dir> [timeout seconds]
```

compiles every `.bas` in `<dir>` with RC.EXE in the Parallels Windows VM and runs the console ones, printing what RC and each program say:

```
== a_print.bas
Compiling as CONSOLE Application (L4)
-- run
a 1 2
== jy6.bas
Line 3: ERROR: Member X not part of class J
jy6.bas: In main section
jy6.bas: PRINT "jy6 "; J.X
jy6.bas:              ^-- error
File in error: jy6.bas
```

What it does, step by step:

1. **The VM.** `RAPIDR_VM` (default `Windows 11 Pro`, Windows 11 on ARM). Parallels pauses an idle VM by itself; the script resumes a paused or suspended one (`prlctl resume`). It doesn't pause it again.
2. **Files through shared folders.** Parallels shares the Mac's home folder with the VM as `\\Mac\Home` (mapped to `Z:` too). The script turns Mac paths under `$HOME` into `\\Mac\Home\…` paths: RapidQ's folder (`RAPIDQ_DIR`, default `~/Downloads/Rapidq`: `RC.EXE`, `Lib\`, `include\`), the programs' folder, and `tools/windows/rc_probe.ps1` itself. So `<dir>` must be under your home (the repository's `tests/conformance/.work/…` is; the session scratchpad in `/private/tmp` isn't).
3. **The run in Windows** (`tools/windows/rc_probe.ps1`, through `prlctl exec "<vm>" --current-user powershell -ExecutionPolicy Bypass -File \\Mac\Home\…\rc_probe.ps1`): RapidQ's `RC.EXE`, `Lib\` and `include\` copied to `%USERPROFILE%\rq`, the programs to `%USERPROFILE%\rq\t` (RC writes the `.exe` beside the source; nothing is written to the Mac's RapidQ folder), then for each program `RC.EXE -I<rq>\include -L<rq>\Lib prog.bas` — its banner and compile summary dropped, its errors kept — and, when an `.exe` came out, the program run with `Start-Process -RedirectStandardOutput` and the output printed. A program still running after the timeout (default 10 s) is ended and reported as `TIMEOUT`. Programs whose name starts with `g_` are only compiled (a GUI program waits for its form to close).
4. **x86 on ARM.** RC.EXE and the programs it makes are 32-bit x86 Windows programs; Windows 11 on ARM runs them under its x86 emulation with nothing to set up. They're quick (a compile in well under a second).

## Writing probes

- **Console programs** start with `$APPTYPE CONSOLE`; PRINT then goes to standard output, which the script captures. RC says `Compiling as CONSOLE Application (L4)` (`L1` when the program uses a DirectX object: the GUI library).
- **CRLF line endings**: the probes so far were all written with them (RapidQ is a DOS-era tool); whether RC minds LF wasn't tried.
- **One question per program when a probe may fail**: RC stops at the first error (`Line N: ERROR: …`, with the source line and a caret), and a program that raises an exception loses the output it had printed (it's buffered) — `QDXJOYSTICK` without a joystick, for instance, prints nothing at all. Several PRINT lines in one program are fine when they all compile and run.
- **RapidQ's names are case-insensitive and global**: `CONST A` and `DIM a(10)` clash (`Identifier A already used`); built-in names are taken (`SUB Pos` → `POS identifier already in use`).
- **Compare what RapidR prints**: the same file through `rapidr build-bc f.bas -o f.rrbc && rapidr run-bc f.rrbc` (or as a conformance case, both backends). PRINT's number formatting differs today (RapidQ prints a fractional double with 9 decimals: `-984.147000000`); keep probes to integers and strings unless the formatting is the question.
- **RC.EXE's own strings** answer "does RapidQ have X?" without running anything: `strings -n 3 ~/Downloads/Rapidq/RC.EXE | grep -i <name>` lists its objects and their members in pairs (`QDXJOYSTICK|ISLEFT|QDXJOYSTICK|ISRIGHT|…`) and every message it can print (`.reference/rapidq-compiler-messages.txt` is that list).

## Quirks met

- A worktree-isolated agent's shell refuses `prlctl exec … cmd /c …` typed on its command line (it can't show the remote command isn't `git`); running a script file (`tools/rc_probe.sh`) works.
- `cmd /c` with `!var:~0,2!` substrings broke on quoting; PowerShell in a `.ps1` is simpler.
- An earlier way of running the programs (`cmd /c prog.exe` attached to the console) hung on a program that raised an exception — RapidQ's message box waited for a click — until the process was killed with `taskkill`; `Start-Process` with redirected output and a timeout doesn't.
- The VM may be paused again while a probe runs (it pauses when idle, or someone pauses it); `prlctl exec` then waits forever — interrupt it, resume the VM, run again.

## Findings so far

- Operands side by side (`A B OR C`, `-9(COS(x))`, `2(3)`, `CASE 4, 7  C = -2`): RC's operator stack runs on and the value is the operand stack's bottom; the dropped operands are still worked out. RapidR matches (parser: `parse_stacked_expression`; `tests/conformance/cases/juxtaposed_operands.bas`).
- `INT` truncates toward zero (`INT(-2.5)` is -2, `INT(-0.5)` is 0), whatever the manual says; RapidR floors — open.
- PRINT of a fractional DOUBLE: 9 decimals (`-984.147000000`, `-9.841470985`); RapidR prints the shortest form — open.
- `QDXJOYSTICK` exists (undocumented): `IsLeft`, `IsRight`, `IsUp`, `IsDown` (read-only), `Button(n)` (read-only, one argument), `Update` (no arguments); no `Tag` / `Parent`; `CREATE` works; an array of them is refused (`Array of QDXJOYSTICK is not supported!`). Without a joystick it raises `EStringListError` (List index out of bounds) — not copied: RapidR's says Connected 0.
- `CASE ELSE <statement>` on one line is refused (`Expected end-of-line but got …`); RapidR accepts it.
- `QRECT` (Left, Top, Right, Bottom) and `QNOTIFYICONDATA` (cbSize, hWnd, uID, uFlags, uCallbackMessage, hIcon, szTip) are objects with fixed fields: 0 until set, except cbSize 88 (read-only: `N.CBSIZE is a read-only value.`) and uID 4194304 (`&H400000`, the instance handle — though `Application.hInstance` prints 0 in a console program). Numbers are 32-bit integers cut toward zero (`3.7` → 3, `-2.5` → -2, `-3.9` → -3); a value outside 32 bits becomes -2147483648 (`2147483648`, `4294967297`); a string stored into a number is 0 (`"12"` too), a number stored into szTip is "". szTip keeps 64 characters, up to the first `CHR$(0)`. `SIZEOF` is 16 and 24. `CREATE r AS QRECT` works; a SUB gets one by reference; no `Tag` (`Member TAG not part of class R` — the member's dotted path, the object's name, both upper case); `r2 = r` is `Component assignment is not yet supported.`; a component field in a TYPE without EXTENDS is `Datatype QRECT not supported in STRUCT` (QFONT too: RapidR refuses only the two data types — its own programs compose TYPEs of components); in a TYPE EXTENDS QOBJECT it's fine; `DIM a(3) AS QRECT` is `Array of QRECT is not supported!`. RapidR matches (`rapidr_value::objects::record`, `rapidr_ast::fixed_members`; cases `qrect_fields`, `notifyicondata_fields`, `record_errors`).
- `QGLASSFRAME` (built into RC.EXE: UtilMind's freeware TGlassy, "Glassy Form", in RapidQ's runtime) has exactly: Left, Top, Width, Height, ClientWidth, ClientHeight, Color, Enabled, Visible, ShowHint, Hint, PopupMenu, Cursor, Handle (read-only: `G.HANDLE is a read-only value.`), Align, Moveable, Transparency, TransparentColor, Parent and OnClick, OnDblClick, OnMouseDown, OnMouseMove, OnMouseUp — no Caption, Tag, Font, OnPaint, Repaint, AutoSize. Inside a form: 105 × 105 at 0, 0, Color clBtnFace, Transparency 60, TransparentColor 0, Moveable 1 (Delphi's True: 1, and a stored 5 reads 5), Align 0, Enabled / Visible 1; Transparency is a byte (`-5` → 251, `256` → 0, `33.7` → 33); ClientWidth sets Width. Without a parent, reading any property raises an access violation (not copied). An array of them is allowed.
- `QBEVEL`, `QDIGDISPLAY`, `QDIRLISTVIEW`, `QDOCKFORM` aren't RC.EXE's (no strings for them): they are TYPEs of RapidQ's include libraries (`QBevel.inc`, `QDigDisplay.inc`, `QDirListView.inc`, `RAPIDQ2.INC`), whose source is the ground truth for their behaviour. QDigDisplay.inc's 33 pictures (`QdigDisplay_Bmp.zip`) are 12 × 24, 16 colours: black, lit segments cyan (`&HFFFF00`), unlit segments a dark green (`&H008000`) checker dither.
