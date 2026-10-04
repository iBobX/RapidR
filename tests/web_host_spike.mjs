// The web host spike (docs/web-host-plan.md, "Spike results"): the UI
// kernel in the browser, measured and checked.
//
// - Sizes: the spike's wasm (kernel + vello_cpu + parley + the built-in
//   fonts), raw / gzip / brotli, against today's web runtime
//   (target/web/rapidrintr_bg.wasm, or RAPIDR_COMPARE_WASM).
// - Pixels: each form's frame in the browser (the wasm's own pixels and the
//   canvas read back) against the desktop host's CPU capture of the same
//   MemStore form (`cargo run -p rapidr-ui-host-web --example
//   desktop_capture --release`), at devicePixelRatio 1 and 2.
// - Timings: wasm load, fonts, first frame, per-frame paint (kernel /
//   vello_cpu / putImageData / mirror), input → frame latency.
// - Input: clicks, keys, the wheel, an input method's composition (CDP
//   Input.imeSetComposition / insertText), the clipboard events.
// - Accessibility: Chrome's tree (CDP Accessibility.getFullAXTree) for the
//   mirror's elements against the kernel's AccessNode tree — what AccessKit
//   gets on the desktop.
//
// Usage (repo root, after the wasm-pack build and the desktop capture, the
// repo served on http://127.0.0.1:8782 or RAPIDR_URL):
//   node tests/web_host_spike.mjs
// Writes target/web-host-spike/report.json and screenshots next to it.

import { existsSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { gzipSync, brotliCompressSync, constants } from "node:zlib";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "target/web-host-spike");
const BASE = process.env.RAPIDR_URL || "http://127.0.0.1:8782";
const report = { sizes: {}, pixels: {}, timings: {}, input: {}, a11y: {} };
let failed = 0, passed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };
mkdirSync(OUT, { recursive: true });

// ------------------------------------------------------------- sizes --

function sizes(file) {
  const raw = readFileSync(file);
  return { raw: raw.length, gzip: gzipSync(raw, { level: 9 }).length, brotli: brotliCompressSync(raw, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length };
}
const mb = (n) => (n / 1e6).toFixed(2) + " MB";
for (const [name, file] of [
  ["spike (vello_cpu)", join(OUT, "rapidr_ui_host_web_bg.wasm")],
  ["spike (+ vello on WebGPU)", join(ROOT, "target/web-host-spike-gpu/rapidr_ui_host_web_bg.wasm")],
  ["web runtime today", process.env.RAPIDR_COMPARE_WASM || join(ROOT, "target/web/rapidrintr_bg.wasm")],
]) {
  if (!existsSync(file)) continue;
  report.sizes[name] = sizes(file);
  const s = report.sizes[name];
  console.log(`  ${name}: ${mb(s.raw)} raw, ${mb(s.gzip)} gzip, ${mb(s.brotli)} brotli`);
}

// ------------------------------------------------------------ helpers --

const median = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.floor(s.length / 2)] : NaN; };
const p95 = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * 0.95))] : NaN; };
const r2 = (v) => Math.round(v * 100) / 100;

/// The desktop's capture of form `name` at `scale`: {w, h, rgba} — with the
/// system's fallback fonts (`desktop`) or the built-in fonts alone
/// (`desktop-builtin`, the fonts a browser has).
function desktop(name, scale, sub = "desktop") {
  const file = join(OUT, sub, `${name}@${scale}x.rgba`);
  if (!existsSync(file)) return null;
  const b = readFileSync(file);
  return { w: b.readUInt32LE(0), h: b.readUInt32LE(4), rgba: b.subarray(8) };
}

/// How two RGBA images of the same size differ.
function diff(a, b, w, h) {
  let same = 0, over8 = 0, max = 0, sum = 0;
  let box = null;
  for (let i = 0, p = 0; i < a.length; i += 4, p++) {
    const d = Math.max(Math.abs(a[i] - b[i]), Math.abs(a[i + 1] - b[i + 1]), Math.abs(a[i + 2] - b[i + 2]));
    if (d === 0) { same++; continue; }
    if (process.env.SPIKE_DEBUG && sum === 0) console.log(`    first difference at ${p % w},${Math.floor(p / w)}: ${[...a.slice(i, i + 4)]} vs ${[...b.slice(i, i + 4)]}`);
    sum += d;
    if (d > max) max = d;
    if (d > 8) over8++;
    const x = p % w, y = Math.floor(p / w);
    box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
  }
  const n = w * h;
  return { pixels: n, identical: same, differing: n - same, over8, maxDiff: max, meanDiffOfDiffering: n - same ? r2(sum / (n - same)) : 0, box };
}

