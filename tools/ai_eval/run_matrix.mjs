#!/usr/bin/env node
// The eval matrix: the same apps, two drivers, any models, one cell a run.
//
//   A  an outside AI app: tools/ai_eval/studio_mcp_agent.mjs, Studio's MCP tools only
//   B  Studio's own RAI: crates/rapidr-studio/examples/ai_eval.rs, headless
//
//   node tools/ai_eval/run_matrix.mjs --out dist/ai-eval-2026-10-29/runs \
//        --cells A:bonsai,B:bonsai,A:haiku,B:haiku [--apps tip-calculator,todo-list] [--held-out] \
//        [--rep N] [--cost-cap 30] [--ledger FILE] [--port-base 47950] [--keychain]
//
// cells: DRIVER:MODEL — MODEL is bonsai (bonsai-2:2b on Ollama, http://localhost:11434/v1),
// haiku (claude-haiku-5-5, Anthropic's Messages API), or mock (the tests' mock, free).
// Every cell's folder is --out/DRIVER-MODEL/APP/ with its transcript, summary and
// result.json (a finished cell is not run again: resume by running the same line).
// One run per cell — a model's answers vary, a cell is a sample, not a mean.
//
// Cost: a Haiku cell adds what it used to the ledger (US$ per million tokens: 0.10 in, 0.50
// out, 0.50 / 2.50 past a 100K prompt — the claude-api skill's table for Haiku 5.5); when the
// ledger reaches --cost-cap no Haiku cell is started, and each says "cost_cap".
//
// Never printed or written: the key. --keychain reads the maintainer's test key from the
// login keychain into this process's environment (and the child processes') only.

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync, appendFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(dirname(HERE));
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : d);
const out = resolve(opt("--out", join(ROOT, "dist", "ai-eval", "runs")));
const cells = opt("--cells", "A:mock").split(",");
const portBase = Number(opt("--port-base", 47950));
const costCap = Number(opt("--cost-cap", 30));
const ledger = resolve(opt("--ledger", join(dirname(out), "cost-ledger.json")));
const steps = opt("--steps", "40");
const minutes = opt("--minutes", "25");
const only = opt("--apps", "") ? opt("--apps").split(",") : null;
const useKeychain = argv.includes("--keychain");
const rep = Number(opt("--rep", 1));   // the run number of a cell: 1 is the first, 2 and 3 the repeats (own folders: APP.r2)

// The models are a parameter: a cell's MODEL is one of these names, `ollama/<model id>`
// (any model Ollama serves, e.g. `A:ollama/qwen3:4b`; free, local), or a name from
// --models FILE.json ({"name": {"id": "...", "url": "http://host:port/v1", "anthropic": false, "paid": false}}).
const MODELS = {
  bonsai: { id: "bonsai-2:2b", url: "http://localhost:11434/v1", openai: true, paid: false },
  haiku: { id: "claude-haiku-5-5", anthropic: true, paid: true },
  mock: { id: "mock-small", mock: true, paid: false },
};
if (opt("--models", "")) Object.assign(MODELS, JSON.parse(readFileSync(resolve(opt("--models")), "utf8")));
function modelOf(key) {
  if (MODELS[key]) return MODELS[key];
  // (Ollama's own API for driver A — num_ctx and thinking off, which /v1 can't say; its /v1 for driver B, RAI's compatible provider)
  if (key.startsWith("ollama/")) return { id: key.slice(7), url: opt("--ollama-url", "http://localhost:11434"), ollama: true, local: true, paid: false };
  return null;
}
// (a folder name: no "/" or ":")
const label = (key) => key.replace(/[\/:]/g, "_");
// which apps: tests/ai-eval/apps and held-out (the default), school, or any --set a,b
const sets = opt("--set", "apps,held-out").split(",");
const price = (i, o) => (i > 100000 ? (i * 0.5 + o * 2.5) / 1e6 : (i * 0.1 + o * 0.5) / 1e6);

function ledgerRead() { try { return JSON.parse(readFileSync(ledger, "utf8")); } catch { return { total: 0, inTokens: 0, outTokens: 0, requests: 0, runs: {} }; } }
function ledgerAdd(usd, i, o, what) {
  const l = ledgerRead();
  l.total += usd; l.inTokens += i; l.outTokens += o; l.requests += 1; l.runs[what] = (l.runs[what] || 0) + usd;
  mkdirSync(dirname(ledger), { recursive: true });
  writeFileSync(ledger, JSON.stringify(l, null, 1));
}

