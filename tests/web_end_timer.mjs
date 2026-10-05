// END in the browser, in both web builds of tests/fixtures/end_timer.bas —
// the interpreter (`rapidr bundle-bc`) and the compiled program (`rapidr
// build --web`): its timer ticks (QTIMER is enabled by default), and END in
// a button's handler stops the program as on the desktop — the statement
// after END doesn't run, the form closes, the timer stops, and the page
// reports no error.
// (On the kernel host the form is a window the kernel draws: the test reads
// its accessibility mirror and clicks the button as the user does,
// tests/web_kernel_page.mjs.)
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on http://localhost:8765):  node tests/web_end_timer.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const WORK = join(ROOT, "tests/conformance/.work/web_end_timer");
const SRC = join(ROOT, "tests/fixtures/end_timer.bas");
const rapidr = (...args) => execFileSync(join(ROOT, "rapidr"), args, { cwd: ROOT, stdio: "ignore" });

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
rapidr("bundle-bc", SRC, "-o", join(WORK, "app.zip"), "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js"));
execFileSync("unzip", ["-q", "-o", join(WORK, "app.zip"), "-d", join(WORK, "site")]);
// (a web build is written next to its source: build a copy here)
copyFileSync(SRC, join(WORK, "end_timer.bas"));
rapidr("build", join(WORK, "end_timer.bas"), join(WORK, "compiled"), "--web");

const builds = {
  interpreted: `${URL_BASE}/tests/conformance/.work/web_end_timer/site/index.html`,
  compiled: `${URL_BASE}/tests/conformance/.work/web_end_timer/end_timer_web/index.html`,
};
const browser = await chromium.launch();
for (const [kind, url] of Object.entries(builds)) {
  const page = await browser.newPage();
  const logs = [], errors = [];
  page.on("console", (m) => (m.type() === "error" ? errors : logs).push(m.text()));
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(url);
  await k.waitFor(page, "Btn", 20000);
  await page.waitForTimeout(700);
  const ticks = async () => Number(((await k.text(page, "lbl")) || "").replace(/\D/g, "")) || 0;
  ok((await ticks()) > 0, `${kind}: the timer ticks without Enabled set`);
  ok(await k.shown(page, "Form"), `${kind}: the form is shown`);
  await k.click(page, "Btn");
  await page.waitForTimeout(200);
  const atEnd = await ticks();
  await page.waitForTimeout(600);
  ok((await ticks()) === atEnd, `${kind}: the timer stopped at END`);
  ok(logs.includes("before end") && !logs.includes("after end"), `${kind}: the statement after END didn't run (${logs.join(" / ")})`);
  ok(!(await k.shown(page, "Form")), `${kind}: the form closed`);
  ok(errors.length === 0, `${kind}: no page errors (${errors.join(" / ")})`);
  await page.close();
}
await browser.close();
if (failed) { console.log(`\nWeb END: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb END: ALL CHECKS PASSED");
