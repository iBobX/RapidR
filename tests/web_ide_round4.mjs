// Round 4 tests:
//   1. Multi-form runtime — Principal + Usuarios with cross-form Show / Focus.
//      Reproduces: user added Show()/Focus() handlers across two forms and
//      the runtime then rendered Principal "with no buttons at all".
//   2. Color picker realtime — `input` event fires while picker is open and
//      the active designer reflects the new color immediately.
//   3. Color picker has explicit OK/dismiss button.
//   4. Font dropdown realtime — selecting a new font updates the designer.
//   5. Build pipeline — generated zip has a syntactically valid loader.js
//      whose import target (rapidrintr.js) actually exports `default`.
//
// Usage:  node tests/web_ide_round4.mjs
// (On the kernel host the preview's forms are windows the kernel draws: the
// test reads their accessibility mirror and clicks as the user does,
// tests/web_kernel_page.mjs.)

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { spawn } from "node:child_process";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as os from "node:os";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
function ok(cond, msg) {
  if (!cond) throw new Error("ASSERT FAILED: " + msg);
  console.log("✓ " + msg);
}

const browser = await chromium.launch();
const ctx = await browser.newContext({ acceptDownloads: true });
const page = await ctx.newPage();
const errors = [];
page.on("pageerror", e => errors.push(`[pageerror] ${e.message}`));
page.on("dialog", d => d.accept());
page.on("console", msg => { if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`); });

await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(
  () => document.getElementById("status")?.textContent?.length > 0,
  { timeout: 30000 }
);

// ─── 1. Multi-form runtime ──────────────────────────────────────────────
// Build a project model directly via state (avoids designer DnD flake).
await page.evaluate(() => {
  // Reset to a known one-form project, then mutate via the model API.
  document.querySelector('[data-cmd="project.new"]').click();
});
await page.waitForTimeout(120);
// Use the project.new + form.new path.
await page.evaluate(() => document.querySelector('.tree-group .tree-btn[data-cmd="form.new"]').click());
await page.waitForTimeout(120);

// Rename forms to Principal / Usuarios; add one button to each; bind handlers.
await page.evaluate(() => {
  const project = window.RapidR.state.project;
  project.forms[0].name = "Principal";
  project.forms[1].name = "Usuarios";
  // Button on Principal that shows Usuarios
  project.forms[0].children = [{
    type: "RButton", name: "BtnShowUsuarios",
    props: { caption: "Show Usuarios", left: 16, top: 16, width: 160, height: 32 },
    code: { handlers: { OnClick: "BtnShowUsuarios_Click" } },
  }];
  // Button on Usuarios that focuses Principal
  project.forms[1].children = [{
    type: "RButton", name: "BtnFocusPrincipal",
    props: { caption: "Focus Principal", left: 16, top: 16, width: 160, height: 32 },
    code: { handlers: { OnClick: "BtnFocusPrincipal_Click" } },
  }];
  project.forms[0].code = project.forms[0].code || {};
  project.forms[0].code.handlers = {};
  project.forms[0].code.source = `SUB BtnShowUsuarios_Click\n  Usuarios.Show()\nEND SUB\n`;
  project.forms[1].code = project.forms[1].code || {};
  project.forms[1].code.handlers = {};
  project.forms[1].code.source = `SUB BtnFocusPrincipal_Click\n  Principal.Focus()\nEND SUB\n`;
});

// Run and inspect the windows inside the preview iframe.
await page.evaluate(() => document.querySelector('[data-cmd="run.start"]').click());
await page.waitForTimeout(2500);
// The preview is cross-origin to the IDE (SEC-02), so inspect it through
// Playwright's frame API rather than contentDocument.
const previewFrame = page.frames().find(f => f.url().includes("preview.html"));
ok(!!previewFrame, "preview iframe mounted");
// (the startup form's window with its button, as its mirror has it; then a
// real click on it shows Usuarios, whose button focuses Principal again)
const shownForms = async () => (await k.windows(previewFrame)).map(w => w.form);
ok((await shownForms()).includes("principal") && (await k.text(previewFrame, "BtnShowUsuarios")) === "Show Usuarios",
   `Principal renders 'Show Usuarios' (windows=${await shownForms()}, button=${await k.text(previewFrame, "BtnShowUsuarios")})`);
await k.click(previewFrame, "BtnShowUsuarios");
await page.waitForTimeout(300);
ok((await shownForms()).at(-1) === "usuarios" && (await k.text(previewFrame, "BtnFocusPrincipal")) === "Focus Principal",
   `Usuarios.Show() shows Usuarios with 'Focus Principal' (windows=${await shownForms()}, button=${await k.text(previewFrame, "BtnFocusPrincipal")})`);
await k.click(previewFrame, "BtnFocusPrincipal");
await page.waitForTimeout(300);
ok(JSON.stringify((await shownForms()).slice(-2)) === '["usuarios","principal"]' && (await k.text(previewFrame, "BtnShowUsuarios")) === "Show Usuarios",
   `Principal.Focus() brings Principal to the front, its button still there (windows=${await shownForms()})`);
await page.evaluate(() => document.querySelector('[data-cmd="run.stop"]').click());

// ─── 2 + 3. Color picker realtime + OK button ──────────────────────────
await page.evaluate(() => document.querySelector('[data-cmd="project.new"]').click());
await page.waitForTimeout(150);
// Pick a color prop that's in the property panel for the current target.
// The form itself always has background-class color props.
await page.evaluate(() => {
  // Click on the form titlebar / empty area to select the form (no widget).
  const fb = document.querySelector(".mdi-pane.active .design-form");
  fb.click();
});
await page.waitForTimeout(120);
const colorRowKey = await page.evaluate(() => {
  const row = Array.from(document.querySelectorAll(".prop-row"))
    .find(r => r.querySelector('input[type="color"]'));
  return row ? row.dataset.key : null;
});
ok(!!colorRowKey, `found a color prop row (key=${colorRowKey})`);

await page.evaluate((key) => {
  const row = Array.from(document.querySelectorAll(".prop-row"))
    .find(r => r.dataset.key === key);
  const swatch = row.querySelector('input[type="color"]');
  swatch.value = "#ff0066";
  swatch.dispatchEvent(new Event("input", { bubbles: true }));
}, colorRowKey);
await page.waitForTimeout(150);
const formBg = await page.evaluate(() => {
  const f = document.querySelector(".mdi-pane.active .design-form");
  return f ? getComputedStyle(f).backgroundColor : null;
});
ok(formBg && formBg.includes("255, 0, 102"),
   `color realtime preview applied to form (got ${formBg})`);

// OK button must exist on color row.
const hasOk = await page.evaluate((key) => {
  const row = Array.from(document.querySelectorAll(".prop-row"))
    .find(r => r.dataset.key === key);
  return !!row?.querySelector(".prop-color-ok");
}, colorRowKey);
ok(hasOk, "color picker has OK/dismiss button");

// ─── 4. Font dropdown realtime ─────────────────────────────────────────
// Drop a button + select it so font props are visible.
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
  await page.waitForTimeout(120);
}
await dropWidget("RButton", 24, 24, 120, 32);
await page.click(".dwidget");
await page.waitForTimeout(120);
await page.evaluate(() => {
  const row = Array.from(document.querySelectorAll(".prop-row"))
    .find(r => r.dataset.key === "fontname");
  if (!row) throw new Error("no fontname row");
  const sel = row.querySelector("select");
  sel.value = "Courier New";
  sel.dispatchEvent(new Event("input", { bubbles: true }));
  sel.dispatchEvent(new Event("change", { bubbles: true }));
});
await page.waitForTimeout(150);
const ff = await page.evaluate(() => {
  const w = document.querySelector(".dwidget .dw-inner") || document.querySelector(".dwidget");
  return w ? w.style.fontFamily || getComputedStyle(w).fontFamily : null;
});
ok(ff && /courier/i.test(ff), `font realtime preview applied (got ${ff})`);

// ─── 5. Build zip → loader.js can resolve default export ───────────────
const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "rapidr-build-"));
const downloadPromise = page.waitForEvent("download");
await page.evaluate(() => document.querySelector('[data-cmd="run.build"]').click());
const dl = await downloadPromise;
const zipPath = path.join(tmpDir, "out.zip");
await dl.saveAs(zipPath);
const stat = await fs.stat(zipPath);
ok(stat.size > 50000, `zip is non-trivially sized (${stat.size} bytes)`);

// Unzip and inspect rapidrintr.js for the default export line.
const unzipDir = path.join(tmpDir, "unz");
await fs.mkdir(unzipDir, { recursive: true });
await new Promise((res, rej) => {
  const p = spawn("unzip", ["-q", "-o", zipPath, "-d", unzipDir], { stdio: "inherit" });
  p.on("exit", c => c === 0 ? res() : rej(new Error("unzip failed")));
});
const rapidrintrJs = await fs.readFile(path.join(unzipDir, "rapidrintr.js"), "utf8");
ok(rapidrintrJs.includes("__wbg_init as default"),
   "bundled rapidrintr.js contains `__wbg_init as default`");
const loaderJs = await fs.readFile(path.join(unzipDir, "loader.js"), "utf8");
ok(loaderJs.includes('import init, { rapidr_run_bc } from "./rapidrintr.js"'),
   "bundled loader.js imports init as default");

ok(errors.length === 0, `no page errors (got ${errors.length})`);
errors.forEach(e => console.warn("   " + e));

console.log("\nRound 4 suite: ALL CHECKS PASSED");
await browser.close();
