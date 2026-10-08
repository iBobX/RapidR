// RapidQ's non-visual objects on the web (rapidr_value::objects):
// a QBITMAP drawn on a QCANVAS with Canvas.Draw, a QFONT assigned to a
// QLABEL, and a QMEMORYSTREAM — the same code the desktop runtime runs.
// (On the kernel host the label and the canvas are pixels in their window's
// client canvas: the label's font is read through the runtime and seen in
// the pixels its text is drawn with.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_objects.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const pageErrors = r.pageErrors;
await r.run([
    '$INCLUDE "RAPIDQ.INC"',
    'DIM Font AS QFONT',
    'Font.Name = "Courier New"',
    'Font.Size = 20',
    'Font.AddStyles(fsBold, fsUnderline, fsStrikeOut)',
    'DIM Bmp AS QBITMAP',
    'Bmp.Width = 40',
    'Bmp.Height = 30',
    'Bmp.FillRect(0, 0, 40, 30, clRed)',
    'Bmp.Circle(5, 5, 26, 26, clBlue, clBlue)',
    'Bmp.Transparent = 1',
    'Bmp.TransparentColor = clRed',
    'CREATE Form AS QFORM',
    '  Width = 300',
    '  Height = 220',
    '  CREATE Label1 AS QLABEL',
    '    Caption = "Styled"',
    '    Left = 10',
    '    Top = 10',
    '    Height = 40',
    '  END CREATE',
    '  CREATE Canvas1 AS QCANVAS',
    '    Left = 10',
    '    Top = 60',
    '    Width = 100',
    '    Height = 80',
    '  END CREATE',
    'END CREATE',
    'Label1.Font = Font',
    'Canvas1.FillRect(0, 0, 100, 80, clLime)',
    'Canvas1.Draw(10, 20, Bmp.BMP)',
    'Canvas1.Draw(60, 20, Bmp)',
    'DIM Mem AS QMEMORYSTREAM',
    'Mem.WriteStr("abc", 3)',
    'Mem.Position = 0',
    'PRINT "mem="; Mem.ReadStr(3); Mem.Size',
    'Form.ShowModal',
].join("\n"));
await page.waitForTimeout(2500);
const out = await Promise.resolve(r.output());
ok(/mem=abc3/.test(out), `QMEMORYSTREAM wrote and read back (${JSON.stringify(out.slice(0, 80))})`);

const frame = page;
await k.waitFor(frame, "Canvas1");
const label = await k.pixels(frame, "Label1");
ok(!!label && !!(await k.pixels(frame, "Canvas1")), "label and canvas rendered");
const font = {};
for (const p of ["FontName", "FontSize", "FontBold", "FontUnderline", "FontStrikeOut"]) font[p] = await k.prop(frame, "Label1", p);
const on = (v) => v !== "" && Number(v) !== 0;
ok(font.FontName === "Courier New" && font.FontSize === "20" && on(font.FontBold),
  `Label.Font = Font applied name, size and bold (${font.FontName}, ${font.FontSize}, ${font.FontBold})`);
ok(on(font.FontUnderline) && on(font.FontStrikeOut), `underline and strike-out together (${font.FontUnderline}, ${font.FontStrikeOut})`);
// (the label's text as drawn: dark pixels on the form's color. 20 points
// is 27 pixels, as Windows and the desktop make them, so the text spans far
// more rows than the default font's; the strike-out and the underline are
// dark lines across the whole text (the text is wider than the label), one
// through it and one under it)
let inkTop = -1, inkBottom = -1;
const lines = [];
if (label) for (let y = 0; y < label.height; y++) {
  let run = 0, longest = 0, any = false;
  for (let x = 0; x < label.width; x++) {
    const i = (y * label.width + x) * 4;
    const dark = (label.data[i] + label.data[i + 1] + label.data[i + 2]) / 3 < 100;
    run = dark ? run + 1 : 0;
    longest = Math.max(longest, run);
    any ||= dark;
  }
  if (longest >= label.width * 0.9) { if (!lines.length || lines.at(-1).end < y - 1) lines.push({ start: y, end: y }); else lines.at(-1).end = y; }
  else if (any) { if (inkTop < 0) inkTop = y; inkBottom = y; }
}
const tall = label ? (inkBottom - inkTop + 1) / label.scale : 0;
ok(tall >= 18, `the label's text is drawn in the 20-point font (${tall} pixels from top to bottom)`);
ok(lines.length === 2 && lines[0].start > inkTop && lines[1].start > lines[0].end + 4,
  `with the strike-out line through it and the underline under it (lines at ${lines.map((l) => l.start).join(", ")}; text ${inkTop}-${inkBottom})`);
const px = async (x, y) => (await k.pixelAt(frame, "Canvas1", x, y))?.join(",");
const got = { center: await px(10 + 15, 20 + 15), corner: await px(10 + 1, 20 + 1), second: await px(60 + 15, 20 + 15), outside: await px(5, 5) };
ok(got.center === "0,0,255", `bitmap's blue circle drawn on the canvas (${got.center})`);
ok(got.corner === "0,255,0", `bitmap's transparent color lets the canvas show through (${got.corner})`);
ok(got.second === "0,0,255", `Canvas.Draw also takes the bitmap itself (${got.second})`);
ok(got.outside === "0,255,0", `canvas drawing untouched outside the bitmap (${got.outside})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_objects.png" });
await browser.close();
if (failed) { console.log(`\nObjects: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nObjects: ALL CHECKS PASSED");
