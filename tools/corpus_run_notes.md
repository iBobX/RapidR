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

## Differences left

- **Form frames.** RapidR's frame is its own (a 1-pixel border, a 29-pixel caption, the same for every BorderStyle), Windows 11's is 8 pixels a side and a 31-pixel caption, 3 for a tool window. A program that sets Width / Height gets a larger client area than RapidQ's, and one that sets ClientWidth / ClientHeight and then BorderStyle (`games/puzzle`) a smaller one — there, scroll bars RapidQ doesn't show. A design decision of RapidR's (`rapidr_value::layout::form_frame`), not changed in this lane.
- **Positions before the first Show.** RapidQ aligns nothing before a form shows (every aligned control reads Left / Top 0 until then); RapidR keeps the layout current, in the order RapidQ uses at the Show.
- **Programs RC.EXE doesn't build or run either**: `CGI/cgi_test`, `CGI/info` (RC.EXE: `Member INITCGI not part of class CGI` — RapidR builds them, warns at run time: member checks at compile time are the compiler's lane); `QToolbar/OldStyle/Toolbar`, `Toolbar/Toolbar` (include files: RC.EXE's builds stop with an access violation, RapidR shows nothing); `Linear_Solver/crap`, `sound/SBFORM`, `sound/sbbas/SBFORM` (no window in RC.EXE's build either); `forms/Two Windows` (an empty form in both).
- **Smaller looks** seen beside RC.EXE's windows: Windows 11's thin scroll bars on lists and grids, QDIRTREE's dotted lines, a list box's disabled bars, list view column widths.
