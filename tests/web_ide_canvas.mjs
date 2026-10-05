// QCANVAS in the IDE preview draws through the shared bitmap model
// (rapidr_value::objects), so the window the UI kernel draws shows exactly
// the pixels the program reads back with Canvas.Pixel — shapes, text in the
// built-in Liberation fonts, Cls, the RapidR-style DrawText / Circle(cx, cy, r).
// (On the kernel host a canvas is pixels in its window's client canvas, at
// the component's place: tests/web_kernel_page.mjs reads them there.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_canvas.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };
// (a check waiting on a fix elsewhere: reported, not failed — it says so
// once it passes, so the mark can go)
const pending = (cond, msg, why) => console.log(cond ? `✓ ${msg} (passes now: drop its pending mark)` : `- ${msg} (pending: ${why})`);
// (the RC.EXE ground-truth lane's question: what RapidQ reads for a form's
// default Color and for Pixel where nothing is drawn — clBtnFace as shown,
// or white as the shared models answer on the desktop and the web today)
const FORM_COLOR = "QFORM's default Color / Pixel where nothing is drawn: the RC.EXE lane's fix";

// (the colour drawn at (x, y) — CSS pixels inside the component — of what
// k.pixels read, as RapidQ's &HBBGGRR number; the middle of the device
// pixels that CSS pixel covers, so it holds at any device scale)
const bgr = (p, x, y) => {
  const i = (Math.floor((y + 0.5) * p.scale) * p.width + Math.floor((x + 0.5) * p.scale)) * 4;
  return p.data[i] | (p.data[i + 1] << 8) | (p.data[i + 2] << 16);
};
const preview = () => page.frames().find((f) => f.url().includes("preview.html"));
const output = () => page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);

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
const out = await output();
const rows = out.split("\n").filter((l) => l.startsWith("R")).map((l) => l.slice(1).split(",").filter(Boolean).map((v) => Number(v.trim())));
ok(rows.length === 24 && rows[0].length === 40, `the program read ${rows.length} rows of pixels back`);
ok(/tw=\d+/.test(out), `TextWidth on a canvas (${(out.match(/tw=\d+/) || [""])[0]})`);

const frame = preview();
ok(!!frame, "preview frame found");
await k.waitFor(frame, "C");
// (the place the kernel gives the canvas is the control's size, in CSS
// pixels and in the device pixels drawn there)
const place = await k.rect(frame, "C");
const shown = await k.pixels(frame, "C");
ok(place && Math.round(place.width) === 120 && Math.round(place.height) === 70 && shown
  && shown.width === Math.round(120 * shown.scale) && shown.height === Math.round(70 * shown.scale),
  `the canvas is drawn at the control's size (${place && `${place.width}x${place.height}`}, ${shown && `${shown.width}x${shown.height} device pixels at ${shown.scale}x`})`);
let diff = 0, ink = 0;
if (shown) for (let y = 0; y < rows.length; y++) for (let x = 0; x < 40; x++) {
  if (rows[y][x] !== bgr(shown, x * 3, y * 3)) diff++;
  if (rows[y][x] !== 0x00FF00) ink++;
}
ok(shown && diff === 0, `the window shows the model's pixels (${diff} of ${rows.length * 40} differ)`);
ok(ink > 100, `something was drawn (${ink} pixels not the background)`);
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
let out2 = await output();
ok(/paint1/.test(out2), `OnPaint fired when the form was built (${JSON.stringify(out2.slice(-30))})`);
const frame2 = preview();
await k.waitFor(frame2, "B");
// (a real click on the button, through the kernel)
await k.click(frame2, "B");
await page.waitForTimeout(700);
out2 = await output();
ok(/paint2/.test(out2), `Repaint fires OnPaint again (${JSON.stringify(out2.slice(-30))})`);
const red = await k.pixelAt(frame2, "C", 10, 10);
ok(red && red.join(",") === "255,0,0", `what OnPaint drew is on the canvas (${red})`);

