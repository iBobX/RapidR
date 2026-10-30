// The pixel benchmark (tools/bench/pixels.rr) on the web runtime: a whole
// 256 x 192 frame put into a bitmap with SetPixels / SetPixels8 and drawn
// twice as large with StretchDraw, in a browser, at device scale 1 and 2.
// Prints the program's table and fails when a frame of the pixel-block
// methods costs 2 ms or more, or the pixels are not the ones put in.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_pixels_bench.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("../tools/bench/pixels.rr", import.meta.url), "utf8");
const browser = await chromium.launch();
for (const dpr of [1, 2]) {
  const r = await openRunner(browser, { viewport: { width: 900, height: 600 }, deviceScaleFactor: dpr });
  await r.run(source, { name: "pixels.rr" });
  await r.waitOutput("checked:", 120000);
  const text = r.output();
  console.log(`\nweb, device scale ${dpr}:\n${text.trim()}\n`);
  const ms = (label) => {
    const m = text.match(new RegExp(`^${label.replace(/[()+]/g, "\\$&")}\\s+([0-9.]+) ms a frame`, "m"));
    return m ? Number(m[1]) : NaN;
  };
  for (const label of ["SetPixels (colours)", "SetPixels8 (palette indexes)", "GetPixels", "StretchDraw 2x (to 512 x 384)", "a whole frame: SetPixels8 + StretchDraw 2x"]) {
    const t = ms(label);
    ok(t < 2, `scale ${dpr}: ${label} ${t} ms a frame (under 2 ms)`);
  }
  ok(/checked: the pixels are the ones put in/.test(text), `scale ${dpr}: the pixels are the ones put in`);
  ok(r.pageErrors.length === 0, `scale ${dpr}: no page errors (${r.pageErrors.join("; ")})`);
  await r.page.close();
}
await browser.close();
if (failed) { console.log(`\nWeb pixels bench: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb pixels bench: ALL CHECKS PASSED");
