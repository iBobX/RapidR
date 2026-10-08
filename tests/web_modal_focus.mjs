// The modal rule on the web, with real input (page.mouse, page.keyboard):
// while a modal window is open — MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE,
// INPUT's box, QCOLORDIALOG, QFONTDIALOG, a program's own ShowModal form —
// the focus, the keys and the clicks are its own:
//
//   * a click on the window below is refused, and the focus stays in the
//     dialog while the window below repaints (its caret blinks: the bug
//     where Save As's field lost the keys to Notepad's editor);
//   * typed keys go into the dialog's field, not the window below;
//   * Tab and Shift+Tab cycle inside it;
//   * Enter presses its default button, Escape cancels;
//   * when it closes, the focus is back on the control that had it.
//
// The program runs on the web runtime's own page (tests/web_run.mjs).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_modal_focus.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = [
  '$INCLUDE "RAPIDQ.INC"',
  'CREATE ColorDlg AS QCOLORDIALOG',
  'END CREATE',
  'CREATE FontDlg AS QFONTDIALOG',
  'END CREATE',
  'SUB ChildOk',
  '  Result.Caption = "child ok " + ChildEdit.Text',
  '  Child.ModalResult = mrOk',
  'END SUB',
  'SUB ChildCancel',
  '  Result.Caption = "child cancel"',
  '  Child.ModalResult = mrCancel',
  'END SUB',
  'CREATE Child AS QFORM',
  '  Caption = "Child"',
  '  Width = 260',
  '  Height = 130',
  '  CREATE ChildEdit AS QEDIT',
  '    Left = 10: Top = 10: Width = 200',
  '  END CREATE',
  '  CREATE OkBtn AS QBUTTON',
  '    Caption = "OK": Left = 10: Top = 50: Default = 1: OnClick = ChildOk',
  '  END CREATE',
  '  CREATE CancelBtn AS QBUTTON',
  '    Caption = "Cancel": Left = 100: Top = 50: Cancel = 1: OnClick = ChildCancel',
  '  END CREATE',
  'END CREATE',
  'SUB AskBox',
  '  Result.Caption = "box " + STR$(MESSAGEBOX("Save changes?", "Box", MB_YESNOCANCEL))',
  'END SUB',
  'SUB AskDlg',
  '  Result.Caption = "dlg " + STR$(MESSAGEDLG("Quit now?", mtConfirmation, mbYes OR mbNo, 0))',
  'END SUB',
  'SUB AskShow',
  '  SHOWMESSAGE "Hello"',
  '  Result.Caption = "shown"',
  'END SUB',
  'SUB AskInput',
  '  DIM N AS STRING',
  '  INPUT "Your name? ", N',
  '  Result.Caption = "input " + N',
  'END SUB',
  'SUB AskColor',
  '  Result.Caption = "color " + STR$(ColorDlg.Execute)',
  'END SUB',
  'SUB AskFont',
  '  Result.Caption = "font " + STR$(FontDlg.Execute)',
  'END SUB',
  'SUB AskChild',
  '  ChildEdit.Text = ""',
  '  Child.ShowModal',
  'END SUB',
  'CREATE Form AS QFORM',
  '  Caption = "Main"',
  '  Width = 420: Height = 260',
  '  CREATE MainEdit AS QEDIT',
  '    Left = 10: Top = 10: Width = 380',
  '  END CREATE',
  '  CREATE Result AS QLABEL',
  '    Left = 10: Top = 40: Width = 380: Caption = "none"',
  '  END CREATE',
  ...["Box", "Dlg", "Show", "Input", "Color", "Font", "Child"].map((n, i) => [
    `  CREATE B${n} AS QBUTTON`,
    `    Caption = "${n}": Left = ${10 + (i % 4) * 95}: Top = ${70 + Math.floor(i / 4) * 35}: Width = 90: OnClick = Ask${n}`,
    '  END CREATE',
  ]).flat(),
  'END CREATE',
  'Form.ShowModal',
].join("\n");

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const errors = r.pageErrors;
page.on("dialog", async (d) => { errors.push("browser dialog " + d.type()); await d.dismiss(); });
await r.run(SOURCE);
// (the program's windows are the runtime page's own)
const frame = page;
await k.waitFor(frame, "MainEdit");

async function until(fn, ms = 5000) {
  const end = Date.now() + ms;
  let v;
  while (Date.now() < end) { v = await fn(); if (v) return v; await new Promise((r) => setTimeout(r, 50)); }
  return v;
}
// (the focused element: its id, else its accessible name; and its window)
const focused = () => frame.evaluate(() => { const a = document.activeElement; return a ? (a.id || a.getAttribute("aria-label") || a.tagName) : ""; });
const focusForm = () => frame.evaluate(() => document.activeElement?.closest(".rr-kwin")?.dataset.rrForm ?? "");
const center = async (name) => { const b = await frame.locator("#rr-" + name.toLowerCase()).boundingBox(); return [b.x + b.width / 2, b.y + b.height / 2]; };
const clickAt = async ([x, y]) => page.mouse.click(x, y);
const result = () => k.text(frame, "Result");
const top = async () => { const w = await k.windows(frame); return w[w.length - 1]; };

