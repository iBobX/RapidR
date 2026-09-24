// Bug-fix smoke test for the issues reported by the user:
//   1. Move-on-first-click works after dropping a widget.
//   2. Real DOM components in the designer (button, input, label…).
//   3. Live property updates reflect on the preview (font-size + background).
//   4. Per-component event lists on dblclick (not global).
//   5. Copy / paste duplicates the selected widget (offset).
//   6. Help → About dialog shows credits + license button.
//   7. View → Full Source dialog shows serialized program.
//   8. Modules tree group + add module + module tab.
//   9. Built .zip uses current year (not 1980) for file dates.
//  10. Runtime iframe has no transparent black backdrop overlay.
//
// Usage:  node tests/web_ide_bugfixes.mjs    (server on http://localhost:8765)

import { chromium } from "playwright";

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

// Hook URL.createObjectURL so we can grab the built .zip blob.
await page.evaluate(() => {
  const origCreate = URL.createObjectURL;
  URL.createObjectURL = function (b) {
    if (b && b.size) {
      window.__lastBlob = b;
      window.__lastBlobType = b.type;
    }
    return origCreate.call(this, b);
  };
});

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

// ── 1 + 2: Drop a Button, verify a real <button> is rendered ────
await dropWidget("RButton", 30, 30, 110, 30);
const btnHtml = await page.evaluate(() => {
  const w = document.querySelector(".mdi-pane.active .design-form .dwidget");
  return { tag: w?.firstElementChild?.tagName, txt: w?.textContent };
});
ok(btnHtml.tag === "BUTTON", `RButton renders <button>, got <${btnHtml.tag}>`);
ok(btnHtml.txt?.includes("Button1"), "button caption shows widget name");

// ── 1: Click + drag the freshly placed button — should move on first try ──
const before = await page.evaluate(() => {
  const w = document.querySelector(".mdi-pane.active .design-form .dwidget");
  const r = w.getBoundingClientRect();
  return { x: r.left, y: r.top };
});
await page.mouse.move(before.x + 20, before.y + 12);
await page.mouse.down();
await page.mouse.move(before.x + 80, before.y + 60, { steps: 6 });
await page.mouse.up();
await page.waitForTimeout(120);
const after = await page.evaluate(() => {
  const w = document.querySelector(".mdi-pane.active .design-form .dwidget");
  const r = w.getBoundingClientRect();
  return { x: r.left, y: r.top };
});
ok(Math.abs(after.x - before.x) > 30 || Math.abs(after.y - before.y) > 30,
   `widget moved on first click (Δx=${(after.x-before.x).toFixed(0)}, Δy=${(after.y-before.y).toFixed(0)})`);

// ── 3: Live property update — change font-size, verify reflected in DOM ──
await page.evaluate(() => {
  const f = window.RapidR?.state?.project?.forms?.[0];
  const w = f?.children?.[0];
  w.props.fontsize = 28;
  w.props.background = "#ffcc00";
  window.RapidR.renderActiveDesigner();
});
await page.waitForTimeout(50);
const liveStyle = await page.evaluate(() => {
  const inner = document.querySelector(".mdi-pane.active .design-form .dwidget > *");
  const cs = inner ? getComputedStyle(inner) : null;
  return {
    fontSize: cs?.fontSize,
    bg: cs?.backgroundColor,
  };
});
ok(parseFloat(liveStyle.fontSize) >= 24, `font-size live preview = ${liveStyle.fontSize}`);
ok(/255,\s*204,\s*0/.test(liveStyle.bg), `background live preview = ${liveStyle.bg}`);

// ── 4: Per-component events on dblclick ──
// Drop a CheckBox so we have a different component type
await dropWidget("RCheckBox", 200, 30, 90, 22);
// Double-click the checkbox to switch to code view
const cbBox = await page.evaluate(() => {
  const ws = document.querySelectorAll(".mdi-pane.active .design-form .dwidget");
  const cb = ws[ws.length - 1];
  const r = cb.getBoundingClientRect();
  return { x: r.left + 20, y: r.top + 10 };
});
await page.mouse.dblclick(cbBox.x, cbBox.y);
await page.waitForFunction(() => {
  const sel = document.querySelector(".mdi-pane.active .evt-dropdown");
  return sel && sel.options.length > 1;
}, { timeout: 5000 });
const cbEvents = await page.evaluate(() => {
  const sel = document.querySelector(".mdi-pane.active .evt-dropdown");
  return sel ? Array.from(sel.options).map(o => o.value) : [];
});
ok(cbEvents.includes("OnClick") || cbEvents.includes("OnChange"),
   `checkbox event list populated (${cbEvents.length} options)`);

