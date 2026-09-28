// An owner-drawn QLISTBOX in the IDE preview (tests/fixtures/owner_list.bas):
// one canvas per item from the shared list model, OnDrawItem's State and
// Rect, a click or the arrow keys selecting (ItemIndex, OnClick, a redraw).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_owner_list.mjs

import { chromium } from "playwright";
import { readFileSync } from "node:fs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/owner_list.bas", import.meta.url), "utf8")
  .replace("SUB Clicked\nEND SUB", 'SUB Clicked\n    PRINT "click"; Lst.ItemIndex\nEND SUB');

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
const state = () => frame.evaluate(() => {
  const list = document.getElementById("rr-lst");
  if (!list) return null;
  const rows = [...list.querySelectorAll("[data-item] canvas")];
  const px = (c, x, y) => Array.from(c.getContext("2d").getImageData(x, y, 1, 1).data).slice(0, 3).join(",");
  return {
    tag: list.tagName, count: rows.length, size: rows.map((c) => `${c.width}x${c.height}`),
    bg: rows.map((c) => px(c, 100, 12)), chip: rows.map((c) => px(c, 8, 8)),
    caption: document.getElementById("rr-lbl")?.textContent || "",
  };
});
let s = await state();
ok(s && s.tag === "DIV" && s.count === 3, `an owner-drawn list box shows a canvas per item (${s && s.count})`);
ok(s && s.size.every((v) => v === "180x24"), `each item is ItemHeight tall and the width less the frame (${s && s.size})`);
ok(s && s.bg.join("|") === "255,255,255|0,255,0|255,255,255", `item 1 (selected, State 0) drawn green, the others white (${s && s.bg.join("|")})`);
ok(s && s.chip.every((v) => v === "255,0,0"), `Draw put the bitmap on every item (${s && s.chip.join("|")})`);
ok(s && /^r1 0:1;1:0;2:1; 0,48,180,72 h24/.test(s.caption), `OnDrawItem got Index, State and Rect (${s && s.caption})`);

// A click on item 2 selects it: OnClick after the model changed, then a redraw.
await frame.evaluate(() => document.querySelector('#rr-lst [data-item="2"] canvas').dispatchEvent(new MouseEvent("click", { bubbles: true })));
await page.waitForTimeout(700);
s = await state();
let out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(/click2/.test(out), `a click selects the item before OnClick (${JSON.stringify(out.slice(-20))})`);
ok(s && s.bg.join("|") === "255,255,255|255,255,255|0,255,0", `the list drew again with item 2 selected (${s && s.bg.join("|")})`);
ok(s && /^r2 0:1;1:1;2:0;/.test(s.caption), `OnDrawItem fired again (${s && s.caption})`);

// Arrow keys.
await frame.evaluate(() => document.getElementById("rr-lst").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
await page.waitForTimeout(700);
s = await state();
out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(s && s.bg.join("|") === "255,255,255|0,255,0|255,255,255" && /click1/.test(out), `ArrowUp selects the item above (${s && s.bg.join("|")})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_ide_owner_list.png" });
await browser.close();
if (failed) { console.log(`\nOwner list: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nOwner list: ALL CHECKS PASSED");
