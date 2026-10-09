# RapidR examples

Small, commented programs that show what RapidR does, a topic a folder.
Each one is a complete program: read it top to bottom, run it, change it.

Get a copy to play with (an installed RapidR SDK has them too):

```sh
rapidr examples                          # the list
rapidr examples copy hello               # one example (and its data files) into ./rapidr-examples
rapidr examples copy all ~/my-examples   # all of them, by topic
```

## Four ways to run an example

Every example says at the top how to start it. The same source runs four ways:

| Way | Command | What you get |
|-----|---------|--------------|
| **run** | `rapidr run gui/hello_form.rr` | Runs it now: compiled in memory, run by the RapidR Runtime (bytecode VM). |
| **interp** | `rapidr build gui/hello_form.rr out --interp` | A standalone executable `out/hello_form`: the runtime and the program's bytecode in one file. No Rust needed. |
| **native** | `rapidr build gui/hello_form.rr` (add `--release` for speed) | A native executable `gui/hello_form`, through generated Rust (needs Rust: `rapidr setup`). |
| **web** | `rapidr bundle-bc gui/hello_form.rr -o hello.zip` | A static web site (unzip it, serve the folder with any web server, e.g. `python3 -m http.server`): the program runs in the browser, drawn by the same UI kernel. |

Data files an example uses (pictures, sounds, a CSV) are built into it with
`$RESOURCE`, so its executables and web pages run from anywhere.

## The examples

The **Runs on** column lists the ways each example is tested on (by
`tests/examples_run.mjs`: every example, every way it claims, headless).

### basics/ — the language, in a console

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [hello.rr](basics/hello.rr) | `PRINT`, variables, `FOR`, a `FUNCTION` and a `SUB`, string functions | run · interp · native · web | `rapidr run basics/hello.rr` |
| [input.rr](basics/input.rr) | `INPUT`: a tip calculator that asks for numbers | run · interp · native | `rapidr run basics/input.rr` |
| [files.rr](basics/files.rr) | Text files: `OPEN` for output / append / input, `LINE INPUT`, `EOF`, an `RFileStream`, `KILL` | run · interp · native · web | `rapidr run basics/files.rr` |
| [language.rr](basics/language.rr) | `TYPE` records, arrays, `DATA` / `READ`, `SELECT CASE`, `DO … LOOP`, an object `TYPE … EXTENDS RObject` with methods and a constructor | run · interp · native · web | `rapidr run basics/language.rr` |

### gui/ — forms and their events

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [hello_form.rr](gui/hello_form.rr) | A form built with `CREATE`, a text box, a button's `OnClick`, `OnChange` | run · interp · native · web | `rapidr run gui/hello_form.rr` |
| [menus.rr](gui/menus.rr) | A main menu: shortcuts, a check item, radio items, a submenu; a pop-up menu; a status bar | run · interp · native · web | `rapidr run gui/menus.rr` |
| [dialogs.rr](gui/dialogs.rr) | Open file, colour and font dialogs; a Yes / No question (`MESSAGEDLG`) | run · interp · native · web | `rapidr run gui/dialogs.rr` |
| [stopwatch.rr](gui/stopwatch.rr) | An `RTimer`: start, lap, stop, reset; a list of laps | run · interp · native · web | `rapidr run gui/stopwatch.rr` |
| [pantry.rr](gui/pantry.rr) | An `RListBox` and an `RStringGrid` working together; adding rows, totals | run · interp · native · web | `rapidr run gui/pantry.rr` |
| [themes.rr](gui/themes.rr) | RapidR's themes: `$THEME modern`, switching with `Application.Theme` (classic, modern, dark, high contrast) | run · interp · native · web | `rapidr run gui/themes.rr` |
| [tray.rr](gui/tray.rr) | A system tray icon, as RapidQ programs make one (`RNotifyIconData`, `Shell_NotifyIcon`, the form's `WndProc`) | run · interp · native · web | `rapidr run gui/tray.rr` |

### studio/ — RapidR Studio's panels in your own program

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [panels.rr](studio/panels.rr) | The IDE's public components docked together: a toolbar, the toolbox, the project tree, a designer with the property inspector following it (Anchors' pin editor), the output console, the command palette | run · interp · native · web | `rapidr run studio/panels.rr` |

### graphics/ and directx/ — drawing

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [canvas.rr](graphics/canvas.rr) | Drawing on an `RCanvas` in `OnPaint`: a bar chart; mouse clicks add dots; `Pixel` | run · interp · native · web | `rapidr run graphics/canvas.rr` |
| [sprites.rr](directx/sprites.rr) | DirectX 2D: an `RDXScreen`, sprites from an `RDXImageList` ([sprites.dxg](directx/sprites.dxg)), an `RDXTimer` animating them, `Flip` | run · interp · native · web | `rapidr run directx/sprites.rr` |
| [d3d_cube.rr](directx/d3d_cube.rr) | Direct3D: a `.X` model ([cube.x](directx/cube.x)) in a mesh builder, lights, the camera, a spinning frame | run · interp · native · web | `rapidr run directx/d3d_cube.rr` |

### media/ — sound and video

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [midi.rr](media/midi.rr) | `RMIDI`: play, pause, stop a MIDI song ([tune.mid](media/tune.mid)); RapidR's built-in synthesizer plays it where the system has none | run · interp · native · web | `rapidr run media/midi.rr` |
| [wave.rr](media/wave.rr) | `RWave`: play a WAV ([chime.wav](media/chime.wav)); record a second from the microphone, save it, play it back | run · interp · native · web | `rapidr run media/wave.rr` |
| [video.rr](media/video.rr) | `RVideo`: an AVI ([clip.avi](media/clip.avi)) on a form, enlarged, with a frame counter | run · interp · native · web | `rapidr run media/video.rr` |

