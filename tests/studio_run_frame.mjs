// RapidR Studio's run frame on the web (ide/web/run.html, ide/web/studio.js):
// the program under development runs sandboxed, apart from Studio.
//
//   1. SEC-02/03: a hostile program (RJAVASCRIPT) tries to read Studio's
//      localStorage and DOM and to forge Studio's messages; then Studio's
//      own page forges messages too (standing in for any other window).
//      The frame runs at an opaque origin, nothing gets through, and what
//      the program prints is delivered exactly once.
//   2. The program's storage (RWEBSTORAGE): kept by Studio per program
//      between runs, under a key of its own; Studio's keys untouched.
//   3. The run frame draws at 1:1: each canvas's backing store is its CSS
//      size × devicePixelRatio, the frame neither scaled nor placed between
//      device pixels, at DPR 1 and 2 (tests/web_pixels.mjs: the same on the
//      runtime's own page).
//
// Usage (repo root, after tools/build_studio_web.sh, with the repo served on
// RAPIDR_URL, default http://localhost:8765; or Studio on
// RAPIDR_STUDIO_URL):  node tests/studio_run_frame.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const STUDIO_URL = process.env.RAPIDR_STUDIO_URL || `${URL_BASE}/target/studio-web`;
const SECRET = "sk-ant-TEST-DO-NOT-LEAK";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const PROGRAM = [
  "CREATE JS AS RJAVASCRIPT",
  "END CREATE",
  "CREATE St AS RWEBSTORAGE",
  "END CREATE",
  "DIM r AS STRING",
  `r = JS.Eval("(function(){try{return 'LS:'+parent.localStorage.getItem('rapidr-test-secret')}catch(e){return 'LS-BLOCKED:'+e.name}})()")`,
  "PRINT r",
  `r = JS.Eval("(function(){try{return 'DOM:'+parent.document.title}catch(e){return 'DOM-BLOCKED:'+e.name}})()")`,
  "PRINT r",
  `r = JS.Eval("(function(){parent.postMessage({__rapidr_session:JSON.stringify({type:'output',stream:'stdout',text:'SPOOF-SESSION'}),__rapidr_console:{level:'log',text:'SPOOF-CONSOLE'},__rapidr_hello:true},'*');return 'sent'})()")`,
  `PRINT "PREV=" + St.Get("visits")`,
  `St.Set("visits", "seen")`,
  `PRINT "PRINT-ONCE-MARKER"`,
  // (the program stays: its frame is looked at)
  "CREATE Form AS QFORM",
  '  Caption = "Isolated"',
  "END CREATE",
  "Form.ShowModal",
].join("\n");

const browser = await chromium.launch();
// (one browser profile for the runs: Studio's storage is kept between them)
const contexts = new Map();

/// Studio with `files` in its page's store, `open` opened and run (F5).
async function studio({ files = [], open, dpr = 1 }) {
  if (!contexts.has(dpr)) contexts.set(dpr, await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: dpr }));
  const page = await contexts.get(dpr).newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(({ files, secret }) => {
    // (Studio's page only, not the run frame)
    if (window !== window.top) return;
    window.RAPIDR_STUDIO_TEST = {};
    window.RAPIDR_STUDIO_TEST_FILES = files;
    localStorage.setItem("rapidr-test-secret", secret);
  }, { files, secret: SECRET });
  const q = new URLSearchParams({ theme: "rapidr-light", fresh: "", window: "normal", open, do: "run.start" });
  await page.goto(`${STUDIO_URL}/index.html?${q}`, { waitUntil: "load" });
  return { page, errors };
}
const output = (page) => k.text(page, "OutputBox").then((t) => t ?? "");
const runFrame = (page) => page.frames().find((f) => f.url().includes("run.html"));
async function until(fn, ms = 30000) {
  for (let waited = 0; waited < ms; waited += 100) {
    const v = await fn();
    if (v) return v;
    await new Promise((r) => setTimeout(r, 100));
  }
  return fn();
}

// ── 1. Isolation ──
const files = [{ path: "iso/iso.bas", text: PROGRAM }];
{
  const { page, errors } = await studio({ files, open: "iso/iso.bas" });
  ok(await until(async () => (await output(page)).includes("PRINT-ONCE-MARKER")), "the program ran in Studio");
  await page.waitForTimeout(400); // (forged messages, were any taken)
  const out = await output(page);
  ok(out.includes("LS-BLOCKED:SecurityError"), "the program can't read Studio's localStorage");
  ok(!out.includes(SECRET), "the secret never reaches the program's output");
  ok(out.includes("DOM-BLOCKED:SecurityError"), "the program can't read Studio's DOM");
  ok(!/SPOOF/.test(out), "messages the program forges to Studio's window are ignored");
  ok(out.split("PRINT-ONCE-MARKER").length - 1 === 1, "PRINT output delivered exactly once");
  ok(/PREV=\s*$/m.test(out), `first run: the program's storage starts empty (${JSON.stringify(out.match(/PREV=.*/)?.[0])})`);
  const frame = runFrame(page);
  ok(!!frame, "the run frame is there");
  ok((await frame?.evaluate(() => self.origin)) === "null", "the program runs at an opaque origin");
  ok((await page.evaluate(() => document.querySelector("#studio-run iframe")?.getAttribute("sandbox") ?? "")).split(/\s+/).every((t) => t !== "allow-same-origin"),
    "the run frame's sandbox has no allow-same-origin");

  // Studio's page forges messages (any other window could): no new
  // channel, nothing in the output.
  await page.evaluate(() => {
    window.postMessage({ __rapidr_hello: true }, "*");
    window.postMessage({ __rapidr_session: JSON.stringify({ type: "output", stream: "stdout", text: "SPOOF-EXTERNAL\n" }) }, "*");
    window.postMessage({ __rapidr_console: { level: "log", text: "SPOOF-EXTERNAL" } }, "*");
  });
  await page.waitForTimeout(400);
  ok(!/SPOOF/.test(await output(page)), "messages forged from another window are ignored");
  ok(page.frames().filter((f) => f.url().includes("run.html")).length === 1 && runFrame(page) === frame, "the run frame is still the one Studio made");
  ok(errors.length === 0, `no Studio page errors (${errors.join("; ")})`);
  await page.screenshot({ path: "scratch/studio_run_frame.png" });
  await page.close();
}

