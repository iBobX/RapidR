// The examples (examples/, indexed by examples/README.md) RUN — not only
// compile — on every runtime each one claims:
//
//   run      `rapidr run <file>` (the RapidR Runtime: compiled in memory,
//            the bytecode VM)
//   interp   `rapidr build <file> <dir> --interp`: a standalone executable
//            (the runner with the program's bytecode)
//   native   `rapidr build <file> <dir>`: generated Rust, built by cargo
//            (its build dropped once it ran: tests/cargo_builds.mjs)
//   web      the web runtime (target/web) drawing with the UI kernel on a
//            canvas — tests/web_kernel.html, as tests/web_gui_run.mjs runs
//            the GUI fixtures
//
// Console examples are checked by what they print (stdin from `input`);
// GUI examples by the desktop's test hooks (RAPIDR_TEST_EVENTS fired in
// order, then RAPIDR_TEST_DUMP's `name.property=value` lines — on the
// headless host, RAPIDR_CAPTURE), in the browser by the same hooks
// (rapidr_set_test_env / rapidr_test_results). Every `expect` line must be
// in the output. No sound, MIDI, serial port or microphone (the RAPIDR_TEST_*
// scripted paths), no printer (RAPIDR_PRINT_TO), no user registry
// (RAPIDR_REGISTRY), no internet: the network examples talk to the tests'
// own local server (tests/http_test_server.mjs), `{http}` in a case's
// arguments its http://127.0.0.1:<port>; the AI examples to the tests' mock
// provider (tests/ai_mock_server.mjs: scripted answers, no key, no cost),
// `{aimock}` its base URL.
//
// The table below must list every program in examples/ and
// examples/README.md every entry of the table (checked first).
//
// (RAPIDR_SHOTS=dir keeps the GUI examples' captures from `rapidr run`, at
// RAPIDR_SCALE: to look at them.)
//
// Usage (repo root, after building ./rapidr; the web runtime needs
// tools/build_web_artifacts.sh and the repo served on RAPIDR_URL, default
// http://localhost:8765):
//   node tests/examples_run.mjs [--runtimes run,interp,native,web] [filter …]

import { spawn as spawnAsync, spawnSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dropBuild } from "./cargo_builds.mjs";
import { startHttpServer } from "./http_test_server.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXAMPLES = join(ROOT, "examples");
const WORK = join(ROOT, "tests/conformance/.work/examples");
const CARGO_TARGET = join(ROOT, "tests/conformance/.work/cargo-target");
const EXE = process.platform === "win32" ? ".exe" : "";
const RAPIDR = join(ROOT, `rapidr${EXE}`);
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

