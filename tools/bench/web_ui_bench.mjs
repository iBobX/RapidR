// web_ui_bench.mjs: the UI's budgets on the web (docs/speed-audit-2026-11.md,
// "Web parity"), in tools/bench/ui_bench's style — the same programs
// (tools/bench/ui/) on the web runtime's own page (tests/web_run.mjs) in
// headless Chromium:
//
//   writes    prop_writes.rr, each case repeated 50 times (the web's TIMER
//             counts whole milliseconds): RapidR Web Server's issue #12 (41
//             moves), 800 place writes and 200 caption writes on a
//             200-control form — one pass's time
//   resize    resize200.rr: its 120 program resizes (the two size writes,
//             the program's own TIMER: their median), then its form dragged
//             by its corner, 120 pointer moves out and back, each step
//             waited for: a step's main-thread work — the page's, all of it:
//             the input, the layout, the events, the paint (Chromium's
//             TaskDuration) — and the frame's paint (the host's probe,
//             RAPIDR_FRAME_TIMES)
//   canvases  resize_canvas12.rr: the program's OnResize (48 writes), and its
//             form dragged as above (OnResize and the 12 OnPaints in a step)
//   studio    RapidR Studio for the web (STUDIO_WEB_URL, tools/build_studio_web.sh)
//             with a generated 5,000-line program: a caret move's (the Down
//             arrow) main-thread work, and its frame's paint
//
// Times are milliseconds, the fastest of --runs runs' medians. `--check`
// compares them with tools/bench/web_ui_budgets.json and exits 1 when one is
// missed (the `perf` stage of tools/regress.sh).
//
//   node tools/bench/web_ui_bench.mjs [--check] [--runs N] [--json FILE] [only…]
//
// Needs the web artifacts (tools/build_web_artifacts.sh) and the repository
// served on RAPIDR_URL (default http://localhost:8765); `studio` needs
// Studio's web build served on STUDIO_WEB_URL (default
// http://127.0.0.1:18473/). Playwright comes from tests/node_modules.

import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { openRunner, URL_BASE } from "../../tests/web_run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const UI = join(HERE, "ui");
const { chromium } = createRequire(join(HERE, "../../tests/"))("playwright");
const STUDIO = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");

const args = process.argv.slice(2);
const check = args.includes("--check");
let runs = 3;
let jsonOut = null;
const only = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--runs") runs = Number(args[++i]);
  else if (args[i] === "--json") jsonOut = args[++i];
  else if (!args[i].startsWith("--")) only.push(args[i]);
}
const want = (n) => !only.length || only.includes(n);

const median = (a) => {
  const s = [...a].sort((x, y) => x - y);
  return s.length ? s[Math.floor(s.length / 2)] : NaN;
};
const pct = (a, p) => {
  const s = [...a].sort((x, y) => x - y);
  return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * p))] : NaN;
};
const number = (out, re) => {
  const m = out.match(re);
  if (!m) throw new Error(`no ${re} in ${out.slice(0, 300)}`);
  return Number(m[1]);
};

/// The fastest of `runs` measures of each value `f` gives (an object).
async function best(f) {
  const out = {};
  for (let i = 0; i < runs; i++) {
    for (const [k, v] of Object.entries(await f())) out[k] = Math.min(out[k] ?? v, v);
  }
  return out;
}

// (the page's main-thread time, all tasks, in ms: Chromium's own count)
async function taskMs(cdp) {
  const { metrics } = await cdp.send("Performance.getMetrics");
  return metrics.find((m) => m.name === "TaskDuration").value * 1000;
}