async function open(browser, dpr, query = "steady=1") {
  const context = await browser.newContext({ deviceScaleFactor: dpr, viewport: { width: 1300, height: 900 } });
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: BASE });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const t = Date.now();
  await page.goto(`${BASE}/tests/web_host_spike.html?${query}`);
  await page.waitForFunction(() => window.spike?.ready || window.spike?.errors?.length, null, { timeout: 60_000 });
  const navMs = Date.now() - t;
  await page.evaluate(() => window.spike.idle());
  return { context, page, errors, navMs };
}

// ------------------------------------------------- pixels and timings --

const browser = await chromium.launch();
for (const dpr of [1, 2]) {
  const { context, page, errors } = await open(browser, dpr);
  ok(errors.length === 0, `@${dpr}x the page runs without errors ${errors.join("; ")}`);
  const load = await page.evaluate(() => ({ ...window.spike.metrics.load, firstFrame: window.spike.metrics.firstFrame, frames: window.spike.metrics.frames, status: document.getElementById("status").textContent }));
  report.timings[`load@${dpr}x`] = load;
  console.log(`  @${dpr}x ${load.status}`);
  await page.screenshot({ path: join(OUT, `page@${dpr}x.png`) });
  for (const name of ["trackbar", "texts", "lists"]) {
    const d = desktop(name, dpr);
    // (both at once: no frame between them)
    const read = () => page.evaluate((n) => [window.spike.pixels(n), window.spike.canvasPixels(n), window.spike.metrics.contextLost || 0], name);
    let [web, canvas, lost] = await read();
    if (lost) {
      // (Chrome dropped the canvas's backing — headless Chrome's first
      // context, while its GPU process settles: the page draws the frame
      // again on contextrestored)
      console.log(`  ${name}@${dpr}x: the page saw ${lost} canvas context loss(es) so far; read again after the restore`);
      await page.waitForTimeout(200);
      [web, canvas] = await read();
    }
    if (!d) {
      ok(false, `${name}@${dpr}x: no desktop capture (run the desktop_capture example)`);
      continue;
    }
    ok(web.length === d.w * d.h * 4, `${name}@${dpr}x: the browser's frame is the desktop's size (${d.w}×${d.h})`);
    const say = (what, r) => `${what}: ${r.identical}/${r.pixels} identical, ${r.differing} differ (${r.over8} by > 8, max ${r.maxDiff})${r.box ? " in x,y " + r.box.join(",") : ""}`;
    // The same fonts on both sides (the built-in ones): the same pixels.
    const builtin = desktop(name, dpr, "desktop-builtin");
    const wasmVsBuiltin = builtin ? diff(web, builtin.rgba, d.w, d.h) : null;
    // The desktop as it is (the system's fonts fill in what Liberation lacks).
    const wasmVsDesktop = diff(web, d.rgba, d.w, d.h);
    const canvasVsWasm = diff(canvas, web, d.w, d.h);
    report.pixels[`${name}@${dpr}x`] = { size: [d.w, d.h], wasmVsBuiltin, wasmVsDesktop, canvasVsWasm };
    if (wasmVsBuiltin) console.log(`  ${name}@${dpr}x ${say("vs the desktop with the built-in fonts", wasmVsBuiltin)}`);
    console.log(`  ${name}@${dpr}x ${say("vs the desktop with system fallback fonts", wasmVsDesktop)}; canvas readback vs wasm: ${canvasVsWasm.differing} differ`);
    ok(canvasVsWasm.differing === 0, `${name}@${dpr}x: the canvas shows exactly the wasm's pixels`);
    ok(wasmVsBuiltin?.differing === 0, `${name}@${dpr}x: byte-identical to the desktop's capture with the same fonts`);
    // (§2.4(c)'s tolerance: ±8 per channel on ≤ 0.5 % of the pixels)
    if (name !== "texts") ok(wasmVsDesktop.over8 <= wasmVsDesktop.pixels * 0.005, `${name}@${dpr}x: within the desktop matrix's tolerance of the desktop's own capture`);
    await page.locator(`canvas[data-form=${name}]`).screenshot({ path: join(OUT, `${name}@${dpr}x.png`) });
    // Per-frame cost, 60 frames drawn again (performance.now() is coarse:
    // the means are the better figure).
    const bench = await page.evaluate((n) => window.spike.bench(n, 60), name);
    const pick = (k) => bench.map((b) => b[k]);
    const mean = (k) => r2(pick(k).reduce((s, v) => s + v, 0) / bench.length);
    report.timings[`frame ${name}@${dpr}x`] = { paint: mean("paint"), raster: mean("raster"), put: mean("put"), total: mean("total"), totalMedian: r2(median(pick("total"))), totalP95: r2(p95(pick("total"))), items: bench[0].items, size: [d.w, d.h] };
  }
  await context.close();
}
for (const [k, v] of Object.entries(report.timings)) if (k.startsWith("frame")) console.log(`  ${k} (${v.size.join("×")} px, ${v.items} items): kernel paint ${v.paint} + vello_cpu ${v.raster} + putImageData ${v.put} ms; whole frame mean ${v.total} ms (p95 ${v.totalP95})`);

