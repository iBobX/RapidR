// QCANVAS in the IDE preview draws through the shared bitmap model
// (rapidr_value::objects), so the HTML canvas shows exactly the pixels the
// program reads back with Canvas.Pixel — shapes, text in the built-in
// Liberation fonts, Cls, the RapidR-style DrawText / Circle(cx, cy, r).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_canvas.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate(() => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'CREATE Form AS QFORM',
    '  Width = 300',
    '  Height = 220',
    '  CREATE C AS QCANVAS',
    '    Left = 10',
    '    Top = 10',
    '    Width = 120',
    '    Height = 70',
    '    Color = &H00FF00',
    '  END CREATE',
    'END CREATE',
    'C.FillRect(0, 0, 120, 14, &H804000)',
    'C.Font.Name = "Arial"',
    'C.Font.Size = 9',
    'C.TextOut(3, 0, "Canvas", &HFFFFFF, -1)',
    'C.Font.Name = "Courier New"',
    'C.TextOut(3, 20, "mono 42", &H0000C0, -1)',
    'C.Line(0, 69, 119, 30, &H00A000)',
    'C.Rectangle(70, 20, 110, 40, &HFF0000)',
    'C.Circle(30, 50, 8, &H0000FF)',
    'C.DrawText(60, 44, "Draw", 0, 12)',
    'PRINT "tw="; C.TextWidth("Canvas")',
    'FOR y = 0 TO 69 STEP 3',
    '  s$ = ""',
    '  FOR x = 0 TO 119 STEP 3',
    '    s$ = s$ + STR$(C.Pixel(x, y)) + ","',
    '  NEXT x',
    '  PRINT "R"; s$',
    'NEXT y',
    'Form.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(3000);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
const rows = out.split("\n").filter((l) => l.startsWith("R")).map((l) => l.slice(1).split(",").filter(Boolean).map((v) => Number(v.trim())));
ok(rows.length === 24 && rows[0].length === 40, `the program read ${rows.length} rows of pixels back`);
ok(/tw=\d+/.test(out), `TextWidth on a canvas (${(out.match(/tw=\d+/) || [""])[0]})`);

