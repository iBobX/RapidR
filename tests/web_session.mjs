// RapidR's program session protocol (rapidr-session) on the web runtime:
// the program's end the preview frame serves (session_open /
// session_request), driven as RapidR Studio drives it.
//   1. A breakpoint in an `$INCLUDE`d file stops there (the web compile
//      fills the source map from the project's files: compile_files); the
//      stack spans both files; evaluate and setVariable work on the
//      stopped frame; the program goes on and exits.
//   2. Pause on demand stops a busy loop (between two time slices).
//   3. Break on a run-time error stops at the faulting statement; going
//      on, the error unwinds and the program exits with 1.
//   4. The eyes and hands (`app`): a calculator's layout, a click by
//      caption the program hears, a picture of its window, a window it
//      hasn't got refused; paused, a picture still and a click refused
//      with where; going on, input again (the desktop's:
//      crates/rapidr-cli/tests/app_eyes.rs).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo
// served): RAPIDR_URL=http://localhost:8765 node tests/web_session.mjs
// (The same cases run on the desktop through `rapidr run --session`:
// crates/rapidr-cli/tests/session.rs.)

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
// (a fresh page — a run frame of its own, as RapidR Studio gives each run)
async function freshPage() {
await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady === true, null, { timeout: 30000 });

// What RapidR Studio's run frame does (ide/web/run.html): the program's console output is the session's
// output; every event is kept for the test.
await page.evaluate(() => {
  window.events = [];
  for (const level of ["log", "info", "warn", "error"]) {
    const orig = console[level].bind(console);
    console[level] = (...args) => {
      orig(...args);
      window.events.push({ type: "output", stream: level === "log" || level === "info" ? "stdout" : "stderr", text: args.join(" ") + "\n" });
    };
  }
  window.seq = 0;
  window.req = (command) => {
    const seq = ++window.seq;
    window.rr.session_request(JSON.stringify({ seq, ...command }));
    return seq;
  };
  window.open_ = (main, files) => {
    window.events.length = 0;
    const bytes = window.rr.compile_files(main, files, {});
    window.rr.session_open(bytes, main, (json) => window.events.push(JSON.parse(json)));
  };
});
}
await freshPage();

const events = () => page.evaluate(() => window.events);
const req = (command) => page.evaluate((c) => window.req(c), command);
/// The first event `pred` accepts (waiting for it).
async function waitEvent(pred, what, ms = 15000) {
  for (let waited = 0; waited < ms; waited += 50) {
    const found = (await events()).find(pred);
    if (found) return found;
    await page.waitForTimeout(50);
  }
  ok(false, `timed out waiting for ${what}; events: ${JSON.stringify(await events()).slice(0, 1500)}`);
  return null;
}
/// A request's reply.
async function call(command) {
  const seq = await req(command);
  return waitEvent((e) => e.re === seq, `the reply to ${command.type}`);
}
const output = async () => (await events()).filter((e) => e.type === "output").map((e) => e.text).join("");

