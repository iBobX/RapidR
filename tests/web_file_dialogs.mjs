// QOPENDIALOG / QSAVEDIALOG on the web, with the user's real files
// (crates/rapidr-runtime-web/src/file_picker_web.rs), driven as the user
// drives them: real mouse clicks on the canvases, real keys into the
// accessibility mirror's fields.
//
//   1. Chromium (File System Access), in RapidR Studio: Studio's page
//      shows the browser's showOpenFilePicker / showSaveFilePicker for the
//      program's sandboxed run frame (an opaque origin may not show them;
//      ide/web/studio.js frameFiles). Headless Chromium can't show the
//      system's sheet, so the page's pickers are replaced by ones that
//      behave as Chromium's do — they throw SecurityError without the user's
//      gesture, AbortError on Cancel — over real files in a temporary folder:
//      Notepad (examples/rapidq/notepad.bas) opens a file from the disk, the
//      user edits it, saves it, and the file on the disk has the new text.
//      The Filter becomes the pickers' file types; Cancel returns 0.
//   2. Execute with no gesture left (from a timer): a kernel-drawn box
//      whose button opens the picker (on the runtime's own page,
//      tests/web_run.mjs, as 3).
//   3. A browser without File System Access (Firefox, Safari: the pickers
//      removed): Open is the page's file input (every file selectable when
//      the Filter has "All files"); Save asks for the name in a kernel box,
//      and the program's writes to it are a download.
//   4. The modal rule for the boxes the host draws: focus, keys and clicks
//      belong to the box; clicks on Notepad under it are refused; Tab and
//      Shift+Tab cycle inside it; Enter presses its default button, Escape
//      cancels; the focus goes back to Notepad's editor when it closes.
//
// Usage (repo root, after tools/build_web_artifacts.sh and
// tools/build_studio_web.sh, with the repo served on RAPIDR_URL, default
// http://localhost:8765; Studio on RAPIDR_STUDIO_URL, default its build
// there):  node tests/web_file_dialogs.mjs

import { chromium } from "playwright";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, basename } from "node:path";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const STUDIO_URL = process.env.RAPIDR_STUDIO_URL || `${URL_BASE}/target/studio-web`;
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };
const SHOTS = process.env.RAPIDR_SHOTS || "scratch";
mkdirSync(SHOTS, { recursive: true });
const dir = mkdtempSync(join(tmpdir(), "rapidr-files-"));
const notepad = readFileSync(new URL("../examples/rapidq/notepad.bas", import.meta.url), "utf8");

// ------------------------------------------------------------- the pickers --

// What the page's pickers answer next (a path in `dir`, or null:
// Cancel), and what they were asked.
const answers = [];
const calls = [];
// (Chromium's pickers, over real files: a handle reads and writes the file
// on the disk through the test)
const MOCK = `(() => {
  const handle = (path, name) => ({
    kind: "file", name,
    async getFile() { const b = await window.__rr_read(path); return new File([Uint8Array.from(atob(b), (c) => c.charCodeAt(0))], name); },
    async createWritable() {
      const parts = [];
      return {
        async write(d) { parts.push(new Uint8Array(d instanceof ArrayBuffer ? d : d.buffer ? d.buffer.slice(d.byteOffset, d.byteOffset + d.byteLength) : new TextEncoder().encode(String(d)))); },
        async close() { let s = ""; for (const p of parts) for (const c of p) s += String.fromCharCode(c); await window.__rr_write(path, btoa(s)); },
      };
    },
  });
  const pick = async (op, opts) => {
    const active = navigator.userActivation ? navigator.userActivation.isActive : true;
    const answer = await window.__rr_pick(op, JSON.stringify(opts ?? {}), active);
    if (!active) throw new DOMException("Must be handling a user gesture to show a file picker.", "SecurityError");
    if (!answer) throw new DOMException("The user aborted a request.", "AbortError");
    return answer;
  };
  window.showOpenFilePicker = async (opts) => (await pick("open", opts)).map((p) => handle(p, p.split("/").pop()));
  window.showSaveFilePicker = async (opts) => { const p = await pick("save", opts); return handle(p, p.split("/").pop()); };
})();`;
const NO_FSA = `window.showOpenFilePicker = undefined; window.showSaveFilePicker = undefined;`;

