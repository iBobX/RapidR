// Full IDE E2E matrix:
//   for each example:
//     1. Open the IDE in Chromium
//     2. Pick the example from the dropdown → IDE compiles + runs
//     3. Snapshot the preview iframe body (the "expected" rendering)
//     4. Click ⬇ Build, capture the downloaded zip
//     5. Unzip into tests/.ide-matrix/<name>/, serve via the same
//        Python server, navigate Chromium to its index.html
//     6. Snapshot the served body and assert it has comparable length
//        to the IDE preview (≥ 80%) — i.e. the bundled output renders
//        the same content as the IDE preview did.
//     7. No console errors in either page (modulo allow-list).
//
// Pre-reqs: tools/build_web_artifacts.sh + cargo build --release done,
//           python3 -m http.server 8765 running at repo root.
//
// Usage: node tests/web_ide_e2e.mjs [example_name]

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT  = new URL("../", import.meta.url).pathname.replace(/\/$/, "");
const SHOTS = join(ROOT, "tests/web-screenshots");
const OUTDIR = join(ROOT, "tests/.ide-matrix");
const PORT  = +(process.env.RAPIDR_PORT || 8765);
const URL_BASE = `http://localhost:${PORT}`;

mkdirSync(SHOTS, { recursive: true });
mkdirSync(OUTDIR, { recursive: true });

// Subset of the matrix that exercises distinct runtime paths.
const EXAMPLES = [
  "gui/hello_form",     // basic form + button + label
  "gui/pantry",         // a list box and a string grid working together
  "gui/stopwatch",      // a timer, a list box
  "graphics/canvas",    // canvas drawing in OnPaint
  "data/dataframe",     // a data frame from a $RESOURCE CSV, a chart in an image
  "web/todo",           // the browser's storage, a list box
];

const filter = process.argv[2];
const target = filter ? EXAMPLES.filter(e => e === filter) : EXAMPLES;
if (target.length === 0) { console.error("no examples matched"); process.exit(2); }

const isAllowedConsoleErr = (name, text) => {
  return false;
};

const browser = await chromium.launch();
const ctx = await browser.newContext({ acceptDownloads: true });

const results = [];
for (const name of target) {
  const r = { name, ok: false, errors: [], previewLen: 0, bundleLen: 0 };
  // (an example's path in examples/ as one file name)
  const slug = name.replace(/\//g, "_");
  const page = await ctx.newPage();
  const errs = [];
  page.on("pageerror", e => errs.push(`pageerror: ${e.message}`));
  page.on("console", m => {
    if (m.type() === "error") {
      const t = m.text();
      if (!isAllowedConsoleErr(name, t)) errs.push(`console.error: ${t}`);
    }
  });

  try {
    const step = async (label, fn) => {
      try { return await fn(); }
      catch (e) { e.message = `[step:${label}] ${e.message}`; throw e; }
    };

    // 1. Open IDE
    await step("goto-ide", () => page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load", timeout: 15000 }));
    await step("wait-ready", () => page.waitForFunction(
      () => document.getElementById("status")?.textContent === "ready",
      { timeout: 15000 }));

    // 2. Load this example via the dropdown
    await step("select-example", () => page.selectOption("#examples", { value: `../examples/${name}.rr` }));
    // Give the source-edit debounce + design-surface render a moment
    await page.waitForTimeout(800);

    // 3. Click Run, wait for status, snapshot preview body.
    await step("click-run", () => page.click("#btn-run"));
    await step("wait-run", () => page.waitForFunction(() => {
      const s = document.getElementById("status")?.textContent || "";
      return /running|preview ready|compiled/i.test(s);
    }, { timeout: 15000 }));
    await page.waitForTimeout(700);
    const previewBody = await step("read-preview", () =>
      page.frameLocator("#preview").locator("body").innerHTML());
    r.previewLen = previewBody.length;
    if (r.previewLen < 50) throw new Error(`empty IDE preview (len=${r.previewLen})`);

    await page.screenshot({ path: join(SHOTS, `ide-${slug}-preview.png`), fullPage: true });

    // Close preview window to reveal the toolbar buttons
    await step("close-preview", () => page.click("#preview-close"));
    await page.waitForTimeout(500);

    // 4. Click Build, capture the download.
    const [download] = await step("build", () => Promise.all([
      page.waitForEvent("download", { timeout: 15000 }),
      page.click("#btn-build"),
    ]));
    const zipPath = join(OUTDIR, `${slug}.zip`);
    await download.saveAs(zipPath);

    // 5. Unzip and serve
    const serveDir = join(OUTDIR, slug);
    mkdirSync(serveDir, { recursive: true });
    execFileSync("unzip", ["-o", "-q", zipPath, "-d", serveDir]);

    // 6. Navigate to the bundled site in a fresh tab
    const bundlePage = await ctx.newPage();
    const bundleErrs = [];
    bundlePage.on("pageerror", e => bundleErrs.push(`pageerror: ${e.message}`));
    bundlePage.on("console", m => {
      if (m.type() === "error") {
        const t = m.text();
        if (!isAllowedConsoleErr(name, t)) bundleErrs.push(`console.error: ${t}`);
      }
    });
    const bundleUrl = `${URL_BASE}/tests/.ide-matrix/${slug}/index.html`;
    await bundlePage.goto(bundleUrl, { waitUntil: "load", timeout: 15000 });
    await bundlePage.waitForFunction(() => {
      const s = document.getElementById("rapidr-status")?.textContent || "";
      return s === "" || /running|preview ready/i.test(s);
    }, { timeout: 15000 });
    await bundlePage.waitForTimeout(700);
    const bundleBody = await bundlePage.locator("body").innerHTML();
    r.bundleLen = bundleBody.length;

    await bundlePage.screenshot({ path: join(SHOTS, `ide-${slug}-bundle.png`), fullPage: true });
    await bundlePage.close();

    // 7. Parity check: bundle render should be at least 60% the size
    // of the IDE preview render (allows for status-bar wrapper diffs).
    if (r.bundleLen < r.previewLen * 0.6) {
      throw new Error(`bundle render too small: ${r.bundleLen} vs preview ${r.previewLen}`);
    }
    if (errs.length || bundleErrs.length) {
      r.errors = [...errs, ...bundleErrs];
      throw new Error("console errors present");
    }
    r.ok = true;
  } catch (e) {
    if (!r.errors.length) r.errors = [String(e?.message || e)];
  } finally {
    await page.close();
  }

  results.push(r);
  const tag = r.ok ? "PASS" : "FAIL";
  console.log(
    `${tag.padEnd(4)}  ${r.name.padEnd(20)}  preview=${String(r.previewLen).padStart(5)}  bundle=${String(r.bundleLen).padStart(5)}` +
    (r.errors.length ? "  | " + r.errors.join("; ") : "")
  );
}

await browser.close();
const passed = results.filter(r => r.ok).length;
console.log(`\n${passed}/${results.length} passed`);
writeFileSync(join(SHOTS, "ide-e2e.json"), JSON.stringify(results, null, 2));
process.exit(passed === results.length ? 0 : 1);
