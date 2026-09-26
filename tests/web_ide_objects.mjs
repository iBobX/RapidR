// RapidQ's non-visual objects in the IDE preview (rapidr_value::objects):
// a QBITMAP drawn on a QCANVAS with Canvas.Draw, a QFONT assigned to a
// QLABEL, and a QMEMORYSTREAM — the same code the desktop runtime runs.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_objects.mjs

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
    'DIM Font AS QFONT',
    'Font.Name = "Courier New"',
    'Font.Size = 20',
    'Font.AddStyles(fsBold, fsUnderline, fsStrikeOut)',
    'DIM Bmp AS QBITMAP',
    'Bmp.Width = 40',
    'Bmp.Height = 30',
    'Bmp.FillRect(0, 0, 40, 30, clRed)',
    'Bmp.Circle(5, 5, 26, 26, clBlue, 1)',
    'Bmp.Transparent = 1',
    'Bmp.TransparentColor = clRed',
    'CREATE Form AS QFORM',
    '  Width = 300',
    '  Height = 220',
    '  CREATE Label1 AS QLABEL',
    '    Caption = "Styled"',
    '    Left = 10',
    '    Top = 10',
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
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(/mem=abc3/.test(out), `QMEMORYSTREAM wrote and read back (${JSON.stringify(out.slice(0, 80))})`);

const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const got = await frame.evaluate(() => {
  const label = document.getElementById("rr-label1");
  const canvas = document.getElementById("rr-canvas1");
  if (!label || !canvas) return { missing: true };
  const st = getComputedStyle(label);
  const ctx = canvas.getContext("2d");
  const px = (x, y) => Array.from(ctx.getImageData(x, y, 1, 1).data).slice(0, 3).join(",");
  return {
    font: st.fontFamily, size: st.fontSize, weight: st.fontWeight, deco: st.textDecorationLine,
    center: px(10 + 15, 20 + 15), corner: px(10 + 1, 20 + 1), second: px(60 + 15, 20 + 15), outside: px(5, 5),
  };
});
ok(!got.missing, "label and canvas rendered");
ok(/Courier New/.test(got.font) && got.size === "20px" && Number(got.weight) >= 700,
  `Label.Font = Font applied name, size and bold (${got.font}, ${got.size}, ${got.weight})`);
ok(/underline/.test(got.deco) && /line-through/.test(got.deco), `underline and strike-out together (${got.deco})`);
ok(got.center === "0,0,255", `bitmap's blue circle drawn on the canvas (${got.center})`);
ok(got.corner === "0,255,0", `bitmap's transparent color lets the canvas show through (${got.corner})`);
ok(got.second === "0,0,255", `Canvas.Draw also takes the bitmap itself (${got.second})`);
ok(got.outside === "0,255,0", `canvas drawing untouched outside the bitmap (${got.outside})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_ide_objects.png" });
await browser.close();
if (failed) { console.log(`\nObjects: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nObjects: ALL CHECKS PASSED");
