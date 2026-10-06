// RCODEEDITOR's performance on the web (docs/ide-plan.md §6.2): the canvas
// host in Chromium, a 200,000-line / 10.7 MB BASIC file — open to the first
// frame, typing latency (the key's DOM event → the frame that shows it,
// drawn by vello_cpu in the wasm), scrolling frames, memory. The host's
// probe (`window.RAPIDR_FRAME_TIMES`, crates/rapidr-ui-host-web) gives each
// frame's work and when it ended.
//
// Usage (repo root, after tools/build_web_artifacts.sh, the repo served on
// RAPIDR_URL or http://localhost:8765):  node tests/web_editor_perf.mjs [--quick] [--dpr 2]
// Exits 1 when a target is missed (web: open ≤ 800 ms, typing p50 ≤ 16 ms
// and p99 ≤ 33 ms, every scrolled frame's work ≤ 33 ms).

import { chromium } from "playwright";

const BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const args = process.argv.slice(2);
const quick = args.includes("--quick");
const dpr = Number(args[args.indexOf("--dpr") + 1]) || 2;
const runtime = args.includes("--rt") ? args[args.indexOf("--rt") + 1] : "target/web";
const LINES = quick ? 20000 : 200000;

// The same generated BASIC as the desktop's benchmark (codeeditor_bench).
function basicSource(lines) {
  let s = "", n = 0, sub = 0, x = 0x9e3779b9 >>> 0;
  const below = (k) => { x ^= x << 13; x >>>= 0; x ^= x >>> 17; x ^= x << 5; x >>>= 0; return x % k; };
  while (n < lines) {
    sub++;
    s += `SUB Handler${sub}(Sender AS QBUTTON, Value AS INTEGER)\r\n    DIM total AS DOUBLE, name$ AS STRING ' locals\r\n`;
    n += 2;
    const body = 6 + below(30);
    for (let i = 0; i < body; i++) {
      const lines9 = [
        `    IF Value > ${i} THEN PRINT "value is "; Value; " in ${sub}"`,
        `    total = total + SQR(Value * ${i}.5) / (1 + ABS(Value - ${i}))`,
        `    ' a comment about what happens next, long enough to wrap`,
        `    name$ = LEFT$("Handler number ${sub}", ${i}) + MID$(name$, 2, 3)`,
        `    FOR i = 1 TO 100 STEP 2: total = total + i: NEXT i`,
        `    Form1.Caption = "Total: " + STR$(total) + " (${i})"`,
        `    SELECT CASE Value: CASE 1: PRINT 1: CASE ELSE: PRINT 0: END SELECT`,
        `    CALL Handler${Math.max(sub, 2) - 1}(Sender, Value - 1)`,
        `    WHILE total < ${i}00: total = total * 1.5: WEND`,
      ];
      s += lines9[below(9)] + "\r\n";
      n++;
    }
    s += "END SUB\r\n\r\n";
    n += 2;
  }
  return s;
}

const pct = (v, p) => { const s = [...v].sort((a, b) => a - b); return s[Math.round((s.length - 1) * p)]; };
const ms = (v) => `${v.toFixed(1)} ms`;
let failed = [];
const row = (what, value, target, ok) => {
  if (target && !ok) failed.push(what);
  console.log(`${what.padEnd(50)} ${value.padStart(18)}  ${target ? `${target.padStart(10)}  ${ok ? "ok" : "MISSED"}` : ""}`);
};

