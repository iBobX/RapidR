// EVENT blocks in TYPE … EXTENDS and `Sender` in event handlers, end to end:
// builds tests/fixtures/oop_events.bas as a web bundle with the CLI, opens it
// in Chromium and clicks through it.
//
// Usage (repo root, after ./build.sh and tools/build_web_artifacts.sh, with
// the repo served on http://localhost:8765):  node tests/web_bundle_oop.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const OUT = join(ROOT, "tests/conformance/.work/oop_events");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
execFileSync(join(ROOT, "rapidr"), ["bundle-bc", join(ROOT, "tests/fixtures/oop_events.bas"), "-o", join(OUT, "app.zip"),
  "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js")]);
execFileSync("unzip", ["-q", "-o", join(OUT, "app.zip"), "-d", join(OUT, "site")]);

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
await page.goto(`${URL_BASE}/tests/conformance/.work/oop_events/site/index.html`);
await page.waitForFunction(() => document.querySelectorAll("button.rr-widget").length >= 3, { timeout: 15000 });

const captions = () => page.evaluate(() => [...document.querySelectorAll("button.rr-widget")].map((b) => b.textContent));
ok(JSON.stringify(await captions()) === '["Click me","Click me","Sender test"]', "CONSTRUCTOR set both instances' captions");
await page.getByText("Click me").first().click();
await page.getByText("Clicked 1").click();
await page.getByText("Click me").click();
await page.getByText("Sender test").click();
await page.waitForTimeout(300);
const after = await captions();
ok(after[0] === "Clicked 2", `first instance counted its own clicks (${after[0]})`);
ok(after[1] === "Clicked 1", `second instance has separate state (${after[1]})`);
ok(after[2] === "Sender works", `Sender refers to the clicked button (${after[2]})`);
ok(errors.length === 0, `no errors (${errors.length}${errors.length ? ": " + errors[0] : ""})`);

await browser.close();
if (failed) { console.log(`\nOOP events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nOOP events: ALL CHECKS PASSED");
