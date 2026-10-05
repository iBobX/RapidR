// The GUI fixtures of tests/gui_parity_cases.mjs in the browser, as
// tests/web_gui_parity.mjs and tests/web_a11y.mjs run them: the web IDE
// opened (its project's assets the fixtures' files), a fixture run in its
// preview and the case's events fired there as the desktop test hooks fire
// them (a click on the component, a key, the mouse, a test action).

import { chromium } from "playwright";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
export const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

// The fixtures' files ($RESOURCE, pictures) are the project's assets.
function fixtureAssets() {
  const assetDirs = [join(HERE, "fixtures"), join(HERE, "conformance/cases/resource_files")];
  const assets = [];
  for (const dir of assetDirs) {
    for (const f of readdirSync(dir)) {
      if (!statSync(join(dir, f)).isFile() || f.endsWith(".bas")) continue;
      assets.push({ name: f, mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64") });
    }
  }
  return assets;
}

/// The browser with the web IDE ready (`dpr`: the screen's scale);
/// `pageErrors` collects the page's uncaught errors.
export async function openIde(dpr = 1) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: dpr });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(e.message));
  await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
  await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
  await page.evaluate((a) => { window.RapidR.state.project.assets = a; }, fixtureAssets());
  return { browser, page, pageErrors };
}

/// Case `c`'s fixture run in the preview and its events fired: the
/// preview's frame (null if it didn't come) and the event targets that
/// weren't there.
export async function runCase(page, c) {
  const source = readFileSync(join(HERE, "fixtures", c.name + ".bas"), "utf8");
  await page.evaluate((src) => {
    window.RapidR.runCommand("run.stop");
    window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
    window.RapidR.runCommand("run.start");
  }, source);
  await page.waitForTimeout(2500);
  const frame = page.frames().find((f) => f.url().includes("preview.html"));
  if (!frame) return { frame: null, missing: [] };
  const missing = [];
  const idOf = (name) => "rr-" + name.toLowerCase();
  // `joystick`: QDXJOYSTICK's gamepad, the tests' script (the page's
  // RAPIDR_TEST_JOYSTICK, read at each look).
  if (c.joystick !== undefined) await frame.evaluate((s) => { window.RAPIDR_TEST_JOYSTICK = s; }, c.joystick);
  // (ENVIRON$'s test values: the page's RAPIDR_TEST_ENV — the tests' own
  // HTTP server, tests/http_test_server.mjs)
  if (process.env.RAPIDR_TEST_HTTP) await frame.evaluate((h) => { window.RAPIDR_TEST_ENV = { RAPIDR_TEST_HTTP: h }; }, process.env.RAPIDR_TEST_HTTP);
  // (QMIDI: no MIDI output; QWAVE records a scripted tone, never the
  // microphone)
  await frame.evaluate(() => { window.RAPIDR_TEST_MIDI = ""; window.RAPIDR_TEST_WAVE_IN = "tone:440"; });
  // (how many of the case's dialog answers were given)
  let colorAnswers = 0, fontAnswers = 0;
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
    // `pn.__dblclick_5_5`: a double click there (the browser's events, each
    // with its click count)
    const dbl = /^__dblclick_(\d+)_(\d+)$/i.exec(action || "");
    // `list.__item_4`: item 4 clicked (an owner-drawn combo box's picked).
    const item = /^__item_(\d+)$/i.exec(action || "");
    // `tree.__edit`: F2 on it; `__enter` / `__escape`: "Renamed" typed in
    // its node editor, then Enter / Escape.
    const edit = /^__(edit|enter|escape)$/i.exec(action || "")?.[1].toLowerCase();
    const fired = await frame.evaluate(({ id, selector, key, mouse, dbl, item, edit }) => {
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
        const names = { 8: "Backspace", 9: "Tab", 13: "Enter", 16: "Shift", 17: "Control", 18: "Alt", 20: "CapsLock", 27: "Escape", 32: " ", 33: "PageUp", 34: "PageDown", 35: "End", 36: "Home", 37: "ArrowLeft", 38: "ArrowUp", 39: "ArrowRight", 40: "ArrowDown" };
        const k = names[vk] ?? String.fromCharCode(vk).toLowerCase();
        for (const type of ["keydown", "keyup"]) el.dispatchEvent(new KeyboardEvent(type, { key: k, bubbles: true, cancelable: true }));
        return true;
      }
      if (dbl) {
        const r = el.getBoundingClientRect();
        const at = { clientX: r.left + Number(dbl[0]), clientY: r.top + Number(dbl[1]), button: 0, bubbles: true, cancelable: true };
        for (const [type, detail] of [["mousedown", 1], ["mouseup", 1], ["click", 1], ["mousedown", 2], ["mouseup", 2], ["click", 2], ["dblclick", 2]]) {
          el.dispatchEvent(new MouseEvent(type, { ...at, detail }));
        }
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
    }, { id: idOf(target), selector, key: key?.[1], mouse: mouse && [mouse[1].toLowerCase(), mouse[2], mouse[3]], dbl: dbl && [dbl[1], dbl[2]], item: item?.[1], edit });
    if (!fired) missing.push(target);
    await page.waitForTimeout(300);
    // A colour dialog the event opened: the case's next answer's swatch
    // pressed, then OK (an empty answer: Cancel).
    if (c.colorDialog !== undefined) {
      const answers = c.colorDialog.split(";");
      const n = colorAnswers++;
      const answered = await frame.evaluate((answer) => {
        const dlg = document.querySelector(".rr-color-dialog");
        if (!dlg) return false;
        const swatch = answer ? dlg.querySelector(`.rr-color-swatch[data-color="${answer}"]`) : null;
        if (swatch) swatch.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
        dlg.querySelector(swatch ? ".rr-color-ok" : ".rr-color-cancel").click();
        return true;
      }, answers[n] ?? "");
      if (!answered) colorAnswers--;
      await page.waitForTimeout(300);
    }
    // A font dialog: the case's next answer (`Name,Size,styles,colour`) set
    // in its lists, check boxes and colour, then OK (empty: Cancel).
    if (c.fontDialog !== undefined) {
      const answers = c.fontDialog.split(";");
      const n = fontAnswers++;
      const answered = await frame.evaluate((answer) => {
        const dlg = document.querySelector(".rr-font-dialog");
        if (!dlg) return false;
        if (!answer) { dlg.querySelector(".rr-font-cancel").click(); return true; }
        const [name, size, styles = "", color = ""] = answer.split(",").map((s) => s.trim());
        const set = (sel, value) => { const el = dlg.querySelector(sel); if (!el) return; el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); };
        const check = (sel, on) => { const el = dlg.querySelector(sel); if (!el) return; el.checked = on; el.dispatchEvent(new Event("change", { bubbles: true })); };
        set(".rr-font-name", name);
        set(".rr-font-size", size);
        // (the style list: Regular, Italic, Bold, Bold Italic)
        set(".rr-font-style", ["Regular", "Italic", "Bold", "Bold Italic"][(styles.includes("i") ? 1 : 0) + (styles.includes("b") ? 2 : 0)]);
        check(".rr-font-under", styles.includes("u"));
        check(".rr-font-strike", styles.includes("s"));
        if (color) set(".rr-font-color", color);
        dlg.querySelector(".rr-font-ok").click();
        return true;
      }, answers[n] ?? "");
      if (!answered) fontAnswers--;
      await page.waitForTimeout(300);
    }
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
  return { frame, missing };
}

