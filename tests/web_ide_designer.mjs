// Playwright E2E for the visual designer.
//
// Boots the IDE, clicks ＋ New, drops Button + Label + Edit from the
// toolbox, edits the button caption via the property grid, asserts:
//   1. the source textarea ends up containing matching CREATE blocks
//   2. the design-surface iframe renders the components (real runtime)
//   3. the resulting source still compiles & runs (Run button works)
//
// Usage:  node tests/web_ide_designer.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const SHOTS = new URL("./web-screenshots/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => errors.push(`[pageerror] ${e.message}`));
page.on("console", msg => {
  if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`);
});

console.log(`→ ${URL_BASE}/web-ide/index.html`);
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.includes("ready"),
  { timeout: 15000 }
);

// New blank designer project
await page.click("#btn-new");
await page.waitForSelector("#mdi-tabs .mtab.active");

// Drop three widgets
await page.click('.tool[data-tool="RButton"]');
await page.click(".design-form", { position: { x: 50, y: 50 } });
await page.click('.tool[data-tool="RLabel"]');
await page.click(".design-form", { position: { x: 50, y: 100 } });
await page.click('.tool[data-tool="REdit"]');
await page.click(".design-form", { position: { x: 50, y: 150 } });

// Wait a tick for design surface re-render
await page.waitForTimeout(500);

// Source should contain the three CREATE blocks
const src = await page.evaluate(() => window.RapidR.serializeProject(window.RapidR.state.project));
for (const t of ["AS RBUTTON", "AS RLABEL", "AS REDIT", "AS RFORM"]) {
  if (!src.toLowerCase().includes(t.toLowerCase())) {
    console.error(`✗ source missing ${t}`);
    console.error(src);
    process.exitCode = 1;
  }
}
if (!process.exitCode) console.log("✓ designer generated all 4 CREATE blocks");

// Edit the BUTTON1 caption via the property grid: pick it in the tree,
// then change the Caption input.
await page.click('.dwidget[data-name="Button1"]');
const captionInput = page.locator('.prop-row[data-key="caption"] input');
await captionInput.fill("Greet");
await captionInput.press("Tab");
await page.waitForTimeout(400);
const src2 = await page.evaluate(() => window.RapidR.serializeProject(window.RapidR.state.project));
if (!src2.includes('Caption = "Greet"')) {
  console.error("✗ caption edit didn't propagate to source");
  process.exitCode = 1;
} else {
  console.log("✓ property edit reflected in source");
}

await page.screenshot({ path: SHOTS + "ide-designer.png", fullPage: true });

// Sanity: the design surface should have rendered something
const designBody = await page.locator(".designer").innerHTML();
if (designBody.length < 50) {
  console.error("✗ design surface iframe is empty");
  process.exitCode = 1;
} else {
  console.log(`✓ design surface body length=${designBody.length}`);
}

// Click Run to make sure the generated source still works end-to-end.
await page.click("#btn-run");
await page.waitForFunction(
  () => /running|preview ready/i.test(document.getElementById("status")?.textContent || ""),
  { timeout: 8000 }
);
await page.waitForTimeout(500);
const previewBody = await page.frameLocator("#preview").locator("body").innerHTML();
if (previewBody.length < 50) {
  console.error("✗ preview iframe is empty after Run");
  process.exitCode = 1;
} else {
  console.log(`✓ Run rendered preview (body length=${previewBody.length})`);
}

await page.screenshot({ path: SHOTS + "ide-designer-after-run.png", fullPage: true });

if (errors.length) {
  console.error("✗ console errors:");
  for (const e of errors) console.error("  " + e);
  process.exitCode = 1;
} else {
  console.log("✓ no console errors");
}

await browser.close();
console.log(process.exitCode ? "FAIL" : "PASS");
