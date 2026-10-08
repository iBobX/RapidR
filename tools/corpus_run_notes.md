## What the sweep found, and what changed

Each cause below was checked against RC.EXE in the Windows VM (`tools/rc_probe.sh`, `tools/corpus_rc_shots.sh`) before RapidR changed, on both backends and the web where the component exists there; the conformance cases named hold it.

| Cause | Programs | Now | Case |
|---|---|---|---|
| `SLEEP .1` read as a member `SLEEP.1` (a space before `.1`): no pause, a warning on every turn | `console/movetext` | a number after a space is a number | `rapidq_sleep_fraction` |
| A program's own `TYPE QToolBar EXTENDS QPANEL` taken for RapidR's RTOOLBAR (RapidQ has no QTOOLBAR) | `QToolbar/Qtoolbar_Simple`, `Toolbar/QToolbar` | the program's TYPE | `rapidq_own_type_names` |
| `Font.AddStyles` / `DelStyles` on a component (and inside its CREATE), `Font.AddStyles = n`; styles read -1 | `games/ateroids2`, `games/Sokoban/*` | the styles change, read 1 / 0; ParentFont ends at a component's first font change | `rapidq_font_styles`, `rapidq_font_styles_assign` |
| OnPaint for a new size fired at once, before the program made what it draws | `Splitter/Splitter` | posted to the message loop, and only once the form has shown | — (GUI: `canvas_onpaint`) |
| No OnResize when a form first shows | `Gauge/CoolGauge`, `graphics/credits` | OnResize, OnShow, OnResize at the first Show / ShowModal | `rapidq_form_first_show` |
| QTIMER firing at its Interval to the millisecond; Interval 0 every second | `graphics/credits`, `Gauge/CoolGauge` | Windows' 15.6 ms ticks (RC.EXE: about 50 a second at Interval 1); Interval 0 never fires | unit test (`timers`) |
| RLE4 / RLE8 / 16-bit BMPs refused | `forms/closeQ` | decoded | unit test (`codec`) |
| An empty QBITMAP's BMP refused when drawn | `graphics/pcx/PCXload` | draws nothing | — |
| Aligned controls made before the form shows placed by RapidR's "changed first" rule (tool buttons reversed, a splitter right of the panel made after it) | `QToolbar/OldStyle/ToolTest`, `Toolbar/ToolTest` | RapidQ's order at the first Show: creation order, alBottom / alRight by reach | `rapidq_align_first_show` |
| A QPANEL's aligned children over its bevels | every tool bar of panels | inside them (Left 1 in a default panel) | `rapidq_align_first_show` |
| Status panels 100 wide | `Mouse/Mouse Position` | 50, as RC.EXE reads them | GUI: `statusbar_panels` |
| QBUTTON ignored its BMP / BMPHandle | `Splitter/qfsplit` | glyph beside the caption | — |
| `List2.Handle = List1.Handle` (QIMAGELIST) did nothing | `QToolbar/Qtoolbar_Simple` | the other list's pictures | `rapidq_imagelist_handle` |
| Canvases made in OnShow never painted (the interpreter) | `games/Sokoban/SokobanBuilder` | first paints once OnShow's handler has run | — |
| QDIRTREE's InitialDir not scrolled into view | `files/dirtree` | it is | — |
| QSTRINGGRID's FixedColor ignored; OnDrawCell's drawing clipped to its cell; list / ellipsis buttons on every cell | `grids/stringGridOnColClick`, `grids/mergeGrid`, `grids/listgrid` | as RC.EXE's windows | — |
| A QFILESTREAM that can't open its file: a warning, the program went on | `games/ateroids2/asteroids.his` | the program stops: `Cannot open file x.` (EFOpenError) | `rapidq_file_open_missing` |
| Native builds failed on an event bound to a SUB only DECLAREd | `grids/QStringGridsTwoLinesBitMap` | it fires nothing, as interpreted | `rapidq_declared_handler` |
| A QBITMAP's `Font.AddStyles = n` / `Font.AddStyles(…)` went nowhere (the styles set as a component's flat FontBold …, which a QBITMAP doesn't keep) | `games/Sokoban/Sokoban_1`, `_2`, `_3`, `_GB` (their bold labels) | the bitmap's own font takes them | `rapidq_bitmap_font_styles` |
| A borderless form (BorderStyle bsNone) showed scroll bars for what lies past its edge | `forms/closeQ` | never a bar, the whole client area (RC.EXE: ClientWidth / Height stay the form's size with a button at 500, 500) | unit test (`scrollbars`) |
| QFILELISTBOX: directories first, no `[.]` | `bmp/BMP_viewer2`, `files/*` | the files, then the directories with `[..]` and `[.]`, in a Windows list box's order (symbols, digits, letters) | `file_list_box`, unit test (`filelist`) |
| `&HFFFF0000???` (RapidQ's own CommCtrl.inc), `&HH1`: compile errors; hex numbers past 8 digits -2147483648 | the `.rqw` programs' includes, `keyboard/CodeKeyPad` | `?` and `@` are the digit 0, a second H is skipped, the low 32 bits kept | `rapidq_hex_question_digits` |
| QFONTDIALOG: a new dialog's Color 0, system colours lost through GetFont / SetFont; `SetFont(Label.Font)` did nothing | Robert's test, `examples/gui/dialogs.rr` | Color clWindowText; colours pass as they are; a component's own Font is taken too (an addition: RC.EXE refuses it, `Wrong type L.FONT`) | `rapidq_font_dialog_fonts`, `component_font_passed` |
| The sweep built GUI programs as `.app` bundles (since B-PKG) and looked for an executable that wasn't there | every GUI program, native | `rapidr build … --no-bundle` | — |
| RapidQ programs saved as `.rqb` / `.rq` (31; none is `$INCLUDE`d by another) never swept | `Encryption/encoding.rqb`, `music/*.rqb`, `arrays/qfixedstringstack.rq` … | the corpus tools take them | — |
| A bare `INPUT$` (`a = INPUT$`, the wait before a program ends) an unknown variable: no wait interpreted, no native build | `arrays/qfixedstringstack.rq` | reads a line (RC.EXE: "hello" in, "hello" back) | `rapidq_input_bare` |
| A FUNCTION named with a dot returned nothing (`Calc.Twice = N * 2` set a member) | — (found probing RapidQ's dotted names) | the function's result | `rapidq_dotted_routines` |
| `CONST Null=&0`, `CONST Application.Path = …`, `SLEEP(T * 11.2) / 600`: compile errors | `music/*.rqb` (DLL programs: they run on Windows) | `&` + decimal digits the number, a dotted CONST read back by its name, the parenthesised start of an argument | `rapidq_const_forms` |
| QFONTDIALOG given a system colour (clWindowText, every font that set none) listed "Custom" | Robert's test, `examples/gui/dialogs.rr` | the colour's name (Black), Color kept clWindowText unless another is picked — a deliberate difference from RC.EXE's Custom | unit tests (`font_dialog`, kernel `dialogs`), gallery `example-dialogs-font` |

## Differences left

- **Form frames.** RapidR's frame is its own (a 1-pixel border, a 29-pixel caption, the same for every BorderStyle: Width − ClientWidth 2, Height − ClientHeight 31); RC.EXE on Windows 11 reads 16 × 39 for bsSizeable and 6 × 29 for bsSingle, bsDialog and bsToolWindow. A program that sets Width / Height gets a larger client area than RapidQ's, and one that sets ClientWidth / ClientHeight a smaller window. A design decision of RapidR's (`rapidr_value::layout::form_frame`), not changed in this lane.
- **Positions before the first Show.** RapidQ aligns nothing before a form shows (every aligned control reads Left / Top 0 until then); RapidR keeps the layout current, in the order RapidQ uses at the Show.
- **Programs RC.EXE doesn't build or run either**: `CGI/cgi_test`, `CGI/info` (RC.EXE: `Member INITCGI not part of class CGI` — RapidR builds them, warns at run time: member checks at compile time are the compiler's lane); `QToolbar/OldStyle/Toolbar`, `Toolbar/Toolbar` (include files: RC.EXE's builds stop with an access violation, RapidR shows nothing); `Linear_Solver/crap`, `sound/SBFORM`, `sound/sbbas/SBFORM` (no window in RC.EXE's build either); `forms/Two Windows` (an empty form in both).
- **Smaller looks** seen beside RC.EXE's windows: Windows 11's thin scroll bars on lists and grids, QDIRTREE's dotted lines, a list box's disabled bars, list view column widths.
- **Console programs' box characters.** RapidQ's console shows CHR$(128 … 255) in the OEM code page (437: `CHR$(201)` is ╔, `games/battle` draws its board so); RapidR's strings are Unicode, so a terminal shows Latin-1 (É). Showing code page 437 needs RapidR to know a program is RapidQ's (its `.bas` / `.rqw` source) and not a RapidR program printing accented text (`unicode_strings`): a decision for Robert, not made here.
- **Downloaded examples ask first.** RapidQ's examples in `~/Downloads` carry macOS' quarantine mark, so `rapidr run` asks whether to run each one (in the terminal, `[y/N]`; without one, in a window) the first time — what a user running the examples from there meets first. The sweep runs copies without the mark.
- **RapidQ takes misnested blocks**: `IF … THEN` / `FOR` / `END IF` / `NEXT` (`mysql/rqlibsql/mySQL_API.bas`, a DLL program) compiles in RC.EXE (a warning) and is a compile error in RapidR — the compilers' lane.
- **`INPUT$(n)` from a pipe**: RC.EXE's build returns a whole line (`INPUT$(2)` gave "world"); RapidR's waits for n keys as the manual says (in a terminal both wait for the keys). Only seen with piped input.
- **`SOUND` in `sound/gunsrose`**: RC.EXE's build stops with EPrivilege on Windows 11 (the PC speaker's ports); RapidR plays the notes — kept.
- **QDXTIMER at Interval 0** (`Gauge/CoolGauge`) ticks faster than RC.EXE's: the DirectX lane's (C-DX).
- **A grid's selected cell** when the window isn't active (`grids/goRangeSelectGrid`: RC.EXE's capture draws OnDrawCell's State 1 fill; RapidR's grid has the focus, State 3): depends on which window is active at the capture.
- **Bold Arial at 8 pt is narrower than Windows'**: RC.EXE measures "bestuur magazijnmedewerker" 169 pixels bold (143 regular), RapidR 155 (144) — Windows' hinted bold advances are a pixel wider for e, r, s, k, m, w (7, 5, 7, 7, 11, 10 against RapidR's 6, 4, 6, 6, 10, 9). Sokoban's labels, drawn word by word at RapidQ's positions, overlap a pixel or two. The fonts' metrics (the bold faces' lane).
