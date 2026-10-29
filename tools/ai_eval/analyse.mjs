#!/usr/bin/env node
// The matrix's folders → the score table, the cost, and the failures sorted into
// categories (each failed run gets ONE primary category by rules; the tool errors
// are counted over every run). Reads what run_matrix.mjs wrote.
//
//   node tools/ai_eval/analyse.mjs dist/ai-eval-2026-10-29/runs [--json out.json]
//
// Categories (a failed run = score 0):
//   server        our server: a protocol error or a connection failure
//   model_api     the model's server failed or refused (context too long, HTTP error)
//   cut_off       a reply hit its length limit
//   loop          the same call, answer and all, five times; or the step cap while still erring
//   empty         the model stopped with nothing: no text, no call
//   gave_up       it said it was done (or asked a question) with nothing built
//   no_compile    it ended with a program that doesn't compile (syntax, names)
//   click_fail    it compiles; the click test fails (a button or box not found, a wrong answer)
//   no_run        it compiles; the program didn't run to the end
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
if (!dir) { console.error("usage: analyse.mjs RUNS_DIR [--json FILE]"); process.exit(2); }
const jsonOut = process.argv.includes("--json") ? process.argv[process.argv.indexOf("--json") + 1] : null;
const readJson = (p) => { try { return JSON.parse(readFileSync(p, "utf8")); } catch { return null; } };

