import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL = "http://localhost:8765/examples/demo_plot_web/index.html";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

(async () => {
  await fs.mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1000, height: 800 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("[pageerror]", e.message));
  page.on("console", msg => console.log("[console]", msg.text()));

  console.log("Navigating to compiled web app...");
  await page.goto(URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(2000); // Allow boot time

  console.log("Clicking 'Sine/Cosine' button inside compiled preview...");
  await page.locator("button:has-text('Sine/Cosine')").click();
  await page.waitForTimeout(1000);

  console.log("Capturing compiled preview screenshot...");
  await page.screenshot({ path: path.join(OUT_DIR, "compiled_demo_plot_preview.png") });

  await browser.close();
  console.log("Verification finished successfully!");
})().catch(e => {
  console.error("Test execution failed:", e);
  process.exit(1);
});
