// QSTRINGGRID OnDrawCell, range selection and gcsList drop-downs in the IDE
// preview (the web interpreter): runs
// tests/fixtures/grid_draw_cell.bas — the program tests/native_gui_events.mjs
// checks natively and interpreted — and checks the same values, that the
// handler's drawing is shown over its cell, and that Repaint fires it again.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_grid_draw.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/grid_draw_cell.bas", import.meta.url), "utf8");
const browser = await chromium.launch();
const page = await browser.newPage();
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

ok((await text("rr-lbl")) === "round1:25|0,130,25,194,49,two|fixed4 selected3", `same values as the desktop builds (${await text("rr-lbl")})`);
const drawn = await frame.evaluate(() => {
  const td = document.querySelector('#rr-grid td[data-col="2"][data-row="1"]');
  const canvas = td?.querySelector("canvas");
  if (!canvas) return null;
  const px = (x, y) => Array.from(canvas.getContext("2d").getImageData(x, y, 1, 1).data).join(",");
  return { edge: px(2, 10), inside: px(20, 10), others: document.querySelectorAll("#rr-grid td canvas").length };
});
ok(drawn?.edge === "255,0,0,255", `the handler's red rectangle is drawn over its cell (${drawn?.edge})`);
ok(drawn?.inside === "0,0,0,0" && drawn?.others === 1, `only there (${drawn?.inside}, ${drawn?.others} canvas)`);

await frame.click("#rr-btn");
await page.waitForTimeout(800);
ok((await text("rr-lbl")) === "round2:25|0,130,25,194,49,two|fixed4 selected3", `Repaint fires OnDrawCell again, once per cell (${await text("rr-lbl")})`);

// Range selection and a gcsList drop-down (tests/fixtures/grid_range_list.bas).
const rangeSource = readFileSync(new URL("./fixtures/grid_range_list.bas", import.meta.url), "utf8");
await page.evaluate((src) => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
  window.RapidR.runCommand("run.start");
}, rangeSource);
await page.waitForTimeout(2500);
const frame2 = page.frames().find((f) => f.url().includes("preview.html"));
const text2 = (id) => frame2.evaluate((id) => document.getElementById(id)?.textContent ?? null, id);
ok((await text2("rr-lbl")) === "selected1", `one cell selected at first (${await text2("rr-lbl")})`);
const cellBox = (c, r) => frame2.locator(`#rr-grid td[data-col="${c}"][data-row="${r}"]`).boundingBox();
const a = await cellBox(1, 1), b = await cellBox(3, 2);
await page.mouse.move(a.x + 10, a.y + 5);
await page.mouse.down();
await page.mouse.move(b.x + 10, b.y + 5, { steps: 5 });
await page.mouse.up();
await page.waitForTimeout(500);
ok((await text2("rr-lbl")) === "selected6", `dragging selects a range; OnDrawCell's State shows it (${await text2("rr-lbl")})`);
const highlighted = await frame2.evaluate(() => document.querySelectorAll('#rr-grid td[aria-selected="true"]').length);
ok(highlighted === 6, `the range is highlighted (${highlighted} cells)`);
await frame2.click('#rr-grid td[data-col="1"][data-row="1"]');
await page.waitForTimeout(400);
ok((await text2("rr-lbl")) === "selected1", `a click selects one cell again (${await text2("rr-lbl")})`);
await frame2.click('#rr-grid td[data-col="1"][data-row="1"] .rr-grid-list');
await page.waitForTimeout(300);
const listed = await frame2.evaluate(() => [...document.querySelectorAll(".rr-grid-dropdown-item")].map((d) => d.textContent));
ok(JSON.stringify(listed) === JSON.stringify(["red", "blue", "green", "pink"]), `the gcsList drop-down lists ColumnList, as OnListDropDown's S left it (${JSON.stringify(listed)})`);
await frame2.locator(".rr-grid-dropdown-item", { hasText: "blue" }).dispatchEvent("mousedown");
await page.waitForTimeout(400);
ok((await text2("rr-info")) === "set1,1 blue blue", `picking stores it, then OnSetEditText (${await text2("rr-info")})`);
ok(await frame2.evaluate(() => !document.querySelector(".rr-grid-dropdown")), "the drop-down closes");
// goColSizing: dragging a header's right border resizes the column.
const head = await cellBox(1, 0);
await page.mouse.move(head.x + head.width - 2, head.y + 5);
await page.mouse.down();
await page.mouse.move(head.x + head.width + 38, head.y + 5, { steps: 4 });
await page.mouse.up();
await page.waitForTimeout(400);
const width = Math.round((await cellBox(1, 0)).width);
ok(width >= head.width + 36 && width <= head.width + 42, `dragging a header border resizes its column (${Math.round(head.width)} → ${width})`);
ok((await text2("rr-lbl")) === "selected1", `resizing doesn't select a cell (${await text2("rr-lbl")})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_grid_draw.png" });
await browser.close();
if (failed) { console.log(`\nGrid drawing: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGrid drawing: ALL CHECKS PASSED");
