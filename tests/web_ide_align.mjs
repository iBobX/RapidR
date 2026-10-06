// Align in the IDE preview (the web interpreter), laid out as on the desktop
// (rapidr_value::layout): runs tests/fixtures/align_layout.bas — the program
// tests/native_gui_events.mjs checks natively and interpreted — and checks
// the same numbers, the elements' places, dragging the splitter (OnMoved)
// and maximizing the form (OnResize).
// (On the kernel host the form is a window of canvases with the kernel's
// accessibility mirror over them: places are the mirror elements', the
// splitter and the maximize button get real mouse input on the canvases,
// tests/web_kernel_page.mjs.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_align.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/align_layout.bas", import.meta.url), "utf8");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
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

await k.waitFor(frame, "Loose");

const text = (name) => k.text(frame, name);
// (the window's element, holding its frame and client canvases)
const win = () => frame.locator(".rr-kwin").first();
// (component `name`'s place in the form's client area: its mirror element's
// rectangle from the client canvas's top left corner, [left, top, width,
// height] — what offsetLeft / … gave on the DOM host)
const rect = (name) => frame.evaluate((n) => {
  const el = document.getElementById("rr-" + n.toLowerCase());
  const client = el?.closest(".rr-kwin")?.querySelector("canvas.rr-kclient");
  if (!el || !client) return null;
  const r = el.getBoundingClientRect(), c = client.getBoundingClientRect();
  return [r.left - c.left, r.top - c.top, r.width, r.height].map(Math.round);
}, name);

// Width / Height are the whole form: the client area of a 400 × 300 form is
// 398 × 269 (1px border, 29px caption), as on the desktop — the numbers are
// the ones tests/native_gui_events.mjs expects there.
ok((await text("Loose")) === "103,40,235,210|100|250", `laid out before the form is shown (${await text("Loose")})`);
const places = {
  bar: await rect("Bar"), status: await rect("Status"), split: await rect("Split"),
  tree: await rect("Tree"), side: await rect("Side"), memo: await rect("Memo"), loose: await rect("Loose"),
};
ok(JSON.stringify(places.bar) === "[0,0,398,40]", `alTop panel (${JSON.stringify(places.bar)})`);
ok(JSON.stringify(places.status) === "[0,250,398,19]", `status bar docked at the bottom (${JSON.stringify(places.status)})`);
ok(JSON.stringify(places.tree) === "[0,40,100,210]", `alLeft list (${JSON.stringify(places.tree)})`);
ok(JSON.stringify(places.split) === "[100,40,3,210]", `splitter after it (alLeft by default) (${JSON.stringify(places.split)})`);
ok(JSON.stringify(places.side) === "[338,40,60,210]", `alRight panel (${JSON.stringify(places.side)})`);
ok(JSON.stringify(places.memo) === "[103,40,235,210]", `alClient fills the rest (${JSON.stringify(places.memo)})`);
ok(places.loose && places.loose[0] === 150 && places.loose[1] === 60, `a control without Align stays put (${JSON.stringify(places.loose)})`);

// (the window's element is the whole form, frame included; the client
// canvas its inside)
const sizes0 = await frame.evaluate(() => {
  const w = document.querySelector(".rr-kwin");
  const c = w?.querySelector("canvas.rr-kclient")?.getBoundingClientRect();
  return { outer: w ? [w.offsetWidth, w.offsetHeight] : null, client: c ? [Math.round(c.width), Math.round(c.height)] : null };
});
ok(sizes0.outer && sizes0.outer[0] === 400 && sizes0.outer[1] === 300, `the form is Width × Height, frame included (${JSON.stringify(sizes0.outer)})`);
ok(JSON.stringify(sizes0.client) === "[398,269]", `its client area is ClientWidth × ClientHeight (${JSON.stringify(sizes0.client)})`);

// Dragging the splitter 60px right widens the list next to it (MinSize
// permitting) and fires OnMoved.
const split = await frame.locator("#rr-split").boundingBox();
await page.mouse.move(split.x + split.width / 2, split.y + split.height / 2);
await page.mouse.down();
await page.mouse.move(split.x + split.width / 2 + 30, split.y + split.height / 2);
await page.mouse.move(split.x + split.width / 2 + 60, split.y + split.height / 2);
await page.mouse.up();
await page.waitForTimeout(300);
ok((await text("Side"))?.startsWith("moved160|160|163"), `dragging the splitter resizes the list and fires OnMoved (${await text("Side")})`);
const memoDragged = await rect("Memo");
ok(JSON.stringify(memoDragged) === "[163,40,175,210]", `alClient follows the splitter (${JSON.stringify(memoDragged)})`);

await k.click(frame, "Btn");
await page.waitForTimeout(300);
ok((await text("Status"))?.includes("235|210|5|398"), `hiding the alRight panel widens alClient (${await text("Status")})`);
const memoAfter = await rect("Memo");
ok(JSON.stringify(memoAfter) === "[163,40,235,210]", `memo moved (${JSON.stringify(memoAfter)})`);

// Maximize: the form fills the preview; OnResize reports the new layout.
// (a real click on the title bar's maximize button, which the kernel draws
// on the frame canvas: the second button from the right, 28px apart inside
// the 1px border — its middle 44px left of the window's right edge, 15px
// down; crates/rapidr-ui-host-web/src/frame.rs)
const box = await win().boundingBox();
await page.mouse.click(box.x + box.width - 44, box.y + 15);
await page.waitForTimeout(400);
const sizes = await frame.evaluate(() => [innerWidth, innerHeight]);
const resized = await text("Bar");
const expected = `${sizes[0]}x${sizes[1]}|${sizes[0] - 2 - 163}x${sizes[1] - 31 - 40 - 19}|`;
ok(resized?.startsWith(expected), `maximize lays out again and fires OnResize (${resized}; expected ${expected}…)`);
const maxBox = await win().boundingBox();
const frameBox = await (await frame.frameElement()).boundingBox();
const placed = maxBox && [maxBox.x - frameBox.x, maxBox.y - frameBox.y, maxBox.width, maxBox.height].map(Math.round);
ok(JSON.stringify(placed) === JSON.stringify([0, 0, sizes[0], sizes[1]]), `the window fills the preview (${JSON.stringify(placed)})`);
const memoMax = await rect("Memo");
ok(memoMax && memoMax[2] === sizes[0] - 2 - 163 && memoMax[3] === sizes[1] - 31 - 40 - 19, `memo fills the maximized form (${JSON.stringify(memoMax)})`);

// (the panel's Caption is its text in the mirror, the button still in it)
const panel = { button: await frame.evaluate(() => !!document.querySelector("#rr-bar #rr-btn")), caption: await text("Bar") };
ok(panel.button && panel.caption?.startsWith(expected), `a panel's Caption doesn't remove its children (${JSON.stringify(panel)})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_align.png" });
await browser.close();
if (failed) { console.log(`\nAlign: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nAlign: ALL CHECKS PASSED");
