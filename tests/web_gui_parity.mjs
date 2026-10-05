// The GUI fixtures of tests/native_gui_events.mjs (tests/gui_parity_cases.mjs)
// in the browser: each fixture runs in the web IDE's preview, the same events
// are fired (a click on the component), and the same properties must read
// what the desktop, native and interpreted builds show. (Running a fixture
// and firing its events: tests/web_gui_run.mjs, which tests/web_a11y.mjs
// shares.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765, or RAPIDR_URL):  node tests/web_gui_parity.mjs [filter …]

//
// RAPIDR_WEB_HOST=kernel: the UI kernel draws the forms instead (Stage W3,
// docs/web-host-plan.md; tests/web_kernel.html?host=kernel), the case's
// events played by the desktop's own test hooks. Besides the expected
// lines, with RAPIDR_DESKTOP_CAPTURES=<dir> (the desktop's runs laid out as
// <case>@<scale>x/window-<n>.bmp and a11y.json: tests/gui_captures.mjs) each
// window's capture must be byte-identical to the desktop's and its
// accessibility tree equal. A case that doesn't run on the kernel host yet
// is counted as pending (`webKernel: "pending: why"` in the case table),
// not failed.

import { readFileSync, existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { cases } from "./gui_parity_cases.mjs";
import { openIde, runCase, runCaseKernel, WEB_HOST } from "./web_gui_run.mjs";

const filters = process.argv.slice(2);
let failed = 0, passed = 0, skipped = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };

/// How two BMP captures differ: their sizes, or how many pixels and where.
function bmpDiff(a, b) {
  const dim = (x) => [x.readInt32LE(18), x.readInt32LE(22)];
  const [aw, ah] = dim(a), [bw, bh] = dim(b);
  if (aw !== bw || ah !== bh) return `${aw}×${ah} against ${bw}×${bh}`;
  const off = (x) => x.readUInt32LE(10), bpp = a.readUInt16LE(28) / 8, stride = (aw * bpp + 3) & ~3;
  let n = 0, box = [aw, ah, -1, -1];
  for (let y = 0; y < Math.abs(ah); y++) for (let x = 0; x < aw; x++) {
    const i = y * stride + x * bpp;
    if (a[off(a) + i] !== b[off(b) + i] || a[off(a) + i + 1] !== b[off(b) + i + 1] || a[off(a) + i + 2] !== b[off(b) + i + 2]) {
      n++;
      const row = ah > 0 ? ah - 1 - y : y;
      box = [Math.min(box[0], x), Math.min(box[1], row), Math.max(box[2], x), Math.max(box[3], row)];
    }
  }
  return `${n} pixels, in ${box[0]},${box[1]}–${box[2]},${box[3]}`;
}

if (WEB_HOST === "kernel") {
  const dpr = Number(process.env.RAPIDR_DPR || 1);
  const desk = process.env.RAPIDR_DESKTOP_CAPTURES;
  const browser = await chromium.launch();
  const tally = { run: 0, expects: 0, pending: 0, pixelsSame: 0, pixelsDiffer: 0, a11ySame: 0, a11yDiffer: 0 };
  const pendingNames = [];
  for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
    if (c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.why || "no browser counterpart"})`); continue; }
    const pending = typeof c.webKernel === "string" ? c.webKernel : null;
    const { results, errors, page } = await runCaseKernel(browser, c, dpr);
    await page.close();
    tally.run++;
    const text = results ? results.dump.join("\n") : `(no results: ${errors.join("; ")})`;
    const good = results && c.expect.every((l) => text.includes(l));
    if (pending && !good) {
      tally.pending++;
      pendingNames.push(c.name);
      console.log(`- ${c.name} (kernel): ${pending}`);
      continue;
    }
    for (const line of c.expect) ok(text.includes(line), `${c.name} (kernel): ${line}${text.includes(line) ? "" : `   [got: ${text.replace(/\n/g, " ; ")}]`}`);
    if (good) tally.expects++;
    ok(errors.length === 0, `${c.name} (kernel): no page errors${errors.length ? ` (${errors.join("; ")})` : ""}`);
    if (!desk || !results) continue;
    const dir = join(desk, `${c.name}@${dpr}x`);
    // (the desktop's captures, bottom to top, as the page's)
    results.captures.forEach((shot, i) => {
      const file = join(dir, `window-${i + 1}.bmp`);
      if (!existsSync(file)) { console.log(`- ${c.name} (kernel): no desktop capture ${i + 1}`); return; }
      const web = Buffer.from(shot.bmp, "base64");
      const desktop = readFileSync(file);
      const same = web.equals(desktop);
      same ? tally.pixelsSame++ : tally.pixelsDiffer++;
      // (RAPIDR_SHOT=dir: the browser's captures kept, to look at)
      if (process.env.RAPIDR_SHOT) writeFileSync(join(process.env.RAPIDR_SHOT, `${c.name}@${dpr}x-${i + 1}.bmp`), web);
      console.log(`${same ? "✓" : "≠"} ${c.name} (kernel): window ${i + 1} '${shot.title}' pixels ${same ? "byte-identical to the desktop's" : `differ from the desktop's (${bmpDiff(web, desktop)})`}`);
    });
    const a11yFile = join(dir, "a11y.json");
    if (existsSync(a11yFile)) {
      // (the very bytes RAPIDR_TEST_A11Y writes on the desktop)
      const same = readFileSync(a11yFile, "utf8") === results.a11y;
      if (process.env.RAPIDR_SHOT) writeFileSync(join(process.env.RAPIDR_SHOT, `${c.name}@${dpr}x.a11y.json`), results.a11y);
      same ? tally.a11ySame++ : tally.a11yDiffer++;
      console.log(`${same ? "✓" : "≠"} ${c.name} (kernel): accessibility trees ${same ? "equal to the desktop's" : "differ from the desktop's"}`);
    }
  }
  await browser.close();
  console.log(`\nKernel host: ${tally.run} cases run, ${tally.expects} with every expected line, ${tally.pending} pending (${pendingNames.join(", ")}); ` +
    `windows byte-identical to the desktop's: ${tally.pixelsSame} of ${tally.pixelsSame + tally.pixelsDiffer}; accessibility trees equal: ${tally.a11ySame} of ${tally.a11ySame + tally.a11yDiffer}`);
  console.log(`Web GUI parity (kernel host): ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
  if (failed) process.exit(1);
  process.exit(0);
}

// (`RAPIDR_DPR=2`: a high-DPI screen — what programs read must not change)
const { browser, page, pageErrors } = await openIde(Number(process.env.RAPIDR_DPR || 1));

for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  if (c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.why || "no browser counterpart"})`); continue; }
  const { frame, missing } = await runCase(page, c);
  if (!frame) { ok(false, `${c.name}: preview frame`); continue; }
  for (const target of missing) ok(false, `${c.name}: ${target} exists`);
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
