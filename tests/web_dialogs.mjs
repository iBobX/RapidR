// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / INPUT on the web: in-page
// dialogs with every button (crates/rapidr-runtime-web/src/dialog_web.rs).
// The program waits for the answer — the VM suspends and resumes — and gets
// the manual's return values (IDYES = 6, IDCANCEL = 2, mrNo = 7, …).
// (On the kernel host a dialog is a modal kernel window: the test reads it
// through its accessibility mirror and answers it with real clicks and keys,
// tests/web_kernel_page.mjs.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_dialogs.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
const nativeDialogs = [];
page.on("dialog", async (d) => { nativeDialogs.push(d.type()); await d.dismiss(); });
const pageErrors = r.pageErrors;

await r.run([
    '$INCLUDE "RAPIDQ.INC"',
    'r = MESSAGEBOX("Save changes?", "Editor", MB_YESNOCANCEL OR MB_ICONQUESTION)',
    'PRINT "r="; r',
    'd = MESSAGEDLG("Quit now?", mtConfirmation, mbYes OR mbNo, 0)',
    'PRINT "d="; d',
    'INPUT "Your name? ", n$',
    'PRINT "hello "; n$',
    'SHOWMESSAGE "Welcome"',
    'DIM Ticks AS INTEGER',
    'SUB Ask',
    '  a = MESSAGEBOX("Really?", "From a button", MB_YESNO)',
    '  Label1.Caption = "answer " + STR$(a)',
    'END SUB',
    'SUB Tick',
    '  Ticks = Ticks + 1',
    '  Label2.Caption = "ticks" + STR$(Ticks)',
    'END SUB',
    'CREATE Form AS QFORM',
    '  Caption = "After the dialogs"',
    '  CREATE Label1 AS QLABEL',
    '    Caption = "no answer"',
    '  END CREATE',
    '  CREATE Button1 AS QBUTTON',
    '    Caption = "Ask"',
    '    Top = 40',
    '    OnClick = Ask',
    '  END CREATE',
    '  CREATE Label2 AS QLABEL',
    '    Caption = "no ticks"',
    '    Top = 70',
    '  END CREATE',
    '  CREATE Tmr AS QTIMER',
    '    Interval = 50',
    '    OnTimer = Tick',
    '  END CREATE',
    'END CREATE',
    'Form.ShowModal',
].join("\n"));
await page.waitForTimeout(1500);
const frame = page;
const output = () => Promise.resolve(r.output());
// (the frontmost dialog as its mirror has it, with the focused element's
// accessible name — the DOM focus follows the kernel's)
const dialog = async () => {
  const d = await k.dialog(frame);
  if (!d) return null;
  Object.assign(d, await frame.evaluate(() => ({ focused: document.activeElement?.getAttribute("aria-label") ?? "", focusedTag: document.activeElement?.tagName })));
  return d;
};
const click = (label) => k.clickButton(frame, label);
// (a real key to the focused element: the kernel gets it from the mirror)
const press = (key) => frame.locator(":focus").press(key);

// 1. MESSAGEBOX with three buttons; the program waits.
let d = await dialog();
ok(d && d.title === "Editor" && d.text === "Save changes?", `MESSAGEBOX shows title and text (${JSON.stringify(d)})`);
ok(d && JSON.stringify(d.buttons) === '["Yes","No","Cancel"]', `all three buttons (${d && d.buttons})`);
ok(d && d.focused === "Yes", `the first button is the default (${d && d.focused})`);
ok(!/\nr=\d/.test(await output()), "the program waits for the answer");
await page.screenshot({ path: "scratch/web_dialog_open.png" });
await click("Cancel");
await page.waitForTimeout(300);
ok(/r=2/.test(await output()), "Cancel returns IDCANCEL (2)");

// 2. MESSAGEDLG titled by its type; Escape = No.
d = await dialog();
ok(d && d.title === "Confirm" && JSON.stringify(d.buttons) === '["Yes","No"]', `MESSAGEDLG (${JSON.stringify(d)})`);
await press("Escape");
await page.waitForTimeout(300);
ok(/d=7/.test(await output()), "Escape returns mrNo (7)");

// 3. INPUT: an in-page field with the printed prompt; Enter answers.
// (typed as the user does: real keys into the focused field)
d = await dialog();
ok(d && d.input && d.text.trimEnd() === "Your name?", `INPUT asks in the page (a kernel dialog) with the prompt (${JSON.stringify(d)})`);
ok(d && d.focusedTag === "INPUT", `the field has the focus (${d && d.focusedTag})`);
await frame.locator(":focus").pressSequentially("Ada");
await press("Enter");
await page.waitForTimeout(300);
const out = await output();
ok(/hello Ada/.test(out), `INPUT stored the text (${JSON.stringify(out.slice(-60))})`);
ok(/Your name\? ?\n?Ada/.test(out), "the typed line is echoed in the output");

// 4. SHOWMESSAGE, then the forms appear once __main finishes.
d = await dialog();
ok(d && d.text === "Welcome" && JSON.stringify(d.buttons) === '["OK"]', `SHOWMESSAGE (${JSON.stringify(d)})`);
await click("OK");
await page.waitForTimeout(500);
ok(await k.shown(frame, "Form"), "the form is shown after the dialogs");

// 5. A dialog from an event handler: the handler waits, then continues.
await k.click(frame, "Button1");
await page.waitForTimeout(300);
d = await dialog();
ok(d && d.title === "From a button" && JSON.stringify(d.buttons) === '["Yes","No"]', `dialog from a button's handler (${JSON.stringify(d)})`);
ok((await k.text(frame, "Label1")) === "no answer", "the handler waits for the answer");
// (a timer's handler runs while the box waits, as the desktop's — native
// and interpreted: tests/fixtures/dialog_timers.bas — and RapidQ's do)
const ticks = async () => (await k.text(frame, "Label2")) || "";
const before = await ticks();
await page.waitForTimeout(400);
const during = await ticks();
ok(/^ticks\s*\d+$/.test(during) && during !== before && await dialog() !== null, `the timer ticks while the box waits (${before} → ${during})`);
await click("Yes");
await page.waitForTimeout(300);
const answer = await k.text(frame, "Label1");
ok(/^answer +6$/.test(answer), `the handler continued with IDYES (6) (${answer})`);
// (the only modal window left is the program's own form, shown with ShowModal)
const modals = (await k.windows(frame)).filter((w) => w.modal).map((w) => w.form);
ok(JSON.stringify(modals) === '["form"]', `no dialog left open (modal windows: ${modals})`);

ok(nativeDialogs.length === 0, `no browser alert/confirm/prompt used (${nativeDialogs.join(",")})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_dialogs.png" });

await browser.close();
if (failed) { console.log(`\nDialogs: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nDialogs: ALL CHECKS PASSED");