// The same with wasm SIMD (a build with `-C target-feature=+simd128` into
// target/web-host-spike-simd, if there is one): vello_cpu's fearless_simd
// then runs on simd128 instead of its scalar fallback.
if (existsSync(join(ROOT, "target/web-host-spike-simd/rapidr_ui_host_web_bg.wasm"))) {
  report.simd = {};
  for (const dpr of [1, 2]) {
    const { context, page } = await open(browser, dpr, "steady=1&pkg=web-host-spike-simd");
    for (const name of ["trackbar", "texts", "lists"]) {
      const d = desktop(name, dpr, "desktop-builtin");
      const web = await page.evaluate((n) => window.spike.pixels(n), name);
      const r = diff(web, d.rgba, d.w, d.h);
      ok(r.differing === 0, `${name}@${dpr}x with wasm SIMD: byte-identical to the desktop's capture with the same fonts (${r.differing} differ, max ${r.maxDiff})`);
      const bench = await page.evaluate((n) => window.spike.bench(n, 60), name);
      const mean = (k) => r2(bench.reduce((s, b) => s + b[k], 0) / bench.length);
      report.simd[`${name}@${dpr}x`] = { differing: r.differing, raster: mean("raster"), total: mean("total") };
      console.log(`  frame ${name}@${dpr}x with wasm SIMD: vello_cpu ${mean("raster")} ms, whole frame ${mean("total")} ms`);
    }
    await context.close();
  }
}

// A window the size of an IDE (forms::BIG: 1200 × 760, 187 components),
// drawn whole each frame — the worst case; the plan's damage rectangles
// make most frames a component's.
report.big = {};
for (const pkg of ["web-host-spike", "web-host-spike-simd"]) {
  if (!existsSync(join(ROOT, `target/${pkg}/rapidr_ui_host_web_bg.wasm`))) continue;
  for (const dpr of [1, 2]) {
    const { context, page } = await open(browser, dpr, `steady=1&forms=big&pkg=${pkg}`);
    const d = desktop("big", dpr, "desktop-builtin");
    const web = await page.evaluate(() => window.spike.pixels("big"));
    const r = d ? diff(web, d.rgba, d.w, d.h) : { differing: -1 };
    ok(r.differing === 0, `big@${dpr}x (${pkg}): byte-identical to the desktop's capture with the same fonts (${r.differing} differ)`);
    const bench = await page.evaluate(() => window.spike.bench("big", 30));
    const mean = (k) => r2(bench.reduce((s, b) => s + b[k], 0) / bench.length);
    report.big[`${pkg}@${dpr}x`] = { paint: mean("paint"), raster: mean("raster"), put: mean("put"), total: mean("total"), items: bench[0].items, size: d ? [d.w, d.h] : null };
    console.log(`  frame big@${dpr}x (${pkg}, ${d?.w}×${d?.h} px, ${bench[0].items} items): kernel paint ${mean("paint")} + vello_cpu ${mean("raster")} + putImageData ${mean("put")} = ${mean("total")} ms`);
    await context.close();
  }
}

