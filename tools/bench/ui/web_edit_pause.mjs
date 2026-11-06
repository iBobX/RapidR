// RapidR Studio for the web with a generated 5,000-line program: the page's
// main-thread work per frame while a letter is typed and taken back and
// typing pauses (the analysis runs in steps between frames there) —
// docs/speed-audit-2026-11.md, "Studio: the edit pause". A measure, not a
// guard (no budgets): run from the repository's root with Studio's web
// build (tools/build_studio_web.sh) served on STUDIO_WEB_URL:
//   node tools/bench/ui/web_edit_pause.mjs
import { execFileSync } from "node:child_process";
import { chromium } from "playwright";

const STUDIO = process.env.STUDIO_WEB_URL || "http://127.0.0.1:18517";
const big = execFileSync("python3", ["tools/bench/ui/gen_source.py", "5000"], { encoding: "utf8", maxBuffer: 64 << 20 });
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
const page = await context.newPage();
const cdp = await context.newCDPSession(page);
await cdp.send("Performance.enable");
const taskMs = async () => (await cdp.send("Performance.getMetrics")).metrics.find((m) => m.name === "TaskDuration").value * 1000;
const frame = () => page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
await page.addInitScript((text) => {
  window.RAPIDR_STUDIO_TEST = {};
  window.RAPIDR_STUDIO_TEST_FILES = [{ path: "bench5000.rr", text }];
}, big);
await page.goto(`${STUDIO}/index.html?fresh&window=normal&theme=rapidr-light&do=wait,view.code,wait&open=bench5000.rr`, { waitUntil: "load" });
await page.waitForFunction(() => window.RAPIDR_STUDIO_STARTED !== undefined, null, { timeout: 60000, polling: 50 });
await page.waitForTimeout(8000);
const win = await page.locator(".rr-kwin canvas").nth(1).boundingBox();
await page.mouse.click(win.x + win.width * 0.45, win.y + win.height * 0.4);
await page.waitForTimeout(2000);
const perFrame = [];
const keys = [];
for (let round = 0; round < 3; round++) {
  for (const k of ["x", "Backspace"]) {
    const t0 = await taskMs();
    await page.keyboard.press(k);
    await frame();
    keys.push((await taskMs()) - t0);
  }
  // (the pause: each frame's work for 2 s — the editor's diagnose at 350 ms,
  // Studio's Analyze at 600, the steps after them)
  const until = Date.now() + 2000;
  while (Date.now() < until) {
    const t0 = await taskMs();
    await frame();
    perFrame.push((await taskMs()) - t0);
  }
}
perFrame.sort((a, b) => a - b);
keys.sort((a, b) => a - b);
const pct = (v, p) => v[Math.min(v.length - 1, Math.round((v.length - 1) * p))];
console.log(`keys: ${keys.length}, max ${keys.at(-1).toFixed(1)} ms; pause frames: ${perFrame.length}, p50 ${pct(perFrame, 0.5).toFixed(1)} ms, p95 ${pct(perFrame, 0.95).toFixed(1)} ms, max ${perFrame.at(-1).toFixed(1)} ms; the 8 longest: ${perFrame.slice(-8).map((x) => x.toFixed(0)).join(" ")}`);
await browser.close();
