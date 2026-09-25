// Compiler diagnostics in the web IDE: errors are underlined in the right
// editor (form or module), listed in the Errors panel with a jump-to-line
// link, refreshed while typing, and nothing runs when the program has errors.
//
// Usage:  node tests/web_ide_diagnostics.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const MOD = process.platform === "darwin" ? "Meta" : "Control";

let failed = 0;
function ok(cond, msg) {
  console.log(`${cond ? "✓" : "✗"} ${msg}`);
  if (!cond) failed++;
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));

await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });

// Form code with two errors (lines 2 and 3), plus a module with one (line 2).
await page.evaluate(() => {
  const p = window.RapidR.state.project;
  p.forms[0].code = { handlers: {}, source: 'PRINT "fine"\nNoSuchRoutine 1\nDIM AS AS\nPRINT "also fine"' };
  p.modules.push({ id: "m_diag", name: "Helpers", source: 'SUB Helper()\n  Frobble counter\nEND SUB' });
});

const markers = () => page.evaluate(() =>
  window.monaco
    ? window.monaco.editor.getModelMarkers({ owner: "rapidr" })
        .map((m) => ({ line: m.startLineNumber, col: m.startColumn, msg: m.message }))
        .sort((a, b) => a.line - b.line)
    : []);
const errorRows = () => page.evaluate(() =>
  Array.from(document.querySelectorAll('.obody[data-tab="errors"] .err-line')).map((e) => e.textContent));

async function run() {
  await page.evaluate(() => {
    document.querySelector('.obody[data-tab="output"]').textContent = "";
    window.RapidR.runCommand("run.start");
  });
  await page.waitForTimeout(500);
}
async function replaceEditorText(text) {
  await page.keyboard.press(`${MOD}+a`);
  await page.keyboard.type(text);
  await page.waitForTimeout(1200);  // > live-check debounce
}

// ── 1. Syntax errors come first: nothing runs, the error is located ──
await run();
let status = await page.evaluate(() => document.getElementById("status").textContent);
ok(/1 compile error/.test(status), `status reports the syntax error ("${status}")`);
ok(await page.evaluate(() => document.getElementById("preview-window").hidden), "program did not run");
let rows = await errorRows();
ok(rows.length === 1 && /Form1 \(line 3, col 1\): 'AS' is a reserved word/.test(rows[0]), `syntax error located in Form1 line 3 (${JSON.stringify(rows)})`);

// ── 2. Clicking the error opens the code at that line, with a squiggle ──
await page.click('.obody[data-tab="errors"] .err-line.err-link >> nth=0');
await page.waitForSelector(".mdi-pane.active .monaco-editor", { timeout: 15000 });
await page.waitForTimeout(400);
ok((await markers()).some((m) => m.line === 3 && /reserved word/.test(m.msg)), "form editor underlines line 3");
const cursorLine = await page.evaluate(() => window.monaco.editor.getEditors().find((e) => e.hasTextFocus())?.getPosition()?.lineNumber);
ok(cursorLine === 3, `click jumped to line 3 (cursor on ${cursorLine})`);

// ── 3. Fix the syntax error: the next run reports every unknown name ──
await replaceEditorText('PRINT "fine"\nNoSuchRoutine 1\nPRINT "also fine"');
await run();
status = await page.evaluate(() => document.getElementById("status").textContent);
ok(/2 compile errors/.test(status), `status reports both remaining errors ("${status}")`);
rows = await errorRows();
ok(rows.some((r) => /Form1 \(line 2, col 1\): Unknown SUB or FUNCTION 'NoSuchRoutine'/.test(r)), "unknown SUB located in Form1 line 2");
ok(rows.some((r) => /Helpers \(line 2, col 3\): Unknown SUB or FUNCTION 'Frobble'/.test(r)), "module error attributed to module Helpers, line 2 col 3");
ok((await markers()).some((m) => m.line === 2 && /NoSuchRoutine/.test(m.msg)), "form editor underlines line 2");

// ── 4. Live checking while typing, without running ──
await replaceEditorText('PRINT "fine"\nPRINT "fixed"\n');
ok(!(await markers()).some((m) => /NoSuchRoutine/.test(m.msg)), "fixing the code clears its squiggle");
await page.keyboard.type("Frobnicate 42");
await page.waitForTimeout(1200);
const live = (await markers()).filter((m) => /Frobnicate/.test(m.msg));
ok(live.length === 1 && live[0].line === 3, `new mistake underlined live on line 3 (${JSON.stringify(live)})`);

// ── 5. Once everything is fixed, the program runs ──
await replaceEditorText('PRINT "fine"\nPRINT "fixed"\n');
await page.evaluate(() => {
  window.RapidR.state.project.modules[0].source = 'SUB Helper()\n  PRINT "helper"\nEND SUB';
});
await run();
await page.waitForTimeout(2000);
const out = await page.evaluate(() => document.querySelector('.obody[data-tab="output"]').textContent);
// Output holds the echoed source (PRINT "fine") and then the program's own
// output; only the latter has bare "fine"/"fixed" lines.
ok(/^fine$/m.test(out) && /^fixed$/m.test(out), "fixed program runs and prints its output");
ok((await markers()).length === 0, "no squiggles left after a successful run");

ok(pageErrors.length === 0, `no page errors (got ${pageErrors.length}${pageErrors.length ? ": " + pageErrors[0] : ""})`);
await browser.close();
if (failed) {
  console.log(`\nDiagnostics: ${failed} CHECK(S) FAILED`);
  process.exit(1);
}
console.log("\nDiagnostics: ALL CHECKS PASSED");
