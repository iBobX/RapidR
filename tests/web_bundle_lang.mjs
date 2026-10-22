// A web bundle carries the files below its program's folder (lane T1b):
// its translations beside the page (lang/es.po and their list,
// lang/index.txt) and the files its source names in subfolders
// (art/logo.svg, among the page's files). In a real browser:
//
//  1. `?es`: Spanish, the logo drawn and found (FILEEXISTS).
//  2. `?es-AR`: Spanish too (es-AR → es), asking only for lang/index.txt and
//     lang/es.po — no failed request.
//  3. `?pt` and `?en-US` (no catalog): English, and not one failed request
//     (before, up to five 404s: en-US.po, en_US.po … en.po).
//  4. A program that translates but has no lang folder: an empty list, no
//     failed request.
//  5. Nothing outside the program's folder goes into the bundle.
//
// Usage (repo root, after building ./rapidr and tools/build_web_artifacts.sh,
// with the repo served on RAPIDR_URL, default http://localhost:8765):
//   node tests/web_bundle_lang.mjs
// (RAPIDR_BIN: another rapidr to build the bundles with.)

import { chromium } from "playwright";
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/lang_bundle");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

/// The program in `src` bundled and unzipped in WORK/<dir>/site; what the
/// build printed.
function bundle(src, dir) {
  const out = join(WORK, dir);
  mkdirSync(out, { recursive: true });
  const r = spawnSync(RAPIDR, ["bundle-bc", src, "-o", join(out, "app.zip"),
    "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js")], { encoding: "utf8" });
  const printed = `${r.stdout}${r.stderr}`;
  if (r.status !== 0) throw new Error(`bundle-bc ${src}: ${printed}`);
  execFileSync("unzip", ["-q", "-o", join(out, "app.zip"), "-d", join(out, "site")]);
  return { site: join(out, "site"), url: `${URL_BASE}/tests/conformance/.work/lang_bundle/${dir}/site/index.html`, printed };
}

rmSync(WORK, { recursive: true, force: true });
// (the fixture copied, with a file beside its folder that it names: never taken)
const src = join(WORK, "src", "app");
cpSync(join(ROOT, "tests/fixtures/carried_files"), src, { recursive: true });
writeFileSync(join(WORK, "src", "secret.txt"), "SECRET-OUTSIDE-THE-FOLDER\n");
writeFileSync(join(src, "carried.rr"), readFileSync(join(src, "carried.rr"), "utf8") + "x$ = \"../secret.txt\"\n");

const b = bundle(join(src, "carried.rr"), "carried");
ok(existsSync(join(b.site, "lang/es.po")), "the bundle has lang/es.po");
const index = readFileSync(join(b.site, "lang/index.txt"), "utf8");
ok(index.split("\n").filter((l) => l && !l.startsWith("#")).join(",") === "es.po", `lang/index.txt lists it (${JSON.stringify(index)})`);
const assetsJs = readFileSync(join(b.site, "rapidr-assets.js"), "utf8");
ok(assetsJs.includes('"art/logo.svg": "data:image/svg+xml;base64,'), "art/logo.svg is among the page's files");
// (the zip's text files are deflated: the unzipped ones are looked through)
const holding = readdirSync(b.site, { recursive: true }).filter((f) => statSync(join(b.site, f)).isFile() && readFileSync(join(b.site, f)).includes("SECRET-OUTSIDE"));
ok(holding.length === 0 && !assetsJs.includes(Buffer.from("SECRET-OUTSIDE").toString("base64").slice(0, 12)), `nothing outside the program's folder is in the bundle (${holding})`);
ok(/warning: "\.\.\/secret\.txt" is outside the program's folder: not copied/.test(b.printed), `the build said so (${b.printed.split("\n").find((l) => l.includes("secret"))})`);

const browser = await chromium.launch();

/// The page with `query`: what it shows and every request it made.
async function open(url, query) {
  const page = await browser.newPage();
  const failedRequests = [], asked = [], errors = [];
  page.on("response", (r) => { if (r.status() >= 400) failedRequests.push(`${r.status()} ${r.url()}`); });
  page.on("requestfailed", (r) => failedRequests.push(`failed ${r.url()}`));
  page.on("request", (r) => { const m = r.url().match(/\/lang\/[^?#]*/); if (m) asked.push(m[0].slice(1)); });
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${url}?${query}`);
  await k.waitFor(page, "Found", 20000);
  await page.waitForTimeout(400);
  const shown = { greeting: await k.text(page, "Greeting"), found: await k.text(page, "Found"), info: await k.text(page, "Info") };
  return { page, shown, failedRequests, asked, errors };
}

for (const [query, greeting, found, asks] of [
  ["es", "¡Hola, mundo!", "El logo está aquí", ["lang/index.txt", "lang/es.po"]],
  ["es-AR", "¡Hola, mundo!", "El logo está aquí", ["lang/index.txt", "lang/es.po"]],
  ["pt", "Hello, world!", "The logo is here", ["lang/index.txt"]],
  ["en-US", "Hello, world!", "The logo is here", ["lang/index.txt"]],
]) {
  const r = await open(b.url, query);
  ok(r.shown.greeting === greeting && r.shown.found === found, `?${query}: ${JSON.stringify(r.shown)}`);
  ok(r.failedRequests.length === 0 && r.errors.length === 0, `?${query}: no failed request, no console error (${JSON.stringify([...r.failedRequests, ...r.errors])})`);
  ok(JSON.stringify(r.asked) === JSON.stringify(asks), `?${query}: asked only for ${asks.join(", ")} (${JSON.stringify(r.asked)})`);
  if (query === "es") {
    // (the logo drawn: its blue square, its yellow sun)
    const blue = await k.pixelAt(r.page, "Logo", 10, 48), sun = await k.pixelAt(r.page, "Logo", 48, 40);
    ok(blue && blue[2] > 200 && blue[0] < 80 && sun && sun[0] > 230 && sun[1] > 180 && sun[2] < 120, `the logo is drawn (blue ${blue}, sun ${sun})`);
    await r.page.screenshot({ path: join(WORK, "carried-es.png") });
  }
  await r.page.close();
}

// 4. Translates, no lang folder: the list is there and empty.
{
  const bare = join(WORK, "src", "bare");
  mkdirSync(bare, { recursive: true });
  cpSync(join(ROOT, "tests/fixtures/carried_files/carried.rr"), join(bare, "carried.rr"));
  const nb = bundle(join(bare, "carried.rr"), "bare");
  ok(existsSync(join(nb.site, "lang/index.txt")) && !existsSync(join(nb.site, "lang/es.po")), "no lang folder: an empty lang/index.txt");
  const r = await open(nb.url, "es");
  ok(r.shown.greeting === "Hello, world!" && r.shown.found === "No logo", `its own English, no logo (${JSON.stringify(r.shown)})`);
  // (its logo isn't there either: that request fails, as the program asks)
  const langFailed = r.failedRequests.filter((f) => f.includes("/lang/"));
  ok(langFailed.length === 0 && JSON.stringify(r.asked) === '["lang/index.txt"]', `no failed request for a translation (${JSON.stringify(langFailed)}; asked ${JSON.stringify(r.asked)})`);
  await r.page.close();
}

await browser.close();
console.log(failed ? `\nWeb bundle languages: ${failed} FAILED` : "\nWeb bundle languages: ALL CHECKS PASSED");
process.exit(failed ? 1 : 0);
