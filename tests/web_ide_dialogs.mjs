// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / INPUT in the IDE preview: in-page
// dialogs with every button (crates/rapidr-runtime-web/src/dialog_web.rs).
// The program waits for the answer — the VM suspends and resumes — and gets
// the manual's return values (IDYES = 6, IDCANCEL = 2, mrNo = 7, …).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_dialogs.mjs

import { chromium } from "playwright";

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
    'r = MESSAGEBOX("Save changes?", "Editor", MB_YESNOCANCEL OR MB_ICONQUESTION)',
    'PRINT "r="; r',
    'd = MESSAGEDLG("Quit now?", mtConfirmation, mbYes OR mbNo, 0)',
    'PRINT "d="; d',
    'INPUT "Your name? ", n$',
    'PRINT "hello "; n$',
    'SHOWMESSAGE "Welcome"',
    'SUB Ask',
    '  a = MESSAGEBOX("Really?", "From a button", MB_YESNO)',
    '  Label1.Caption = "answer " + STR$(a)',
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
    'END CREATE',
    'Form.ShowModal',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(1500);
const frame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!frame, "preview frame found");
const output = () => page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
const dialog = () => frame.evaluate(() => {
  const d = document.querySelector(".rr-dialog");
  if (!d) return null;
  return {
    title: d.querySelector(".rr-dialog-title")?.textContent || "",
    text: d.querySelector(".rr-dialog-text")?.textContent || "",
    buttons: [...d.querySelectorAll(".rr-dialog-button")].map((b) => b.textContent),
    focused: document.activeElement?.textContent || document.activeElement?.className || "",
    input: !!d.querySelector(".rr-dialog-input"),
  };
});
const click = (label) => frame.evaluate((l) => [...document.querySelectorAll(".rr-dialog-button")].find((b) => b.textContent === l).click(), label);

// 1. MESSAGEBOX with three buttons; the program waits.
let d = await dialog();
ok(d && d.title === "Editor" && d.text === "Save changes?", `MESSAGEBOX shows title and text (${JSON.stringify(d)})`);
ok(d && JSON.stringify(d.buttons) === '["Yes","No","Cancel"]', `all three buttons (${d && d.buttons})`);
ok(d && d.focused === "Yes", `the first button is the default (${d && d.focused})`);
ok(!/\nr=\d/.test(await output()), "the program waits for the answer");
await page.screenshot({ path: "scratch/web_ide_dialog_open.png" });
await click("Cancel");
await page.waitForTimeout(300);
ok(/r=2/.test(await output()), "Cancel returns IDCANCEL (2)");

// 2. MESSAGEDLG titled by its type; Escape = No.
d = await dialog();
ok(d && d.title === "Confirm" && JSON.stringify(d.buttons) === '["Yes","No"]', `MESSAGEDLG (${JSON.stringify(d)})`);
await frame.press(".rr-dialog-button", "Escape");
await page.waitForTimeout(300);
ok(/d=7/.test(await output()), "Escape returns mrNo (7)");

// 3. INPUT: an in-page field with the printed prompt; Enter answers.
d = await dialog();
ok(d && d.input && d.text === "Your name? ", `INPUT asks in the page with the prompt (${JSON.stringify(d)})`);
await frame.fill(".rr-dialog-input", "Ada");
await frame.press(".rr-dialog-input", "Enter");
await page.waitForTimeout(300);
const out = await output();
ok(/hello Ada/.test(out), `INPUT stored the text (${JSON.stringify(out.slice(-60))})`);
ok(/Your name\? ?\n?Ada/.test(out), "the typed line is echoed in the output");

// 4. SHOWMESSAGE, then the forms appear once __main finishes.
d = await dialog();
ok(d && d.text === "Welcome" && JSON.stringify(d.buttons) === '["OK"]', `SHOWMESSAGE (${JSON.stringify(d)})`);
await click("OK");
await page.waitForTimeout(500);
ok(await frame.evaluate(() => !!document.querySelector(".rr-form")), "the form is shown after the dialogs");

// 5. A dialog from an event handler: the handler waits, then continues.
await frame.evaluate(() => [...document.querySelectorAll("button")].find((b) => b.textContent === "Ask").click());
await page.waitForTimeout(300);
d = await dialog();
ok(d && d.title === "From a button" && JSON.stringify(d.buttons) === '["Yes","No"]', `dialog from a button's handler (${JSON.stringify(d)})`);
ok(await frame.evaluate(() => document.body.innerText.includes("no answer")), "the handler waits for the answer");
await click("Yes");
await page.waitForTimeout(300);
ok(await frame.evaluate(() => document.body.innerText.includes("answer 6") || document.body.innerText.includes("answer  6")), "the handler continued with IDYES (6)");
ok(await frame.evaluate(() => !document.querySelector(".rr-dialog")), "no dialog left open");

ok(nativeDialogs.length === 0, `no browser alert/confirm/prompt used (${nativeDialogs.join(",")})`);
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_ide_dialogs.png" });

await browser.close();
if (failed) { console.log(`\nDialogs: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nDialogs: ALL CHECKS PASSED");