// ------------------------------------------------------------- input --

{
  const { context, page } = await open(browser, 2, "");
  const box = async (name) => page.locator(`canvas[data-form=${name}]`).boundingBox();
  const tree = async (name) => JSON.parse(await page.evaluate((n) => window.spike.access(n), name));
  const find = (t, pred) => { let r = null; const walk = (n) => { if (!r && pred(n)) r = n; (n.children || []).forEach(walk); }; walk(t); return r; };
  const named = async (form, role, nameRe) => find(await tree(form), (n) => n.role === role && nameRe.test(n.name || ""));

  // The track bar: a click right of the thumb pages toward it, → moves a line;
  // then Report shows the OnChange log (the fixture's handlers).
  let b = await box("trackbar");
  await page.mouse.click(b.x + 150, b.y + 12);
  await page.keyboard.press("ArrowRight");
  await page.mouse.click(b.x + 37, b.y + 102);
  await page.evaluate(() => window.spike.idle());
  const lbl = await named("trackbar", "label", /\|/);
  report.input.trackbar = lbl?.name;
  ok(/^102111\|10\|5,6,\|4$/.test(lbl?.name || ""), `track bar: page click, → and Report through the kernel's routing ("${lbl?.name}")`);

  // An edit: a click puts the caret, typing goes to the model.
  b = await box("texts");
  await page.mouse.click(b.x + 140, b.y + 218);
  const focused = await page.evaluate(() => [document.activeElement?.tagName, window.spike.focusInfo("texts").id]);
  ok(focused[0] === "INPUT" && focused[1] === "name", `a click on an edit focuses it, and its mirror's <input> has the DOM focus (${focused})`);
  await page.keyboard.press("End");
  await page.keyboard.type(" web");
  let info = await page.evaluate(() => window.spike.focusInfo("texts"));
  ok(info.text === "RapidR web", `typed into the edit: "${info.text}"`);
  const field = await page.evaluate(() => [document.activeElement.value, document.activeElement.selectionStart]);
  ok(field[0] === "RapidR web" && field[1] === 10, `the mirror's field follows the text and the caret (${field})`);

  // An input method's composition (CDP: what an IME sends the page).
  const cdp = await context.newCDPSession(page);
  await cdp.send("Input.imeSetComposition", { text: "に", selectionStart: 1, selectionEnd: 1 });
  await cdp.send("Input.imeSetComposition", { text: "にほ", selectionStart: 2, selectionEnd: 2 });
  info = await page.evaluate(() => window.spike.focusInfo("texts"));
  ok(info.text === "RapidR web", `a composition isn't the text yet ("${info.text}")`);
  await cdp.send("Input.insertText", { text: "日本" });
  await page.evaluate(() => window.spike.idle());
  info = await page.evaluate(() => window.spike.focusInfo("texts"));
  ok(info.text === "RapidR web日本", `the composition committed into the edit ("${info.text}")`);
  // (a mobile keyboard's text: beforeinput, no key)
  await cdp.send("Input.insertText", { text: "!" });
  info = await page.evaluate(() => window.spike.focusInfo("texts"));
  ok(info.text === "RapidR web日本!", `text inserted without a key event (a phone's keyboard, dictation): "${info.text}"`);
  report.input.ime = info.text;

  // The clipboard: select all, copy, paste at the end.
  const mod = process.platform === "darwin" ? "Meta" : "Control";
  await page.keyboard.press(`${mod}+A`);
  const copied = await page.evaluate(() => new Promise((res) => {
    const el = document.activeElement;
    const dt = new DataTransfer();
    const e = new ClipboardEvent("copy", { clipboardData: dt, bubbles: true, cancelable: true });
    el.dispatchEvent(e);
    res(dt.getData("text/plain"));
  }));
  ok(copied === "RapidR web日本!", `copy gives the edit's selection ("${copied}")`);
  await page.keyboard.press("End");
  await page.evaluate(() => {
    const dt = new DataTransfer();
    dt.setData("text/plain", "+");
    document.activeElement.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
  });
  info = await page.evaluate(() => window.spike.focusInfo("texts"));
  ok(info.text === "RapidR web日本!+", `paste types the clipboard's text ("${info.text}")`);

  // The tab control: a click on a tab (OnChange → the program sets the
  // panel's caption).
  await page.mouse.click(b.x + 100, b.y + 10);
  if (process.env.SPIKE_DEBUG) console.log("    log:", await page.evaluate(() => document.getElementById("log").textContent.split("\n").slice(-12).join(" / ")), JSON.stringify(b));
  await page.evaluate(() => window.spike.idle());
  const panel = await named("texts", "pane", /Panel/);
  ok(panel?.name === "Panel 3", `a tab clicked (Tab 2; Tab 1 was selected, InsertTab keeps it): the program heard OnChange ("${panel?.name}")`);

  // A list box: a click picks; the wheel scrolls.
  b = await box("lists");
  await page.mouse.click(b.x + 260, b.y + 10 + 2 + 16 * 2 + 8);
  await page.evaluate(() => window.spike.idle());
  const pick = await named("lists", "label", /ItemIndex/);
  ok(pick?.name === "ItemIndex 2", `a list row clicked ("${pick?.name}")`);
  await page.mouse.move(b.x + 260, b.y + 60);
  await page.mouse.wheel(0, 150);
  await page.evaluate(() => window.spike.idle());
  const top = await page.evaluate(() => window.spike.dump("lists").split("\n").find((l) => l.includes("\"Apple\"")) ?? "");
  ok(top === "", `the wheel scrolled the list (Apple out of view)`);
  // The canvas: OnClick → the program draws again.
  await page.mouse.click(b.x + 47, b.y + 112);
  await page.evaluate(() => window.spike.idle());
  await page.locator("canvas[data-form=lists]").screenshot({ path: join(OUT, "lists-after-input@2x.png") });
  await page.locator("canvas[data-form=texts]").screenshot({ path: join(OUT, "texts-after-input@2x.png") });

  const m = await page.evaluate(() => ({ latency: window.spike.metrics.latency, handler: window.spike.metrics.handler, mirror: Object.values(window.spike.metrics.frames).flat().filter((f) => !f.first && f.a11y !== undefined).map((f) => f.a11y) }));
  report.timings.mirrorSync = { n: m.mirror.length, mean: r2(m.mirror.reduce((s, v) => s + v, 0) / Math.max(1, m.mirror.length)), p95: r2(p95(m.mirror)) };
  console.log(`  ARIA mirror kept in step after a frame: mean ${report.timings.mirrorSync.mean} ms, p95 ${report.timings.mirrorSync.p95} ms (${m.mirror.length} frames)`);
  report.timings.inputToFrame = { n: m.latency.length, median: r2(median(m.latency)), p95: r2(p95(m.latency)) };
  report.timings.inputHandler = { n: m.handler.length, median: r2(median(m.handler)), p95: r2(p95(m.handler)) };
  console.log(`  input → frame drawn: median ${report.timings.inputToFrame.median} ms, p95 ${report.timings.inputToFrame.p95} ms (${m.latency.length} events); handlers ${report.timings.inputHandler.median} ms`);
  await context.close();
}

