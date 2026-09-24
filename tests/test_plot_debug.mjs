import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = "http://localhost:8765";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

(async () => {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1000, height: 800 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("[pageerror]", e.message));
  page.on("console", m => console.log(`[console.${m.type()}]`, m.text()));

  console.log("Loading Web IDE...");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

  console.log("Selecting demo_plot example...");
  await page.selectOption("#examples", "../examples/demo_plot.rr");
  await page.waitForTimeout(1000);

  console.log("Starting the preview...");
  await page.click('#btn-run');
  await page.waitForTimeout(4000);

  const frame = page.frameLocator("#preview");
  
  console.log("Clicking 'Sine/Cosine' button inside plot preview...");
  await frame.locator("button:has-text('Sine/Cosine')").click();
  await page.waitForTimeout(2000);

  console.log("Capturing screenshots...");
  const previewBox = page.locator("#preview-window");
  await previewBox.screenshot({ path: path.join(OUT_DIR, "plot_debug_preview.png") });

  console.log("Stopping the preview...");
  await page.click('#preview-close');
  await page.waitForTimeout(1000);

  await browser.close();
  console.log("Done!");
})().catch(e => {
  console.error("Test execution failed:", e);
  process.exit(1);
});
