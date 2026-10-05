// An owner-drawn QLISTBOX in the IDE preview (tests/fixtures/owner_list.bas):
// each item drawn from the shared list model, OnDrawItem's State and Rect, a
// click or the arrow keys selecting (ItemIndex, OnClick, a redraw). (On the
// kernel host the items are pixels in the window's client canvas, at the
// places the accessibility mirror gives its list options.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_owner_list.mjs

import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import * as k from "./web_kernel_page.mjs";

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
await k.waitFor(frame, "Lst");
// (the list's items as the mirror has them — its options — and what the
// window's client canvas shows at each option's place: its size in CSS
// pixels and the colours at two points, as the old per-item canvases had them)
const state = () => frame.evaluate(() => {
  const list = document.getElementById("rr-lst");
  if (!list) return null;
  const canvas = list.closest(".rr-kwin").querySelector("canvas.rr-kclient");
  const c = canvas.getBoundingClientRect(), s = canvas.width / c.width;
  const ctx = canvas.getContext("2d");
  const rows = [...list.querySelectorAll('[role="option"]')];
  const px = (r, x, y) => Array.from(ctx.getImageData(Math.floor((r.left - c.left + x + 0.5) * s), Math.floor((r.top - c.top + y + 0.5) * s), 1, 1).data).slice(0, 3).join(",");
  const rects = rows.map((o) => o.getBoundingClientRect());
  return {
    role: list.getAttribute("role"), names: rows.map((o) => o.getAttribute("aria-label")), size: rects.map((r) => `${r.width}x${r.height}`),
    selected: rows.map((o) => o.getAttribute("aria-selected")),
    bg: rects.map((r) => px(r, 100, 12)), edge: rects.map((r) => px(r, r.width - 2, r.height - 2)), chip: rects.map((r) => px(r, 8, 8)),
    caption: document.getElementById("rr-lbl")?.textContent || "",
  };
});
const output = () => page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
let s = await state();
ok(s && s.role === "listbox" && s.names.join(",") === "Alpha,Beta,Gamma", `an owner-drawn list box shows its items (${s && s.names})`);
ok(s && s.size.every((v) => v === "180x24") && s.edge.join("|") === s.bg.join("|"), `each item is ItemHeight tall and the width less the frame, drawn to its far corner (${s && s.size}; ${s && s.edge.join("|")})`);
ok(s && s.bg.join("|") === "255,255,255|0,255,0|255,255,255", `item 1 (selected, State 0) drawn green, the others white (${s && s.bg.join("|")})`);
ok(s && s.chip.every((v) => v === "255,0,0"), `Draw put the bitmap on every item (${s && s.chip.join("|")})`);
ok(s && /^r1 0:1;1:0;2:1; 0,48,180,72 h24/.test(s.caption), `OnDrawItem got Index, State and Rect (${s && s.caption})`);

// A click on item 2 selects it: OnClick after the model changed, then a redraw.
// (a real mouse click at the item's place)
await frame.locator('#rr-lst [role="option"]').nth(2).click({ force: true });
await page.waitForTimeout(700);
s = await state();
let out = await output();
ok(/click2/.test(out), `a click selects the item before OnClick (${JSON.stringify(out.slice(-20))})`);
ok(s && s.bg.join("|") === "255,255,255|255,255,255|0,255,0" && s.selected.join(",") === "false,false,true", `the list drew again with item 2 selected (${s && s.bg.join("|")}; ${s && s.selected})`);
ok(s && /^r2 0:1;1:1;2:0;/.test(s.caption), `OnDrawItem fired again (${s && s.caption})`);

// Arrow keys (real keys to the list, which the click focused).
await page.keyboard.press("ArrowUp");
await page.waitForTimeout(700);
s = await state();
out = await output();
ok(s && s.bg.join("|") === "255,255,255|0,255,0|255,255,255" && /click1/.test(out), `ArrowUp selects the item above (${s && s.bg.join("|")})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_ide_owner_list.png" });
await browser.close();
if (failed) { console.log(`\nOwner list: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nOwner list: ALL CHECKS PASSED");