// A phone (emulated: touch, devicePixelRatio 3): a tap on an edit puts the
// DOM focus on its text field inside the gesture — what makes a phone show
// its keyboard — and the keyboard's text (no key events) reaches the edit.
{
  const context = await browser.newContext({ hasTouch: true, isMobile: true, deviceScaleFactor: 3, viewport: { width: 420, height: 900 } });
  const page = await context.newPage();
  await page.goto(`${BASE}/tests/web_host_spike.html?forms=texts`);
  await page.waitForFunction(() => window.spike?.ready, null, { timeout: 60_000 });
  const b = await page.locator("canvas[data-form=texts]").boundingBox();
  await page.touchscreen.tap(b.x + 140, b.y + 218);
  const tag = await page.evaluate(() => document.activeElement?.tagName);
  await page.keyboard.insertText("ok");
  const info = await page.evaluate(() => window.spike.focusInfo("texts"));
  report.input.touch = { focused: tag, text: info.text };
  ok(tag === "INPUT" && info.id === "name" && info.text.endsWith("ok"), `a tap on an edit focuses its field (${tag}) and a phone keyboard's text types ("${info.text}")`);
  await context.close();
}

// ----------------------------------------------------- accessibility --

// The kernel's roles (AccessNode::to_json, ARIA's names) and what Chrome
// calls the node a screen reader gets for each (tests/web_a11y.mjs' table).
const ROLES = {
  window: ["dialog"], dialog: ["dialog"], pane: ["group"], group: ["group"], status: ["status"],
  label: ["StaticText"], button: ["button"], checkbox: ["checkbox"], radio: ["radio"],
  textbox: ["textbox"], "textbox-multiline": ["textbox"], slider: ["slider"], spinbutton: ["spinbutton"],
  progressbar: ["progressbar"], tablist: ["tablist"], tab: ["tab"], listbox: ["listbox"], option: ["option"],
  combobox: ["combobox"], tree: ["tree"], treeitem: ["treeitem"], grid: ["grid"], row: ["row"], gridcell: ["gridcell"],
  menubar: ["menubar"], menuitem: ["menuitem"], img: ["image", "img"], canvas: ["Canvas"], separator: ["separator", "splitter"],
  generic: ["generic"],
};

