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
| **native** | `rapidr build gui/hello_form.rr` | A native executable `gui/hello_form` (optimized), through generated Rust (needs Rust once: `rapidr setup`). |
| **web** | `rapidr bundle-bc gui/hello_form.rr -o hello.zip` | A static web site (unzip it, serve the folder with any web server, e.g. `python3 -m http.server`): the program runs in the browser, drawn by the same UI kernel. |

To hand a program to someone who has the RapidR Runtime, `rapidr build-bc
gui/hello_form.rr -o hello.rrbc` makes a file that runs with `rapidr
hello.rrbc` or a double click.

Data files an example uses (pictures, sounds) are built into it with
`$RESOURCE`, so its executables and web pages run from anywhere; a data
example's CSV or JSON file sits beside it.

## The examples

In the order to read them: the language first, then windows, drawing, data,
the debugger, media and DirectX, the network, AI, boards, RapidQ programs,
Windows messages, the web, and last the IDE's own parts. The **Runs on** column lists the ways each
example is tested on (by `tests/examples_run.mjs`: every example, every way it
claims, headless).

### basics/ — the language, in a console

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [hello.rr](basics/hello.rr) | The first program: `PRINT`, variables, `FOR`, a `FUNCTION` and a `SUB`, string functions | run · interp · native · web | `rapidr run basics/hello.rr` |
| [input.rr](basics/input.rr) | `INPUT`: a tip calculator that asks for numbers | run · interp · native | `rapidr run basics/input.rr` |
| [files.rr](basics/files.rr) | Text files: `OPEN` for output / append / input, `LINE INPUT`, `EOF`, an `RFileStream`, `KILL` | run · interp · native · web | `rapidr run basics/files.rr` |
| [language.rr](basics/language.rr) | A tour: `TYPE` records, arrays, `DATA` / `READ`, `SELECT CASE`, `DO … LOOP`, an object `TYPE` with methods and a constructor | run · interp · native · web | `rapidr run basics/language.rr` |