// ---- 1. a breakpoint in an included file
await page.evaluate(() => window.open_("main.bas", {
  "main.bas": '$INCLUDE "util.inc"\ntotal = 0\nFOR i = 1 TO 3\n  total = Twice(total + i)\nNEXT\nPRINT "total="; total\n',
  "util.inc": "FUNCTION Twice(n)\n  Twice = n * 2\nEND FUNCTION\n",
}));
const ready = await waitEvent((e) => e.type === "ready", "ready");
ok(ready?.protocol === 1 && ready?.program === "main.bas", `ready (${JSON.stringify(ready)})`);
const placed = await call({ type: "setBreakpoints", file: "util.inc", breakpoints: [{ line: 2 }] });
ok(placed?.type === "breakpoints" && placed.breakpoints[0].verified && placed.breakpoints[0].actualLine === 2, `the breakpoint in util.inc is placed (${JSON.stringify(placed)})`);
ok((await call({ type: "start", debug: true }))?.type === "ok", "start");
const stop = await waitEvent((e) => e.type === "stopped", "the stop");
ok(stop?.reason === "breakpoint" && stop?.file === "util.inc" && stop?.line === 2, `stopped in the included file (${JSON.stringify(stop)})`);
const trace = await call({ type: "stackTrace" });
const where = (trace?.frames || []).map((f) => `${f.name}@${f.file}:${f.line}`).join(" ");
ok(where === "Twice@util.inc:2 __main@main.bas:4", `the stack spans both files (${where})`);
ok((await call({ type: "evaluate", expr: "n * 100 + i" }))?.result === "101", "evaluate in the stopped frame");
ok((await call({ type: "setVariable", name: "n", value: "10" }))?.result === "10", "set a variable");
const bad = await call({ type: "evaluate", expr: "nosuch(" });
ok(bad?.type === "error", `a bad expression is an error (${bad?.message})`);
await call({ type: "setBreakpoints", file: "util.inc", breakpoints: [] });
ok((await call({ type: "continue" }))?.type === "ok", "continue");
await waitEvent((e) => e.type === "continued", "continued");
const exit = await waitEvent((e) => e.type === "exited", "the end");
ok(exit?.code === 0, `exited 0 (${JSON.stringify(exit)})`);
for (let waited = 0; !(await output()).includes("total=94") && waited < 5000; waited += 100) await page.waitForTimeout(100);
ok((await output()).includes("total=94"), `the changed variable changed the result (${JSON.stringify(await output())})`);

// ---- 2. pause on demand
await page.evaluate(() => window.open_("spin.bas", { "spin.bas": 'PRINT "go"\ni = 0\nDO\n  i = i + 1\nLOOP UNTIL i < 0\n' }));
await waitEvent((e) => e.type === "ready", "ready");
await call({ type: "start", debug: true });
for (let waited = 0; !(await output()).includes("go") && waited < 5000; waited += 50) await page.waitForTimeout(50);
await page.waitForTimeout(200);
ok((await call({ type: "pause" }))?.type === "ok", "pause");
const paused = await waitEvent((e) => e.type === "stopped", "the pause");
ok(paused?.reason === "pause" && paused?.file === "spin.bas", `paused in the busy loop (${JSON.stringify(paused)})`);
const i = Number((await call({ type: "evaluate", expr: "i" }))?.result);
ok(i > 0, `the loop ran (${i})`);
await call({ type: "stop" });
ok(!!(await waitEvent((e) => e.type === "exited", "the end after stop")), "stop ends it");

// ---- 3. break on a run-time error
await page.evaluate(() => window.open_("oops.bas", { "oops.bas": 'z = 0\nSUB Bad\n  y = 5 \\ z\nEND SUB\nPRINT "before"\nBad\nPRINT "after"\n' }));
await waitEvent((e) => e.type === "ready", "ready");
await call({ type: "start", debug: true, breakOnError: true });
const fault = await waitEvent((e) => e.type === "stopped", "the error stop");
ok(fault?.reason === "exception" && fault?.file === "oops.bas" && fault?.line === 3 && /division/i.test(fault?.description || ""), `stopped at the faulting statement (${JSON.stringify(fault)})`);
ok((await call({ type: "evaluate", expr: "z" }))?.result === "0", "the frame is there to inspect");
await call({ type: "continue" });
const failedExit = await waitEvent((e) => e.type === "exited", "the end after the error");
ok(failedExit?.code === 1, `exited 1 (${JSON.stringify(failedExit)})`);
await page.waitForTimeout(300);
const out3 = await output();
ok(out3.includes("before") && !out3.includes("after") && out3.includes("oops.bas line 3"), `the error unwound (${JSON.stringify(out3)})`);

