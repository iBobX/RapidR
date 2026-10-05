// PRINT output on the page of an exported web bundle: the bundle's loader
// installs web-ide/bundle_console.js, which renders what the program prints
// (CLS / COLOR / LOCATE included) with web-ide/ansi_screen.js.
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on http://localhost:8765):  node tests/web_bundle_console.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const WORK = join(ROOT, "tests/conformance/.work/console_bundle");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

function bundle(name) {
  const out = join(WORK, name);
  mkdirSync(out, { recursive: true });
  execFileSync(join(ROOT, "rapidr"), ["bundle-bc", join(ROOT, `tests/fixtures/${name}.bas`), "-o", join(out, "app.zip"),
    "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js")]);
  execFileSync("unzip", ["-q", "-o", join(out, "app.zip"), "-d", join(out, "site")]);
  return `${URL_BASE}/tests/conformance/.work/console_bundle/${name}/site/index.html`;
}

rmSync(WORK, { recursive: true, force: true });
const browser = await chromium.launch();
const errors = [];
const open = async (url) => {
  const page = await browser.newPage();
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  await page.goto(url);
  await page.waitForSelector("#rapidr-console", { timeout: 15000 });
  await page.waitForTimeout(300);
  return page;
};

// 1. A console program: the panel fills the page.
const page = await open(bundle("console_bundle"));
const got = await page.evaluate(() => {
  const el = document.getElementById("rapidr-console");
  const rect = el.getBoundingClientRect();
  const colored = [...el.querySelectorAll("span span")].find((s) => s.textContent.includes("ow on blue"));
  return { text: el.textContent, docked: el.classList.contains("docked"), height: rect.height, winHeight: innerHeight,
    color: colored && colored.style.color, bg: colored && colored.style.backgroundColor };
});
const lines = got.text.split("\n");
ok(lines[0] === "Hello RAPIDRapidR", `LOCATE 1, 7 overwrote row 1 from column 7, as on a terminal (${JSON.stringify(lines[0])})`);
ok(lines[1] === "yellow on blue", `second line (${JSON.stringify(lines[1])})`);
ok(/^sum = ?5$/.test(lines[2]), `PRINT with ; (${JSON.stringify(lines[2])})`);
ok(lines[3] === "partial line", `a partial line joined with the next PRINT (${JSON.stringify(lines[3])})`);
ok(got.color === "rgb(255, 255, 85)" && got.bg === "rgb(0, 0, 170)", `COLOR 14, 1 → yellow on blue (${got.color}, ${got.bg})`);
ok(!got.docked && got.height >= got.winHeight - 1, `console program: the panel fills the page (${got.height}/${got.winHeight})`);
ok(!got.text.includes("\x1b"), "no raw escape sequences");
await page.screenshot({ path: join(WORK, "console.png") });

// 2. A program with a form: the panel docks at the bottom, the form's
// window stays in reach (what is under its title bar's middle is the window).
const formPage = await open(bundle("console_bundle_form"));
await k.waitFor(formPage, "Form");
const form = await formPage.evaluate(() => {
  const el = document.getElementById("rapidr-console");
  const win = document.querySelector('.rr-kwin[data-rr-form="form"]');
  const r = win?.getBoundingClientRect();
  const top = r && document.elementFromPoint(r.left + r.width / 2, r.top + 10);
  return { text: el.textContent, docked: el.classList.contains("docked"), reachable: !!top && win.contains(top) };
});
const forms = (await k.windows(formPage)).map((w) => w.form);
ok(forms.length === 1 && form.docked, `with a form the console docks at the bottom (forms=${forms}, docked=${form.docked})`);
ok(form.reachable, "the console doesn't cover the form's window");
ok(form.text.startsWith("log line"), `form program's PRINT shown (${JSON.stringify(form.text)})`);
// 3. INPUT in a bundle: an in-page question (a kernel dialog with a field),
// the answer typed with real keys and echoed in the console.
const inPage = await browser.newPage();
inPage.on("pageerror", (e) => errors.push(e.message));
inPage.on("dialog", async (d) => { errors.push(`browser ${d.type()} dialog used`); await d.dismiss(); });
await inPage.goto(bundle("console_input"));
const answer = async (prompt, typed) => {
  await inPage.waitForFunction(() => ["INPUT", "TEXTAREA"].includes(document.activeElement?.tagName), null, { timeout: 15000 });
  const d = await k.dialog(inPage);
  ok(d && d.input && d.text.trim() === prompt.trim(), `INPUT asks with a kernel dialog: ${JSON.stringify(prompt)} (${JSON.stringify(d)})`);
  await inPage.locator(":focus").pressSequentially(typed);
  await inPage.locator(":focus").press("Enter");
  await inPage.waitForTimeout(300);
};
await answer("What is your name? ", "Ada");
await answer("How old are you? ", "36");
const typed = await inPage.evaluate(() => document.getElementById("rapidr-console").textContent);
ok(typed.startsWith("What is your name? Ada\nHow old are you? 36\n"), `prompts and typed lines in the console (${JSON.stringify(typed)})`);
ok(/Hello Ada, next year you'll be ?37/.test(typed), "the program continued with the answers (a number for age)");

ok(errors.length === 0, `no errors (${errors.length}${errors.length ? ": " + errors[0] : ""})`);

await browser.close();
if (failed) { console.log(`\nBundle console: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nBundle console: ALL CHECKS PASSED");
