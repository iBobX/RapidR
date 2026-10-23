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
import { chmodSync, cpSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync } from "node:fs";
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
//   reload:  (web) { events, dump }: the page opened again — the browser's
//            storage kept — and the program run again; `expect` is of that run
//   extra:   files of other examples' folders (`media/tune.mid`) copied beside it
//   elsewhere: (interp, native) built outside the example's folder and
//            started from another, empty one, the example's folder hidden
//            meanwhile: what it needs (lang/*.po …) must have come with the build
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
  // (code blocks coloured by their language — sql, basic — in each theme)
  { file: "gui/markdown_code.rr", runtimes: ["run", "interp", "native", "web"], events: "themebtn.onclick", dump: "themelabel.caption,view.linkcount",
    expect: ["themelabel.caption=Now: rapidr dark", "view.linkcount=0"] },
  { file: "gui/hello_form.rr", runtimes: ["run", "interp", "native", "web"], events: "greetbutton.onclick,greetbutton.onclick", dump: "answer.caption",
    expect: ["answer.caption=Hello, World! (2)"] },
  // (Tab out of each box: its OnExit checks it, the next one's OnEnter
  // shows its help)
  { file: "gui/focus_events.rr", runtimes: ["run", "interp", "native", "web"], events: "nameedit.__key_65,nameedit.__key_9,ageedit.__key_51,ageedit.__key_9", dump: "help.caption,check.caption",
    expect: ["help.caption=Press Space to finish.", "check.caption=Age: 3"] },
  // (a label given TabStop is a link: Enter on the second, Space on the
  // third — the keys go to the link, which takes the focus)
  { file: "gui/links.rr", runtimes: ["run", "interp", "native", "web"], events: "link2.__key_13", dump: "answer.caption",
    expect: ["answer.caption=Opened pantry.rr"] },
  { file: "gui/links.rr", runtimes: ["run", "interp", "native", "web"], events: "link3.__key_32", dump: "answer.caption",
    expect: ["answer.caption=Opened stopwatch.rr"] },
  // (Add a point twice: the score 2, marked changed; the values' plain text)
  { file: "gui/value_tree.rr", runtimes: ["run", "interp", "native", "web"], events: "addbtn.onclick,addbtn.onclick", dump: "said.caption,values.text",
    expect: ["said.caption=Score = 2 (changed)", "\tBackground = -2147483633", "\tScores = Array(1 TO 10)"] },
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
  // (your own SVG icons: four in the tree's image list and one in an RImage,
  // all $RESOURCEs — `elsewhere`: a built app started from another folder
  // still has them; the theme switched to dark, they're drawn again)
  { file: "gui/svg_icons.rr", runtimes: ["run", "interp", "native", "web"], includes: true, elsewhere: true, events: "pickdark.onclick", dump: "now.caption,tree.itemcount,icons.count,picture.width",
    expect: ["now.caption=Theme: rapidr dark", "tree.itemcount=5", "icons.count=4", "picture.width=48"] },
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
  // translations/ (TR$ / TRN$ with lang/*.po beside it — `includes`: the
  // folder copied whole; on the web its lang/ files are the page's): it
  // starts in the language given, the list switches it, OnLanguageChange
  // writes the captions again. Built executables carry lang/ (T1b):
  // `elsewhere`, started from another folder with the example's gone.
  { file: "translations/languages.rr", runtimes: ["run", "interp", "native", "web"], includes: true, elsewhere: true, args: ["es"], dump: "form.caption,greeting.caption,filecount.caption,savebutton.caption",
    expect: ["form.caption=Idiomas", "greeting.caption=¡Hola, Ana!", "filecount.caption=1 archivo", "savebutton.caption=&Guardar"] },
  { file: "translations/languages.rr", runtimes: ["run", "interp", "web"], includes: true, elsewhere: true, args: ["en"], events: "languages.__item_2,addbutton.onclick,addbutton.onclick", dump: "greeting.caption,filecount.caption,addbutton.caption",
    expect: ["greeting.caption=Olá, Ana!", "filecount.caption=3 arquivos", "addbutton.caption=&Adicionar um arquivo"] },
  { file: "translations/languages.rr", runtimes: ["run", "web"], includes: true, args: ["zh-Hans"], events: "addbutton.onclick", dump: "greeting.caption,filecount.caption",
    expect: ["greeting.caption=你好，Ana！", "filecount.caption=2 个文件"] },
  { file: "translations/languages.rr", runtimes: ["run", "interp", "web"], includes: true, elsewhere: true, args: ["pt-BR"], dump: "savebutton.caption,languages.itemindex",
    expect: ["savebutton.caption=&Salvar", "languages.itemindex=2"] },
  // recipes/ (the short programs RapidR Studio's assistant starts from: each
  // clicked as a user would)
  { file: "recipes/calculator.rr", runtimes: ["run", "interp", "native", "web"], events: "k2.onclick,kadd.onclick,k3.onclick,kequals.onclick", dump: "display.caption",
    expect: ["display.caption=5"] },
  { file: "recipes/calculator.rr", runtimes: ["run", "interp", "native", "web"], events: "k9.onclick,ksqrt.onclick,kmul.onclick,k3.onclick,k0.onclick,ksin.onclick,kequals.onclick", dump: "display.caption",
    expect: ["display.caption=1.5"] },
  { file: "recipes/calculator.rr", runtimes: ["run", "interp", "native", "web"], events: "k7.onclick,kdiv.onclick,k0.onclick,kequals.onclick", dump: "display.caption",
    expect: ["display.caption=Error"] },
  // (100 typed as the bill, the tip's 15 replaced by 20 — Ctrl+A, then typed —, 4 people)
  { file: "recipes/tip_calculator.rr", runtimes: ["run", "interp", "native", "web"],
    events: `${typeKeys("billedit", "100")},tipedit.__key_65_16,${typeKeys("tipedit", "20")},peopleedit.__key_65_16,${typeKeys("peopleedit", "4")}`, dump: "tipamount.caption,total.caption,each.caption",
    expect: ["tipamount.caption=Tip: 20.00", "total.caption=Total: 120.00", "each.caption=Each pays: 30.00"] },
  { file: "recipes/stopwatch.rr", runtimes: ["run", "interp", "native", "web"], events: `startbtn.onclick,${wait("form", 12)},startbtn.onclick`, dump: "startbtn.caption,display.caption",
    expect: ["startbtn.caption=&Start", "display.caption=0:00."] },
  { file: "recipes/stopwatch.rr", runtimes: ["run", "interp", "native", "web"], events: `startbtn.onclick,${wait("form", 12)},startbtn.onclick,resetbtn.onclick`, dump: "display.caption",
    expect: ["display.caption=0:00.0"] },
  // (two tasks added, the first removed; Remove with nothing picked says so)
  { file: "recipes/todo_list.rr", runtimes: ["run", "interp", "native", "web"],
    events: `${typeKeys("taskedit", "buy milk")},addbtn.onclick,${typeKeys("taskedit", "call bob")},addbtn.onclick,tasklist.__item_0,removebtn.onclick`, dump: "tasklist.itemcount,tasklist.text,status.caption,taskedit.text",
    expect: ["tasklist.itemcount=1", "tasklist.text=call bob", "status.caption=1 task", "taskedit.text="] },
  { file: "recipes/todo_list.rr", runtimes: ["run", "interp", "native", "web"], events: "removebtn.onclick", dump: "status.caption",
    expect: ["status.caption=Pick a task to remove."] },
  { file: "recipes/unit_converter.rr", runtimes: ["run", "interp", "native", "web"], events: `valueedit.__key_65_16,${typeKeys("valueedit", "10")},convertbtn.onclick`, dump: "resultlabel.caption",
    expect: ["resultlabel.caption== 6.21 miles"] },
  { file: "recipes/unit_converter.rr", runtimes: ["run", "interp", "native", "web"], events: `kindbox.__item_2,valueedit.__key_65_16,${typeKeys("valueedit", "100")}`, dump: "resultlabel.caption",
    expect: ["resultlabel.caption== 212.00 F"] },
  // (Paris right, Seoul wrong, Lima right: 2 of 3; then Play again)
  { file: "recipes/quiz_game.rr", runtimes: ["run", "interp", "native", "web"], events: "choice2.onclick,choice2.onclick,choice3.onclick", dump: "questionlabel.caption,scorelabel.caption,feedback.caption,againbtn.visible",
    expect: ["questionlabel.caption=Done! You got 2 of 3.", "scorelabel.caption=Score: 2", "feedback.caption=Right!", "againbtn.visible=1"] },
  { file: "recipes/quiz_game.rr", runtimes: ["run", "interp", "native", "web"], events: "choice1.onclick", dump: "questionlabel.caption,feedback.caption",
    expect: ["questionlabel.caption=Question 2 of 3: What is the capital of Japan?", "feedback.caption=No: it's Paris."] },
  { file: "recipes/drawing_pad.rr", runtimes: ["run", "interp", "native", "web"],
    events: "redbtn.onclick,pad.__mousedown_30_30,pad.__mousemove_80_60,pad.__mousemove_140_110,pad.__mouseup_140_110,pad.__mousedown_200_40,pad.__mouseup_220_60", dump: "status.caption",
    expect: ["status.caption=2 strokes"] },
  { file: "recipes/password_generator.rr", runtimes: ["run", "interp", "native", "web"], events: `lengthedit.__key_65_16,${typeKeys("lengthedit", "20")},generatebtn.onclick`, dump: "info.caption",
    expect: ["info.caption=20 characters"] },
  { file: "recipes/csv_viewer.rr", runtimes: ["run", "interp", "native", "web"], fileDialog: "people.csv", events: "openbtn.onclick", dump: "grid.rowcount,grid.colcount,status.caption",
    expect: ["grid.rowcount=4", "grid.colcount=3", "status.caption=people.csv: 3 rows"] },
  // (the second site picked: started, the first stopped; Restart with none picked)
  { file: "recipes/server_manager.rr", runtimes: ["run", "interp", "native", "web"], events: "sitelist.__item_1,startbtn.onclick,sitelist.__item_0,stopbtn.onclick", dump: "sitelist.text,status.caption",
    expect: ["www.example.com  -  stopped", "shop.example.com  -  running", "status.caption=www.example.com is stopped."] },
  { file: "recipes/server_manager.rr", runtimes: ["run", "interp", "native", "web"], events: "sitelist.__item_2,restartbtn.onclick", dump: "status.caption",
    expect: ["status.caption=blog.example.com restarted."] },
  // studio/
  { file: "studio/panels.rr", runtimes: ["run", "interp", "native", "web"], events: "box.__dblclick_60_170", dump: "tree.projectname,tree.filecount,designer.compcount,insp.targettype",
    expect: ["tree.projectname=Greeter", "tree.filecount=4", "designer.compcount=2", "insp.targettype=RCheckBox"] },
  // (the form designer's surface: six components added, the resize
  // preview switched on by its button)
  { file: "studio/form_designer.rr", runtimes: ["run", "interp", "native", "web"], events: "bpreview.onclick", dump: "ds.compcount,ds.previewwidth,ds.previewheight",
    expect: ["ds.compcount=6", "ds.previewwidth=440", "ds.previewheight=300"] },
  // (RDiffView's views, Versions G2: the text view at the start, then the
  // table; the form view where the designer reads forms — `rapidr run`
  // and the web; a built executable has no designer and says so)
  { file: "studio/diff_views.rr", runtimes: ["run", "interp", "native", "web"], dump: "diff.kind,diff.hunkcount,info.caption",
    expect: ["diff.kind=text", "diff.hunkcount=1", "info.caption=1 change(s): Line 2 changed"] },
  { file: "studio/diff_views.rr", runtimes: ["run", "interp", "native", "web"], events: "btable.onclick", dump: "diff.kind,diff.changecount,info.caption",
    expect: ["diff.kind=table", "diff.changecount=4", "info.caption=4 change(s): Added column: City ..."] },
  { file: "studio/diff_views.rr", runtimes: ["run", "web"], events: "bform.onclick", dump: "diff.kind,diff.changecount,info.caption",
    expect: ["diff.kind=form", "diff.changecount=2", "info.caption=2 change(s): Go: Caption \"OK\" → \"Say hello\"; Width = 110 (new) ..."] },
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
  // (RSecrets in the test keystore — the runner's RAPIDR_AI_KEYSTORE=memory,
  // never the keychain: a password typed, saved, read back; an empty one
  // refused; deleted twice; in a browser every button says there's no keychain)
  { file: "data/secrets.rr", runtimes: ["run", "interp", "native"], events: `${typeKeys("valueedit", "s3c")},savebtn.onclick,readbtn.onclick`, dump: "status.caption,keptin.caption,valueedit.text",
    expect: ["status.caption=Read back: 3 characters.", "keptin.caption=Kept in memory, for this run only (the test keystore)", "valueedit.text=s3c"] },
  { file: "data/secrets.rr", runtimes: ["run", "interp", "native"], events: "savebtn.onclick", dump: "status.caption",
    expect: ["status.caption=Not saved: the value is empty (Delete removes a secret)"] },
  { file: "data/secrets.rr", runtimes: ["run", "interp", "native"], events: `${typeKeys("valueedit", "s3c")},savebtn.onclick,deletebtn.onclick,deletebtn.onclick`, dump: "status.caption,valueedit.text",
    expect: ["status.caption=There was nothing to delete.", "valueedit.text="] },
  { file: "data/secrets.rr", runtimes: ["web"], events: `${typeKeys("valueedit", "s3c")},savebtn.onclick`, dump: "status.caption,keptin.caption",
    expect: ["status.caption=Not saved: RSecrets isn't available in the browser: a web page has no keychain", "keptin.caption=Nowhere to keep it: no keychain (a web page has none)"] },
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
  // (REQUEST-001: background queries — 20,000 rows here, a million by
  // default; a Ping answered while the table is made, a heading's click
  // runs the query again sorted, End goes to the last row; Ctrl+Enter in
  // the SQL editor runs the statement at the caret — here before there's
  // a table: SQLite's message; Cancel stops making 3,000,000 rows)
  { file: "data/million_rows_async.rr", runtimes: ["run", "interp", "native", "web"], args: ["20000"],
    events: "makebtn.onclick,pingbtn.onclick,db.__until_busy_0,grid.__cell_1_-1,db.__until_busy_0,grid.__do_end",
    dump: "db.rowcount,grid.rowcount,grid.sortcolumn,grid.row,pinglabel.caption,status.caption,cancelbtn.enabled,runbtn.enabled",
    expect: ["db.rowcount=20000", "grid.rowcount=20000", "grid.sortcolumn=1", "grid.row=19999", "pinglabel.caption=Pings: 1", "status.caption=20000 rows in", "cancelbtn.enabled=0", "runbtn.enabled=1"] },
  { file: "data/million_rows_async.rr", runtimes: ["run", "interp", "native", "web"],
    events: "editor.__key_13_16", dump: "status.caption",
    expect: ["status.caption=SQLite says: no such table: big"] },
  { file: "data/million_rows_async.rr", runtimes: ["run", "interp", "native"], args: ["3000000"],
    events: "makebtn.onclick,cancelbtn.onclick,db.__until_busy_0", dump: "status.caption,db.busy,makebtn.enabled",
    expect: ["status.caption=Cancelled: the table is as it was.", "db.busy=0", "makebtn.enabled=1"] },
  // (DB-D: InTransaction follows BEGIN, a row, ROLLBACK; closing with a
  // transaction open asks — Enter's answer, No, keeps the window)
  { file: "data/transactions.rr", runtimes: ["run", "interp", "native", "web"], messageDialog: "__enter",
    events: "beginbtn.onclick,addbtn.onclick,addbtn.onclick,form.__close", dump: "status.caption,form.__shown,db.intransaction,grid.rowcount,commitbtn.enabled,beginbtn.enabled",
    expect: ["status.caption=Still open: Commit keeps the rows.", "form.__shown=1", "db.intransaction=1", "grid.rowcount=2", "commitbtn.enabled=-1", "beginbtn.enabled=0"] },
  { file: "data/transactions.rr", runtimes: ["run", "interp", "native", "web"],
    events: "addbtn.onclick,beginbtn.onclick,addbtn.onclick,rollbackbtn.onclick", dump: "status.caption,db.intransaction,grid.rowcount,commitbtn.enabled",
    expect: ["status.caption=No transaction: each row is kept as it's added.", "db.intransaction=0", "grid.rowcount=1", "commitbtn.enabled=0"] },
  // (a query of the user's own after a sorted one: unsorted, shown from the top)
  { file: "data/million_rows_async.rr", runtimes: ["run", "interp", "web"], args: ["5000"],
    events: "makebtn.onclick,db.__until_busy_0,grid.__do_end,grid.__cell_1_-1,db.__until_busy_0,runbtn.onclick,db.__until_busy_0",
    dump: "grid.sortcolumn,grid.row,grid.toprow,grid.rowcount",
    expect: ["grid.sortcolumn=-1", "grid.row=0", "grid.toprow=0", "grid.rowcount=5000"] },
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
  // (servers: on 127.0.0.1, a port of the suite's, tests/http_client_probe.mjs
  // their client) echo_server.rr: RServerSocket's Mode "http" — 1 MB by
  // Content-Length and chunked, each back whole; it ends after 3 requests
  { file: "network/echo_server.rr", runtimes: ["run", "interp", "native"], args: ["{port}", "3"], client: { mode: "echo" },
    expect: ["client: echo word 200 same", "client: echo 1 MB 200 same", "client: echo 1 MB chunked 200 same", "1048576 characters", "Served 3 requests."] },
  // local_api.rr: RHttpServer refusing what it must by itself (no or a
  // wrong token, another site's page, another host name, too large), its
  // answers, three Server-Sent Events
  { file: "network/local_api.rr", runtimes: ["run", "interp", "native"], args: ["{port}", "suite-token"], client: { mode: "api", args: ["suite-token"] },
    events: wait("log", 140), dump: "status.caption,urllabel.caption",
    expect: ["client: no token 401", "client: wrong token 401", "client: foreign origin 403", "client: foreign host 403", "client: cross-site 403", "client: hello 200 Hello from RapidR",
      "client: echo 200 application/json same", 'client: events 200 text/event-stream 3 {"tick":1} {"tick":2} {"tick":3}', "client: too large 413", "client: not found 404",
      "status.caption=4 requests answered", "(this computer only)"] },
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
  // (make_song.rr writes song.mid with a file stream: the same bytes as
  // the one shipped, which midi_visualizer.rr plays)
  { file: "showcase/make_song.rr", runtimes: ["run", "interp", "native", "web"], without: ["song.mid"], sameFile: "song.mid",
    expect: ["Wrote song.mid: 7120 bytes, 32 bars, 5 tracks, 128 beats a minute."] },
  // (Space launches a rocket; left to burst, the count in the line under the
  // sky says it went off)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 30)}`,
    dump: "info.caption", expect: ["info.caption=Bursts: 1   Shape: any   Auto show: off"] },
  // (the keys: 4 picks the heart; A turns the automatic show on, and it launches)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: "form.__key_52", dump: "info.caption",
    expect: ["info.caption=Bursts: 0   Shape: heart   Auto show: off"] },
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_65,${wait("info", 40)}`, dump: "info.caption",
    expect: ["Auto show: on"] },
  // (a click on the sky sends a rocket there)
  { file: "showcase/fireworks.rr", runtimes: ["run", "interp", "native", "web"], events: `sky.__mousedown_400_200,sky.__mouseup_400_200,${wait("info", 30)}`,
    dump: "info.caption", expect: ["info.caption=Bursts: 1 "] },
  // notes.rr: the first note read and shown (96 words, 21 lines, one link); a letter typed (the
  // preview follows, the note is "changed"); New asks first: No throws the note away, Cancel keeps it;
  // Save as then New then Open brings the saved note back; Ctrl+B with nothing selected puts the
  // two marks in and the caret between them
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], events: `${wait("info", 8)}`,
    dump: "status.simpletext,form.caption,preview.linkcount,preview.plaintext",
    expect: ["status.simpletext=Untitled  (saved)   96 words, 21 lines.", "form.caption=Notes - Untitled", "preview.linkcount=1", "Write Markdown on the left. It shows on the right, as you type."] },
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], events: `editor.__key_65,${wait("info", 8)}`,
    dump: "status.simpletext,form.caption,preview.plaintext", expect: ["status.simpletext=Untitled  (changed, not saved)", "form.caption=Notes - Untitled *", "preview.plaintext=a# Notes"] },
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], messageDialog: "No", events: `editor.__key_65,newbtn.onclick,${wait("info", 8)}`,
    dump: "status.simpletext,form.caption,editor.text", expect: ["status.simpletext=Untitled  (saved)   0 words, 0 lines.", "form.caption=Notes - Untitled"] },
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], messageDialog: "Cancel", events: `editor.__key_65,newbtn.onclick,${wait("info", 8)}`,
    dump: "status.simpletext,form.caption", expect: ["status.simpletext=Untitled  (changed, not saved)   96 words, 21 lines.", "form.caption=Notes - Untitled *"] },
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], fileDialog: "note.md;note.md", events: `saveasbtn.onclick,newbtn.onclick,${wait("info", 8)},openbtn.onclick,${wait("info", 8)}`,
    dump: "status.simpletext,form.caption,preview.linkcount", expect: ["status.simpletext=note.md  (saved)   96 words, 21 lines.", "form.caption=Notes - note.md", "preview.linkcount=1"] },
  { file: "showcase/notes.rr", runtimes: ["run", "interp", "native", "web"], events: `bolditem.onclick,${wait("info", 4)}`,
    dump: "editor.selstart,form.caption", expect: ["editor.selstart=2", "form.caption=Notes - Untitled *"] },
  // (the web: the note is kept in the browser's storage — the page opened again, the typed letter is
  // still there, the file name too)
  { file: "showcase/notes.rr", runtimes: ["web"], events: `editor.__key_65,${wait("info", 16)}`, dump: "status.simpletext",
    reload: { events: `${wait("info", 8)}`, dump: "status.simpletext,preview.plaintext" },
    expect: ["Kept in this browser.", "preview.plaintext=a# Notes"] },
  // snake.rr: Space starts it; the first apple is straight ahead (7 steps of 140 ms, the wall 17), so
  // between one second and 2.4 seconds on the score is 1 and it is still going (28 events: 1.4 s, with room
  // for a busy machine); unsteered the snake reaches the wall and the game is over (Best says 1), Space
  // starts again at 0; P pauses; a turn (Down) avoids the apple and ends at the bottom wall with 0;
  // a click on the board starts it too
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 28)}`,
    dump: "scorelabel.caption,statelabel.caption", expect: ["scorelabel.caption=Score 1   Best 0   Speed 1", "statelabel.caption=Playing. Space or P pauses."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 90)}`,
    dump: "scorelabel.caption,statelabel.caption", expect: ["scorelabel.caption=Score 1   Best 1   Speed 1", "statelabel.caption=Game over: you scored 1. Press Space for another game."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 90)},form.__key_32,${wait("info", 4)}`,
    dump: "scorelabel.caption,statelabel.caption", expect: ["scorelabel.caption=Score 0   Best 1   Speed 1", "statelabel.caption=Playing. Space or P pauses."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 6)},form.__key_80`,
    dump: "statelabel.caption", expect: ["statelabel.caption=Paused. Space or P goes on."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `form.__key_32,${wait("info", 6)},form.__key_40,${wait("info", 80)}`,
    dump: "scorelabel.caption,statelabel.caption", expect: ["scorelabel.caption=Score 0   Best 0   Speed 1", "statelabel.caption=Game over: you scored 0."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], events: `board.__mousedown_100_100,board.__mouseup_100_100,${wait("info", 4)}`,
    dump: "statelabel.caption", expect: ["statelabel.caption=Playing. Space or P pauses."] },
  // (C lets the computer play: it eats the apple straight ahead and goes on; an arrow key takes over)
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], args: ["7"], events: `form.__key_32,form.__key_67,${wait("info", 40)}`,
    dump: "scorelabel.caption,statelabel.caption", expect: ["scorelabel.caption=Score ", "statelabel.caption=The computer is playing. Any arrow key takes over; C stops it."] },
  { file: "showcase/snake.rr", runtimes: ["run", "interp", "native", "web"], args: ["7"], events: `form.__key_32,form.__key_67,${wait("info", 10)},form.__key_40`,
    dump: "statelabel.caption", expect: ["statelabel.caption=Playing. Space or P pauses."] },
  // same_numbers.rr: the four sets have the same numbers, measured by RNum
  // from the points drawn — as they open, and after New points (other points,
  // the same numbers) —; the line switched off with the mouse
  { file: "showcase/same_numbers.rr", runtimes: ["run", "interp", "native", "web"], events: `${wait("info", 3)}`,
    dump: "nums(0).caption,nums(1).caption,nums(2).caption,nums(3).caption,chart(3).title,linebox.checked",
    expect: [0, 1, 2, 3].map((k) => `nums(${k}).caption=mean x 10.00   mean y 6.00\nspread x 3.00   spread y 2.00\ncorrelation 0.800   line y = 0.667 + 0.533 x`).concat(["chart(3).title=D: a column, and one point far off", "linebox.checked=-1"]) },
  { file: "showcase/same_numbers.rr", runtimes: ["run", "interp", "native", "web"], events: `newbtn.onclick,newbtn.onclick,linebox.__mousedown_8_8,linebox.__mouseup_8_8,${wait("info", 3)}`,
    dump: "nums(1).caption,nums(2).caption,linebox.checked,chart(0).count",
    expect: [1, 2].map((k) => `nums(${k}).caption=mean x 10.00   mean y 6.00\nspread x 3.00   spread y 2.00\ncorrelation 0.800   line y = 0.667 + 0.533 x`).concat(["linebox.checked=0", "chart(0).count=1"]) },
  // monte_carlo_pi.rr: the speed slider's End key (300,000 darts a second),
  // a second of darts: pi is found to its first digit and the numbers say how many;
  // Pause then Reset: no darts, and the button offers to go on
  { file: "showcase/monte_carlo_pi.rr", runtimes: ["run", "interp", "native", "web"], events: `speedbar.__key_35,${wait("info", 20)}`,
    dump: "pilabel.caption,errlabel.caption,countlabel.caption,speedlabel.caption",
    expect: ["pilabel.caption=pi is about 3.1", "errlabel.caption=Off by 0.0", " darts, ", " inside the circle", "speedlabel.caption=Speed: 300,000 darts a second"] },
  { file: "showcase/monte_carlo_pi.rr", runtimes: ["run", "interp", "native", "web"], events: `${wait("info", 10)},runbtn.onclick,resetbtn.onclick`,
    dump: "pilabel.caption,errlabel.caption,runbtn.caption", expect: ["pilabel.caption=pi is about ?", "errlabel.caption=No darts yet", "runbtn.caption=&Go on"] },
  // spin3d.rr: the arrow keys (the form hears them first) and the buttons change the speeds
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
  // windows/: a form's Windows messages (WndProc). On Windows real windows
  // (RAPIDR_CAPTURE_WINDOWS: the headless host has no window to send to) and
  // the messages arrive; elsewhere, and on the web, the examples say they
  // need Windows and do nothing.
  ...(process.platform === "win32" ? [
    { file: "windows/wndproc_postmessage.rr", runtimes: ["run", "interp", "native"], env: { RAPIDR_CAPTURE_WINDOWS: "1" },
      events: `postbtn.onclick,${wait("count", 4)}`, dump: "status.caption,count.caption",
      expect: ["status.caption=WM_USER + 1 arrived: wParam 1, lParam 9", "count.caption=Posted 1, arrived 1"] },
    { file: "windows/wndproc_messages.rr", runtimes: ["run", "interp", "native"], env: { RAPIDR_CAPTURE_WINDOWS: "1" },
      events: wait("hint", 4), dump: "count.caption", expect: ["messages so far"] },
    { file: "windows/midi_in.rr", runtimes: ["run", "interp", "native"], env: { RAPIDR_CAPTURE_WINDOWS: "1" },
      events: `testbtn.onclick,${wait("status", 4)}`, dump: "notes.itemcount", expect: ["notes.itemcount=1"] },
    { file: "windows/wndproc_postmessage.rr", runtimes: ["web"], events: "postbtn.onclick", dump: "status.caption,postbtn.enabled",
      expect: ["status.caption=Windows only: here a form's WndProc hears only its tray icon's messages.", "postbtn.enabled=0"] },
    { file: "windows/wndproc_messages.rr", runtimes: ["web"], events: "clearbtn.onclick", dump: "hint.caption",
      expect: ["hint.caption=Windows only: here a form's WndProc hears only its tray icon's messages."] },
    { file: "windows/midi_in.rr", runtimes: ["web"], events: "openbtn.onclick,testbtn.onclick", dump: "status.caption,notes.itemcount",
      expect: ["status.caption=Windows only: here a form's WndProc hears only its tray icon's messages.", "notes.itemcount=0"] },
  ] : [
    { file: "windows/wndproc_postmessage.rr", runtimes: ["run", "interp", "native", "web"], events: "postbtn.onclick", dump: "status.caption,postbtn.enabled",
      expect: ["status.caption=Windows only: here a form's WndProc hears only its tray icon's messages.", "postbtn.enabled=0"] },
    { file: "windows/wndproc_messages.rr", runtimes: ["run", "interp", "native", "web"], events: "clearbtn.onclick", dump: "hint.caption",
      expect: ["hint.caption=Windows only: here a form's WndProc hears only its tray icon's messages."] },
    { file: "windows/midi_in.rr", runtimes: ["run", "interp", "native", "web"], events: "openbtn.onclick,testbtn.onclick", dump: "status.caption,notes.itemcount",
      expect: ["status.caption=Windows only: here a form's WndProc hears only its tray icon's messages.", "notes.itemcount=0"] },
  ]),
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

// (a RapidR program, a `.rr`, has RapidQ's constants built in: its examples
// don't include RAPIDQ.INC — `.bas` programs, examples/rapidq/ among them, do)
{
  const including = programs(EXAMPLES).filter((p) => p.endsWith(".rr") && /^\s*\$INCLUDE\s+"RAPIDQ\.INC"/im.test(readFileSync(join(EXAMPLES, p), "utf8")));
  ok(!including.length, "no .rr example includes RAPIDQ.INC (its constants are built in)", including.join(", "));
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
const args = (c) => (c.args || []).map((a) => a.replace("{http}", `http://${http.address}`).replace("{aimock}", AI_MOCK).replace("{port}", String(c.port || 0)));

