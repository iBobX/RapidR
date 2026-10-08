// The web VM gives the page a turn every few milliseconds (VmError::Yielded,
// interpreter/rapidr-vm-host-web): a program that never waits — a busy
// loop, a long computation — keeps running, and the page stays responsive
// (it repaints, a debugger's requests are answered). Program semantics don't change:
// events that arrive meanwhile wait until the program waits (DoEvents,
// ShowModal, the end of main), as on the desktop.
//   1. `cpuhog: GOTO cpuhog`: the page answers within a second and
//      repaints, the program can be stopped, and a new program then runs.
//   2. A loop updating a label and printing a counter, then END: the final
//      output is right, and intermediate states were visible meanwhile
//      (the label's caption, printed lines, repaints).
//   3. A click while main is busy runs its handler only once main waits
//      (ShowModal) — or, in a loop with DoEvents, at a DoEvents.
//   4. A busy handler: the page takes a second click, run after the first.
//   5. The debugger (the session protocol, tests/web_run.mjs): a breakpoint
//      after a long loop, Step Over a SUB that loops long.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_vm_yield.mjs
// (The program runs on the runtime's own page, tests/web_run.mjs; its form
// is a window the kernel draws: the test reads its accessibility mirror and
// clicks as the user does, tests/web_kernel_page.mjs.)

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const pageErrors = r.pageErrors;

/// The output's lines.
const output = async () => r.lines.map((l) => l.trim());
const errors = async () => r.errors();
/// Runs `lines` as written on the runtime's page (tests/web_run.mjs).
async function run(lines) {
  await r.run(lines.join("\n") + "\n");
}
/// The program's page (its windows).
async function preview() {
  return page;
}
/// How long `promise` takes, in ms (Infinity after `limit` ms).
async function latency(promise, limit = 5000) {
  const t0 = Date.now();
  const done = await Promise.race([promise.then(() => true, () => true), new Promise((r) => setTimeout(() => r(false), limit))]);
  return done ? Date.now() - t0 : Infinity;
}
async function waitFor(cond, ms = 15000) {
  for (let waited = 0; waited < ms; waited += 100) {
    if (await cond()) return true;
    await page.waitForTimeout(100);
  }
  return false;
}

// 1. A busy loop: the page stays responsive and Stop stops it.
await run(['PRINT "hog start"', "cpuhog: GOTO cpuhog"]);
ok(await waitFor(async () => (await output()).includes("hog start")), "the busy program started");
await page.waitForTimeout(500);
const ideMs = await latency(page.evaluate(() => 1));
ok(ideMs < 1000, `the page answers while the program runs (${ideMs} ms)`);
const frames = await (await preview()).evaluate(() => new Promise((r) => { let n = 0; const t0 = performance.now(); const f = () => (performance.now() - t0 < 500 ? (n++, requestAnimationFrame(f)) : r(n)); requestAnimationFrame(f); })).catch(() => 0);
ok(frames >= 10, `the page repaints while the program runs (${frames} frames in 0.5 s)`);
const stopMs = await latency(r.stop());
ok(stopMs < 1000, `Stop works while the program runs (${stopMs} ms)`);
await run(['PRINT "second run "; 6 * 7']);
ok(await waitFor(async () => (await output()).includes("second run 42")), "a new program runs after the stopped one");

// 2. A busy loop that updates a label and prints, then ENDs.
await run([
  "CREATE Form AS QFORM",
  "  CREATE Label1 AS QLABEL",
  '    Caption = "start"',
  "    Width = 200",
  "  END CREATE",
  "END CREATE",
  "Form.Show",
  "DEFINT i, n",
  "t# = TIMER",
  "DO",
  "  n = n + 1",
  "  IF n MOD 2000 = 0 THEN Label1.Caption = \"count\" + STR$(n)",
  "  IF n MOD 200000 = 0 THEN PRINT \"tick\"; n",
  "LOOP UNTIL TIMER - t# > 2",
  'PRINT "part";',
  "t# = TIMER",
  "DO: n = n + 1: LOOP UNTIL TIMER - t# > 0.3",
  'PRINT "ial line"',
  'IF n > 0 THEN PRINT "done counting"',
  "END",
]);
const captions = new Set(), printed = new Set(), mainDone = new Set();
for (let i = 0; i < 40 && !(await output()).includes("[RapidR] Program ended."); i++) {
  const c = await k.text(await preview(), "Label1").catch(() => "");
  if (c) captions.add(c);
  // (sampled before the output is read: a sample taken after the END —
  // the output then says so — doesn't count)
  const done = await (await preview()).evaluate(() => window.rr.rapidr_main_done()).catch(() => "?");
  const lines = await output();
  if (!lines.includes("[RapidR] Program ended.")) mainDone.add(done);
  for (const l of lines) if (l.startsWith("tick")) printed.add(l);
  await page.waitForTimeout(100);
}
const out2 = await output();
ok(out2.includes("done counting") && out2.includes("[RapidR] Program ended."), `the loop finished and ENDed (${JSON.stringify(out2.slice(-4))})`);
ok([...captions].filter((c) => c.startsWith("count")).length >= 3, `the label's caption was seen changing while the loop ran (${[...captions].slice(0, 5).join(" | ")} …)`);
ok(printed.size >= 2, `printed lines came while the loop ran (${printed.size} seen before the end)`);
ok(mainDone.has(false) && !mainDone.has(true), `main isn't done while it runs between time slices (${[...mainDone]})`);
ok((await (await preview()).evaluate(() => window.rr.rapidr_main_done())) === true, "main is done after END");
ok(out2.includes("partial line") && !out2.includes("part"), "a line printed across time slices stays one line");
ok(!(await errors()).includes("error"), "no run-time error");