// The main window's edit has the focus and some text. ("the control that
// had the focus" below is the button clicked to open each dialog: a click
// focuses a button, as on Windows; the edit's own case is Notepad's,
// tests/web_file_dialogs.mjs)
await clickAt(await center("MainEdit"));
await page.keyboard.type("main");
ok(await until(async () => (await k.text(frame, "MainEdit")) === "main"), "typed into the main window's edit");

/// Opens dialog `button`'s and checks the rule; `answer` closes it (real
/// keys or clicks); `expect` the result label after.
async function check(name, button, answer, expect, { field = false, single = false } = {}) {
  const edit = await center("MainEdit");
  await clickAt(await center(button));
  const w = await until(async () => { const t = await top(); return t && t.form !== "form" ? t : null; });
  ok(!!w, `${name}: its window opens (${JSON.stringify(w)})`);
  if (!w) return;
  ok(await until(async () => (await focusForm()) === w.form), `${name}: the focus is in it (${await focused()})`);
  // A click on the main window's edit below: refused. Its caret blinks on
  // (the main window repaints): the focus stays in the dialog.
  await clickAt(edit);
  await page.waitForTimeout(1300);
  ok((await focusForm()) === w.form && (await top()).form === w.form, `${name}: a click on the window below is refused; the focus stays (${await focused()} in ${await focusForm()})`);
  // Keys go to the dialog, never to the edit below.
  await page.keyboard.type(field ? "Ada" : "x");
  await page.waitForTimeout(200);
  ok((await k.text(frame, "MainEdit")) === "main", `${name}: keys never reach the window below (${await k.text(frame, "MainEdit")})`);
  // Tab and Shift+Tab cycle inside it.
  const seen = new Set();
  let outside = false;
  for (let i = 0; i < 12; i++) {
    await page.keyboard.press(i % 5 === 4 ? "Shift+Tab" : "Tab");
    if ((await focusForm()) !== w.form) outside = true;
    seen.add(await focused());
  }
  ok(!outside && seen.size >= Math.min(2, single ? 1 : 2), `${name}: Tab / Shift+Tab cycle inside it (${[...seen].join(", ")})`);
  await answer(w);
  ok(await until(async () => (await top()).form === "form"), `${name}: it closes`);
  ok(await until(async () => expect.test(await result())), `${name}: the program got its answer (${await result()})`);
  // (the control that had it: the button clicked to open it, as on Windows)
  ok(await until(async () => (await focused()) === "rr-" + button.toLowerCase()), `${name}: the focus is back on the control that had it (${await focused()})`);
}

const press = (key) => async () => page.keyboard.press(key);
// MESSAGEBOX: Escape cancels (IDCANCEL 2).
await check("MESSAGEBOX", "BBox", press("Escape"), /^box\s+2$/);
// MESSAGEBOX: Enter presses the focused button (Yes after a Shift+Tab… the
// default is Yes: back to it by a click on it, then Enter — IDYES 6).
await check("MESSAGEBOX (Enter)", "BBox", async () => { await k.clickButton(frame, "No"); }, /^box\s+7$/);
// MESSAGEDLG: Escape (mrNo 7 for Yes / No).
await check("MESSAGEDLG", "BDlg", press("Escape"), /^dlg\s+7$/);
// SHOWMESSAGE: Enter.
await check("SHOWMESSAGE", "BShow", async () => { await k.clickButton(frame, "OK"); }, /^shown$/, { single: true });
// INPUT's box: the text typed, Enter answers.
await check("INPUT", "BInput", async (w) => {
  const f = frame.locator(`.rr-kwin[data-rr-form="${w.form}"] input`);
  const b = await f.boundingBox();
  await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("Grace");
  await page.keyboard.press("Enter");
}, /^input Grace$/, { field: true });
// QCOLORDIALOG: Escape cancels (Execute 0).
await check("QCOLORDIALOG", "BColor", press("Escape"), /^color\s+0$/);
// QFONTDIALOG: Enter presses OK (Execute 1 / True).
await check("QFONTDIALOG", "BFont", async (w) => { await k.clickButton(frame, "OK"); }, /^font\s+(1|-1)$/);
// A ShowModal form: typed into its edit, Enter presses its Default button…
await check("ShowModal (Enter)", "BChild", async () => {
  await clickAt(await center("ChildEdit"));
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("Lin");
  await page.keyboard.press("Enter");
}, /^child ok Lin$/, { field: true });
// …and Escape its Cancel button.
await check("ShowModal (Escape)", "BChild", press("Escape"), /^child cancel$/, { field: true });

// The default button answers Enter in a box (the focus on it from the
// start): MESSAGEBOX's Yes.
await clickAt(await center("BBox"));
const box = await until(async () => { const t = await top(); return t?.form !== "form" ? t : null; });
await until(async () => (await focusForm()) === box?.form);
await page.keyboard.press("Enter");
ok(await until(async () => /^box\s+6$/.test(await result())), `Enter presses the default button (${await result()})`);

ok((await k.text(frame, "MainEdit")) === "main", "the main window's edit kept its text");
ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
await browser.close();
if (failed) { console.log(`\nModal focus: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nModal focus: ALL CHECKS PASSED");
