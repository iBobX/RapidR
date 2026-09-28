// The GUI fixtures of tests/native_gui_events.mjs (tests/gui_parity_cases.mjs)
// in the browser: each fixture runs in the web IDE's preview, the same events
// are fired (a click on the component), and the same properties must read
// what the desktop, native and interpreted builds show.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_gui_parity.mjs [filter …]

import { chromium } from "playwright";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const filters = process.argv.slice(2);
let failed = 0, passed = 0, skipped = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };

// The fixtures' files ($RESOURCE, pictures) are the project's assets.
const assetDirs = [join(HERE, "fixtures"), join(HERE, "conformance/cases/resource_files")];
const assets = [];
for (const dir of assetDirs) {
  for (const f of readdirSync(dir)) {
    if (!statSync(join(dir, f)).isFile() || f.endsWith(".bas")) continue;
    assets.push({ name: f, mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64") });
  }
}

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate((a) => { window.RapidR.state.project.assets = a; }, assets);

for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  if (c.resize || c.split || c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.resize || c.split ? "desktop window hooks" : c.why || "no browser counterpart"})`); continue; }
  const source = readFileSync(join(HERE, "fixtures", c.name + ".bas"), "utf8");
  await page.evaluate((src) => {
    window.RapidR.runCommand("run.stop");
    window.RapidR.state.project.forms[0].code = { handlers: {}, source: src };
    window.RapidR.runCommand("run.start");
  }, source);
  await page.waitForTimeout(2500);
  const frame = page.frames().find((f) => f.url().includes("preview.html"));
  if (!frame) { ok(false, `${c.name}: preview frame`); continue; }
  const idOf = (name) => "rr-" + name.toLowerCase();
  for (const ev of c.events.split(",").filter(Boolean)) {
    const [target] = ev.split(".");
    const fired = await frame.evaluate(({ id, selector }) => {
      const host = document.getElementById(id);
      const el = selector ? host?.querySelector(selector) : host;
      if (!el) return false;
      // (a list box answers a pick with `change`, anything else a click)
      el.dispatchEvent(host.tagName === "SELECT" ? new Event("change", { bubbles: true }) : new MouseEvent("click", { bubbles: true, cancelable: true }));
      return true;
    }, { id: idOf(target), selector: c.webClick?.[target.toLowerCase()] });
    if (!fired) ok(false, `${c.name}: ${target} exists`);
    await page.waitForTimeout(300);
  }
  await page.waitForTimeout(500);
  // (The properties as the program reads them, as the desktop test prints them.)
  const dumped = await frame.evaluate((names) => Object.fromEntries(names.map((n) => {
    const [comp, ...rest] = n.split(".");
    return [n, window.__rapidr_rt.rapidr_get_prop(comp.toUpperCase(), rest.join(".").toLowerCase())];
  })), c.dump.split(","));
  const text = Object.entries(dumped).map(([k, v]) => `${k}=${v}`).join("\n");
  for (const line of c.expect) ok(text.includes(line), `${c.name} (web): ${line}${text.includes(line) ? "" : `   [got: ${text.replace(/\n/g, " ; ")}]`}`);
}
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
console.log(`\nWeb GUI parity: ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
if (failed) process.exit(1);
