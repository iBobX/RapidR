// The debugger on the web: a program under RapidR's program session
// protocol (rapidr-session) on the web runtime, driven as RapidR Studio's
// debugger drives its run frame (tests/web_run.mjs). tests/web_session.mjs
// has the protocol's own cases ($INCLUDEd files, evaluate / setVariable,
// pause, break on error); these are what a user does with a debugger:
//   1. A breakpoint in the main code: the stop, the stack (`__main`), the
//      variables; Step Over goes to the next line; Stop ends it.
//   2. A breakpoint in an event handler: the program runs (waits in its
//      form, not stopped) until the user's real click on the button stops
//      it in the handler; the stack has the handler, its local; Step Over
//      updates it; a watch (evaluate) reads it; Continue runs on (the
//      output has it); Stop ends it.
//   3. A breakpoint in a handler that isn't called: the program runs on,
//      waiting in its form.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_debugger.mjs

import { chromium } from "playwright";
import { openRunner } from "./web_run.mjs";
import * as k from "./web_kernel_page.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const r = await openRunner(browser);

// 1. The main code.
await r.debug(["DIM x AS INTEGER", "x = 42", "PRINT x"].join("\n"), { breakpoints: [2] });
let stop = await r.waitStopped();
ok(stop?.line === 2, `stops at the breakpoint on line 2 (${JSON.stringify(stop)})`);
let snap = await r.snapshot();
ok(snap.stack.some((f) => f.name === "__main"), `the call stack has __main (${JSON.stringify(snap.stack)})`);
ok("x" in snap.globals || "X" in snap.globals || "x" in snap.locals || "X" in snap.locals, `the variables have x (${JSON.stringify(snap)})`);
r.send({ type: "stepOver" });
await r.page.waitForTimeout(100);
stop = await r.waitStopped();
ok(stop?.line === 3, `Step Over stops on line 3 (${JSON.stringify(stop)})`);
snap = await r.snapshot();
ok(Object.entries({ ...snap.globals, ...snap.locals }).some(([n, v]) => n.toLowerCase() === "x" && v === 42), `x is 42 after the step (${JSON.stringify(snap)})`);
r.send({ type: "stop" });
ok(await r.waitExited(5000), "Stop ends the program");

// 2. An event handler.
const handler = [
  "SUB Button1_Click",
  "  DIM x AS INTEGER",
  "  x = 100",
  "  x = x + 5",
  "  PRINT x",
  "END SUB",
  "CREATE Form1 AS QFORM",
  "  CREATE Button1 AS QBUTTON",
  '    Caption = "Button1"',
  "    OnClick = Button1_Click",
  "  END CREATE",
  "END CREATE",
  "Form1.ShowModal",
].join("\n");
await r.debug(handler, { breakpoints: [3] });
await k.waitFor(r.page, "Button1");
await r.page.waitForTimeout(300);
ok(!r.paused && !r.session.exited, "the program runs, waiting in its form (not stopped)");
await k.click(r.page, "Button1");
stop = await r.waitStopped(5000);
ok(stop?.line === 3, `the click stops in the handler, on line 3 (${JSON.stringify(stop)})`);
snap = await r.snapshot();
ok(snap.stack[0]?.name?.toLowerCase() === "button1_click", `the call stack's top is Button1_Click (${JSON.stringify(snap.stack)})`);
ok(Object.keys(snap.locals).some((n) => n.toLowerCase() === "x"), `the locals have x (${JSON.stringify(snap.locals)})`);
const x = async () => Object.entries((await r.snapshot()).locals).find(([n]) => n.toLowerCase() === "x")?.[1];
r.send({ type: "stepOver" });
await r.page.waitForTimeout(100);
stop = await r.waitStopped();
ok(stop?.line === 4 && (await x()) === 100, `Step Over: line 4, x = 100 (${JSON.stringify(stop)}, ${await x()})`);
// (a watch: the expression evaluated in the stopped frame)
const top = (await r.request({ type: "stackTrace" })).frames[0];
const watch = await r.request({ type: "evaluate", expr: "x", frame: top.id, context: "watch" }).catch((e) => ({ error: e.message }));
ok(/^100$/.test(String(watch.value ?? watch.result ?? "")), `a watch on x reads 100 (${JSON.stringify(watch)})`);
r.send({ type: "stepOver" });
await r.page.waitForTimeout(100);
stop = await r.waitStopped();
ok(stop?.line === 5 && (await x()) === 105, `Step Over: line 5, x = 105 (${JSON.stringify(stop)}, ${await x()})`);
r.send({ type: "continue" });
ok(await r.waitOutput(/^105$/m, 5000), `Continue runs on: the handler printed 105 (${JSON.stringify(r.lines.slice(-3))})`);
ok(!r.paused, "not stopped after Continue");
r.send({ type: "stop" });
ok(await r.waitExited(5000), "Stop ends the program");

// 3. A breakpoint in a handler nobody calls.
await r.debug([
  "SUB Button1_Click",
  '  MESSAGEBOX("Robert", "Blah", 0)',
  "END SUB",
  "CREATE Form1 AS QFORM",
  "  CREATE Button1 AS QBUTTON",
  "    OnClick = Button1_Click",
  "  END CREATE",
  "END CREATE",
  "Form1.ShowModal",
].join("\n"), { breakpoints: [2] });
await k.waitFor(r.page, "Button1");
await r.page.waitForTimeout(500);
ok(!r.paused && !r.session.exited && (await k.shown(r.page, "Form1")), "the debugger runs the program: it waits in its form, not stopped");

ok(r.pageErrors.length === 0, `no page errors (${r.pageErrors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb debugger: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb debugger: ALL CHECKS PASSED");