// ---- 4. the eyes and hands (in a page of its own: one where a program was
// stopped keeps it ended — each run of RapidR Studio has its own frame)
await freshPage();
const CALC = [
  "DECLARE SUB Digit(Sender AS RButton)",
  "DECLARE SUB Clear",
  "CREATE Form1 AS RForm",
  '    Caption = "Calculator"',
  "    ClientWidth = 200: ClientHeight = 120",
  "    CREATE Display AS RLabel",
  '        Caption = "0"',
  "        Left = 10: Top = 10: Width = 180",
  "    END CREATE",
  "    CREATE B7 AS RButton",
  '        Caption = "7"',
  "        Left = 10: Top = 40: Width = 40: Height = 30",
  "        OnClick = Digit",
  "    END CREATE",
  "    CREATE BC AS RButton",
  '        Caption = "C"',
  "        Left = 110: Top = 40: Width = 40: Height = 30",
  "        OnClick = Clear",
  "    END CREATE",
  "END CREATE",
  "SUB Digit(Sender AS RButton)",
  '    IF Display.Caption = "0" THEN Display.Caption = Sender.Caption ELSE Display.Caption = Display.Caption + Sender.Caption',
  '    PRINT "display "; Display.Caption',
  "END SUB",
  "SUB Clear",
  '    Display.Caption = "0"',
  "END SUB",
  "Form1.ShowModal",
  "",
].join("\n");
await page.evaluate((src) => window.open_("calc.rr", { "calc.rr": src }), CALC);
await waitEvent((e) => e.type === "ready", "ready");
await call({ type: "start", debug: true });
const app = async (request) => (await call({ type: "app", request }))?.result || {};
let look = {};
for (let waited = 0; waited < 10000; waited += 100) {
  look = await app({ do: "layout" });
  if ((look.text || "").includes("b7 (RButton)")) break;
  await page.waitForTimeout(100);
}
ok(look.ok && look.text.includes('Window form1 "Calculator" (RForm), inside 200 × 120, in front') && look.text.includes('b7 (RButton) "7" at 10,40 size 40×30'), `the layout lists the window and its buttons (${JSON.stringify(look.text)})`);
const clicked = await app({ do: "click", caption: "7" });
ok(clicked.ok && clicked.text.startsWith('Clicked b7 (RButton "7") in form1 ("Calculator") at 30,55.'), `a click by caption (${JSON.stringify(clicked)})`);
for (let waited = 0; !(await output()).includes("display 7") && waited < 5000; waited += 100) await page.waitForTimeout(100);
ok((await output()).includes("display 7"), `the program heard it (${JSON.stringify(await output())})`);
const shot = await app({ do: "screenshot", window: "form1", scale: "1x", frame: false });
ok(shot.ok && shot.width === 200 && shot.height === 120 && atob(shot.png || "").startsWith("\x89PNG"), `a picture of its window at 1× (${shot.text})`);
const framed = await app({ do: "screenshot" });
ok(framed.ok && framed.width > 200 && framed.height > 120, `a framed picture at the page's scale (${framed.text})`);
const other = await app({ do: "screenshot", window: "RapidR Studio" });
ok(!other.ok && other.text.includes('No window named "RapidR Studio" in this program. Its windows now: form1 ("Calculator").'), `another window can't be asked for (${other.text})`);
ok((await call({ type: "pause" }))?.type === "ok", "pause");
await waitEvent((e) => e.type === "stopped", "the pause");
const refused = await app({ do: "click", caption: "C" });
ok(!refused.ok && /^The program is paused in the debugger at line \d+ of calc.rr: continue it first/.test(refused.text), `paused: a click refused with where (${refused.text})`);
const still = await app({ do: "layout" });
ok(still.ok && still.text.includes('display (RLabel) "7"'), "paused: the layout still answers");
await call({ type: "continue" });
await waitEvent((e) => e.type === "continued", "continued");
const cleared = await app({ do: "click", caption: "C" });
ok(cleared.ok, `going on: input again (${cleared.text})`);
const after = await app({ do: "layout" });
ok(after.text.includes('display (RLabel) "0"'), `C cleared the display (${JSON.stringify(after.text)})`);
await call({ type: "stop" });

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
console.log(failed ? `Web session: ${failed} failed` : "Web session: all ok");
process.exit(failed ? 1 : 0);
