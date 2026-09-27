// Align in the IDE preview (the web interpreter), laid out as on the desktop
// (rapidr_value::layout): runs tests/fixtures/align_layout.bas — the program
// tests/native_gui_events.mjs checks natively and interpreted — and checks
// the same numbers, the elements' places, and maximizing the form (OnResize).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_align.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/align_layout.bas", import.meta.url), "utf8");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate((src) => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
  window.RapidR.runCommand("run.start");
}, source);
await page.waitForTimeout(2500);
const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");

const text = (id) => frame.evaluate((id) => document.getElementById(id)?.textContent ?? null, id);
const rect = (id) => frame.evaluate((id) => {
  const el = document.getElementById(id);
  if (!el) return null;
  return [el.offsetLeft, el.offsetTop, el.offsetWidth, el.offsetHeight];
}, id);

// The web form's client area is 400 × (300 − 29 title bar) = 400 × 271.
ok((await text("rr-loose")) === "105,40,235,207|100|247", `laid out before the form is shown (${await text("rr-loose")})`);
const places = {
  bar: await rect("rr-bar"), status: await rect("rr-status"), split: await rect("rr-split"),
  tree: await rect("rr-tree"), side: await rect("rr-side"), memo: await rect("rr-memo"), loose: await rect("rr-loose"),
};
ok(JSON.stringify(places.bar) === "[0,0,400,40]", `alTop panel (${JSON.stringify(places.bar)})`);
ok(JSON.stringify(places.status) === "[0,247,400,24]", `status bar docked at the bottom (${JSON.stringify(places.status)})`);
ok(JSON.stringify(places.tree) === "[0,40,100,207]", `alLeft list (${JSON.stringify(places.tree)})`);
ok(JSON.stringify(places.split) === "[100,40,5,207]", `splitter after it (alLeft by default) (${JSON.stringify(places.split)})`);
ok(JSON.stringify(places.side) === "[340,40,60,207]", `alRight panel (${JSON.stringify(places.side)})`);
ok(JSON.stringify(places.memo) === "[105,40,235,207]", `alClient fills the rest (${JSON.stringify(places.memo)})`);
ok(places.loose && places.loose[0] === 150 && places.loose[1] === 60, `a control without Align stays put (${JSON.stringify(places.loose)})`);

await frame.click("#rr-btn");
await page.waitForTimeout(300);
ok((await text("rr-status"))?.includes("295|207|5|400"), `hiding the alRight panel widens alClient (${await text("rr-status")})`);
const memoAfter = await rect("rr-memo");
ok(JSON.stringify(memoAfter) === "[105,40,295,207]", `memo moved (${JSON.stringify(memoAfter)})`);

// Maximize: the form fills the preview; OnResize reports the new layout.
await frame.click("#rr-form .rr-form-btn-max");
await page.waitForTimeout(400);
const sizes = await frame.evaluate(() => [innerWidth, innerHeight]);
const resized = await text("rr-bar");
const expected = `${sizes[0]}x${sizes[1]}|${sizes[0] - 105}x${sizes[1] - 29 - 40 - 24}|`;
ok(resized?.startsWith(expected), `maximize lays out again and fires OnResize (${resized}; expected ${expected}…)`);
const memoMax = await rect("rr-memo");
ok(memoMax && memoMax[2] === sizes[0] - 105 && memoMax[3] === sizes[1] - 29 - 40 - 24, `memo fills the maximized form (${JSON.stringify(memoMax)})`);

const panel = await frame.evaluate(() => ({
  button: !!document.querySelector("#rr-bar #rr-btn"),
  caption: document.querySelector("#rr-bar .rr-panel-caption")?.textContent,
}));
ok(panel.button && panel.caption?.startsWith(expected), `a panel's Caption doesn't remove its children (${JSON.stringify(panel)})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_align.png" });
await browser.close();
if (failed) { console.log(`\nAlign: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nAlign: ALL CHECKS PASSED");
