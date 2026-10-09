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
// arguments its http://127.0.0.1:<port>.
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

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
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
// (`wait(c, n)`: n clicks on a component that does nothing with them — the
// hooks fire an event every 50 ms, so the program's timers run meanwhile)
const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");
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
  // gui/
  { file: "gui/hello_form.rr", runtimes: ["run", "interp", "native", "web"], events: "greetbutton.onclick,greetbutton.onclick", dump: "answer.caption",
    expect: ["answer.caption=Hello, World! (2)"] },
  { file: "gui/menus.rr", runtimes: ["run", "interp", "native", "web"], events: "wrapitem.onclick,smallitem.onclick,upperitem.onclick", dump: "status.simpletext,text.text,wrapitem.checked,largeitem.checked",
    expect: ["status.simpletext=Text size 9", "text.text=RIGHT-CLICK ME FOR THE POP-UP MENU.", "wrapitem.checked=0", "largeitem.checked=0"] },
  { file: "gui/dialogs.rr", runtimes: ["run", "interp", "native", "web"], events: "colourbtn.onclick,askbtn.onclick", dump: "answer.caption,sample.color",
    colorDialog: "65280", messageDialog: "Yes", expect: ["answer.caption=Glad to hear it!", "sample.color=65280"] },
  { file: "gui/dialogs.rr", runtimes: ["run", "interp", "native", "web"], events: "fontbtn.onclick,openbtn.onclick", dump: "answer.caption",
    fontDialog: "Courier New,14", fileDialog: "notes.txt", expect: ["answer.caption=Chose ", "notes.txt"] },
  { file: "gui/stopwatch.rr", runtimes: ["run", "interp", "native", "web"], events: `startbtn.onclick,${wait("state", 6)},lapbtn.onclick,${wait("state", 4)},lapbtn.onclick,startbtn.onclick`,
    dump: "laps.itemcount,state.caption,time.caption", expect: ["laps.itemcount=2", "state.caption=Stopped", "time.caption=0:0"] },
  { file: "gui/pantry.rr", runtimes: ["run", "interp", "native", "web"], events: "shelves.__item_1,addbtn.onclick", dump: "total.caption,grid.rowcount",
    expect: ["total.caption=Vegetables: 3 items, worth 4.80", "grid.rowcount=4"] },
  { file: "gui/themes.rr", runtimes: ["run", "interp", "native", "web"], events: "pickdark.onclick", dump: "now.caption",
    expect: ["now.caption=Theme: rapidr dark"] },
  { file: "gui/tray.rr", runtimes: ["run", "interp", "native", "web"], events: "hidebtn.onclick,form.__tray_513,form.__tray_514", dump: "info.caption,form.__shown",
    expect: ["info.caption=Back from the tray (1)", "form.__shown=1"] },
  // studio/
  { file: "studio/panels.rr", runtimes: ["run", "interp", "native", "web"], events: "box.__dblclick_60_170", dump: "tree.projectname,tree.filecount,designer.compcount,insp.targettype",
    expect: ["tree.projectname=Greeter", "tree.filecount=4", "designer.compcount=2", "insp.targettype=RCheckBox"] },
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
    expect: ["info.caption=4 people earn over 70000; Charlie the most", "grid.rowcount=5"] },
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
  // network/ (the tests' own server, local)
  { file: "network/http_json.rr", runtimes: ["run", "interp", "native", "web"], args: ["{http}/examples/network/forecast.json"],
    expect: ["Forecast for Harbour Town (updated 2026-10-06 06:00)", "  Tuesday   10 to 15 C, showers", "Warmest: Monday"] },
  { file: "network/download.rr", runtimes: ["run", "interp", "native", "web"], args: ["{http}/examples/network/forecast.json"], events: "fetchbtn.onclick", dump: "info.caption",
    expect: ["info.caption=Saved 290 bytes of examples/network/forecast.json"] },
  // iot/: a scripted ESP32 (RAPIDR_TEST_COMPORT's `esp32`: DTR / RTS reset it, it prints a boot log)
  { file: "iot/esp32_monitor.rr", runtimes: ["run", "interp", "native", "web"], env: { RAPIDR_TEST_COMPORT: "COM5:esp32" },
    events: `connectbtn.onclick,${wait("state", 2)},resetbtn.onclick,${wait("state", 6)}`, dump: "ports.text,state.caption,log.linecount",
    expect: ["ports.text=COM5 (CP2102N USB to UART Bridge Controller)", "state.caption=Connected to COM5 at 115200   DTR 0  RTS 0", "log.linecount=9"] },
  // rapidq/: RapidQ's own way, Q names, as RC.EXE compiles it
  { file: "rapidq/notepad.bas", runtimes: ["run", "interp", "native", "web"], events: "saveitem.onclick,newitem.onclick,openitem.onclick", dump: "form.caption,editor.linecount",
    fileDialog: "note.txt;note.txt", expect: ["form.caption=Notepad - note.txt", "editor.linecount=3"] },
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
  ...extra,
});
const isGui = (c) => c.dump !== undefined;
const args = (c) => (c.args || []).map((a) => a.replace("{http}", `http://${http.address}`));

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
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  const dir = join(EXAMPLES, dirname(c.file));
  for (const f of readdirSync(dir)) {
    if (statSync(join(dir, f)).isFile()) cpSync(join(dir, f), join(work, f));
  }
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
  const runEnv = env({ ...hooks(c, work), ...(c.env || {}) });
  const runOpts = { cwd: work, env: runEnv, input: c.input || "", timeout: c.timeout || 60000 };
  if (runtime === "run") {
    const r = spawn(RAPIDR, ["run", src, ...args(c)], runOpts);
    // (RAPIDR_SHOTS=dir: the windows as captured kept, to look at —
    // <example>-<case>-<window>@<scale>x.bmp; RAPIDR_SCALE sets the scale)
    if (process.env.RAPIDR_SHOTS && isGui(c)) {
      mkdirSync(process.env.RAPIDR_SHOTS, { recursive: true });
      const scale = process.env.RAPIDR_SCALE || "1";
      for (const f of readdirSync(work).filter((f) => /^window-\d+\.bmp$/.test(f))) {
        const n = f.match(/\d+/)[0];
        cpSync(join(work, f), join(process.env.RAPIDR_SHOTS, `${c.file.replace(/[/.]/g, "_")}-${cases.indexOf(c)}-${n}@${scale}x.bmp`));
      }
    }
    return r;
  }
  const native = runtime === "native";
  const out = join(work, `${stem}-${runtime}`);
  // (a native build: debug, quick to compile; either kind's executable in
  // the output folder)
  const build = spawn(RAPIDR, ["build", src, out, "--no-bundle", ...(native ? ["--debug"] : ["--interp"])], {
    cwd: work, env: env({ CARGO_TARGET_DIR: CARGO_TARGET, RAPIDR_BUILD_CACHE: join(dirname(CARGO_TARGET), "build-cache") }), timeout: 1800000,
  });
  const exe = join(out, `${stem}${EXE}`);
  let r;
  if (!build.ok || !existsSync(exe)) r = { out: build.out.split("\n").filter((l) => /^error|error:/.test(l)).slice(0, 8).join("\n") || build.out.slice(-800), ok: false, why: ` (build failed${build.why})` };
  else r = spawn(exe, args(c), runOpts);
  // (the native build's ~350 MB in the shared target, gone)
  if (native) dropBuild(CARGO_TARGET, stem);
  rmSync(out, { recursive: true, force: true });
  return r;
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
    const assets = Object.fromEntries(readdirSync(dir).filter((f) => statSync(join(dir, f)).isFile() && ![".rr", ".bas", ".md"].includes(extname(f).toLowerCase()))
      .map((f) => [f, "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64")]));
    const env = { ...hooks(c, "web"), ...(isGui(c) ? { RAPIDR_CAPTURE: "web" } : {}), ...(c.env || {}) };
    await page.evaluate(({ source, assets, env, gui }) => {
      window.__rapidr_assets = assets;
      window.RAPIDR_TEST_MIDI = ""; window.RAPIDR_TEST_WAVE_IN = "tone:440";
      if (env.RAPIDR_TEST_COMPORT) window.RAPIDR_TEST_COMPORT = env.RAPIDR_TEST_COMPORT;
      const bc = window.rr.compile(source, "example", assets);
      if (gui) window.rr.rapidr_set_test_env(env);
      window.rr.rapidr_run_bc(bc);
    }, { source: readFileSync(join(EXAMPLES, c.file), "utf8"), assets, env, gui: isGui(c) });
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
  }
}
if (browser) await browser.close();
http.close();
console.log(`\nExamples: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
