// QSTRINGGRID in the IDE preview (the web interpreter), drawn from the data
// the desktop runtime draws too (rapidr_value::objects::grid; the desktop
// side is checked by tests/fixtures/string_grid.bas):
//   * Cell(col, row) read/write, ColCount / RowCount, FixedRows / FixedCols,
//     ColWidths(i), InsertRow, SwapRows, Row / Col;
//   * clicking selects a cell (OnSelectCell, OnClick); fixed cells can't be
//     selected; arrow keys move the selection;
//   * goEditing: double-click edits in place, Enter stores (OnSetEditText);
//   * an ellipsis column (ColumnStyle = gcsEllipsis) fires OnEllipsisClick;
//   * RapidR's AddRow / SetCell / GetCell / Clear.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_grid.mjs

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
    'DECLARE SUB Sel (Col AS INTEGER, Row AS INTEGER, CanSelect AS INTEGER)',
    'DECLARE SUB Edited (Col AS INTEGER, Row AS INTEGER, Value AS STRING)',
    'DECLARE SUB Ellipsis (Col AS INTEGER, Row AS INTEGER)',
    'CREATE Form AS QFORM',
    '  Width = 480',
    '  Height = 320',
    '  CREATE Grid AS QSTRINGGRID',
    '    Width = 440',
    '    Height = 170',
    '    ColCount = 4',
    '    RowCount = 5',
    '    Cell(1, 0) = "Name"',
    '    Cell(2, 0) = "Age"',
    '    Cell(3, 0) = "More"',
    '    ColWidths(0) = 30',
    '    ColumnStyle(3) = 1',
    '    AddOptions 10',
    '    OnSelectCell = Sel',
    '    OnSetEditText = Edited',
    '    OnEllipsisClick = Ellipsis',
    '  END CREATE',
    '  CREATE Lbl AS QLABEL',
    '    Top = 180',
    '    Width = 440',
    '  END CREATE',
    '  CREATE Ide AS QSTRINGGRID',
    '    Top = 210',
    '    Width = 300',
    '    Height = 70',
    '    RowCount = 0',
    '    ColCount = 2',
    '  END CREATE',
    'END CREATE',
    'DIM i AS INTEGER',
    'FOR i = 1 TO 4',
    '  Grid.Cell(0, i) = STR$(i)',
    '  Grid.Cell(1, i) = "P" + STR$(i)',
    '  Grid.Cell(2, i) = STR$(20 + i)',
    'NEXT',
    'Grid.InsertRow 2',
    'Grid.SwapRows 1, 3',
    'Grid.Cell(1, 5) = "<b>plain</b>"',
    'PRINT "grid="; Grid.RowCount; "|"; Grid.ColCount; "|"; Grid.Cell(1, 3); "|"; Grid.Cell(1, 1); "|"; Grid.ColWidths(0); "|"; Grid.FixedRows',
    'Ide.AddRow "Event", "Handler"',
    'Ide.AddRow "OnClick", "Go", "..."',
    'Ide.SetCell 1, 1, "Run"',
    'PRINT "ide="; Ide.RowCount; "|"; Ide.ColCount; "|"; Ide.GetCell(1, 1); "|"; Ide.Cell(2, 1)',
    'SUB Sel (Col AS INTEGER, Row AS INTEGER, CanSelect AS INTEGER)',
    '  Lbl.Caption = "sel" + STR$(Col) + "," + STR$(Row) + "|" + STR$(Grid.Col) + "," + STR$(Grid.Row)',
    'END SUB',
    'SUB Edited (Col AS INTEGER, Row AS INTEGER, Value AS STRING)',
    '  Lbl.Caption = "edit" + STR$(Col) + "," + STR$(Row) + "=" + Value + "|" + Grid.Cell(Col, Row)',
    'END SUB',
    'SUB Ellipsis (Col AS INTEGER, Row AS INTEGER)',
    '  Lbl.Caption = "ellipsis" + STR$(Col) + "," + STR$(Row)',
    'END SUB',
    'Form.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2500);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
const printed = out.slice(out.lastIndexOf("grid="));
ok(/grid=6\|4\|P1\|P2\|30\|1/.test(printed), `cells, sizes, InsertRow and SwapRows read back (${JSON.stringify(printed.slice(0, 60))})`);
ok(/ide=2\|3\|Run\|\.\.\./.test(printed), `RapidR's AddRow / SetCell / GetCell (${JSON.stringify(printed.slice(0, 120))})`);