const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const shown = await frame.evaluate(() => {
  const canvas = document.getElementById("rr-c");
  if (!canvas) return null;
  const d = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height);
  const at = (x, y) => { const i = (y * d.width + x) * 4; return d.data[i] | (d.data[i + 1] << 8) | (d.data[i + 2] << 16); };
  const out = [];
  for (let y = 0; y < 70; y += 3) { const r = []; for (let x = 0; x < 120; x += 3) r.push(at(x, y)); out.push(r); }
  return { w: canvas.width, h: canvas.height, rows: out };
});
ok(shown && shown.w === 120 && shown.h === 70, `the HTML canvas has the control's size (${shown && shown.w}x${shown && shown.h})`);
let diff = 0, ink = 0;
if (shown) for (let y = 0; y < rows.length; y++) for (let x = 0; x < 40; x++) {
  if (rows[y][x] !== shown.rows[y][x]) diff++;
  // (undrawn: the form's face shows — RapidQ's QCANVAS doesn't paint its
  // own Color, RC.EXE: docs/rapidq-ground-truth.md)
  if (rows[y][x] !== 0xF0F0F0) ink++;
}
ok(shown && diff === 0, `the HTML canvas shows the model's pixels (${diff} of ${rows.length * 40} differ)`);
ok(ink > 100 && ink < rows.length * 40 - 100, `something was drawn (${ink} pixels not the background)`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

// OnPaint: fired when the form is built, and again by Repaint; what the
// handler draws stays on the canvas.
await page.evaluate(() => {
  window.RapidR.runCommand("run.stop");
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'DECLARE SUB PaintIt',
    'DECLARE SUB Again',
    'DIM paints AS INTEGER',
    'CREATE Form AS QFORM',
    '  Width = 300',
    '  Height = 220',
    '  CREATE B AS QBUTTON',
    '    Caption = "Again"',
    '    Left = 10',
    '    Top = 100',
    '    OnClick = Again',
    '  END CREATE',
    '  CREATE C AS QCANVAS',
    '    Left = 10',
    '    Top = 10',
    '    Width = 100',
    '    Height = 60',
    '    OnPaint = PaintIt',
    '  END CREATE',
    'END CREATE',
    'SUB PaintIt',
    '  paints = paints + 1',
    '  C.FillRect(0, 0, 30, 30, &HFF)',
    '  PRINT "paint"; paints',
    'END SUB',
    'SUB Again',
    '  C.Repaint',
    'END SUB',
    'Form.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
let out2 = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(/paint1/.test(out2), `OnPaint fired when the form was built (${JSON.stringify(out2.slice(-30))})`);
const frame2 = page.frames().find((f) => f.url().includes("preview.html"));
await frame2.evaluate(() => document.getElementById("rr-b").click());
await page.waitForTimeout(700);
out2 = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(/paint2/.test(out2), `Repaint fires OnPaint again (${JSON.stringify(out2.slice(-30))})`);
const red = await frame2.evaluate(() => {
  const d = document.getElementById("rr-c").getContext("2d").getImageData(10, 10, 1, 1).data;
  return Array.from(d).slice(0, 3).join(",");
});
ok(red === "255,0,0", `what OnPaint drew is on the canvas (${red})`);

// Drawing on a QFORM itself: its surface lies under the controls, and the
// form's own color shows through where nothing is drawn.
await page.evaluate(() => {
  window.RapidR.runCommand("run.stop");
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'DECLARE SUB FormPaint',
    'CREATE Win AS QFORM',
    '  Width = 300',
    '  Height = 200',
    '  OnPaint = FormPaint',
    '  CREATE Btn AS QBUTTON',
    '    Caption = "Top"',
    '    Left = 10',
    '    Top = 60',
    '  END CREATE',
    'END CREATE',
    'SUB FormPaint',
    '  Win.FillRect(10, 10, 60, 40, &H0000FF)',
    '  Win.TextOut(80, 10, "Hello", &H000000, -1)',
    '  PRINT "tw="; Win.TextWidth("Hello")',
    '  FOR y = 10 TO 39 STEP 5',
    '    s$ = ""',
    '    FOR x = 10 TO 109 STEP 5',
    '      s$ = s$ + STR$(Win.Pixel(x, y)) + ","',
    '    NEXT x',
    '    PRINT "F"; s$',
    '  NEXT y',
    'END SUB',
    'Win.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
const out3 = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
const frows = out3.split("\n").filter((l) => /^F\d/.test(l)).map((l) => l.slice(1).split(",").filter(Boolean).map((v) => Number(v.trim())));
ok(frows.length === 6 && frows[0].length === 20, `the form's pixels were read back (${frows.length} rows)`);
const frame3 = page.frames().find((f) => f.url().includes("preview.html"));
const fshown = await frame3.evaluate(() => {
  const c = document.querySelector('canvas[id$="-surface"]');
  const btn = document.getElementById("rr-btn");
  if (!c || !btn) return { missing: true, canvas: !!c, btn: !!btn };
  const d = c.getContext("2d").getImageData(0, 0, c.width, c.height);
  const at = (x, y) => { const i = (y * d.width + x) * 4; return { rgb: d.data[i] | (d.data[i + 1] << 8) | (d.data[i + 2] << 16), a: d.data[i + 3] }; };
  const rows = [];
  for (let y = 10; y < 40; y += 5) { const r = []; for (let x = 10; x < 110; x += 5) r.push(at(x, y)); rows.push(r); }
  const before = btn.compareDocumentPosition(c) & Node.DOCUMENT_POSITION_PRECEDING;
  return { rows, under: !!before, events: getComputedStyle(c).pointerEvents };
});
ok(!fshown.missing, "the form's surface canvas exists next to its controls");
let fdiff = 0, painted = 0, clear = 0;
if (!fshown.missing) for (let y = 0; y < frows.length; y++) for (let x = 0; x < 20; x++) {
  const m = frows[y][x], w = fshown.rows[y][x];
  if (w.a === 0) { clear++; } else { painted++; if (w.rgb !== m) fdiff++; }
  if (w.a === 0 && m !== 0xF0F0F0) fdiff++;
}
ok(!fshown.missing && fdiff === 0, `the browser shows the model's pixels (${fdiff} differ; ${painted} drawn, ${clear} see-through)`);
ok(!fshown.missing && painted > 20 && clear > 20, "drawn pixels are opaque and the rest lets the form show");
ok(!fshown.missing && fshown.under && fshown.events === "none", "it lies under the controls and takes no mouse events");

await page.screenshot({ path: "scratch/web_ide_canvas.png" });
await browser.close();
if (failed) { console.log(`\nCanvas: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nCanvas: ALL CHECKS PASSED");
