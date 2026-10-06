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
await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady === true, null, { timeout: 30000 });

// What preview.html does: the program's console output is the session's
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

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
console.log(failed ? `Web session: ${failed} failed` : "Web session: all ok");
process.exit(failed ? 1 : 0);