const big = basicSource(LINES);
console.log(`RCODEEDITOR on the web: ${LINES} lines, ${(big.length / 1e6).toFixed(1)} MB, a 900×640 window at ${dpr}× (${runtime})\n`);
const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: dpr, viewport: { width: 1000, height: 760 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto(`${BASE}/tests/web_kernel.html`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady, null, { timeout: 20000 });
const program = [
  "CREATE Form AS QFORM",
  "  Caption = \"perf\"",
  "  Width = 900: Height = 640",
  "  CREATE Ed AS RCODEEDITOR",
  "    Left = 0: Top = 0: Width = 884: Height = 601",
  "  END CREATE",
  "END CREATE",
  "Ed.LoadFromFile \"big.bas\"",
  `Ed.GotoLineColumn ${LINES / 2}, 5`,
  "Ed.SetFocus",
  "Form.ShowModal",
].join("\n");
const open = await page.evaluate(async ({ program, big }) => {
  const dataUrl = "data:application/octet-stream;base64," + btoa(big);
  const bc = window.rr.compile(program, "perf", [{ name: "big.bas", mime: "application/octet-stream", dataUrl }]);
  window.RAPIDR_FRAME_TIMES = [];
  const t0 = Date.now();
  window.rr.rapidr_run_bc(bc);
  for (let i = 0; i < 400 && window.RAPIDR_FRAME_TIMES.length === 0; i++) await new Promise((r) => setTimeout(r, 10));
  const first = window.RAPIDR_FRAME_TIMES[0];
  return first ? first[1] - t0 : -1;
}, { program, big });
row("open: LoadFromFile + the first frame", ms(open), "≤ 800 ms", open >= 0 && open <= 800);

// typing: each key's DOM event → the frame that shows it
await page.waitForTimeout(500);
await page.evaluate(() => {
  window.__keys = [];
  window.addEventListener("keydown", () => window.__keys.push(Date.now()), true);
});
const text = "x = total + 1";
const latencies = [];
for (let round = 0; round < (quick ? 4 : 10); round++) {
  for (const ch of [...text, "Enter"]) {
    const before = await page.evaluate(() => window.RAPIDR_FRAME_TIMES.length);
    await page.keyboard.press(ch === " " ? "Space" : ch);
    const lat = await page.evaluate(async (before) => {
      for (let i = 0; i < 200 && window.RAPIDR_FRAME_TIMES.length <= before; i++) await new Promise((r) => setTimeout(r, 2));
      const f = window.RAPIDR_FRAME_TIMES[window.RAPIDR_FRAME_TIMES.length - 1];
      return f[1] - window.__keys[window.__keys.length - 1];
    }, before);
    latencies.push(lat);
  }
}
row(`typing (${latencies.length} keys): key → frame p50`, ms(pct(latencies, 0.5)), "≤ 16 ms", pct(latencies, 0.5) <= 16);
row("typing: key → frame p99", ms(pct(latencies, 0.99)), "≤ 33 ms", pct(latencies, 0.99) <= 33);

// scrolling: a wheel notch at a time
await page.evaluate(() => (window.RAPIDR_FRAME_TIMES = []));
const canvas = await page.locator("canvas").first().boundingBox();
await page.mouse.move(canvas.x + canvas.width / 2, canvas.y + canvas.height / 2);
for (let i = 0; i < (quick ? 60 : 200); i++) {
  await page.mouse.wheel(0, 120);
  await page.waitForTimeout(16);
}
await page.waitForTimeout(200);
const work = await page.evaluate(() => window.RAPIDR_FRAME_TIMES.map((f) => f[0]));
row(`scroll (${work.length} frames): a frame's work p50`, ms(pct(work, 0.5)), "", true);
row("scroll: the longest frame's work", ms(Math.max(...work)), "≤ 33 ms", Math.max(...work) <= 33);

const mem = await page.evaluate(() => (performance.memory ? performance.memory.usedJSHeapSize : 0));
const wasm = await page.evaluate(() => {
  try { return window.rr.__wbg_get_imports ? 0 : 0; } catch { return 0; }
});
if (mem) row("memory: the page's JS heap", `${(mem / 1e6).toFixed(1)} MB`, "", true);
void wasm;
if (errors.length) console.log("page errors: " + errors.join(" | "));
await browser.close();
if (failed.length) {
  console.log(`\nmissed: ${failed.join("; ")}`);
  process.exit(1);
}
console.log("\nall targets met");
