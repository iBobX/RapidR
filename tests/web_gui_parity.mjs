// The GUI fixtures of tests/native_gui_events.mjs (tests/gui_parity_cases.mjs)
// in the browser: each fixture runs in the web IDE's preview, the same events
// are fired (a click on the component), and the same properties must read
// what the desktop, native and interpreted builds show. (Running a fixture
// and firing its events: tests/web_gui_run.mjs, which tests/web_a11y.mjs
// shares.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765, or RAPIDR_URL):  node tests/web_gui_parity.mjs [filter …]

import { join } from "node:path";
import { cases } from "./gui_parity_cases.mjs";
import { openIde, runCase } from "./web_gui_run.mjs";
import { startHttpServer } from "./http_test_server.mjs";

// (QDOWNLOAD's server: the tests' own, local — never the internet)
process.env.RAPIDR_TEST_HTTP = (await startHttpServer()).address;

const filters = process.argv.slice(2);
let failed = 0, passed = 0, skipped = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };

// (`RAPIDR_DPR=2`: a high-DPI screen — what programs read must not change)
const { browser, page, pageErrors } = await openIde(Number(process.env.RAPIDR_DPR || 1));

for (const c of cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  if (c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.why || "no browser counterpart"})`); continue; }
  const { frame, missing } = await runCase(page, c);
  if (!frame) { ok(false, `${c.name}: preview frame`); continue; }
  for (const target of missing) ok(false, `${c.name}: ${target} exists`);
  // (The properties as the program reads them, as the desktop test prints them.)
  const dumped = await frame.evaluate((names) => Object.fromEntries(names.map((n) => {
    const [comp, ...rest] = n.split(".");
    return [n, window.__rapidr_rt.rapidr_get_prop(comp.toUpperCase(), rest.join(".").toLowerCase())];
  })), c.dump.split(","));
  const text = Object.entries(dumped).map(([k, v]) => `${k}=${v}`).join("\n");
  for (const line of c.expect) ok(text.includes(line), `${c.name} (web): ${line}${text.includes(line) ? "" : `   [got: ${text.replace(/\n/g, " ; ")}]`}`);
  // (RAPIDR_SHOT=dir: a screenshot of each case's page, to look at)
  if (process.env.RAPIDR_SHOT) await page.screenshot({ path: join(process.env.RAPIDR_SHOT, c.name + ".png") });
  // (`webCheck`: what the page shows, where the program can't read it)
  if (c.webCheck) {
    const got = await frame.evaluate(c.webCheck);
    ok(got === c.webExpect, `${c.name} (web page): ${c.webExpect}${got === c.webExpect ? "" : `   [got: ${got}]`}`);
  }
}
ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
console.log(`\nWeb GUI parity: ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
if (failed) process.exit(1);
