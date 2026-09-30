// Components with indexed sub-objects in the IDE preview (the web
// interpreter), drawn like the desktop runtime (tests/fixtures/
// statusbar_panels.bas and listview_columns.bas check that one):
//   * QSTATUSBAR: AddPanels, Panel(i).Caption / Width, PanelCount, SimpleText;
//   * QLISTVIEW: AddColumns, AddItems, AddSubItem, InsertItem, Item(i),
//     Column(i), SubItem(i, j), ItemIndex, OnClick, OnColumnClick.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_components.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate(() => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
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
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
ok(/sb=Ready\|Line 42\|3/.test(out), `panel properties read back (${JSON.stringify(out.slice(0, 80))})`);

const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const got = await frame.evaluate(() => {
  const sb = document.getElementById("rr-sb");
  const sb2 = document.getElementById("rr-sb2");
  if (!sb || !sb2) return { missing: true };
  const spans = [...sb.querySelectorAll("span")];
  return {
    texts: spans.map((s) => s.textContent),
    first: Math.round(spans[0]?.getBoundingClientRect().width || 0),
    second: Math.round(spans[1]?.getBoundingClientRect().width || 0),
    simple: sb2.textContent,
    simpleBold: !!sb2.querySelector("b"),
  };
});
ok(!got.missing, "status bars rendered");
ok(JSON.stringify(got.texts) === JSON.stringify(["Ready", "Line 42", "INS"]), `three panels with their captions (${JSON.stringify(got.texts)})`);
ok(got.first === 150, `Panel(0).Width = 150 (${got.first})`);
ok(got.second === 100, `default panel width 100 (${got.second})`);
ok(got.simple === "<b>plain</b>" && !got.simpleBold, `SimpleText shown as plain text, never markup (${got.simple})`);

// The list view is a canvas the shared model paints (rapidr_value::objects::
// listview, the same on the desktop); what it holds is the program's to read.
const lv = await frame.evaluate(() => {
  const canvas = document.getElementById("rr-lv");
  if (!canvas || canvas.tagName !== "CANVAS") return { missing: true };
  const rt = window.__rapidr_rt;
  const get = (p) => rt.rapidr_get_prop("LV", p);
  const r = canvas.getBoundingClientRect();
  return { size: [Math.round(r.width), Math.round(r.height)], count: get("itemcount"), columns: get("columnscount"), index: get("itemindex"), painted: canvas.width > 0 };
});
ok(!lv.missing, "list view rendered (a canvas)");
ok(JSON.stringify(lv.size) === "[400,150]" && lv.painted, `drawn at its size (${JSON.stringify(lv.size)})`);
ok(String(lv.count) === "5" && String(lv.columns) === "3", `five items in three columns (${lv.count}, ${lv.columns})`);
ok(String(lv.index) === "2", `ItemIndex = 2 (${lv.index})`);

// Row 3 (the header 21 px high, rows 18: its middle at y = 1 + 21 + 3 * 18 + 9).
const box = await frame.locator("#rr-lv").boundingBox();
await page.mouse.click(box.x + 20, box.y + 85);
await page.waitForTimeout(300);
const clicked = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(clicked === "3|data.bin|Deflated|5|3|200|Method", `clicking a row sets ItemIndex and fires OnClick (${clicked})`);
await page.mouse.click(box.x + 230, box.y + 10);
await page.waitForTimeout(300);
const headed = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(headed === "column1", `clicking a header fires OnColumnClick(1) (${headed})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await page.screenshot({ path: "scratch/web_ide_components.png" });
await browser.close();
if (failed) { console.log(`\nComponents: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nComponents: ALL CHECKS PASSED");
