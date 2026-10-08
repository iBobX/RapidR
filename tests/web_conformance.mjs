// The conformance suite (tests/conformance/cases) in the browser: each
// console case is compiled by the web runtime's wasm compiler and run by the
// wasm VM on the runtime's own page (tests/web_run.mjs), and its output must be the same `.expected` text the desktop
// interpreter and the native build are held to (ROADMAP: the web stays in
// step with desktop, native and interpreter).
//
// A case that can't be compared in a browser (needs stdin or files on disk,
// prints on paper, …) carries a first-line marker in its .bas:
//   ' xfail: web — reason
// A marked case that passes fails the run (so the marker gets removed).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_conformance.mjs [filter …]

import { chromium } from "playwright";
import { readdirSync, readFileSync, existsSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { openRunner } from "./web_run.mjs";

const CASES = join(dirname(fileURLToPath(import.meta.url)), "conformance/cases");
const filters = process.argv.slice(2);
const norm = (s) => s.replace(/\r\n/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").trimEnd();

const names = readdirSync(CASES).filter((f) => f.endsWith(".bas")).map((f) => f.slice(0, -4)).sort()
  // (not the ones with stdin or a command line: tests/web_bundle_console.mjs
  // runs a page with a query string, the web's command line)
  .filter((n) => existsSync(join(CASES, n + ".expected")) && !existsSync(join(CASES, n + ".input")) && !existsSync(join(CASES, n + ".args")))
  .filter((n) => !filters.length || filters.some((f) => n.includes(f)));

const browser = await chromium.launch();
// (`RAPIDR_DPR=2`: a high-DPI screen — what programs read must not change)
const r = await openRunner(browser, { deviceScaleFactor: Number(process.env.RAPIDR_DPR || 1) });

// The cases' `$RESOURCE` files are the program's assets, as a project has
// them (found by their last path part).
const assets = ["resource_files", "picture_files", "video_files"].map((d) => join(CASES, d)).filter((d) => existsSync(d)).flatMap((RES) => readdirSync(RES).filter((f) => statSync(join(RES, f)).isFile()).map((f) => ({
  name: f, mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(join(RES, f)).toString("base64"),
})));

let passed = 0, failed = 0, xfail = 0, xpass = 0;
for (const name of names) {
  const source = readFileSync(join(CASES, name + ".bas"), "utf8");
  const marker = /^'\s*xfail:\s*([^—\-\n]*)/i.exec(source.split("\n")[0]);
  const expectedFail = !!marker && /\bweb\b/i.test(marker[1]);
  const expected = norm(readFileSync(join(CASES, name + ".expected"), "utf8"));
  // (A case that stops with a run-time error — `.expected-runtime-error`,
  // tests/conformance/run.mjs — ends at the error's line, which has its
  // message.)
  const runtimePath = join(CASES, name + ".expected-runtime-error");
  const runtimeError = existsSync(runtimePath) ? readFileSync(runtimePath, "utf8").split("\n").map((l) => l.trim()).filter(Boolean) : null;
  const hasError = () => r.errs.some((l) => runtimeError.every((needle) => l.includes(needle)));
  // (the closing marker on a line of its own, even after a program whose
  // last PRINT stays on its line; a program that ENDs itself never prints it)
  const tag = `@@${name}@@`;
  await r.run(`${source}\nPRINT\nPRINT "${tag}E"\n`, { assets, name: "program" });
  const ended = () => r.lines.includes(`${tag}E`) || r.ended();
  for (let waited = 0; waited < 20000; waited += 100) {
    if (runtimeError ? hasError() : ended()) break;
    if (r.errs.some((e) => e.startsWith("compile:"))) break;
    await r.page.waitForTimeout(100);
  }
  // (what the program printed before its error may come just after it)
  if (runtimeError) await r.page.waitForTimeout(300);
  let got;
  if (runtimeError) {
    got = hasError() ? norm(r.output()) : `<no run-time error: ${JSON.stringify(r.errors().slice(-200))}>`;
  } else {
    const text = r.output();
    const m = new RegExp(`^([\\s\\S]*?)\\n?^(?:${tag}E|\\[RapidR\\] Program ended\\.)$`, "m").exec(text);
    got = m ? norm(m[1]) : `<no output: ${JSON.stringify((text + " " + r.errors()).slice(-200))}>`;
  }
  const same = got === expected;
  if (same && expectedFail) { xpass++; console.log(`XPASS  ${name} (remove its web xfail marker)`); }
  else if (same) { passed++; console.log(`PASS   ${name}`); }
  else if (expectedFail) { xfail++; console.log(`XFAIL  ${name}`); }
  else {
    failed++;
    console.log(`FAIL   ${name}`);
    const a = expected.split("\n"), b = got.split("\n");
    const at = a.findIndex((l, i) => l !== b[i]);
    console.log(`       first difference at line ${at < 0 ? Math.min(a.length, b.length) + 1 : at + 1}:`);
    console.log(`       expected: ${JSON.stringify(a[at] ?? "<end>")}`);
    console.log(`       got:      ${JSON.stringify(b[at] ?? "<end>")}`);
  }
}
await browser.close();
console.log(`\n${passed} passed, ${xfail} known failures (xfail), ${failed} failed, ${xpass} unexpectedly passed, ${r.pageErrors.length} page errors`);
if (failed || xpass) process.exit(1);
