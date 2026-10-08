// Components with indexed sub-objects on the web (the web
// interpreter), drawn like the desktop runtime (tests/fixtures/
// statusbar_panels.bas and listview_columns.bas check that one):
//   * QSTATUSBAR: AddPanels, Panel(i).Caption / Width, PanelCount, SimpleText;
//   * QLISTVIEW: AddColumns, AddItems, AddSubItem, InsertItem, Item(i),
//     Column(i), SubItem(i, j), ItemIndex, OnClick, OnColumnClick.
// (On the kernel host both are drawn on the window's canvas: the test reads
// them through the accessibility mirror and the canvas's pixels and clicks
// as the user does, tests/web_kernel_page.mjs.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_components.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const pageErrors = r.pageErrors;
await r.run([
    'DECLARE SUB Clicked',
    'DECLARE SUB Headed (Column AS INTEGER)',
    'CREATE Form AS QFORM',
    '  Width = 440',
    '  Height = 300',
    '  CREATE LV AS QLISTVIEW',
    '    Width = 400',
    '    Height = 150',
    '    ViewStyle = 3',
    '    AddColumns "FileName", "Size", "Method"',
    '    Column(0).Width = 200',
    '    OnClick = Clicked',
    '    OnColumnClick = Headed',
    '  END CREATE',
    '  CREATE Lbl AS QLABEL',
    '    Top = 160',
    '    Width = 400',
    '  END CREATE',
    '  CREATE SB AS QSTATUSBAR',
    '    AddPanels "Ready", "Line 1"',
    '    Panel(0).Width = 150',
    '  END CREATE',
    '  CREATE SB2 AS QSTATUSBAR',
    '    SimplePanel = 1',
    '    SimpleText = "<b>plain</b>"',
    '  END CREATE',
    'END CREATE',
    'SB.AddPanels "INS"',
    'SB.Panel(1).Caption = "Line 42"',
    'PRINT "sb="; SB.Panel(0).Caption; "|"; SB.Panel(1).Caption; "|"; SB.PanelCount',
    'LV.AddItems "readme.txt", "photo", "data.bin"',
    'LV.AddSubItem 0, "1200"',
    'LV.AddSubItem 0, "Stored"',
    'LV.AddSubItem 2, "88"',
    'LV.AddSubItem 2, "Deflated"',
    'LV.Item(1).Caption = "photo.jpg"',
    'LV.AddItems "<b>not bold</b>"',
    'LV.InsertItem 1, "first.txt"',
    'LV.ItemIndex = 2',
    'SUB Clicked',
    '  Lbl.Caption = STR$(LV.ItemIndex) + "|" + LV.Item(LV.ItemIndex).Caption + "|" + LV.SubItem(3, 1) + "|" + STR$(LV.ItemCount) + "|" + STR$(LV.ColumnsCount) + "|" + STR$(LV.Column(0).Width) + "|" + LV.Column(2).Caption',
    'END SUB',
    'SUB Headed (Column AS INTEGER)',
    '  Lbl.Caption = "column" + STR$(Column)',
    'END SUB',
    'Form.ShowModal',
].join("\n"));
await page.waitForTimeout(2500);
const out = await Promise.resolve(r.output());
ok(/sb=Ready\|Line 42\|3/.test(out), `panel properties read back (${JSON.stringify(out.slice(0, 80))})`);

const frame = page;
await k.waitFor(frame, "SB2");
// (a status bar is a `status` element in the mirror, its panels the
// elements in it with their captions as text and their places: a panel's
// Width is the step from its left edge to the next one's)
const got = await frame.evaluate(() => {
  const sb = document.getElementById("rr-sb");
  const sb2 = document.getElementById("rr-sb2");
  if (!sb || !sb2) return { missing: true };
  const panels = [...sb.children];
  const left = (i) => panels[i]?.getBoundingClientRect().left ?? 0;
  return {
    role: sb.getAttribute("role"),
    texts: panels.map((s) => s.textContent),
    first: Math.round(left(1) - left(0)),
    second: Math.round(left(2) - left(1)),
    simple: sb2.textContent,
    simpleBold: !!sb2.querySelector("b"),
  };
});
ok(!got.missing && got.role === "status", `status bars rendered (${got.role})`);
ok(JSON.stringify(got.texts) === JSON.stringify(["Ready", "Line 42", "INS"]), `three panels with their captions (${JSON.stringify(got.texts)})`);
ok(got.first === 150, `Panel(0).Width = 150 (${got.first})`);
ok(got.second === 100, `default panel width 100 (${got.second})`);
ok(got.simple === "<b>plain</b>" && !got.simpleBold, `SimpleText shown as plain text, never markup (${got.simple})`);

// The list view is drawn on the window's canvas from the shared model
// (rapidr_value::objects::listview, the same on the desktop); what it holds
// is the program's to read, and the mirror lists its items.
const lv = await frame.evaluate(() => {
  const el = document.getElementById("rr-lv");
  if (!el) return { missing: true };
  const r = el.getBoundingClientRect();
  const items = [...el.querySelectorAll('[role="option"]')];
  return {
    role: el.getAttribute("role"),
    size: [Math.round(r.width), Math.round(r.height)],
    items: items.map((o) => o.getAttribute("aria-label")),
    selected: items.findIndex((o) => o.getAttribute("aria-selected") === "true"),
  };
});
const get = async (p) => String(await k.prop(frame, "LV", p));
lv.count = await get("itemcount");
lv.columns = await get("columnscount");
lv.index = await get("itemindex");
// (painted: more than one colour drawn at its place — the header, the rows'
// texts, the selected row)
const lvPixels = await k.pixels(frame, "LV");
const lvColours = new Set();
for (let i = 0; lvPixels && i < lvPixels.data.length; i += 4) lvColours.add(`${lvPixels.data[i]},${lvPixels.data[i + 1]},${lvPixels.data[i + 2]}`);
ok(!lv.missing && lv.role === "listbox", `list view rendered (in the mirror: ${lv.role})`);
ok(JSON.stringify(lv.size) === "[400,150]" && lvColours.size > 2, `drawn at its size (${JSON.stringify(lv.size)}, ${lvColours.size} colours)`);
ok(lv.count === "5" && lv.columns === "3", `five items in three columns (${lv.count}, ${lv.columns})`);
ok(JSON.stringify(lv.items) === JSON.stringify(["readme.txt", "first.txt", "photo.jpg", "data.bin", "<b>not bold</b>"]), `its items, the text as it is (${JSON.stringify(lv.items)})`);
ok(lv.index === "2" && lv.selected === 2, `ItemIndex = 2 (${lv.index}; selected in the mirror: ${lv.selected})`);

// Row 3 (the client edge 2, the header 17 px high, rows 14 — Windows' classic
// list view, 2 pixels under the header: its middle at y = 2 + 17 + 2 + 3 * 14 + 5),
// a real click there.
await k.click(frame, "LV", [20, 68]);
await page.waitForTimeout(300);
const clicked = await k.text(frame, "Lbl");
ok(clicked === "3|data.bin|Deflated|5|3|200|Method", `clicking a row sets ItemIndex and fires OnClick (${clicked})`);
await k.click(frame, "LV", [230, 10]);
await page.waitForTimeout(300);
const headed = await k.text(frame, "Lbl");
ok(headed === "column1", `clicking a header fires OnColumnClick(1) (${headed})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_components.png" });
await browser.close();
if (failed) { console.log(`\nComponents: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nComponents: ALL CHECKS PASSED");
