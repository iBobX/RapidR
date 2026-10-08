// A program's windows on the web are drawn at exactly 1:1 logical pixels:
// each canvas's CSS size × devicePixelRatio is its backing store's size in
// both axes, nothing scaled or zoomed by CSS, at DPR 1 and 2 (the program:
// examples/gui/menus.rr, on the runtime's own page, tests/web_run.mjs).
// RapidR Studio's run frame keeps it so too: tests/studio_run_frame.mjs.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_pixels.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("../examples/gui/menus.rr", import.meta.url), "utf8");
const browser = await chromium.launch();
for (const dpr of [1, 2]) {
  const r = await openRunner(browser, { viewport: { width: 1400, height: 900 }, deviceScaleFactor: dpr });
  const page = r.page;
  await r.run(source, { name: "menus.rr" });
  await page.waitForFunction(() => [...document.querySelectorAll("canvas")].some((c) => c.offsetWidth > 0), null, { timeout: 20000 });
  await page.waitForTimeout(500);
  const m = await page.evaluate(() => ({
    dpr: devicePixelRatio,
    canvases: [...document.querySelectorAll("canvas")].filter((c) => c.offsetWidth > 0).map((c) => {
      const r = c.getBoundingClientRect();
      let scaled = [];
      for (let el = c; el; el = el.parentElement) {
        const t = getComputedStyle(el).transform;
        if (t !== "none") scaled.push(t);
      }
      return { cssW: r.width, cssH: r.height, x: r.left, y: r.top, w: c.width, h: c.height, transforms: scaled };
    }),
    zoom: getComputedStyle(document.documentElement).zoom,
    body: getComputedStyle(document.body).transform,
  }));
  ok(m.dpr === dpr, `DPR ${dpr}: the page's devicePixelRatio (${m.dpr})`);
  ok(m.canvases.length > 0, `DPR ${dpr}: the program's windows are drawn (${m.canvases.length} canvases)`);
  for (const c of m.canvases) {
    ok(Math.round(c.cssW * dpr) === c.w && Math.round(c.cssH * dpr) === c.h && c.transforms.length === 0,
      `DPR ${dpr}: canvas ${c.cssW}×${c.cssH} CSS × ${dpr} = ${c.w}×${c.h} backing, no transform (${c.transforms.join(" ") || "none"})`);
    ok(Number.isInteger(c.x * dpr) && Number.isInteger(c.y * dpr), `DPR ${dpr}: canvas at (${c.x}, ${c.y}), on whole device pixels`);
  }
  ok(m.zoom === "1" && m.body === "none", `DPR ${dpr}: the page isn't zoomed or transformed`);
  ok(r.pageErrors.length === 0, `DPR ${dpr}: no page errors (${r.pageErrors.join("; ")})`);
  await page.screenshot({ path: `scratch/web_pixels_${dpr}x.png` });
  await page.close();
}
await browser.close();
if (failed) { console.log(`\nWeb pixels: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb pixels: ALL CHECKS PASSED");