const env = { ...process.env };
if (useKeychain) {
  const get = (service) => { const r = spawnSync("security", ["find-generic-password", "-a", "rapidr-test", "-s", service, "-w"], { encoding: "utf8" }); return r.status === 0 ? r.stdout.trim() : ""; };
  env.RAPIDR_AI_EVAL_KEY = get("rapidr-test-anthropic");
  const ws = get("rapidr-test-anthropic-workspace");
  if (ws) env.RAPIDR_AI_EVAL_WORKSPACE = ws;
}
const secretList = () => [env.RAPIDR_AI_EVAL_KEY, env.RAPIDR_AI_EVAL_WORKSPACE].filter((s) => s && s.length > 6);
const redact = (t) => secretList().reduce((a, s) => a.split(s).join("[redacted]"), String(t)).replace(/sk-ant-[A-Za-z0-9_-]+/g, "[redacted]");

function appsList() {
  const dir = (d) => readdirSync(join(ROOT, "tests", "ai-eval", d)).filter((f) => f.endsWith(".txt")).sort().map((f) => ({ name: f.replace(/\.txt$/, ""), file: join(ROOT, "tests", "ai-eval", d, f), set: d, heldOut: d === "held-out" }));
  let apps = sets.flatMap(dir);
  if (only) apps = apps.filter((a) => only.includes(a.name));
  return apps;
}

function run(cmd, args, o) {
  return new Promise((res) => {
    const child = spawn(cmd, args, { cwd: ROOT, env: o.env || env, stdio: ["ignore", "pipe", "pipe"] });
    let text = "";
    child.stdout.on("data", (d) => (text += d));
    child.stderr.on("data", (d) => (text += d));
    const timer = setTimeout(() => child.kill(), (o.minutes || 40) * 60000);
    child.on("exit", (code) => { clearTimeout(timer); res({ code, text }); });
  });
}

