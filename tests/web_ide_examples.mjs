// The web IDE shows what it loads, and runs it at 1:1.
//
//   1. Every example in the examples list loads into the editor: a program
//      with a form shows the form in the designer (its components, its menu's
//      captions) and its code; one without (a console program) shows its code
//      in a module. The examples are RapidQ code (CREATE Form AS QFORM,
//      QMEMO, `Caption = "&New": OnClick = NewText`): RapidQ's names are the
//      RapidR components (the compiler's rule, rapidr_ast::canonical_type_name;
//      the registry's `name` in lang-data.js).
//   2. Open of a local .rr and .bas file does the same, and the program runs
//      as written.
//   3. The run window draws the program at exactly 1:1 logical pixels: each
//      canvas's CSS size × devicePixelRatio is its backing store's size in
//      both axes, nothing scaled by CSS, at DPR 1 and 2.
//
// Usage:  node tests/web_ide_examples.mjs   (repo served on RAPIDR_URL,
//         default http://localhost:8765; tools/build_web_artifacts.sh first)

import { chromium } from "playwright";
import { readFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const ROOT = new URL("..", import.meta.url).pathname;

let failed = 0;
function ok(cond, msg) {
  if (cond) console.log("✓ " + msg);
  else { console.log("✗ " + msg); failed++; }
}

const browser = await chromium.launch();

async function openIde(dpr = 1) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, deviceScaleFactor: dpr });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
  await page.waitForFunction(() => document.getElementById("status")?.textContent === "ready", null, { timeout: 30000 });
  return { page, errors };
}

/// What the IDE shows: tabs, the active pane's design components and the
/// project tree's forms / modules.
function shown(page) {
  return page.evaluate(() => ({
    status: document.getElementById("status")?.textContent || "",
    tabs: [...document.querySelectorAll("#mdi-tabs .mtab")].map((t) => t.textContent.replace("×", "").trim()),
    widgets: [...document.querySelectorAll(".mdi-pane.active .design-form .dwidget")].map((w) => ({ name: w.dataset.name, type: w.dataset.type, text: (w.querySelector("textarea, input")?.value ?? w.textContent).trim(), w: w.offsetWidth, h: w.offsetHeight })),
    tree: document.querySelector("#project-tree, .project-tree")?.innerText || "",
  }));
}

/// The code the IDE's editor holds for the active tab (F7: the form's code).
async function editorText(page) {
  await page.keyboard.press("F7");
  await page.waitForFunction(() => !!window.monaco?.editor?.getModels?.().length, null, { timeout: 15000 });
  await page.waitForTimeout(300);
  return page.evaluate(() => window.monaco.editor.getModels().map((m) => m.getValue()).join("\n"));
}

