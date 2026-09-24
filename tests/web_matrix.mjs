// Web matrix test: bundles every relevant .rr example with `rapidr
// bundle-bc`, serves it via the same Python static server used for
// the IDE, navigates Chromium to it, waits for the loader to reach
// the "running" / blank state, asserts no console errors, and
// screenshots into tests/web-screenshots/<example>-web.png.
//
// Pre-reqs:
//   - tools/build_web_artifacts.sh has produced target/web/*
//   - cargo build --release (./rapidr exists)
//   - python3 -m http.server 8765 running at repo root
//
// Usage:  node tests/web_matrix.mjs [example_name]

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const ROOT  = new URL("../", import.meta.url).pathname.replace(/\/$/, "");
const SHOTS = join(ROOT, "tests/web-screenshots");
const PORT  = +(process.env.RAPIDR_PORT || 8765);
const URL_BASE = `http://localhost:${PORT}`;

mkdirSync(SHOTS, { recursive: true });

// All examples that should run in a browser. Network-blocked or
// inherently impossible ones are commented with their reason.
const EXAMPLES = [
  // GUI / forms
  "hello_web", "gui_app", "web_calculator", "web_todo",
  "web_canvas", "web_dashboard",
  "test_canvas", "test_new_components", "test_gui_quick",
  // Data science
  "demo_num", "demo_dataframe", "demo_plot", "web_datascience",
  // Database
  "demo_sqlite",
  // Networking
  "demo_http",
  "demo_chat_client",      // RSocket → WS; will fail to connect (no server)
                            // but must not crash.
  // Skipped:
  // "demo_chat_server"  — cannot listen() in a browser
  // "demo_mysql"        — no MySQL driver in the web runtime
];

const filter = process.argv[2];
const target = filter ? EXAMPLES.filter(e => e === filter) : EXAMPLES;
if (target.length === 0) {
  console.error("no examples matched filter");
  process.exit(2);
}

const browser = await chromium.launch();
const ctx = await browser.newContext();
const page = await ctx.newPage();

const results = [];
for (const name of target) {
  const result = { name, ok: false, errors: [], note: "" };
  try {
    // 1. Bundle
    const tmp = mkdtempSync(join(tmpdir(), `rapidr-${name}-`));
    const zipPath = join(tmp, `${name}-web.zip`);
    execFileSync(join(ROOT, "rapidr"),
      ["bundle-bc", join(ROOT, `examples/${name}.rr`), "-o", zipPath],
      { cwd: ROOT, stdio: ["ignore", "pipe", "pipe"] });
    // 2. Unzip into a folder served by the same root server
    const serveDir = join(ROOT, "tests", ".matrix", name);
    mkdirSync(serveDir, { recursive: true });
    execFileSync("unzip", ["-o", "-q", zipPath, "-d", serveDir]);

    // 3. Drive the page
    const errors = [];
    page.removeAllListeners("pageerror");
    page.removeAllListeners("console");
    page.on("pageerror", e => errors.push(`pageerror: ${e.message}`));
    page.on("console", msg => {
      if (msg.type() === "error") {
        const t = msg.text();
        // Suppress expected "WebSocket failed to connect" noise for
        // demo_chat_client when no server is listening.
        if (name === "demo_chat_client" && /WebSocket|connect/i.test(t)) return;
        errors.push(`console.error: ${t}`);
      }
    });

    const url = `${URL_BASE}/tests/.matrix/${name}/index.html`;
    await page.goto(url, { waitUntil: "load", timeout: 15000 });
    // Wait until the bundle loader finishes (status text becomes empty
    // after rapidr_run_bc returns, or stays at "running…" if the program
    // entered an event loop). Either way is success.
    await page.waitForFunction(() => {
      const s = document.getElementById("rapidr-status")?.textContent || "";
      return s === "" || /running|preview ready/i.test(s) || /error/i.test(s);
    }, { timeout: 10000 });

    const status = await page.evaluate(() =>
      document.getElementById("rapidr-status")?.textContent || ""
    );
    if (/error/i.test(status)) errors.push(`status reads: ${status}`);

    await page.screenshot({ path: join(SHOTS, `${name}-web.png`), fullPage: true });

    result.errors = errors;
    result.ok = errors.length === 0;
    result.note = status || "(running)";
  } catch (e) {
    result.errors = [String(e?.message || e)];
  }
  results.push(result);
  const tag = result.ok ? "PASS" : "FAIL";
  console.log(`${tag.padEnd(4)}  ${name.padEnd(22)}  ${result.note}${result.errors.length ? "  | " + result.errors.join("; ") : ""}`);
}

await browser.close();

const passed = results.filter(r => r.ok).length;
console.log(`\n${passed}/${results.length} passed`);
writeFileSync(join(SHOTS, "matrix.json"), JSON.stringify(results, null, 2));
process.exit(passed === results.length ? 0 : 1);