/// Chrome's tree for the mirror's elements against the kernel's trees of
/// every form: { compared, diffs, focused }.
async function compareTrees(page, cdp, active = "trackbar") {
  const { root } = await cdp.send("DOM.getDocument", { depth: -1, pierce: true });
  const byElementId = new Map();
  const walk = (n) => {
    const a = n.attributes || [];
    for (let i = 0; i + 1 < a.length; i += 2) if (a[i] === "id") byElementId.set(a[i + 1], n.backendNodeId);
    (n.children || []).forEach(walk);
  };
  walk(root);
  const { nodes } = await cdp.send("Accessibility.getFullAXTree");
  const byBackend = new Map();
  for (const n of nodes) if (n.backendDOMNodeId !== undefined && !byBackend.has(n.backendDOMNodeId)) byBackend.set(n.backendDOMNodeId, n);
  const byAx = new Map(nodes.map((n) => [n.nodeId, n]));
  const prop = (n, name) => (n.properties || []).find((p) => p.name === name)?.value?.value;
  const squash = (s) => String(s ?? "").replace(/\s+/g, " ").trim();
  let compared = 0;
  const diffs = [];
  let kernelFocus = null;
  for (const form of ["trackbar", "texts", "lists"]) {
    const kernel = JSON.parse((await page.evaluate((n) => window.spike.access(n), form)).replace(/"(id|labelledBy)":(\d+)/g, '"$1":"$2"'));
    const visit = (k) => {
      const where = `${form} ${k.role} "${k.name}"`;
      const backend = byElementId.get(`rrn-${form}-${k.id}`);
      let w = backend !== undefined ? byBackend.get(backend) : null;
      if ((k.states || []).includes("focused") && form === active) kernelFocus = w;
      if (k.role === "label" && w) {
        // (a label: the text it shows)
        const texts = (w.childIds || []).map((c) => byAx.get(c)).filter((c) => c?.role?.value === "StaticText");
        w = texts[0] || (w.role?.value === "StaticText" ? w : null);
      }
      compared++;
      if (!w || w.ignored) { diffs.push(`${where}: not in Chrome's tree`); (k.children || []).forEach(visit); return; }
      const out = [];
      const roles = ROLES[k.role] || [k.role];
      if (!roles.includes(w.role?.value)) out.push(`role ${w.role?.value}`);
      if (squash(k.name) !== squash(w.name?.value)) out.push(`name "${squash(w.name?.value)}"`);
      if (k.value !== undefined && k.role.startsWith("textbox") && String(w.value?.value ?? "") !== k.value.replace(/\r\n/g, "\n")) out.push(`value ${JSON.stringify(w.value?.value)}`);
      if (k.numeric) {
        const num = (v) => (v === undefined ? undefined : Number(v));
        if (num(prop(w, "valuemin")) !== k.numeric.min || num(prop(w, "valuemax")) !== k.numeric.max || num(w.value?.value) !== k.numeric.value) out.push(`numbers ${prop(w, "valuemin")}..${prop(w, "valuemax")} = ${w.value?.value}`);
      }
      const states = new Set(k.states || []);
      const tri = (s) => (states.has(s) ? true : states.has("not-" + s) ? false : undefined);
      if (!!prop(w, "disabled") !== states.has("disabled")) out.push(`disabled ${!!prop(w, "disabled")}`);
      const checked = k.role === "button" ? prop(w, "pressed") : prop(w, "checked");
      if (tri("checked") !== undefined && String(checked) !== String(tri("checked"))) out.push(`checked ${checked}`);
      if (tri("selected") !== undefined && !!prop(w, "selected") !== tri("selected")) out.push(`selected ${prop(w, "selected")}`);
      if (tri("expanded") !== undefined && prop(w, "expanded") !== tri("expanded")) out.push(`expanded ${prop(w, "expanded")}`);
      if (k.role === "textbox-multiline" && !prop(w, "multiline")) out.push("not multiline");
      if (k.level !== undefined && Number(prop(w, "level")) !== k.level) out.push(`level ${prop(w, "level")}`);
      const ks = (k.shortcut || "").toLowerCase(), ws = String(prop(w, "keyshortcuts") || "").toLowerCase();
      if (ks !== ws) out.push(`shortcut "${ws}"`);
      if (squash(k.description) !== squash(w.description?.value)) out.push(`description "${w.description?.value}"`);
      out.forEach((d) => diffs.push(`${where}: ${d}`));
      (k.children || []).forEach(visit);
    };
    visit(kernel);
  }
  // (Chrome's focused node, and whether it's the kernel's focused one in
  // the window in use)
  const focusedAx = nodes.find((n) => prop(n, "focused") && n.role?.value !== "RootWebArea");
  return { compared, diffs, focused: focusedAx ? `${focusedAx.role?.value} "${focusedAx.name?.value}"` : null, same: !!focusedAx && focusedAx === kernelFocus };
}