// (`client`: a server example's client — tests/http_client_probe.mjs —
// started with the program, on the port `{port}` gave it; what it got back
// added to the program's output for `expect`. Ports from 47800, one a run:
// 127.0.0.1 only.)
let nextPort = 47800;
function startClient(c) {
  if (!c.client) return null;
  const out = join(WORK, `client-${c.port}.txt`);
  rmSync(out, { force: true });
  rmSync(`${out}.done`, { force: true });
  const child = spawnAsync(process.execPath, [join(ROOT, "tests/http_client_probe.mjs"), c.client.mode, String(c.port), out, ...(c.client.args || [])], { stdio: "ignore" });
  return { child, out };
}
function clientLines(cl) {
  if (!cl) return "";
  const until = Date.now() + 30000;
  const nap = new Int32Array(new SharedArrayBuffer(4));
  while (!existsSync(`${cl.out}.done`) && Date.now() < until) Atomics.wait(nap, 0, 0, 100);
  if (!existsSync(`${cl.out}.done`)) cl.child.kill();
  return `\n${existsSync(cl.out) ? readFileSync(cl.out, "utf8") : "client: no answer"}`;
}

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
  if (c.client) c.port = nextPort++;
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
    const cl = startClient(c);
    const r = readOnly(() => spawn(RAPIDR, ["run", src, ...args(c)], runOpts));
    r.out += clientLines(cl);
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
  const out = c.readOnly || c.elsewhere ? `${work}-${runtime}-out` : join(work, `${stem}-${runtime}`);
  // (a native build: debug, quick to compile; either kind's executable in
  // the output folder)
  const build = spawn(RAPIDR, ["build", src, out, "--no-bundle", ...(native ? ["--debug"] : ["--interp"])], {
    cwd: work, env: env({ CARGO_TARGET_DIR: CARGO_TARGET, RAPIDR_BUILD_CACHE: join(dirname(CARGO_TARGET), "build-cache") }), timeout: 1800000,
  });
  const exe = join(out, `${stem}${EXE}`);
  let r;
  if (!build.ok || !existsSync(exe)) r = { out: build.out.split("\n").filter((l) => /^error|error:/.test(l)).slice(0, 8).join("\n") || build.out.slice(-800), ok: false, why: ` (build failed${build.why})` };
  else if (c.elsewhere) {
    // (started from an empty folder of its own, the example's folder hidden:
    // only what the build put beside the executable can help)
    const cwd = `${work}-${runtime}-cwd`, hidden = `${work}-hidden`;
    rmSync(cwd, { recursive: true, force: true });
    mkdirSync(cwd, { recursive: true });
    renameSync(work, hidden);
    try {
      r = spawn(exe, args(c), { ...runOpts, cwd, env: env({ ...hooks(c, cwd), ...(c.env || {}) }) });
    } finally {
      renameSync(hidden, work);
      rmSync(cwd, { recursive: true, force: true });
    }
  } else {
    const cl = startClient(c);
    r = readOnly(() => spawn(exe, args(c), runOpts));
    r.out += clientLines(cl);
  }
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
    // (its translations, lang/es.po …: the page's files by that name, as a
    // bundle serves them beside the program)
    if (existsSync(join(dir, "lang"))) {
      for (const f of readdirSync(join(dir, "lang"))) assets[`lang/${f}`] = "data:application/octet-stream;base64," + readFileSync(join(dir, "lang", f)).toString("base64");
    }
    // (`includes`: its subfolders' data files too — icons/table.svg … — by
    // their path, as a bundle serves them and $RESOURCE names them)
    if (c.includes) {
      for (const sub of readdirSync(dir).filter((f) => f !== "lang" && statSync(join(dir, f)).isDirectory())) {
        for (const f of readdirSync(join(dir, sub)).filter((f) => statSync(join(dir, sub, f)).isFile() && ![".rr", ".bas", ".inc", ".md"].includes(extname(f).toLowerCase()))) {
          assets[`${sub}/${f}`] = "data:application/octet-stream;base64," + readFileSync(join(dir, sub, f)).toString("base64");
        }
      }
    }
    const env = { ...hooks(c, "web"), ...(isGui(c) ? { RAPIDR_CAPTURE: "web" } : {}), RAPIDR_AI_TEST_SERVER: AI_MOCK.replace(/\/v1$/, ""), ...(c.env || {}) };
    // (the program run once; `reload`: the page opened again — the browser's
    // storage kept — and the program run again with its own events and dump)
    const go = async (env) => {
      await page.evaluate(({ source, assets, env, gui, main, files }) => {
        window.__rapidr_assets = assets;
        window.RAPIDR_TEST_MIDI = ""; window.RAPIDR_TEST_WAVE_IN = "tone:440";
        if (env.RAPIDR_TEST_COMPORT) window.RAPIDR_TEST_COMPORT = env.RAPIDR_TEST_COMPORT;
        // (named as its file: a .rr is a RapidR program — RAPIDQ.INC's
        // constants built in —, a .bas a RapidQ one, as on the desktop)
        const bc = files ? window.rr.compile_files(main, files, assets) : window.rr.compile(source, main, assets);
        if (gui) window.rr.rapidr_set_test_env(env);
        window.rr.rapidr_run_bc(bc);
      }, { source: readFileSync(join(EXAMPLES, c.file), "utf8"), assets, env, gui: isGui(c), main: basename(c.file), files: c.includes ? sourcesUnder(dir) : null });
      let found = [];
      if (isGui(c)) {
        await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: c.timeout || 60000, polling: 100 });
        found = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results())).dump;
      } else {
        await page.waitForFunction(() => window.rr.rapidr_main_done(), null, { timeout: c.timeout || 60000, polling: 100 });
        await page.waitForTimeout(200);
      }
      return found;
    };
    let dump = await go(env);
    if (c.reload) {
      lines.length = 0;
      await page.goto(`${URL_BASE}/tests/web_kernel.html${query}`, { waitUntil: "load" });
      await page.waitForFunction(() => window.rrReady, null, { timeout: 20000 });
      dump = await go({ ...env, RAPIDR_TEST_EVENTS: c.reload.events || "", RAPIDR_TEST_DUMP: c.reload.dump || c.dump });
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
