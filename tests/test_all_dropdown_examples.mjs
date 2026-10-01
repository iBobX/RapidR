import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = "http://localhost:8765";
const OUT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;

const EXAMPLES = [
  { name: "hello_web", optionValue: "../examples/hello_web.rr" },
  { name: "web_calculator", optionValue: "../examples/web_calculator.rr" },
  { name: "web_dashboard", optionValue: "../examples/web_dashboard.rr" },
  { name: "web_canvas", optionValue: "../examples/web_canvas.rr" },
  { name: "demo_plot", optionValue: "../examples/demo_plot.rr" },
  { name: "demo_sqlite", optionValue: "../examples/demo_sqlite.rr" },
  { name: "demo_num", optionValue: "../examples/demo_num.rr" },
  { name: "demo_dataframe", optionValue: "../examples/demo_dataframe.rr" },
  { name: "web_datascience", optionValue: "../examples/web_datascience.rr" },
  { name: "web_videoplayer", optionValue: "../examples/web_videoplayer.rr" },
  { name: "web_webviewer", optionValue: "../examples/web_webviewer.rr" },
  { name: "web_dom_js_demo", optionValue: "../examples/web_dom_js_demo.rr" },
  { name: "web_system_apis", optionValue: "../examples/web_system_apis.rr" }
];

(async () => {
  await fs.mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1000, height: 800 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("[pageerror]", e.message));
  
  console.log("Loading Web IDE...");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

  for (const ex of EXAMPLES) {
    console.log(`\n--- Testing Example: ${ex.name} ---`);
    
    // Select Option
    console.log(`Selecting option ${ex.optionValue}...`);
    await page.selectOption("#examples", ex.optionValue);
    await page.waitForTimeout(1000);

    // Start Preview
    console.log("Starting the preview...");
    await page.click('#btn-run');
    await page.waitForTimeout(4000); // Allow compile and boot time

    // Interactive actions for specific examples to demonstrate working functionality
    const frame = page.frameLocator("#preview");
    if (ex.name === "demo_plot") {
      console.log("Clicking 'Sine/Cosine' button inside plot preview...");
      await frame.locator("button:has-text('Sine/Cosine')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "web_calculator") {
      console.log("Clicking buttons '7', '*', '9', '=' inside calculator...");
      await frame.locator("button:has-text('7')").click();
      await frame.locator("button:has-text('*')").click();
      await frame.locator("button:has-text('9')").click();
      await frame.locator("button:has-text('=')").click();
      await page.waitForTimeout(500);
    } else if (ex.name === "web_canvas") {
      console.log("Drawing a line on the canvas...");
      const canvas = frame.locator("canvas");
      const box = await canvas.boundingBox();
      if (box) {
        await page.mouse.move(box.x + 100, box.y + 100);
        await page.mouse.down();
        await page.mouse.move(box.x + 300, box.y + 200, { steps: 5 });
        await page.mouse.up();
      }
      await page.waitForTimeout(1000);
    } else if (ex.name === "demo_sqlite") {
      console.log("Interacting with SQLite: Click 'Add User'...");
      await frame.locator("button:has-text('Add User')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "demo_num") {
      console.log("Interacting with NumPy: Click 'Basic'...");
      await frame.locator("button:has-text('Basic')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "demo_dataframe") {
      console.log("Interacting with DataFrame: Click 'Load Data'...");
      await frame.locator("button:has-text('Load Data')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "web_datascience") {
      console.log("Interacting with DataScience: Click 'Plot' tab then 'Line Chart'...");
      // (the tabs are drawn: a click where the "Plot" caption is)
      const plot = await frame.locator(".rr-tab-back text", { hasText: "Plot" }).boundingBox();
      await page.mouse.click(plot.x + plot.width / 2, plot.y + plot.height / 2);
      await page.waitForTimeout(500);
      await frame.locator("button:has-text('Line Chart')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "web_dom_js_demo") {
      console.log("Interacting with DOM & JS Showcase...");
      await frame.locator("button:has-text('Create DOM Element')").click();
      await frame.locator("button:has-text('Eval JS')").click();
      await page.waitForTimeout(1000);
    } else if (ex.name === "web_system_apis") {
      console.log("Interacting with System APIs Dashboard...");
      await frame.locator("button:has-text('Set Item')").click();
      await frame.locator("button:has-text('List Keys')").click();
      await frame.locator("button:has-text('Get Location')").click();
      await page.waitForTimeout(1000);
    }

    // Take Preview Screenshot
    console.log("Capturing preview screenshot...");
    const previewBox = page.locator("#preview-window");
    await previewBox.screenshot({ path: path.join(OUT_DIR, `${ex.name}_preview.png`) });

    // Close/Stop Preview using the preview-close button (or forcing stop button click)
    console.log("Stopping the preview...");
    await page.click('#preview-close');
    await page.waitForTimeout(1000);
  }

  await browser.close();
  console.log("\nAll examples screenshotted successfully!");
})().catch(e => {
  console.error("Test execution failed:", e);
  process.exit(1);
});