### data/ — databases, JSON, data science

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [sqlite.rr](data/sqlite.rr) | `RSQLite`: a table, inserts and queries with `?` parameters (and why they matter), `QueryScalar`, `GROUP BY` | run · interp · native · web | `rapidr run data/sqlite.rr` |
| [json.rr](data/json.rr) | `RJson`: parse, read by path (`customer.city`, `items.0`), set, remove, save to and load from a file | run · interp · native · web | `rapidr run data/json.rr` |
| [numbers.rr](data/numbers.rr) | `RNum` arrays: `FromList`, `Arange`, `Linspace`, statistics, arithmetic on every element, `Cumsum`, `Unique` | run · interp · native · web | `rapidr run data/numbers.rr` |
| [csv_explorer.rr](data/csv_explorer.rr) | Drop a CSV file on the window (or open one, or the sample [shop.csv](data/shop.csv)): a table sorted by a click on a heading, a filter, each column's count, min, max and mean, and a live `RPlot` bar, line or scatter chart of any two columns; the form's `OnDropFiles` | run · interp · native · web | `rapidr run data/csv_explorer.rr` |
| [dataframe.rr](data/dataframe.rr) | `RDataFrame` from a CSV ([staff.csv](data/staff.csv)): filter, sort, into a grid; an `RPlot` bar chart on the form | run · interp · native · web | `rapidr run data/dataframe.rr` |

### network/ — HTTP (against a server on your own machine)

Start a server in the folder first, e.g. `cd network && python3 -m http.server 8000`.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [http_json.rr](network/http_json.rr) | `RHttp.Get` fetching a JSON document ([forecast.json](network/forecast.json)), read with `RJson` | run · interp · native · web | `rapidr run network/http_json.rr http://127.0.0.1:8000/forecast.json` |
| [download.rr](network/download.rr) | `RDownload` (RapidQ's `QDOWNLOAD`, from its `Qdownload.inc`): a file downloaded into a file, its progress bar on the form | run · interp · native · web | `rapidr run network/download.rr http://127.0.0.1:8000/forecast.json` |

On a web page the address goes after `?` in the page's address
(`index.html?http://127.0.0.1:8000/forecast.json`); serve the page from the
same server, or from one that allows it (CORS).

### iot/ — boards on a serial port (ESP32, Arduino)

Plug the board in by USB first. [Serial ports and IoT boards](../docs/manual/serial-ports.md) says more.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [esp32_monitor.rr](iot/esp32_monitor.rr) | `RComPort` for IoT boards: the serial ports in a combo box with their USB adapters (`FillList`), connect at 115200, **Reset** the board through DTR / RTS, its boot log and output line by line (`OnLine`), adapters plugged in and out (`OnPortsChanged`). Never writes to the board | run · interp · native · web | `rapidr run iot/esp32_monitor.rr` |

In a browser (Chrome, Edge) the first Connect asks which port to use.

### rapidq/ — RapidQ programs, unchanged

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [notepad.bas](rapidq/notepad.bas) | A text editor in plain RapidQ style: `$TYPECHECK ON`, `DECLARE SUB`, Q components (`QRICHEDIT`, `QMAINMENU`, `QSTATUSBAR` panels), open / save dialogs, `OnClose`'s `Action` | run · interp · native · web | `rapidr run rapidq/notepad.bas` |

RapidR runs RapidQ programs unchanged. The other examples use RapidR's own
names: `RButton`, `RForm`, `RStringGrid` … (RapidQ's names, `QBUTTON`,
`QFORM` …, are the same components and work everywhere). `rapidq/notepad.bas`
is kept in RapidQ's own style on purpose, to show that it runs as it is. To
bring a RapidQ program of your own over to RapidR's names, use
`rapidr import-rapidq` (it writes a converted copy and a report, and leaves
the original alone).

### web/ — what only a web page has

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [todo.rr](web/todo.rr) | A to-do list kept in the browser's storage (`RWebStorage`): it's there again next visit | web | `rapidr bundle-bc web/todo.rr -o todo.zip` |
| [browser.rr](web/browser.rr) | Asking the browser with `RJavaScript.Eval`; pages in the address's `#` with `RRouter` (Back works) | web | `rapidr bundle-bc web/browser.rr -o browser.zip` |

### The IDE

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [ide.rr](ide.rr) | RapidR's visual IDE, itself a RapidR program (`rapidr ide` starts it; the SDK ships it compiled) | run · interp · native | `rapidr ide` |

## Making the data files again

The media and DirectX files are RapidR's own, made by a script (nothing
downloaded): `python3 tools/make_example_media.py` (from a RapidR checkout)
writes `media/tune.mid`, `media/chime.wav`, `media/clip.avi`,
`directx/sprites.dxg` and `directx/cube.x`. `data/staff.csv` and
`network/forecast.json` are made-up data.

## Testing them

From a RapidR checkout: `node tests/examples_run.mjs` (or the `examples`
stage of `tools/regress.sh`) runs every example on every way listed above,
GUI ones through RapidR's test hooks (scripted clicks, then the components'
properties read back), with no sound device, printer or internet.
