// The web's own components without a window (crates/rapidr-runtime-web's
// webapi_web.rs) on the UI kernel's page: RJAVASCRIPT's Eval / Call,
// RWEBSTORAGE's Set / Get / Keys / Remove, RWEBNOTIFICATION's Show (its
// own, not a form's), RROUTER's Navigate and Route; and QIMAGE's
// LoadFromPlot (a chart's pixels as the picture, as on the desktop).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo
// served on http://localhost:8765):  node tests/web_webapi.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = `
CREATE Form AS QFORM
  Caption = "web api"
  Width = 420
  Height = 300
  CREATE Lbl AS QLABEL
    Left = 8: Top = 8: Width = 400
  END CREATE
  CREATE Img AS QIMAGE
    Left = 8: Top = 40: Width = 10: Height = 10
    AutoSize = 1
  END CREATE
END CREATE
CREATE JS AS RJAVASCRIPT
END CREATE
CREATE Store AS RWEBSTORAGE
  StorageType = "session"
END CREATE
CREATE Note AS RWEBNOTIFICATION
  Title = "Hello"
  Body = "from RapidR"
END CREATE
CREATE Router AS RROUTER
END CREATE
CREATE Plot1 AS RPLOT
  Width = 200
  Height = 120
END CREATE

DIM s AS STRING
s = STR$(JS.Eval("6 * 7")) + " " + JS.Call("twice", 21)
Store.Set("k", "v1")
s = s + " " + Store.Get("k") + " " + STR$(Store.HasKey("k"))
Store.Remove("k")
s = s + " " + STR$(Store.HasKey("k"))
Note.Show
Router.Navigate("page2")
s = s + " " + Router.Route
DIM xs(4) AS DOUBLE
DIM ys(4) AS DOUBLE
FOR i = 0 TO 4
  xs(i) = i
  ys(i) = i * i
NEXT
Plot1.plot(xs, ys, "squares", "red")
Img.LoadFromPlot("Plot1")
s = s + " " + STR$(Img.Width) + "x" + STR$(Img.Height)
Lbl.Caption = s
Form.ShowModal
`;

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto(`${URL_BASE}/${process.env.RAPIDR_KERNEL_PAGE || "tests/web_kernel.html"}`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
// (the page's side: a function RJAVASCRIPT calls, and notifications
// recorded instead of shown)
await page.evaluate(() => {
  window.twice = (n) => n * 2;
  window.__notes = [];
  window.Notification = class { constructor(title, options) { window.__notes.push([title, options?.body]); } static requestPermission() { return Promise.resolve("granted"); } };
});
await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "webapi", {})), SOURCE);
await k.waitFor(page, "Lbl");
await page.waitForTimeout(300);

const parts = (await k.text(page, "Lbl")).trim().split(/\s+/);
ok(parts[0] === "42" && parts[1] === "42", `RJAVASCRIPT: Eval and Call answer (${parts.slice(0, 2)})`);
ok(parts[2] === "v1" && Number(parts[3]) !== 0 && Number(parts[4]) === 0, `RWEBSTORAGE: Set, Get, HasKey, Remove (${parts.slice(2, 5)})`);
ok(JSON.stringify(await page.evaluate(() => window.__notes)) === JSON.stringify([["Hello", "from RapidR"]]), "RWEBNOTIFICATION.Show shows its notification");
ok(parts[5] === "page2" && (await page.evaluate(() => location.hash)) === "#page2", `RROUTER: Navigate sets the address, Route reads it (${parts[5]})`);
const [w, h] = (parts[6] || "").split("x").map(Number);
ok(w > 50 && h > 50, `QIMAGE.LoadFromPlot: the chart is the picture, AutoSize takes its size (${parts[6]})`);
const shown = await k.pixels(page, "Img");
let red = 0;
if (shown) for (let i = 0; i < shown.data.length; i += 4) if (shown.data[i] > 200 && shown.data[i + 1] < 80 && shown.data[i + 2] < 80) red++;
ok(red > 20, `and the window shows its red line (${red} red pixels)`);
ok(errors.length === 0, `no page errors (${errors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb API components: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb API components: ALL CHECKS PASSED");