// A browser with (`fsa`) or without File System Access: `studio` — RapidR
// Studio running Notepad in its run frame (`run` gives that frame) — or the
// runtime's own page (`run` runs a program there).
async function open(fsa, studio = false) {
  const browser = await chromium.launch();
  const context = await browser.newContext({ acceptDownloads: true });
  const r = studio ? null : await openRunner(context);
  const page = r ? r.page : await context.newPage();
  const errors = r ? r.pageErrors : [];
  if (!r) page.on("pageerror", (e) => errors.push(e.message));
  page.on("dialog", async (d) => { errors.push("browser dialog " + d.type()); await d.dismiss(); });
  await page.exposeFunction("__rr_pick", (op, opts, active) => {
    calls.push({ op, opts: JSON.parse(opts), active });
    const a = answers.shift();
    return a === undefined ? null : a;
  });
  await page.exposeFunction("__rr_read", (path) => readFileSync(path).toString("base64"));
  await page.exposeFunction("__rr_write", (path, b64) => { writeFileSync(path, Buffer.from(b64, "base64")); });
  await page.addInitScript(fsa ? MOCK : NO_FSA);
  page.on("console", (m) => { if (process.env.DEBUG_RR) console.log("console:", m.type(), m.text()); });
  page.on("requestfailed", (r) => console.log("requestfailed", r.url(), r.failure()?.errorText));
  return { browser, page, errors, r };
}

// Studio, Notepad open and run (F5): the run frame.
async function studioRun(page) {
  const q = new URLSearchParams({ theme: "rapidr-light", fresh: "", window: "normal", open: "examples/rapidq/notepad.bas", do: "run.start" });
  await page.goto(`${STUDIO_URL}/index.html?${q}`, { waitUntil: "load" });
  let frame;
  for (let i = 0; i < 300 && !frame; i++) {
    await page.waitForTimeout(100);
    frame = page.frames().find((f) => f.url().includes("run.html"));
  }
  return frame;
}

// `source` run on the runtime's page: the page is the program's.
async function run(r, source) {
  await r.run(source);
  return r.page;
}

// (a real click on Notepad's File menu, then on its item `row` (0 New,
// 1 Open…, 2 Save…): the drop-down is drawn on the canvas)
async function menu(page, frame, row) {
  const b = await frame.locator('[role=menuitem][aria-label="File"]').boundingBox();
  await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
  await page.waitForTimeout(250);
  await page.mouse.click(b.x + 40, b.y + 40 + row * 20);
}

async function until(fn, ms = 5000) {
  const end = Date.now() + ms;
  let v;
  while (Date.now() < end) { v = await fn(); if (v) return v; await new Promise((r) => setTimeout(r, 50)); }
  return v;
}

const editor = (frame) => k.text(frame, "Editor");
const caption = (frame) => frame.evaluate(() => document.getElementById("rr-form")?.getAttribute("aria-label") ?? "");
const focused = (frame) => frame.evaluate(() => { const a = document.activeElement; return a ? (a.id || a.getAttribute("aria-label") || a.tagName) : ""; });
const crlf = (s) => s.replace(/\r?\n/g, "\r\n");

