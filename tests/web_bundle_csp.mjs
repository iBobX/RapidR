// The web bundle's defences, in a real browser (docs/security-audit.md
// SEC-15 / SEC-16 / SEC-12): every page `rapidr bundle-bc` builds carries a
// Content-Security-Policy derived from what the program uses, and works
// under it.
//
//  1. RDOM showing markup it didn't write: the page works, the markup's
//     script (an <img onerror>) is stopped by the policy.
//  2. An RWEBVIEW's Html runs its own scripts in its sandboxed frame, at an
//     opaque origin: it can't reach the program's page.
//  3. RJAVASCRIPT: the policy allows Eval for that program (and no other).
//  4. A project whose file name is markup: the page's title is that text,
//     nothing of it runs, the program runs.
//  5. A console program works under the policy, with no violation.
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on http://localhost:8765):  node tests/web_bundle_csp.mjs
// (RAPIDR_BIN: another rapidr to build the bundles with.)

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/csp_bundle");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

function bundle(source, dir) {
  const out = join(WORK, dir);
  mkdirSync(out, { recursive: true });
  execFileSync(RAPIDR, ["bundle-bc", source, "-o", join(out, "app.zip"),
    "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js")], { stdio: "pipe" });
  execFileSync("unzip", ["-q", "-o", join(out, "app.zip"), "-d", join(out, "site")]);
  return { url: `${URL_BASE}/tests/conformance/.work/csp_bundle/${dir}/site/index.html`, html: readFileSync(join(out, "site/index.html"), "utf8") };
}

const policyOf = (html) => {
  const m = html.match(/http-equiv="Content-Security-Policy" content="([^"]*)"/);
  return m ? m[1].replaceAll("&#39;", "'") : null;
};

rmSync(WORK, { recursive: true, force: true });
const browser = await chromium.launch();
async function open(url) {
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  // (what the policy stopped, as the page reports it)
  await page.addInitScript(() => {
    window.__violations = [];
    document.addEventListener("securitypolicyviolation", (e) => window.__violations.push(`${e.violatedDirective} ${e.blockedURI}`));
  });
  await page.goto(url);
  return { page, errors };
}

// 1. RDOM with markup from elsewhere.
{
  const b = bundle(join(ROOT, "tests/fixtures/csp_dom_injection.bas"), "dom");
  const csp = policyOf(b.html);
  ok(csp && /script-src 'self' 'wasm-unsafe-eval';/.test(csp) && !/'unsafe-eval'/.test(csp) && !/script-src[^;]*'unsafe-inline'/.test(csp), `the page has a strict policy (${csp})`);
  const { page, errors } = await open(b.url);
  await k.waitFor(page, "Lbl");
  await page.waitForFunction(() => document.getElementById("shown"), null, { timeout: 15000 }).catch(() => {});
  await page.waitForTimeout(500);
  const got = await page.evaluate(() => ({ shown: !!document.getElementById("shown"), pwned: window.__pwned ?? null, violations: window.__violations }));
  ok(got.shown && await k.text(page, "Lbl") === "shown", "the program runs and its markup shows");
  ok(got.pwned === null, `the markup's script didn't run (window.__pwned = ${got.pwned})`);
  ok(got.violations.some((v) => v.startsWith("script-src")), `the policy stopped it (${JSON.stringify(got.violations)})`);
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await page.close();
}

// 2. RWEBVIEW: its Html's scripts run in its frame, away from the page.
{
  const b = bundle(join(ROOT, "tests/fixtures/csp_webview.bas"), "webview");
  const csp = policyOf(b.html);
  ok(csp && /frame-src 'self' https:/.test(csp) && !/'unsafe-eval'/.test(csp), `RWEBVIEW opens frames, nothing more (${csp})`);
  const { page, errors } = await open(b.url);
  await k.waitFor(page, "Lbl");
  let inner = null;
  for (let i = 0; i < 50 && !inner?.text?.includes("ran"); i++) {
    for (const f of page.frames().filter((f) => f !== page.mainFrame())) {
      const got = await f.evaluate(() => ({ text: document.getElementById("p")?.textContent ?? null, reach: document.body?.dataset.reach, storage: document.body?.dataset.storage, origin: self.origin })).catch(() => null);
      if (got?.text) inner = got;
    }
    if (!inner?.text?.includes("ran")) await page.waitForTimeout(100);
  }
  const sandbox = await page.evaluate(() => document.getElementById("rr-web")?.getAttribute("sandbox"));
  ok(inner?.text === "script ran", `the Html's own script ran (${JSON.stringify(inner)})`);
  ok(inner?.origin === "null" && !/allow-same-origin/.test(sandbox ?? "x allow-same-origin"), `in a sandboxed frame at an opaque origin (${inner?.origin}; ${sandbox})`);
  ok(inner?.reach === "blocked" && (await page.title()) !== "pwned", `it can't reach the program's page (${inner?.reach}, title ${await page.title()})`);
  ok(inner?.storage === "no", `nor the program's storage (${inner?.storage})`);
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await page.close();
}

// 3. RJAVASCRIPT: Eval works, for this program.
{
  const b = bundle(join(ROOT, "tests/fixtures/csp_javascript.bas"), "js");
  ok(/'unsafe-eval'/.test(policyOf(b.html) ?? ""), "a program with RJAVASCRIPT may Eval");
  const { page } = await open(b.url);
  await k.waitFor(page, "Lbl");
  await page.waitForTimeout(300);
  ok(/eval\s*42/.test(await k.text(page, "Lbl") ?? ""), `Eval answers under the policy (${await k.text(page, "Lbl")})`);
  await page.close();
}

// 4. A file name that is markup.
{
  const dir = join(WORK, "src");
  mkdirSync(dir, { recursive: true });
  const name = `evil"'<img src=x onerror=window.__pwned=1><script>window.__pwned=2<!--`;
  const source = join(dir, `${name}.bas`);
  copyFileSync(join(ROOT, "tests/fixtures/console_bundle.bas"), source);
  const b = bundle(source, "hostile");
  const { page, errors } = await open(b.url);
  await page.waitForSelector("#rapidr-console", { timeout: 15000 }).catch(() => {});
  await page.waitForTimeout(500);
  const got = await page.evaluate(() => ({ title: document.title, pwned: window.__pwned ?? null, imgs: document.querySelectorAll("head img, body > img").length, printed: document.getElementById("rapidr-console")?.textContent ?? "" }));
  ok(got.title === name, `the title is the name, as text (${JSON.stringify(got.title)})`);
  ok(got.pwned === null && got.imgs === 0, `nothing of the name ran or became markup (${JSON.stringify(got)})`);
  ok(got.printed.includes("yellow on blue"), "the program ran");
  ok(errors.length === 0, `no page errors (${errors.join("; ")})`);
  await page.close();
}

// 5. A console program: no violation at all.
{
  const b = bundle(join(ROOT, "tests/fixtures/console_bundle.bas"), "console");
  const { page, errors } = await open(b.url);
  await page.waitForSelector("#rapidr-console", { timeout: 15000 });
  await page.waitForTimeout(500);
  const v = await page.evaluate(() => window.__violations);
  ok(v.length === 0 && errors.length === 0, `runs under its policy with no violation (${JSON.stringify(v)} ${errors.join("; ")})`);
  await page.close();
}

await browser.close();
console.log(failed ? `\nWeb bundle CSP: ${failed} FAILED` : "\nWeb bundle CSP: ALL CHECKS PASSED");
process.exit(failed ? 1 : 0);
