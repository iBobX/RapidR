// Round 4 / item #10 — true end-to-end build test.
//
//   1. Drive the IDE to build a small project (a button that, when clicked,
//      writes a known string into a Label) into a .zip bundle.
//   2. Unzip it into a temp dir.
//   3. Spawn `python3 -m http.server` on an isolated port serving the temp dir.
//   4. Open the bundle in a fresh Playwright page.
//   5. Click the button and assert the label text changes.
//
//   If this passes, we know the IDE → build → bundle → standalone-runtime
//   path is working end-to-end.
//
// Usage:  node tests/web_ide_e2e_build.mjs

import { chromium } from "playwright";
import { spawn } from "node:child_process";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as os from "node:os";
import { createWriteStream } from "node:fs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const E2E_PORT = Number(process.env.RAPIDR_E2E_PORT || 8788);

function ok(cond, msg) {
  if (!cond) throw new Error("ASSERT FAILED: " + msg);
  console.log("✓ " + msg);
}

// Tiny ZIP reader (STORED only) — same shape as web_ide_assets.mjs.
function parseZip(u8) {
  const out = new Map(); let i = 0;
  const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  while (i + 4 <= u8.length) {
    const sig = dv.getUint32(i, true);
    if (sig !== 0x04034b50) break;
    const method = dv.getUint16(i + 8, true);
    const compSize = dv.getUint32(i + 18, true);
    const uncSize = dv.getUint32(i + 22, true);
    const nameLen = dv.getUint16(i + 26, true);
    const extraLen = dv.getUint16(i + 28, true);
    const name = new TextDecoder().decode(u8.subarray(i + 30, i + 30 + nameLen));
    const dataStart = i + 30 + nameLen + extraLen;
    const data = u8.subarray(dataStart, dataStart + compSize);
    out.set(name, { method, data, uncSize });
    i = dataStart + compSize;
  }
  return out;
}

async function unzipTo(zipBytes, destDir) {
  const entries = parseZip(zipBytes);
  for (const [name, e] of entries) {
    if (e.method !== 0) throw new Error(`unsupported method ${e.method} for ${name}`);
    const outPath = path.join(destDir, name);
    await fs.mkdir(path.dirname(outPath), { recursive: true });
    await fs.writeFile(outPath, e.data);
  }
  return Array.from(entries.keys());
}

function startServer(dir, port) {
  const proc = spawn("python3", ["-m", "http.server", String(port)], {
    cwd: dir,
    stdio: ["ignore", "pipe", "pipe"],
  });
  return proc;
}

async function waitFor(url, timeoutMs = 10000) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    try {
      const r = await fetch(url);
      if (r.ok || r.status === 404) return true;
    } catch (_) {}
    await new Promise(r => setTimeout(r, 100));
  }
  throw new Error(`server at ${url} did not start in ${timeoutMs}ms`);
}

