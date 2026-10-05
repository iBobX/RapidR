// The web-only components on the UI kernel's page (docs/web-host-plan.md
// §3.6, Stage W6): tests/fixtures/web_overlays.bas' RWEBVIEW, RDOM
// elements, RWEBVIDEO and RPLOT are the page's own elements, placed by the
// kernel over the form's canvas at their components' places, clipped to
// their parents, hidden with them; they take their own clicks; an open
// drop-down list is drawn on a layer above them and still picks.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo
// served on http://localhost:8765):  node tests/web_overlays.mjs

import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: Number(process.env.RAPIDR_DPR || 1) });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto(`${URL_BASE}/${process.env.RAPIDR_KERNEL_PAGE || "tests/web_kernel.html"}`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
const source = readFileSync(join(HERE, "fixtures/web_overlays.bas"), "utf8");
await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "web_overlays", {})), source);
await k.waitFor(page, "Lbl");
await page.waitForTimeout(300);

/// An element's place relative to its window's client canvas, its clip
/// and whether it shows.
const placed = (name) => page.evaluate((n) => {
  const el = document.getElementById("rr-" + n.toLowerCase());
  if (!el) return null;
  const win = el.closest(".rr-kwin");
  const client = win?.querySelector("canvas.rr-kclient")?.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  return {
    tag: el.tagName.toLowerCase(), form: win?.dataset.rrForm ?? null, inOverlays: !!el.closest(".rr-koverlays"),
    x: client ? Math.round(r.left - client.left) : null, y: client ? Math.round(r.top - client.top) : null,
    w: Math.round(r.width), h: Math.round(r.height), shown: getComputedStyle(el).display !== "none", clip: el.style.clipPath,
  };
}, name);

// ---- RWEBVIEW: an iframe at its place, its page there ----
let web = await placed("Web");
ok(web?.tag === "iframe" && web.form === "form" && web.inOverlays, `the web view is an iframe over the form's canvas (${JSON.stringify(web)})`);
ok(web?.x === 10 && web?.y === 34 && web?.w === 240 && web?.h === 120, `at its Left / Top / Width / Height (${web?.x},${web?.y} ${web?.w}x${web?.h})`);
const frameText = await page.evaluate(() => document.getElementById("rr-web")?.contentDocument?.getElementById("p")?.textContent ?? null);
ok(frameText === "hello from the frame", `SetHtml's page shows in it (${frameText})`);
ok(await k.prop(page, "Web", "Html") === "<html><body><p id='p'>hello from the frame</p></body></html>", "Html reads the page it was given");

// ---- RDOM in a panel: clipped by the panel, its own clicks ----
const inner = await placed("Inner");
ok(inner?.x === 270 && inner?.y === 44 && inner.shown, `an RDOM in a panel is placed in it (${inner?.x},${inner?.y})`);
ok(/inset\(0px 90px 0px 0px\)/.test(inner?.clip ?? ""), `and clipped to the panel's right edge (${inner?.clip})`);
await page.locator("#rr-inner").click({ position: { x: 20, y: 10 } });
await page.waitForTimeout(200);
ok(await k.text(page, "Lbl") === "dom clicked inside", `its OnClick runs on a real click (${await k.text(page, "Lbl")})`);
const free = await page.evaluate(() => { const e = document.getElementById("rr-free"); return e ? { parent: e.parentElement.tagName, text: e.textContent } : null; });
ok(free?.parent === "BODY" && free.text === "free element", `an RDOM without a parent is the page's (${JSON.stringify(free)})`);

// ---- RWEBVIDEO, RPLOT ----
const video = await placed("Video");
ok(video?.tag === "video" && video.x === 10 && video.y === 170 && video.shown, `the video is a <video> at its place (${JSON.stringify(video)})`);
const plotDrawn = await page.evaluate(() => {
  const c = document.querySelector("#rr-plot1 canvas");
  if (!c || !c.width) return 0;
  const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
  let n = 0;
  for (let i = 0; i < d.length; i += 4) if (d[i] > 200 && d[i + 1] < 80 && d[i + 2] < 80) n++;
  return n;
});
ok(plotDrawn > 20, `the plot's red line is drawn in its element (${plotDrawn} red pixels)`);

// ---- the program moves and hides them ----
await k.click(page, "Move");
await page.waitForTimeout(300);
web = await placed("Web");
ok(web?.x === 40 && web?.y === 39, `Left / Top set by the program move it (${web?.x},${web?.y})`);
ok((await placed("Video"))?.shown === false, "Visible = False hides it");

// ---- a drop-down list over the web view, on the layer above ----
const before = await page.evaluate(() => getComputedStyle(document.querySelector('.rr-kwin[data-rr-form="form"] canvas.rr-kpopups')).display);
await k.click(page, "Combo");
await page.waitForTimeout(300);
const layer = await page.evaluate(() => {
  const c = document.querySelector('.rr-kwin[data-rr-form="form"] canvas.rr-kpopups');
  const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
  let opaque = 0;
  for (let i = 3; i < d.length; i += 4) if (d[i] === 255) opaque++;
  return { display: getComputedStyle(c).display, opaque };
});
ok(before === "none" && layer.display === "block" && layer.opaque > 500, `the open list is drawn on the layer above the elements (${before} → ${layer.display}, ${layer.opaque} pixels)`);
// (the list's fourth item: under the combo box, over the web view)
const combo = await k.rect(page, "Combo");
await page.mouse.click(combo.x + 30, combo.y + combo.height + 3 * 16 + 8);
await page.waitForTimeout(300);
ok(Number(await k.prop(page, "Combo", "ItemIndex")) === 3, `a click on it picks the item (${await k.prop(page, "Combo", "ItemIndex")})`);
ok(await page.evaluate(() => getComputedStyle(document.querySelector('.rr-kwin[data-rr-form="form"] canvas.rr-kpopups')).display) === "none", "closed, the layer goes");

ok(errors.length === 0, `no page errors (${errors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb overlays: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb overlays: ALL CHECKS PASSED");
