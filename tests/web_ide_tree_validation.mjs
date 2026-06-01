// Playwright E2E test for Widget Tree Outline and Module Name Sanitization Validation.
//
// Usage:  node tests/web_ide_tree_validation.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const SHOTS = new URL("./web-screenshots/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

function ok(cond, msg) {
  if (!cond) throw new Error("ASSERT FAILED: " + msg);
  console.log("✓ " + msg);
}

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => errors.push(`[pageerror] ${e.message}`));
page.on("console", msg => {
  console.log(`[browser.${msg.type()}] ${msg.text()}`);
  if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`);
});

console.log(`→ ${URL_BASE}/web-ide/index.html`);
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.includes("ready"),
  { timeout: 15000 }
);

async function dropWidget(toolName, dx, dy, w, h) {
  await page.click(`.tool[data-tool="${toolName}"]`);
  const fb = await page.evaluate(() => {
    const el = document.querySelector(".mdi-pane.active .design-form");
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top };
  });
  await page.mouse.move(fb.x + dx, fb.y + dy);
  await page.mouse.down();
  await page.mouse.move(fb.x + dx + w, fb.y + dy + h, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(150);
}

try {
  // 1. Create a new blank project
  await page.click("#btn-new");
  await page.waitForSelector("#mdi-tabs .mtab.active");

  // 2. Drop standard widgets (RButton and RLabel)
  await dropWidget("RButton", 40, 40, 100, 30);
  await dropWidget("RLabel", 160, 40, 80, 20);

  // Wait for UI to update
  await page.waitForTimeout(300);

  // 3. Verify widgets appear in the project tree under Form1
  const treeSubItems = await page.evaluate(() => {
    const items = Array.from(document.querySelectorAll("#proj-tree .tree-sub-item"));
    return items.map(el => ({
      text: el.querySelector(".tree-label")?.textContent,
      icon: el.querySelector(".ico")?.textContent,
      classes: Array.from(el.classList)
    }));
  });

  ok(treeSubItems.length === 2, `Expected 2 child widgets in tree, got ${treeSubItems.length}`);
  ok(treeSubItems.some(item => item.text === "Button1" && item.icon === "▭"), "Button1 shown in tree with correct icon");
  ok(treeSubItems.some(item => item.text === "Label1" && item.icon === "A"), "Label1 shown in tree with correct icon");

  // 4. Verify selection sync (Click tree node -> updates designer and properties)
  const buttonTreeLocator = page.locator('#proj-tree .tree-sub-item:has-text("Button1")');
  await buttonTreeLocator.click();
  await page.waitForTimeout(200);

  const selectionState = await page.evaluate(() => {
    const formItem = Array.from(document.querySelectorAll("#proj-tree .tree-item")).find(x => x.textContent.includes("Form1"));
    const btnSubItem = Array.from(document.querySelectorAll("#proj-tree .tree-sub-item")).find(x => x.textContent.includes("Button1"));
    return {
      sel: window.RapidR.state.selection,
      formActive: formItem ? formItem.classList.contains("active") : false,
      btnActive: btnSubItem ? btnSubItem.classList.contains("active") : false
    };
  });

  ok(selectionState.sel.includes("Button1") && selectionState.sel.length === 1, "Button1 selected in state via tree click");
  ok(selectionState.btnActive, "Button1 tree node highlighted active");
  ok(!selectionState.formActive, "Parent Form1 tree node NOT highlighted active when child is selected");

  // 5. Verify double-click (Double-click tree node -> switches to code view and inserts default event handler stub)
  console.log("Triggering double-click on Button1 tree node...");
  await page.evaluate(() => {
    const el = Array.from(document.querySelectorAll("#proj-tree .tree-sub-item")).find(x => x.textContent.includes("Button1"));
    if (el) el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  });

  console.log("Waiting for Button1_Click handler stub to appear...");
  // Wait for event stub to be inserted asynchronously (Monaco editor loading takes some time)
  await page.waitForFunction(() => {
    const form = window.RapidR?.state?.project?.forms?.[0];
    return form?.code?.source?.includes("SUB Button1_Click");
  }, { timeout: 15000 });

  const viewState = await page.evaluate(() => {
    return {
      view: window.RapidR.state.activeView,
      activeTab: document.querySelector("#mdi-tabs .mtab.active")?.textContent
    };
  });

  ok(viewState.view === "code", `Expected view to be "code", got ${viewState.view}`);
  ok(viewState.activeTab?.includes("Code"), "Active tab renamed/marked as Code view");

  // Verify that an event stub was inserted into the editor source
  const codeContent = await page.evaluate(() => {
    const form = window.RapidR.state.project.forms[0];
    return form.code?.source;
  });
  ok(codeContent && codeContent.includes("SUB Button1_Click"), "Double click successfully inserted Button1_Click handler stub");

  // Switch back to designer
  await page.evaluate(() => window.RapidR.switchView("designer"));
  await page.waitForTimeout(200);

  // 6. Verify module creation with space name sanitization confirm flows
  await page.evaluate(() => window.RapidR.runCommand("module.new"));
  await page.waitForSelector(".ide-modal-overlay input[type=text]");
  // Fill input with invalid name (contains spaces)
  await page.fill(".ide-modal-overlay input[type=text]", "Test Module 2");
  await page.click(".ide-modal-overlay button.primary"); // Click OK on the prompt
  await page.waitForTimeout(200);

  // Verify confirmation dialog popped up offering the sanitized name
  const confirmBody = await page.locator(".ide-modal-overlay .ide-modal-body").innerHTML();
  ok(confirmBody.includes("Test Module 2") && confirmBody.includes("Test_Module_2"), "Sanitization confirm dialog shown offering 'Test_Module_2'");

  // Click "Use Sanitized" (it is the primary/second button in confirm dialog)
  const useSanitizedBtn = page.locator('.ide-modal-overlay button:has-text("Use Sanitized")');
  await useSanitizedBtn.click();
  await page.waitForTimeout(300);

  // Verify module created successfully with the sanitized name
  const finalModules = await page.evaluate(() => {
    return window.RapidR.state.project.modules.map(m => m.name);
  });
  ok(finalModules.includes("Test_Module_2"), `Sanitized module created successfully: ${finalModules.join(", ")}`);

  // Take screenshot for verification
  await page.screenshot({ path: SHOTS + "tree-validation.png", fullPage: true });

} catch (err) {
  console.error("Test failed with error:", err);
  process.exitCode = 1;
} finally {
  if (errors.length) {
    console.error("✗ console/page errors occurred:");
    for (const e of errors) console.error("  " + e);
    process.exitCode = 1;
  } else if (process.exitCode !== 1) {
    console.log("✓ all E2E validation checks passed successfully");
  }
  await browser.close();
  console.log(process.exitCode ? "FAIL" : "PASS");
}
