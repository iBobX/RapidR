#!/usr/bin/env node
// Two run folders' cells side by side (a baseline and a run after a fix):
//   node tools/ai_eval/compare.mjs RUNS_A/CELL RUNS_B/CELL
// e.g. runs/A-ollama_qwen3.5_4b-q4_K_M runs-fixed/A-ollama_qwen3.5_4b-q4_K_M
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const [a, b] = process.argv.slice(2);
if (!a || !b) { console.error("usage: compare.mjs CELL_DIR_A CELL_DIR_B"); process.exit(2); }
const load = (d) => Object.fromEntries(readdirSync(d).filter((n) => existsSync(join(d, n, "result.json"))).map((n) => [n, JSON.parse(readFileSync(join(d, n, "result.json"), "utf8"))]));
const A = load(a), B = load(b);
const apps = [...new Set([...Object.keys(A), ...Object.keys(B)])].sort();
let pa = 0, pb = 0, n = 0;
console.log("| app | before | after |\n|---|---|---|");
for (const app of apps) {
  const x = A[app], y = B[app];
  const f = (r) => (r ? `${r.score === 1 ? "PASS" : "fail"} (${r.stopped}, ${r.requests ?? "?"} req, ${r.toolErrors ?? "?"} tool errors, ${r.seconds}s)` : "–");
  console.log(`| ${app} | ${f(x)} | ${f(y)} |`);
  if (x && y) { n++; pa += x.score === 1 ? 1 : 0; pb += y.score === 1 ? 1 : 0; }
}
console.log(`\nboth ran: ${n} apps; passed before ${pa}, after ${pb}`);
