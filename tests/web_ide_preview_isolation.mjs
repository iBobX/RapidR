// SEC-02/03 regression test: the preview iframe must be isolated from the IDE.
//
// Runs a hostile program in the IDE preview that tries to read the IDE's
// localStorage and DOM and to forge IDE protocol messages, then forges
// messages from the IDE page itself (standing in for any other window).
// Also checks that the storage shim keeps RWebStorage working across runs
// and that PRINT output is delivered exactly once.
//
// Usage:  node tests/web_ide_preview_isolation.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const SECRET = "sk-ant-TEST-DO-NOT-LEAK";

let failed = 0;
function ok(cond, msg) {
  console.log(`${cond ? "✓" : "✗"} ${msg}`);
  if (!cond) failed++;
}

const PROGRAM = [
  "CREATE JS AS RJAVASCRIPT",
  "END CREATE",
  "CREATE St AS RWEBSTORAGE",
  "END CREATE",
  "DIM r AS STRING",
  `r = JS.Eval("(function(){try{return 'LS:'+parent.localStorage.getItem('rapidr-test-secret')}catch(e){return 'LS-BLOCKED:'+e.name}})()")`,
  "PRINT r",
  `r = JS.Eval("(function(){try{return 'DOM:'+parent.document.title}catch(e){return 'DOM-BLOCKED:'+e.name}})()")`,
  "PRINT r",
  `r = JS.Eval("(function(){parent.postMessage({__rapidr_log:'SPOOF-LOG',__rapidr_status:'SPOOF-STATUS',__rapidr_console:{level:'log',text:'SPOOF-CONSOLE'},__rapidr_hello:true},'*');return 'sent'})()")`,
  `PRINT "PREV=" + St.Get("visits")`,
  `St.Set("visits", "seen")`,
  `PRINT "PRINT-ONCE-MARKER"`,
].join("\n");

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));

await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });

const projectName = await page.evaluate(({ secret, program }) => {
  localStorage.setItem("rapidr-test-secret", secret);
  const project = window.RapidR.state.project;
  localStorage.removeItem(`rapidr-app-storage:${project.name}`);
  const form = project.forms[0];
  form.code = form.code || {};
  form.code.source = program;
  return project.name;
}, { secret: SECRET, program: PROGRAM });

// Run echoes the program source into the Output panel first; only the text
// after the source's last line is runtime output.
const SOURCE_END = 'PRINT "PRINT-ONCE-MARKER"';
const runtimeOutput = () => page.evaluate((end) => {
  const all = document.querySelector('.obody[data-tab="output"]')?.textContent || "";
  const i = all.lastIndexOf(end);
  return i < 0 ? "" : all.slice(i + end.length);
}, SOURCE_END);
async function runOnce() {
  await page.evaluate(() => { document.querySelector('.obody[data-tab="output"]').textContent = ""; });
  await page.evaluate(() => window.RapidR.runCommand("run.start"));
  await page.waitForFunction((end) => {
    const all = document.querySelector('.obody[data-tab="output"]')?.textContent || "";
    const i = all.lastIndexOf(end);
    return i >= 0 && all.slice(i + end.length).includes("PRINT-ONCE-MARKER");
  }, SOURCE_END, { timeout: 15000 });
  await page.waitForTimeout(300);  // let any forged messages arrive
  return runtimeOutput();
}

// ── Run 1 ──
const out1 = await runOnce();
ok(out1.includes("LS-BLOCKED:SecurityError"), "program cannot read IDE localStorage");
ok(!out1.includes(SECRET), "secret never reaches the program output");
ok(out1.includes("DOM-BLOCKED:SecurityError"), "program cannot read IDE DOM");
ok(!/SPOOF/.test(out1), "forged window messages from the program are ignored (output)");
const status1 = await page.evaluate(() => document.getElementById("status")?.textContent || "");
ok(!/SPOOF/.test(status1), `forged status ignored (status="${status1}")`);
ok(out1.split("PRINT-ONCE-MARKER").length - 1 === 1, "PRINT output delivered exactly once");
ok(out1.includes("PREV=\n") || /PREV=\s*$/m.test(out1), "first run: storage starts empty");

const previewFrame = page.frames().find((f) => f.url().includes("preview.html"));
ok(!!previewFrame, "preview frame present");
const frameOrigin = await previewFrame.evaluate(() => self.origin);
ok(frameOrigin === "null", `preview runs in an opaque origin (self.origin=${frameOrigin})`);

// ── Forge messages from the IDE page itself (any other window could do this) ──
await page.evaluate(() => {
  window.postMessage({ __rapidr_status: "SPOOF-EXTERNAL", __rapidr_debug_paused: { line: 1, stack: [], vars: [] } }, "*");
  window.postMessage({ __rapidr_hello: true }, "*");
});
await page.waitForTimeout(300);
const afterForge = await page.evaluate(() => ({
  status: document.getElementById("status")?.textContent || "",
  paused: !!window.RapidR.state.isDebugPaused,
}));
ok(!/SPOOF/.test(afterForge.status), "external forged status ignored");
ok(!afterForge.paused, "external forged debug-pause ignored");

// ── Run 2: storage shim persisted per project ──
await page.evaluate(() => window.RapidR.runCommand("run.stop"));
const out2 = await runOnce();
ok(out2.includes("PREV=seen"), "RWebStorage value persists across runs");
const stored = await page.evaluate((name) => ({
  app: localStorage.getItem(`rapidr-app-storage:${name}`),
  secret: localStorage.getItem("rapidr-test-secret"),
}), projectName);
ok(stored.app && JSON.parse(stored.app).visits === "seen", "program storage kept under its project namespace");
ok(stored.secret === SECRET, "IDE's own keys untouched");

await page.evaluate(() => {
  window.RapidR.runCommand("run.stop");
  localStorage.removeItem("rapidr-test-secret");
});
ok(pageErrors.length === 0, `no IDE page errors (got ${pageErrors.length}${pageErrors.length ? ": " + pageErrors[0] : ""})`);

await browser.close();
if (failed) {
  console.log(`\nPreview isolation: ${failed} CHECK(S) FAILED`);
  process.exit(1);
}
console.log("\nPreview isolation: ALL CHECKS PASSED");
