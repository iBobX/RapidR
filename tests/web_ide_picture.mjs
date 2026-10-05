// QIMAGE in the IDE preview (the web interpreter), from the picture the
// desktop runtime shows too (rapidr_value::objects, a Bitmap): drawing on an
// image without a picture, Pixel, AutoSize, Stretch, and RapidQ's mouse
// events — OnMouseDown (Button, X, Y, Shift), OnClick — with MOUSEX / MOUSEY
// relative to the form's client area. ($RESOURCE needs a native or
// `--interp` build: tests/native_gui_events.mjs checks it.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_picture.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = `
DECLARE SUB Down (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "Picture": Width = 300: Height = 200
  CREATE Pad AS QIMAGE
    Left = 20: Top = 10: Width = 40: Height = 30
    OnMouseDown = Down
    OnClick = Clicked
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 120: Width = 280
  END CREATE
  CREATE Summary AS QLABEL
    Top = 145: Width = 280
  END CREATE
END CREATE
Pad.FillRect(0, 0, 40, 30, &HFFFFFF)
Pad.Line(0, 5, 39, 5, &HFF)
Pad.Circle(10, 10, 30, 28, &HFF0000, &H00FF00)
Summary.Caption = STR$(Pad.Width) + "|" + HEX$(Pad.Pixel(10, 5)) + "|" + HEX$(Pad.Pixel(20, 19)) + "|" + HEX$(Pad.Pixel(35, 25))
Form.Show

SUB Down (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Lbl.Caption = "down" + STR$(Button) + "," + STR$(X) + "," + STR$(Y) + ";"
END SUB

SUB Clicked
  Lbl.Caption = Lbl.Caption + "click" + STR$(MOUSEX) + "," + STR$(MOUSEY)
END SUB
`;

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

ok((await text("rr-summary")) === "40|000000FF|0000FF00|00FFFFFF", `same values as the desktop builds (HEX$ gives 8 digits, as RC.EXE) (${await text("rr-summary")})`);
// The picture is shown: its pixels, in the element.
const shown = await frame.evaluate(() => {
  const img = document.getElementById("rr-pad");
  if (!img || !img.complete || !img.naturalWidth) return null;
  const c = document.createElement("canvas");
  c.width = img.naturalWidth; c.height = img.naturalHeight;
  const ctx = c.getContext("2d");
  ctx.drawImage(img, 0, 0);
  const px = (x, y) => Array.from(ctx.getImageData(x, y, 1, 1).data.slice(0, 3)).join(",");
  return { size: `${img.naturalWidth}x${img.naturalHeight}`, line: px(10, 5), fill: px(20, 19) };
});
ok(shown?.size === "40x30", `the element shows the picture (${shown?.size})`);
ok(shown?.line === "255,0,0" && shown?.fill === "0,255,0", `with its pixels (${shown?.line} / ${shown?.fill})`);

// Click at (5, 7) in the image: OnMouseDown (Button, X, Y, Shift), then OnClick
// with MOUSEX / MOUSEY in the form's client area (the image is at 20, 10).
const box = await frame.locator("#rr-pad").boundingBox();
await page.mouse.click(box.x + 5, box.y + 7);
await page.waitForTimeout(300);
ok((await text("rr-lbl")) === "down0,5,7;click25,17", `mouse events in RapidQ's order (${await text("rr-lbl")})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_picture.png" });
await browser.close();
if (failed) { console.log(`\nPicture: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nPicture: ALL CHECKS PASSED");