// ---- load every cell's runs ----------------------------------------------------------------
const runs = [];
for (const cell of readdirSync(dir).filter((d) => /^[AB]-/.test(d))) {
  for (const sub of readdirSync(join(dir, cell))) {
    const r = readJson(join(dir, cell, sub, "result.json"));
    if (!r) continue;
    const run = { ...r, dir: join(dir, cell, sub), name: sub };
    if (r.driver === "A") {
      run.summary = readJson(join(run.dir, "summary.json")) || {};
      run.events = existsSync(join(run.dir, "transcript.jsonl")) ? readFileSync(join(run.dir, "transcript.jsonl"), "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l)) : [];
    } else {
      run.summary = readJson(join(run.dir, "ai", "summary.json")) || {};
      run.log = existsSync(join(run.dir, "ai_eval.log")) ? readFileSync(join(run.dir, "ai_eval.log"), "utf8") : "";
    }
    runs.push(run);
  }
}

// ---- the failure category of a run ------------------------------------------------------------
const compileKind = (msg) => {
  if (/not part of class|unknown (component|type|class|member|property)|no such (member|property|component)|not a member/i.test(msg)) return "invented component, member or property";
  if (/unexpected '\\'|Unexpected character|isn't BASIC|Expected end-of-line|Syntax error|unexpected|Expected/i.test(msg)) return "syntax from another language";
  if (/not declared|undefined|unknown (variable|identifier|sub|function)|never declared|isn't declared/i.test(msg)) return "undeclared name";
  return "other compile error";
};
function categorise(run) {
  if (run.score === 1) return null;
  const s = run.summary;
  const click = run.clickTest || "";
  if (run.driver === "A") {
    if (s.protocolErrors > 0 || run.stopped === "server_error") return { cat: "server", why: s.detail || "protocol errors" };
    if (run.stopped === "model_error") return { cat: "model_api", why: s.detail };
    if (run.stopped === "max_tokens") return { cat: "cut_off", why: "" };
    if (run.stopped === "stuck_loop") return { cat: "loop", why: s.detail };
    const last = [...run.events].reverse().find((e) => e.type === "model");
    const built = (s.mainSourceLines || 0) > 3 || (s.tools?.create_project > 0);
    if (run.stopped === "step_cap" || run.stopped === "time_cap") {
      return run.compiles === false ? { cat: "loop", why: `step cap, never compiled: ${compileKind(click)}`, sub: compileKind(click) } : { cat: "loop", why: "step cap, compiles but fails the click test" };
    }
    if (last && !last.text && !last.calls.length) return { cat: "empty", why: "the last reply had no text and no call" };
    if (!built && run.compiles !== false) return { cat: "gave_up", why: (s.finalAnswer || "").slice(0, 120) };
    if (run.compiles === false) return { cat: "no_compile", why: click.replace(/^compile FAIL /, ""), sub: compileKind(click) };
    if (/didn't run to the end/.test(click)) return { cat: "no_run", why: click };
    return { cat: "click_fail", why: click, sub: /no (button|edit|list)/.test(click) ? "a button, box or list the test looks for isn't there" : "a wrong or missing result" };
  }
  // B: Studio's own RAI
  if ((run.errors || []).length) return { cat: "model_api", why: run.errors[0] };
  if (/final project: none/.test(run.log)) return { cat: "gave_up", why: "no project made (the answer was text)" };
  if (run.compiles === false) return { cat: "no_compile", why: click, sub: compileKind(click) };
  if (/didn't run to the end/.test(click)) return { cat: "no_run", why: click };
  return { cat: "click_fail", why: click, sub: /no (button|edit|list)/.test(click) ? "a button, box or list the test looks for isn't there" : "a wrong or missing result" };
}
for (const r of runs) r.failure = categorise(r);

// ---- the score table -----------------------------------------------------------------------------
const cells = [...new Set(runs.map((r) => r.cell))].sort();
const apps = [...new Set(runs.map((r) => r.app))].sort();
const held = new Set(runs.filter((r) => r.heldOut).map((r) => r.app));
const lines = [];
const out = (t = "") => lines.push(t);
const rate = (rs) => (rs.length ? `${rs.filter((r) => r.score === 1).length}/${rs.length}` : "–");
out("| app | " + cells.join(" | ") + " |");
out("|---|" + cells.map(() => "---").join("|") + "|");
for (const app of apps) out(`| ${app}${held.has(app) ? " (held out)" : ""} | ` + cells.map((c) => rate(runs.filter((r) => r.cell === c && r.app === app))).join(" | ") + " |");
out("| **all** | " + cells.map((c) => { const rs = runs.filter((r) => r.cell === c); const p = rs.filter((r) => r.score === 1).length; return `**${p}/${rs.length}** (${rs.length ? Math.round((100 * p) / rs.length) : 0}%)`; }).join(" | ") + " |");
out();

// ---- cost, time, size per cell ---------------------------------------------------------------------
out("| cell | runs | requests (mean) | tool calls (mean) | tool errors | tokens in / out (total) | seconds (mean) | US$ |");
out("|---|---|---|---|---|---|---|---|");
for (const c of cells) {
  const rs = runs.filter((r) => r.cell === c);
  const sum = (f) => rs.reduce((a, r) => a + (f(r) || 0), 0);
  out(`| ${c} | ${rs.length} | ${(sum((r) => r.requests) / rs.length).toFixed(1)} | ${(sum((r) => r.toolCalls) / rs.length).toFixed(1)} | ${sum((r) => r.toolErrors)} | ${sum((r) => r.inTok)} / ${sum((r) => r.outTok)} | ${(sum((r) => r.seconds) / rs.length).toFixed(0)} | ${sum((r) => r.usd).toFixed(4)} |`);
}
out();

// ---- failure categories ----------------------------------------------------------------------------------
const cats = {};
for (const r of runs) if (r.failure) (cats[`${r.cell}|${r.failure.cat}`] ||= []).push(r);
const catNames = ["server", "model_api", "cut_off", "loop", "empty", "gave_up", "no_compile", "click_fail", "no_run"];
out("Failed runs by primary category:");
out();
out("| category | " + cells.join(" | ") + " |");
out("|---|" + cells.map(() => "---").join("|") + "|");
for (const k of catNames) out(`| ${k} | ` + cells.map((c) => (cats[`${c}|${k}`] || []).length || "").join(" | ") + " |");
out();
out("Compile failures by kind:");
out();
const kinds = {};
for (const r of runs) if (r.failure?.sub && (r.failure.cat === "no_compile" || (r.failure.cat === "loop" && r.compiles === false))) { const k = `${r.cell}|${r.failure.sub}`; kinds[k] = (kinds[k] || 0) + 1; }
out("| kind | " + cells.join(" | ") + " |");
out("|---|" + cells.map(() => "---").join("|") + "|");
for (const k of ["invented component, member or property", "syntax from another language", "undeclared name", "other compile error"]) out(`| ${k} | ` + cells.map((c) => kinds[`${c}|${k}`] || "").join(" | ") + " |");
out();

// ---- A: which tools were used, which errors came back (over every run) -------------------------------
const aRuns = runs.filter((r) => r.driver === "A");
const errorsOf = (r) => r.events.filter((e) => e.type === "tool" && !e.ok);
const norm = (t) => String(t).replace(/\s+/g, " ").replace(/"[^"]{30,}"/g, '"…"').replace(/\d+/g, "N").replace(/main\.rr:N:N:?/g, "main.rr:L:C").slice(0, 110);
out("Tools called (runs that called the tool at least once / runs), outside AI app:");
out();
const toolNames = [...new Set(aRuns.flatMap((r) => r.events.filter((e) => e.type === "tool").map((e) => e.name)))].sort();
out("| tool | " + cells.filter((c) => c.startsWith("A-")).join(" | ") + " |");
out("|---|" + cells.filter((c) => c.startsWith("A-")).map(() => "---").join("|") + "|");
for (const t of toolNames) out(`| ${t} | ` + cells.filter((c) => c.startsWith("A-")).map((c) => { const rs = aRuns.filter((r) => r.cell === c); return `${rs.filter((r) => r.events.some((e) => e.type === "tool" && e.name === t)).length}/${rs.length}`; }).join(" | ") + " |");
out();
out("Most common tool errors (outside AI app, by normalised text):");
out();
const errCount = {};
for (const r of aRuns) for (const e of errorsOf(r)) { const k = `${e.name}: ${norm(e.error)}`; (errCount[k] ||= { n: 0, cells: new Set(), runs: new Set(), example: e.error }).n++; errCount[k].cells.add(r.cell); errCount[k].runs.add(r.dir); }
for (const [k, v] of Object.entries(errCount).sort((a, b) => b[1].n - a[1].n).slice(0, 25)) out(`- ${v.n}× in ${v.runs.size} runs (${[...v.cells].join(", ")}): ${k}`);
out();

// ---- B: what RAI's runs show ---------------------------------------------------------------------------------
const bRuns = runs.filter((r) => r.driver === "B");
if (bRuns.length) {
  out("Studio's own RAI (driver B): tool results by tool, over every run:");
  out();
  const bt = {};
  for (const r of bRuns) for (const t of (r.summary.turns || []).flatMap((x) => x.tools || [])) { const m = /^(\w+)\((\w+)\)$/.exec(t); if (!m) continue; bt[`${r.cell}|${m[1]}|${m[2]}`] = (bt[`${r.cell}|${m[1]}|${m[2]}`] || 0) + 1; }
  for (const [k, v] of Object.entries(bt).sort()) out(`- ${k.replace(/\|/g, " ")}: ${v}`);
  out();
}

console.log(lines.join("\n"));
if (jsonOut) writeFileSync(jsonOut, JSON.stringify(runs.map((r) => ({ cell: r.cell, app: r.app, rep: r.rep, name: r.name, score: r.score, compiles: r.compiles, stopped: r.stopped, requests: r.requests, toolCalls: r.toolCalls, toolErrors: r.toolErrors, inTok: r.inTok, outTok: r.outTok, usd: r.usd, seconds: r.seconds, failure: r.failure, clickTest: r.clickTest })), null, 1));
