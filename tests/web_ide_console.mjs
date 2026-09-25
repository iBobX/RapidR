// Console statements in the IDE: CLS / COLOR / LOCATE compile to ANSI escape
// sequences (crates/rapidr-value/src/console.rs) and the Output panel
// renders them (web-ide/ansi_screen.js). Plain PRINT output is unchanged.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_ide_console.mjs

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

async function runProgram(source) {
  await page.evaluate((src) => {
    window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
    window.RapidR.runCommand("run.start");
  }, source);
  await page.waitForTimeout(1200);
}
const outputText = () => page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);

// 1. Plain PRINT output looks exactly as before.
await runProgram('PRINT "one"\nPRINT "two"');
ok(/one\ntwo\n/.test(await outputText()), "plain PRINT lines unchanged");

// 2. CLS clears, COLOR colors, LOCATE positions.
await runProgram([
  "CLS",
  'PRINT "first line"',
  "COLOR 14, 1",
  'PRINT "yellow on blue"',
  "COLOR",
  "LOCATE 1, 7",
  'PRINT "LINE"',
  'PRINT "row"; CSRLIN',
].join("\n"));
const text = await outputText();
ok(!/one\ntwo/.test(text), `CLS cleared the earlier output (${JSON.stringify(text.slice(0, 60))})`);
ok(text.startsWith("first LINE"), `LOCATE 1, 7 overwrote row 1 from column 7 (${JSON.stringify(text.split("\n")[0])})`);
ok(!text.includes("\x1b") && !text.includes("[93"), "no raw escape sequences in the panel");
const colored = await page.evaluate(() =>
  [...document.querySelectorAll('.obody[data-tab="output"] span span')]
    .filter((s) => s.style.color || s.style.backgroundColor)
    .map((s) => ({ text: s.textContent, color: s.style.color, bg: s.style.backgroundColor })));
// "row" printed on row 2 overwrote the start of "yellow on blue", as on a terminal.
ok(text.split("\n")[1] === "row2ow on blue", `the next PRINT continued on row 2 (${JSON.stringify(text.split("\n")[1])})`);
ok(/^first LINE\nrow2ow on blue\n?$/.test(text), "no stray empty rows after CLS");
const yellow = colored.find((c) => c.text.includes("ow on blue"));
ok(yellow && yellow.color === "rgb(255, 255, 85)" && yellow.bg === "rgb(0, 0, 170)", `COLOR 14, 1 → yellow on blue (${JSON.stringify(yellow)})`);
ok(!colored.some((c) => c.text.includes("LINE")), "COLOR with no arguments reset the colors");
ok(pageErrors.length === 0, `no page errors (${pageErrors.length})`);

await browser.close();
if (failed) { console.log(`\nConsole: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nConsole: ALL CHECKS PASSED");
