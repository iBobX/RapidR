// QIMAGE on the web (the web interpreter), from the picture the
// desktop runtime shows too (rapidr_value::objects, a Bitmap): drawing on an
// image without a picture, Pixel, AutoSize, Stretch, and RapidQ's mouse
// events — OnMouseDown (Button, X, Y, Shift), OnClick — with MOUSEX / MOUSEY
// relative to the form's client area. ($RESOURCE needs a native or
// `--interp` build: tests/native_gui_events.mjs checks it.) (The form is
// shown with ShowModal: the IDE shows its own start form, Form1, modally
// after the program's code, and a modal form takes the mouse from the
// others, as in RapidQ — the kernel draws it over a form merely Shown.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_picture.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

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
Form.ShowModal

SUB Down (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Lbl.Caption = "down" + STR$(Button) + "," + STR$(X) + "," + STR$(Y) + ";"
END SUB

SUB Clicked
  Lbl.Caption = Lbl.Caption + "click" + STR$(MOUSEX) + "," + STR$(MOUSEY)
END SUB
`;

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const pageErrors = r.pageErrors;
await r.run(source);
await page.waitForTimeout(2500);
const frame = page;
await k.waitFor(frame, "Summary");
const text = (name) => k.text(frame, name);

ok((await text("Summary")) === "40|000000FF|0000FF00|00FFFFFF", `same values as the desktop builds (HEX$ gives 8 digits, as RC.EXE) (${await text("Summary")})`);
// The picture is shown: its pixels, at the image's place in the window
// (the kernel draws it in the window's client canvas). The picture spans
// the 40x30 image: its red line runs from one side to the other, its white
// reaches the far corner.
const place = await k.rect(frame, "Pad");
const px = async (x, y) => (await k.pixelAt(frame, "Pad", x, y))?.join(",");
const shown = { size: place && `${Math.round(place.width)}x${Math.round(place.height)}`, ends: [await px(0, 5), await px(39, 5), await px(39, 29)], line: await px(10, 5), fill: await px(20, 19) };
ok(shown.size === "40x30" && shown.ends.join(" ") === "255,0,0 255,0,0 255,255,255", `the image shows the picture (${shown.size}; ${shown.ends.join(" ")})`);
ok(shown.line === "255,0,0" && shown.fill === "0,255,0", `with its pixels (${shown.line} / ${shown.fill})`);

// Click at (5, 7) in the image: OnMouseDown (Button, X, Y, Shift), then OnClick
// with MOUSEX / MOUSEY in the form's client area (the image is at 20, 10).
// (a real mouse click there, through the kernel)
await k.click(frame, "Pad", [5, 7]);
await page.waitForTimeout(300);
ok((await text("Lbl")) === "down0,5,7;click25,17", `mouse events in RapidQ's order (${await text("Lbl")})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_picture.png" });
await browser.close();
if (failed) { console.log(`\nPicture: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nPicture: ALL CHECKS PASSED");