// Each case: `file` (under examples/), `runtimes`, and what it checks —
//   input:   what a console program reads (desktop runtimes)
//   args:    its command line (COMMAND$; on the web the page's query)
//   events / dump: a GUI program's test hooks (see tests/gui_parity_cases.mjs)
//   fileDialog / messageDialog / colorDialog / fontDialog: the dialogs' answers
//   drop:    the files `form.__drop` drops on a form (RAPIDR_TEST_DROP, a;b)
//   expect:  lines that must be in the output (stdout, or the dump)
//   env:     environment it runs with (the tests' scripted devices; on the
//            web the page's RAPIDR_TEST_COMPORT)
//   timeout: ms (default 60000)
//   includes: the program's $INCLUDEs are in subfolders of its own: the
//            folder is copied whole, and the web compiles it with every
//            source file under it (compile_files)
//   readOnly: the program runs in its folder made read-only (RapidR's
//            examples where RapidR is installed): what it writes fails, and
//            it must say so (the built executable is made elsewhere)
//   without: files of the example's folder left out (its data missing)
//   extra:   files of other examples' folders (`media/tune.mid`) copied beside it
//   sameFile: a file the program writes must be byte for byte the one shipped
//            beside it (make_song.rr's song.mid)
// (`wait(c, n)`: n clicks on a component that does nothing with them — the
// hooks fire an event every 50 ms, so the program's timers run meanwhile)
const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");
// (text typed key by key into `comp`, a US keyboard's keys: a fake API key,
// a workspace ID — the keyboard's path through the kernel, as a user types)
const typeKeys = (comp, text) => [...text].map((c) => {
  if (/[a-z]/.test(c)) return c.toUpperCase().charCodeAt(0);
  if (/[A-Z]/.test(c)) return `${c.charCodeAt(0)}_256`;
  if (/[0-9 ]/.test(c)) return c.charCodeAt(0);
  return { "-": "189", ".": "190", "_": "189_256" }[c];
}).map((k) => `${comp}.__key_${k}`).join(",");
export const cases = [
  // basics/
  { file: "basics/hello.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["Hello from RapidR!", "3 x 7 = 21", "Squares: 1 4 9 16 25", "RAPIDR in reverse: RDIPAR"] },
  { file: "basics/input.rr", runtimes: ["run", "interp", "native"], input: "Ada\n80\n\n4\n",
    expect: ["Hello, Ada.", "Total with a 15% tip: 92.00", "Each of 4 pays: 23.00"] },
  { file: "basics/files.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["shopping.txt has", "1. bread", "4. coffee", "First line again: bread", "Removed: yes"] },
  { file: "basics/language.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["Mercury: no moons, 57.9 million km", "Earth: one moon", "Mars: 2 moons", "Average distance: 135.9", "Initials: MVEM", "The counter counted to 15"] },
  // debugging/ (the debugger's example: here it runs on its own)
  { file: "debugging/shop.rr", runtimes: ["run", "interp", "native", "web"], events: "addbtn.onclick,totalbtn.onclick", dump: "totallabel.caption,orderlist.itemcount",
    expect: ["totallabel.caption=Total: 25.00", "orderlist.itemcount=4"] },
  // gui/
  { file: "gui/hello_form.rr", runtimes: ["run", "interp", "native", "web"], events: "greetbutton.onclick,greetbutton.onclick", dump: "answer.caption",
    expect: ["answer.caption=Hello, World! (2)"] },
  // (Tab out of each box: its OnExit checks it, the next one's OnEnter
  // shows its help)
  { file: "gui/focus_events.rr", runtimes: ["run", "interp", "native", "web"], events: "nameedit.__key_65,nameedit.__key_9,ageedit.__key_51,ageedit.__key_9", dump: "help.caption,check.caption",
    expect: ["help.caption=Press Space to finish.", "check.caption=Age: 3"] },
  { file: "gui/menus.rr", runtimes: ["run", "interp", "native", "web"], events: "wrapitem.onclick,smallitem.onclick,upperitem.onclick", dump: "status.simpletext,text.text,wrapitem.checked,largeitem.checked",
    expect: ["status.simpletext=Text size 9", "text.text=RIGHT-CLICK ME FOR THE POP-UP MENU.", "wrapitem.checked=0", "largeitem.checked=0"] },
  { file: "gui/dialogs.rr", runtimes: ["run", "interp", "native", "web"], events: "colourbtn.onclick,askbtn.onclick", dump: "answer.caption,sample.color",
    colorDialog: "65280", messageDialog: "Yes", expect: ["answer.caption=Glad to hear it!", "sample.color=65280"] },
  { file: "gui/dialogs.rr", runtimes: ["run", "interp", "native", "web"], events: "fontbtn.onclick,openbtn.onclick", dump: "answer.caption",
    fontDialog: "Courier New,14", fileDialog: "notes.txt", expect: ["answer.caption=Chose ", "notes.txt"] },
  // (Ctrl+End to the last paragraph, B pressed there — the bold "x" typed
  // next keeps it — then centred and bulleted: the tool bar follows)
  { file: "gui/richtext.rr", runtimes: ["run", "interp", "native", "web"], events: "doc.__key_35_16,boldbtn.onclick,doc.__key_88,centrebtn.onclick,bulletbtn.onclick",
    dump: "status.simpletext,boldbtn.down,centrebtn.down,leftbtn.down,bulletbtn.down",
    expect: ["status.simpletext=Line 5, column 36   MS Sans Serif 8 pt", "boldbtn.down=-1", "centrebtn.down=-1", "leftbtn.down=0", "bulletbtn.down=-1"] },
  { file: "gui/stopwatch.rr", runtimes: ["run", "interp", "native", "web"], events: `startbtn.onclick,${wait("state", 6)},lapbtn.onclick,${wait("state", 4)},lapbtn.onclick,startbtn.onclick`,
    dump: "laps.itemcount,state.caption,time.caption", expect: ["laps.itemcount=2", "state.caption=Stopped", "time.caption=0:0"] },
  { file: "gui/pantry.rr", runtimes: ["run", "interp", "native", "web"], events: "shelves.__item_1,addbtn.onclick", dump: "total.caption,grid.rowcount",
    expect: ["total.caption=Vegetables: 3 items, worth 4.80", "grid.rowcount=4"] },
  { file: "gui/themes.rr", runtimes: ["run", "interp", "native", "web"], events: "pickdark.onclick", dump: "now.caption",
    expect: ["now.caption=Theme: rapidr dark"] },
  // (THEME-LIVE) the system switches to dark while it runs (the test step
  // __look_dark): its OnThemeChange takes the card's colours again
  { file: "gui/themes.rr", runtimes: ["run", "interp", "native", "web"], events: "form.__look_dark", dump: "now.caption,card.color",
    expect: ["now.caption=Theme: rapidr dark", "card.color=3812133"] },
  { file: "gui/sales_dashboard.rr", runtimes: ["run", "interp", "native", "web"], dump: "status.caption,averagevalue.caption",
    expect: ["status.caption=All regions: 1265 thousand in 8890 orders; best month December", "averagevalue.caption=142.29"] },
  { file: "gui/sales_dashboard.rr", runtimes: ["run", "interp", "native", "web"], events: "southbtn.onclick", dump: "status.caption,bestvalue.caption",
    expect: ["status.caption=South: 577 thousand in 4060 orders; best month December", "bestvalue.caption=December (66)"] },
  { file: "gui/tray.rr", runtimes: ["run", "interp", "native", "web"], events: "hidebtn.onclick,form.__tray_513,form.__tray_514", dump: "info.caption,form.__shown",
    expect: ["info.caption=Back from the tray (1)", "form.__shown=1"] },
  // studio/
  { file: "studio/panels.rr", runtimes: ["run", "interp", "native", "web"], events: "box.__dblclick_60_170", dump: "tree.projectname,tree.filecount,designer.compcount,insp.targettype",
    expect: ["tree.projectname=Greeter", "tree.filecount=4", "designer.compcount=2", "insp.targettype=RCheckBox"] },
  // (the form designer's surface: six components added, the resize
  // preview switched on by its button)
  { file: "studio/form_designer.rr", runtimes: ["run", "interp", "native", "web"], events: "bpreview.onclick", dump: "ds.compcount,ds.previewwidth,ds.previewheight",
    expect: ["ds.compcount=6", "ds.previewwidth=440", "ds.previewheight=300"] },
  // graphics/, directx/
  { file: "graphics/canvas.rr", runtimes: ["run", "interp", "native", "web"], events: "chart.__mousedown_60_40,chart.__mouseup_60_40,chart.__mousedown_300_30,chart.__mouseup_300_30", dump: "info.caption",
    expect: ["info.caption=2 dots, last at 300,30 (002828DC)"] },
  { file: "graphics/canvas.rr", runtimes: ["run", "interp", "native", "web"], events: "clearbtn.onclick", dump: "info.caption",
    expect: ["info.caption=No dots; bar 4 is &H00B48246"] },
  { file: "directx/sprites.rr", runtimes: ["run", "interp", "native", "web"], events: `screen.__mousedown_100_100,${wait("info", 8)},pausebtn.onclick`, dump: "info.caption,pausebtn.caption",
    expect: ["info.caption=4 balls", "pausebtn.caption=&Go"] },
  { file: "directx/d3d_cube.rr", runtimes: ["run", "interp", "native", "web"], events: `fasterbtn.onclick,${wait("info", 4)}`, dump: "info.caption",
    expect: ["info.caption=6 faces"] },
  // media/
  { file: "media/midi.rr", runtimes: ["run", "interp", "native", "web"], events: `playbtn.onclick,${wait("info", 24)}`, dump: "info.caption",
    expect: ["info.caption=playing, 1 of 8 s"] },
  { file: "media/midi.rr", runtimes: ["run", "interp", "native", "web"], events: `playbtn.onclick,${wait("info", 4)},pausebtn.onclick`, dump: "info.caption",
    expect: ["info.caption=paused"] },
  { file: "media/wave.rr", runtimes: ["run", "interp", "native", "web"], events: "chimebtn.onclick", dump: "info.caption",
    expect: ["info.caption=Chime: 1200 ms, 16-bit, 11025 Hz"] },
  { file: "media/wave.rr", runtimes: ["run", "interp", "native", "web"], events: `recordbtn.onclick,${wait("info", 30)},minebtn.onclick`, dump: "info.caption",
    expect: ["info.caption=Playing my recording"] },
  { file: "media/video.rr", runtimes: ["run", "interp", "native", "web"], events: "", dump: "info.caption",
    expect: ["info.caption=24 frames of 64x48"] },
  { file: "media/video.rr", runtimes: ["run", "interp", "native", "web"], events: `playbtn.onclick,${wait("info", 50)}`, dump: "info.caption",
    expect: ["info.caption=Played to the end: 24 frames"] },
  // data/
  { file: "data/sqlite.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["  1968 | A Wizard of Earthsea", "  Ursula K. Le Guin | 2 | 4.35", "Best rated: The Hitch-Hiker's Guide to the Galaxy", "Neuromancer is now rated 4.8", "finds 0 books", "Done."] },
  { file: "data/json.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["Order 1042 for Ada in London", "First item: tea, last: jam", "Has a discount? no", "Now paid: 1, to Cambridge, items kept: no", "Read back: delivery in the afternoon, for Ada"] },
  { file: "data/numbers.rr", runtimes: ["run", "interp", "native", "web"],
    expect: ["Days: 7, mean 17.36, warmest 21.5 on day 5", "In Fahrenheit, rounded: 58,61,60,66,71,64,63", "Linspace(0, 1, 5): 0,0.25,0.5,0.75,1", "Running total: 1,5,14,30,55", "Dice faces seen: 1,2,4,5,6"] },
  { file: "data/dataframe.rr", runtimes: ["run", "interp", "native", "web"], events: "topbtn.onclick", dump: "info.caption,grid.rowcount",
    expect: ["info.caption=4 people earn over 70000; Charlie the most", "grid.rowcount=4"] },
  { file: "data/dataframe.rr", runtimes: ["run", "interp", "native", "web"], events: "engbtn.onclick,chartbtn.onclick", dump: "info.caption",
    expect: ["info.caption=Averages: 85,70,61 (thousands). Highest: Engineering"] },
  // (the sample at the start; then staff.csv dropped on the window — the
  // OnDropFiles test hook —, a scatter chart, the table sorted by Salary
  // and again the other way, "sa" typed in Filter)
  { file: "data/csv_explorer.rr", runtimes: ["run", "interp", "native", "web"], dump: "status.caption,table.itemcount,stats.itemcount,chart.title",
    expect: ["status.caption=shop.csv (sample): 12 rows, 6 columns", "table.itemcount=12", "stats.itemcount=6", "chart.title=Visitors by Month"] },
  { file: "data/csv_explorer.rr", runtimes: ["run", "interp", "native", "web"], drop: "staff.csv",
    events: "form.__drop,kindbox.__item_2,table.__mousedown_250_10,table.__mouseup_250_10,table.__mousedown_250_10,table.__mouseup_250_10,filterbox.__key_83,filterbox.__key_65",
    dump: "status.caption,form.caption,table.itemcount,chart.title,kindbox.text",
    expect: ["status.caption=staff.csv: 8 rows, 5 columns; 3 contain \"sa\"", "form.caption=CSV Explorer - staff.csv", "table.itemcount=3", "chart.title=Salary by Name", "kindbox.text=Scatter"] },
  // (Open CSV... with the dialog's answer, a line chart)
  { file: "data/csv_explorer.rr", runtimes: ["run", "interp", "native", "web"], fileDialog: "staff.csv",
    events: "openbtn.onclick,kindbox.__item_1", dump: "status.caption,chart.title,stats.itemcount",
    expect: ["status.caption=staff.csv: 8 rows, 5 columns", "chart.title=Salary by Name", "stats.itemcount=5"] },
  // (I7: linked components — a row clicked in the grid, Next, a heading
  // clicked: the bound edit and labels follow, the view sorted)
  { file: "data/linked_data.rr", runtimes: ["run", "interp", "native", "web"],
    events: "grid.__cell_1_2,nextbtn.onclick,grid.__cell_2_-1",
    dump: "nameedit.text,deptedit.text,salary.caption,status.caption,grid.row,grid.sortcolumn",
    expect: ["nameedit.text=Diana", "deptedit.text=Sales", "salary.caption=58000", "status.caption=Row 4 of 8", "grid.row=3", "grid.sortcolumn=2"] },
  // (Engineering: the frame filtered — the grid and the edits follow)
  { file: "data/linked_data.rr", runtimes: ["run", "interp", "native", "web"],
    events: "engbtn.onclick,lastbtn.onclick",
    dump: "nameedit.text,status.caption,grid.rowcount",
    expect: ["nameedit.text=Eduardo", "status.caption=Row 3 of 3", "grid.rowcount=3"] },
  // (I7: JSON records as a data source — a product picked, the bound edits
  // and label follow; grouped by category)
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"],
    events: "grid.__cell_0_4",
    dump: "status.caption,nameedit.text,stockedit.text,price.caption,grid.rowcount",
    expect: ["status.caption=Calculator (Electronics)", "nameedit.text=Calculator", "stockedit.text=40", "price.caption=12.5", "grid.rowcount=12"] },
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"],
    events: "groupbtn.onclick,grid.__cell_0_2",
    dump: "status.caption,grid.rowcount,price.caption,stockedit.text",
    expect: ["status.caption=School: 553 in stock", "grid.rowcount=4", "price.caption=28.85", "stockedit.text=553"] },
  // (I7: a big table — 20,000 rows here to keep the suite quick; the
  // example's own default is a million)
  { file: "data/million_rows.rr", runtimes: ["run", "interp", "native", "web"], args: ["20000"],
    events: "highbtn.onclick", dump: "grid.rowcount,grid.columns",
    expect: ["grid.rowcount=4383", "grid.columns=id,dept,salary,years"] },
  { file: "data/million_rows.rr", runtimes: ["run", "interp", "native", "web"], args: ["20000"],
    events: "groupbtn.onclick,grid.__cell_0_1", dump: "grid.rowcount,grid.columns,grid.row",
    expect: ["grid.rowcount=5", "grid.columns=dept,salary,years", "grid.row=1"] },
  // (EXAMPLES-DATA: where the example's folder can't be written — RapidR's
  // examples where it's installed — the window says why, nothing pretends;
  // a data file missing, the same; letters typed into a number field are
  // refused, the arrows move the row the status line shows, grouped by
  // category the first edit shows the category)
  { file: "data/million_rows.rr", runtimes: ["run", "interp", "native"], args: ["2000"], readOnly: true,
    dump: "status.caption,problem.visible,grid.visible,highbtn.enabled,problem.caption",
    expect: ["status.caption=Couldn't write big_2000.csv", "problem.visible=1", "grid.visible=0", "highbtn.enabled=0", "can't be written: it is read-only, or the disk is full"] },
  { file: "data/linked_data.rr", runtimes: ["run", "interp", "native"], readOnly: true, events: "savebtn.onclick", dump: "status.caption",
    expect: ["status.caption=Couldn't save staff_edited.csv: this folder can't be written (it is read-only, or not yours)"] },
  { file: "data/linked_data.rr", runtimes: ["run", "interp", "native", "web"], events: "savebtn.onclick", dump: "status.caption",
    expect: ["status.caption=Saved 8 rows to staff_edited.csv"] },
  { file: "data/linked_data.rr", runtimes: ["run", "interp", "native", "web"], without: ["staff.csv"], dump: "status.caption,grid.rowcount",
    expect: ["status.caption=Couldn't read the staff: there is no file \"staff.csv\"", "grid.rowcount=0"] },
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"], without: ["products.json"], dump: "status.caption,grid.rowcount",
    expect: ["status.caption=Couldn't read the products: there is no file \"products.json\"", "grid.rowcount=0"] },
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"], events: "grid.__cell_0_0,stockedit.__key_65", dump: "stockedit.dataerror,status.caption,stockedit.text",
    expect: ["stockedit.dataerror=stock is a whole number: \"a\" isn't one", "status.caption=Not changed: stock is a whole number: \"a\" isn't one", "stockedit.text=a"] },
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"], events: "grid.__cell_0_0,grid.__key_40,grid.__key_40", dump: "status.caption,grid.row,nameedit.text",
    expect: ["status.caption=Ruler (School)", "grid.row=2", "nameedit.text=Ruler"] },
  { file: "data/json_source.rr", runtimes: ["run", "interp", "native", "web"], events: "groupbtn.onclick,allbtn.onclick,groupbtn.onclick", dump: "namelbl.caption,nameedit.text,nameedit.datafield",
    expect: ["namelbl.caption=Category", "nameedit.text=Art", "nameedit.datafield=category"] },
  // network/ (the tests' own server, local)
  { file: "network/http_json.rr", runtimes: ["run", "interp", "native", "web"], args: ["{http}/examples/network/forecast.json"],
    expect: ["Forecast for Harbour Town (updated 2026-10-06 06:00)", "  Tuesday   10 to 15 C, showers", "Warmest: Monday"] },
  { file: "network/download.rr", runtimes: ["run", "interp", "native", "web"], args: ["{http}/examples/network/forecast.json"], events: "fetchbtn.onclick", dump: "info.caption",
    expect: ["info.caption=Saved 290 bytes of examples/network/forecast.json"] },
  // ai/ (the tests' mock provider, local: an OpenAI-compatible server)
  { file: "ai/rai_models.rr", runtimes: ["run", "interp", "native", "web"], args: ["{aimock}"],
    events: `listbtn.onclick,${wait("info", 4)},modelbox.__item_0,askbtn.onclick,${wait("info", 6)}`, dump: "modelbox.itemcount,modelbox.text,ai.model,answer.text,info.caption",
    expect: ["modelbox.itemcount=2", "modelbox.text=mock-small  (fast · low cost)", "ai.model=mock-small",
      "answer.text=Hello, friend, nice meeting you.", "info.caption=1,000 tokens sent, 20 received"] },
  // (AI-REAL: Anthropic played by the mock — RAPIDR_AI_TEST_SERVER — with
  // an organisation's key: refused without a workspace, the Workspace ID
  // row filled in, the list and the answer then)
  { file: "ai/rai_models.rr", runtimes: ["run", "interp", "native", "web"],
    events: `providerbox.__item_0,${typeKeys("keyedit", "mock-org-key-1")},listbtn.onclick,${wait("info", 6)},${typeKeys("wsedit", "wrk_ex_01")},listbtn.onclick,${wait("info", 6)},modelbox.__item_0,askbtn.onclick,${wait("info", 8)}`,
    dump: "wsedit.__shown,wsedit.text,modelbox.itemcount,ai.model,answer.text,listbtn.enabled",
    expect: ["wsedit.__shown=1", "wsedit.text=wrk_ex_01", "modelbox.itemcount=3", "ai.model=claude-mock-haiku-1", "answer.text=Hello, friend, nice meeting you.", "listbtn.enabled=1"] },
  // (... a key OpenAI refuses: said, List Models usable again at once ...)
  { file: "ai/rai_models.rr", runtimes: ["run", "web"],
    events: `providerbox.__item_1,${typeKeys("keyedit", "mock-bad-key-1")},listbtn.onclick,${wait("info", 6)}`,
    dump: "info.caption,listbtn.enabled",
    expect: ["info.caption=OpenAI refused the API key (401)", "listbtn.enabled=1"] },
  // (... and the key typed again: the list comes; no Workspace ID row for OpenAI)
  { file: "ai/rai_models.rr", runtimes: ["run", "web"],
    events: `providerbox.__item_1,${typeKeys("keyedit", "mock-bad-key-1")},listbtn.onclick,${wait("info", 6)},keyedit.__key_65_16,${typeKeys("keyedit", "mock-good-key-1")},listbtn.onclick,${wait("info", 6)}`,
    dump: "modelbox.itemcount,listbtn.enabled,wsedit.__shown",
    expect: ["modelbox.itemcount=3", "listbtn.enabled=1", "wsedit.__shown=0"] },
  { file: "ai/ask.rr", runtimes: ["run", "interp", "native", "web"], args: ["{aimock}"],
    expect: ["2 models; asking mock-small (fast · low cost):", "Hello, friend, nice meeting you.", "1,000 tokens sent, 20 received"] },
  // iot/: a scripted ESP32 (RAPIDR_TEST_COMPORT's `esp32`: DTR / RTS reset it, it prints a boot log)
  { file: "iot/esp32_monitor.rr", runtimes: ["run", "interp", "native", "web"], env: { RAPIDR_TEST_COMPORT: "COM5:esp32" },
    events: `connectbtn.onclick,${wait("state", 2)},resetbtn.onclick,${wait("state", 6)}`, dump: "ports.text,state.caption,log.linecount",
    expect: ["ports.text=COM5 (CP2102N USB to UART Bridge Controller)", "state.caption=Connected to COM5 at 115200   DTR 0  RTS 0", "log.linecount=9"] },
  // showcase/: the release post's programs (docs/manual/showcase.md)
  // (a rocket launched and left to burst, then the next: the count of
  // bursts says the first went off)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `launchbtn.onclick,${wait("info", 30)},launchbtn.onclick`,
    dump: "autobox.checked,info.caption", expect: ["autobox.checked=0", "info.caption=Bursts so far: 1."] },
  // (the keys: 4 picks the heart; A turns the automatic show on, and it launches)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: "form.__key_52", dump: "info.caption",
    expect: ["info.caption=Next bursts: heart"] },
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_65,${wait("info", 16)}`, dump: "autobox.checked,info.caption",
    expect: ["autobox.checked=-1", "info.caption=Bursts so far: "] },
  // (a click on the sky sends a rocket there; Space another one)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `sky.__mousedown_400_200,sky.__mouseup_400_200,${wait("info", 30)},form.__key_32`,
    dump: "info.caption", expect: ["info.caption=Bursts so far: 1."] },
  // monte_carlo_pi.rr: the speed slider's End key (300,000 darts a second),
  // a second of darts: pi is found to its first digit and the numbers say how many;
  // Pause then Reset: no darts, and the button offers to go on
  { file: "showcase/monte_carlo_pi.rr", runtimes: ["run", "interp", "native", "web"], events: `speedbar.__key_35,${wait("info", 20)}`,
    dump: "pilabel.caption,errlabel.caption,countlabel.caption,speedlabel.caption",
    expect: ["pilabel.caption=pi is about 3.1", "errlabel.caption=Off by 0.0", " darts, ", " inside the circle", "speedlabel.caption=Speed: 300,000 darts a second"] },
  { file: "showcase/monte_carlo_pi.rr", runtimes: ["run", "interp", "native", "web"], events: `${wait("info", 10)},runbtn.onclick,resetbtn.onclick`,
    dump: "pilabel.caption,errlabel.caption,runbtn.caption", expect: ["pilabel.caption=pi is about ?", "errlabel.caption=No darts yet", "runbtn.caption=&Go on"] },
  // spin3d.rr: the arrow keys (used up by the form) and the buttons change the speeds
  { file: "showcase/spin3d.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_39,form.__key_39,${wait("info", 6)}`,
    dump: "info.caption", expect: ["info.caption=Planet turning at 2.0x"] },
  { file: "showcase/spin3d.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_40,${wait("info", 6)}`,
    dump: "info.caption", expect: ["info.caption=Moon going round at 0.5x"] },
  { file: "showcase/spin3d.rr", runtimes: ["run", "interp", "native", "web"], events: `slowerbtn.onclick,slowerbtn.onclick,slowerbtn.onclick,moonfasterbtn.onclick,${wait("info", 6)}`,
    dump: "info.caption", expect: ["info.caption=Moon going round at 1.5x"] },
  { file: "showcase/spin3d.rr", runtimes: ["run", "interp", "native", "web"], events: `slowerbtn.onclick,slowerbtn.onclick,slowerbtn.onclick,${wait("info", 6)}`,
    dump: "info.caption", expect: ["info.caption=Planet turning at 0.0x"] },
  // signal_lab.rr: with the noise off the analyzer finds the two tones the
  // sliders say (438 and 683 Hz), a screen reader is told; a square wave with
  // no second tone shows its odd harmonics, the 11th folded back (aliasing)
  { file: "showcase/signal_lab.rr", runtimes: ["run", "interp", "native", "web"], events: `noisebar.__key_36,${wait("info", 30)}`,
    dump: "freqlabel.caption,freq2label.caption,amplabel.caption,noiselabel.caption,spec.accessibledescription,scope.accessibledescription",
    expect: ["freqlabel.caption=Frequency: 438 Hz", "freq2label.caption=Frequency: 683 Hz", "amplabel.caption=Amplitude: 80 %", "noiselabel.caption=Noise: 0 %",
      "spec.accessibledescription=Peaks at 438 Hz, 683 Hz", "scope.accessibledescription=438 Hz sine wave, about three cycles shown"] },
  { file: "showcase/signal_lab.rr", runtimes: ["run", "interp", "native", "web"],
    events: `noisebar.__key_36,wsquare.__mousedown_8_8,wsquare.__mouseup_8_8,tone2on.__mousedown_8_8,tone2on.__mouseup_8_8,${wait("info", 30)}`,
    dump: "freq2bar.enabled,amp2bar.enabled,spec.accessibledescription,scope.accessibledescription",
    expect: ["freq2bar.enabled=0", "amp2bar.enabled=0", "spec.accessibledescription=Peaks at 438 Hz, 1314 Hz, 2190 Hz, 3066 Hz, 3942 Hz, 3374 Hz", "scope.accessibledescription=438 Hz square wave"] },
  // (Hold freezes the screens and says Run; again, and it runs; a frequency
  // slider moved with the keyboard changes its label; the trigger switched off)
  { file: "showcase/signal_lab.rr", runtimes: ["run", "interp", "native", "web"], events: `holdbtn.onclick,freqbar.__key_39,trigbox.__mousedown_8_8,trigbox.__mouseup_8_8`,
    dump: "holdbtn.caption,freqlabel.caption,trigbox.checked", expect: ["holdbtn.caption=&Run", "freqlabel.caption=Frequency: 444 Hz", "trigbox.checked=0"] },
  { file: "showcase/signal_lab.rr", runtimes: ["run", "interp", "native", "web"], events: `holdbtn.onclick,holdbtn.onclick`, dump: "holdbtn.caption", expect: ["holdbtn.caption=&Hold"] },
  // (the song read by the program itself — 819 notes, 128 beats a minute —,
  // played a little, paused: the clock and the caption say so)
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], events: `playbtn.onclick,${wait("info", 24)},pausebtn.onclick`,
    dump: "info.caption,clock.caption", expect: ["info.caption=Neon Loop - paused", " / 1:00"] },
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], events: `playbtn.onclick,${wait("info", 12)},stopbtn.onclick`,
    dump: "info.caption,clock.caption,bar.position", expect: ["info.caption=Neon Loop - stopped", "clock.caption=0:00 / 1:00", "bar.position=0"] },
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], dump: "info.caption,clock.caption",
    expect: ["info.caption=Neon Loop - 819 notes, 128 beats a minute. Press Play.", "clock.caption=0:00 / 1:00"] },
  // (Load MIDI... with a file that isn't one: said in the window; then
  // another song, a format 0 file with no name)
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], extra: ["media/chime.wav"], fileDialog: "chime.wav", events: "loadbtn.onclick", dump: "info.caption,clock.caption",
    expect: ["info.caption=That isn't a MIDI file (it doesn't start with MThd)"] },
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], extra: ["media/tune.mid"], fileDialog: "tune.mid", events: "loadbtn.onclick", dump: "info.caption,clock.caption",
    expect: ["info.caption=Untitled song - 54 notes, 120 beats a minute. Press Play.", "clock.caption=0:00 / 0:08"] },
  // (Calm ticked with the mouse, then play)
  { file: "showcase/midi_visualizer.rr", runtimes: ["run", "interp", "native", "web"], events: `calmbox.__mousedown_8_8,calmbox.__mouseup_8_8,playbtn.onclick,${wait("info", 10)}`,
    dump: "calmbox.checked,info.caption", expect: ["calmbox.checked=-1", "info.caption=Neon Loop - 819 notes"] },
  // rapidq/: RapidQ's own way, Q names, as RC.EXE compiles it
  { file: "rapidq/notepad.bas", runtimes: ["run", "interp", "native", "web"], events: "saveitem.onclick,newitem.onclick,openitem.onclick", dump: "form.caption,editor.linecount",
    fileDialog: "note.txt;note.txt", expect: ["form.caption=Notepad - note.txt", "editor.linecount=3"] },
  // (a program in several files: its includes in lib/ and forms/, named as
  // on Windows — `Lib\Geometry.INC` is lib/geometry.inc)
  { file: "rapidq/shapes/shapes.bas", runtimes: ["run", "interp", "native", "web"], includes: true, events: "areabutton.onclick", dump: "answerlabel.caption",
    expect: ["answerlabel.caption=Circle: 12.57 square cm"] },
  // web/: what only a web page has
  { file: "web/todo.rr", runtimes: ["web"], events: "addbtn.onclick", dump: "info.caption,tasks.itemcount",
    expect: ["info.caption=1 to do, saved", "tasks.itemcount=1"] },
  { file: "web/browser.rr", runtimes: ["web"], events: "aboutbtn.onclick,homebtn.onclick", dump: "title.caption,body.caption",
    expect: ["title.caption=Home", "body.caption=Your browser speaks en-US; the window is 1920 pixels wide; 6 x 7 is 42."] },
  // the IDE (`rapidr ide`; the release builds it from here): it starts
  { file: "ide.rr", runtimes: ["run", "interp", "native"], events: "", dump: "ide.caption",
    expect: ["ide.caption=RapidR"] },
];

