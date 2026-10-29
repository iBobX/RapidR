#!/usr/bin/env node
// Scores finished runs again with the click test as it is now (a test that was wrong is fixed:
// the runs it judged are judged again, from the files they left — no model is asked).
//
//   node tools/ai_eval/rescore.mjs RUNS_DIR --app logic-gates [--cell B-haiku]
//
// RUNS_DIR/CELL/APP[.rN]/result.json gets score, compiles, clickTest and "rescored": true.
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(dirname(HERE));
const argv = process.argv.slice(2);
const dir = resolve(argv[0] || "");
const opt = (k) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : null);
const only = opt("--app");
const cellOnly = opt("--cell");
if (!argv[0] || !only) { console.error("usage: rescore.mjs RUNS_DIR --app NAME [--cell CELL]"); process.exit(2); }
const script = ["apps", "held-out", "school"].map((d) => join(ROOT, "tests", "ai-eval", d, `${only}.txt`)).find(existsSync);
if (!script) { console.error(`no request file for ${only}`); process.exit(2); }
const evalBin = process.env.RAPIDR_AI_EVAL_BIN || join(process.env.CARGO_TARGET_DIR || join(ROOT, "target"), "release", "examples", "ai_eval");
const rapidr = process.env.RAPIDR || join(ROOT, "target", "release", "rapidr");

for (const cell of readdirSync(dir).filter((d) => /^[AB]-/.test(d) && (!cellOnly || d === cellOnly))) {
  for (const sub of readdirSync(join(dir, cell)).filter((s) => s === only || s.startsWith(`${only}.r`))) {
    const run = join(dir, cell, sub);
    const resultFile = join(run, "result.json");
    if (!existsSync(resultFile)) continue;
    const row = JSON.parse(readFileSync(resultFile, "utf8"));
    const project = row.driver === "A" ? join(run, "final") : join(run, "ai", "final");
    if (!existsSync(project)) { console.log(`${cell}/${sub}: no files to score`); continue; }
    let main = "main.rr";
    if (row.driver === "A") { try { main = JSON.parse(readFileSync(join(run, "summary.json"), "utf8")).main || main; } catch { /* default */ } }
    else { const m = /== final project: .* \(main (.+?)\)/.exec(existsSync(join(run, "ai_eval.log")) ? readFileSync(join(run, "ai_eval.log"), "utf8") : ""); if (m) main = m[1]; }
    const r = spawnSync(evalBin, ["--score", script, "--project", project, "--main", main, "--out", join(run, "rescore")], { env: { ...process.env, RAPIDR_AI_EVAL_RAPIDR: rapidr, RAPIDR_AI_KEYSTORE: "memory", RAPIDR_REGISTRY: join(run, "rescore.reg"), RAPIDR_AI_EVAL_KEY: "", RAPIDR_AI_EVAL_WORKSPACE: "" }, encoding: "utf8", timeout: 600000 });
    const text = `${r.stdout || ""}${r.stderr || ""}`;
    const score = /== SCORE (\d)/.exec(text);
    const line = (/== click test:.*/.exec(text) || [""])[0];
    if (!score) { console.log(`${cell}/${sub}: no score (${text.slice(-200)})`); continue; }
    const before = row.score;
    Object.assign(row, { score: Number(score[1]), compiles: /compiles yes/.test(line) ? true : /compiles no/.test(line) ? false : null, clickTest: line.replace(/^== click test: /, ""), rescored: true });
    writeFileSync(resultFile, JSON.stringify(row, null, 1));
    console.log(`${cell}/${sub}: ${before} -> ${row.score}  ${row.clickTest}`);
  }
}
