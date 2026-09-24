// Undo/redo regression test for the web IDE project history.
//
// Drives the IDE the way a user does (toolbox clicks, mouse drags, property
// grid, keyboard, code editor) and checks that each action is one undo step,
// that redo re-applies it, that a new edit clears redo, that Ctrl+Z inside the
// code editor stays Monaco's text undo, and that New Project resets history.
//
// Usage:  node tests/web_ide_undo.mjs   (server on http://localhost:8765)

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const MOD = process.platform === "darwin" ? "Meta" : "Control";

let failed = 0;
function ok(cond, msg) {
  console.log(`${cond ? "✓" : "✗"} ${msg}`);
  if (!cond) failed++;
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));

await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });

const settle = () => page.waitForTimeout(600);  // > checkpoint debounce
const widgets = () => page.evaluate(() => window.RapidR.state.project.forms[0].children.map((w) => w.name));
const undoEnabled = () => page.evaluate(() => !document.querySelector('#toolbar [data-cmd="edit.undo"]').disabled);
const redoEnabled = () => page.evaluate(() => !document.querySelector('#toolbar [data-cmd="edit.redo"]').disabled);
const clickUndo = async () => { await page.click('#toolbar [data-cmd="edit.undo"]'); await settle(); };
const clickRedo = async () => { await page.click('#toolbar [data-cmd="edit.redo"]'); await settle(); };

ok(!(await undoEnabled()) && !(await redoEnabled()), "fresh IDE: undo and redo disabled");

// ── Add two widgets: each is one step ──
await page.click('.tool[data-tool="RButton"]');
await page.click(".design-form", { position: { x: 50, y: 50 } });
await settle();
await page.click('.tool[data-tool="RLabel"]');
await page.click(".design-form", { position: { x: 50, y: 120 } });
await settle();
ok(JSON.stringify(await widgets()) === '["Button1","Label1"]', "added Button1 and Label1");
ok(await undoEnabled(), "undo enabled after edits");

await clickUndo();
ok(JSON.stringify(await widgets()) === '["Button1"]', "undo removes Label1");
ok(await redoEnabled(), "redo enabled after undo");
await clickRedo();
ok(JSON.stringify(await widgets()) === '["Button1","Label1"]', "redo restores Label1");

// ── Drag Button1: the whole drag is a single step ──
const pos = () => page.evaluate(() => {
  const b = window.RapidR.state.project.forms[0].children.find((w) => w.name === "Button1");
  return { left: +b.props.left, top: +b.props.top };
});
const before = await pos();
const box = await page.locator('.mdi-pane.active .dwidget[data-name="Button1"]').boundingBox();
await page.mouse.move(box.x + 15, box.y + 10);
await page.mouse.down();
await page.mouse.move(box.x + 95, box.y + 70, { steps: 8 });
await page.mouse.up();
await settle();
const moved = await pos();
ok(moved.left !== before.left || moved.top !== before.top, `drag moved Button1 (${before.left},${before.top} → ${moved.left},${moved.top})`);
await clickUndo();
const back = await pos();
ok(back.left === before.left && back.top === before.top, "one undo reverts the entire drag");
await clickRedo();
const again = await pos();
ok(again.left === moved.left && again.top === moved.top, "redo re-applies the drag");

// ── Property grid edit ──
await page.click('.mdi-pane.active .dwidget[data-name="Button1"]');
const caption = page.locator('.prop-row[data-key="caption"] input');
await caption.fill("Greet");
await caption.press("Tab");
await settle();
const cap = () => page.evaluate(() => window.RapidR.state.project.forms[0].children.find((w) => w.name === "Button1").props.caption);
ok((await cap()) === "Greet", "caption edited to Greet");
// Keyboard undo (focus outside inputs/editor).
await page.click("#mdi-tabs");
await page.keyboard.press(`${MOD}+z`);
await settle();
ok((await cap()) !== "Greet", `Ctrl/Cmd+Z undoes the caption edit (now "${await cap()}")`);
await page.keyboard.press(`${MOD}+Shift+z`);
await settle();
ok((await cap()) === "Greet", "Ctrl/Cmd+Shift+Z redoes it");

// ── A new edit after undo clears redo ──
await clickUndo();
ok(await redoEnabled(), "redo available after undo");
await page.evaluate(() => window.RapidR.runCommand("form.new"));
await settle();
ok(!(await redoEnabled()), "new edit (Add Form) clears redo");
ok((await page.evaluate(() => window.RapidR.state.project.forms.length)) === 2, "Add Form created Form2");
await clickUndo();
ok((await page.evaluate(() => window.RapidR.state.project.forms.length)) === 1, "undo removes Form2");

// ── Delete key on a selected widget, then undo ──
await page.click('.mdi-pane.active .dwidget[data-name="Label1"]');
await page.keyboard.press("Delete");
await settle();
ok(!(await widgets()).includes("Label1"), "Delete removed Label1");
await clickUndo();
ok((await widgets()).includes("Label1"), "undo restores deleted Label1");

// ── Code editor: typing is a project step; Ctrl+Z inside the editor stays text undo ──
await page.evaluate(() => window.RapidR.runCommand("view.code"));
await page.waitForSelector(".mdi-pane.active .monaco-editor", { timeout: 15000 });
const src = () => page.evaluate(() => window.RapidR.state.project.forms[0].code.source);
const srcBefore = await src();
await page.click(".mdi-pane.active .monaco-editor .view-lines");
await page.keyboard.press(`${MOD}+End`);
await page.keyboard.type("\n' undo-marker");
await settle();
ok((await src()).includes("undo-marker"), "typed text reached the model");
const formsBeforeEditorUndo = await page.evaluate(() => window.RapidR.state.project.forms.length);
await page.keyboard.press(`${MOD}+z`);  // focus is in Monaco
await settle();
ok(!(await src()).includes("undo-marker"), "Ctrl/Cmd+Z in the editor performed a text undo");
ok((await page.evaluate(() => window.RapidR.state.project.forms.length)) === formsBeforeEditorUndo &&
   (await widgets()).includes("Label1"),
   "editor Ctrl/Cmd+Z did not trigger a project-level undo");

// Retype, then undo from the toolbar: the code change is restored in the editor too.
await page.keyboard.type("\n' toolbar-marker");
await settle();
ok((await src()).includes("toolbar-marker"), "second edit reached the model");
await clickUndo();
ok(!(await src()).includes("toolbar-marker"), "toolbar undo reverts code typed in the editor");
const editorText = await page.evaluate(() => document.querySelector(".mdi-pane.active .monaco-editor .view-lines")?.innerText || "");
ok(!editorText.includes("toolbar-marker"), "editor view shows the restored code");

// ── New Project resets history ──
await page.evaluate(() => window.RapidR.runCommand("project.new"));
await settle();
ok(!(await undoEnabled()) && !(await redoEnabled()), "New Project starts with empty history");

ok(pageErrors.length === 0, `no page errors (got ${pageErrors.length}${pageErrors.length ? ": " + pageErrors[0] : ""})`);
await browser.close();
if (failed) {
  console.log(`\nUndo/redo: ${failed} CHECK(S) FAILED`);
  process.exit(1);
}
console.log("\nUndo/redo: ALL CHECKS PASSED");