const ALL = ["run", "interp", "native", "web"];
const argv = process.argv.slice(2);
let runtimes = ALL;
const filters = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === "--runtimes") runtimes = argv[++i].split(",");
  else filters.push(argv[i]);
}

let failed = 0, passed = 0;
const ok = (cond, msg, detail = "") => {
  console.log(`${cond ? "✓" : "✗"} ${msg}${cond || !detail ? "" : `\n    ${detail}`}`);
  cond ? passed++ : failed++;
};

// 1. The index: every program in examples/ is a case, every case is in
// examples/README.md.
function programs(dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) return programs(p);
    return [".rr", ".bas"].includes(extname(f).toLowerCase()) ? [relative(EXAMPLES, p).split("\\").join("/")] : [];
  });
}
const readme = readFileSync(join(EXAMPLES, "README.md"), "utf8");
const listed = new Set(cases.map((c) => c.file));
if (!filters.length) {
  for (const p of programs(EXAMPLES)) ok(listed.has(p), `${p}: in the examples' test table`);
  for (const c of cases) ok(readme.includes(`(${c.file})`), `${c.file}: in examples/README.md`);
}

// 2. Running them.
const http = await startHttpServer();
// (a process of its own: the desktop runs block this one's event loop)
const aiMock = spawnAsync(process.execPath, [join(ROOT, "tests/ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
const AI_MOCK = await new Promise((resolve) => aiMock.stdout.once("data", (d) => resolve(String(d).trim())));
const env = (extra = {}) => ({
  ...process.env,
  RAPIDR_PRINT_TO: join(WORK, "prints"),
  RAPIDR_REGISTRY: join(WORK, "registry.reg"),
  RAPIDR_TEST_CLIPBOARD: "1",
  RAPIDR_TEST_SOUND: "",
  RAPIDR_TEST_MIDI: "",
  RAPIDR_TEST_COMPORT: "",
  RAPIDR_TEST_JOYSTICK: "",
  RAPIDR_TEST_WAVE_IN: "tone:440",
  // (no AI provider's key from the machine, none kept in its keychain)
  RAPIDR_AI_KEYSTORE: "memory",
  // (the cloud providers played by the mock: a loopback address only)
  RAPIDR_AI_TEST_SERVER: AI_MOCK.replace(/\/v1$/, ""),
  OPENAI_API_KEY: "",
  ANTHROPIC_API_KEY: "",
  DEEPSEEK_API_KEY: "",
  ...extra,
});
const isGui = (c) => c.dump !== undefined;
const args = (c) => (c.args || []).map((a) => a.replace("{http}", `http://${http.address}`).replace("{aimock}", AI_MOCK));

/// The hooks' environment for a GUI case (the desktop's names; the web
/// takes the same through rapidr_set_test_env).
function hooks(c, work) {
  if (!isGui(c)) return {};
  const h = { RAPIDR_CAPTURE: join(work, "window"), RAPIDR_TEST_EVENTS: c.events || "", RAPIDR_TEST_DUMP: c.dump };
  const opt = { fileDialog: "RAPIDR_TEST_FILE_DIALOG", colorDialog: "RAPIDR_TEST_COLOR_DIALOG", fontDialog: "RAPIDR_TEST_FONT_DIALOG", messageDialog: "RAPIDR_TEST_MESSAGE_DIALOG", delay: "RAPIDR_CAPTURE_DELAY", drop: "RAPIDR_TEST_DROP" };
  for (const [k, v] of Object.entries(opt)) if (c[k] !== undefined) h[v] = String(c[k]);
  return h;
}

/// The example's folder copied to a fresh work folder (what it reads and
/// writes stays there; a native build's outputs too).
function workCopy(c, runtime) {
  const work = join(WORK, `${c.file.replace(/[/.]/g, "_")}-${runtime}`);
  if (existsSync(work)) chmodSync(work, 0o755);
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  const dir = join(EXAMPLES, dirname(c.file));
  for (const f of readdirSync(dir)) {
    if ((c.without || []).includes(f) || /\.tmp\.|^big(_\d+)?\.csv$/.test(f)) continue;
    if (statSync(join(dir, f)).isFile() || c.includes) cpSync(join(dir, f), join(work, f), { recursive: true });
  }
  for (const e of c.extra || []) cpSync(join(EXAMPLES, e), join(work, basename(e)));
  return work;
}

function spawn(cmd, cmdArgs, opts) {
  const r = spawnSync(cmd, cmdArgs, { encoding: "utf8", timeout: 60000, maxBuffer: 1 << 22, ...opts });
  const why = r.error ? ` (${r.error.code || r.error.message})` : r.status ? ` (exit ${r.status})` : "";
  return { out: (r.stdout || "") + (r.stderr || ""), ok: !r.error && r.status === 0, why };
}

function runDesktop(c, runtime) {
  const work = workCopy(c, runtime);
  const src = join(work, basename(c.file));
  const stem = basename(c.file, extname(c.file));
  // (readOnly: the captures go elsewhere, the folder can't be written)
  const shots = c.readOnly ? `${work}-shots` : work;
  if (c.readOnly) mkdirSync(shots, { recursive: true });
  const runEnv = env({ ...hooks(c, shots), ...(c.env || {}) });
  const runOpts = { cwd: work, env: runEnv, input: c.input || "", timeout: c.timeout || 60000 };
  const readOnly = (fn) => {
    if (c.readOnly) chmodSync(work, 0o555);
    try {
      return fn();
    } finally {
      if (c.readOnly) chmodSync(work, 0o755);
    }
  };
  if (runtime === "run") {
    const r = readOnly(() => spawn(RAPIDR, ["run", src, ...args(c)], runOpts));
    // (RAPIDR_SHOTS=dir: the windows as captured kept, to look at —
    // <example>-<case>-<window>@<scale>x.bmp; RAPIDR_SCALE sets the scale)
    if (process.env.RAPIDR_SHOTS && isGui(c)) {
      mkdirSync(process.env.RAPIDR_SHOTS, { recursive: true });
      const scale = process.env.RAPIDR_SCALE || "1";
      for (const f of readdirSync(shots).filter((f) => /^window-\d+\.bmp$/.test(f))) {
        const n = f.match(/\d+/)[0];
        cpSync(join(shots, f), join(process.env.RAPIDR_SHOTS, `${c.file.replace(/[/.]/g, "_")}-${cases.indexOf(c)}-${n}@${scale}x.bmp`));
      }
    }
    return r;
  }
  const native = runtime === "native";
  const out = c.readOnly ? `${work}-${runtime}-out` : join(work, `${stem}-${runtime}`);
  // (a native build: debug, quick to compile; either kind's executable in
  // the output folder)
  const build = spawn(RAPIDR, ["build", src, out, "--no-bundle", ...(native ? ["--debug"] : ["--interp"])], {
    cwd: work, env: env({ CARGO_TARGET_DIR: CARGO_TARGET, RAPIDR_BUILD_CACHE: join(dirname(CARGO_TARGET), "build-cache") }), timeout: 1800000,
  });
  const exe = join(out, `${stem}${EXE}`);
  let r;
  if (!build.ok || !existsSync(exe)) r = { out: build.out.split("\n").filter((l) => /^error|error:/.test(l)).slice(0, 8).join("\n") || build.out.slice(-800), ok: false, why: ` (build failed${build.why})` };
  else r = readOnly(() => spawn(exe, args(c), runOpts));
  // (the native build's ~350 MB in the shared target, gone)
  if (native) dropBuild(CARGO_TARGET, stem);
  rmSync(out, { recursive: true, force: true });
  return r;
}

/// Every source file under `dir`, by its path from it (`lib/geometry.inc`):
/// what a program's `$INCLUDE`s find on the web.
function sourcesUnder(dir, prefix = "") {
  const out = {};
  for (const f of readdirSync(join(dir, prefix))) {
    const rel = prefix ? `${prefix}/${f}` : f;
    if (statSync(join(dir, rel)).isDirectory()) Object.assign(out, sourcesUnder(dir, rel));
    else if ([".rr", ".bas", ".inc", ".rqw", ".rqb", ".rq"].includes(extname(f).toLowerCase())) out[rel] = readFileSync(join(dir, rel), "utf8");
  }
  return out;
}

// The web: the example compiled by the web runtime's compiler, its folder's
// files its assets, run on tests/web_kernel.html.
let browser = null;
async function runWeb(c) {
  if (!browser) browser = await (await import("playwright")).chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  const lines = [];
  const errors = [];
  page.on("console", (m) => { if (m.type() === "log") lines.push(m.text()); });
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const query = args(c).length ? `?${args(c).join(" ")}` : "";
    await page.goto(`${URL_BASE}/tests/web_kernel.html${query}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rrReady, null, { timeout: 20000 });
    const dir = join(EXAMPLES, dirname(c.file));
    const assets = Object.fromEntries(readdirSync(dir).filter((f) => statSync(join(dir, f)).isFile() && !(c.without || []).includes(f) && ![".rr", ".bas", ".md"].includes(extname(f).toLowerCase()))
      .map((f) => [f, "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64")]));
    for (const e of c.extra || []) assets[basename(e)] = "data:application/octet-stream;base64," + readFileSync(join(EXAMPLES, e)).toString("base64");
    const env = { ...hooks(c, "web"), ...(isGui(c) ? { RAPIDR_CAPTURE: "web" } : {}), RAPIDR_AI_TEST_SERVER: AI_MOCK.replace(/\/v1$/, ""), ...(c.env || {}) };
    await page.evaluate(({ source, assets, env, gui, main, files }) => {
      window.__rapidr_assets = assets;
      window.RAPIDR_TEST_MIDI = ""; window.RAPIDR_TEST_WAVE_IN = "tone:440";
      if (env.RAPIDR_TEST_COMPORT) window.RAPIDR_TEST_COMPORT = env.RAPIDR_TEST_COMPORT;
      const bc = files ? window.rr.compile_files(main, files, assets) : window.rr.compile(source, "example", assets);
      if (gui) window.rr.rapidr_set_test_env(env);
      window.rr.rapidr_run_bc(bc);
    }, { source: readFileSync(join(EXAMPLES, c.file), "utf8"), assets, env, gui: isGui(c), main: basename(c.file), files: c.includes ? sourcesUnder(dir) : null });
    let dump = [];
    if (isGui(c)) {
      await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: c.timeout || 60000, polling: 100 });
      dump = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results())).dump;
    } else {
      await page.waitForFunction(() => window.rr.rapidr_main_done(), null, { timeout: c.timeout || 60000, polling: 100 });
      await page.waitForTimeout(200);
    }
    return { out: [...lines, ...dump].join("\n"), ok: errors.length === 0, why: errors.length ? ` (page errors: ${errors.join("; ")})` : "" };
  } catch (e) {
    return { out: [...lines, ...errors].join("\n"), ok: false, why: ` (${String(e.message).split("\n")[0]})` };
  } finally {
    await page.close();
  }
}

rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.file.includes(f)))) {
  for (const runtime of c.runtimes.filter((r) => runtimes.includes(r))) {
    const r = runtime === "web" ? await runWeb(c) : runDesktop(c, runtime);
    const missing = c.expect.filter((l) => !r.out.includes(l));
    ok(r.ok && !missing.length, `${c.file} (${runtime})`,
      `${r.why}${missing.length ? ` missing: ${missing.map((m) => JSON.stringify(m)).join(", ")}` : ""}\n    got: ${r.out.trim().split("\n").slice(-12).join("\n         ")}`);
    if (c.sameFile && runtime === "run") {
      const made = join(WORK, `${c.file.replace(/[/.]/g, "_")}-${runtime}`, c.sameFile);
      const shipped = join(EXAMPLES, dirname(c.file), c.sameFile);
      ok(existsSync(made) && readFileSync(made).equals(readFileSync(shipped)), `${c.file}: ${c.sameFile} is made again byte for byte`);
    }
  }
}
if (browser) await browser.close();
http.close();
aiMock.kill();
console.log(`\nExamples: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