// (two animation frames: what a step asked for is drawn)
const frames = (page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/// Drags the corner of the window titled `caption` out by (dx, dy) a step
/// for `steps / 2` steps and back, each step waited for; per step: the
/// page's main-thread work, and the paint of the frames it drew.
async function drag(r, cdp, caption, steps = 120, dx = 4, dy = 3) {
  const page = r.page;
  const grip = page.locator(".rr-kwin", { has: page.locator(`[aria-label="${caption}"], [title="${caption}"]`) }).locator('[data-edge="se"]');
  let box = (await grip.count()) ? await grip.first().boundingBox() : null;
  if (!box) box = await page.locator('.rr-kwin [data-edge="se"]').first().boundingBox();
  if (!box) throw new Error(`no window ${caption} to drag`);
  const x0 = box.x + box.width / 2;
  const y0 = box.y + box.height / 2;
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  await frames(page);
  // (an idle step, the same waits without a move: what measuring costs)
  const idle = [];
  for (let i = 0; i < 10; i++) {
    const t0 = await taskMs(cdp);
    await frames(page);
    idle.push((await taskMs(cdp)) - t0);
  }
  await page.evaluate(() => (window.RAPIDR_FRAME_TIMES = []));
  const work = [];
  const half = steps / 2;
  for (let i = 1; i <= steps; i++) {
    const k = i <= half ? i : steps - i;
    const t0 = await taskMs(cdp);
    await page.mouse.move(x0 + k * dx, y0 + k * dy);
    await frames(page);
    work.push((await taskMs(cdp)) - t0);
  }
  await page.mouse.up();
  const paints = await page.evaluate(() => window.RAPIDR_FRAME_TIMES.map((f) => f[0]));
  return { step: median(work), p95: pct(work, 0.95), idle: median(idle), paint: median(paints), drawn: paints.length };
}

/// Runs `source` on the page, waits for `ready` in its output.
async function start(r, source, name, ready, opts = {}) {
  if (!(await r.run(source, { name, ...opts }))) throw new Error(`${name}: ${r.errors().slice(0, 300)}`);
  if (!(await r.waitOutput(ready, 120000))) throw new Error(`${name}: no ${ready} (${r.output().slice(-300)} ${r.errors().slice(0, 300)})`);
}

const browser = await chromium.launch();
console.log(`web_ui_bench: ${URL_BASE}${want("studio") ? `, Studio ${STUDIO}` : ""}; the fastest of ${runs} runs`);
const results = {};
const notes = {};

if (want("writes")) {
  const source = readFileSync(join(UI, "prop_writes.rr"), "utf8");
  const r = await openRunner(browser);
  Object.assign(results, await best(async () => {
    await start(r, source, "prop_writes.rr", /caption200 [\d.]+/, { args: ["50"] });
    const out = r.output();
    await r.stop();
    return Object.fromEntries(["moves41_hidden", "moves41_shown", "geom200_shown", "caption200"].map((k) => [k, number(out, new RegExp(`${k} ([\\d.]+)`))]));
  }));
  await r.page.close();
}

/// A bench form's program steps, then its form dragged (`caption`).
async function resized(file, tag, caption, programValues) {
  const source = readFileSync(join(UI, file), "utf8");
  // (dragged: the program's by-hand mode — its form shown, waiting)
  const byHand = source.replace(/ByHand = VAL\(ENVIRON\$\("RESIZE_BY_HAND"\)\)/, "ByHand = 600");
  if (byHand === source) throw new Error(`${file}: no RESIZE_BY_HAND line`);
  const r = await openRunner(browser, { viewport: { width: 1400, height: 1000 } });
  const cdp = await r.page.context().newCDPSession(r.page);
  await cdp.send("Performance.enable");
  Object.assign(results, await best(async () => {
    await start(r, source, file, programValues.ready);
    const out = r.output();
    await r.stop();
    const values = Object.fromEntries(programValues.read.map(([k, re]) => [k, number(out, re)]));
    if (!(await r.run(byHand, { name: file }))) throw new Error(`${file}: ${r.errors().slice(0, 300)}`);
    await r.until(() => r.page.locator('.rr-kwin [data-edge="se"]').first().isVisible(), 30000);
    await frames(r.page);
    await r.page.waitForTimeout(300);
    const d = await drag(r, cdp, caption);
    await r.stop();
    notes[`${tag}_drag`] = `idle ${d.idle.toFixed(2)} ms, ${d.drawn} frames drawn, p95 ${d.p95.toFixed(2)} ms`;
    return { ...values, [`${tag}_drag_step`]: d.step, [`${tag}_drag_paint`]: d.paint };
  }));
  await r.page.close();
}

if (want("resize")) {
  await resized("resize200.rr", "resize200", "resize200", {
    ready: /resize200 writes .*median=[\d.]+/,
    read: [["resize200_writes", /resize200 writes .*?median=([\d.]+)/]],
  });
}
if (want("canvases")) {
  await resized("resize_canvas12.rr", "canvas12", "canvas12", {
    ready: /canvas12 onpaint .*median=[\d.]+/,
    read: [["canvas12_onresize", /canvas12 onresize .*?median=([\d.]+)/]],
  });
}

if (want("studio")) {
  const big = execFileSync("python3", [join(UI, "gen_source.py"), "5000"], { encoding: "utf8", maxBuffer: 64 << 20 });
  const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  Object.assign(results, await best(async () => {
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send("Performance.enable");
    // (the file in the page's store, as a test gives Studio its files;
    // Studio opens it in its code view)
    await page.addInitScript((text) => {
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_STUDIO_TEST_FILES = [{ path: "bench5000.rr", text }];
    }, big);
    await page.goto(`${STUDIO}/index.html?fresh&window=normal&theme=rapidr-light&do=wait,view.code,wait&open=bench5000.rr`, { waitUntil: "load" });
    await page.waitForFunction(() => window.RAPIDR_STUDIO_STARTED !== undefined, null, { timeout: 60000, polling: 50 });
    // (opened, analysed, drawn: Studio settles)
    await page.waitForTimeout(6000);
    const win = await page.locator(".rr-kwin canvas").nth(1).boundingBox();
    // (a click in the code, left of the middle: the caret there)
    await page.mouse.click(win.x + win.width * 0.45, win.y + win.height * 0.4);
    await page.waitForTimeout(1500);
    await page.evaluate(() => (window.RAPIDR_FRAME_TIMES = []));
    const work = [];
    for (let i = 0; i < 40; i++) {
      const t0 = await taskMs(cdp);
      await page.keyboard.press("ArrowDown");
      await frames(page);
      work.push((await taskMs(cdp)) - t0);
    }
    const paints = await page.evaluate(() => window.RAPIDR_FRAME_TIMES.map((f) => f[0]));
    if (process.env.RAPIDR_BENCH_SHOT) await page.screenshot({ path: process.env.RAPIDR_BENCH_SHOT });
    await page.close();
    notes.studio_caret = `${paints.length} frames drawn for 40 keys, p95 ${pct(work, 0.95).toFixed(2)} ms`;
    return { studio_caret_step: median(work), studio_caret_paint: median(paints) };
  }));
  await context.close();
}
await browser.close();

const budgetsFile = JSON.parse(readFileSync(join(HERE, "web_ui_budgets.json"), "utf8"));
const budgets = budgetsFile.budgets || {};
const labels = {
  moves41_hidden: "#12: 41 moves, form never shown",
  moves41_shown: "#12: 41 moves, form shown",
  geom200_shown: "800 place writes, 200 controls",
  caption200: "200 caption writes",
  resize200_writes: "200-control resize: the two size writes",
  resize200_drag_step: "200-control form dragged: a step's work",
  resize200_drag_paint: "… its frame's paint",
  canvas12_onresize: "12 canvases: OnResize (48 writes)",
  canvas12_drag_step: "12 canvases dragged: a step's work",
  canvas12_drag_paint: "… its frame's paint",
  studio_caret_step: "Studio, 5,000 lines: a caret move's work",
  studio_caret_paint: "… its frame's paint",
};
const missed = [];
for (const [k, v] of Object.entries(results)) {
  const b = budgets[k];
  let verdict = "";
  if (b !== undefined) {
    const ok = v <= b;
    verdict = `  (budget ${b} ms${ok ? "" : ": MISSED"})`;
    if (!ok) missed.push(k);
  }
  console.log(`  ${(labels[k] || k).padEnd(44)}${v.toFixed(2).padStart(9)} ms${verdict}`);
}
for (const [k, n] of Object.entries(notes)) console.log(`  (${k}: ${n})`);
if (jsonOut) writeFileSync(jsonOut, JSON.stringify({ url: URL_BASE, results, notes }, null, 2));
if (check) {
  if (missed.length) {
    console.log(`web_ui_bench: ${missed.length} budget(s) MISSED: ${missed.join(", ")}`);
    process.exit(1);
  }
  console.log("web_ui_bench: every budget met");
}
