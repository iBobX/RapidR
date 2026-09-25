// MESSAGEBOX / MESSAGEDLG in the IDE preview: blocking browser dialogs with
// the manual's return values (IDYES = 6, mrNo = 7, IDOK = 1). Browsers offer
// OK or OK/Cancel, so a Yes/No box says which is which.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_dialogs.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();
const page = await browser.newPage();
const dialogs = [];
// Answer in order: Yes (OK), No (Cancel), then OK on the plain message.
const answers = ["accept", "dismiss", "accept"];
page.on("dialog", async (d) => {
  dialogs.push({ type: d.type(), message: d.message() });
  const a = answers[dialogs.length - 1] || "accept";
  await (a === "accept" ? d.accept() : d.dismiss());
});
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate(() => {
  window.RapidR.state.project.forms[0].code = { handlers: {}, source: [
    '$INCLUDE "RAPIDQ.INC"',
    'r = MESSAGEBOX("Save changes?", "Editor", MB_YESNO OR MB_ICONQUESTION)',
    'PRINT "r="; r',
    'd = MESSAGEDLG("Quit now?", mtConfirmation, mbYes OR mbNo, 0)',
    'PRINT "d="; d',
    'm = MESSAGEBOX("Saved.", "Editor", MB_OK)',
    'PRINT "m="; m',
  ].join("\n") };
  window.RapidR.runCommand("run.start");
});
await page.waitForTimeout(2000);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);

ok(dialogs.length === 3, `three dialogs shown (${dialogs.length})`);
ok(dialogs[0]?.type === "confirm" && /Editor\n\nSave changes\?/.test(dialogs[0].message) && /OK = Yes, Cancel = No/.test(dialogs[0].message),
  `MESSAGEBOX Yes/No: a confirm with title and legend (${JSON.stringify(dialogs[0])})`);
ok(/r=6/.test(out), "choosing Yes returns IDYES (6)");
ok(dialogs[1]?.type === "confirm" && /^Confirm\n\nQuit now\?/.test(dialogs[1].message), `MESSAGEDLG titled by its type (${JSON.stringify(dialogs[1])})`);
ok(/d=7/.test(out), "choosing No returns mrNo (7)");
ok(dialogs[2]?.type === "alert", "MB_OK is a plain message");
ok(/m=1/.test(out), "OK returns IDOK (1)");

await browser.close();
if (failed) { console.log(`\nDialogs: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nDialogs: ALL CHECKS PASSED");