// ======================================================= 1. Chromium, FSA --
{
  const { browser, page, errors } = await open(true, true);
  const frame = await studioRun(page);
  ok(!!frame, "Studio runs Notepad in its run frame");
  await k.waitFor(frame, "Editor", 30000);
  const path = join(dir, "hello.txt");
  writeFileSync(path, "Hello from the disk.\r\nSecond line.");

  // Open… (mouse): the picker, the file read into the editor.
  answers.push([path]);
  calls.length = 0;
  await menu(page, frame, 1);
  await until(async () => (await editor(frame))?.startsWith("Hello from the disk"));
  ok((await editor(frame)) === "Hello from the disk.\nSecond line.", `Open… loads the real file into Notepad (${JSON.stringify(await editor(frame))})`);
  ok((await caption(frame)) === "Notepad - hello.txt", `FileName is the file's name (${await caption(frame)})`);
  const o = calls[0]?.opts ?? {};
  ok(calls.length === 1 && calls[0].op === "open" && calls[0].active, `the browser's open picker, inside the click's gesture (${JSON.stringify(calls[0])})`);
  ok(o.types?.[0]?.description === "Text files (*.txt)" && JSON.stringify(Object.values(o.types[0].accept)[0]) === '[".txt"]' && o.excludeAcceptAllOption === false && !o.multiple,
    `the Filter is the picker's types, "All files" offered (${JSON.stringify(o)})`);

  // The user edits it: a real click in the editor, real keys.
  const er = await frame.locator("#rr-editor").boundingBox();
  await page.mouse.click(er.x + er.width / 2, er.y + er.height / 2);
  await page.keyboard.press("Control+End");
  await page.keyboard.type(" Edited in RapidR.");
  await until(async () => (await editor(frame))?.endsWith("Edited in RapidR."));
  const text = await editor(frame);
  ok(text.endsWith("Second line. Edited in RapidR."), `typed into Notepad (${JSON.stringify(text)})`);

  // Save… (mouse): the save picker, the program's name proposed; the file
  // on the disk has the new text.
  answers.push(path);
  calls.length = 0;
  await menu(page, frame, 2);
  const saved = await until(() => readFileSync(path, "utf8").includes("Edited in RapidR."));
  ok(saved && readFileSync(path, "utf8") === crlf(text), `Save… writes the real file (${JSON.stringify(readFileSync(path, "utf8"))})`);
  ok(calls[0]?.op === "save" && calls[0].opts.suggestedName === "hello.txt" && calls[0].active, `the save picker proposes FileName (${JSON.stringify(calls[0])})`);
  ok((await caption(frame)) === "Notepad - hello.txt", "the caption shows the saved name");

  // Cancel: Execute returns 0, nothing changes.
  answers.push(null);
  calls.length = 0;
  await menu(page, frame, 1);
  await until(() => calls.length === 1);
  await page.waitForTimeout(300);
  ok((await editor(frame)) === text && (await caption(frame)) === "Notepad - hello.txt", "Cancel: Execute is 0, Notepad unchanged");

  // Ctrl+O (a key is a gesture too), a second file; Save to a new one.
  const other = join(dir, "other.txt");
  writeFileSync(other, "Another file.");
  answers.push([other]);
  await page.mouse.click(er.x + 20, er.y + 20);
  await page.keyboard.press("Control+o");
  await until(async () => (await editor(frame)) === "Another file.");
  ok((await editor(frame)) === "Another file.", "Ctrl+O opens the picker too");
  const fresh = join(dir, "new.txt");
  answers.push(fresh);
  await page.keyboard.press("Control+s");
  ok(await until(() => { try { return readFileSync(fresh, "utf8") === "Another file."; } catch { return false; } }), "Ctrl+S saves to a new real file");
  // (the page is unchanged by its pickers: no in-page dialog of its own)
  ok(!(await frame.evaluate(() => !!document.querySelector(".rr-dialog, .rr-file-dialog"))), "no in-page file dialog");
  await page.screenshot({ path: `${SHOTS}/web_file_dialogs_fsa.png` });
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await browser.close();
}