// ── 5: Copy / paste ──
// Switch back to designer, select first widget, copy, paste.
await page.evaluate(() => {
  window.RapidR.switchView?.("designer");
  const f = window.RapidR.state.project.forms[0];
  window.RapidR.state.selection = [f.children[0].name];
  window.RapidR.renderActiveDesigner();
});
await page.waitForTimeout(80);
const beforeCount = await page.evaluate(
  () => window.RapidR.state.project.forms[0].children.length);
// Trigger Edit → Copy → Paste via runCommand
await page.evaluate(() => window.RapidR.runCommand("edit.copy"));
await page.evaluate(() => window.RapidR.runCommand("edit.paste"));
await page.waitForTimeout(80);
const afterCount = await page.evaluate(
  () => window.RapidR.state.project.forms[0].children.length);
ok(afterCount === beforeCount + 1, `paste added 1 widget (${beforeCount} → ${afterCount})`);

// ── 6: About dialog ──
await page.evaluate(() => window.RapidR.runCommand("help.about"));
await page.waitForTimeout(80);
const aboutTxt = await page.evaluate(
  () => document.querySelector(".ide-modal-body")?.textContent || "");
ok(/Roberto Berrospe/i.test(aboutTxt), "About dialog credits Roberto Berrospe");
ok(/Claude/i.test(aboutTxt), "About dialog credits Claude");
// Close it
await page.evaluate(() => document.querySelector(".ide-modal-overlay")?.remove());

// ── 7: View → Full Source ──
await page.evaluate(() => window.RapidR.runCommand("view.source"));
await page.waitForTimeout(80);
const srcTxt = await page.evaluate(
  () => document.querySelector(".ide-modal-body pre")?.textContent || "");
ok(/\$APPTYPE\s+WEB/i.test(srcTxt), "Full Source dialog shows $APPTYPE WEB");
ok(/CREATE/i.test(srcTxt), "Full Source dialog includes CREATE blocks");
await page.evaluate(() => document.querySelector(".ide-modal-overlay")?.remove());

// ── 8: Add a module ──
await page.evaluate(() => {
  window.RapidR.runCommand("module.new");
});
// In-IDE prompt dialog: type a name and press Enter
await page.waitForSelector(".ide-modal-overlay input[type=text]", { timeout: 2000 });
await page.fill(".ide-modal-overlay input[type=text]", "MyMod");
await page.keyboard.press("Enter");
await page.waitForTimeout(180);
const modOk = await page.evaluate(() => ({
  count: window.RapidR.state.project.modules?.length || 0,
  hasTab: !!document.querySelector('.mtab[data-mod]'),
}));
ok(modOk.count === 1 && modOk.hasTab, `module created (count=${modOk.count}, tab=${modOk.hasTab})`);

// ── 9: Build → zip → check date ──
await page.evaluate(() => window.RapidR.runCommand("run.build"));
// wait until blob captured
await page.waitForFunction(() => !!window.__lastBlob, { timeout: 15000 });
const zipBytes = await page.evaluate(async () => {
  const buf = await window.__lastBlob.arrayBuffer();
  return Array.from(new Uint8Array(buf));
});
const buf = Buffer.from(zipBytes);
// Local file header at offset 0: bytes 10-11 = mod time, 12-13 = mod date
const modDate = buf.readUInt16LE(12);
const yr = ((modDate >> 9) & 0x7f) + 1980;
ok(yr >= new Date().getFullYear() - 1, `zip date year = ${yr} (current)`);

// ── 10: Runtime — no dim backdrop overlay ──
await page.evaluate(() => window.RapidR.runCommand("run.start"));
await page.waitForTimeout(1200);
// The preview is cross-origin to the IDE (SEC-02); use Playwright's frame API.
const previewFrame = page.frames().find(f => f.url().includes("preview.html"));
const overlay = previewFrame
  ? await previewFrame.evaluate(() => ({
      iframe: true,
      backdrops: document.querySelectorAll('div[id$="-backdrop"]').length,
    }))
  : { iframe: false };
ok(overlay.iframe, "preview iframe is mounted");
ok(overlay.backdrops === 0, `no -backdrop divs in runtime (got ${overlay.backdrops})`);

// Final
if (errors.length) {
  console.log("\nPage errors during run:");
  errors.forEach(e => console.log("  " + e));
}
ok(errors.length === 0, `no page errors (got ${errors.length})`);

await browser.close();
console.log("\nBugfix suite: ALL CHECKS PASSED");