// ── 2. The program's storage, the next run ──
{
  const { page, errors } = await studio({ files, open: "iso/iso.bas" });
  ok(await until(async () => (await output(page)).includes("PRINT-ONCE-MARKER")), "the program ran again");
  ok((await output(page)).includes("PREV=seen"), "RWEBSTORAGE's value is kept between runs");
  const stored = await page.evaluate(() => {
    const keys = Object.keys(localStorage).filter((k) => k.startsWith("rapidr-app-storage:"));
    return { keys, app: keys.map((k) => localStorage.getItem(k)), secret: localStorage.getItem("rapidr-test-secret") };
  });
  ok(stored.keys.length === 1 && JSON.parse(stored.app[0]).visits === "seen", `kept under the program's own key (${JSON.stringify(stored)})`);
  ok(stored.secret === SECRET, "Studio's own keys untouched");
  ok(errors.length === 0, `no Studio page errors (${errors.join("; ")})`);
  await page.close();
}

// ── 3. 1:1 pixels in the run frame ──
for (const dpr of [1, 2]) {
  const { page, errors } = await studio({ open: "examples/gui/menus.rr", dpr });
  const frame = await until(async () => {
    const f = runFrame(page);
    return f && (await f.evaluate(() => [...document.querySelectorAll("canvas")].some((c) => c.offsetWidth > 0)).catch(() => false)) ? f : null;
  });
  ok(!!frame, `DPR ${dpr}: menus.rr runs in the run frame`);
  if (!frame) continue;
  await page.waitForTimeout(500);
  const m = await frame.evaluate(() => ({
    dpr: devicePixelRatio,
    canvases: [...document.querySelectorAll("canvas")].filter((c) => c.offsetWidth > 0).map((c) => {
      const r = c.getBoundingClientRect();
      const tf = [];
      for (let el = c; el; el = el.parentElement) { const t = getComputedStyle(el).transform; if (t !== "none") tf.push(t); }
      return { cssW: r.width, cssH: r.height, x: r.left, y: r.top, w: c.width, h: c.height, tf };
    }),
  }));
  const outer = await page.evaluate(() => {
    const f = document.querySelector("#studio-run iframe");
    const r = f.getBoundingClientRect();
    const tf = [];
    for (let el = f; el; el = el.parentElement) { const t = getComputedStyle(el).transform; if (t !== "none") tf.push(t); }
    return { x: r.left, y: r.top, w: r.width, h: r.height, cw: f.clientWidth, ch: f.clientHeight, tf, zoom: getComputedStyle(f).zoom };
  });
  ok(m.dpr === dpr, `DPR ${dpr}: the frame's devicePixelRatio is the page's (${m.dpr})`);
  for (const c of m.canvases) {
    ok(Math.round(c.cssW * dpr) === c.w && Math.round(c.cssH * dpr) === c.h && c.tf.length === 0 && Number.isInteger(c.x * dpr) && Number.isInteger(c.y * dpr),
      `DPR ${dpr}: canvas ${c.cssW}×${c.cssH} CSS at (${c.x}, ${c.y}) = ${c.w}×${c.h} backing, no transform`);
  }
  // (a translation only — `matrix(1, 0, 0, 1, x, y)` — and to a whole device pixel)
  const translateOnly = outer.tf.every((t) => /^matrix\(1, 0, 0, 1, [-\d.]+, [-\d.]+\)$/.test(t));
  ok(translateOnly && outer.zoom === "1" && Math.round(outer.w) === outer.cw && Math.round(outer.h) === outer.ch,
    `DPR ${dpr}: the run frame isn't scaled (${JSON.stringify(outer)})`);
  ok(Number.isInteger(outer.x * dpr) && Number.isInteger(outer.y * dpr), `DPR ${dpr}: the run frame sits on whole device pixels (${outer.x}, ${outer.y})`);
  ok(errors.length === 0, `DPR ${dpr}: no Studio page errors (${errors.join("; ")})`);
  await page.screenshot({ path: `scratch/studio_run_frame_${dpr}x.png` });
  await page.close();
}

await browser.close();
if (failed) { console.log(`\nStudio run frame: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nStudio run frame: ALL CHECKS PASSED");
