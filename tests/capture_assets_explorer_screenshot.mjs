import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = "http://localhost:8765";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

const DUMMY_PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgAAIAAAUAAeImBZsAAAAASUVORK5CYII=";
const DUMMY_MP3 = "data:audio/mp3;base64,SUQzBAAAAAAAI1RTU0UAAAAPAAADTGF2ZjU4LjM4LjEwMAAAAAAAAAAAAAAA";
const DUMMY_MP4 = "data:video/mp4;base64,AAAAIGZ0eXBpc29tAAAAAGlzb21tcDQybXA0MQAAAAhmcmVlAAAAAG1kYXQ=";
const DUMMY_CSV = "data:text/csv;base64,aWQsbmFtZSxhZ2UsZGVwYXJ0bWVudCxzYWxhcnkKMSxBbGljZSwzMCxFbmdpbmVlcmluZyw5NTAwMAoyLEJvYiwyNSxNYXJrZXRpbmcsNzUwMDAKMyxDaGFybGllLDM1LFByb2R1Y3QsMTEwMDAwCjQsRGF2aWQsMjgsRGVzaWduLDg1MDAw"; 
const DUMMY_TXT = "data:text/plain;base64,U3lzdGVtIEFQSSBEZW1vCg== ";

(async () => {
  await fs.mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("[pageerror]", e.message));
  
  console.log("Loading Web IDE...");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

  console.log("Injecting dummy assets...");
  await page.evaluate((args) => {
    const R = window.RapidR;
    const proj = R.state.project;
    proj.assets = [
      { name: "image_logo.png", mime: "image/png", dataUrl: args.png },
      { name: "bg_music.mp3", mime: "audio/mp3", dataUrl: args.mp3 },
      { name: "intro_video.mp4", mime: "video/mp4", dataUrl: args.mp4 },
      { name: "employee_data.csv", mime: "text/csv", dataUrl: args.csv },
      { name: "readme.txt", mime: "text/plain", dataUrl: args.txt }
    ];
    R.renderActiveDesigner();
    R.renderProperties();
  }, { png: DUMMY_PNG, mp3: DUMMY_MP3, mp4: DUMMY_MP4, csv: DUMMY_CSV, txt: DUMMY_TXT });

  console.log("Opening Assets Manager...");
  await page.evaluate(() => window.RapidR.runCommand("asset.manage"));
  await page.waitForSelector("#premium-modal.open");

  console.log("Selecting employee_data.csv for preview...");
  await page.click('.assets-explorer-row:has-text("employee_data.csv")');
  await page.waitForSelector(".csv-table");
  
  await page.waitForTimeout(1000); // Wait for transitions

  console.log("Capturing Assets Explorer screenshot...");
  await page.screenshot({ path: path.join(OUT_DIR, "assets_explorer_premium.png") });
  
  console.log("Screenshot saved successfully!");
  await browser.close();
})();
