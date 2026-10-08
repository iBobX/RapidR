// node tests/_webprobe.mjs file.bas comp.prop,…  — run a program on the web
// runtime's page (tests/web_run.mjs), print its output and the properties
// (as the program reads them) after 2 s
import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import { openRunner } from "./web_run.mjs";
const [file, props = ""] = process.argv.slice(2);
const browser = await chromium.launch();
const r = await openRunner(browser);
await r.run(readFileSync(file, "latin1"));
await r.page.waitForTimeout(2000);
for (const l of r.lines) console.log("OUT", l.slice(0, 200));
for (const l of r.errs) console.log("ERR", l.slice(0, 200));
for (const p of props.split(",").filter(Boolean)) {
  const [c, k] = p.split(".");
  console.log(`${p}=${await r.prop(c, k)} (shown ${await r.prop(c, "__shown")})`);
}
await browser.close();