const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const cells = () => frame.evaluate(() => {
  const table = document.getElementById("rr-grid-table");
  if (!table) return null;
  return [...table.querySelectorAll("tr")].map((tr) => [...tr.children].map((td) => td.querySelector(".rr-grid-text")?.textContent ?? ""));
});
const grid = await cells();
ok(!!grid, "grid rendered");
ok(grid && grid.length === 6 && grid[0].length === 4, `6 rows × 4 columns (${grid && grid.length}×${grid && grid[0].length})`);
ok(grid && JSON.stringify(grid[0]) === JSON.stringify(["", "Name", "Age", "More"]), `header row (${grid && JSON.stringify(grid[0])})`);
ok(grid && grid[1][1] === "P2" && grid[3][1] === "P1" && grid[2][1] === "", `rows after InsertRow / SwapRows (${grid && JSON.stringify(grid.map((r) => r[1]))})`);
const look = await frame.evaluate(() => {
  const table = document.getElementById("rr-grid-table");
  const td = (c, r) => table.querySelector(`td[data-col="${c}"][data-row="${r}"]`);
  return {
    firstWidth: Math.round(td(0, 1).getBoundingClientRect().width),
    fixedBg: getComputedStyle(td(1, 0)).backgroundColor,
    markup: !!table.querySelector("td b"),
    ellipsis: !!td(3, 2).querySelector(".rr-grid-ellipsis"),
    fixedEllipsis: !!td(3, 0).querySelector(".rr-grid-ellipsis"),
    ideDots: !!document.querySelector('#rr-ide-table td[data-col="2"][data-row="1"] .rr-grid-ellipsis'),
  };
});
ok(look.firstWidth === 30, `ColWidths(0) = 30 (${look.firstWidth})`);
ok(look.fixedBg === "rgb(212, 208, 200)", `fixed row shaded (${look.fixedBg})`);
ok(!look.markup, "cell text is plain text, never markup");
ok(look.ellipsis && !look.fixedEllipsis, "ellipsis column has a button (not in the fixed row)");
ok(look.ideDots, `RapidR "..." cell shows a button`);

// Selecting: a click selects and fires OnSelectCell; fixed cells don't.
await frame.click('#rr-grid-table td[data-col="2"][data-row="4"]');
await page.waitForTimeout(300);
let lbl = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(lbl === "sel2,4|2,4", `click selects cell (2, 4) (${lbl})`);
const selected = await frame.evaluate(() => [...document.querySelectorAll('#rr-grid-table td[aria-selected="true"]')].map((td) => td.dataset.col + "," + td.dataset.row));
ok(JSON.stringify(selected) === JSON.stringify(["2,4"]), `selected cell highlighted (${JSON.stringify(selected)})`);
await frame.click('#rr-grid-table td[data-col="1"][data-row="0"]');
await page.waitForTimeout(300);
lbl = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(lbl === "sel2,4|2,4", `a fixed cell isn't selected (${lbl})`);
await frame.focus("#rr-grid");
await frame.press("#rr-grid", "ArrowUp");
await page.waitForTimeout(300);
lbl = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(lbl === "sel2,3|2,3", `ArrowUp moves the selection (${lbl})`);

// Editing (goEditing): double-click, type, Enter.
await frame.dblclick('#rr-grid-table td[data-col="1"][data-row="4"]');
await page.waitForTimeout(300);
const editing = await frame.evaluate(() => !!document.querySelector('#rr-grid-table td[data-col="1"][data-row="4"] [contenteditable="true"]'));
ok(editing, "double-click edits the cell in place");
await frame.evaluate(() => {
  const el = document.querySelector('#rr-grid-table td[data-col="1"][data-row="4"] [contenteditable="true"]');
  el.textContent = "Ana";
});
await frame.press('#rr-grid-table td[data-col="1"][data-row="4"] [contenteditable="true"]', "Enter");
await page.waitForTimeout(400);
lbl = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(lbl === "edit1,4=Ana|Ana", `Enter stores the cell and fires OnSetEditText (${lbl})`);
const after = await cells();
ok(after && after[4][1] === "Ana", `edited text drawn (${after && after[4][1]})`);

// Ellipsis button.
await frame.click('#rr-grid-table td[data-col="3"][data-row="2"] .rr-grid-ellipsis');
await page.waitForTimeout(300);
lbl = await frame.evaluate(() => document.getElementById("rr-lbl")?.textContent);
ok(lbl === "ellipsis3,2", `ellipsis button fires OnEllipsisClick(3, 2) (${lbl})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_grid.png" });
await browser.close();
if (failed) { console.log(`\nGrid: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGrid: ALL CHECKS PASSED");
