// The main program's end is the program's in the browser too, in both web
// builds of tests/fixtures/main_end_show.bas — the interpreter (`rapidr
// bundle-bc`) and the compiled program (`rapidr build --web`): its main code
// ends after `Form.Show`, so (as RapidQ's RC.EXE runs it, and as the
// desktop does) OnShow runs, then "after show", then the program ends — the
// form goes and its timer never ticks. tests/fixtures/end_timer.bas, whose
// main program waits in ShowModal, keeps running (tests/web_end_timer.mjs).
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on http://localhost:8765, or RAPIDR_URL):
//   node tests/web_main_end.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const WORK = join(ROOT, "tests/conformance/.work/web_main_end");
const SRC = join(ROOT, "tests/fixtures/main_end_show.bas");
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
copyFileSync(SRC, join(WORK, "main_end_show.bas"));
rapidr("build", join(WORK, "main_end_show.bas"), "--web", "--debug");

const builds = {
  interpreted: `${URL_BASE}/tests/conformance/.work/web_main_end/site/index.html`,
  compiled: `${URL_BASE}/tests/conformance/.work/web_main_end/main_end_show_web/index.html`,
};
const browser = await chromium.launch();
for (const [kind, url] of Object.entries(builds)) {
  const page = await browser.newPage();
  const logs = [], errors = [];
  page.on("console", (m) => (m.type() === "error" ? errors : logs).push(m.text()));
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(url);
  for (let i = 0; i < 100 && !logs.includes("[RapidR] Program ended."); i++) await page.waitForTimeout(100);
  // (time for ticks, were the timer running: its interval is 50 ms)
  await page.waitForTimeout(600);
  ok(logs.includes("[RapidR] Program ended."), `${kind}: the program ended with its main code (${logs.join(" / ")})`);
  ok(logs.includes("onshow") && logs.indexOf("onshow") < logs.indexOf("after show"), `${kind}: OnShow ran inside Show, before "after show"`);
  ok(!logs.includes("tick"), `${kind}: the timer never ticked`);
  ok(!(await k.shown(page, "Form")), `${kind}: the form went with the program`);
  ok(errors.length === 0, `${kind}: no page errors (${errors.join(" / ")})`);
  await page.close();
}
await browser.close();
if (failed) { console.log(`\nWeb main end: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb main end: ALL CHECKS PASSED");
