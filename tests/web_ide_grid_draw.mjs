// QSTRINGGRID OnDrawCell, range selection and gcsList drop-downs in the IDE
// preview (the web interpreter): runs
// tests/fixtures/grid_draw_cell.bas — the program tests/native_gui_events.mjs
// checks natively and interpreted — and checks the same values, that the
// handler's drawing is shown over its cell, and that Repaint fires it again.
// (On the kernel host the grid is pixels in its window's client canvas; the
// accessibility mirror gives each cell's place — rows of gridcells, with
// aria-selected — and the mouse acts there as the user's does.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_grid_draw.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

// Cell (col, row) of the grid as the mirror places it: { x, y, width,
// height } inside the grid (CSS pixels), and its aria-selected. (rows and
// cells sorted by their places, the fixed row and column included)
const cells = (where) => where.evaluate(() => {
  const g = document.getElementById("rr-grid").getBoundingClientRect();
  return [...document.querySelectorAll('#rr-grid [role="row"]')]
    .map((row) => [...row.querySelectorAll('[role="gridcell"]')].map((e) => {
      const r = e.getBoundingClientRect();
      return { x: r.left - g.left, y: r.top - g.top, width: r.width, height: r.height, selected: e.getAttribute("aria-selected") === "true" };
    }).sort((a, b) => a.x - b.x))
    .filter((row) => row.length).sort((a, b) => a[0].y - b[0].y);
});
const cellAt = async (where, c, r) => (await cells(where))[r]?.[c];
// (the colour [r, g, b] the window shows at (x, y) inside the grid)
const gridPixel = async (where, x, y) => k.pixelAt(where, "Grid", x, y);
// (the grid's place on the page, for the mouse)
const gridBox = (where) => where.locator("#rr-grid").boundingBox();
const mouseAt = async (where, x, y) => { const g = await gridBox(where); return [g.x + x, g.y + y]; };

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
await k.waitFor(frame, "Grid");
const text = (name) => k.text(frame, name);

ok((await text("Lbl")) === "round1:25|0,130,25,194,49,two|fixed4 selected3", `same values as the desktop builds (${await text("Lbl")})`);
// (the handler drew a red rectangle 2 pixels inside cell (2, 1): its left
// side is red; the same spot of the cells around it isn't)
const red = (p) => p && p.join(",") === "255,0,0";
const c21 = await cellAt(frame, 2, 1);
const edge = c21 && await gridPixel(frame, c21.x + 2, c21.y + 10);
const inside = c21 && await gridPixel(frame, c21.x + 20, c21.y + 10);
let elsewhere = 0;
for (const [c, r] of [[1, 1], [3, 1], [2, 2], [2, 0], [1, 2], [3, 3]]) {
  const cell = await cellAt(frame, c, r);
  if (cell && red(await gridPixel(frame, cell.x + 2, cell.y + 10))) elsewhere++;
}
ok(red(edge), `the handler's red rectangle is drawn over its cell (${edge})`);
ok(c21 && !red(inside) && elsewhere === 0, `only there (${inside}, ${elsewhere} other cells)`);

await k.click(frame, "Btn");
await page.waitForTimeout(800);
ok((await text("Lbl")) === "round2:25|0,130,25,194,49,two|fixed4 selected3", `Repaint fires OnDrawCell again, once per cell (${await text("Lbl")})`);

// Range selection and a gcsList drop-down (tests/fixtures/grid_range_list.bas).
const rangeSource = readFileSync(new URL("./fixtures/grid_range_list.bas", import.meta.url), "utf8");
await page.evaluate((src) => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
  window.RapidR.runCommand("run.start");
}, rangeSource);
await page.waitForTimeout(2500);
const frame2 = page.frames().find((f) => f.url().includes("preview.html"));
await k.waitFor(frame2, "Grid");
const text2 = (name) => k.text(frame2, name);
ok((await text2("Lbl")) === "selected1", `one cell selected at first (${await text2("Lbl")})`);
const a = await cellAt(frame2, 1, 1), b = await cellAt(frame2, 3, 2);
const background = await gridPixel(frame2, (await cellAt(frame2, 4, 3)).x + 3, (await cellAt(frame2, 4, 3)).y + 3);
await page.mouse.move(...await mouseAt(frame2, a.x + 10, a.y + 5));
await page.mouse.down();
const [bx, by] = await mouseAt(frame2, b.x + 10, b.y + 5);
await page.mouse.move(bx, by, { steps: 5 });
await page.mouse.up();
await page.waitForTimeout(500);
ok((await text2("Lbl")) === "selected6", `dragging selects a range; OnDrawCell's State shows it (${await text2("Lbl")})`);
// (the mirror says which cells are selected; the window draws them in the
// selection's colour, not the background of a cell outside the range)
const grid = await cells(frame2);
const marked = grid.flat().filter((c) => c.selected).length;
let painted = 0;
for (let r = 1; r <= 2; r++) for (let c = 1; c <= 3; c++) {
  const cell = grid[r][c];
  const p = await gridPixel(frame2, cell.x + 3, cell.y + 3);
  if (p && p.join(",") !== background.join(",")) painted++;
}
ok(marked === 6 && painted === 6, `the range is highlighted (${marked} cells selected, ${painted} drawn so; background ${background})`);
await k.click(frame2, "Grid", [a.x + 10, a.y + 5]);
await page.waitForTimeout(400);
ok((await text2("Lbl")) === "selected1", `a click selects one cell again (${await text2("Lbl")})`);

