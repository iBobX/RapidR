// RapidR's `END n` in the browser, in both web builds of
// tests/fixtures/end_code.bas — the interpreter (`rapidr bundle-bc`) and the
// compiled program (`rapidr build --web`): a page has no exit code, so the
// program ends as a plain END does ("[RapidR] Program ended.", what follows
// the END never runs) and the number is ignored, with no error on the page.
// (On the desktop the program ends with the code: the conformance case
// end_exit_code_literal, and the three build kinds.)
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on http://localhost:8765, or RAPIDR_URL):
//   node tests/web_end_code.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const WORK = join(ROOT, "tests/conformance/.work/web_end_code");
const SRC = join(ROOT, "tests/fixtures/end_code.bas");
// (the native web build's generated Rust and cargo's files in the work
// folder's build cache, not the user's)
const rapidr = (...args) => execFileSync(join(ROOT, "rapidr"), args, { cwd: ROOT, stdio: "ignore", env: { ...process.env, RAPIDR_BUILD_CACHE: process.env.RAPIDR_BUILD_CACHE || join(ROOT, "tests/conformance/.work/build-cache") } });

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
rapidr("bundle-bc", SRC, "-o", join(WORK, "app.zip"), "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js"));
execFileSync("unzip", ["-q", "-o", join(WORK, "app.zip"), "-d", join(WORK, "site")]);
// (a web build is written next to its source: build a copy here)
copyFileSync(SRC, join(WORK, "end_code.bas"));
rapidr("build", join(WORK, "end_code.bas"), "--web", "--debug");

const builds = {
  interpreted: `${URL_BASE}/tests/conformance/.work/web_end_code/site/index.html`,
  compiled: `${URL_BASE}/tests/conformance/.work/web_end_code/end_code_web/index.html`,
};
const browser = await chromium.launch();
for (const [kind, url] of Object.entries(builds)) {
  const page = await browser.newPage();
  const logs = [], errors = [];
  page.on("console", (m) => (m.type() === "error" ? errors : logs).push(m.text()));
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(url);
  for (let i = 0; i < 100 && !logs.includes("[RapidR] Program ended."); i++) await page.waitForTimeout(100);
  await page.waitForTimeout(300);
  // (the interpreter prints to the page, the compiled program to the console)
  const text = (await page.evaluate(() => document.body.innerText)) + "\n" + logs.join("\n");
  ok(logs.includes("[RapidR] Program ended."), `${kind}: END n ended the program (${logs.join(" / ")})`);
  ok(text.includes("before"), `${kind}: what ran before the END printed (${JSON.stringify(text.slice(0, 80))})`);
  ok(!text.includes("after"), `${kind}: nothing after the END ran`);
  ok(errors.length === 0, `${kind}: no page errors (${errors.join(" / ")})`);
  await page.close();
}
await browser.close();
console.log(failed ? `\n${failed} check(s) failed` : "\nWeb END n: all checks passed");
process.exit(failed ? 1 : 0);
