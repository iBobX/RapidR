// The GUI fixtures of tests/native_gui_events.mjs (tests/gui_parity_cases.mjs)
// in the browser: each fixture runs in the web IDE's preview, the same events
// are fired (a click on the component), and the same properties must read
// what the desktop, native and interpreted builds show.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_gui_parity.mjs [filter …]

import { chromium } from "playwright";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const filters = process.argv.slice(2);
let failed = 0, passed = 0, skipped = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };

// The fixtures' files ($RESOURCE, pictures) are the project's assets.
const assetDirs = [join(HERE, "fixtures"), join(HERE, "conformance/cases/resource_files")];
const assets = [];
for (const dir of assetDirs) {
  for (const f of readdirSync(dir)) {
    if (!statSync(join(dir, f)).isFile() || f.endsWith(".bas")) continue;
    assets.push({ name: f, mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64") });
  }
}

const browser = await chromium.launch();
// (`RAPIDR_DPR=2`: a high-DPI screen — what programs read must not change)
const page = await browser.newPage({ deviceScaleFactor: Number(process.env.RAPIDR_DPR || 1) });
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate((a) => { window.RapidR.state.project.assets = a; }, assets);

for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  if (c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.why || "no browser counterpart"})`); continue; }
  const source = readFileSync(join(HERE, "fixtures", c.name + ".bas"), "utf8");
  await page.evaluate((src) => {
    window.RapidR.runCommand("run.stop");
    window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
    window.RapidR.runCommand("run.start");
  }, source);
  await page.waitForTimeout(2500);
  const frame = page.frames().find((f) => f.url().includes("preview.html"));
  if (!frame) { ok(false, `${c.name}: preview frame`); continue; }
  const idOf = (name) => "rr-" + name.toLowerCase();
  // `resize: "w,h"` / `split: "splitter:delta"`: the user drags a QSPLITTER,
  // then resizes the frontmost form — before the events, as the desktop
  // test's hooks do.
  if (c.resize || c.split) {
    const [w, h] = (c.resize || "0,0").split(",").map(Number);
    const [sp, delta] = c.split ? c.split.split(":") : ["", "0"];
    await frame.evaluate(({ w, h, sp, delta }) => {
      const forms = [...document.querySelectorAll(".rr-form")].filter((f) => f.offsetWidth > 0);
      const top = forms.sort((a, b) => (Number(b.style.zIndex) || 0) - (Number(a.style.zIndex) || 0))[0];
      window.__rapidr_rt.rapidr_test_resize(top?.dataset.rrName || "", w, h, sp, Number(delta));
    }, { w, h, sp, delta });
    await page.waitForTimeout(300);
  }
  for (const ev of c.events.split(",").filter(Boolean)) {
    const [target, action] = ev.split(".");
    // The desktop's test actions: `form.__close` (the close button) and
    // `grid.__cell_c_r` (a click on a cell).
    const cell = /^__cell_(\d+)_(\d+)$/i.exec(action || "");
    // `tree.__node_2` / `tree.__toggle_0`: a click on node 2's text / node 0's button.
    const node = /^__(node|toggle)_(\d+)$/i.exec(action || "");
    const selector = node ? `[data-node="${node[2]}"] ${node[1].toLowerCase() === "node" ? ".rr-tree-text" : ".rr-tree-button"}`
      : action?.toLowerCase() === "__close" ? ".rr-form-btn-close"
      : cell ? `td[data-col="${cell[1]}"][data-row="${cell[2]}"]`
      : c.webClick?.[target.toLowerCase()];
    // `edit.__key_65`: the key typed in it; `canvas.__mousedown_10_20`
    // (…up, …move): the mouse at (10, 20) in it.
    const key = /^__key_(\d+)$/i.exec(action || "");
    const mouse = /^__mouse(down|up|move)_(\d+)_(\d+)$/i.exec(action || "");
    // `list.__item_4`: item 4 clicked (an owner-drawn combo box's picked).
    const item = /^__item_(\d+)$/i.exec(action || "");
    // `tree.__edit`: F2 on it; `__enter` / `__escape`: "Renamed" typed in
    // its node editor, then Enter / Escape.
    const edit = /^__(edit|enter|escape)$/i.exec(action || "")?.[1].toLowerCase();
    const fired = await frame.evaluate(({ id, selector, key, mouse, item, edit }) => {
      const host = document.getElementById(id);
      const el = selector ? host?.querySelector(selector) : host;
      if (!el) return false;
      if (edit === "edit") {
        el.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true, cancelable: true }));
        return true;
      }
      if (edit) {
        // (a list view's editor lies over its canvas: `data-for` its id)
        const input = el.querySelector(".rr-tree-editor") || document.querySelector(`.rr-tree-editor[data-for="${id}"]`);
        if (input) {
          input.value = "Renamed";
          input.dispatchEvent(new KeyboardEvent("keydown", { key: edit === "enter" ? "Enter" : "Escape", bubbles: true, cancelable: true }));
        }
        return true;
      }
      if (item) {
        const down = (e) => e.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
        if (host.classList.contains("rr-owner-combo")) {
          down(host);
          const pick = document.querySelector(`.rr-grid-dropdown [data-item="${item}"]`);
          if (!pick) return false;
          down(pick);
        } else {
          const it = host.querySelector(`[data-item="${item}"]`);
          if (!it) return false;
          it.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
        }
        return true;
      }
      if (key) {
        const vk = Number(key);
        const names = { 8: "Backspace", 9: "Tab", 13: "Enter", 27: "Escape", 32: " ", 37: "ArrowLeft", 38: "ArrowUp", 39: "ArrowRight", 40: "ArrowDown" };
        const k = names[vk] ?? String.fromCharCode(vk).toLowerCase();
        for (const type of ["keydown", "keyup"]) el.dispatchEvent(new KeyboardEvent(type, { key: k, bubbles: true, cancelable: true }));
        return true;
      }
      if (mouse) {
        const r = el.getBoundingClientRect();
        el.dispatchEvent(new MouseEvent("mouse" + mouse[0], { clientX: r.left + Number(mouse[1]), clientY: r.top + Number(mouse[2]), button: 0, bubbles: true, cancelable: true }));
        return true;
      }
      // (a list box answers a pick with `change`, anything else a click)
      el.dispatchEvent(host.tagName === "SELECT" ? new Event("change", { bubbles: true }) : new MouseEvent("click", { bubbles: true, cancelable: true }));
      return true;
    }, { id: idOf(target), selector, key: key?.[1], mouse: mouse && [mouse[1].toLowerCase(), mouse[2], mouse[3]], item: item?.[1], edit });
    if (!fired) ok(false, `${c.name}: ${target} exists`);
    await page.waitForTimeout(300);
    // A file dialog the event opened: the case's answer typed in, then Open / Save.
    if (c.fileDialog !== undefined) {
      await frame.evaluate((answer) => {
        const dlg = document.querySelector(".rr-file-dialog");
        if (!dlg) return;
        dlg.querySelector(".rr-file-name").value = answer;
        dlg.querySelector(answer ? ".rr-file-ok" : ".rr-file-cancel").click();
      }, c.fileDialog);
      await page.waitForTimeout(300);
    }
  }
  await page.waitForTimeout(500);
  // (The properties as the program reads them, as the desktop test prints them.)
  const dumped = await frame.evaluate((names) => Object.fromEntries(names.map((n) => {
    const [comp, ...rest] = n.split(".");
    return [n, window.__rapidr_rt.rapidr_get_prop(comp.toUpperCase(), rest.join(".").toLowerCase())];
  })), c.dump.split(","));
  const text = Object.entries(dumped).map(([k, v]) => `${k}=${v}`).join("\n");
  for (const line of c.expect) ok(text.includes(line), `${c.name} (web): ${line}${text.includes(line) ? "" : `   [got: ${text.replace(/\n/g, " ; ")}]`}`);
  // (RAPIDR_SHOT=dir: a screenshot of each case's page, to look at)
  if (process.env.RAPIDR_SHOT) await page.screenshot({ path: join(process.env.RAPIDR_SHOT, c.name + ".png") });
  // (`webCheck`: what the page shows, where the program can't read it)
  if (c.webCheck) {
    const got = await frame.evaluate(c.webCheck);
    ok(got === c.webExpect, `${c.name} (web page): ${c.webExpect}${got === c.webExpect ? "" : `   [got: ${got}]`}`);
  }
}
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
console.log(`\nWeb GUI parity: ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
if (failed) process.exit(1);
