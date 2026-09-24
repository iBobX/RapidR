// Visual smoke — drive the real IDE in a headed browser, snapshot every
// significant state, and dump them to /tmp/rapidr-shots/.
import { chromium } from "playwright";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const OUT = "/tmp/rapidr-shots";

function tinyPng() {
  // 1x1 red PNG
  return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
}

async function shot(page, name) {
  const p = path.join(OUT, name + ".png");
  await page.screenshot({ path: p, fullPage: false });
  console.log("→", p);
}

(async () => {
  await fs.mkdir(OUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });    // headed not strictly needed for screenshots
  const ctx = await browser.newContext({ viewport: { width: 1500, height: 950 } });
  const page = await ctx.newPage();
  page.on("pageerror", e => console.log("[pageerror]", e.message));
  page.on("console", m => { if (m.type() === "error") console.log("[console.error]", m.text()); });

  console.log("1. boot IDE");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });
  await shot(page, "01-boot");

  console.log("2. add a button + label via toolbox");
  await page.evaluate(() => {
    const R = window.RapidR;
    const f = R.state.project.forms[0];
    f.props.caption = "Hello 1.0";
    f.children = [
      { name: "Label1",  type: "RLabel",  props: { left: 20, top: 20, width: 240, height: 20, caption: "Click below" }, code: { handlers: {} } },
      { name: "Button1", type: "RButton", props: { left: 20, top: 50, width: 100, height: 26, caption: "Greet" },        code: { handlers: { OnClick: "Button1_OnClick" } } },
    ];
    f.code = { handlers: {}, source: 'SUB Button1_OnClick\n  Label1.caption = "Hello, RapidR 1.0!"\nEND SUB\n' };
    R.renderProjectTree();
    R.renderActiveDesigner();
    R.renderProperties();
  });
  await page.waitForTimeout(500);
  await shot(page, "02-designer-with-button");

  console.log("3. open Properties for the button");
  await page.evaluate(() => {
    const R = window.RapidR;
    R.state.selectedWidgetName = "Button1";
    R.renderProperties();
  });
  await page.waitForTimeout(200);
  await shot(page, "03-properties-button");

  console.log("4. flip to Code tab");
  await page.evaluate(() => window.RapidR.switchView && window.RapidR.switchView("code"));
  await page.waitForTimeout(400);
  await shot(page, "04-code-view");

  console.log("5. inject a fake asset and confirm dropdown shows it");
  await page.evaluate((dataUrl) => {
    const R = window.RapidR;
    R.state.project.assets = [{ name: "logo.png", mime: "image/png", dataUrl }];
    R.state.project.forms[0].children.push({
      name: "Image1", type: "RImage",
      props: { left: 140, top: 50, width: 64, height: 64, picture: "assets/logo.png" },
      code: { handlers: {} },
    });
    R.state.selectedWidgetName = "Image1";
    R.switchView && R.switchView("design");
    R.renderActiveDesigner();
    R.renderProperties();
  }, tinyPng());
  await page.waitForTimeout(400);
  await shot(page, "05-asset-pipeline");

  console.log("6. open About dialog");
  await page.evaluate(() => window.RapidR.runCommand("help.about"));
  await page.waitForTimeout(300);
  await shot(page, "06-about-dialog");

  console.log("7. close dialog, run the program");
  await page.locator(".ide-modal-overlay button").last().click().catch(() => {});
  await page.waitForTimeout(200);
  await page.evaluate(() => window.RapidR.runCommand("run.start"));
  await page.waitForTimeout(2000);   // let the bytecode boot in the iframe
  await shot(page, "07-running-preview");

  console.log("8. click the Greet button inside the preview iframe");
  const frame = page.frameLocator("#preview");
  await frame.locator('button:has-text("Greet")').first().click({ timeout: 5000 }).catch(e => console.log("click err:", e.message));
  await page.waitForTimeout(400);
  await shot(page, "08-after-click");

  await browser.close();
  console.log("done");
})().catch(e => { console.error(e); process.exit(1); });
