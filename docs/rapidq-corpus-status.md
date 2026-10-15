# RapidQ's example programs: where each one stands

RapidQ's distribution ships 428 example programs (`examples/`: `.bas`, and
the `.rqw` / `.rqb` / `.rq` its IDE saves). This page says, program by
program, how each one does under RapidR against RapidQ's own compiler — no
RapidQ code is in it, only the programs' names and what was seen.

## On Windows (CORPUS-WIN, 2026-10-15)

Every program was run in the Windows 11 VM (ARM64) three ways, side by side,
by [`tools/windows/corpus_win.py`](../tools/windows/corpus_win.py):

- **RC.EXE** — RapidQ's compiler (2006), the ground truth
  ([rapidq-ground-truth.md](rapidq-ground-truth.md)): the program compiled
  and run, its windows captured after three seconds (a console program's
  output kept);
- **Windows interpreted** — the program run by an installed RapidR built
  from this branch (`rapidr run`, the SDK laid out as the installer lays it
  out: the runtime's sources vendored, the LLVM-MinGW toolchain), in the
  classic theme (`RAPIDR_THEME=classic`: RapidQ's look, for the
  comparison), its windows captured from the screen and by RapidR;
- **Windows native** — the same program built by that RapidR
  (`rapidr build`, a Rust build) and run the same way.

What the columns say, against RC.EXE's run of the same program in the same
Windows ([`tools/windows/corpus_win_report.py`](../tools/windows/corpus_win_report.py)
writes them):

- **same** — no error; a console program printed what RC.EXE's printed (the
  text: RapidR also draws COLOR / LOCATE / CLS with the terminal's escape
  sequences); a GUI one showed as many windows, of the same size, looking
  alike. "but RapidR's form frame": the window's inside is larger by the
  frame difference below, nothing else.
- **differs** — it runs, but something isn't RC.EXE's; what follows says what.
- **fails** — a compile error, a run-time error, a crash, or no window where
  RC.EXE shows one.
- **RC.EXE fails** — RC.EXE doesn't compile it, or its program stops with
  an error (RapidR's result follows).
- **not compared** — it runs, but there is nothing of RC.EXE's to hold it
  against: RC.EXE's console output was lost to the file it went to (its
  program waited for a key), or the program's first window is one of
  Windows' file, print or colour dialogs, which the test run answers.
- **not run** — it would print, write the registry through Windows, start
  other programs, inject input, move or delete files, change system
  settings, shut the machine down, open a server socket or use a serial port
  or port I/O: only built (by both compilers).

Programs whose output depends on the clock or random numbers count as the
same when they run without an error. The notes are what a person saw
comparing the captures ([`tools/windows/corpus_win_notes.json`](../tools/windows/corpus_win_notes.json)).

**Form frames.** RapidR draws its windows' frames itself, the same on every
system: a form's Width × Height holds a larger inside than RC.EXE's on
Windows 11, whose frames are wider (a sizeable form's frame takes 16 × 39
there, RapidR's 2 × 31). A main menu is part of RapidR's inside (28 high);
RC.EXE's sits above its inside (20). Where only that differs the column
says "same windows, RapidR's form frame"; where RC.EXE's smaller inside
makes the program look different (scroll bars, a control cut off) it says
"differs". "difference" is the captures' mean grey difference (0–255;
fonts and anti-aliasing make a little everywhere).

### Summary

<!-- corpus-win-summary:start -->
| | same | differs | fails | RC.EXE fails | not compared | not run | — |
|---|---|---|---|---|---|---|---|
| Windows interpreted | 140 | 51 | 32 | 128 | 13 | 64 | 0 |
| Windows native | 134 | 53 | 41 | 128 | 13 | 59 | 0 |

RC.EXE: 231 run, 131 fail (compile error, no window), 66 built but not run, of 428 programs.
<!-- corpus-win-summary:end -->

The table is the run of 2026-10-15 (RapidR built from this branch's state
that morning). Fixes made after it, from what it found, are marked
"(fixed)" in the notes; they weren't run again in Windows.

### What is left, most programs first

Most of what fails, fails in both RapidR columns and comes from what a
64-bit program that draws its own controls can't do (docs/windows-dll-calls.md):

