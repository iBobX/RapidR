// Playwright E2E test for the in-browser RapidR debugger.
//
// The IDE's debugger runs the program under RapidR's program session
// protocol (rapidr-session) in the preview frame.
//
// Usage:  node tests/web_ide_debugger_test.mjs   (repo served on RAPIDR_URL,
// else http://localhost:8765)

import { chromium } from "playwright";
import { join } from "node:path";
import { mkdirSync } from "node:fs";

const PORT = +(process.env.RAPIDR_PORT || 8765);
const URL_BASE = process.env.RAPIDR_URL || `http://localhost:${PORT}`;
const SHOT_DIR = new URL("./web-screenshots/", import.meta.url).pathname;
mkdirSync(SHOT_DIR, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => {
  console.log(`[browser pageerror] ${e.message}`);
  errors.push(`[pageerror] ${e.message}`);
});
page.on("console", msg => {
  console.log(`[browser console] [${msg.type()}] ${msg.text()}`);
  if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`);
});

console.log(`→ navigating to ${URL_BASE}/web-ide/index.html`);
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });

// Wait for "ready"
await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.includes("ready"),
  { timeout: 15000 }
);
console.log("✓ IDE booted (status=ready)");

// 1. Add code and breakpoint on line 2 of Form1 code
await page.evaluate(() => {
  const form = window.RapidR.state.project.forms[0];
  form.code = form.code || {};
  form.code.source = [
    "DIM x AS INTEGER",
    "x = 42",
    "PRINT x"
  ].join("\n");
  window.RapidR.state.breakpoints.add(`${form.id}:2`);
});
console.log("✓ Code set and Breakpoint added on Form1:2");

// 2. Click Debug
await page.click("#btn-debug");
console.log("✓ Clicked Debug button");

// 3. Wait for VM to pause
await page.waitForFunction(
  () => window.RapidR.state.isDebugging && window.RapidR.state.isDebugPaused,
  { timeout: 10000 }
);
console.log("✓ Debugger paused successfully!");

await page.waitForTimeout(500);
await page.screenshot({ path: SHOT_DIR + "debugger-paused.png", fullPage: true });

// Assertions on paused state
const statusText = await page.textContent("#status");
console.log("Status text:", statusText);

const callStackHtml = await page.innerHTML("#debug-callstack");
console.log("Call stack:", callStackHtml);
if (!callStackHtml.includes("__main")) {
  console.error("✗ Call stack does not contain __main");
  process.exitCode = 1;
}

const varsHtml = await page.innerHTML("#debug-variables");
console.log("Variables view:", varsHtml);
if (!varsHtml.toLowerCase().includes("x")) {
  console.error("✗ Variables does not contain x");
  process.exitCode = 1;
}

// 4. Click Step Over
await page.click("#btn-stepover");
console.log("✓ Clicked Step Over");

// Wait for it to pause again on next line
await page.waitForTimeout(500);
await page.waitForFunction(
  () => window.RapidR.state.isDebugging && window.RapidR.state.isDebugPaused,
  { timeout: 5000 }
);
console.log("✓ Debugger paused on next step!");

const stepLine = await page.evaluate(() => window.RapidR.state.currentPausedLineInFile);
console.log("Paused line number in file:", stepLine);
if (stepLine !== 3) {
  console.error(`✗ Expected line 3 but got ${stepLine}`);
  process.exitCode = 1;
}

// 5. Stop debugging
await page.click(".tb.stop");
console.log("✓ Clicked Stop");

await page.waitForTimeout(500);
const isDebuggingAfterStop = await page.evaluate(() => window.RapidR.state.isDebugging);
if (isDebuggingAfterStop) {
  console.error("✗ Debugging state should be false after stop");
  process.exitCode = 1;
}
console.log("✓ Stopped successfully");

if (errors.length) {
  console.error("✗ console errors:");
  for (const e of errors) console.error("  " + e);
  process.exitCode = 1;
} else {
  console.log("✓ no console errors");
}

await browser.close();
console.log(process.exitCode ? "FAIL" : "PASS");