const evalBin = process.env.RAPIDR_AI_EVAL_BIN || join(process.env.CARGO_TARGET_DIR || join(ROOT, "target"), "release", "examples", "ai_eval");
let n = 0;
for (const cell of cells) {
  const driver = cell.slice(0, 1);
  const modelKey = cell.slice(2);
  const model = modelOf(modelKey);
  if (!model || !["A", "B"].includes(driver) || cell[1] !== ":") { console.error(`bad cell ${cell}`); continue; }
  for (const app of appsList()) {
    const dir = join(out, `${driver}-${label(modelKey)}`, rep > 1 ? `${app.name}.r${rep}` : app.name);
    if (existsSync(join(dir, "result.json"))) { console.log(`skip ${driver}-${label(modelKey)} ${app.name} r${rep} (done)`); continue; }
    if (model.paid && ledgerRead().total >= costCap) {
      console.log(`${driver}-${label(modelKey)} ${app.name} r${rep}: not run, the US$${costCap} cap was reached (ledger US$${ledgerRead().total.toFixed(4)})`);
      continue;
    }
    mkdirSync(dir, { recursive: true });
    const port = portBase + (n++ % 24);
    const started = Date.now();
    let row = { cell: `${driver}-${label(modelKey)}`, driver, model: model.id, app: app.name, set: app.set, rep, heldOut: app.heldOut };
    const childEnv = { ...env, RAPIDR_AI_EVAL_RAPIDR: process.env.RAPIDR || env.RAPIDR_AI_EVAL_RAPIDR || "" };
    if (driver === "A") {
      const args = [join(HERE, "studio_mcp_agent.mjs"), "--app", app.file, "--out", dir, "--steps", steps, "--minutes", minutes, "--port", String(port)];
      if (model.mock) args.push("--dry");
      else if (model.anthropic) args.push("--driver", "anthropic", "--model", model.id, "--ledger", ledger, "--cost-cap", String(costCap));
      else if (model.ollama) args.push("--driver", "ollama", "--model", model.id, "--base-url", model.url, "--num-ctx", opt("--num-ctx", "32768"), "--think", opt("--think", "off"));
      else args.push("--driver", "openai", "--model", model.id, "--base-url", model.url);
      const r = await run(process.execPath, args, { env: childEnv, minutes: Number(minutes) + 20 });
      writeFileSync(join(dir, "agent.log"), redact(r.text));
      let s = {};
      try { s = JSON.parse(readFileSync(join(dir, "summary.json"), "utf8")); } catch { s = { stopped: "harness_error", detail: redact(r.text.split("\n").slice(-4).join(" | ")) }; }
      row = { ...row, score: s.score ?? null, compiles: s.compiles ?? null, stopped: s.stopped, detail: s.detail, requests: s.steps, toolCalls: s.toolCalls, toolErrors: s.toolErrors, inTok: s.inTokens, outTok: s.outTokens, usd: model.paid ? s.usd : 0, seconds: s.seconds, clickTest: s.clickTest, nativeCalls: s.nativeCallRequests, textCalls: s.textFallbacks, emptyReplies: s.emptyReplies, badArgs: s.badArgs, unknownTools: s.unknownTools };
    } else {
      // (B: Studio's own RAI: ai_eval clears its --out first, so its log lives beside it)
      const aiEnv = { ...childEnv, RAPIDR_AI_EVAL_MODEL: model.id, RAPIDR_AI_EVAL_PROVIDER: model.anthropic ? "anthropic" : "compatible", RAPIDR_AI_EVAL_URL: model.ollama ? `${model.url.replace(/\/v1\/?$/, "")}/v1` : model.url || "" };
      if (model.mock) { row = { ...row, stopped: "skipped", note: "B has no mock cell: tests/studio_flows.mjs ai-* cases cover it" }; writeFileSync(join(dir, "result.json"), JSON.stringify(row, null, 1)); continue; }
      const r = await run(evalBin, [app.file, "--out", join(dir, "ai")], { env: aiEnv, minutes: 30 });
      writeFileSync(join(dir, "ai_eval.log"), redact(r.text));
      const score = /== SCORE (\d)/.exec(r.text);
      const line = (/== click test:.*/.exec(r.text) || [""])[0];
      let s = {};
      try { s = JSON.parse(readFileSync(join(dir, "ai", "summary.json"), "utf8")); } catch { /* no summary */ }
      const turns = s.turns || [];
      let inTok = 0, outTok = 0;
      for (const t of turns) for (const u of t.usage || []) { const [i, o] = String(u).split("/").map(Number); inTok += i || 0; outTok += o || 0; }
      const usd = model.paid ? price(inTok, outTok) : 0;
      if (model.paid) ledgerAdd(usd, inTok, outTok, `B:${model.id}:${app.name}`);
      const toolList = turns.flatMap((t) => t.tools || []);
      row = { ...row, score: score ? Number(score[1]) : null, compiles: /compiles yes/.test(line) ? true : /compiles no/.test(line) ? false : null, stopped: turns.some((t) => t.aborted) ? "aborted" : turns.length ? "answered" : "no_summary", requests: turns.reduce((a, t) => a + (t.requests || []).length, 0), toolCalls: toolList.length, toolErrors: toolList.filter((t) => /\(error\)/.test(t)).length, inTok, outTok, usd, seconds: Math.round((Date.now() - started) / 1000), clickTest: line.replace(/^== click test: /, ""), falseClaims: s.falseClaims, nudges: turns.reduce((a, t) => a + (t.nudges || 0), 0), errors: turns.flatMap((t) => t.errors || []).slice(0, 3) };
    }
    row.seconds = row.seconds ?? Math.round((Date.now() - started) / 1000);
    writeFileSync(join(dir, "result.json"), JSON.stringify(row, null, 1));
    appendFileSync(join(out, "results.jsonl"), redact(JSON.stringify(row)) + "\n");
    console.log(`${row.cell} ${app.name} r${rep}: score ${row.score} (${row.stopped}) ${row.requests ?? "?"} req ${row.toolCalls ?? "?"} calls ${row.seconds}s${row.usd ? ` US$${row.usd.toFixed(4)}` : ""}`);
  }
  // (a local model is put away when its cells are done: one at a time in memory — only
  // models this matrix loaded, and only when the next cell is another model's)
  const next = cells[cells.indexOf(cell) + 1];
  if (argv.includes("--unload") && (model.local || model.openai) && next && modelOf(next.slice(2))?.id !== model.id) spawnSync("ollama", ["stop", model.id], { stdio: "ignore" });
}
console.log(`ledger: US$${ledgerRead().total.toFixed(4)}`);
