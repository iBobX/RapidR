// QLISTBOX / QCOMBOBOX in the IDE preview (the web interpreter), from the
// items the desktop runtime draws too (rapidr_value::objects::list): runs
// tests/fixtures/list_items.bas — the program tests/native_gui_events.mjs
// checks natively and interpreted — and checks the same values, the options
// shown, and picking an item with the mouse.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_lists.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/list_items.bas", import.meta.url), "utf8");
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
// (A label shows "&" as the accelerator mark, as RapidQ does: "a/b & c"
// reads "a/b  c"; the desktop test compares the Caption property itself.)
ok((await text("rr-summary")) === "5|zero|four|a/b  c|3|Applepear|2|2", `same values as native and interpreted desktop builds (${await text("rr-summary")})`);
const shown = await frame.evaluate(() => {
  const opts = (id) => [...(document.getElementById(id)?.options ?? [])].map((o) => o.text);
  const items = document.getElementById("rr-items");
  const combo = document.getElementById("rr-combo");
  return { items: opts("rr-items"), combo: opts("rr-combo"), itemIndex: items?.selectedIndex, comboIndex: combo?.selectedIndex, list: items?.size > 1 };
});
ok(JSON.stringify(shown.items) === JSON.stringify(["zero", "ONE", "@b not bold", "four", "five"]), `list box options (${JSON.stringify(shown.items)})`);
ok(JSON.stringify(shown.combo) === JSON.stringify(["red", "a/b & c", "blue"]), `combo box options (${JSON.stringify(shown.combo)})`);
ok(shown.itemIndex === 3 && shown.comboIndex === 1, `ItemIndex shown selected (${shown.itemIndex}, ${shown.comboIndex})`);
ok(shown.list, "a list box shows rows, not a drop-down");

await frame.selectOption("#rr-items", "4");
await page.waitForTimeout(300);
ok((await text("rr-lbl")) === "picked 4 five", `picking an item sets ItemIndex before OnClick (${await text("rr-lbl")})`);
await frame.selectOption("#rr-combo", "2");
await page.waitForTimeout(300);
ok((await text("rr-lbl")) === "combo 2 blue", `picking a combo item sets ItemIndex / Text before OnChange (${await text("rr-lbl")})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_lists.png" });
await browser.close();
if (failed) { console.log(`\nLists: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nLists: ALL CHECKS PASSED");