// ── 1. Every example in the list ──────────────────────────────────
{
  const { page, errors } = await openIde();
  const examples = await page.evaluate(() => [...document.querySelectorAll("#examples option")].map((o) => o.value).filter(Boolean));
  ok(examples.length >= 10, `the examples list has ${examples.length} examples`);
  for (const ex of examples) {
    const src = readFileSync(join(ROOT, ex.replace(/^\.\.\//, "")), "utf8");
    const hasForm = /^\s*CREATE\s+\w+\s+AS\s+[QR]FORM\b/im.test(src);
    await page.selectOption("#examples", "");
    await page.selectOption("#examples", ex);
    await page.waitForFunction(() => /^loaded |failed/.test(document.getElementById("status")?.textContent || ""), null, { timeout: 15000 });
    await page.waitForTimeout(150);
    const s = await shown(page);
    const name = ex.split("/").pop();
    if (hasForm) {
      ok(s.tabs.some((t) => /\[Design\]$/.test(t)) && s.widgets.length > 0,
        `${name}: its form in the designer (${s.widgets.length} components; tabs ${JSON.stringify(s.tabs)})`);
    } else {
      ok(s.tabs.some((t) => /\[Module\]$/.test(t)), `${name}: its code in a module (tabs ${JSON.stringify(s.tabs)})`);
    }
  }
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await page.close();
}

// ── 1b. Menus: what the designer and the editor show ──────────────
{
  const { page } = await openIde();
  await page.selectOption("#examples", "../examples/gui/menus.rr");
  await page.waitForFunction(() => /^loaded /.test(document.getElementById("status")?.textContent || ""), null, { timeout: 15000 });
  await page.waitForTimeout(200);
  const s = await shown(page);
  const by = (n) => s.widgets.find((w) => w.name === n);
  ok(by("MainMenu")?.text.replace(/\s+/g, " ") === "File View Help", `menus: the main menu shows its menus (${JSON.stringify(by("MainMenu")?.text)})`);
  ok(by("Text")?.type === "RMemo" && by("Text").text.includes("Right-click me for the pop-up menu.") && by("Text").w > 300 && by("Text").h > 150,
    `menus: QMEMO is a memo filling the form (alClient): ${JSON.stringify(by("Text"))}`);
  ok(by("Status")?.type === "RStatusBar" && by("Status").text === "Ready", `menus: the status bar shows its SimpleText (${JSON.stringify(by("Status"))})`);
  ok(!s.widgets.some((w) => w.type === "RMenuItem"), "menus: menu items are in the bar, not boxes on the form");
  const props = await page.evaluate(() => document.getElementById("props-target")?.textContent || "");
  ok(/RForm/.test(props), `menus: the form's properties are RForm's (${props})`);
  // (a line's several properties: `Caption = "&New": ShortCut = "Ctrl+N": OnClick = NewText`)
  await page.locator("#proj-tree .tree-sub-item", { hasText: /^\s*\S*\s*NewItem\s*$/ }).first().click();
  const item = await page.evaluate(() => ({
    target: document.getElementById("props-target")?.textContent || "",
    caption: document.querySelector('#props-body .prop-row[data-key="caption"] input')?.value,
    shortcut: document.querySelector('#props-body .prop-row[data-key="shortcut"] input')?.value,
  }));
  ok(/NewItem\s+RMenuItem/.test(item.target) && item.caption === "&New" && item.shortcut === "Ctrl+N",
    `menus: NewItem's properties from one line (${JSON.stringify(item)})`);
  const code = await editorText(page);
  ok(code.includes("SUB NewText") && code.includes("Status.SimpleText"), "menus: the editor shows the program's code");
  await page.close();
}

// ── 2. Open of a local .rr / .bas file ────────────────────────────
{
  const dir = mkdtempSync(join(tmpdir(), "rapidr-ide-open-"));
  const files = {
    "opened.bas": readFileSync(join(ROOT, "examples/gui/menus.rr"), "utf8"),
    "opened.rr": readFileSync(join(ROOT, "examples/gui/hello_form.rr"), "utf8"),
    "console.rr": readFileSync(join(ROOT, "examples/basics/hello.rr"), "utf8"),
  };
  for (const [n, t] of Object.entries(files)) writeFileSync(join(dir, n), t);
  for (const input of ["#file-open-project", "#file-open-example"]) {
    const { page, errors } = await openIde();
    for (const [n, t] of Object.entries(files)) {
      await page.setInputFiles(input, join(dir, n));
      await page.waitForFunction((n) => (document.getElementById("status")?.textContent || "").includes(n), n, { timeout: 15000 });
      await page.waitForTimeout(150);
      const s = await shown(page);
      if (n === "console.rr") ok(s.tabs.some((t) => t === "console [Module]"), `${input} ${n}: its code in a module (${JSON.stringify(s.tabs)})`);
      else ok(s.tabs.some((t) => /\[Design\]$/.test(t)) && s.widgets.length > 0, `${input} ${n}: its form in the designer (${s.widgets.length} components)`);
      ok(/^loaded /.test(s.status), `${input} ${n}: ${s.status}`);
    }
    // (it runs as written: the menus program, from the .bas)
    await page.setInputFiles(input, join(dir, "opened.bas"));
    await page.waitForFunction(() => (document.getElementById("status")?.textContent || "").includes("opened.bas"), null, { timeout: 15000 });
    await page.click("#btn-run");
    const frame = page.frameLocator("#preview");
    await frame.locator("#rr-text").waitFor({ state: "attached", timeout: 20000 }).catch(() => {});
    const text = await frame.locator("#rr-text").evaluate((e) => e.value ?? e.textContent).catch(() => null);
    ok(text === "Right-click me for the pop-up menu.", `${input}: the opened program runs as written (${JSON.stringify(text)})`);
    ok(errors.length === 0, `${input}: no page errors (${errors.join("; ")})`);
    await page.close();
  }
}

// ── 3. The run window draws at 1:1 ─────────────────────────────────
for (const dpr of [1, 2]) {
  const { page } = await openIde(dpr);
  await page.selectOption("#examples", "../examples/gui/menus.rr");
  await page.waitForFunction(() => /^loaded /.test(document.getElementById("status")?.textContent || ""), null, { timeout: 15000 });
  await page.click("#btn-run");
  const frameEl = await page.$("#preview");
  const frame = await frameEl.contentFrame();
  await frame.waitForFunction(() => [...document.querySelectorAll("canvas")].some((c) => c.offsetWidth > 0), null, { timeout: 20000 });
  await page.waitForTimeout(500);
  const m = await frame.evaluate(() => ({
    dpr: devicePixelRatio,
    canvases: [...document.querySelectorAll("canvas")].filter((c) => c.offsetWidth > 0).map((c) => {
      const r = c.getBoundingClientRect();
      return { cssW: r.width, cssH: r.height, w: c.width, h: c.height, transform: getComputedStyle(c).transform };
    }),
    zoom: getComputedStyle(document.documentElement).zoom,
    body: getComputedStyle(document.body).transform,
  }));
  const outer = await page.evaluate(() => {
    const f = document.getElementById("preview");
    const r = f.getBoundingClientRect();
    let tf = [], el = f;
    while (el) { const t = getComputedStyle(el).transform; if (t !== "none") tf.push(t); el = el.parentElement; }
    return { w: r.width, h: r.height, cw: f.clientWidth, ch: f.clientHeight, transforms: tf, zoom: getComputedStyle(f).zoom };
  });
  ok(m.dpr === dpr, `DPR ${dpr}: the frame's devicePixelRatio is the page's (${m.dpr})`);
  ok(m.canvases.length > 0, `DPR ${dpr}: the program's windows are drawn (${m.canvases.length} canvases)`);
  for (const c of m.canvases) {
    ok(Math.round(c.cssW * dpr) === c.w && Math.round(c.cssH * dpr) === c.h && c.transform === "none",
      `DPR ${dpr}: canvas ${c.cssW}×${c.cssH} CSS × ${dpr} = ${c.w}×${c.h} backing, no transform (${c.transform})`);
  }
  ok(m.zoom === "1" && m.body === "none", `DPR ${dpr}: the frame's page isn't zoomed or transformed`);
  ok(outer.transforms.length === 0 && outer.zoom === "1" && Math.round(outer.w) === outer.cw && Math.round(outer.h) === outer.ch,
    `DPR ${dpr}: the run window's frame isn't CSS-scaled (${JSON.stringify(outer)})`);
  await page.close();
}

await browser.close();
console.log(failed ? `\n${failed} check(s) failed` : "\nAll checks passed");
process.exit(failed ? 1 : 0);