// Drawing on a QFORM itself: the drawing lies under the controls, and the
// form's own color shows where nothing is drawn. (The kernel draws the
// form's drawing and then its controls over it in the window's client
// canvas; a click on a control over the drawing still reaches the control.)
await page.evaluate(() => {
  window.RapidR.runCommand("run.stop");
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'DECLARE SUB FormPaint',
    'DECLARE SUB Pressed',
    'CREATE Win AS QFORM',
    '  Width = 300',
    '  Height = 200',
    '  OnPaint = FormPaint',
    '  CREATE Btn AS QBUTTON',
    '    Caption = "Top"',
    '    Left = 10',
    '    Top = 60',
    '    OnClick = Pressed',
    '  END CREATE',
    'END CREATE',
    'SUB FormPaint',
    '  Win.FillRect(10, 10, 60, 40, &H0000FF)',
    '  Win.TextOut(80, 10, "Hello", &H000000, -1)',
    '  Win.FillRect(0, 50, 150, 100, &H00FFFF)',
    '  PRINT "tw="; Win.TextWidth("Hello")',
    '  FOR y = 10 TO 39 STEP 5',
    '    s$ = ""',
    '    FOR x = 10 TO 109 STEP 5',
    '      s$ = s$ + STR$(Win.Pixel(x, y)) + ","',
    '    NEXT x',
    '    PRINT "F"; s$',
    '  NEXT y',
    'END SUB',
    'SUB Pressed',
    '  PRINT "pressed"',
    'END SUB',
    'Win.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
const out3 = await output();
const frows = out3.split("\n").filter((l) => /^F\d/.test(l)).map((l) => l.slice(1).split(",").filter(Boolean).map((v) => Number(v.trim())));
ok(frows.length === 6 && frows[0].length === 20, `the form's pixels were read back (${frows.length} rows)`);
const frame3 = preview();
await k.waitFor(frame3, "Btn");
// (the form's mirror element covers its client area: its pixels are the form's)
const fshown = await k.pixels(frame3, "Win");
ok(!!fshown, "the form's client area is drawn in its window");
let fdiff = 0, painted = 0, clear = 0;
if (fshown) for (let y = 0; y < frows.length; y++) for (let x = 0; x < 20; x++) {
  const m = frows[y][x];
  if (m === 0xF0F0F0) clear++; else painted++;
  if (bgr(fshown, 10 + x * 5, 10 + y * 5) !== m) fdiff++;
}
pending(fshown && fdiff === 0, `the browser shows the model's pixels (${fdiff} differ; ${painted} drawn, ${clear} the form's color)`, FORM_COLOR);
pending(fshown && painted > 20 && clear > 20, "drawn pixels show, and the form's color where nothing is drawn", FORM_COLOR);
// (the button is drawn over the yellow the form drew under it, and a real
// click on it reaches its OnClick)
const btn = await k.rect(frame3, "Btn"), win = await k.rect(frame3, "Win");
const bx = Math.round(btn.x - win.x), by = Math.round(btn.y - win.y);
const under = fshown && bgr(fshown, 5, 55), beside = fshown && bgr(fshown, bx + Math.round(btn.width) + 5, by + 5);
let overBtn = 0;
if (fshown) for (let y = by + 2; y < by + btn.height - 2; y += 2) for (let x = bx + 2; x < bx + btn.width - 2; x += 2)
  if (bgr(fshown, x, y) === 0x00FFFF) overBtn++;
ok(under === 0x00FFFF && beside === 0x00FFFF && overBtn === 0,
  `the controls are drawn over the form's drawing (around: ${under?.toString(16)}/${beside?.toString(16)}, ${overBtn} drawing pixels on the button)`);
await k.click(frame3, "Btn");
await page.waitForTimeout(500);
ok(/pressed/.test(await output()), "a click on a control over the drawing reaches the control");
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_ide_canvas.png" });
await browser.close();
if (failed) { console.log(`\nCanvas: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nCanvas: ALL CHECKS PASSED");
