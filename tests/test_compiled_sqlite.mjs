import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL = "http://localhost:8765/examples/demo_sqlite_web/index.html";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

(async () => {
  await fs.mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1000, height: 800 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("[pageerror]", e.message));
  page.on("console", msg => console.log("[console]", msg.text()));

  console.log("Navigating to compiled SQLite web app...");
  await page.goto(URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(3000); // Allow boot and DB load time

  console.log("Capturing compiled SQLite screenshot...");
  await page.screenshot({ path: path.join(OUT_DIR, "compiled_demo_sqlite_loaded.png") });

  await browser.close();
  console.log("Verification finished successfully!");
})().catch(e => {
  console.error("Test execution failed:", e);
  process.exit(1);
});
