// examples/graphics/pixels_fast.rr on the web runtime draws its plasma at
// 50 frames a second: its readout (the frames a second it really drew in the
// last second, and where each frame's time went) is read after a few seconds
// at device scale 1 and 2. The test is loose (40 or more) so a busy machine
// does not fail it; the number it prints is the measurement.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_pixels_fps.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("../examples/graphics/pixels_fast.rr", import.meta.url), "utf8");
const browser = await chromium.launch();
for (const dpr of [1, 2]) {
  const r = await openRunner(browser, { viewport: { width: 700, height: 700 }, deviceScaleFactor: dpr });
  await r.run(source, { name: "pixels_fast.rr" });
  const seen = await r.until(async () => /fps/.test(String(await r.prop("info", "caption"))), 20000);
  ok(seen, `device scale ${dpr}: the readout appears`);
  // (a few more readouts, the last one counts)
  await r.page.waitForTimeout(3500);
  const caption = String(await r.prop("info", "caption"));
  const fps = parseFloat(caption);
  console.log(`  device scale ${dpr}: "${caption.trim()}"`);
  ok(fps >= 40, `device scale ${dpr}: ${fps} frames a second (40 or more; the target is 50)`);
  ok(r.pageErrors.length === 0, `device scale ${dpr}: no page errors (${r.pageErrors.join("; ")})`);
  if (process.env.PIXELS_FPS_SHOT) await r.page.screenshot({ path: `${process.env.PIXELS_FPS_SHOT}/web_pixels_fps_${dpr}x.png` });
  await r.page.close();
}
await browser.close();
if (failed) { console.log(`\nWeb pixels fps: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb pixels fps: ALL CHECKS PASSED");
