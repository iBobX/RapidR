// Screenshots of every dialog the web runtime draws, to look at: the
// message boxes (MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE), INPUT's box,
// QCOLORDIALOG, QFONTDIALOG, a program's own ShowModal form, and the boxes
// the host draws for Open / Save (the name where the browser has no save
// picker, the gesture a picker needs) — at 1× and 2×, in the four kernel
// themes. Each is opened by a real click and closed with Escape.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL): node tests/capture_dialog_screens.mjs [out-dir]
// (default scratch/dialogs): <theme>_<dialog>_<scale>x.png

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const OUT = process.argv[2] || "scratch/dialogs";
mkdirSync(OUT, { recursive: true });
const THEMES = ["classic", "modern", "dark", "highcontrast"];
const DIALOGS = ["Box", "Dlg", "Show", "Input", "Color", "Font", "Child", "SaveAs", "Gesture"];

const source = (theme) => [
  `$THEME ${theme}`,
  '$INCLUDE "RAPIDQ.INC"',
  'CREATE ColorDlg AS QCOLORDIALOG',
  'END CREATE',
  'CREATE FontDlg AS QFONTDIALOG',
  'END CREATE',
  'CREATE SaveDlg AS QSAVEDIALOG',
  '  Filter = "Text files (*.txt)|*.txt|All files (*.*)|*.*"',
  '  FileName = "notes.txt"',
  'END CREATE',
  'CREATE OpenDlg AS QOPENDIALOG',
  'END CREATE',
  'SUB ChildOk', '  Child.ModalResult = mrOk', 'END SUB',
  'SUB ChildCancel', '  Child.ModalResult = mrCancel', 'END SUB',
  'CREATE Child AS QFORM',
  '  Caption = "Options"',
  '  Width = 280: Height = 140',
  '  CREATE ChildLabel AS QLABEL',
  '    Caption = "&Name:": Left = 10: Top = 14',
  '  END CREATE',
  '  CREATE ChildEdit AS QEDIT',
  '    Left = 60: Top = 10: Width = 200: Text = "RapidR"',
  '  END CREATE',
  '  CREATE OkBtn AS QBUTTON',
  '    Caption = "OK": Left = 104: Top = 60: Default = 1: OnClick = ChildOk',
  '  END CREATE',
  '  CREATE CancelBtn AS QBUTTON',
  '    Caption = "Cancel": Left = 184: Top = 60: Cancel = 1: OnClick = ChildCancel',
  '  END CREATE',
  'END CREATE',
  'SUB AskBox', '  Result.Caption = STR$(MESSAGEBOX("Save the changes to notes.txt?", "Notepad", MB_YESNOCANCEL OR MB_ICONQUESTION))', 'END SUB',
  'SUB AskDlg', '  Result.Caption = STR$(MESSAGEDLG("The disk is almost full.", mtWarning, mbOK OR mbCancel, 0))', 'END SUB',
  'SUB AskShow', '  SHOWMESSAGE "The file was saved."', 'END SUB',
  'SUB AskInput',
  '  DIM N AS STRING',
  '  INPUT "Your name? ", N',
  'END SUB',
  'SUB AskColor', '  Result.Caption = STR$(ColorDlg.Execute)', 'END SUB',
  'SUB AskFont', '  Result.Caption = STR$(FontDlg.Execute)', 'END SUB',
  'SUB AskChild', '  Child.ShowModal', 'END SUB',
  'SUB AskSaveAs', '  Result.Caption = STR$(SaveDlg.Execute)', 'END SUB',
  'SUB Later',
  '  Tmr.Enabled = 0',
  '  Result.Caption = STR$(OpenDlg.Execute)',
  'END SUB',
  'SUB AskGesture', '  Tmr.Enabled = 1', 'END SUB',
  'CREATE Form AS QFORM',
  '  Caption = "Dialogs"',
  '  Width = 420: Height = 200',
  '  CREATE Result AS QLABEL',
  '    Left = 10: Top = 10: Width = 380: Caption = ""',
  '  END CREATE',
  ...DIALOGS.map((n, i) => [
    `  CREATE B${n} AS QBUTTON`,
    `    Caption = "${n}": Left = ${10 + (i % 4) * 95}: Top = ${40 + Math.floor(i / 4) * 35}: Width = 90: OnClick = Ask${n}`,
    '  END CREATE',
  ]).flat(),
  '  CREATE Tmr AS QTIMER',
  '    Interval = 5600: Enabled = 0: OnTimer = Later',
  '  END CREATE',
  'END CREATE',
  'Form.ShowModal',
].join("\n");

const browser = await chromium.launch();
const shots = [];
for (const scale of [1, 2]) {
  for (const theme of THEMES) {
    const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 1280, height: 900 } });
    // (no File System Access: Save asks for the name in the host's box)
    await page.addInitScript("window.showOpenFilePicker = undefined; window.showSaveFilePicker = undefined;");
    await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rrReady, null, { timeout: 30000 });
    await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "dialogs", {})), source(theme));
    await k.waitFor(page, "BBox");
    for (const d of DIALOGS) {
      const b = await page.locator("#rr-b" + d.toLowerCase()).boundingBox();
      await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
      if (d === "Gesture") await page.waitForTimeout(6200);
      let win = null;
      for (let i = 0; i < 100 && !win; i++) {
        await page.waitForTimeout(50);
        const w = await k.windows(page);
        const t = w[w.length - 1];
        if (t && t.form !== "form") win = t;
      }
      if (!win) { console.log(`✗ ${theme} ${d} ${scale}x: no window`); continue; }
      await page.waitForTimeout(400);
      const path = `${OUT}/${theme}_${d.toLowerCase()}_${scale}x.png`;
      await page.locator(`.rr-kwin[data-rr-form="${win.form}"]`).screenshot({ path });
      shots.push(path);
      await page.keyboard.press("Escape");
      for (let i = 0; i < 60; i++) {
        const w = await k.windows(page);
        if (w[w.length - 1]?.form === "form") break;
        await page.waitForTimeout(50);
      }
    }
    // (the program's own window too, for the frame beside the dialogs')
    await page.locator('.rr-kwin[data-rr-form="form"]').screenshot({ path: `${OUT}/${theme}_form_${scale}x.png` });
    await page.close();
  }
}
await browser.close();
console.log(`${shots.length} screenshots in ${OUT}`);