// ================================ 2. no gesture left (the runtime's page) --
{
  const { browser, page, errors, r } = await open(true);
  const path = join(dir, "hello.txt");
  // 2. Execute with no gesture left: the box whose button opens the picker.
  calls.length = 0;
  answers.push([path]);
  const frame2 = await run(r, [
    '$INCLUDE "RAPIDQ.INC"',
    'CREATE Dlg AS QOPENDIALOG',
    '  Filter = "Pictures|*.bmp;*.ico|Text|*.txt"',
    '  FilterIndex = 2',
    'END CREATE',
    'SUB Later',
    '  Tmr.Enabled = 0',
    '  IF Dlg.Execute THEN Lbl.Caption = "picked " + Dlg.FileName ELSE Lbl.Caption = "cancelled"',
    'END SUB',
    'CREATE Form AS QFORM',
    '  Caption = "Timer"',
    '  CREATE Lbl AS QLABEL',
    '    Caption = "waiting"',
    '    Width = 200',
    '  END CREATE',
    '  CREATE Tmr AS QTIMER',
    '    Interval = 5500',
    '    OnTimer = Later',
    '  END CREATE',
    'END CREATE',
    'Form.ShowModal',
  ].join("\n"));
  // (nothing evaluated in the pages meanwhile: Playwright's evaluate is a
  // user gesture; the last one's activation, 5 s in Chromium, is over when
  // the timer fires)
  await page.waitForTimeout(7000);
  const box = await until(async () => { const d = await k.dialog(frame2); return d?.title === "Open" ? d : null; });
  ok(box && box.title === "Open" && JSON.stringify(box.buttons) === '["Open…","Cancel"]', `no gesture: a kernel box asks first (${JSON.stringify(box)})`);
  ok(calls.length === 0, "the picker waits for the user's click");
  await page.screenshot({ path: `${SHOTS}/web_file_dialogs_gesture.png` });
  await k.clickButton(frame2, "Open…");
  ok(await until(async () => (await k.text(frame2, "Lbl")) === "picked hello.txt"), `its button opens the picker (${await k.text(frame2, "Lbl")})`);
  ok(calls[0]?.active && calls[0].opts.types?.[0]?.description === "Text" && calls[0].opts.excludeAcceptAllOption === true,
    `FilterIndex's type first; no "All files" when the Filter hasn't it (${JSON.stringify(calls[0]?.opts)})`);
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await browser.close();
}

// ============================================ 3. no File System Access ----
{
  const { browser, page, errors, r } = await open(false);
  const frame = await run(r, notepad);
  await k.waitFor(frame, "Editor");
  const path = join(dir, "plain.txt");
  writeFileSync(path, "From a file input.");

  // Open…: the page's file input, every file selectable ("All files").
  const chooser = page.waitForEvent("filechooser");
  await menu(page, frame, 1);
  const fc = await chooser;
  const accept = await fc.element().evaluate((e) => e.accept);
  ok(accept === "", `the file input lets every file through (accept=${JSON.stringify(accept)})`);
  await fc.setFiles(path);
  await until(async () => (await editor(frame)) === "From a file input.");
  ok((await editor(frame)) === "From a file input.", "Open… loads the file picked");
  ok((await caption(frame)) === "Notepad - plain.txt", `FileName (${await caption(frame)})`);

  // Save…: the name asked in a kernel box (no save picker here).
  await menu(page, frame, 2);
  let d = await until(async () => { const b = await k.dialog(frame); return b?.title === "Save As" ? b : null; });
  ok(d && d.title === "Save As" && d.input && JSON.stringify(d.buttons) === '["Save","Cancel"]', `Save asks for the name in a kernel box (${JSON.stringify(d)})`);
  const field = frame.locator(`.rr-kwin[data-rr-form="${d?.form}"] input`);
  ok((await field.inputValue()) === "plain.txt", `it proposes FileName (${await field.inputValue()})`);
  ok((await focused(frame)) === (await field.getAttribute("id")) || (await frame.evaluate(() => document.activeElement?.tagName)) === "INPUT", "the name field has the focus");
  await page.screenshot({ path: `${SHOTS}/web_file_dialogs_name.png` });

  // The modal rule, with real input: a click on Notepad's editor below is
  // refused; the caret blinks on (Notepad repaints) and the focus stays.
  const er = await frame.locator("#rr-editor").boundingBox();
  await page.mouse.click(er.x + 30, er.y + er.height - 30);
  await page.waitForTimeout(1500);
  ok((await frame.evaluate(() => document.activeElement?.tagName)) === "INPUT" && (await frame.evaluate(() => document.activeElement?.closest(".rr-kwin")?.dataset.rrForm)) === d.form,
    `a click on Notepad under the box is refused; the focus stays in the box (${await focused(frame)})`);
  // A real click in the field, real keys: the name goes to the box, not
  // to Notepad.
  const fb = await field.boundingBox();
  await page.mouse.click(fb.x + fb.width - 5, fb.y + fb.height / 2);
  await page.keyboard.press("End");
  for (let i = 0; i < 9; i++) await page.keyboard.press("Backspace");
  await page.keyboard.type("sd.txt");
  await page.waitForTimeout(300);
  ok((await field.inputValue()) === "sd.txt", `the keys went to the box (${await field.inputValue()})`);
  ok((await editor(frame)) === "From a file input.", `Notepad's text is unchanged (${JSON.stringify(await editor(frame))})`);
  // Tab and Shift+Tab cycle inside the box: field → Save → Cancel → field.
  const order = [];
  for (let i = 0; i < 3; i++) { await page.keyboard.press("Tab"); order.push(await focused(frame)); }
  await page.keyboard.press("Shift+Tab");
  order.push(await focused(frame));
  ok(/:ok$/.test(order[0]) && /:cancel$/.test(order[1]) && /:field$/.test(order[2]) && /:cancel$/.test(order[3]), `Tab cycles inside the box (${order.join(" → ")})`);
  // Enter (back in the field) presses Save: the program's write is a
  // download of that name.
  await page.keyboard.press("Tab");
  const download = page.waitForEvent("download");
  await page.keyboard.press("Enter");
  const dl = await download;
  const dlPath = join(dir, "downloaded.txt");
  await dl.saveAs(dlPath);
  ok(dl.suggestedFilename() === "sd.txt" && readFileSync(dlPath, "utf8") === crlf("From a file input."), `Save is a download of the name given (${dl.suggestedFilename()}, ${JSON.stringify(readFileSync(dlPath, "utf8"))})`);
  ok(await until(async () => (await caption(frame)) === "Notepad - sd.txt"), `FileName is the name given (${await caption(frame)})`);
  // The focus is back on Notepad's editor: keys go to it.
  ok(await until(async () => (await focused(frame)) === "rr-editor"), `the focus goes back to the editor (${await focused(frame)})`);
  await page.keyboard.type("Z");
  ok(await until(async () => (await editor(frame))?.includes("Z")), "keys go to Notepad again");

  // Escape cancels the box: Execute is 0.
  await menu(page, frame, 2);
  d = await until(async () => { const b = await k.dialog(frame); return b?.title === "Save As" ? b : null; });
  ok(!!d, "the box again");
  ok(await until(async () => /:field$/.test(await focused(frame))), `its field has the focus (${await focused(frame)})`);
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
  const left = (await k.windows(frame)).map((w) => w.title);
  ok(JSON.stringify(left) === '["Notepad - sd.txt"]', `Escape cancels: the box gone, Execute 0, Notepad as it was (${left})`);
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await browser.close();
}