// 3. A click while main is busy waits until main waits.
const clickProgram = (doEvents) => [
  "DIM busy AS INTEGER",
  "SUB Clicked",
  '  PRINT "clicked, busy ="; busy',
  "  Label1.Caption = \"clicked\"",
  "END SUB",
  "CREATE Form AS QFORM",
  "  CREATE Button1 AS QBUTTON",
  '    Caption = "Click"',
  "    OnClick = Clicked",
  "  END CREATE",
  "  CREATE Label1 AS QLABEL",
  "    Top = 40",
  '    Caption = "-"',
  "  END CREATE",
  "END CREATE",
  "Form.Show",
  "busy = 1",
  'PRINT "looping"',
  "t# = TIMER",
  "DO",
  "  n = n + 1",
  doEvents ? "  IF n MOD 1000 = 0 THEN DOEVENTS" : "",
  "LOOP UNTIL TIMER - t# > 2",
  "busy = 0",
  'PRINT "loop over"',
  "Form.ShowModal",
];
for (const doEvents of [false, true]) {
  const kind = doEvents ? "with DoEvents" : "without DoEvents";
  await run(clickProgram(doEvents));
  ok(await waitFor(async () => (await output()).includes("looping")), `${kind}: the busy loop started`);
  await k.click(await preview(), "Button1", null, { timeout: 1500 }).catch((e) => console.log("  click: " + e.message.split("\n")[0]));
  await page.waitForTimeout(300);
  const during = await output();
  const caption = (await k.text(await preview(), "Label1")) || "";
  const clicked = (lines) => lines.filter((l) => l.startsWith("clicked, busy ="));
  if (doEvents) ok(clicked(during).join() === "clicked, busy =1" && !during.includes("loop over") && caption === "clicked", `${kind}: the click ran at a DoEvents while the loop goes on (${JSON.stringify(during.slice(-3))}, ${caption})`);
  else ok(clicked(during).length === 0 && caption === "-", `${kind}: the click's handler didn't run while main is busy (${JSON.stringify(during.slice(-3))}, ${caption})`);
  ok(await waitFor(async () => (await output()).includes("loop over")), `${kind}: the loop ended`);
  await page.waitForTimeout(300);
  const after = await output();
  if (doEvents) ok(clicked(after).length === 1, `${kind}: the handler ran once (${JSON.stringify(after.slice(-4))})`);
  else ok(clicked(after).join() === "clicked, busy =0" && after.indexOf("clicked, busy =0") > after.indexOf("loop over"), `${kind}: the handler ran once main waits in ShowModal (${JSON.stringify(after.slice(-4))})`);
}

// 4. A busy event handler (main waits in ShowModal): the page stays
// responsive, and a second click runs its handler after the first one, not
// inside it.
await run([
  "DIM clicks AS INTEGER",
  "SUB Clicked",
  "  clicks = clicks + 1",
  "  c = clicks",
  '  PRINT "start"; c',
  "  t# = TIMER",
  "  DO: LOOP UNTIL TIMER - t# > 1",
  '  PRINT "end"; c',
  "END SUB",
  "CREATE Form AS QFORM",
  "  CREATE Button1 AS QBUTTON",
  "    OnClick = Clicked",
  "  END CREATE",
  "END CREATE",
  "Form.ShowModal",
]);
await k.waitFor(await preview(), "Button1", 10000);
await k.click(await preview(), "Button1");
await page.waitForTimeout(200);
const handlerMs = await latency(k.click(await preview(), "Button1", null, { timeout: 1500 }));
ok(handlerMs < 1000, `the page takes a click while a handler runs (${handlerMs} ms)`);
ok(await waitFor(async () => (await output()).includes("end2"), 8000), "both clicks' handlers ran");
const handlerLines = (await output()).filter((l) => /^(start|end)\d$/.test(l));
ok(handlerLines.join() === "start1,end1,start2,end2", `one handler after the other (${handlerLines.join()})`);

// 5. The debugger: a breakpoint after a long loop, and Step Over a SUB that
// loops long, while the program runs on between time slices.
await r.debug([
  "DIM n AS INTEGER",
  "SUB Spin",
  "  FOR i = 1 TO 300000: n = n + 1: NEXT",
  "END SUB",
  "FOR i = 1 TO 300000: n = n + 1: NEXT",
  'PRINT "after loop"; n',
  "Spin",
  'PRINT "after spin"; n',
].join("\n"), { breakpoints: [6] });
let stop = await r.waitStopped();
ok(!!stop, "the debugger stops at the breakpoint after the long loop");
ok(stop?.line === 6, `… on its line (${stop?.line})`);
r.send({ type: "stepOver" });
await page.waitForTimeout(50);
ok(!!(await r.waitStopped()), "Step Over");
r.send({ type: "stepOver" });
await page.waitForTimeout(50);
const debugMs = await latency(page.evaluate(() => 1));
ok(debugMs < 1000, `the page answers while the stepped SUB runs (${debugMs} ms)`);
stop = await r.waitStopped();
ok(!!stop, "Step Over a SUB that loops long stops after it");
ok(stop?.line === 8, `… on the next line (${stop?.line})`);
const debugOut = await output();
ok(debugOut.includes("after loop300000") && !debugOut.some((l) => l.startsWith("after spin")), `output so far (${JSON.stringify(debugOut.slice(-3))})`);
r.send({ type: "stop" });

ok(pageErrors.length === 0, `no page errors (${pageErrors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb VM yield: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb VM yield: ALL CHECKS PASSED");