// (the drop-down's button: the right end of the selected gcsList cell)
await k.click(frame2, "Grid", [a.x + a.width - 6, a.y + a.height / 2]);
await page.waitForTimeout(300);
// (the drop-down as the accessibility mirror describes it)
const listed = await frame2.evaluate(() => [...document.querySelectorAll('[role="listbox"] [role="option"], [role="menu"] [role="menuitem"]')].map((o) => o.getAttribute("aria-label") ?? o.textContent));
ok(JSON.stringify(listed) === JSON.stringify(["red", "blue", "green", "pink"]), `the gcsList drop-down lists ColumnList, as OnListDropDown's S left it (${JSON.stringify(listed)})`);
// (the drop-down as drawn: a box hanging from the cell, its left border a
// dark line; each item's text a band of dark pixels inside it)
const dropDown = () => frame2.evaluate(({ a }) => {
  const el = document.getElementById("rr-grid");
  const canvas = el.closest(".rr-kwin").querySelector("canvas.rr-kclient");
  const c = canvas.getBoundingClientRect(), g = el.getBoundingClientRect(), s = canvas.width / c.width;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  const dark = (x, y) => { const d = ctx.getImageData(Math.floor((g.left - c.left + x + 0.5) * s), Math.floor((g.top - c.top + y + 0.5) * s), 1, 1).data; return (d[0] + d[1] + d[2]) / 3 < 64; };
  const top = a.y + a.height;
  let height = 0;
  while (height < 200 && dark(a.x, top + height)) height++;
  if (height < 10) return { height, bands: [] };
  const bands = [];
  let start = -1;
  for (let y = top + 2; y < top + height - 1; y++) {
    let ink = false;
    for (let x = a.x + 2; x < a.x + a.width - 2 && !ink; x++) ink = dark(x, y);
    if (ink && start < 0) start = y;
    if (!ink && start >= 0) { bands.push((start + y) / 2); start = -1; }
  }
  if (start >= 0) bands.push((start + top + height - 1) / 2);
  return { height, bands };
}, { a });
let dd = await dropDown();
ok(dd.bands.length === 4, `the drop-down shows four items: ColumnList's three and the one OnListDropDown added (${dd.bands.length} drawn, ${dd.height} pixels tall)`);
// (a real click on the second item)
if (dd.bands.length >= 2) await k.click(frame2, "Grid", [a.x + 10, dd.bands[1]]);
await page.waitForTimeout(400);
ok((await text2("Info")) === "set1,1 blue blue", `picking stores it, then OnSetEditText (${await text2("Info")})`);
ok((await dropDown()).height < 10, "the drop-down closes");
// (again, the fourth item: what OnListDropDown added)
await k.click(frame2, "Grid", [a.x + a.width - 6, a.y + a.height / 2]);
await page.waitForTimeout(300);
dd = await dropDown();
if (dd.bands.length >= 4) await k.click(frame2, "Grid", [a.x + 10, dd.bands[3]]);
await page.waitForTimeout(400);
ok((await text2("Info")) === "set1,1 pink pink", `the item OnListDropDown added is the last one (${await text2("Info")})`);
// goColSizing: dragging a header's right border resizes the column.
const head = await cellAt(frame2, 1, 0);
const [hx, hy] = await mouseAt(frame2, head.x + head.width - 2, head.y + 5);
await page.mouse.move(hx, hy);
await page.mouse.down();
await page.mouse.move(hx + 40, hy, { steps: 4 });
await page.mouse.up();
await page.waitForTimeout(400);
const width = Math.round((await cellAt(frame2, 1, 0)).width);
ok(width >= head.width + 36 && width <= head.width + 42, `dragging a header border resizes its column (${Math.round(head.width)} → ${width})`);
ok((await text2("Lbl")) === "selected1", `resizing doesn't select a cell (${await text2("Lbl")})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_grid_draw.png" });
await browser.close();
if (failed) { console.log(`\nGrid drawing: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGrid drawing: ALL CHECKS PASSED");