### gui/ — forms and their events

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [hello_form.rr](gui/hello_form.rr) | A form built with `CREATE`: a text box, a button's `OnClick`, `OnChange` | run · interp · native · web | `rapidr run gui/hello_form.rr` |
| [focus_events.rr](gui/focus_events.rr) | `OnEnter` and `OnExit` (RapidR's own): a box's help when the focus comes to it, its check when the focus leaves it ([the manual](../docs/manual/components.md#the-focus-onenter-and-onexit)) | run · interp · native · web | `rapidr run gui/focus_events.rr` |
| [stopwatch.rr](gui/stopwatch.rr) | An `RTimer`: start, lap, stop, reset; a list of laps | run · interp · native · web | `rapidr run gui/stopwatch.rr` |
| [pantry.rr](gui/pantry.rr) | An `RListBox` and an `RStringGrid` working together; adding rows, totals | run · interp · native · web | `rapidr run gui/pantry.rr` |
| [menus.rr](gui/menus.rr) | A main menu with shortcuts, check and radio items, a submenu; a pop-up menu; a status bar | run · interp · native · web | `rapidr run gui/menus.rr` |
| [dialogs.rr](gui/dialogs.rr) | Open file, colour and font dialogs; a Yes / No question (`MESSAGEDLG`) | run · interp · native · web | `rapidr run gui/dialogs.rr` |
| [richtext.rr](gui/richtext.rr) | A small rich-text editor: `SelAttributes`, paragraphs aligned and bulleted, open and save `.rtf` ([the manual](../docs/manual/rich-text.md)) | run · interp · native · web | `rapidr run gui/richtext.rr` |
| [themes.rr](gui/themes.rr) | RapidR's looks: following the system's light, dark or high contrast as it switches, picking one with `Application.Theme` (light, dark, high contrast, classic), and `Application.OnThemeChange` to take the theme's colours again | run · interp · native · web | `rapidr run gui/themes.rr` |
| [svg_icons.rr](gui/svg_icons.rr) | Your own SVG icons ([icons/](gui/icons)) in a tree's `RImageList` (`AddSVGFile`) and an `RImage` (`Load` of a `$RESOURCE`): sharp at 2×, in the theme's colours (`currentColor`, RapidR's colour tokens) and drawn again when it changes ([the manual](../docs/manual/svg-icons.md)) | run · interp · native · web | `rapidr run gui/svg_icons.rr` |
| [sales_dashboard.rr](gui/sales_dashboard.rr) | A big window (1200 × 800): figures, two `RPlot` charts, a grid; in Studio on the web it shows whole, the run area scrolls ([the web](../docs/manual/web.md#windows-bigger-than-the-page)) | run · interp · native · web | `rapidr run gui/sales_dashboard.rr` |
| [value_tree.rr](gui/value_tree.rr) | A tree that shows values as RapidR Studio's debugger does (`SetItemValue`, `Item(i).Changed`): each with its kind's icon, numbers, text and True/False in their colours, a colour as a swatch and its name, an array with its count, a changed value marked ([the manual](../docs/manual/debugging.md#how-values-look)) | run · interp · native · web | `rapidr run gui/value_tree.rr` |
| [tray.rr](gui/tray.rr) | A system tray icon, as RapidQ programs make one (`RNotifyIconData`, `Shell_NotifyIcon`, `WndProc`) | run · interp · native · web | `rapidr run gui/tray.rr` |

### translations/ — one program in every language

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [languages.rr](translations/languages.rr) | `TR$` and `TRN$` with PO files in `lang/` beside it (Spanish, Portuguese, Chinese): it starts in the system's language, a list switches `Application.Language`, `Application.OnLanguageChange` writes the captions again, "1 file" / "5 files" in each language's way ([the manual](../docs/manual/translations.md)) | run · web | `rapidr run translations/languages.rr es` |

### recipes/ — short programs to start from

Small, complete programs of the shapes people ask for most. RapidR Studio's
assistant starts from the closest one when you ask it for a new program
([the manual](../docs/manual/ai-assistant.md#recipes-and-friendly-hints)); they're just as good to copy by hand.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [calculator.rr](recipes/calculator.rr) | A calculator with sqrt, x², log, sin, cos, tan: one handler for every key (`Sender`), the display on top (`Align = alTop`) | run · interp · native · web | `rapidr run recipes/calculator.rr` |
| [tip_calculator.rr](recipes/tip_calculator.rr) | Bill, tip and people in text boxes; the totals follow every change (`OnChange`) | run · interp · native · web | `rapidr run recipes/tip_calculator.rr` |
| [stopwatch.rr](recipes/stopwatch.rr) | An `RTimer` stopwatch: Start / Stop and Reset | run · interp · native · web | `rapidr run recipes/stopwatch.rr` |
| [todo_list.rr](recipes/todo_list.rr) | A to-do list: add with Enter, remove the one picked (`AddItems`, `DelItems`, `ItemIndex`) | run · interp · native · web | `rapidr run recipes/todo_list.rr` |
| [unit_converter.rr](recipes/unit_converter.rr) | Kilometres, miles, degrees, kilograms: a combo box picks the conversion | run · interp · native · web | `rapidr run recipes/unit_converter.rr` |
| [quiz_game.rr](recipes/quiz_game.rr) | Three questions from `DATA`, answer buttons sharing a handler, a score, Play again | run · interp · native · web | `rapidr run recipes/quiz_game.rr` |
| [drawing_pad.rr](recipes/drawing_pad.rr) | Draw with the mouse on an `RCanvas` (`Align = alClient`), pick a colour, clear; `OnPaint` draws it again | run · interp · native · web | `rapidr run recipes/drawing_pad.rr` |
| [password_generator.rr](recipes/password_generator.rr) | Random passwords (`RANDOMIZE`, `RND`) of a length you choose, with or without digits and symbols | run · interp · native · web | `rapidr run recipes/password_generator.rr` |
| [csv_viewer.rr](recipes/csv_viewer.rr) | Open a CSV file (`ROpenDialog`) and show it in an `RStringGrid` (people.csv beside it) | run · interp · native · web | `rapidr run recipes/csv_viewer.rr` |
| [server_manager.rr](recipes/server_manager.rr) | A list of sites with their status and Start / Stop / Restart buttons | run · interp · native · web | `rapidr run recipes/server_manager.rr` |

### graphics/ — drawing

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [canvas.rr](graphics/canvas.rr) | Drawing on an `RCanvas` in `OnPaint`: a bar chart; clicks add dots; `Pixel` | run · interp · native · web | `rapidr run graphics/canvas.rr` |

### data/ — databases, JSON, data science

Start with the CSV Explorer; [Data science with RapidR](../docs/manual/data-science-with-rapidr.md) walks through the linked ones.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [sqlite.rr](data/sqlite.rr) | `RSQLite`: a table, inserts and queries with `?` parameters (and why they matter), `QueryScalar`, `GROUP BY` | run · interp · native · web | `rapidr run data/sqlite.rr` |
| [secrets.rr](data/secrets.rr) | `RSecrets`: a password kept in the OS keychain (Save, Read, Delete), never in the program or a file; on the web the buttons say there's no keychain ([the manual](../docs/manual/secrets.md)) | run · interp · native · web | `rapidr run data/secrets.rr` |
| [json.rr](data/json.rr) | `RJson`: parse, read by path (`customer.city`, `items.0`), set, remove, save and load | run · interp · native · web | `rapidr run data/json.rr` |
| [numbers.rr](data/numbers.rr) | `RNum` arrays: `FromList`, `Arange`, `Linspace`, statistics, arithmetic on every element, `Cumsum`, `Unique` | run · interp · native · web | `rapidr run data/numbers.rr` |
| [csv_explorer.rr](data/csv_explorer.rr) | Drop a CSV on the window ([shop.csv](data/shop.csv) is the sample): a sortable table, a filter, each column's statistics, a live `RPlot` chart ([the tutorial](../docs/manual/data-science.md#tutorial-the-csv-explorer)) | run · interp · native · web | `rapidr run data/csv_explorer.rr` |
| [dataframe.rr](data/dataframe.rr) | Linked: an `RCSV` of a `$RESOURCE` ([staff.csv](data/staff.csv)) → two `RDataFrame`s → an `RDBGrid` and an `RPlot` bound to columns (`DataSource`, `Kind`, `X`, `Y`); the buttons Filter, Sort and GroupBy the frames and both follow; live in Studio's designer | run · interp · native · web | `rapidr run data/dataframe.rr` |
| [linked_data.rr](data/linked_data.rr) | Linked components with almost no code: `RCSV` → `RDataFrame` → `RDataSource` → an `RDBGrid` and bound edits and labels; live in Studio's designer | run · interp · native · web | `rapidr run data/linked_data.rr` |
| [json_source.rr](data/json_source.rr) | A JSON file ([products.json](data/products.json)) as a data source for a grid and edits; filtered, grouped, saved back as JSON | run · interp · native · web | `rapidr run data/json_source.rr` |
| [million_rows.rr](data/million_rows.rr) | A million rows in an `RDBGrid` that draws only the rows on screen; filter, sort and group them all, with the time each took | run · interp · native · web | `rapidr run data/million_rows.rr [rows]` |
| [million_rows_async.rr](data/million_rows_async.rr) | A million rows from SQLite in the background (`QueryAsync`, `OnRows`, `OnDone`, `Cancel`): the window keeps answering, an `RDBGrid` shows the rows as they stream in, a heading's click sorts with `ORDER BY`; an SQL `RCodeEditor` completes the table's columns (`SetSchema`) and runs the statement with Ctrl+Enter | run · interp · native · web | `rapidr run data/million_rows_async.rr [rows]` |

### debugging/ — RapidR Studio's debugger

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [shop.rr](debugging/shop.rr) | A small shop's order book to look at in the debugger: nested `TYPE`s, an `RDataFrame`, an `RNum`, two FUNCTIONs on one line (Step Into Specific), a division by zero (Set Next Statement) — [Debugging](../docs/manual/debugging.md) | run · interp · native · web | `rapidr ide debugging/shop.rr` |

### media/ — sound and video

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [midi.rr](media/midi.rr) | `RMIDI`: play, pause, stop a MIDI song ([tune.mid](media/tune.mid)); RapidR's own synthesizer where the system has none | run · interp · native · web | `rapidr run media/midi.rr` |
| [wave.rr](media/wave.rr) | `RWave`: play a WAV ([chime.wav](media/chime.wav)); record a second from the microphone, save, play back | run · interp · native · web | `rapidr run media/wave.rr` |
| [video.rr](media/video.rr) | `RVideo`: an AVI ([clip.avi](media/clip.avi)) on a form, enlarged, with a frame counter | run · interp · native · web | `rapidr run media/video.rr` |

### directx/ — DirectX 2D and Direct3D

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [sprites.rr](directx/sprites.rr) | An `RDXScreen`, sprites from an `RDXImageList` ([sprites.dxg](directx/sprites.dxg)), an `RDXTimer` animating them, `Flip` | run · interp · native · web | `rapidr run directx/sprites.rr` |
| [d3d_cube.rr](directx/d3d_cube.rr) | Direct3D: a `.X` model ([cube.x](directx/cube.x)) in a mesh builder, lights, the camera, a spinning frame | run · interp · native · web | `rapidr run directx/d3d_cube.rr` |

### network/ — HTTP (against a server on your own machine)

Start a server in the folder first, e.g. `cd network && python3 -m http.server 8000`.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [http_json.rr](network/http_json.rr) | `RHttp.Get` fetching a JSON document ([forecast.json](network/forecast.json)), read with `RJson` | run · interp · native · web | `rapidr run network/http_json.rr http://127.0.0.1:8000/forecast.json` |
| [download.rr](network/download.rr) | `RDownload` (RapidQ's `QDOWNLOAD`): a file into a file, its progress bar on the form | run · interp · native · web | `rapidr run network/download.rr http://127.0.0.1:8000/forecast.json` |

On a web page the address goes after `?` in the page's address
(`index.html?http://127.0.0.1:8000/forecast.json`); serve the page from the
same server, or from one that allows it (CORS).

### ai/ — AI models in your program (RAI)

You need an account with an AI provider (Anthropic, OpenAI, DeepSeek …) and
its API key, or a model server on your own machine (Ollama, LM Studio),
which needs no key. A question to a paid provider costs a little: the
example shows each provider's prices. [The AI assistant](../docs/manual/ai-assistant.md)
says more.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [rai_models.rr](ai/rai_models.rr) | `RAI` in a window: pick a provider, give a key (never written in the program), list its models, ask, watch the answer stream into an `RMarkdownView`, see the tokens it took | run · interp · native · web | `rapidr run ai/rai_models.rr` |
| [ask.rr](ai/ask.rr) | `RAI` in a console program: a local model server or a provider whose key is in its environment variable; the answer printed as it arrives | run · interp · native · web | `rapidr run ai/ask.rr http://localhost:11434/v1` |

### iot/ — boards on a serial port (ESP32, Arduino)

Plug the board in by USB first. [Serial ports and IoT boards](../docs/manual/serial-ports.md) says more; in a browser (Chrome, Edge) the first Connect asks which port to use.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [esp32_monitor.rr](iot/esp32_monitor.rr) | `RComPort` for boards: ports listed with their USB adapters, connect at 115200, reset through DTR / RTS, the boot log line by line, plug and unplug events. Never writes to the board | run · interp · native · web | `rapidr run iot/esp32_monitor.rr` |

### rapidq/ — RapidQ programs, unchanged

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [notepad.bas](rapidq/notepad.bas) | A text editor in plain RapidQ style: `$TYPECHECK ON`, `DECLARE SUB`, Q components, open / save dialogs, `OnClose`'s `Action` | run · interp · native · web | `rapidr run rapidq/notepad.bas` |
| [shapes/shapes.bas](rapidq/shapes/shapes.bas) | A RapidQ program in several files, brought in with `$INCLUDE` written as on Windows; try `rapidr import-rapidq` on it ([Include files](../docs/manual/importing-rapidq.md#include-files)) | run · interp · native · web | `rapidr run rapidq/shapes/shapes.bas` |

RapidR runs RapidQ programs unchanged. The other examples use RapidR's own
names: `RButton`, `RForm`, `RStringGrid` … (RapidQ's names, `QBUTTON`,
`QFORM` …, are the same components and work everywhere). The `rapidq/`
examples are kept in RapidQ's own style on purpose, to show that they run as
they are. To bring a RapidQ program of your own over to RapidR's names, use
`rapidr import-rapidq` (it writes a converted copy and a report, and leaves
the original alone).

### windows/ — Windows only: a form's Windows messages

A form's `WndProc` hears every message Windows sends or posts to its window,
as in RapidQ ([Windows messages](../docs/manual/components.md#windows-messages-a-forms-wndproc)).
Windows only: on other systems WndProc receives only the tray icon's
messages, and these examples say so instead of doing anything (the tests
run them there for that).

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [wndproc_messages.rr](windows/wndproc_messages.rr) | A little Spy++: each message of the form's window as it comes, its number in hex and its name | run · interp · native · web | `rapidr run windows/wndproc_messages.rr` |
| [wndproc_postmessage.rr](windows/wndproc_postmessage.rr) | A button posts `WM_USER + 1` to the form with `PostMessageA(Form.Handle, …)`; WndProc shows its wParam and lParam | run · interp · native · web | `rapidr run windows/wndproc_postmessage.rr` |
| [midi_in.rr](windows/midi_in.rr) | A MIDI keyboard's notes: winmm's `midiInOpen` with `CALLBACK_WINDOW` sends them to the form as `MM_MIM_DATA`; says so when there is no MIDI input; Test note posts one to try it without a keyboard | run · interp · native · web | `rapidr run windows/midi_in.rr` |

### web/ — what only a web page has

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [todo.rr](web/todo.rr) | A to-do list kept in the browser's storage (`RWebStorage`): it's there again next visit | web | `rapidr bundle-bc web/todo.rr -o todo.zip` |
| [browser.rr](web/browser.rr) | Asking the browser with `RJavaScript.Eval`; pages in the address's `#` with `RRouter` (Back works) | web | `rapidr bundle-bc web/browser.rr -o browser.zip` |

### showcase/ — bigger programs, written from the idea alone

Each is a whole program in one file, built only from what RapidR has today,
with made-up data (the weather one asks a public server). Listed in
RapidR Studio under **Open an Example**; [the manual's page](../docs/manual/showcase.md)
has a picture of each and explains its best lines.

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [fireworks.rr](showcase/fireworks.rr) | Click the sky or press Space: a rocket flies up and bursts into thousands of sparks, each its own element of arrays with its own gravity, drag and fade; RNum works out the shapes (ball, ring, two rings, a heart); the spark count and the frame rate show in the corner | run · interp · native · web | `rapidr run showcase/fireworks.rr` |
| [notes.rr](showcase/notes.rr) | A Markdown notes app: edit on the left, the formatted note on the right as you type (`RMarkdownView`), clickable links, New / Open / Save with their shortcuts and a Format menu (Ctrl+B, Ctrl+I, Ctrl+E); in a browser the note is also kept in its storage (`RWebStorage`) | run · interp · native · web | `rapidr run showcase/notes.rr` |
| [snake.rr](showcase/snake.rr) | A snake game for the keyboard alone: arrow keys or W A S D, a growing snake, a score, a speed that rises with it, game over and again; an `RCanvas` drawn in `OnPaint` and an `RTimer` | run · interp · native · web | `rapidr run showcase/snake.rr` |
| [same_numbers.rr](showcase/same_numbers.rr) | Four sets of points built by the program to have the same mean, spread, correlation and line (the idea of F. J. Anscombe, 1973), drawn as four `RPlot` scatter charts with the numbers, measured from the points by `RNum`, under each; New points makes four others | run · interp · native · web | `rapidr run showcase/same_numbers.rr` |
| [monte_carlo_pi.rr](showcase/monte_carlo_pi.rr) | Pi by throwing darts: RNum throws and counts a whole batch in a few vector steps (`Uniform`, `Square`, `Sign`, `Sum`), an `RPlot` scatter shows the darts, a second chart the estimate settling; a speed slider from 30 to 300,000 darts a second, Pause, Reset | run · interp · native · web | `rapidr run showcase/monte_carlo_pi.rr` |
| [spin3d.rr](showcase/spin3d.rr) | A lit, shaded 3D planet with a ring and a moon, built from faces by the program (an `RDXScreen` with `RD3DMeshBuilder` and lights); the arrow keys change the speeds | run · interp · native · web | `rapidr run showcase/spin3d.rr` |
| [signal_lab.rr](showcase/signal_lab.rr) | A signal generator and a two-screen oscilloscope: four waves, frequency, amplitude, noise and a second tone; the top screen held still by a trigger, the bottom the spectrum from an FFT written in BASIC (radix 2, 1024 points), its peaks marked in hertz | run · interp · native · web | `rapidr run showcase/signal_lab.rr` |
| [midi_visualizer.rr](showcase/midi_visualizer.rr) | A 3D scene that dances to a song: the program reads the `.mid` file itself (chunks, variable-length times, tempo, Note On), RMIDI plays it, every note moves the scene; Load MIDI…, a Calm switch | run · interp · native · web | `rapidr run showcase/midi_visualizer.rr` |
| [make_song.rr](showcase/make_song.rr) | Composes [song.mid](showcase/song.mid), the visualizer's own 32-bar loop (kick, bass, arpeggio, pad), and writes it as a MIDI file | run · interp · native · web | `rapidr run showcase/make_song.rr` |

### studio/ and the IDE — RapidR Studio's parts

| Example | What it shows | Runs on | Try |
|---------|---------------|---------|-----|
| [panels.rr](studio/panels.rr) | The IDE's public components docked together: toolbar, toolbox, project tree, designer with the inspector, output console, command palette | run · interp · native · web | `rapidr run studio/panels.rr` |
| [form_designer.rr](studio/form_designer.rr) | The designer surface alone (`RDesignSurface`): select and drag components, smart guides, anchor pins, undo, the resize preview | run · interp · native · web | `rapidr run studio/form_designer.rr` |
| [ide.rr](ide.rr) | The first RapidR IDE, itself a large RapidR program, kept to read until RapidR Studio (`rapidr ide`, source in `ide/`) has replaced it | run · interp · native | `rapidr run ide.rr` |

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