// ---- the UI kernel as the web runtime's GUI host (docs/web-host-plan.md,
// Stage W3): RAPIDR_WEB_HOST=kernel ----
//
// The fixture runs in tests/web_kernel.html?host=kernel with the desktop's
// test hooks (rapidr_ui_app::script: RAPIDR_TEST_EVENTS fired through the
// kernel's routing as on the desktop, RAPIDR_TEST_DUMP, the resize and the
// splitter, the dialogs' answers), given to the runtime as its environment
// (rapidr_set_test_env). When the script ends the page has the dump lines,
// each window's accessibility tree and its capture (the wasm's pixels, as
// the desktop's RAPIDR_CAPTURE BMP) — rapidr_test_results.

export const WEB_HOST = process.env.RAPIDR_WEB_HOST || "dom";

/// The case's environment for the test hooks, as tests/native_gui_events.mjs
/// gives the desktop's.
export function hookEnv(c) {
  const env = { RAPIDR_CAPTURE: "web", RAPIDR_TEST_EVENTS: c.events, RAPIDR_TEST_DUMP: c.dump, RAPIDR_TEST_RESIZE: c.resize || "", RAPIDR_TEST_SPLIT: c.split || "" };
  const opt = { fileDialog: "RAPIDR_TEST_FILE_DIALOG", colorDialog: "RAPIDR_TEST_COLOR_DIALOG", fontDialog: "RAPIDR_TEST_FONT_DIALOG", messageDialog: "RAPIDR_TEST_MESSAGE_DIALOG", dialogHold: "RAPIDR_TEST_DIALOG_HOLD", delay: "RAPIDR_CAPTURE_DELAY", joystick: "RAPIDR_TEST_JOYSTICK" };
  for (const [k, v] of Object.entries(opt)) if (c[k] !== undefined) env[v] = String(c[k]);
  return env;
}

const assetMap = () => Object.fromEntries(fixtureAssets().map((a) => [a.name, a.dataUrl]));

/// Case `c` on the kernel host, in a page of its own (`dpr`: the screen's
/// scale; the viewport — the screen — the desktop's headless host's,
/// 1920 × 1080): `{ results: {dump, a11y, captures} | null, errors, host, page }`
/// (the caller closes the page).
export async function runCaseKernel(browser, c, dpr = 1, timeout = 30000) {
  const page = await browser.newPage({ deviceScaleFactor: dpr, viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  try {
    await page.goto(`${URL_BASE}/tests/web_kernel.html?host=kernel`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
    const source = readFileSync(join(HERE, "fixtures", c.name + ".bas"), "utf8");
    const host = await page.evaluate(({ source, assets, env }) => {
      window.__rapidr_assets = assets;
      // (QDXJOYSTICK's gamepad: the tests' script, read at each look)
      if (env.RAPIDR_TEST_JOYSTICK !== undefined) window.RAPIDR_TEST_JOYSTICK = env.RAPIDR_TEST_JOYSTICK;
      const bc = window.rr.compile(source, "fixture", assets);
      window.rr.rapidr_set_test_env(env);
      window.rr.rapidr_run_bc(bc);
      return window.rr.rapidr_host();
    }, { source, assets: assetMap(), env: hookEnv(c) });
    await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { results, errors, host, page };
  } catch (e) {
    return { results: null, errors: [...errors, String(e.message).split("\n")[0]], host: "?", page };
  }
}
