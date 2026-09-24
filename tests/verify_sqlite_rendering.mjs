import { chromium } from "playwright";

const URL_BASE = "http://localhost:8765";

(async () => {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  const page = await ctx.newPage();

  page.on("pageerror", e => console.log("PAGE_ERROR:", e.message));
  page.on("console", m => {
    if (m.type() === "error") {
      console.log("CONSOLE_ERROR:", m.text());
    }
  });

  console.log("Loading Web IDE...");
  await page.goto(URL_BASE + "/web-ide/index.html", { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

  console.log("Selecting SQLite Demo example...");
  await page.selectOption("#examples", "../examples/demo_sqlite.rr");
  await page.waitForTimeout(2000);

  // Check state.project structure
  const projectState = await page.evaluate(() => {
    return {
      formsCount: window.RapidR.state.project.forms.length,
      firstFormName: window.RapidR.state.project.forms[0]?.name,
      firstFormChildren: window.RapidR.state.project.forms[0]?.children.map(c => ({ name: c.name, type: c.type }))
    };
  });
  console.log("Project State from window.RapidR:", JSON.stringify(projectState, null, 2));

  // 1. Check if the visual editor shows the forms and widgets
  const formInDesigner = await page.evaluate(() => {
    const activePane = document.querySelector(".mdi-pane.active");
    if (!activePane) return { error: "No active pane found" };
    const formEl = activePane.querySelector(".design-form");
    if (!formEl) return { error: "No design-form found in active pane" };
    
    // Find all widgets inside the design-form
    const widgets = Array.from(formEl.querySelectorAll(".dwidget")).map(el => {
      return {
        name: el.dataset.name,
        type: el.dataset.type,
        caption: el.innerText.trim(),
        left: el.style.left,
        top: el.style.top
      };
    });
    const titleText = formEl.querySelector(".form-title-text")?.textContent;
    return {
      titleText,
      widgets
    };
  });

  console.log("Form in Designer (IDE Editor):", JSON.stringify(formInDesigner, null, 2));

  // 2. Check the Monaco editor content (Code view)
  console.log("Switching to Code View...");
  await page.evaluate(() => {
    // Click Code tab
    const tabs = Array.from(document.querySelectorAll("#mdi-tabs .mtab"));
    if (tabs.length > 0) {
      // Switch view to code
      window.RapidR.runCommand("view.code");
    }
  });
  await page.waitForTimeout(1000);

  const monacoCode = await page.evaluate(() => {
    const activePane = document.querySelector(".mdi-pane.active");
    if (!activePane) return "No active pane";
    // Get monaco editor content if exists
    const editorsMap = window.RapidR._editors;
    const activeFormId = window.RapidR.state.activeFormId;
    if (editorsMap && activeFormId && editorsMap.has(activeFormId)) {
      return editorsMap.get(activeFormId).getValue();
    }
    return "Monaco editor not found/not loaded";
  });

  console.log("Monaco Editor Code (first 300 chars):");
  console.log(monacoCode.substring(0, 300) + "...\n");

  // 3. Start preview runtime and inspect DOM inside iframe
  console.log("Starting the preview...");
  await page.click('#btn-run');
  await page.waitForTimeout(4000);

  const runtimeDOM = await page.evaluate(() => {
    const iframe = document.querySelector("#preview");
    if (!iframe) return { error: "No preview iframe found" };
    const doc = iframe.contentDocument;
    if (!doc) return { error: "No iframe contentDocument" };

    const titlebar = doc.querySelector(".form-titlebar")?.innerText;
    const menuItems = Array.from(doc.querySelectorAll(".rr-menu-item-top, .rr-menu-item-sub")).map(el => el.innerText.trim());
    const buttons = Array.from(doc.querySelectorAll("button.rr-widget")).map(el => el.innerText.trim());
    const labels = Array.from(doc.querySelectorAll("span.rr-widget")).map(el => el.innerText.trim());
    
    return {
      titlebar,
      menuItems,
      buttons,
      labels
    };
  });

  console.log("Runtime DOM (WASM Runtime):", JSON.stringify(runtimeDOM, null, 2));

  await browser.close();
})().catch(e => {
  console.error("Verification script failed:", e);
});