async function main() {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({
    acceptDownloads: true,
    viewport: { width: 1400, height: 900 },
  });
  let serverProc = null;
  let tmpDir = null;
  try {
    const page = await ctx.newPage();
    page.on("console", m => { if (m.type() === "error") console.log("[ide-error]", m.text()); });
    await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "networkidle" });
    await page.waitForFunction(() => window.RapidR && window.RapidR.state.wasmReady, null, { timeout: 30000 });

    // 1. Build a tiny project: Button "Go" → Label.text = "hello e2e"
    await page.evaluate(() => {
      const R = window.RapidR;
      const proj = R.state.project;
      proj.name = "e2eapp";
      const form = proj.forms[0];
      form.name = "Form1";
      form.props.caption = "E2E";
      form.children = [
        {
          name: "Label1",
          type: "RLabel",
          props: { left: 16, top: 16, width: 200, height: 20, caption: "ready" },
          code: { handlers: {} },
        },
        {
          name: "Button1",
          type: "RButton",
          props: { left: 16, top: 48, width: 100, height: 24, caption: "Go" },
          code: { handlers: { OnClick: "Button1_OnClick" } },
        },
      ];
      form.code = {
        handlers: {},
        source: 'SUB Button1_OnClick\n  Label1.caption = "hello e2e"\n  PRINT "printed on click"\nEND SUB\n',
      };
      R.renderActiveDesigner();
      R.renderProperties();
    });

    ok(true, "project model built");

    // 2. Build the zip and capture the download.
    const downloadPromise = page.waitForEvent("download");
    await page.evaluate(() => window.RapidR.runCommand("run.build"));
    const dl = await downloadPromise;
    tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "rapidr-e2e-"));
    const zipPath = path.join(tmpDir, "out.zip");
    await dl.saveAs(zipPath);
    const zipBytes = new Uint8Array(await fs.readFile(zipPath));
    const names = await unzipTo(zipBytes, tmpDir);
    ok(names.includes("index.html") && names.includes("loader.js") &&
       names.includes("rapidrintr.js") && names.includes("rapidrintr_bg.wasm") &&
       names.includes("manifest.json") && names.includes("e2eapp.rrbc"),
       `bundle has all required files (${JSON.stringify(names)})`);
    ok(names.includes("bundle_console.js") && names.includes("ansi_screen.js"),
       "bundle ships the on-page console (bundle_console.js, ansi_screen.js)");

    // 3. Verify manifest.json carries the IDE version.
    const manifest = JSON.parse(await fs.readFile(path.join(tmpDir, "manifest.json"), "utf8"));
    ok(manifest.rapidr_bundle === 1 && /^\d+\.\d+\.\d+$/.test(String(manifest.ide_version || "")),
       `manifest carries IDE version (${JSON.stringify(manifest)})`);

    // 4. Verify CSP meta tag is present in bundled index.html.
    const idxHtml = await fs.readFile(path.join(tmpDir, "index.html"), "utf8");
    ok(idxHtml.includes("Content-Security-Policy"),
       "bundled index.html ships a CSP meta tag");

    // 5. Spawn HTTP server on E2E_PORT serving the bundle.
    serverProc = startServer(tmpDir, E2E_PORT);
    const baseUrl = `http://127.0.0.1:${E2E_PORT}`;
    await waitFor(baseUrl + "/index.html", 10000);
    ok(true, `bundle server running on ${baseUrl}`);

    // 6. Open bundle in a fresh page and verify the program actually runs.
    const page2 = await ctx.newPage();
    page2.on("console", m => console.log(`[bundle-${m.type()}]`, m.text()));
    await page2.goto(baseUrl + "/index.html", { waitUntil: "networkidle" });
    // Wait for the runtime status text to clear (means init() resolved + run started).
    await page2.waitForFunction(() => {
      const s = document.getElementById("rapidr-status");
      return s && (s.textContent === "" || s.textContent === "loading…" === false);
    }, null, { timeout: 20000 }).catch(() => {});
    // Look for the Label widget (the runtime renders it as a DOM node).
    const initial = await page2.evaluate(() => document.body.innerText);
    ok(/ready/.test(initial), `bundle rendered Label1 initial caption (got "${initial.slice(0,200)}")`);

    // 7. Click the button and verify Label1 updates.
    // The runtime renders RButton as a real <button>; find by visible text.
    await page2.locator('button:has-text("Go")').first().click();
    await page2.waitForTimeout(200);
    const after = await page2.evaluate(() => document.body.innerText);
    ok(/hello e2e/.test(after),
       `Button click updated Label1 text (got "${after.slice(0,200)}")`);
    // 8. PRINT shows on the page, docked under the form, despite the CSP.
    const printed = await page2.evaluate(() => {
      const el = document.getElementById("rapidr-console");
      return el ? { text: el.textContent, docked: el.classList.contains("docked") } : null;
    });
    ok(printed && /printed on click/.test(printed.text) && printed.docked,
       `PRINT shown in the docked on-page console (${JSON.stringify(printed)})`);

    console.log("\nE2E build suite: ALL CHECKS PASSED");
  } finally {
    if (serverProc) { try { serverProc.kill("SIGTERM"); } catch (_) {} }
    if (tmpDir)     { try { await fs.rm(tmpDir, { recursive: true, force: true }); } catch (_) {} }
    await browser.close();
  }
}

main().catch(err => { console.error(err); process.exit(1); });