// ================================= 4. a page of its own (a bundle), FSA --
// (the program is the page: it shows the browser's pickers itself)
{
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.exposeFunction("__rr_pick", (op, opts, active) => {
    calls.push({ op, opts: JSON.parse(opts), active });
    const a = answers.shift();
    return a === undefined ? null : a;
  });
  await page.exposeFunction("__rr_read", (path) => readFileSync(path).toString("base64"));
  await page.exposeFunction("__rr_write", (path, b64) => { writeFileSync(path, Buffer.from(b64, "base64")); });
  await page.addInitScript(MOCK);
  await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 30000 });
  await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "notepad", {})), notepad);
  await k.waitFor(page, "Editor");
  const path = join(dir, "page.txt");
  writeFileSync(path, "Opened by the page.");
  answers.push([path]);
  calls.length = 0;
  await menu(page, page, 1);
  ok(await until(async () => (await editor(page)) === "Opened by the page."), `a page of its own opens a real file (${JSON.stringify(await editor(page))})`);
  const er = await page.locator("#rr-editor").boundingBox();
  await page.mouse.click(er.x + er.width / 2, er.y + er.height / 2);
  await page.keyboard.press("Control+End");
  await page.keyboard.type(" And saved.");
  answers.push(path);
  await menu(page, page, 2);
  ok(await until(() => readFileSync(path, "utf8") === "Opened by the page. And saved."), `…and saves it (${JSON.stringify(readFileSync(path, "utf8"))})`);
  ok(calls.length === 2 && calls.every((c) => c.active), "its own pickers, each in the click's gesture");
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await browser.close();
}

rmSync(dir, { recursive: true, force: true });
if (failed) { console.log(`\nFile dialogs: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nFile dialogs: ALL CHECKS PASSED");
