import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

function ok(cond, msg) {
  if (!cond) throw new Error("ASSERT FAILED: " + msg);
  console.log("✓ " + msg);
}

// Dummy base64 values
const DUMMY_PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgAAIAAAUAAeImBZsAAAAASUVORK5CYII=";
const DUMMY_MP3 = "data:audio/mp3;base64,SUQzBAAAAAAAI1RTU0UAAAAPAAADTGF2ZjU4LjM4LjEwMAAAAAAAAAAAAAAA";
const DUMMY_MP4 = "data:video/mp4;base64,AAAAIGZ0eXBpc29tAAAAAGlzb21tcDQybXA0MQAAAAhmcmVlAAAAAG1kYXQ=";
const DUMMY_CSV = "data:text/csv;base64,aWQsbmFtZSxhZ2UKMSxBbGljZSwzMAoyLEJvYiwyNQo="; // id,name,age\n1,Alice,30\n2,Bob,25
const DUMMY_TXT = "data:text/plain;base64,SGVsbG8gV29ybGQ="; // Hello World

async function main() {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({
    acceptDownloads: true,
    viewport: { width: 1400, height: 900 },
  });
  
  try {
    const page = await ctx.newPage();
    page.on("console", m => {
      if (m.type() === "error") console.log("[page-error]", m.text());
    });
    
    await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "networkidle" });
    await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });
    
    console.log("Injecting test assets and test widget...");
    await page.evaluate((args) => {
      const R = window.RapidR;
      const proj = R.state.project;
      proj.assets = [
        { name: "image.png", mime: "image/png", dataUrl: args.png },
        { name: "audio.mp3", mime: "audio/mp3", dataUrl: args.mp3 },
        { name: "video.mp4", mime: "video/mp4", dataUrl: args.mp4 },
        { name: "data.csv", mime: "text/csv", dataUrl: args.csv },
        { name: "hello.txt", mime: "text/plain", dataUrl: args.txt }
      ];
      
      // Add a widget to verify reference syncing
      const form = proj.forms[0];
      form.children.push({
        name: "Image1",
        type: "RImage",
        props: { left: 20, top: 20, width: 64, height: 64, picture: "assets/image.png" },
        code: { handlers: {} }
      });
      R.state.selection = ["Image1"];
      R.renderActiveDesigner();
      R.renderProperties();
    }, { png: DUMMY_PNG, mp3: DUMMY_MP3, mp4: DUMMY_MP4, csv: DUMMY_CSV, txt: DUMMY_TXT });

    // 1. Open the Assets Explorer Modal
    console.log("Opening Assets Manager...");
    await page.evaluate(() => window.RapidR.runCommand("asset.manage"));
    
    // Verify modal is open and has wide layout
    const modalSelector = "#premium-modal";
    await page.waitForSelector(`${modalSelector}.open`);
    const isWide = await page.evaluate(() => {
      const content = document.querySelector("#premium-modal .premium-modal-content");
      return content.classList.contains("premium-modal-content-wide");
    });
    ok(isWide, "Modal content has wide styling (.premium-modal-content-wide)");
    
    const titleText = await page.textContent("#premium-modal-title");
    ok(titleText.includes("Assets Manager"), `Title is correct: ${titleText}`);

    // Verify 5 items listed in the sidebar
    let rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 5, `Listed 5 assets initially (found ${rowsCount})`);

    // 2. Test Search Filter
    console.log("Testing Search input...");
    await page.fill("#assets-search-input", "csv");
    rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 1, `Listed 1 asset after searching for 'csv' (found ${rowsCount})`);
    
    const rowName = await page.textContent(".assets-row-name");
    ok(rowName === "data.csv", `Filtered row is data.csv (got ${rowName})`);

    // Clear search
    await page.fill("#assets-search-input", "");
    rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 5, "Cleared search, restored 5 items");

    // 3. Test Type Filter Tabs
    console.log("Testing Category tabs...");
    await page.click('.assets-tab-btn[data-type="audio"]');
    rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 1, `Only 1 asset listed under Audio tab (found ${rowsCount})`);
    
    let activeTabName = await page.textContent(".assets-row-name");
    ok(activeTabName === "audio.mp3", `Listed audio asset is audio.mp3`);

    // Restore to All
    await page.click('.assets-tab-btn[data-type="all"]');
    rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 5, "Restored All tab, listed 5 items");

    // 4. Test CSV Table Preview rendering
    console.log("Testing CSV Table preview...");
    await page.click('.assets-explorer-row:has-text("data.csv")');
    await page.waitForSelector(".csv-table");
    
    const csvHeaders = await page.locator(".csv-table th").allTextContents();
    ok(csvHeaders.join(",") === "id,name,age", `CSV Headers parse correctly: ${csvHeaders.join(",")}`);
    
    const cellVal = await page.textContent('.csv-table td:has-text("Alice")');
    ok(cellVal === "Alice", "CSV body rows are rendered in table layout");

    // 5. Test Text preview rendering
    console.log("Testing Text file preview...");
    await page.click('.assets-explorer-row:has-text("hello.txt")');
    await page.waitForSelector(".text-preview-code");
    const preText = await page.textContent(".text-preview-code");
    ok(preText === "Hello World", `Text preview renders decoded payload: ${preText}`);

    // 6. Test Image preview rendering
    console.log("Testing Image preview...");
    await page.click('.assets-explorer-row:has-text("image.png")');
    await page.waitForSelector(".preview-img");
    const imgSrc = await page.getAttribute(".preview-img", "src");
    ok(imgSrc.startsWith("data:image/png;base64,"), "Image preview renders dataUrl");

    // 7. Test Rename & Reference Sync
    console.log("Testing Rename and reference sync...");
    await page.fill("#asset-rename-input", "logo_new.png");
    await page.click("#asset-rename-btn");
    
    // Verify name updated in sidebar list
    await page.waitForSelector('.assets-explorer-row:has-text("logo_new.png")');
    console.log("Rename succeeded in sidebar");

    // Verify widget picture property was automatically updated from assets/image.png to assets/logo_new.png
    const updatedProp = await page.evaluate(() => {
      const R = window.RapidR;
      const form = R.state.project.forms[0];
      const w = form.children.find(c => c.name === "Image1");
      return w ? w.props.picture : null;
    });
    ok(updatedProp === "assets/logo_new.png", `Reference synced: Image1.props.picture is ${updatedProp}`);

    // 8. Test Delete
    console.log("Testing Deletion...");
    // Select audio.mp3
    await page.click('.assets-explorer-row:has-text("audio.mp3")');
    // Click Delete, dismiss confirmation box
    page.once("dialog", d => d.accept());
    await page.click("#asset-delete-btn");
    
    // Verify item deleted
    await page.waitForTimeout(500);
    rowsCount = await page.locator(".assets-explorer-row").count();
    ok(rowsCount === 4, `List reduced to 4 assets after deletion (found ${rowsCount})`);

    // Close Explorer
    await page.click("#asset-close-btn");
    await page.waitForSelector(`${modalSelector}.open`, { state: "detached" });
    console.log("Assets Explorer closed");

    // 9. Test Selector Mode (Real-time Preview & Revert)
    console.log("Testing Selector Mode (real-time previews & revert)...");
    
    // Global tracking to verify callback invocations
    await page.evaluate(() => {
      window.selectorHistory = [];
      window.RapidR.openAssetModal("assets/logo_new.png", (val) => {
        window.selectorHistory.push(val);
      });
    });
    
    await page.waitForSelector(`${modalSelector}.open`);
    
    // Click on video.mp4 in sidebar
    await page.click('.assets-explorer-row:has-text("video.mp4")');
    
    let history = await page.evaluate(() => window.selectorHistory);
    ok(history[history.length - 1] === "assets/video.mp4", `Real-time callback invoked on click: ${history[history.length - 1]}`);

    // Dismiss modal (simulation of cancel click)
    await page.click('.premium-modal-close');
    await page.waitForSelector(`${modalSelector}.open`, { state: "detached" });
    
    history = await page.evaluate(() => window.selectorHistory);
    ok(history[history.length - 1] === "assets/logo_new.png", `Reverted to initial value on cancel: ${history[history.length - 1]}`);
    console.log("Cancel/Revert test passed");

    // Test Confirm Select
    await page.evaluate(() => {
      window.selectorHistory = [];
      window.RapidR.openAssetModal("assets/logo_new.png", (val) => {
        window.selectorHistory.push(val);
      });
    });
    await page.waitForSelector(`${modalSelector}.open`);
    await page.click('.assets-explorer-row:has-text("video.mp4")');
    await page.click("#asset-select-btn");
    await page.waitForSelector(`${modalSelector}.open`, { state: "detached" });
    
    history = await page.evaluate(() => window.selectorHistory);
    ok(history.length === 2 && history[1] === "assets/video.mp4", `Select confirmed. Callback final value: ${history[1]}`);

    console.log("\nALL ASSETS EXPLORER INTERACTIVE TESTS PASSED SUCCESSFULLY!");

  } finally {
    await browser.close();
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
