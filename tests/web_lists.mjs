// QLISTBOX / QCOMBOBOX on the web (the web interpreter, tests/web_run.mjs), from the
// items the desktop runtime draws too (rapidr_value::objects::list): runs
// tests/fixtures/list_items.bas — the program tests/native_gui_events.mjs
// checks natively and interpreted — and checks the same values, the items
// shown, and picking an item with the mouse and typing into the combo box.
// (On the kernel host the lists are drawn on the window's canvas: the test
// reads them through the accessibility mirror — a list box is a `listbox`
// of `option`s, a combo box an `<input role="combobox">` with its options
// beside it — and acts with real clicks and keys, tests/web_kernel_page.mjs.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_lists.mjs

import { readFileSync } from "node:fs";
import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(new URL("./fixtures/list_items.bas", import.meta.url), "utf8");
const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
await r.run(source);
// (the program's windows are the runtime page's own)
const frame = page;
await k.waitFor(frame, "Summary");

// (A label shows "&" as the accelerator mark, as RapidQ does: "a/b & c"
// reads "a/b  c"; the desktop test compares the Caption property itself.)
ok((await k.text(frame, "Summary")) === "5|zero|four|a/b  c|3|Applepear|2|2", `same values as native and interpreted desktop builds (${await k.text(frame, "Summary")})`);
// (a combo box's options: the mirror puts a text field's list beside it, the
// `option` elements right after the `<input>`)
const comboOptions = () => frame.evaluate(() => {
  const out = [];
  for (let e = document.getElementById("rr-combo")?.nextElementSibling; e && e.getAttribute("role") === "option"; e = e.nextElementSibling) {
    out.push({ name: e.getAttribute("aria-label"), selected: e.getAttribute("aria-selected") === "true" });
  }
  return out;
});
const shown = await frame.evaluate(() => {
  const items = document.getElementById("rr-items");
  const opts = [...(items?.querySelectorAll('[role="option"]') ?? [])];
  const combo = document.getElementById("rr-combo");
  const tops = opts.map((o) => o.getBoundingClientRect()).map((r) => [Math.round(r.top), Math.round(r.height)]);
  return {
    items: opts.map((o) => o.getAttribute("aria-label")),
    itemIndex: opts.findIndex((o) => o.getAttribute("aria-selected") === "true"),
    comboText: combo?.value,
    // A combo box (csDropDown, the default) is an edit box listing its items.
    editable: combo?.tagName === "INPUT" && combo.getAttribute("role") === "combobox",
    // (a list box's items are rows one under the other, each with its place)
    list: items?.getAttribute("role") === "listbox" && tops.length > 1 && tops.every(([t, h], i) => h > 0 && (i === 0 || t >= tops[i - 1][0] + tops[i - 1][1])),
  };
});
const combo = await comboOptions();
ok(JSON.stringify(shown.items) === JSON.stringify(["zero", "ONE", "@b not bold", "four", "five"]), `list box items (${JSON.stringify(shown.items)})`);
ok(JSON.stringify(combo.map((o) => o.name)) === JSON.stringify(["red", "a/b & c", "blue"]), `combo box items (${JSON.stringify(combo.map((o) => o.name))})`);
ok(shown.itemIndex === 3 && shown.comboText === "a/b & c" && combo.findIndex((o) => o.selected) === 1, `ItemIndex / Text shown (${shown.itemIndex}, ${shown.comboText}, ${combo.findIndex((o) => o.selected)})`);
ok(shown.editable, "a combo box has an edit box (csDropDown)");
ok(shown.list, "a list box shows rows, not a drop-down");

// Picking "five" with the mouse: a real click on its row.
const five = frame.locator('#rr-items [role="option"][aria-label="five"]');
await five.click({ force: true });
await page.waitForTimeout(300);
ok((await k.text(frame, "Lbl")) === "picked 4 five", `picking an item sets ItemIndex before OnClick (${await k.text(frame, "Lbl")})`);
ok(String(await k.prop(frame, "Items", "itemindex")) === "4" && (await five.getAttribute("aria-selected")) === "true", `the picked item is selected (${await k.prop(frame, "Items", "itemindex")})`);

// Typing into the combo box's edit part (real keys to the focused field;
// OnChange fires on each, the label shows the last).
const retype = async (s) => {
  await k.click(frame, "Combo", [20, 12]);
  await page.waitForTimeout(150);
  const len = (await k.text(frame, "Combo"))?.length ?? 0;
  await page.keyboard.press("End");
  for (let i = 0; i < len; i++) await page.keyboard.press("Backspace");
  await page.keyboard.type(s, { delay: 20 });
  await page.waitForTimeout(300);
};
await retype("blue");
ok((await k.text(frame, "Lbl")) === "combo -1 blue", `typing, even an item's text, chooses no item while the list is closed (Windows' combo box: CBUpdateLBox) (${await k.text(frame, "Lbl")})`);
await retype("violet");
ok((await k.text(frame, "Lbl")) === "combo -1 violet", `other text: ItemIndex -1 (${await k.text(frame, "Lbl")})`);
ok((await k.text(frame, "Combo")) === "violet", `the edit box holds the typed text (${await k.text(frame, "Combo")})`);

ok(r.pageErrors.length === 0, `no page errors (${r.pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_lists.png" });
await browser.close();
if (failed) { console.log(`\nLists: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nLists: ALL CHECKS PASSED");
