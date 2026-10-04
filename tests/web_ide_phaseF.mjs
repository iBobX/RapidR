// Phase F smoke test: multi-form project + .rrproj save/load round-trip.
//
// Verifies:
//   1. Toolbox drag-create places a widget on the active form.
//   2. The "+" button in the Forms tree adds a second form (new tab).
//   3. Each form keeps its own widgets when switching tabs.
//   4. Saving the project produces a JSON .rrproj capturing both forms.
//   5. project.new resets to a single empty form.
//   6. Loading the captured .rrproj restores Form1 + Form2 with widgets.
//   7. Running the restored project mounts both forms in the runtime iframe.
//
// Usage:  node tests/web_ide_phaseF.mjs    (server on http://localhost:8765)

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

function ok(cond, msg) {
  if (!cond) throw new Error("ASSERT FAILED: " + msg);
  console.log("✓ " + msg);
}

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const errors = [];
page.on("pageerror", e => errors.push(`[pageerror] ${e.message}`));
page.on("dialog", d => d.accept());
page.on("console", msg => { if (msg.type() === "error") errors.push(`[console.error] ${msg.text()}`); });
page.on("requestfailed", r => errors.push(`[requestfailed] ${r.url()} ${r.failure()?.errorText}`));

await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
try {
  await page.waitForFunction(
    () => document.getElementById("status")?.textContent?.length > 0,
    { timeout: 30000 }
  );
} catch (e) {
  const status = await page.evaluate(() => document.getElementById("status")?.textContent);
  console.error("status never reached ready. Last status:", status);
  errors.forEach(x => console.error("  " + x));
  throw e;
}

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

// 1) Drop a Button on Form1
await dropWidget("RButton", 24, 24, 120, 32);
let widgets = await page.evaluate(() =>
  Array.from(document.querySelectorAll(".mdi-pane.active .dwidget")).map(e => e.dataset.name)
);
ok(widgets.length === 1 && widgets[0] === "Button1", "Form1 has Button1");

// 2) Add Form2
await page.click('.tree-group .tree-btn[data-cmd="form.new"]');
await page.waitForTimeout(150);
const tabs1 = await page.$$eval("#mdi-tabs .mtab-name", els => els.map(e => e.textContent));
ok(tabs1.length === 2 && tabs1[1].startsWith("Form2"), "Form2 tab created");

// 3) Drop a Label on Form2 (now active)
await dropWidget("RLabel", 32, 32, 168, 24);
widgets = await page.evaluate(() =>
  Array.from(document.querySelectorAll(".mdi-pane.active .dwidget")).map(e => e.dataset.name)
);
ok(widgets.length === 1 && widgets[0] === "Label1", "Form2 has Label1");

// 4) Capture saved JSON via createObjectURL hook
await page.evaluate(() => {
  window.__lastBlob = null;
  const orig = URL.createObjectURL;
  URL.createObjectURL = function (b) { window.__lastBlob = b; return orig.call(URL, b); };
});
await page.evaluate(() => document.querySelector('[data-cmd="project.save"]').click());
await page.waitForTimeout(200);
const saved = await page.evaluate(async () => JSON.parse(await window.__lastBlob.text()));
ok(saved.rapidr_project === 1, "rrproj has rapidr_project: 1");
ok(saved.forms.length === 2, "rrproj has 2 forms");
ok(saved.forms[0].name === "Form1" && saved.forms[0].children[0].name === "Button1", "saved Form1.Button1");
ok(saved.forms[1].name === "Form2" && saved.forms[1].children[0].name === "Label1", "saved Form2.Label1");

// 5) project.new wipes back to one empty form
await page.evaluate(() => document.querySelector('[data-cmd="project.new"]').click());
await page.waitForTimeout(150);
const cleanForms = await page.$$eval("#proj-tree .tree-item:not(.tree-sub-item) .tree-label", els => els.map(e => e.textContent));
ok(cleanForms.length === 1, "project.new resets to 1 form");

// 6) Load saved JSON back
await page.evaluate((j) => {
  const input = document.getElementById("file-open-project");
  const dt = new DataTransfer();
  dt.items.add(new File([JSON.stringify(j)], "roundtrip.rrproj", { type: "application/json" }));
  input.files = dt.files;
  input.dispatchEvent(new Event("change", { bubbles: true }));
}, saved);
await page.waitForTimeout(400);
const restored = await page.evaluate(() => ({
  forms: Array.from(document.querySelectorAll("#proj-tree .tree-item:not(.tree-sub-item) .tree-label")).map(e => e.textContent),
  tabs: Array.from(document.querySelectorAll("#mdi-tabs .mtab-name")).map(e => e.textContent),
}));
ok(restored.forms.join(",") === "Form1,Form2", "restored Form1,Form2 in tree");
ok(restored.tabs.length === 2, "restored 2 tabs");

// 7) Run: both forms exist; the startup form shows (Form1.ShowModal), Form2
// waits until the program shows it — as on the desktop and in RapidQ.
await page.evaluate(() => document.querySelector('[data-cmd="run.start"]').click());
await page.waitForTimeout(2500);
// The preview is cross-origin to the IDE (SEC-02); use Playwright's frame API.
const previewFrame = page.frames().find(f => f.url().includes("preview.html"));
const runtime = previewFrame
  ? await previewFrame.evaluate(() => ({ text: document.body?.innerText || "", form2: !!document.getElementById("rr-form2"), label1: !!document.getElementById("rr-label1") }))
  : { text: "", form2: false, label1: false };
ok(runtime.text.includes("Button1") && !runtime.text.includes("Label1") && runtime.form2 && runtime.label1,
   "runtime shows Form1 (Button1); Form2 (Label1) created, hidden until shown");
await page.evaluate(() => document.querySelector('[data-cmd="run.stop"]').click());

ok(errors.length === 0, `no page errors (got ${errors.length})`);
errors.forEach(e => console.warn("   " + e));

console.log("\nPhase F: ALL CHECKS PASSED");
await browser.close();
