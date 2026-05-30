// E2E test for RapidR debugger: event handlers, breakpoints, stepping, variables, and stack.
// Usage: node tests/debug_e2e_event_handling.mjs

import { chromium } from "playwright";
import { join } from "node:path";
import { mkdirSync } from "node:fs";

const PORT = 8765;
const URL_BASE = `http://localhost:${PORT}`;
const SHOT_DIR = "/Users/roanbema/.gemini/antigravity/brain/3bde725b-d3cb-4df5-b6f5-8aa2d5c6b3d2/screenshots/";
mkdirSync(SHOT_DIR, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => {
  console.log(`[main pageerror] ${e.stack || e.message}`);
  errors.push(e.message);
});
page.on("console", msg => {
  console.log(`[main console] [${msg.type()}] ${msg.text()}`);
  if (msg.type() === "error") errors.push(msg.text());
});

try {
  console.log(`→ navigating to ${URL_BASE}/web-ide/index.html`);
  await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });

  await page.waitForFunction(
    () => document.getElementById("status")?.textContent?.includes("ready"),
    { timeout: 15000 }
  );
  console.log("✓ IDE booted successfully");

  // Create a new blank project
  console.log("→ creating a new project");
  await page.click("#btn-new");
  await page.waitForSelector("#mdi-tabs .mtab.active");

  // Drop RButton widget onto the designer
  console.log("→ dragging/dropping button on designer form");
  await page.click('.tool[data-tool="RButton"]');
  await page.click(".design-form", { position: { x: 80, y: 80 } });
  await page.waitForTimeout(500);

  // Switch to code view using F7 key
  console.log("→ switching to code view");
  await page.keyboard.press("F7");
  await page.waitForTimeout(500);

  // Inject Button1_Click event handler code into Monaco Editor
  console.log("→ injecting code in Monaco");
  await page.evaluate(() => {
    const form = window.RapidR.state.project.forms[0];
    const button = form.children.find(w => w.name === "Button1");
    button.code = button.code || {};
    button.code.handlers = button.code.handlers || {};
    button.code.handlers.OnClick = "Button1_Click";

    form.code = form.code || {};
    form.code.source = [
      "SUB Button1_Click",
      "  DIM x AS INTEGER",
      "  x = 100",
      "  x = x + 5",
      "  PRINT x",
      "END SUB"
    ].join("\n");
    const ed = window.RapidR._editors.get(form.id);
    if (ed) ed.setValue(form.code.source);
  });
  await page.waitForTimeout(500);

  // Set a breakpoint on line 3 (which is 'x = 100') by clicking the gutter
  console.log("→ clicking gutter to set breakpoint on line 3");
  const line3Gutter = page.locator(".monaco-editor .margin-view-overlays .line-numbers").nth(2);
  await line3Gutter.click();
  await page.waitForTimeout(500);

  // Verify breakpoint exists in RapidR state
  const bpExists = await page.evaluate(() => {
    const form = window.RapidR.state.project.forms[0];
    return window.RapidR.state.breakpoints.has(`${form.id}:3`);
  });
  console.log(`✓ Breakpoint in state: ${bpExists}`);
  if (!bpExists) {
    throw new Error("Breakpoint was not set in state upon clicking the gutter!");
  }

  // Take a screenshot of the designer/editor state with breakpoint
  await page.screenshot({ path: join(SHOT_DIR, "03_breakpoint_click.png"), fullPage: true });

  // Start Debug session
  console.log("→ starting debug session");
  await page.click("#btn-debug");
  await page.waitForTimeout(1000);

  // Verify debugger status and DOM classes
  const isDebugging = await page.evaluate(() => window.RapidR.state.isDebugging);
  const isPausedInitially = await page.evaluate(() => window.RapidR.state.isDebugPaused);
  console.log(`✓ Is debugging: ${isDebugging}, Is paused initially: ${isPausedInitially}`);
  if (!isDebugging || isPausedInitially) {
    throw new Error(`Invalid initial debug state. isDebugging=${isDebugging}, isPausedInitially=${isPausedInitially}`);
  }

  await page.screenshot({ path: join(SHOT_DIR, "04_debug_started.png"), fullPage: true });

  // Click the button in the app preview to fire the event handler
  console.log("→ clicking Button1 in the application preview");
  const previewFrame = page.frameLocator("#preview");
  const previewButton = previewFrame.locator("#rr-button1");
  await previewButton.click();
  await page.waitForTimeout(800);

  // Wait for the debugger to pause on the breakpoint (line 3)
  console.log("→ waiting for debugger to pause on breakpoint");
  await page.waitForFunction(
    () => window.RapidR.state.isDebugging && window.RapidR.state.isDebugPaused,
    { timeout: 5000 }
  );
  console.log("✓ Debugger successfully paused on button click event!");

  await page.screenshot({ path: join(SHOT_DIR, "05_breakpoint_hit.png"), fullPage: true });

  // Assert Call Stack contents
  const callStackHtml = await page.innerHTML("#debug-callstack");
  console.log("Call Stack content:", callStackHtml);
  if (!callStackHtml.includes("Button1_Click")) {
    throw new Error("Call stack does not show Button1_Click!");
  }

  // Assert variables view (x should be present in Locals, but uninitialized or 0 before execution of line 3)
  let varsHtml = await page.innerHTML("#debug-variables");
  console.log("Variables HTML (paused on line 3):", varsHtml);
  if (!varsHtml.includes("x")) {
    throw new Error("Variables panel does not show local variable 'x'!");
  }

  // Step Over the line "x = 100" (executing line 3, pausing on line 4)
  console.log("→ performing Step Over");
  await page.click("#btn-stepover");
  await page.waitForTimeout(500);
  await page.waitForFunction(
    () => window.RapidR.state.isDebugging && window.RapidR.state.isDebugPaused,
    { timeout: 5000 }
  );

  const currentLine = await page.evaluate(() => window.RapidR.state.currentPausedLineInFile);
  console.log(`✓ Currently paused on line: ${currentLine}`);
  if (currentLine !== 4) {
    throw new Error(`Expected debugger to be paused on line 4, but got line ${currentLine}`);
  }

  // Verify variables update: x should now be 100!
  varsHtml = await page.innerHTML("#debug-variables");
  console.log("Variables HTML (paused on line 4):", varsHtml);
  if (!varsHtml.includes("100")) {
    throw new Error("Local variable 'x' value did not update to 100!");
  }

  // Test Monaco context menu action: Add "x" to Watch list
  console.log("→ testing Monaco 'Add to Watch' context menu action");
  await page.evaluate(() => {
    const ed = window.RapidR._editors.get(window.RapidR.state.project.forms[0].id);
    // Set cursor on 'x' in 'x = x + 5' (line 4)
    ed.setPosition({ lineNumber: 4, column: 3 });
    // Trigger the context menu action
    ed.trigger('keyboard', 'rapidr-add-watch');
  });
  await page.waitForTimeout(500);

  const watchExists = await page.evaluate(() => window.RapidR.state.watchExpressions.includes("x"));
  console.log(`✓ 'x' in watchExpressions: ${watchExists}`);
  if (!watchExists) {
    throw new Error("Add to Watch context menu action failed to add 'x' to state!");
  }

  const watchHtml = await page.innerHTML("#debug-watch-list");
  console.log("Watch List HTML:", watchHtml);
  if (!watchHtml.includes("x") || !watchHtml.includes("100")) {
    throw new Error("Watch List panel does not render the watched variable 'x' with value 100!");
  }

  await page.screenshot({ path: join(SHOT_DIR, "06_watch_added_layout.png"), fullPage: true });

  // Step Over line 4 (executing "x = x + 5", pausing on line 5)
  console.log("→ performing Step Over to line 5");
  await page.click("#btn-stepover");
  await page.waitForTimeout(500);
  await page.waitForFunction(
    () => window.RapidR.state.isDebugging && window.RapidR.state.isDebugPaused,
    { timeout: 5000 }
  );

  const currentLineAfterStep = await page.evaluate(() => window.RapidR.state.currentPausedLineInFile);
  console.log(`✓ Currently paused on line: ${currentLineAfterStep}`);
  if (currentLineAfterStep !== 5) {
    throw new Error(`Expected debugger to be paused on line 5, but got line ${currentLineAfterStep}`);
  }

  // Verify variables update: x should now be 105!
  varsHtml = await page.innerHTML("#debug-variables");
  console.log("Variables HTML (paused on line 5):", varsHtml);
  if (!varsHtml.includes("105")) {
    throw new Error("Local variable 'x' value did not update to 105!");
  }

  // Click Continue to resume execution
  console.log("→ clicking Continue");
  await page.click("#btn-resume");
  await page.waitForTimeout(500);

  // Debugger should transition back to waiting state (running but not paused)
  const isPausedAfterResume = await page.evaluate(() => window.RapidR.state.isDebugPaused);
  console.log(`✓ Is paused after resume: ${isPausedAfterResume}`);
  if (isPausedAfterResume) {
    throw new Error("Debugger should not be paused after clicking continue!");
  }

  // Stop debugger session
  console.log("→ clicking Stop");
  await page.click(".tb.stop");
  await page.waitForTimeout(500);

  const isDebuggingAfterStop = await page.evaluate(() => window.RapidR.state.isDebugging);
  console.log(`✓ Is debugging after stop: ${isDebuggingAfterStop}`);
  if (isDebuggingAfterStop) {
    throw new Error("Debugger is still active after clicking Stop!");
  }

  console.log("✓ Debugger E2E flow test successfully passed!");

} catch (err) {
  console.error("✗ Debugger E2E flow test FAILED:", err);
  process.exitCode = 1;
} finally {
  await browser.close();
}
