import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = "http://localhost:8765";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

(async () => {
  await fs.mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1000, height: 800 } });
  const page = await ctx.newPage();
  
  page.on("pageerror", e => console.log("[pageerror]", e.message));
  page.on("console", m => console.log(`[console.${m.type()}]`, m.text()));

  console.log("Loading Web IDE...");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

  console.log("Selecting SQLite Demo example...");
  await page.selectOption("#examples", "../examples/demo_sqlite.rr");
  await page.waitForTimeout(1000);

  console.log("Starting the preview...");
  await page.click('#btn-run');
  await page.waitForTimeout(4000);

  console.log("Locating preview iframe and hovering over &File...");
  const frame = page.frameLocator("#preview");
  const fileMenu = frame.locator('.rr-menu-item-top:has-text("File")').first();
  await fileMenu.hover({ timeout: 5000 });
  await page.waitForTimeout(500);

  console.log("Taking screenshot of the preview window with dropdown open...");
  const previewBox = page.locator("#preview-window");
  await previewBox.screenshot({ path: path.join(OUT_DIR, "sqlite_demo_hover.png") });

  await browser.close();
  console.log("Dropdown hover screenshot captured successfully!");
})().catch(e => {
  console.error("Test failed:", e);
  process.exit(1);
});