{
  const { context, page } = await open(browser, 1);
  const cdp = await context.newCDPSession(page);
  // (a headless page has no window focus: emulate it, so Chrome reports the
  // focused node as it would to a screen reader)
  await cdp.send("Emulation.setFocusEmulationEnabled", { enabled: true });
  report.a11y.rounds = [];
  const round = async (when, active) => {
    await page.evaluate(() => window.spike.idle());
    const r = await compareTrees(page, cdp, active);
    report.a11y.rounds.push({ when, ...r });
    console.log(`  accessibility ${when}: ${r.compared} kernel nodes compared with Chrome's tree, ${r.diffs.length} differences; Chrome's focus: ${r.focused}`);
    r.diffs.forEach((d) => console.log(`    ${d}`));
    ok(r.diffs.length === 0, `${when}: Chrome's accessibility tree says what the kernel's says (${r.compared} nodes)`);
    return r;
  };
  const first = await round("at load", "trackbar");
  ok(first.same, `at load: the kernel's focus (the track bar) is Chrome's focused node (${first.focused})`);
  // After input: the track bar moved by keys, the check box checked, a
  // list row picked, a tab picked — the states follow.
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  const lists = await page.locator("canvas[data-form=lists]").boundingBox();
  await page.mouse.click(lists.x + 260, lists.y + 10 + 2 + 16 * 4 + 8);
  const texts = await page.locator("canvas[data-form=texts]").boundingBox();
  await page.mouse.click(texts.x + 245, texts.y + 278);
  await page.mouse.click(texts.x + 100, texts.y + 10);
  const after = await round("after input", "texts");
  // (the kernel focuses the tab control, so its node, a tablist, is what
  // has the focus — on the desktop as here; ARIA would rather focus the
  // selected tab: a kernel describe change for both hosts, docs/web-host-plan.md)
  ok(after.same, `after input: the kernel's focus (the tab control clicked) is Chrome's focused node (${after.focused})`);
  // A screen reader's click on the mirror (VoiceOver's VO-Space, NVDA's
  // Enter on a button): the kernel's OnClick.
  await page.evaluate(() => document.querySelector("#rrn-trackbar-" + [...window.spike.win("trackbar").nodes.keys()].find((id) => window.spike.win("trackbar").nodes.get(id).getAttribute("role") === "button")).click());
  await page.evaluate(() => window.spike.idle());
  const caption = JSON.parse(await page.evaluate(() => window.spike.access("trackbar")).then((s) => s.replace(/"(id|labelledBy)":(\d+)/g, '"$1":"$2"')));
  const label = (caption.children || []).find((c) => c.role === "label");
  ok(/\|4$/.test(label?.name || ""), `a screen reader's click on the Report button ran its OnClick ("${label?.name}")`);
  report.a11y.screenReaderClick = label?.name;
  await context.close();
}

