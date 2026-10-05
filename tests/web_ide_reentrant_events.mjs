// Events fired while the interpreter is running, in the IDE preview (the
// web VM host, interpreter/rapidr-vm-host-web): the runtime only queues
// them and the VM runs each at a safe point, to completion.
//   * A click handler calls Form2.Close (Form2 is shown), which fires Form2's OnClose
//     synchronously — inside the VM. That handler opens a dialog (the VM
//     suspends); after the answer, OnClose finishes and then the click
//     handler continues after its Close. (Before v2.30.0 this re-entered
//     the VM through a raw pointer and could run the rest of the click
//     handler on the wrong frames.)
//   * Two clicks in a row: each handler runs to completion, in order.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_reentrant_events.mjs
// (On the kernel host the clicks are real mouse clicks on the window the
// kernel draws, and the dialog is a modal kernel window:
// tests/web_kernel_page.mjs.)

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage();
const nativeDialogs = [];
page.on("dialog", async (d) => { nativeDialogs.push(d.type()); await d.dismiss(); });
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate(() => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'DIM Log AS STRING',
    'DIM Count AS INTEGER',
    'SUB Closing',
    '  Log = Log + "closing;"',
    '  a = MESSAGEBOX("Close it?", "Second form", MB_YESNO)',
    '  Log = Log + "answer" + STR$(a) + ";"',
    'END SUB',
    'SUB CloseIt',
    '  Log = Log + "click;"',
    '  Form2.Close',
    '  Log = Log + "after-close;"',
    '  Label1.Caption = Log',
    'END SUB',
    'SUB Counting',
    '  Count = Count + 1',
    '  Label2.Caption = "count " + STR$(Count)',
    'END SUB',
    'CREATE Form2 AS QFORM',
    '  Caption = "Second"',
    '  OnClose = Closing',
    'END CREATE',
    'CREATE Form AS QFORM',
    '  Caption = "Main"',
    '  Width = 400',
    '  CREATE Label1 AS QLABEL',
    '    Width = 380',
    '    Caption = "-"',
    '  END CREATE',
    '  CREATE Label2 AS QLABEL',
    '    Top = 30',
    '    Caption = "count 0"',
    '  END CREATE',
    '  CREATE Button1 AS QBUTTON',
    '    Caption = "Close"',
    '    Top = 60',
    '    OnClick = CloseIt',
    '  END CREATE',
    '  CREATE Button2 AS QBUTTON',
    '    Caption = "Count"',
    '    Top = 60',
    '    Left = 100',
    '    OnClick = Counting',
    '  END CREATE',
    'END CREATE',
    // (Form2 shown, so its Close fires OnClose: the UI kernel, desktop and
    // web, fires it only for a form whose window is up)
    'Form2.Show',
    'Form.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(1500);
const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const text = (name) => k.text(frame, name);
const dialog = async () => (await k.dialog(frame))?.text ?? null;

await k.waitFor(frame, "Button1");
await k.click(frame, "Button1");
await page.waitForTimeout(300);
ok((await dialog()) === "Close it?", `OnClose, fired inside the click handler, shows its dialog (${await dialog()})`);
ok((await text("Label1")) === "-", "the click handler waits for it");
await k.clickButton(frame, "Yes");
await page.waitForTimeout(300);
const log = await text("Label1");
ok(log === "click;closing;answer6;after-close;", `OnClose finishes, then the click handler continues after its Close (${log})`);

// (three real clicks in a row, without waiting between them: each one's
// handler is queued and runs to completion)
await k.click(frame, "Button2");
await k.click(frame, "Button2");
await k.click(frame, "Button2");
await page.waitForTimeout(300);
ok((await text("Label2")) === "count 3", `three quick clicks run the handler three times (${await text("Label2")})`);
ok(nativeDialogs.length === 0, `no browser dialogs (${nativeDialogs.join(",")})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);

await browser.close();
if (failed) { console.log(`\nRe-entrant events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nRe-entrant events: ALL CHECKS PASSED");
