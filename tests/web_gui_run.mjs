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
    // `form.__tray_513`: Windows' mouse message 513 (WM_LBUTTONDOWN …) on
    // the form's system tray icon (the page's tray strip, tray_web.rs).
    const tray = /^__tray_(\d+)$/i.exec(action || "");
    if (tray) {
      const done = await frame.evaluate(({ form, msg }) => {
        // (no icon: nothing to press, as on the desktop — the case's dump tells)
        const icon = document.querySelector(`.rr-tray-icon[data-form="${form}"]`);
        if (!icon) return true;
        const kinds = { 513: ["mousedown", 0, 1], 514: ["mouseup", 0, 1], 515: ["mousedown", 0, 2], 516: ["mousedown", 2, 1], 517: ["mouseup", 2, 1], 519: ["mousedown", 1, 1], 520: ["mouseup", 1, 1] };
        const [type, button, detail] = kinds[msg] || [];
        if (!type) return false;
        icon.dispatchEvent(new MouseEvent(type, { button, detail, bubbles: true, cancelable: true }));
        return true;
      }, { form: target.toLowerCase(), msg: Number(tray[1]) });
      if (!done) missing.push(target);
      await page.waitForTimeout(300);
      continue;
    }
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
