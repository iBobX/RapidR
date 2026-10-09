// QSTRINGGRID on the web (the web interpreter), drawn from the data
// the desktop runtime draws too (rapidr_value::objects::grid; the desktop
// side is checked by tests/fixtures/string_grid.bas):
//   * Cell(col, row) read/write, ColCount / RowCount, FixedRows / FixedCols,
//     ColWidths(i), InsertRow, SwapRows, Row / Col;
//   * clicking selects a cell (OnSelectCell, OnClick); fixed cells can't be
//     selected; arrow keys move the selection;
//   * goEditing: double-click edits in place, Enter stores (OnSetEditText);
//   * an ellipsis column (ColumnStyle = gcsEllipsis) fires OnEllipsisClick;
//   * RapidR's AddRow / SetCell / GetCell / Clear.
// (On the kernel host the grid is drawn on the window's canvas: the test
// reads its cells through the accessibility mirror — a `grid` of `row`s of
// `gridcell`s, each in its place — reads what is drawn from the canvas's
// pixels, and acts with real clicks and keys, tests/web_kernel_page.mjs.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_grid.mjs

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
].join("\n"));
await page.waitForTimeout(2500);
const out = await Promise.resolve(r.output());
const printed = out.slice(out.lastIndexOf("grid="));
ok(/grid=6\|4\|P1\|P2\|30\|1/.test(printed), `cells, sizes, InsertRow and SwapRows read back (${JSON.stringify(printed.slice(0, 60))})`);
ok(/ide=2\|3\|Run\|\.\.\./.test(printed), `RapidR's AddRow / SetCell / GetCell (${JSON.stringify(printed.slice(0, 120))})`);

const frame = page;
await k.waitFor(frame, "Grid");

// (the grid's cells as the mirror has them: [row][col] → { text, selected,
// x, y, w, h } — the place in CSS pixels from the grid's top left corner)
const cells = (name) => frame.evaluate((n) => {
  const grid = document.getElementById("rr-" + n);
  if (!grid) return null;
  const g = grid.getBoundingClientRect();
  return [...grid.querySelectorAll('[role="row"]')].map((row) => [...row.querySelectorAll('[role="gridcell"]')].map((c) => {
    const r = c.getBoundingClientRect();
    return { text: c.getAttribute("aria-label") ?? "", selected: c.getAttribute("aria-selected") === "true", x: r.left - g.left, y: r.top - g.top, w: r.width, h: r.height };
  }));
}, name);
const texts = (g) => g && g.map((r) => r.map((c) => c.text));
const label = () => k.text(frame, "Lbl");
// (a real click inside cell (c, r) of grid `name`, at (dx, dy) from its top
// left corner — its middle by default)
const clickCell = async (g, c, r, at = null, opts = {}) => {
  const cell = g[r][c];
  await k.click(frame, "Grid", at ? [cell.x + at[0], cell.y + at[1]] : [cell.x + cell.w / 2, cell.y + cell.h / 2], opts);
};
// (the colours drawn in cell (c, r)'s square [x0, y0, x1, y1) — CSS pixels
// from the cell's corner — as a map "r,g,b" → count)
const colours = async (name, cell, [x0, y0, x1, y1]) => {
  const p = await k.pixels(frame, name);
  const seen = {};
  for (let y = Math.ceil((cell.y + y0) * p.scale); y < Math.floor((cell.y + y1) * p.scale); y++) {
    for (let x = Math.ceil((cell.x + x0) * p.scale); x < Math.floor((cell.x + x1) * p.scale); x++) {
      const i = (y * p.width + x) * 4;
      const key = `${p.data[i]},${p.data[i + 1]},${p.data[i + 2]}`;
      seen[key] = (seen[key] || 0) + 1;
    }
  }
  return seen;
};
const dark = (seen) => Object.keys(seen).some((key) => key.split(",").every((v) => Number(v) < 100));
const most = (seen) => Object.entries(seen).sort((a, b) => b[1] - a[1])[0]?.[0];
// (an ellipsis button: the square at the cell's right, a face-coloured
// button with the "..." in it; inset 3px past its edges)
const button = async (name, cell) => {
  const s = await colours(name, cell, [cell.w - cell.h + 3, 3, cell.w - 3, cell.h - 3]);
  return { dots: dark(s), back: most(s) };
};

