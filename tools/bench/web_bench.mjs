// web_bench.mjs: the VM's benchmark programs (tools/bench/vm/) in a browser —
// compiled by the web runtime's wasm compiler and run by its wasm VM on the
// runtime's own page (tests/web_run.mjs), as a web build runs them. Each
// program prints its own time ("ms …", TIMER: the work, not the page's load);
// the fastest of --runs runs is shown.
//
//   node tools/bench/web_bench.mjs [--runs N] [program …]
//
// Needs the web artifacts (tools/build_web_artifacts.sh) and the repository
// served on RAPIDR_URL (default http://localhost:8765:
// python3 -m http.server 8765 --bind 127.0.0.1). Playwright comes from
// tests/node_modules.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { openRunner, URL_BASE } from "../../tests/web_run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(HERE, "../../tests/"))("playwright");
const DEFAULT = ["mid_scan", "concat_build", "byte_array", "stream_read", "binary_read"];

const args = process.argv.slice(2);
let runs = 3;
const names = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--runs") runs = Number(args[++i]);
  else names.push(args[i].replace(/\.rr$/, ""));
}

const browser = await chromium.launch();
const r = await openRunner(browser);
console.log(`web_bench: ${URL_BASE}, the fastest of ${runs} runs`);
let failed = 0;
for (const name of names.length ? names : DEFAULT) {
  const source = readFileSync(join(HERE, "vm", `${name}.rr`), "utf8");
  let best = null;
  let shown = "";
  for (let i = 0; i < runs; i++) {
    if (!(await r.run(source, { name: `${name}.rr` })) || !(await r.waitOutput(/^ms \d+/m, 300000))) {
      console.log(`  ${name}: FAILED ${r.errors().slice(0, 300)}`);
      failed++;
      break;
    }
    const line = r.output().split("\n").find((l) => /^ms \d+/.test(l));
    const ms = Number(line.match(/^ms (\d+)/)[1]);
    if (best === null || ms < best) {
      best = ms;
      shown = r.output().split("\n").filter((l) => l && !l.startsWith("[RapidR]")).join(" | ");
    }
    await r.stop();
  }
  if (best !== null) console.log(`  ${name.padEnd(14)} ${String(best).padStart(7)} ms   (${shown})`);
}
await browser.close();
process.exit(failed ? 1 : 0);
