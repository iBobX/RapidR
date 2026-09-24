import { chromium } from "playwright";
import { join } from "node:path";
import { mkdirSync } from "node:fs";

const PORT = 8765;
const URL_BASE = `http://localhost:${PORT}`;
const SHOT_DIR = process.env.RAPIDR_SHOT_DIR || new URL("./screenshots/", import.meta.url).pathname;
mkdirSync(SHOT_DIR, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

page.on("pageerror", e => {
  console.log(`[main pageerror] ${e.stack || e.message}`);
});
page.on("console", msg => {
  console.log(`[main console] [${msg.type()}] ${msg.text()}`);
});

console.log(`→ navigating to ${URL_BASE}/web-ide/index.html`);
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });

await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.includes("ready"),
  { timeout: 15000 }
);

console.log("→ entering user code");
await page.keyboard.press("F7");
await page.waitForTimeout(500);

await page.evaluate(() => {
  const form = window.RapidR.state.project.forms[0];
  form.code = form.code || {};
  form.code.source = [
    "' Code-behind for Form1",
    "' Add SUB handlers below; designer-defined widgets are auto-emitted.",
    "",
    "SUB Button1_Click",
    "  MESSAGEBOX(\"Robert\", \"Blah\", 0)",
    "END SUB"
  ].join("\n");
  const ed = window.RapidR._editors.get(form.id);
  if (ed) ed.setValue(form.code.source);
});
await page.waitForTimeout(500);

// Click gutter to set a breakpoint
console.log("→ attempting gutter click to set breakpoint on line 5");
const line5Gutter = page.locator(".monaco-editor .margin-view-overlays .line-numbers").nth(4);
await line5Gutter.click();
await page.waitForTimeout(500);

// Click Debug
console.log("→ clicking debug");
await page.click("#btn-debug");
await page.waitForTimeout(1000);

// Log computed styles of #preview-window
const DOMState = await page.evaluate(() => {
  const bodyClasses = document.body.className;
  const win = document.getElementById("preview-window");
  const style = window.getComputedStyle(win);
  return {
    bodyClasses,
    hidden: win.hidden,
    position: style.position,
    top: style.top,
    left: style.left,
    bottom: style.bottom,
    right: style.right,
    width: style.width,
    height: style.height,
    zIndex: style.zIndex,
    display: style.display,
  };
});
console.log("Computed DOM state:", DOMState);

await page.screenshot({ path: join(SHOT_DIR, "04_debug_started.png"), fullPage: true });

await browser.close();