/// What the screen shows of form `name`'s canvas (an element screenshot,
/// decoded by the page): RGBA, device pixels.
async function shown(page, name) {
  const png = (await page.locator(`canvas[data-form=${name}]`).screenshot()).toString("base64");
  return page.evaluate(async (b64) => {
    const img = new Image();
    img.src = "data:image/png;base64," + b64;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const x = c.getContext("2d", { willReadFrequently: true });
    x.drawImage(img, 0, 0);
    return Array.from(x.getImageData(0, 0, c.width, c.height).data);
  }, png);
}

// (the screenshot path checked on the CPU canvases first: it must give the
// wasm's pixels)
{
  const { context, page } = await open(browser, 2);
  const shot = await shown(page, "trackbar");
  const web = await page.evaluate(() => window.spike.pixels("trackbar"));
  const d = desktop("trackbar", 2, "desktop-builtin");
  const r = diff(shot, web, d.w, d.h);
  ok(shot.length === web.length && r.differing === 0, `a screenshot of a CPU canvas is its pixels (${r.differing} differ)`);
  await context.close();
}
await browser.close();

// -------------------------------------------------- vello on WebGPU --

// The same forms drawn by vello on the browser's WebGPU (the build with
// feature gpu), where Chrome has an adapter: headless Chrome on macOS needs
// --enable-unsafe-webgpu and ANGLE on Metal for the real GPU (without them
// it has none, or SwiftShader's CPU adapter).
if (existsSync(join(ROOT, "target/web-host-spike-gpu/rapidr_ui_host_web_bg.wasm"))) {
  const gpuBrowser = await chromium.launch({ args: ["--enable-unsafe-webgpu", "--use-angle=metal", "--enable-features=WebGPU"] });
  const { context, page, errors } = await open(gpuBrowser, 2, "steady=1&gpu=1");
  const m = await page.evaluate(() => ({ gpu: window.spike.metrics.gpu, firstFrame: window.spike.metrics.firstFrame, load: window.spike.metrics.load, renderers: window.spike.wins.map((w) => !!w.gpu) }));
  report.gpu = { adapter: m.gpu, firstFrame: r2(m.firstFrame), load: m.load, pixels: {}, frames: {} };
  const on = m.renderers.every(Boolean);
  ok(on && errors.length === 0, `WebGPU: vello draws every form (${m.gpu}) ${errors.join("; ")}`);
  if (on) {
    console.log(`  WebGPU: first frame at ${r2(m.firstFrame)} ms (the device and vello's pipelines first)`);
    await page.screenshot({ path: join(OUT, "page-webgpu@2x.png") });
    for (const name of ["trackbar", "texts", "lists"]) {
      const d = desktop(name, 2, "desktop-builtin");
      const r = diff(await shown(page, name), d.rgba, d.w, d.h);
      report.gpu.pixels[`${name}@2x`] = r;
      console.log(`  ${name}@2x WebGPU vs the desktop's CPU capture: ${r.differing} of ${r.pixels} differ (${r.over8} by > 8, max ${r.maxDiff})`);
      ok(r.over8 <= r.pixels * 0.005, `${name}@2x: WebGPU within the desktop matrix's tolerance of the CPU capture`);
      const bench = await page.evaluate((n) => window.spike.bench(n, 60), name);
      const mean = (k) => r2(bench.reduce((s, b) => s + b[k], 0) / bench.length);
      report.gpu.frames[name] = { paint: mean("paint"), encodeAndSubmit: mean("gpu"), total: mean("total") };
      console.log(`  frame ${name}@2x on WebGPU: kernel paint ${mean("paint")} + scene, encode, submit ${mean("gpu")} ms (the GPU's own time not waited for)`);
    }
  }
  await context.close();
  await gpuBrowser.close();
}

writeFileSync(join(OUT, "report.json"), JSON.stringify(report, null, 1));
console.log(`\n${passed} passed, ${failed} failed (report: ${join(OUT, "report.json")})`);
process.exit(failed ? 1 : 0);