const g0 = await cells("grid");
const grid = texts(g0);
ok(!!grid, "grid rendered");
ok(grid && grid.length === 6 && grid.every((r) => r.length === 4), `6 rows × 4 columns (${grid && grid.length}×${grid && grid[0].length})`);
ok(grid && JSON.stringify(grid[0]) === JSON.stringify(["", "Name", "Age", "More"]), `header row (${grid && JSON.stringify(grid[0])})`);
ok(grid && grid[1][1] === "P2" && grid[3][1] === "P1" && grid[2][1] === "", `rows after InsertRow / SwapRows (${grid && JSON.stringify(grid.map((r) => r[1]))})`);
ok(g0 && g0[1][0].w === 30, `ColWidths(0) = 30 (${g0 && g0[1][0].w})`);
// (a fixed cell is drawn in the face colour — the form's own, the theme's
// rapidr_value::theme — a normal one in the window's: sampled right of
// their texts; the form's face beside the grid)
const fixedBack = most(await colours("grid", g0[0][2], [g0[0][2].w - 20, 4, g0[0][2].w - 4, g0[0][2].h - 4]));
const cellBack = most(await colours("grid", g0[2][2], [4, 4, g0[2][2].w - 4, g0[2][2].h - 4]));
const face = (await k.pixelAt(frame, "Form", 460, 100))?.join(",");
ok(fixedBack === face && cellBack === "255,255,255" && fixedBack !== cellBack, `fixed row shaded (${fixedBack}; the form's face ${face}; a normal cell ${cellBack})`);
// (the kernel draws a cell's text as it is: the mirror's name is the text,
// "<b>" and all)
ok(grid && grid[5][1] === "<b>plain</b>", `cell text is plain text, never markup (${grid && grid[5][1]})`);
const ell = await button("grid", g0[2][3]);
const fixedEll = await button("grid", g0[0][3]);
const plainCell = await button("grid", g0[2][2]);
// (a list / ellipsis column's button shows in the cell's editor only —
// RC.EXE's windows —: no cell at rest has one drawn)
ok(!ell.dots && ell.back === "255,255,255" && !fixedEll.dots && !plainCell.dots && plainCell.back === "255,255,255",
  `an ellipsis column draws no button on cells at rest (${JSON.stringify({ ell, fixedEll, plainCell })})`);
const ide = await cells("ide");
const ideDots = ide && ide[1] && ide[1][2] ? await button("ide", ide[1][2]) : null;
ok(ide && ide[1]?.[2]?.text === "..." && ideDots && !ideDots.dots, `RapidR "..." cell: its text, and no button at rest (${JSON.stringify(ideDots)})`);

// Selecting: a click selects and fires OnSelectCell; fixed cells don't.
await clickCell(g0, 2, 4);
await page.waitForTimeout(300);
let lbl = await label();
ok(lbl === "sel2,4|2,4", `click selects cell (2, 4) (${lbl})`);
const selected = async () => {
  const g = await cells("grid");
  return g.flatMap((row, r) => row.map((c, i) => (c.selected ? `${i},${r}` : null)).filter((s) => s));
};
let sel = await selected();
ok(JSON.stringify(sel) === JSON.stringify(["2,4"]), `selected cell highlighted (${JSON.stringify(sel)})`);
await clickCell(g0, 1, 0);
await page.waitForTimeout(300);
lbl = await label();
ok(lbl === "sel2,4|2,4", `a fixed cell isn't selected (${lbl})`);
// (the arrow key to the focused grid: the DOM focus is the kernel's)
const focused = await frame.evaluate(() => document.activeElement?.id);
ok(focused === "rr-grid", `the clicked grid has the focus (${focused})`);
await page.keyboard.press("ArrowUp");
await page.waitForTimeout(300);
lbl = await label();
ok(lbl === "sel2,3|2,3", `ArrowUp moves the selection (${lbl})`);

// Editing (goEditing): double-click, type, Enter.
await clickCell(g0, 1, 4, null, { clickCount: 2 });
await page.waitForTimeout(300);
// (the cell's editor in the mirror: a text field with the cell's text, with
// the DOM focus — what a screen reader, an input method and a phone's
// keyboard type into, as for an edit)
const editor = await frame.evaluate(() => {
  const el = document.activeElement;
  return { tag: el?.tagName, id: el?.id, value: el?.value };
});
ok(editor.tag === "INPUT" && editor.value === "P3", `double-click edits the cell in place, a focused text field in the mirror (${JSON.stringify(editor)})`);
// (the cell's text replaced with real keys: the editor starts with it all
// selected, as Delphi's in-place editor)
await page.keyboard.type("Ana", { delay: 20 });
await page.keyboard.press("Enter");
await page.waitForTimeout(400);
lbl = await label();
ok(lbl === "edit1,4=Ana|Ana", `Enter stores the cell and fires OnSetEditText (${lbl})`);
const after = texts(await cells("grid"));
ok(after && after[4][1] === "Ana", `edited text drawn (${after && after[4][1]})`);

// The ellipsis cell's editor (double-click) has the button: a square with
// the "..." in it beside the editor box; Escape closes the editor.
await clickCell(g0, 3, 2, null, { clickCount: 2 });
await page.waitForTimeout(300);
const ellEdited = await button("grid", g0[2][3]);
ok(ellEdited.dots, `the ellipsis cell being edited shows its button (${JSON.stringify(ellEdited)})`);
await page.keyboard.press("Escape");
await page.waitForTimeout(300);

// Ellipsis button: a click at the cell's right edge.
await clickCell(g0, 3, 2, [g0[2][3].w - 6, g0[2][3].h / 2]);
await page.waitForTimeout(300);
lbl = await label();
ok(lbl === "ellipsis3,2", `ellipsis button fires OnEllipsisClick(3, 2) (${lbl})`);

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_grid.png" });
await browser.close();
if (failed) { console.log(`\nGrid: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGrid: ALL CHECKS PASSED");
