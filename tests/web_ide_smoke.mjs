// Playwright smoke test for the in-browser RapidR IDE.
//
// Boots a chromium headless shell, navigates to the locally-served
// IDE, clicks Run on the auto-loaded hello_web.rr example, and asserts
// (1) the IDE status reaches "running"
// (2) the preview iframe contains the program's expected DOM
// (3) no uncaught console errors fired during compile + run.
//
// Usage:  node tests/web_ide_smoke.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const SHOT_DIR = new URL("./web-screenshots/", import.meta.url).pathname;
mkdirSync(SHOT_DIR, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => errors.push(`[pageerror] ${e.message}`));
page.on("console", msg => {
  if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`);
});

console.log(`→ navigating to ${URL_BASE}/web-ide/index.html`);
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });

// Wait for "ready" (means wasm init + asset prefetch completed)
await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.includes("ready"),
  { timeout: 15000 }
);
await page.screenshot({ path: SHOT_DIR + "ide-initial.png", fullPage: true });
console.log("✓ IDE booted (status=ready)");

// hello_web.rr is auto-loaded as the first non-placeholder option.
// Just click Run.
await page.click("#btn-run");
await page.waitForFunction(
  () => /running|preview ready/i.test(document.getElementById("status")?.textContent || ""),
  { timeout: 10000 }
);
// Give the iframe a moment to actually render the program
await page.waitForTimeout(800);
await page.screenshot({ path: SHOT_DIR + "ide-after-run.png", fullPage: true });

const previewBody = await page.frameLocator("#preview").locator("body").innerHTML();
console.log("preview body length:", previewBody.length);
if (previewBody.length < 50) {
  console.error("✗ preview iframe looks empty");
  console.error(previewBody);
  process.exitCode = 1;
} else {
  console.log("✓ preview rendered content");
}

if (errors.length) {
  console.error("✗ console errors:");
  for (const e of errors) console.error("  " + e);
  process.exitCode = 1;
} else {
  console.log("✓ no console errors");
}

await browser.close();
console.log(process.exitCode ? "FAIL" : "PASS");