1. **64-bit pointers and handles kept in LONGs** (13 runs): memory a DLL
   returned, read with PEEK or a TYPE (RapidQ's callback includes, ODBC,
   AVIFile, IPicture, FileSearch, the grids' editing).
2. **x86 machine code run through CallWindowProc** (12 runs: the `asm`
   folder, AlphaTest): RapidR programs are 64-bit; not possible.
3. **Callbacks** — `CODEPTR` handed to a DLL (window procedures,
   CreateThread, subclassing: ColorButton, MinToTaskbar, titlebtn, Pipe,
   purewindows, winex): not supported yet (ROADMAP).
4. **Window messages and per-control handles**: a control's Handle isn't a
   window (QTabVertical's SendMessage), a form's isn't one before it shows
   (hotkeys' RegisterHotKey), WndProc gets no messages (FD_File_Receiver).
5. **32-bit DLLs shipped with a program** (PASCAL.DLL, RqUtils.dll) and DLLs
   Windows 11 doesn't have (STKIT432.DLL).
6. **Native-only compile errors** (the interpreter runs them): a component
   array in a SUB (`QTabContol_add_del`), a callback include's
   `expected function, found &Value` (QAnimate, QUpDown, QToolBar,
   ResourceView), `baseGG3`, `SQL_blobs/archive`, `jpeg_Blend`
   (`loadimage` twice), `BDX-8`; `QDerulantPanel_test`'s native build stops
   at start (0xC000007B).
7. **Looks**: RapidR's form frame and in-window menu against Windows 11's
   (above); RapidQ's message boxes take Windows 11's message font (Segoe UI
   9, 90 high), RapidR's classic look MS Sans Serif 8 (78); Windows' smallest
   window width, which RapidR doesn't apply (QRICHEDIT's formatting, not
   drawn in this run, came after it: RICHEDIT).
8. **RapidQ accepts what RapidR stops at** (decisions for later, each
   checked against RC.EXE): an IF left open at END SUB and closed by an extra
   END IF in the next SUB (Terrain16), an END IF inside a FOR closing an IF
   opened before it (mySQL_API), a call of an unknown name read as 0
   (a typo in the asteroids games), `COMMAND$(0)` naming the source under the
   interpreter (programs that make their data file's name from it with
   its extension replaced: GMSynth, StutterBox, …), a number stored in a STRING
   variable (cgi_test).

### Every program

<!-- corpus-win:start -->
| Program | Windows interpreted | Windows native | RC.EXE | Notes |
|---|---|---|---|---|
| arrays/arrins.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console, runs until stopped |  |
| arrays/control array.rqb | same (same windows, RapidR's form frame, difference 5.4) | same (same windows, RapidR's form frame, difference 5.4) | 984×711 |  |
| arrays/DynamicArrayOfUDTs.rqw | same (same windows, RapidR's form frame, difference 7.9) | same (same windows, RapidR's form frame, difference 7.9) | 264×134 |  |
| arrays/qfixedstringstack.rq | same (same output) | same (same output) | console |  |
| asm/Nasm/Bin_To_Inc/Demo/ReverseStringDemo_1.Bas | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | console |  |
| asm/Nasm/Bin_To_Inc/Demo/ReverseStringDemo_2.Bas | RC.EXE fails; RapidR: compile error: Include file not found: 'ReverseString.Inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file ReverseString.Inc does not exist |  |
| asm/Nasm/Bin_To_Inc/RQ_Sources/BinToInc.Bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 984×541 |  |
| asm/Nasm/Bin_To_Inc/RQ_Sources/BinToInc_Plus.Bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 984×541 |  |
| asm/Nasm/Pavel Crc32/crctest.rq | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | console |  |
| asm/Nasm/Pavel Vector/vector_asm.rq | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | console |  |
| asm/Nasm/Pavel Vector/vector_rq.rq | same (output depends on the clock or random numbers) | same (output depends on the clock or random numbers) | console |  |
| asm/Pointer_test.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: Member X not part of Type MYUDT.Z |  |
| asm/TestRqAsmUtils.Bas | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | 322×150 (dialog) |  |
| asm/TestRqUtilsDll.Bas | fails: run-time error: 'RqUtils.dll' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DL | fails: run-time error: 'RqUtils.dll' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DL | 240×150 (dialog) | Its RqUtils.dll is 32-bit; RapidR programs are 64-bit. |
| asm/vector_asm.rq | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | console |  |
| asm/vector_rq.rq | same (output depends on the clock or random numbers) | same (output depends on the clock or random numbers) | console |  |
| AVI/animateAVI.bas | differs: looks different (capture difference 17.2) | differs: looks different (capture difference 17.2) | 272×79 | Its QANIMATE's area: RC.EXE's white, RapidR's the form's colour. |
| AVI/aviplay.bas | same (same windows, RapidR's form frame, difference 6.3) | same (same windows, RapidR's form frame, difference 6.3) | 314×191 |  |
| AVI/testAVItoBMP.bas | fails: run-time error: the call to AVIFileGetStream in avifil32 crashed (access violation reading address 0x0); an argument wasn't th | fails: run-time error: the call to AVIFileGetStream in avifil32 crashed (access violation reading address 0x0); an argument wasn't th | 255×90 (dialog) | AVIFile's interface pointers kept in LONGs: 64-bit pointers don't fit. |
| AVI/VideoPlayer.bas | differs: looks different (capture difference 15.2) | differs: looks different (capture difference 15.2) | 282×241 | The frame shown at the capture differs (the video plays). |
| Base64/example.bas | same (same windows, RapidR's form frame, difference 5.9) | same (same windows, RapidR's form frame, difference 5.9) | 284×161 |  |
| bmp/BMP_viewer2.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| bmp/BMPClipboard.bas | RC.EXE fails; RapidR: compile error: Syntax error in END statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Misplaced EXIT SUB without SUB declaration |  |
| bmp/bmpdll/bmpdll.bas | same (same windows, RapidR's form frame, difference 1.2) | same (same windows, RapidR's form frame, difference 1.2) | 304×201 |  |
| bmp/ReScaleBMP.bas | RC.EXE fails; RapidR: compile error: $RESOURCE jpeg_DLL: file not found: 'jpeg.dll' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: RESOURCE file jpeg.dll does not exist |  |
| bmp/Save2BMP.bas | same (same windows, RapidR's form frame, difference 1.4) | same (same windows, RapidR's form frame, difference 1.4) | 504×481 |  |
| buttons/ButtonBar.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| buttons/ColorButton.bas | fails: run-time error: 'SetWindowLongA' is given CODEPTR(ButtonProc), a callback the DLL would call back into the program; RapidR doe | fails: run-time error: 'SetWindowLongA' is given CODEPTR(ButtonProc), a callback the DLL would call back into the program; RapidR doe | 216×82 |  |
| buttons/QColorButton.bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 304×201 |  |
| buttons/XPBtn.bas | same (same windows, RapidR's form frame, difference 3.3) | same (same windows, RapidR's form frame, difference 3.3) | 384×261 |  |
| Calendar/ApiDate.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Calendar/qcalendarTEST.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: compile error: cannot find value `setnewcallback_4` in this scope | compile error: Error in expression, unresolved parenthesis |  |
| CallBacks/BindSpeedTest.Bas | same (output depends on the clock or random numbers) | same (output depends on the clock or random numbers) | console |  |
| CallBacks/NewWndProcDemo.Bas | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | 304×411 |  |
| CallBacks/OverIncludingTest.Bas | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | 304×411 |  |
| CallBacks/TestNoMouse.Bas | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | 784×741, 445×150 (dialog) |  |
| CGI/ADODB_server.bas | RC.EXE fails; RapidR: compile error: Unknown SUB or FUNCTION 'RESPONSEWRITE' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got """ |  |
| CGI/arachnoWare Web.Objects/WebObjects tips.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: compile error: cannot find value `environn` in this scope | compile error: Member WRITE not part of class RESPONSE |  |
| CGI/cgi_test.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | compile error: Member INITCGI not part of class CGI |  |
| CGI/cgi_test.rqb | differs: output line 9 differs from RC.EXE's | differs: output line 9 differs from RC.EXE's | console | It stores a numeric method's result (whether a form field was found) in the STRING it passed BYREF for the field's value: RapidQ then keeps that argument's text ("" here), RapidR stores "0" (a judgment call: docs/rapidq-ground-truth.md). |
| CGI/crap.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| CGI/info.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | compile error: Member INITCGI not part of class CGI |  |
| CGI/info.rqb | same (same output) | same (same output) | console |  |
| CGI/test_256Byte.bas | differs: output line 3 differs from RC.EXE's | differs: output line 3 differs from RC.EXE's | console | RapidQ's CGI PRINT sends 255 characters of a longer string, then NULs (the program shows that bug); RapidR sends the whole string. |
| CGI/trash.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'qcgi_New_test2.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file qcgi_New_test2.inc does not exist |  |
| CGI/WebObjects.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'OBJ_HTTP.INC' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file OBJ_HTTP.INC does not exist |  |
| Compile/CodeBox.bas | RC.EXE fails; RapidR: compile error: Invalid $RESOURCE syntax: Please check your original-code. You described an other bmp-source! | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: RESOURCE file folder.bmp does not exist |  |
| Compile/rapidq_boosta.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| Compile/RQdecl.bas | RC.EXE fails; RapidR: compile error: Syntax error in IF statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \RapidQ\include\WINDOWS.INC does not exist |  |
| Compile/RunTime/RunTimeSimpl.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'eMail.Inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file rapidq2\RapidQ2.inc does not exist |  |
| ComPort/comdialer/comportdialer.rq | RC.EXE fails; RapidR: compile error: Syntax error in CONST statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file c:\rapidq\COMPORT.CMP does not exist |  |
| ComPort/ComPort_example.bas | not run: builds (not run: serial port) | not run: builds (not run: serial port) | builds (not run: serial port) |  |
| console/3dbox/3DBOX.BAS | same (runs until stopped, as RC.EXE's) | same (runs until stopped, as RC.EXE's) | console, runs until stopped |  |
| console/Command Line Parameters.bas | same (same output) | same (same output) | console |  |
| console/movetext/MOVETEXT.BAS | same (runs until stopped, as RC.EXE's) | same (runs until stopped, as RC.EXE's) | console, runs until stopped |  |
| console/printf/printf.bas | same (same output) | same (same output) | console |  |
| console/seeqsort/seeqsort.bas | same (runs until stopped, as RC.EXE's) | same (runs until stopped, as RC.EXE's) | console, runs until stopped |  |
| console/sieve/sieve.bas | same (output depends on the clock or random numbers) | same (output depends on the clock or random numbers) | console |  |
| cursors/animated/cursors.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| cursors/loadcur/newcur.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| Database/baseGG/baseGG3.bas | not run: builds (not run: prints) | fails: compile error: cannot find value `fichier` in this scope | builds (not run: prints) |  |
| Database/DatabaseViewer/DatabaseViewer.rq | fails: run-time error: the call to SQLAllocConnect in odbc32 crashed (access violation reading address 0x74974530); an argument wasn' | fails: run-time error: the call to SQLAllocConnect in odbc32 crashed (access violation reading address 0x4010f620); an argument wasn' | 1590×945 | ODBC handles kept in LONGs: 64-bit handles don't fit. |
| Database/mdb/0-TestDrivers.bas | fails: run-time error: the call to SQLSetEnvAttr in odbc32 crashed (access violation reading address 0x1f858390); an argument wasn't  | fails: run-time error: the call to SQLSetEnvAttr in odbc32 crashed (access violation reading address 0xffffffffc310ca20); an argument | 291×390 (dialog) | As Database/DatabaseViewer (ODBC handles in LONGs). |
| Database/mdb/1-CreateDB.bas | differs: window 1 434×78 (RC.EXE 460×90); looks different (capture difference 16.0) | differs: window 1 436×78 (RC.EXE 460×90); looks different (capture difference 18.2) | 460×90 (dialog) | Its message box: RC.EXE's (Delphi's) sizes itself to Windows 11's message font, Segoe UI 9 (90 high); RapidR's classic look uses MS Sans Serif 8 (78). |
| Database/mdb/2-CreateTable.bas | differs: window 1 414×78 (RC.EXE 436×90); looks different (capture difference 16.3) | differs: window 1 416×78 (RC.EXE 436×90); looks different (capture difference 18.8) | 436×90 (dialog) | Message box font, as 1-CreateDB. |
| Database/mdb/3-AddRecord.bas | differs: window 1 414×78 (RC.EXE 436×90); looks different (capture difference 16.3) | differs: window 1 416×78 (RC.EXE 436×90); looks different (capture difference 18.8) | 436×90 (dialog) | Message box font, as 1-CreateDB. |
| Database/mdb/4-EditRecord.bas | differs: window 1 414×78 (RC.EXE 436×90); looks different (capture difference 16.3) | differs: window 1 416×78 (RC.EXE 436×90); looks different (capture difference 18.8) | 436×90 (dialog) | Message box font, as 1-CreateDB. |
| Database/mdb/5-DelRecord.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | compile error: Missing parameter(s) |  |
| Database/mdb/6-ViewRecord.bas | differs: window 1 414×78 (RC.EXE 436×90); looks different (capture difference 16.3) | differs: window 1 416×78 (RC.EXE 436×90); looks different (capture difference 18.8) | 436×90 (dialog) | Message box font, as 1-CreateDB. |
| Database/mdb/7-FormRecord.bas | differs: 2 window(s) (RC.EXE 1); window 1 348×199 (RC.EXE 436×90); looks different (capture difference 22.2) | differs: 2 window(s) (RC.EXE 1); window 1 348×199 (RC.EXE 436×90); looks different (capture difference 22.2) | 436×90 (dialog) | RC.EXE's run had no Test.mdb (only RapidR's runs made it, with 1-CreateDB): RC.EXE says so, RapidR shows the form. |
| Database/qDB2.bas | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | 420×239 (dialog) |  |
| Database/QODBC.bas | same (same output) | same (same output) | console |  |
| Database/SQL_blobs/archive.bas | same (same output) | fails: compile error: invalid left-hand side of assignment | console |  |
| Database/SQL_blobs/hexconvert.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got ptr | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "EXPLICIT" |  |
| Database/SQL_blobs/SQ3.BAS | RC.EXE fails; RapidR: compile error: Unexpected character: / | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got """ |  |
| devices/joystick.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console, runs until stopped |  |
| devices/LPT_IO/LPT.rqw | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| dialogs/colordlg/colordlg.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: Undefined symbol PICTURE1 |  |
| dialogs/colordlg/RGBV11.BAS | same (same windows, RapidR's form frame, difference 4.2) | same (same windows, RapidR's form frame, difference 4.2) | 294×351 |  |
| dialogs/printdlg/printdlg.bas | not compared: its first window is Windows' 'Page Setup' dialog, which the test run answers (not compared) | not compared: its first window is Windows' 'Page Setup' dialog, which the test run answers (not compared) | 360×390 (dialog) |  |
| direct3d/3d_clock/3d_orologio.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: Display mode cannot be changed (640x480 16bit) |  |
| direct3d/3dConvert/xview2.bas | same (same windows, RapidR's form frame, difference 7.4) | same (same windows, RapidR's form frame, difference 7.4) | 624×441 |  |
| direct3d/3DPong_aDelic2.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| direct3d/3DXplusV2_2.bas | RC.EXE fails; RapidR: runs until stopped | RC.EXE fails; RapidR: runs, 1 window(s) | error box: Display mode cannot be changed (640x480 16bit) |  |
| direct3d/alphablend/D3D.BAS | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| direct3d/billboard_ex.bas | RC.EXE fails; RapidR: compile error: Syntax error: unexpected '/' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "/" |  |
| direct3d/circularScreen/Circular3DScreen.bas | differs: looks different (capture difference 14.5) | differs: looks different (capture difference 28.2) | 624×441 |  |
| direct3d/lights_motion.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | error box: D3DRM.dll not loaded |  |
| direct3d/Lights_pyramid.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| direct3d/Lights_terrain.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | compile error: $INCLUDE file \rapidq\include\RapidQ_D3D.INC does not exist |  |
| direct3d/RQ_3DTerrain.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| direct3d/RQ_3DTerrain/Terrain16.bas | fails: compile error: Syntax error in END statement | fails: compile error (as interpreted) | builds (not run: deletes or moves files) | An IF left open at END SUB, closed by an extra END IF in the next SUB: RC.EXE keeps one stack per kind of block across SUBs; RapidR stops at the END IF. |
| direct3d/RQ_3DTerrain/TraceBitmapForm.bas | same (same windows, RapidR's form frame, difference 5.4) | same (same windows, RapidR's form frame, difference 5.4) | 492×292 |  |
| direct3d/shadows/SHADOW.BAS | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| direct3d/smooth_move.bas | differs: window 1 400×600 (RC.EXE 410×610) | differs: window 1 400×600 (RC.EXE 410×610) | 410×610 | ClientWidth / ClientHeight, then BorderStyle = bsDialog: RC.EXE keeps the window's outer size, so on Windows 11 its inside grows by the frames' difference (10); RapidR keeps the inside the program set. |
| direct3d/wrap/WRAP.BAS | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| direct3d/xview/xview.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| direct3d/xview/xview2.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: D3DRM.dll not loaded |  |
| directx/3dcube.bas | differs: looks different (capture difference 14.7) | differs: looks different (capture difference 14.5) | 304×201 |  |
| directx/dxball.bas | same (same windows, difference 1.0) | same (same windows, difference 1.0) | 320×200 |  |
| directx/scroll/dx.bas | differs: window 1 640×480 (RC.EXE 650×490) | differs: window 1 640×480 (RC.EXE 650×490) | 650×490 | As direct3d/smooth_move (ClientWidth, then bsDialog). |
| directx/shooter/shooter.bas | same (same windows, difference 1.9) | same (same windows, difference 1.9) | 400×300 |  |
| directx/starfield/stars.bas | same (same windows, difference 0.3) | same (same windows, difference 0.3) | 640×480 |  |
| dll/GetDllsFuncList.Bas | not run: builds (not run: writes the registry through Windows) | not run: builds (not run: writes the registry through Windows) | builds (not run: writes the registry through Windows) |  |
| dll/Resource in DLL/emptyDLL.bas | RC.EXE fails; RapidR: compile error: Unknown SUB or FUNCTION 'export' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected end-of-line but got EXPORT |  |
| dll/Resource in DLL/TesBmpDLL.rqw | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| dll/simple/DLL.BAS | fails: run-time error: 'PASCAL.DLL' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DLL | fails: run-time error: 'PASCAL.DLL' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DLL | 231×101 (dialog) | Its PASCAL.DLL is 32-bit; RapidR programs are 64-bit. |
| Encryption/encoding.rqb | same (same windows, RapidR's form frame, difference 3.2) | same (same windows, RapidR's form frame, difference 3.2) | 319×204 |  |
| Encryption/encrypt/encrypt.rqb | same (same windows, RapidR's form frame, difference 1.7) | same (same windows, RapidR's form frame, difference 1.7) | 304×201 |  |
| Encryption/encrypt/quizzer2.rqb | same (same windows, RapidR's form frame, difference 0.9) | same (same windows, RapidR's form frame, difference 0.9) | 784×461 |  |
| Encryption/padlock.bas | same (same windows, RapidR's form frame, difference 2.0) | same (same windows, RapidR's form frame, difference 2.0) | 238×83 |  |
| Encryption/RC4.bas | same (same windows, RapidR's form frame, difference 4.7) | same (same windows, RapidR's form frame, difference 4.7) | 429×393 |  |
| Encryption/secure memo.bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 584×341 |  |
| files/Conceal_File.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| files/dirtree/dirtree.bas | differs: looks different (capture difference 16.6) | differs: looks different (capture difference 16.6) | 304×201 | RapidR's QDIRTREE gave every closed folder a + (fixed: only folders with folders in them, as RC.EXE's). |
| files/DiskVolumeInfo.bas | differs: window 1 99×78 (RC.EXE 105×90); looks different (capture difference 32.3) | differs: window 1 99×78 (RC.EXE 105×90); looks different (capture difference 32.3) | 105×90 (dialog) | Message box font, as Database/mdb/1-CreateDB. |
| files/FileSearch.bas | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | builds (not run: writes the registry) |  |
| files/FileSearch_FB_DLL.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got PTR | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected ( but got "ALIAS" |  |
| files/GetShortFileName.bas | same (same output) | same (same output) | console |  |
| files/launcher/filelist.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| files/QFolder/QFolder.bas | same (same windows, RapidR's form frame, difference 3.9) | same (same windows, RapidR's form frame, difference 3.9) | 751×597 |  |
| files/Read_all_Files_IN_Dir.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console, runs until stopped |  |
| FolderTree/QFolderTree.rqb | differs: didn't end at the capture (a wait outside the message loop?) | differs: didn't end at the capture (a wait outside the message loop?) | 474×334 | No window before the capture. Its folder list asks DIR$ and QFILELISTBOX for names without an extension (a pattern ending in a dot): RapidR read the dot literally, and listed Windows' hidden and system folders (fixed). |
| Fonts/add & remove font.bas | not run: builds (not run: installs fonts) | not run: builds (not run: installs fonts) | builds (not run: installs fonts) |  |
| Fonts/VECTOR_Font/VECTOR.BAS | same (same windows, RapidR's form frame, difference 5.7) | same (same windows, RapidR's form frame, difference 5.7) | 617×434 |  |
| forms/AlwaysOnTop.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×181 |  |
| forms/Cancel Form Close.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| forms/closeQ/close.bas | same (same windows, difference 0.0) | same (same windows, difference 0.7) | 400×299 |  |
| forms/custom/custom.bas | same (same windows, difference 1.2) | same (same windows, difference 1.2) | 320×240 |  |
| forms/deskform/deskform.bas | differs: looks different (capture difference 233.6) | differs: looks different (capture difference 233.6) | 304×201 | PaintDesktop on the form's DC: RapidR repaints its form over it (docs/windows-dll-calls.md §3). |
| forms/dialogFRM.bas | same (same windows, RapidR's form frame, difference 0.3) | same (same windows, RapidR's form frame, difference 0.3) | 1054×630 |  |
| forms/dragdrop/dragdrop.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| forms/findWinTest.bas | same (same windows, RapidR's form frame, difference 6.1) | same (same windows, RapidR's form frame, difference 6.1) | 304×201 |  |
| forms/Form titlebar buttons.bas | same (same windows, RapidR's form frame, difference 3.0) | same (same windows, RapidR's form frame, difference 3.0) | 304×201 |  |
| forms/FrmOnClose.bas | same (same windows, RapidR's form frame, difference 0.4) | same (same windows, RapidR's form frame, difference 0.4) | 304×201 |  |
| forms/IEWindowMaximizer.BAS | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit None) |  |
| forms/LockWin.bas | same (same windows, RapidR's form frame, difference 5.1) | same (same windows, RapidR's form frame, difference 5.1) | 334×161 |  |
| forms/MenuBarColor.bas | not run: builds (not run: changes system settings) | not run: builds (not run: changes system settings) | builds (not run: changes system settings) |  |
| forms/minimize to taskbar0.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| forms/MinToTaskbar.bas | fails: run-time error: 'SetWindowLongA' is given CODEPTR(WinProc), a callback the DLL would call back into the program; RapidR doesn' | fails: run-time error: 'SetWindowLongA' is given CODEPTR(WinProc), a callback the DLL would call back into the program; RapidR doesn' | 304×201 |  |
| forms/multform/multform.bas | differs: looks different (capture difference 13.0) | differs: looks different (capture difference 13.0) | 184×41, 304×201 |  |
| forms/MultiCaptiveChildWnds.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit 0) |  |
| forms/newform/newform.bas | same (same windows, RapidR's form frame, difference 4.3) | same (same windows, RapidR's form frame, difference 4.4) | 304×201 |  |
| forms/previnst/previnst.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| forms/purewindows/wnd.bas | differs: didn't end at the capture (a wait outside the message loop?) | differs: didn't end at the capture (a wait outside the message loop?) | 304×201 | A window of its own (RegisterClass, CreateWindowEx, a CODEPTR window procedure, its own message loop): callbacks aren't supported. |
| forms/Qeform.bas | differs: looks different (capture difference 36.4) | differs: looks different (capture difference 36.4) | 314×211 | SetWindowRgn: RapidR's window stays a rectangle (docs/windows-dll-calls.md §3). |
| forms/QrForm.bas | same (same windows, RapidR's form frame, difference 0.2) | same (same windows, RapidR's form frame, difference 0.2) | 314×211 |  |
| forms/QTabControl/QTabVertical.bas | fails: run-time error: the call to SendMessageA in user32 crashed (access violation reading address 0xffffffff0000000b); an argument  | fails: run-time error: the call to SendMessageA in user32 crashed (access violation reading address 0xffffffff0000000b); an argument  | 304×201 | SendMessage to a control's Handle with a 32-bit pointer in a LONG (TCITEM): RapidR's controls have no window, and the pointer doesn't fit. |
| forms/setfocus.bas | same (same windows, RapidR's form frame, difference 3.4) | same (same windows, RapidR's form frame, difference 3.4) | 154×131 |  |
| forms/titlebtn.bas | fails: run-time error: 'SetWindowLongA' is given CODEPTR(Form_WndProc), a callback the DLL would call back into the program; RapidR d | fails: run-time error: 'SetWindowLongA' is given CODEPTR(Form_WndProc), a callback the DLL would call back into the program; RapidR d | 304×201 |  |
| forms/Two Windows.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 334×241 |  |
| forms/Window Sizer.BAS | fails: compile error: Syntax error in ELSE statement | fails: compile error (as interpreted) | no window (exit None) | A second ELSE in one IF: RC.EXE compiles it and never runs what follows (fixed). |
| forms/wndproc/winex.bas | differs: looks different (capture difference 17.8) | differs: looks different (capture difference 17.8) | 304×201 | Controls made with CreateWindowEx on the form's window: RapidR draws its form over them. |
| forms/wndproc/winexPanel.bas | same (same windows, RapidR's form frame, difference 0.6) | same (same windows, RapidR's form frame, difference 0.6) | 484×461 |  |
| FreeImage/FreeImage.rqb | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: EOleSysError exe at 00051BE9. Invalid class string. |  |
| FreeImage/MFreeImage.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got STRETCH_MODE | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "VB_NAME" |  |
| games/asteroids.bas | fails: compile error: Unknown SUB or FUNCTION 'shoy' | fails: compile error (as interpreted) | 1606×1004, 209×208 | A typo calls an undefined name (shoy for shot): RC.EXE compiles a call of an unknown name as 0; RapidR stops with "Unknown SUB or FUNCTION". |
| games/ateroids2/asteroids.bas | fails: compile error: Unknown SUB or FUNCTION 'shoy' | fails: compile error (as interpreted) | 1606×1004, 219×218 | As games/asteroids. |
| games/ateroids2/asteroids.his.bas | same (same windows, RapidR's form frame, difference 2.9) | same (same windows, RapidR's form frame, difference 1.6) | 304×201 |  |
| games/battle/BATTLE.BAS | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| games/blocks/blocks.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | harness error |  |
| games/columns/COLUMNS.BAS | same (same windows, difference 4.7) | same (same windows, difference 3.2) | 220×260 |  |
| games/eye_game.bas | RC.EXE fails; RapidR: compile error: $RESOURCE steen0: file not found: 'leeg.bmp' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: RESOURCE file leeg.bmp does not exist |  |
| games/fiar/FIAR.BAS | same (runs until stopped, as RC.EXE's) | same (runs until stopped, as RC.EXE's) | console, runs until stopped |  |
| games/flashgames.bas | RC.EXE fails; RapidR: compile error: flashgames.bas:1:1: ICON file FACE03.ico does not exist. | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: ICON file FACE03.ico does not exist. |  |
| games/Jawbreaker/JawbreakE.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| games/Jawbreaker/JawbreakG.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| games/msweep/msweep.bas | same (same windows, difference 0.0) | same (same windows, difference 0.0) | 299×154 |  |
| games/othello/othello.bas | same (same windows, RapidR's form frame, difference 0.8) | same (same windows, RapidR's form frame, difference 0.8) | 417×239 |  |
| games/puzzle/puzzle.bas | differs: window 1 197×233 (RC.EXE 207×223); looks different (capture difference 78.1) | differs: window 1 197×233 (RC.EXE 207×223); looks different (capture difference 79.0) | 207×223 | ClientWidth / ClientHeight set before its main menu: RapidR's menu is inside the window (28) and leaves less room, so scroll bars show; RC.EXE's inside is 207 × 223 on Windows 11. |
| games/qmorp/QMORP.BAS | same (same windows, RapidR's form frame, difference 1.2) | same (same windows, RapidR's form frame, difference 1.2) | 634×426 |  |
| games/shisen/shisen.bas | differs: looks different (capture difference 23.2) | differs: looks different (capture difference 22.5) | 800×549 | Random tiles; RapidR's in-window menu moves the board down. |
| games/Sokoban/Sokoban_1.bas | same (same windows, difference 4.7) | same (same windows, difference 4.7) | 272×272 |  |
| games/Sokoban/Sokoban_2.bas | same (same windows, difference 1.3) | same (same windows, difference 1.3) | 270×270 |  |
| games/Sokoban/Sokoban_3.bas | same (same windows, difference 6.6) | same (same windows, difference 6.6) | 270×270 |  |
| games/Sokoban/Sokoban_GB.bas | same (same windows, difference 2.1) | same (same windows, difference 2.1) | 270×270 |  |
| games/Sokoban/SokobanBuilder.bas | differs: window 1 350×300 (RC.EXE 350×261) | differs: window 1 350×300 (RC.EXE 350×261) | 350×261 | Form frame: RC.EXE's inside is smaller on Windows 11. |
| games/WIP_asteroids3D.bas | RC.EXE fails; RapidR: compile error: Unknown SUB or FUNCTION 'shoy' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "(" |  |
| Gauge/CoolGauge.bas | differs: looks different (capture difference 45.3) | differs: looks different (capture difference 44.0) | 384×161 | RC.EXE's capture of its canvas is black; RapidR draws the bars. |
| graphics/AlphaTest/AlphaTest1.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'XpGroupBox.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file XpGroupBox.inc does not exist |  |
| graphics/AlphaTest/AlphaTest2.bas | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | fails: run-time error: 'CallWindowProcA' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's w | 784×531 |  |
| graphics/bitblt.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: EFOpenError exe at 0000F5E1. Cannot open file rq.bmp. |  |
| graphics/Choosecolor.bas | differs: looks different (capture difference 35.1) | differs: looks different (capture difference 35.1) | 242×11 | A form 50 high: what fits inside differs with the frame (RapidR shows the form's scroll bar). |
| graphics/credits/credits.bas | differs: looks different (capture difference 12.2) | differs: looks different (capture difference 12.2) | 384×161 | Scrolling text: the moment of the capture. |
| graphics/Gamma.bas | differs: looks different (capture difference 13.3) | differs: looks different (capture difference 13.3) | 684×561 | Gamma ramp: the capture's moment and colours. |
| graphics/GDI_p.bas | differs: window 1 129×78 (RC.EXE 566×437); looks different (capture difference 24.4) | differs: window 1 129×78 (RC.EXE 566×437); looks different (capture difference 24.4) | 566×437 | GdiplusStartup is given VARPTR of a TYPE through a LONG: under RapidQ VARPTR of a TYPE isn't an address (RC.EXE: 1 here, GDI+'s version by accident); RapidR's is: error 17 (docs/windows-dll-calls.md). |
| graphics/image_test.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\rapidq2.inc does not exist |  |
| graphics/jpeg_Blend.bas | same (same windows, RapidR's form frame, difference 0.0) | fails: compile error: the name `loadimage` is defined multiple times | 1590×937 |  |
| graphics/LoadJPG.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 384×361 |  |
| graphics/MaxRGB.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 654×483 |  |
| graphics/pcx/PCXload.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 587×385 |  |
| graphics/Picview.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\rapidq.inc does not exist |  |
| graphics/polybez.rq | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| graphics/PolyDrawLineTo.bas | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| graphics/Qcanvas2BMP.bas | same (same windows, RapidR's form frame, difference 1.4) | same (same windows, RapidR's form frame, difference 1.4) | 504×481 |  |
| graphics/rgb.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: Member ONCLICK of PAINTBOX expected an Event but got PAINTBOX_CLICK |  |
| graphics/rotate/rotate.bas | RC.EXE fails; RapidR: compile error: $RESOURCE BMP_RES: file not found: 'rapidq.bmp' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: RESOURCE file rapidq.bmp does not exist |  |
| graphics/vfont/VECTOR.BAS | same (same windows, RapidR's form frame, difference 5.7) | same (same windows, RapidR's form frame, difference 5.7) | 617×434 |  |
| grids/chkgrid.bas | same (same windows, RapidR's form frame, difference 1.6) | same (same windows, RapidR's form frame, difference 1.6) | 304×201 |  |
| grids/column.bas | same (same windows, RapidR's form frame, difference 2.1) | same (same windows, RapidR's form frame, difference 2.1) | 379×245 |  |
| grids/datagrid.BAS | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | error box: EFOpenError exe at 0000F5E1. Cannot open file DG.ICO. |  |
| grids/goRangeSelectGrid.bas | same (same windows, RapidR's form frame, difference 1.7) | same (same windows, RapidR's form frame, difference 1.7) | 584×411 |  |
| grids/listgrid.bas | same (same windows, RapidR's form frame, difference 1.7) | same (same windows, RapidR's form frame, difference 1.7) | 304×201 |  |
| grids/ManageGridEditing.bas | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | 424×271 |  |
| grids/mergeGrid.bas | same (same windows, RapidR's form frame, difference 0.8) | same (same windows, RapidR's form frame, difference 0.8) | 784×561 |  |
| grids/QGridEx/QGridExDemo.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'QDebug2.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file QDebug2.inc does not exist |  |
| grids/QStringGridsTwoLinesBitMap.Bas | same (same windows, RapidR's form frame, difference 1.4) | same (same windows, RapidR's form frame, difference 1.4) | 784×441 |  |
| grids/stringGridOnColClick.bas | same (same windows, RapidR's form frame, difference 1.5) | same (same windows, RapidR's form frame, difference 1.5) | 384×261 |  |
| html/HTMLColor.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| html/QHTML/QHTML example.bas | RC.EXE fails; RapidR: compile error: Syntax error in DIM statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\examples\network\qhtml\QHTML.RQO does not exist |  |
| html/QHTML/QHTMLexample.bas | RC.EXE fails; RapidR: compile error: Syntax error in DIM statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\examples\network\qhtml\QHTML.RQO does not exist |  |
| html/rqb2html.bas | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | 420×239 (dialog) |  |
| imageLibrary/mgplist.bas | same (same windows, difference 1.2) | same (same windows, difference 1.2) | 183×94 |  |
| io/IO_RQ.bas | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | compile error: Undefined symbol LPTINPORTADDR |  |
| IPicture/RQ_IPictureDemo.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×181 |  |
| IPicture/RQ_IPictureDemo2.bas | fails: run-time error: address -773849088 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: run-time error: address -773849088 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | 632×466 |  |
| keyboard/CodeKeyPad/RQKeypad - 02.rqw | not run: builds (not run: injects or hooks input) | not run: builds (not run: injects or hooks input) | builds (not run: injects or hooks input) |  |
| keyboard/ExamineKeys.bas | same (same windows, RapidR's form frame, difference 9.7) | same (same windows, RapidR's form frame, difference 9.7) | 214×101 |  |
| keyboard/hook.bas | RC.EXE fails; RapidR: compile error: Unknown SUB or FUNCTION 'that' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got end-of-line |  |
| keyboard/hotkeys/hotkeys.bas | differs: window 1 210×78 (RC.EXE 304×201) | differs: window 1 210×78 (RC.EXE 304×201) | 304×201 | RegisterHotKey with the form's Handle before it shows (RapidR's form has no window yet), and WM_HOTKEY through a WndProc: the program says the key is taken (docs/windows-dll-calls.md §3). |
| keyboard/KEYMAP1a.bas | same (same windows, RapidR's form frame, difference 10.4) | same (same windows, RapidR's form frame, difference 10.4) | 227×44 |  |
| keyboard/sendkeys_to_app.bas | not run: builds (not run: injects or hooks input) | not run: builds (not run: injects or hooks input) | builds (not run: injects or hooks input) |  |
| labels/SuperGrid.bas | same (same windows, RapidR's form frame, difference 1.7) | same (same windows, RapidR's form frame, difference 1.7) | 404×301 |  |
| Linear_Solver/crap.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Linear_Solver/Linear.bas | RC.EXE fails; RapidR: compile error: Syntax error in IF statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Undefined symbol MATRIXSIZE |  |
| listbox/multi/multi.bas | same (same windows, RapidR's form frame, difference 9.3) | same (same windows, RapidR's form frame, difference 9.3) | 317×234 |  |
| listbox/owndraw/listbox.bas | same (same windows, RapidR's form frame, difference 7.9) | same (same windows, RapidR's form frame, difference 7.9) | 304×201 |  |
| listview/extended/eview.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\RAPIDQ.INC does not exist |  |
| listview/zipview/zipview.bas | same (same windows, RapidR's form frame, difference 4.3) | same (same windows, RapidR's form frame, difference 4.3) | 509×271 |  |
| Math/Linear_Solver/Linear.bas | RC.EXE fails; RapidR: compile error: Syntax error in IF statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Undefined symbol MATRIXSIZE |  |
| Math/mathfunc.bas | same (same output) | same (same output) | console |  |
| Math/ProbCalc.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'C:\rapidq\qstat\ProbDists.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file C:\rapidq\qstat\ProbDists.inc does not exist |  |
| Math/ProbDists.rqb | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Math/quest.bas | RC.EXE fails; RapidR: compile error: Syntax error near 'S' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected AS but got , |  |
| menu/EasyMenu.bas | same (same windows, RapidR's form frame, difference 3.3) | same (same windows, RapidR's form frame, difference 3.3) | 304×181 |  |
| menu/ownerdraw/menu.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×181 |  |
| Mouse/Mouse Position.bas | same (same windows, RapidR's form frame, difference 3.3) | same (same windows, RapidR's form frame, difference 3.3) | 304×201 |  |
| Mouse/Mouse_Wheel.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\windows.inc does not exist |  |
| Mouse/MoveMouse.bas | differs: window 1 62×33 (RC.EXE 103×25); looks different (capture difference 28.8) | differs: window 1 62×33 (RC.EXE 103×25); looks different (capture difference 28.8) | 103×25 | As Mouse/select2 (Windows' smallest window). |
| Mouse/Right Click.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| Mouse/select2.bas.bas | differs: window 1 98×139 (RC.EXE 120×131) | differs: window 1 98×139 (RC.EXE 120×131) | 120×131 | A form narrower than Windows' smallest window: Windows widens RC.EXE's; RapidR's keeps the program's Width. |
| Mouse/select2dx.bas | same (same windows, difference 0.2) | same (same windows, difference 0.2) | 640×480 |  |
| music/BDX-8/BDX-8.rqb | not run: builds (not run: deletes or moves files) | fails: compile error: no field `width` on type `rapidr_runtime_core::prelude::Value` | builds (not run: deletes or moves files) |  |
| music/GMDrum.rqb | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit None) |  |
| music/GMDrum1.1 (Davids Improvements).rqb | differs: looks different (capture difference 18.0) | differs: window 1 1×1 (RC.EXE 660×535) | 660×535 | Timing and the capture's moment (a drum machine). |
| music/GMSynth.rqb | differs: looks different (capture difference 14.0) | differs: window 1 1×1 (RC.EXE 804×428) | 804×428 | It reads its settings from COMMAND$(0) with ".exe" replaced by ".gms": run by the interpreter COMMAND$(0) is the .rqb, so it starts empty (native: as RC.EXE). |
| music/PCM-Q2.rqb | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| music/StutterBox.rqb | fails: compile error: Syntax error in FOR statement | fails: compile error (as interpreted) | 549×336 | A FOR with no start value: RC.EXE counts from 0 (fixed). |
| music/VisMetronome/Metronome.rqb | same (same windows, RapidR's form frame, difference 2.2) | differs: window 1 1×1 (RC.EXE 925×611) | 925×611 |  |
| music/Wav-Scratcher.rqb | same (same windows, RapidR's form frame, difference 6.6) | differs: window 1 1×1 (RC.EXE 369×251) | 369×251 |  |
| music/WaveRec&Play.rqb | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| mysql/rqlibsql/mySQL_API.bas | fails: compile error: Syntax error in END statement | fails: compile error (as interpreted) | 115×105 (dialog) | An END IF inside a FOR closing an IF opened before it: RC.EXE accepts the misnesting; RapidR stops at the END IF. |
| mysql/rqlibsql/MySQL_Qlistview.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'qodbc.bas' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file qodbc.bas does not exist |  |
| mysql/rqlibsql/sql.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit 3221225781) |  |
| mysql/SQLServerTutorial/create Taxes.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 3221225781) |  |
| mysql/SQLServerTutorial/sql.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit 3221225781) |  |
| mysql/SQLServerTutorial/Taxes.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 3221225781) |  |
| Network/bind port to IP.bas | not run: builds (not run: opens a server socket) | not run: builds (not run: opens a server socket) | builds (not run: opens a server socket) |  |
| Network/BlockingQSocket.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Network/Download/qdownload.bas | not compared: runs, 1 window(s) | not compared: runs, 1 window(s) | builds (not run: writes the registry) |  |
| Network/ftp/ChangeMyWebPage.Bas | RC.EXE fails; RapidR: compile error: Undefined symbol IDOK | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Undefined symbol IDOK |  |
| Network/ftp/FTPBASIC.BAS | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console, runs until stopped |  |
| Network/ftp/QFTPCLIENT.BAS | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| Network/ftp/QFTPGUITEST.BAS | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| Network/ftp/QListView dir and files.bas | RC.EXE fails; RapidR: compile error: Syntax error near 'AddText' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: GRIDSERVER is not a type identifier |  |
| Network/HTMLColor.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Network/httppost.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Network/InetIsOffline.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console |  |
| Network/Internet Connection.bas | RC.EXE fails; RapidR: compile error: Syntax error in END statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown type ANY |  |
| Network/Ip_Message.bas | differs: 2 window(s) (RC.EXE 1); window 1 320×240 (RC.EXE 173×105); looks different (capture difference 15.1) | differs: 2 window(s) (RC.EXE 1); window 1 320×240 (RC.EXE 173×105); looks different (capture difference 15.1) | 173×105 (dialog) | As sockets/Ip_Message. |
| Network/MIME_Coding.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| Network/QHTML/QHTML example.bas | RC.EXE fails; RapidR: compile error: Syntax error in DIM statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\examples\network\qhtml\QHTML.RQO does not exist |  |
| Network/QHTML/QHTMLexample.bas | RC.EXE fails; RapidR: compile error: Syntax error in DIM statement | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\examples\network\qhtml\QHTML.RQO does not exist |  |
| Network/ra1d.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got AS | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown type LO |  |
| Network/Server_Client95/clientSocket.BAS | not run: builds (not run: writes the registry through Windows) | not run: builds (not run: writes the registry through Windows) | builds (not run: writes the registry through Windows) |  |
| Network/Server_Client95/serverSocket.BAS | not run: builds (not run: writes the registry through Windows) | not run: builds (not run: writes the registry through Windows) | builds (not run: writes the registry through Windows) |  |
| Network/tel.bas | same (same output (RC.EXE's, redirected to a file, stops short)) | same (same output (RC.EXE's, redirected to a file, stops short)) | console |  |
| Network/UDP.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\RAPIDQ.INC does not exist |  |
| Network/URLDownload.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console |  |
| Network/wsksock.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got ; | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "VB_NAME" |  |
| Object/Examples/Cd.bas | RC.EXE fails; RapidR: runs, 2 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file Object\Qcdaudio.inc does not exist |  |
| Object/Examples/colordlg1.bas | RC.EXE fails; RapidR: compile error: Include file not found: '\rapidq\examples\OBJECT\QCOLORDIALOG.INC' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\examples\OBJECT\QCOLORDIALOG.INC does not exist |  |
| Object/Examples/Editeur.bas | RC.EXE fails; RapidR: compile error: Include file not found: '\rapidq\examples\Object\QColorDialog.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \Rapidq\examples\Object\QTextEdit.inc does not exist |  |
| Object/Examples/IconLib.bas | RC.EXE fails; RapidR: compile error: Include file not found: '\rapidq\examples\Object\QCanvasEx.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: ICON file \rapidq\examples\Object\examples\IconLib.ico does not exist. |  |
| Object/Examples/midi.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file Object\QMidi.inc does not exist |  |
| Object/Examples/video.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file Object\QVideo.inc does not exist |  |
| Object/Examples/wave.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file Object\QWave.inc does not exist |  |
| OLE/clsid2iid.bas | same (same windows, RapidR's form frame, difference 2.4) | same (same windows, RapidR's form frame, difference 2.4) | 314×211 |  |
| OLE/Excel_ole.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| OLE/IE_ole.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| OLE/IE_sim.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| OLE/MSscript.bas | RC.EXE fails; RapidR: compile error: Unexpected character: % | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "(" |  |
| OLE/ole.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | error box: EOleSysError exe at 00030FA1. Invalid class string. |  |
| OLE/OLE_IExplorer_Demo.Bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| OLE/oleWebBrowser.bas | same (same windows, RapidR's form frame, difference 0.9) | same (same windows, RapidR's form frame, difference 0.9) | 384×361 |  |
| OLE/QOLEContainer IE.bas | fails: compile error: Expected end-of-line but got . | fails: compile error (as interpreted) | 384×361 | OLE (QOLECONTAINER): not supported. |
| OLE/xls/ExcelReader.bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 1004×781 |  |
| OpenGL/GL_test1.bas | fails: no window | fails: no window | 624×421 | It closes its form in OnShow when back.bmp is missing; under ShowModal RC.EXE's form stays (VCL resets ModalResult after OnShow): fixed. |
| OpenGL/GL_test2.bas | differs: 2 window(s) (RC.EXE 1); looks different (capture difference 187.5) | differs: 2 window(s) (RC.EXE 1); looks different (capture difference 187.5) | 624×441 | OpenGL through opengl32: RapidR's form is drawn over the GL surface (and its message box). |
| OpenGL/GL_test3.bas | differs: looks different (capture difference 179.5) | differs: looks different (capture difference 179.5) | 240×217 | As OpenGL/GL_test2. |
| OpenGL/GL_test4.bas | differs: 2 window(s) (RC.EXE 1); looks different (capture difference 95.4) | differs: 2 window(s) (RC.EXE 1); looks different (capture difference 163.9) | 624×441 | As OpenGL/GL_test2. |
| OpenGL/OGL_Demo1.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'gl.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file gl.inc does not exist |  |
| OpenGL/OGL_Demo2.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'glBMP.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \rapidq\include\Windows.inc does not exist |  |
| OpenGL/QGLobject.bas | RC.EXE fails; RapidR: compile error: Unknown data type QBITMAPEX | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown data type QBITMAPEX |  |
| OpenGL/QGLobject2.bas | RC.EXE fails; RapidR: compile error: Unknown data type QBITMAPEX | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown data type QBITMAPEX |  |
| OpenGL/QGLobject3.bas | RC.EXE fails; RapidR: compile error: Unknown data type QBITMAPEX | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown data type QBITMAPEX |  |
| Printer/lptport.bas | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| Printer/Print QBitmap.bas | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| Printer/printer.BAS | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| Printer/printqr.bas | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| Printer/selprn.bas | not run: builds (not run: prints) | not run: builds (not run: prints) | builds (not run: prints) |  |
| QAnimate/Test_QAnimate.rqw | not run: builds (not run: shuts down / locks / suspends the machine) | fails: compile error: expected function, found `&rapidr_runtime_core::prelude::Value` | builds (not run: shuts down / locks / suspends the machine) |  |
| qchart/QChartEx.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'Common.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: ICON file c:\rapidq\icon\Graph.ico does not exist. |  |
| QCursorListBox/TestQCursorListBox.bas | same (same windows, RapidR's form frame, difference 6.7) | same (same windows, RapidR's form frame, difference 6.7) | 384×270 |  |
| QCursorListBox/TestQCursorListBox.rqw | same (same windows, RapidR's form frame, difference 6.7) | same (same windows, RapidR's form frame, difference 6.7) | 384×270 |  |
| QDerulantPanel/QDerulantPanel_test.rqw | same (same windows, RapidR's form frame, difference 3.7) | fails: crash: exit 3221225595 | 602×428 | Native: the build stops as it starts (0xC000007B, a bad image); interpreted as RC.EXE. |
| QFlatButton/QFlatButton.Bas | not compared: runs, 1 window(s) | not compared: runs, 1 window(s) | builds (not run: writes the registry) |  |
| QGrid/TestQGrid..bas | same (same windows, RapidR's form frame, difference 8.0) | same (same windows, RapidR's form frame, difference 8.0) | 572×326 |  |
| QIcon/$noname.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'INC\QFuncLib.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: ICON file C:\TMP\IconLib.ico does not exist. |  |
| QResourceStream/example.rq | RC.EXE fails; RapidR: compile error: Include file not found: 'QResourceStream' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file QResourceStream does not exist |  |
| QResourceStream/QResourceStream.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| QResourceStream/RQ_ResourceView/ResourceView.rqw | fails: run-time error: address -773849087 is memory a DLL returned (a 64-bit pointer kept in a LONG), not the program's own: PEEK, PO | fails: compile error: expected function, found `&rapidr_runtime_core::prelude::Value` | 302×89 |  |
| QTabControl/QTabContol_add_del.bas | same (same windows, RapidR's form frame, difference 8.5) | fails: compile error: cannot find value `tab` in this scope | 324×281 | Native: an array of components DIMmed in a SUB isn't found by the generated Rust (`tab`); interpreted as RC.EXE. |
| QTabControl/TabIcon/Tab_Icon_Demo.bas | same (same windows, RapidR's form frame, difference 10.3) | same (same windows, RapidR's form frame, difference 10.3) | 314×191 |  |
| QTButton/QTButton_FullExamples.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: RESOURCE file C:\Rapidq\examples\QTButton\Boutons.bmp does not exist |  |
| QToolbar/OldStyle/Toolbar.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | error box: EAccessViolation exe at 0007F127. Access violation at address 0047F127 in module 'Toolbar.exe'. Read of addres |  |
| QToolbar/OldStyle/ToolTest.bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 624×421 |  |
| QToolbar/QReBar/ReBarTest.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| QToolbar/QToolBar2/TestQToolBar.bas | same (same windows, RapidR's form frame, difference 1.7) | same (same windows, RapidR's form frame, difference 1.4) | 461×187 |  |
| QToolbar/QToolBar2/TestQToolBar.rqw | same (same windows, RapidR's form frame, difference 1.4) | same (same windows, RapidR's form frame, difference 1.4) | 461×187 |  |
| QToolbar/Qtoolbar_Simple/QTtoolBar.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| QToolbar/QToolBar_vista/Test_QToolBar.rqw | not run: builds (not run: prints) | fails: compile error: expected function, found `&rapidr_runtime_core::prelude::Value` | builds (not run: prints) |  |
| QToolbar/QToolBarEx/MyToolBar.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| QUpDown/Test_QUpDown.rqw | not run: builds (not run: shuts down / locks / suspends the machine) | fails: compile error: expected function, found `&rapidr_runtime_core::prelude::Value` | builds (not run: shuts down / locks / suspends the machine) |  |
| Registry/Registry2.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| Registry/sqregistry.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| reminder/addrbook.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| reminder/dayfunction.bas | same (same windows, RapidR's form frame, difference 2.4) | same (same windows, RapidR's form frame, difference 2.4) | 374×313 |  |
| reminder/reminder.bas | RC.EXE fails; RapidR: compile error: Undefined symbol GWL_HWNDPARENT | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Undefined symbol GWL_HWNDPARENT |  |
| richedit/deTags.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | compile error: LABEL2 is not a type identifier |  |
| richedit/editor/editor.bas | same (same windows, RapidR's form frame, difference 0.3) | same (same windows, RapidR's form frame, difference 0.3) | 484×341 |  |
| richedit/Find.rqb | RC.EXE fails; RapidR: compile error: Include file not found: 'Error.rqb' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file Error.rqb does not exist |  |
| richedit/FindReplace.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 1590×917 |  |
| richedit/medit_test.bas | same (same windows, RapidR's form frame, difference 2.7) | same (same windows, RapidR's form frame, difference 2.7) | 304×201 |  |
| richedit/multilevelUndo.bas | same (same windows, RapidR's form frame, difference 0.4) | same (same windows, RapidR's form frame, difference 0.4) | 357×234 |  |
| richedit/QRedExDemo.Bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | no window (exit None) |  |
| richedit/SearchRich.bas | same (same windows, RapidR's form frame, difference 6.1) | same (same windows, RapidR's form frame, difference 6.1) | 304×161 |  |
| richedit/syntax/syntax.bas | differs: looks different (capture difference 14.8) | differs: looks different (capture difference 14.8) | 304×201 | In this run RRichEdit drew no formatting (SelAttributes): the code showed without its colours; rich text came after it (RICHEDIT). |
| richedit/using.bas | RC.EXE fails; RapidR: runs until stopped | RC.EXE fails; RapidR: runs until stopped | compile error: Expected ( but got ")" |  |
| sockets/bind port to IP.bas | not run: builds (not run: opens a server socket) | not run: builds (not run: opens a server socket) | builds (not run: opens a server socket) |  |
| sockets/BlockingQSocket.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| sockets/chat/chatcli.bas | same (same windows, RapidR's form frame, difference 3.1) | same (same windows, RapidR's form frame, difference 3.1) | 267×239 |  |
| sockets/chat/chatserv.bas | same (same windows, RapidR's form frame, difference 11.8) | same (same windows, RapidR's form frame, difference 11.8) | 367×271 |  |
| sockets/chat/chatserv_test.bas | same (same windows, RapidR's form frame, difference 3.4) | same (same windows, RapidR's form frame, difference 3.4) | 384×271 |  |
| sockets/download_YaGroups.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\RAPIDQ.INC does not exist |  |
| sockets/download_YaGroups2.bas | same (same windows, RapidR's form frame, difference 2.3) | same (same windows, RapidR's form frame, difference 2.3) | 607×424 |  |
| sockets/FD_File_Receiver.bas | differs: window 1 243×104 (RC.EXE 734×261); looks different (capture difference 38.4) | differs: window 1 243×104 (RC.EXE 734×261); looks different (capture difference 38.4) | 734×261 | WSAAsyncSelect on QSOCKET's socket and the form's WndProc: RapidR's sockets are its own, and window messages don't reach a WndProc (docs/windows-dll-calls.md §3). |
| sockets/FD_File_Sender.bas | same (same windows, RapidR's form frame, difference 7.0) | same (same windows, RapidR's form frame, difference 7.0) | 734×261 |  |
| sockets/gethttp/http.bas | same (same windows, RapidR's form frame, difference 1.3) | same (same windows, RapidR's form frame, difference 1.3) | 367×344 |  |
| sockets/getMyIp.bas | differs: window 1 99×78 (RC.EXE 99×90); looks different (capture difference 34.7) | differs: window 1 99×78 (RC.EXE 99×90); looks different (capture difference 34.7) | 99×90 (dialog) | QSOCKET's GetHostIP / GetHostName read without parentheses gave "" (fixed: they call the method); message box font, as Database/mdb/1-CreateDB. |
| sockets/getname.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'RSocket.INC' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file RSocket.INC does not exist |  |
| sockets/httppost.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| sockets/Ip_Message.bas | differs: 2 window(s) (RC.EXE 1); window 1 320×240 (RC.EXE 173×105); looks different (capture difference 15.1) | differs: 2 window(s) (RC.EXE 1); window 1 320×240 (RC.EXE 173×105); looks different (capture difference 15.1) | 173×105 (dialog) | GetHostIP / GetHostName without parentheses read "" (fixed); RC.EXE's capture leaves out its borderless form. |
| sockets/raw_sock.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'RSocket.INC' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file RSocket.INC does not exist |  |
| sockets/RQ_Ychat.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'QRichEdit.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: RESOURCE file YCH.ico does not exist |  |
| sockets/SimpleServer.bas | same (same output (runs until stopped)) | same (same output (runs until stopped)) | console, runs until stopped |  |
| sockets/tel.bas | same (same output (RC.EXE's, redirected to a file, stops short)) | same (same output (RC.EXE's, redirected to a file, stops short)) | console |  |
| sockets/trash.bas | same (same output) | same (same output) | console |  |
| sockets/UDP.bas | RC.EXE fails; RapidR: runs, 1 window(s) | RC.EXE fails; RapidR: runs, 1 window(s) | compile error: $INCLUDE file \rapidq\include\RAPIDQ.INC does not exist |  |
| sockets/wsksock.bas | RC.EXE fails; RapidR: compile error: Expected end-of-line but got ; | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected = but got "VB_NAME" |  |
| sound/gunsrose/GUNSROSE.BAS | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | error box: EPrivilege exe at 00036D4E. Privileged instruction. |  |
| sound/midi/playmidi.bas | differs: didn't end at the capture (a wait outside the message loop?) | differs: didn't end at the capture (a wait outside the message loop?) | 377×120 (dialog) | mciExecute shows Windows' own error box (no passport.mid on Windows 11) in both; RapidR's process is DPI-aware, so the box is drawn at the screen's scale. |
| sound/QDXSound.bas | same (same windows, RapidR's form frame, difference 1.6) | same (same windows, RapidR's form frame, difference 1.6) | 304×201 |  |
| sound/qdxsound2.bas | same (same windows, RapidR's form frame, difference 2.3) | same (same windows, RapidR's form frame, difference 2.3) | 314×211 |  |
| sound/record.bas | not compared: it prints; RC.EXE's output to a file is lost | not compared: it prints; RC.EXE's output to a file is lost | console, runs until stopped |  |
| sound/SBBAS.BAS | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | compile error: $INCLUDE file C:\RQBETA\SBTest\SBform.bas does not exist |  |
| sound/sbbas/SBBAS.BAS | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| sound/sbbas/SBFORM.BAS | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| sound/sbbas/SBLIB.BAS | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| sound/SBFORM.BAS | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| sound/SBLIB.BAS | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| sound/soundgenerator.bas | differs: window 1 120×85 (RC.EXE 130×95) | differs: window 1 120×85 (RC.EXE 130×95) | 130×95 | As direct3d/smooth_move (ClientWidth, then bsDialog). |
| sound/wavefile.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| spin/SpinControl.bas | same (same windows, RapidR's form frame, difference 0.2) | same (same windows, RapidR's form frame, difference 0.2) | 292×168 |  |
| spin/test.bas | same (same windows, RapidR's form frame, difference 1.5) | same (same windows, RapidR's form frame, difference 1.5) | 304×201 |  |
| Splitter/qfsplit/QFSplitter.bas | same (same windows, RapidR's form frame, difference 5.0) | same (same windows, RapidR's form frame, difference 5.0) | 297×206 |  |
| Splitter/Splitter.bas | differs: looks different (capture difference 12.6) | differs: looks different (capture difference 12.6) | 784×511 | Fonts and the frame. |
| Splitter/splitter13.bas | same (same windows, RapidR's form frame, difference 9.8) | same (same windows, RapidR's form frame, difference 9.8) | 784×561 |  |
| statusbar/STATUSBITMAPS.RQB | same (same windows, RapidR's form frame, difference 2.9) | same (same windows, RapidR's form frame, difference 2.9) | 474×461 |  |
| System/AsciiKrypt.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'WindowsAPI.Inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file WindowsAPI.Inc does not exist |  |
| System/computerOff/ComputerOff.BAS | not run: builds (not run: shuts down / locks / suspends the machine) | not run: builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) |  |
| System/CreateShellLink.bas | fails: run-time error: can't find the DLL 'STKIT432.DLL': it isn't one of Windows' and isn't beside the program (at CreateShellLink.b | fails: run-time error: can't find the DLL 'STKIT432.DLL': it isn't one of Windows' and isn't beside the program | 115×105 (dialog) | STKIT432.DLL (Visual Basic 4's setup kit) isn't on Windows 11. |
| System/GetFreeDrive_Example.BAS | RC.EXE fails; RapidR: compile error: Include file not found: 'WindowsAPI.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file WindowsAPI.inc does not exist |  |
| System/GetSystemMetrics.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'WindowsAPI.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file WindowsAPI.inc does not exist |  |
| System/NamesAndDocs_v_0_9beta.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'WindowsAPI.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file WindowsAPI.inc does not exist |  |
| System/ShellExecute.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| System/showhide.bas | differs: looks different (capture difference 22.6) | differs: looks different (capture difference 22.6) | 380×47 | Form frame: RC.EXE's smaller inside cuts its oval buttons. |
| System/shutdownTest.bas | not run: builds (not run: shuts down / locks / suspends the machine) | not run: builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) |  |
| System/SPYINFO3A.bas | differs: looks different (capture difference 23.9) | differs: looks different (capture difference 23.9) | 214×236 | Another window's coordinates from Windows: RapidR is DPI-aware (the VM's screen at 200%: 3212 × 2008), RC.EXE gets scaled ones (1606 × 1004). |
| System/test_api.bas | RC.EXE fails; RapidR: compile error: Include file not found: 'WindowsAPI.Inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file WindowsAPI.Inc does not exist |  |
| System/thread.bas | same (same windows, RapidR's form frame, difference 5.2) | same (same windows, RapidR's form frame, difference 5.2) | 304×201 |  |
| thread/microTimer.bas | not run: builds (not run: port I/O (hardware)) | not run: builds (not run: port I/O (hardware)) | builds (not run: port I/O (hardware)) |  |
| thread/Pipe.BAS | fails: run-time error: the call to WaitForSingleObject in kernel32 crashed (exception 0xC00000AA); an argument wasn't the pointer or  | fails: run-time error: the call to WaitForSingleObject in kernel32 crashed (exception 0xC00000AA); an argument wasn't the pointer or  | console, runs until stopped | CreateThread with a CODEPTR: callbacks aren't supported. |
| thread/previnst.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| thread/RQprocess/testProcessInfo.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| thread/RQprocess/testProcessLview.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| thread/run.bas | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| thread/SendKeys.bas | not run: builds (not run: injects or hooks input) | not run: builds (not run: injects or hooks input) | builds (not run: injects or hooks input) |  |
| thread/sendkeys_to_app.bas | not run: builds (not run: injects or hooks input) | not run: builds (not run: injects or hooks input) | builds (not run: injects or hooks input) |  |
| thread/sendMessage2.bas | same (same windows, RapidR's form frame, difference 6.5) | same (same windows, RapidR's form frame, difference 6.5) | 614×124 |  |
| thread/showhide.bas | differs: looks different (capture difference 22.6) | differs: looks different (capture difference 22.6) | 380×47 | As System/showhide. |
| thread/SPYINFO3A.bas | differs: looks different (capture difference 23.9) | differs: looks different (capture difference 23.9) | 214×236 | As System/SPYINFO3A. |
| thread/Talk_between_Windows.bas | RC.EXE fails; RapidR: compile error: Syntax error: unexpected '-' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Unknown identifier: - |  |
| thread/thread1.bas | same (same windows, RapidR's form frame, difference 5.2) | same (same windows, RapidR's form frame, difference 5.2) | 304×201 |  |
| Toolbar/QToolbar/QTtoolBar.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 304×201 |  |
| Toolbar/Toolbar.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | error box: EAccessViolation exe at 0007F127. Access violation at address 0047F127 in module 'Toolbar.exe'. Read of addres |  |
| Toolbar/ToolTest.bas | same (same windows, RapidR's form frame, difference 0.1) | same (same windows, RapidR's form frame, difference 0.1) | 624×421 |  |
| USBinfo/GetUSBinfo.rqb | not run: builds (not run: starts other programs) | not run: builds (not run: starts other programs) | builds (not run: starts other programs) |  |
| VideoCapture/Direct_Show_ex.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 484×461 |  |
| VideoCapture/DirectShow_test.bas | RC.EXE fails; RapidR: compile error: Unexpected character: ! | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: In CONST undeclared identifier WM_USER |  |
| VideoCapture/VideoCapture.bas | same (same windows, RapidR's form frame, difference 0.0) | same (same windows, RapidR's form frame, difference 0.0) | 584×741 |  |
| VideoCapture/VideoPlayer.bas | RC.EXE fails; RapidR: compile error: $RESOURCE PLAYBMP: file not found: 'play.bmp' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file \Rapidq\include\Rapidq.inc does not exist |  |
| VideoCapture/WebCam2.BAS | RC.EXE fails; RapidR: compile error: Include file not found: 'rapidq2_2.inc' | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: $INCLUDE file rapidq2_2.inc does not exist |  |
| VideoCapture/WebCam_Class.bas | RC.EXE fails; RapidR: runs | RC.EXE fails; RapidR: runs | no window (exit 0) |  |
| weatherQ/WeatherQ.bas | not run: builds (not run: deletes or moves files) | not run: builds (not run: deletes or moves files) | builds (not run: deletes or moves files) |  |
| zlib/QunZip.bas | RC.EXE fails; RapidR: compile error: Undefined symbol VBNEWLINE | RC.EXE fails; RapidR: compile error (as interpreted) | compile error: Expected ALIAS but got "(" |  |
| zlib/z_test.bas | differs: window 1 111×78 (RC.EXE 120×90); looks different (capture difference 38.9) | differs: window 1 111×78 (RC.EXE 120×90); looks different (capture difference 38.9) | 120×90 (dialog) | Message box font, as Database/mdb/1-CreateDB. |
| zlib/zunzip.bas | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | not compared: its first window is Windows' 'Open' dialog, which the test run answers (not compared) | 420×239 (dialog) |  |
<!-- corpus-win:end -->

### How to run it again

In the Windows VM, with an installed RapidR (or one staged as the installer
lays it out) and a copy of RapidQ's folder (`RC.EXE`, `Lib\`, `include\`,
`examples\`) — RapidQ's files are never part of this repository:

```
python tools\windows\corpus_win.py --rapidr <RapidR>\bin\rapidr.exe ^
    --examples <copy>\examples --rq <copy> --out <folder>
python tools\windows\corpus_win.py ... --no-rc --no-native --rc-from <folder> --out <folder2>   (interpreted only)
python3 tools/windows/corpus_win_report.py <folder>/results.jsonl [<folder2>/results.jsonl …] --doc docs/rapidq-corpus-status.md
```

Programs that would touch the machine are only compiled (see "not run");
RapidR's printing goes to `RAPIDR_PRINT_TO` and its QREGISTRY to
`RAPIDR_REGISTRY`; every process is ended by its own PID.
