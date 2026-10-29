#!/usr/bin/env node
// An "outside AI app" for RapidR Studio: one model builds one eval app
// (tests/ai-eval/apps/*.txt) using ONLY what Studio's MCP server offers —
// its tool list, its tool descriptions and its server instructions, nothing
// else — so what a small model gets stuck on shows what those words teach
// (and don't). The same app is then scored by the click tests of
// `ai_eval --score` (tests/ai-eval/README.md), exactly as Studio's own RAI's
// runs are, so the two drivers' scores can be set side by side.
//
//   node tools/ai_eval/studio_mcp_agent.mjs --app tests/ai-eval/apps/tip-calculator.txt \
//        --driver openai --model bonsai-2:2b --out scratch/run1
//   node tools/ai_eval/studio_mcp_agent.mjs --app … --driver anthropic --model claude-haiku-5-5 --out …
//   node tools/ai_eval/studio_mcp_agent.mjs --dry --app …        (the mock, no model, no cost)
//   node tools/ai_eval/studio_mcp_agent.mjs --list-tools          (Studio's tools as a model sees them)
//
// What it does: Studio is started headless on a COPY of the empty project
// tests/fixtures/ai_eval_empty with its MCP server on (mode "all": nothing
// asks), a registry, keychain, printer and documents folder of its own
// under --out (never the user's). The tool list is fetched over MCP and
// given to the model (Anthropic's tool format for --driver anthropic,
// OpenAI's for --driver openai: Ollama, LM Studio, mlx-serve …); the app's
// request (one user turn a line, as ai_eval replays them) is sent; tool
// calls are made over MCP and their results returned, until the model stops
// without a call, or a cap (steps, time, cost, a loop) ends the run. Then the
// project Studio has open is scored by the click tests.
//
// A small system prompt only: "You are working in RapidR Studio through its
// tools; build the app the user asks for." plus the server's own
// instructions (what any MCP client hands its model).
//
// Written to --out: transcript.jsonl (every request and tool call: tool,
// argument size, result status and error text, tokens, seconds),
// summary.json (steps, tokens, seconds, where it stopped, the score),
// final/ (the project's files), studio.log, score.log, window.png (Studio at the end).
//
// Environment (keys are read here, kept in memory, never printed or written;
// transcripts are redacted):
//   RAPIDR                   the rapidr to run Studio with (default ./rapidr, else target/release/rapidr)
//   RAPIDR_AI_EVAL_BIN       the built ai_eval example (default $CARGO_TARGET_DIR/release/examples/ai_eval, else cargo)
//   RAPIDR_AI_EVAL_URL       --driver openai: the server (default http://localhost:11434/v1)
//   RAPIDR_AI_EVAL_KEY       the key, when the server wants one (--driver anthropic: required)
//   RAPIDR_AI_EVAL_WORKSPACE Anthropic's workspace id, when the key needs one
//   AI_EVAL_PRICE_IN/OUT     US$ per million tokens (default: Haiku 5.5, 0.10 / 0.50; 0.50 / 2.50 past 100K a request)
//
// Options: --steps N (model requests, default 40) --minutes N (default 25)
// --cost-cap USD --ledger FILE (a running total shared by many runs: the cap
// is over all of them) --port N --keep-studio --no-score.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, writeFileSync, appendFileSync } from "node:fs";
import net from "node:net";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(dirname(HERE));
const MODERN = "2026-07-28";
const CLIENT = "eval-agent";
const END_CLIENT = "eval-end";
const SYSTEM = "You are working in RapidR Studio through its tools; build the app the user asks for.";

// ---- arguments -------------------------------------------------------------------------
function parseArgs(argv) {
  const a = { steps: 40, minutes: 25, driver: "openai", port: 0, flags: new Set() };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    const val = () => argv[++i];
    switch (k) {
      case "--app": a.app = val(); break;
      case "--model": a.model = val(); break;
      case "--driver": a.driver = val(); break;
      case "--base-url": a.baseUrl = val(); break;
      case "--out": a.out = val(); break;
      case "--steps": a.steps = Number(val()); break;
      case "--minutes": a.minutes = Number(val()); break;
      case "--cost-cap": a.costCap = Number(val()); break;
      case "--ledger": a.ledger = val(); break;
      case "--port": a.port = Number(val()); break;
      case "--calls": a.calls = val(); break;   // a JSON list of {"name", "args", "timeoutSeconds"} made as an AI app would, no model (to reproduce a problem)
      case "--rapidr": a.rapidr = val(); break;
      case "--max-tokens": a.maxTokens = Number(val()); break;
      case "--num-ctx": a.numCtx = Number(val()); break;     // --driver ollama: the context window asked for (default 32768)
      case "--system-extra": a.systemExtra = val(); break;   // text added to the system prompt after the server's instructions (an experiment: what a better instruction would be worth)
      case "--think": a.think = val() === "on"; break;       // --driver ollama: thinking on or off (default off)
      case "--dry": case "--list-tools": case "--keep-studio": case "--no-score": case "--keychain": a.flags.add(k.slice(2)); break;
      default: console.error(`unknown option ${k}`); process.exit(2);
    }
  }
  return a;
}
const args = parseArgs(process.argv.slice(2));
const has = (f) => args.flags.has(f);

// --keychain (macOS): the maintainer's test key and workspace id from the login
// keychain (`rapidr-test-anthropic`, `rapidr-test-anthropic-workspace`), into this
// process's memory only — never an argument, a file or the output.
if (has("keychain") && args.driver === "anthropic" && !process.env.RAPIDR_AI_EVAL_KEY) {
  const get = (service) => {
    const r = spawnSync("security", ["find-generic-password", "-a", "rapidr-test", "-s", service, "-w"], { encoding: "utf8" });
    return r.status === 0 ? r.stdout.trim() : "";
  };
  process.env.RAPIDR_AI_EVAL_KEY = get("rapidr-test-anthropic");
  const ws = get("rapidr-test-anthropic-workspace");
  if (ws) process.env.RAPIDR_AI_EVAL_WORKSPACE = ws;
}

// ---- secrets: kept in memory, redacted from everything written ------------------------------
const secrets = () => [process.env.RAPIDR_AI_EVAL_KEY, process.env.RAPIDR_AI_EVAL_WORKSPACE, process.env.ANTHROPIC_API_KEY, process.env.ANTHROPIC_WORKSPACE_ID].filter((s) => s && s.length > 6);
function redact(text) {
  let t = String(text);
  for (const s of secrets()) t = t.split(s).join("[redacted]");
  return t.replace(/sk-ant-[A-Za-z0-9_-]+/g, "[redacted]").replace(/(authorization|x-api-key|anthropic-workspace-id)\s*[:=]\s*\S+/gi, "$1: [redacted]");
}

// ---- the processes started here (only these are ever killed) ------------------------------
const children = [];
process.on("exit", () => children.forEach((c) => { try { c.kill(); } catch { /* gone */ } }));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function freePort() {
  return new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, "127.0.0.1", () => { const p = s.address().port; s.close(() => resolve(p)); });
  });
}
async function listening(port, ms = 90000) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const ok = await new Promise((res) => {
      const s = net.connect({ host: "127.0.0.1", port }, () => { s.end(); res(true); });
      s.on("error", () => res(false));
    });
    if (ok) return true;
    await sleep(250);
  }
  return false;
}

// ---- Studio, headless, with its MCP server on (as tests/studio_mcp.mjs does it) -------------
const TOKEN = "eval-agent-token-" + Math.random().toString(16).slice(2, 18).padEnd(16, "0");
function rapidrBinary() {
  if (args.rapidr) return resolve(args.rapidr);
  if (process.env.RAPIDR) return resolve(process.env.RAPIDR);
  for (const p of [join(ROOT, "rapidr"), join(ROOT, "target", "release", "rapidr"), join(process.env.CARGO_TARGET_DIR || "", "release", "rapidr")]) if (p && existsSync(p)) return p;
  console.error("no rapidr: set RAPIDR or build `cargo build --release -p rapidr-cli`");
  process.exit(2);
}

function startStudio(work, port, capture) {
  const project = join(work, "project");
  rmSync(project, { recursive: true, force: true });
  cpSync(join(ROOT, "tests", "fixtures", "ai_eval_empty"), project, { recursive: true });
  const dirs = ["prints", "docs", "chats"].map((d) => join(work, d));
  dirs.forEach((d) => mkdirSync(d, { recursive: true }));
  // (Studio's "home": the repository's folders, linked in — a project it makes
  // goes to <home>/projects, here under --out, never into the repository)
  const home = join(work, "home");
  mkdirSync(home, { recursive: true });
  for (const e of readdirSync(ROOT)) {
    if ([".git", ".claude", ".github", ".cargo", "target", "dist", "projects", "scratch", "node_modules"].includes(e)) continue;
    try { symlinkSync(join(ROOT, e), join(home, e)); } catch { /* there */ }
  }
  const env = {
    ...process.env,
    // (the test events play only under RAPIDR_CAPTURE; the picture is taken with the capture tool)
    RAPIDR_CAPTURE: join(work, "window"),
    RAPIDR_CAPTURE_DELAY: "1.5",
    RAPIDR_MENU: "window",
    RAPIDR_TEST_MCP_TOKEN: TOKEN,
    RAPIDR_PRINT_TO: join(work, "prints"),
    RAPIDR_REGISTRY: join(work, "studio.reg"),
    RAPIDR_DOCUMENTS: join(work, "docs"),
    RAPIDR_AI_KEYSTORE: "memory",
    RAPIDR_AI_AUDIT: join(work, "audit.jsonl"),
    RAPIDR_AI_CHATS: join(work, "chats"),
    RAPIDR_LOGIN_PATH: "off",
    // (Studio never sees a key: it is not the one talking to a model here)
    ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "",
    RAPIDR_AI_EVAL_KEY: "", RAPIDR_AI_EVAL_WORKSPACE: "",
    // (the "user": a project the AI app makes is in the default mode, "Ask before
    // changes", and Studio would wait for a person in the AI Apps pane; this
    // person presses Allow for This Session whenever something waits, then
    // Studio is held for the run — the harness ends it itself)
    RAPIDR_TEST_EVENTS: [...Array(12).fill("mcpask.__until_Visible_1,mcpsessionbtn.onclick"), "studio.__wait_3000000"].join(","),
    RAPIDR_TEST_DUMP: "mcphead.caption,mcplog.itemcount",
  };
  const argv = ["run", "ide/studio.rr", "--home", home, "--fresh", "--theme", "rapidr-light", "--do", `mcp:mode=all,mcp:on=${port}`, join(project, "MyApp.rrproj")];
  const child = spawn(rapidrBinary(), argv, { cwd: ROOT, env, stdio: ["ignore", "pipe", "pipe"] });
  children.push(child);
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  child.stderr.on("data", (d) => (out += d));
  const done = new Promise((res) => child.on("exit", (code) => res(code)));
  return { child, project, home, done, log: () => out };
}

// ---- an MCP client (the protocol code of tests/studio_mcp.mjs) ---------------------------
function mcpClient(port, clientName, timeoutMs = 240000) {
  let nextId = 1;
  const meta = { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": { name: clientName, version: "1.0" }, "io.modelcontextprotocol/clientCapabilities": {} };
  async function post(body, headers = {}) {
    const r = await fetch(`http://127.0.0.1:${port}/mcp`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream", Authorization: `Bearer ${TOKEN}`, ...headers },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(timeoutMs),
    });
    const text = await r.text();
    let json = {};
    const progress = [];
    if ((r.headers.get("content-type") || "").includes("text/event-stream")) {
      for (const block of text.split("\n\n")) {
        const data = block.split("\n").filter((l) => l.startsWith("data: ")).map((l) => l.slice(6)).join("\n");
        if (!data) continue;
        const m = JSON.parse(data);
        if (m.method) progress.push(m);
        else json = m;
      }
    } else {
      try { json = JSON.parse(text); } catch { /* not JSON */ }
    }
    return { status: r.status, json, text, progress };
  }
  async function call(method, params = {}) {
    const name = params.name || params.uri;
    return post({ jsonrpc: "2.0", id: nextId++, method, params: { ...params, _meta: meta } }, { "MCP-Protocol-Version": MODERN, "Mcp-Method": method, ...(name ? { "Mcp-Name": name } : {}) });
  }
  /// One tool call: { text, error, images, ok } — `ok` false means OUR server
  /// or the connection failed (not "the tool said no").
  async function tool(name, toolArgs = {}) {
    try {
      const r = await call("tools/call", { name, arguments: toolArgs });
      if (r.status !== 200 && !r.json.result) return { text: `HTTP ${r.status}: ${r.text.slice(0, 500)}`, error: true, protocol: true, images: 0 };
      const res = r.json.result || {};
      const text = (res.content || []).filter((c) => c.type !== "image").map((c) => c.text || "").join("\n") || (r.json.error ? JSON.stringify(r.json.error) : "");
      return { text, error: !!res.isError || !!r.json.error, protocol: !!r.json.error, images: (res.content || []).filter((c) => c.type === "image").length };
    } catch (e) {
      return { text: `(no answer from Studio: ${e.message})`, error: true, protocol: true, images: 0 };
    }
  }
  /// A picture of Studio (the capture tool's PNG), as bytes, or null.
  async function picture() {
    try {
      const r = await call("tools/call", { name: "capture", arguments: {} });
      const img = (r.json.result?.content || []).find((c) => c.type === "image");
      return img ? Buffer.from(img.data, "base64") : null;
    } catch { return null; }
  }
  return { post, call, tool, picture };
}

// ---- the money (a ledger shared by many runs; the cap is over all of them) --------------------
const PRICE = { in: Number(process.env.AI_EVAL_PRICE_IN || 0.10), out: Number(process.env.AI_EVAL_PRICE_OUT || 0.50), inLong: 0.50, outLong: 2.50, longAt: 100000 };
function costOf(inTok, outTok) {
  const long = inTok > PRICE.longAt && !process.env.AI_EVAL_PRICE_IN;
  return ((inTok * (long ? PRICE.inLong : PRICE.in)) + (outTok * (long ? PRICE.outLong : PRICE.out))) / 1e6;
}
function ledgerTotal() {
  if (!args.ledger || !existsSync(args.ledger)) return 0;
  try { return JSON.parse(readFileSync(args.ledger, "utf8")).total || 0; } catch { return 0; }
}
function ledgerAdd(usd, inTok, outTok, what) {
  if (!args.ledger) return;
  let l = { total: 0, inTokens: 0, outTokens: 0, requests: 0, runs: {} };
  try { l = JSON.parse(readFileSync(args.ledger, "utf8")); } catch { /* new */ }
  l.total += usd; l.inTokens += inTok; l.outTokens += outTok; l.requests += 1;
  l.runs[what] = (l.runs[what] || 0) + usd;
  writeFileSync(args.ledger, JSON.stringify(l, null, 1));
}

// ---- the models ---------------------------------------------------------------------------
/// A model's answer: { text, thinking, calls: [{ id, name, input }], stop, inTok, outTok, raw (the assistant message to keep), format }
const toolFormats = {
  anthropic: (tools) => tools.map((t) => ({ name: t.name, description: t.description || "", input_schema: t.inputSchema || { type: "object", properties: {} } })),
  // ({"type": "object"} with no "properties", as six of Studio's tools are listed, is valid MCP but Ollama's grammar for some models' tool-call templates answers HTTP 500 "properties must be an object": an OpenAI-format client says it)
  openai: (tools) => tools.map((t) => ({ type: "function", function: { name: t.name, description: t.description || "", parameters: t.inputSchema && t.inputSchema.type === "object" && !t.inputSchema.properties ? { ...t.inputSchema, properties: {} } : t.inputSchema || { type: "object", properties: {} } } })),
};

/// Text tool calls (a small model that writes the call instead of making it):
/// `<tool_call>{"name":…,"arguments":{…}}</tool_call>`, or a bare/fenced JSON
/// object with `name` and `arguments`.
function textToolCalls(text, known) {
  const found = [];
  const candidates = [];
  for (const m of text.matchAll(/<tool_call>([\s\S]*?)(?:<\/tool_call>|$)/g)) candidates.push(m[1]);
  for (const m of text.matchAll(/```(?:json)?\s*([\s\S]*?)```/g)) candidates.push(m[1]);
  if (!candidates.length && /^\s*\{[\s\S]*\}\s*$/.test(text)) candidates.push(text);
  for (const c of candidates) {
    try {
      const j = JSON.parse(c.trim());
      const name = j.name || j.tool || j.function?.name;
      const input = j.arguments ?? j.parameters ?? j.input ?? j.function?.arguments ?? {};
      if (name && known.has(name)) found.push({ id: `text_${found.length}_${Date.now()}`, name, input: typeof input === "string" ? JSON.parse(input) : input });
    } catch { /* not a call */ }
  }
  return found;
}

async function anthropicTurn(model, system, messages, tools) {
  const key = process.env.RAPIDR_AI_EVAL_KEY || process.env.ANTHROPIC_API_KEY;
  if (!key) throw Object.assign(new Error("no key: set RAPIDR_AI_EVAL_KEY"), { fatal: true });
  const headers = { "content-type": "application/json", "x-api-key": key, "anthropic-version": "2023-06-01" };
  const ws = process.env.RAPIDR_AI_EVAL_WORKSPACE || process.env.ANTHROPIC_WORKSPACE_ID;
  if (ws) headers["anthropic-workspace-id"] = ws;
  const base = (args.baseUrl || process.env.RAPIDR_AI_EVAL_URL || "https://api.anthropic.com").replace(/\/v1\/?$/, "").replace(/\/$/, "");
  const body = { model, max_tokens: args.maxTokens || 16000, system, messages, tools: toolFormats.anthropic(tools) };
  for (let attempt = 0; ; attempt++) {
    const r = await fetch(`${base}/v1/messages`, { method: "POST", headers, body: JSON.stringify(body), signal: AbortSignal.timeout(600000) });
    const text = await r.text();
    if ((r.status === 429 || r.status === 529 || r.status >= 500) && attempt < 3) { await sleep(2000 * (attempt + 1)); continue; }
    let j;
    try { j = JSON.parse(text); } catch { throw new Error(`HTTP ${r.status}: not JSON: ${redact(text.slice(0, 300))}`); }
    if (!r.ok) throw new Error(`HTTP ${r.status}: ${redact(j.error?.message || text.slice(0, 300))}`);
    const content = j.content || [];
    return {
      text: content.filter((b) => b.type === "text").map((b) => b.text).join("\n"),
      thinking: content.filter((b) => b.type === "thinking").map((b) => b.thinking || "").join("\n"),
      calls: content.filter((b) => b.type === "tool_use").map((b) => ({ id: b.id, name: b.name, input: b.input || {} })),
      stop: j.stop_reason,
      inTok: (j.usage?.input_tokens || 0) + (j.usage?.cache_read_input_tokens || 0) + (j.usage?.cache_creation_input_tokens || 0),
      outTok: j.usage?.output_tokens || 0,
      // (the whole content back, thinking blocks and all, as the API wants them)
      assistant: { role: "assistant", content },
      format: "native",
    };
  }
}

/// OpenAI-compatible chat, streamed (Ollama, LM Studio, mlx-serve, the mock).
async function openaiTurn(model, system, messages, tools, known) {
  const key = process.env.RAPIDR_AI_EVAL_KEY;
  const base = (args.baseUrl || process.env.RAPIDR_AI_EVAL_URL || "http://localhost:11434/v1").replace(/\/$/, "");
  const headers = { "content-type": "application/json", ...(key ? { authorization: `Bearer ${key}` } : {}) };
  const body = { model, stream: true, stream_options: { include_usage: true }, max_tokens: args.maxTokens || 8192, messages: [{ role: "system", content: system }, ...messages], tools: toolFormats.openai(tools) };
  const r = await fetch(`${base}/chat/completions`, { method: "POST", headers, body: JSON.stringify(body), signal: AbortSignal.timeout(900000) });
  if (!r.ok) throw new Error(`HTTP ${r.status}: ${redact((await r.text()).slice(0, 400))}`);
  let text = "", thinking = "", stop = "", inTok = 0, outTok = 0;
  const calls = new Map();
  const decoder = new TextDecoder();
  let buf = "";
  const tail = [];   // (the stream's last data lines: what ended it, when nothing says)
  const allLines = async function* () {
    for await (const chunk of r.body) {
      buf += decoder.decode(chunk, { stream: true });
      let at;
      while ((at = buf.indexOf("\n")) >= 0) { const l = buf.slice(0, at).trim(); buf = buf.slice(at + 1); yield l; }
    }
    if (buf.trim()) yield buf.trim();   // (a last line with no newline after it)
  };
  for await (const line of allLines()) {
    {
      if (!line.startsWith("data:")) continue;
      const data = line.slice(5).trim();
      tail.push(data.slice(0, 300)); if (tail.length > 4) tail.shift();
      if (data === "[DONE]") continue;
      let j;
      try { j = JSON.parse(data); } catch { continue; }
      if (j.error) throw new Error(`server: ${redact(JSON.stringify(j.error)).slice(0, 300)}`);
      if (j.usage) { inTok = j.usage.prompt_tokens || inTok; outTok = j.usage.completion_tokens || outTok; }
      const ch = j.choices?.[0];
      if (!ch) continue;
      const d = ch.delta || {};
      if (d.content) text += d.content;
      if (d.reasoning_content) thinking += d.reasoning_content;
      if (d.reasoning) thinking += d.reasoning;
      for (const tc of d.tool_calls || []) {
        const i = tc.index ?? 0;
        const c = calls.get(i) || { id: "", name: "", args: "" };
        if (tc.id) c.id = tc.id;
        if (tc.function?.name) c.name += tc.function.name;
        if (tc.function?.arguments) c.args += typeof tc.function.arguments === "string" ? tc.function.arguments : JSON.stringify(tc.function.arguments);
        calls.set(i, c);
      }
      if (ch.finish_reason) stop = ch.finish_reason;
    }
  }
  let list = [...calls.values()].map((c, i) => {
    let input = {};
    let bad = "";
    try { input = c.args.trim() ? JSON.parse(c.args) : {}; } catch (e) { bad = c.args; }
    return { id: c.id || `call_${i}_${Date.now()}`, name: c.name, input, badArgs: bad };
  });
  let format = "native";
  if (!list.length && text) {
    const t = textToolCalls(text, known);
    if (t.length) { list = t; format = "text-fallback"; }
  }
  return {
    text, thinking, calls: list, stop, inTok, outTok, format, rawTail: stop ? undefined : tail,
    assistant: { role: "assistant", content: list.length ? text || null : text, ...(list.length ? { tool_calls: list.map((c) => ({ id: c.id, type: "function", function: { name: c.name, arguments: JSON.stringify(c.input) } })) } : {}) },
  };
}

/// Ollama's own chat API (/api/chat), streamed: what /v1 can't say — the context window
/// (options.num_ctx) and thinking on or off (think) — and where a call arrived: in
/// message.tool_calls ("native") or as text in message.content ("text-fallback": the parser
/// of that model's template didn't make it a call).
async function ollamaTurn(model, system, messages, tools, known) {
  const base = (args.baseUrl || process.env.RAPIDR_AI_EVAL_URL || "http://localhost:11434").replace(/\/v1\/?$/, "").replace(/\/$/, "");
  const names = new Map();   // (a call's id → its tool, for the results that name it)
  const wire = messages.map((m) => {
    if (m.role === "assistant" && m.tool_calls) {
      for (const c of m.tool_calls) names.set(c.id, c.function.name);
      return { role: "assistant", content: m.content || "", tool_calls: m.tool_calls.map((c) => ({ function: { name: c.function.name, arguments: JSON.parse(c.function.arguments || "{}") } })) };
    }
    if (m.role === "tool") return { role: "tool", tool_name: names.get(m.tool_call_id) || "", content: m.content };
    return m;
  });
  const body = { model, stream: true, messages: [{ role: "system", content: system }, ...wire], tools: toolFormats.openai(tools), options: { num_ctx: args.numCtx || 32768, num_predict: args.maxTokens || 8192 } };
  if (args.think !== undefined) body.think = args.think;
  else body.think = false;
  let r = await fetch(`${base}/api/chat`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body), signal: AbortSignal.timeout(900000) });
  if (r.status === 400) {
    const said = await r.text();
    if (/does not support thinking|think/i.test(said)) {
      delete body.think;   // (a model without a thinking switch)
      r = await fetch(`${base}/api/chat`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body), signal: AbortSignal.timeout(900000) });
    } else throw new Error(`HTTP 400: ${redact(said.slice(0, 400))}`);
  }
  if (!r.ok) throw new Error(`HTTP ${r.status}: ${redact((await r.text()).slice(0, 400))}`);
  let text = "", thinking = "", stop = "", inTok = 0, outTok = 0;
  const found = [];
  const tail = [];
  const decoder = new TextDecoder();
  let buf = "";
  const handle = (line) => {
    if (!line.trim()) return;
    tail.push(line.slice(0, 300)); if (tail.length > 4) tail.shift();
    let j;
    try { j = JSON.parse(line); } catch { return; }
    if (j.error) throw new Error(`server: ${redact(String(j.error)).slice(0, 300)}`);
    const m = j.message || {};
    if (m.content) text += m.content;
    if (m.thinking) thinking += m.thinking;
    for (const tc of m.tool_calls || []) {
      const f = tc.function || {};
      let input = f.arguments ?? {};
      let bad = "";
      if (typeof input === "string") { try { input = JSON.parse(input); } catch { bad = input; input = {}; } }
      found.push({ id: `call_${found.length}_${Date.now()}`, name: f.name || "", input, badArgs: bad });
    }
    if (j.done) { stop = j.done_reason || "stop"; inTok = j.prompt_eval_count || inTok; outTok = j.eval_count || outTok; }
  };
  for await (const chunk of r.body) {
    buf += decoder.decode(chunk, { stream: true });
    let at;
    while ((at = buf.indexOf("\n")) >= 0) { handle(buf.slice(0, at)); buf = buf.slice(at + 1); }
  }
  handle(buf);
  let list = found;
  let format = "native";
  if (!list.length && text) {
    const t = textToolCalls(text, known);
    if (t.length) { list = t.map((c) => ({ ...c, badArgs: "" })); format = "text-fallback"; }
  }
  return {
    text, thinking, calls: list, stop, inTok, outTok, format, rawTail: stop ? undefined : tail,
    assistant: { role: "assistant", content: list.length ? text || null : text, ...(list.length ? { tool_calls: list.map((c) => ({ id: c.id, type: "function", function: { name: c.name, arguments: JSON.stringify(c.input) } })) } : {}) },
  };
}

// ---- the run -------------------------------------------------------------------------------------
function turnsOf(file) {
  return readFileSync(file, "utf8").split("\n").map((l) => l.trim()).filter((l) => l && !l.startsWith("#"));
}
function short(s, n) { s = String(s); return s.length > n ? s.slice(0, n) + `… (+${s.length - n})` : s; }
function walkFiles(root, dir = root, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.name.startsWith(".")) continue;
    const p = join(dir, e.name);
    if (e.isDirectory()) walkFiles(root, p, out);
    else out.push(p.slice(root.length + 1));
  }
  return out;
}

async function main() {
  const out = resolve(args.out || join(process.env.TMPDIR || "/tmp", "rapidr-mcp-agent"));
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const transcript = join(out, "transcript.jsonl");
  const log = (o) => appendFileSync(transcript, redact(JSON.stringify({ t: Math.round((Date.now() - T0) / 100) / 10, ...o })) + "\n");
  const T0 = Date.now();

  let mock = null;
  if (has("dry")) {
    mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
    children.push(mock);
    args.baseUrl = await new Promise((res) => mock.stdout.once("data", (d) => res(String(d).trim())));
    args.driver = "openai";
    args.model = args.model || "mock-small";
  }
  const port = args.port || (await freePort());
  const studio = startStudio(out, port, !has("list-tools"));
  if (!(await listening(port))) {
    writeFileSync(join(out, "studio.log"), redact(studio.log()));
    console.error("Studio didn't start listening; see studio.log");
    process.exit(3);
  }
  const mcp = mcpClient(port, CLIENT);
  const disc = await mcp.call("server/discover");
  const instructions = disc.json.result?._meta?.["io.modelcontextprotocol/serverInfo"]?.instructions || disc.json.result?.instructions || "";
  const tools = (await mcp.call("tools/list")).json.result?.tools || [];
  if (has("list-tools")) {
    console.log(JSON.stringify({ instructions, discover: disc.json.result, tools }, null, 1));
    studio.child.kill();
    return;
  }
  if (args.calls) {
    // (no model: the calls of a file, in order, each with its own wait — what Studio's MCP server
    // does with them, to reproduce a problem a run showed)
    const list = JSON.parse(readFileSync(resolve(args.calls), "utf8"));
    for (const c of list) {
      const t0 = Date.now();
      const r = await mcpClient(port, "eval-calls", (c.timeoutSeconds || 30) * 1000).tool(c.name, c.args || {});
      console.log(`-> ${c.name} ${short(JSON.stringify(c.args || {}), 120)}\n   ${r.error ? "ERROR" : "ok"}${r.protocol ? " (NO ANSWER / PROTOCOL)" : ""} after ${((Date.now() - t0) / 1000).toFixed(1)}s: ${short(r.text.replace(/\s+/g, " "), 300)}`);
    }
    const shot = await mcpClient(port, "eval-calls", 20000).picture();
    if (shot) writeFileSync(join(out, "window.png"), shot);
    studio.child.kill();
    return;
  }
  const known = new Set(tools.map((t) => t.name));
  // (--system-extra: a sentence more in the server's instructions, to measure what a change to them would be worth)
  const system = [SYSTEM, instructions, args.systemExtra].filter(Boolean).join("\n\n");
  const turns = args.app ? turnsOf(args.app) : ["Build me a tip calculator"];
  const run = { app: args.app ? basename(args.app, ".txt") : "adhoc", driver: args.driver, model: args.model, tools: tools.length, toolBytes: JSON.stringify(tools).length, systemChars: system.length, turns: turns.length };
  log({ type: "start", ...run, instructionsFromServer: instructions.length });
  console.log(`== ${run.driver} ${run.model} · ${run.app} · ${tools.length} tools (${run.toolBytes} bytes) · system ${system.length} chars`);

  const messages = [];
  const ledgerKey = `${run.driver}:${run.model}:${run.app}`;
  let steps = 0, inTotal = 0, outTotal = 0, cost = 0, toolCalls = 0, toolErrors = 0, protocolErrors = 0, textFallbacks = 0, formatRetries = 0, nativeCallRequests = 0, emptyReplies = 0, badArgs = 0, unknownTools = 0, maxPrompt = 0;
  let stopped = "", detail = "";
  const toolHist = new Map();
  const sig = [];
  const deadline = Date.now() + args.minutes * 60000;
  let nextTurn = 0;
  const callModel = args.driver === "anthropic" ? anthropicTurn : args.driver === "ollama" ? (m, s, ms, t) => ollamaTurn(m, s, ms, t, known) : (m, s, ms, t) => openaiTurn(m, s, ms, t, known);
  turnLoop: while (nextTurn < turns.length) {
    const userText = turns[nextTurn++];
    messages.push({ role: "user", content: userText });
    log({ type: "user", text: userText });
    for (;;) {
      if (steps >= args.steps) { stopped = "step_cap"; break turnLoop; }
      if (Date.now() > deadline) { stopped = "time_cap"; break turnLoop; }
      if (args.costCap && ledgerTotal() >= args.costCap) { stopped = "cost_cap"; detail = `ledger at US$${ledgerTotal().toFixed(4)}`; break turnLoop; }
      steps++;
      const t0 = Date.now();
      let ans;
      // (Ollama: a model's own tool-call syntax that its template's parser can't read comes back as
      // an error ("XML syntax error …"), or as a reply with nothing in it; a client asks again —
      // up to three times in a run, each said in the transcript and counted as the model's format
      // failures, not hidden)
      for (;;) {
        try {
          ans = await callModel(args.model, system, messages, tools);
        } catch (e) {
          if (args.driver === "ollama" && formatRetries < 3 && /XML syntax error|parsing failed|invalid character|unexpected EOF|tool call/i.test(e.message)) {
            formatRetries++;
            log({ type: "format_retry", step: steps, error: redact(e.message).slice(0, 200) });
            continue;
          }
          stopped = e.fatal ? "setup_error" : "model_error";
          detail = redact(e.message);
          log({ type: "model_error", step: steps, error: detail });
          break turnLoop;
        }
        if (args.driver === "ollama" && formatRetries < 3 && !ans.calls.length && !ans.text) {
          formatRetries++;
          log({ type: "format_retry", step: steps, error: "an empty reply (no text, no call)" });
          continue;
        }
        break;
      }
      const usd = args.driver === "anthropic" ? costOf(ans.inTok, ans.outTok) : 0;
      cost += usd; inTotal += ans.inTok; outTotal += ans.outTok; maxPrompt = Math.max(maxPrompt, ans.inTok);
      ledgerAdd(usd, ans.inTok, ans.outTok, ledgerKey);
      if (ans.format === "text-fallback") textFallbacks++;
      else if (ans.calls.length) nativeCallRequests++;
      if (!ans.calls.length && !ans.text) emptyReplies++;
      log({ type: "model", step: steps, secs: Math.round((Date.now() - t0) / 100) / 10, inTok: ans.inTok, outTok: ans.outTok, usd: Number(usd.toFixed(6)), stop: ans.stop, format: ans.format, thinkingChars: ans.thinking.length, rawTail: ans.rawTail, text: short(ans.text, 1500), calls: ans.calls.map((c) => ({ name: c.name, argBytes: JSON.stringify(c.input).length, badArgs: c.badArgs ? short(c.badArgs, 200) : undefined })) });
      messages.push(ans.assistant);
      if (!ans.calls.length) {
        if (ans.stop === "length" || ans.stop === "max_tokens") { stopped = "max_tokens"; detail = "the reply hit its length limit"; break turnLoop; }
        break; // (the model is done with this turn)
      }
      const results = [];
      for (const c of ans.calls) {
        toolCalls++;
        toolHist.set(c.name, (toolHist.get(c.name) || 0) + 1);
        const t1 = Date.now();
        let r;
        if (!known.has(c.name)) {
          unknownTools++;
          r = { text: `Unknown tool ${c.name}.`, error: true, protocol: false, images: 0 };
        } else if (c.badArgs) {
          badArgs++;
          r = { text: `The arguments of ${c.name} weren't valid JSON: ${short(c.badArgs, 300)}`, error: true, protocol: false, images: 0, ours: false };
        } else {
          r = await mcp.tool(c.name, c.input);
        }
        if (r.error) toolErrors++;
        if (r.protocol) protocolErrors++;
        const argText = JSON.stringify(c.input);
        log({ type: "tool", step: steps, name: c.name, argBytes: argText.length, args: short(argText, 6000), ok: !r.error, protocolError: !!r.protocol || undefined, error: r.error ? short(r.text, 1500) : undefined, resultBytes: r.text.length, result: r.error ? undefined : short(r.text, 700), images: r.images || undefined, secs: Math.round((Date.now() - t1) / 100) / 10 });
        results.push({ call: c, text: r.text + (r.images ? `\n[${r.images} image(s) not shown to this model]` : ""), error: r.error });
        sig.push(`${c.name}|${argText}|${r.text.slice(0, 200)}`);
      }
      if (args.driver === "anthropic") {
        messages.push({ role: "user", content: results.map((x) => ({ type: "tool_result", tool_use_id: x.call.id, content: x.text.slice(0, 60000) || "(empty)", ...(x.error ? { is_error: true } : {}) })) });
      } else {
        for (const x of results) messages.push({ role: "tool", tool_call_id: x.call.id, content: x.text.slice(0, 60000) || "(empty)" });
      }
      if (protocolErrors >= 2) { stopped = "server_error"; detail = "Studio stopped answering (two calls failed: a timeout, a connection)"; break turnLoop; }
      // (the same call, the same answer, five times: it is stuck, and going on only costs)
      if (sig.length >= 5 && sig.slice(-5).every((s) => s === sig[sig.length - 1])) { stopped = "stuck_loop"; detail = sig[sig.length - 1].split("|")[0]; break turnLoop; }
    }
  }
  if (!stopped) stopped = "model_stopped";

  // ---- the end: the project Studio has open, a picture of Studio, then Studio is ended -------
  const end = mcpClient(port, END_CLIENT, 20000);   // (a Studio that has stopped answering is not waited for again)
  const status = await end.tool("studio_status");
  const picture = await end.picture();
  // What the AI app made lives in Studio's editor (a change is "applied, unsaved":
  // Run saves it) — so the files are read back from Studio as the user would see
  // them, not from the disk.
  const editor = {};
  const listed = await end.tool("list_files");
  for (const line of listed.error ? [] : listed.text.split("\n")) {
    const f = line.replace(/ \(main\)$/, "").trim();
    if (!f || !/\.(rr|bas|inc|txt|csv|json|md)$/i.test(f)) continue;
    const r = await end.tool("read_file", { path: f });
    if (r.error) continue;
    try {
      const j = JSON.parse(r.text.split("\n(only its first")[0]);
      if (typeof j.content === "string" && !r.text.includes("(only its first")) editor[f] = j.content;
    } catch { /* not the usual wrapper */ }
  }
  const lastText = [...messages].reverse().find((m) => m.role === "assistant");
  const finalAnswer = typeof lastText?.content === "string" ? lastText.content : Array.isArray(lastText?.content) ? lastText.content.filter((b) => b.type === "text").map((b) => b.text).join("\n") : "";
  if (picture) writeFileSync(join(out, "window.png"), picture);
  studio.child.kill();
  await Promise.race([studio.done, sleep(10000)]);
  writeFileSync(join(out, "studio.log"), redact(studio.log()));

  // which project: the one Studio says it has open, else the one we started (a run may
  // have made a new project: create_project puts it in the documents folder)
  let folder = studio.project;
  let main = "main.rr";
  // ("Project: NAME (FOLDER); main file main.rr; files: …")
  const said = /^Project: .*? \((.+?)\); main file (.+?);/m.exec(status.text);
  if (said && existsSync(said[1].trim()) && statSync(said[1].trim()).isDirectory()) folder = said[1].trim();
  const mainRe = said ? [null, said[2]] : null;
  const proj = readdirSync(folder).find((f) => f.endsWith(".rrproj"));
  if (proj) {
    const m = /^main\s*=\s*"([^"]+)"/m.exec(readFileSync(join(folder, proj), "utf8"));
    if (m) main = m[1];
  } else if (mainRe) main = mainRe[1].trim();
  if (mainRe && !existsSync(join(folder, main))) main = mainRe[1].trim();
  const files = walkFiles(folder);
  for (const f of files) {
    const dest = join(out, "final", f);
    mkdirSync(dirname(dest), { recursive: true });
    cpSync(join(folder, f), dest);
  }
  // (the editor's text over the disk's: what Run would save)
  let unsaved = 0;
  for (const [f, text] of Object.entries(editor)) {
    const dest = join(out, "final", f);
    const onDisk = existsSync(join(folder, f)) ? readFileSync(join(folder, f), "utf8") : null;
    if (onDisk !== text) unsaved++;
    mkdirSync(dirname(dest), { recursive: true });
    writeFileSync(dest, text);
  }
  if (!existsSync(join(out, "final", main)) && existsSync(join(folder, main))) cpSync(join(folder, main), join(out, "final", main));
  const mainSource = existsSync(join(out, "final", main)) ? readFileSync(join(out, "final", main), "utf8") : "";

  // ---- the click test --------------------------------------------------------------------------------
  let score = null, scoreLine = "", compiles = null;
  if (!has("no-score") && args.app) {
    const bin = process.env.RAPIDR_AI_EVAL_BIN || (process.env.CARGO_TARGET_DIR ? join(process.env.CARGO_TARGET_DIR, "release", "examples", "ai_eval") : join(ROOT, "target", "release", "examples", "ai_eval"));
    const scoreArgs = ["--score", resolve(args.app), "--project", join(out, "final"), "--main", main, "--out", join(out, "score")];
    const cmd = existsSync(bin) ? [bin, scoreArgs] : ["cargo", ["run", "-q", "--release", "-p", "rapidr-studio", "--example", "ai_eval", "--", ...scoreArgs]];
    const s = spawnSync(cmd[0], cmd[1], { cwd: ROOT, env: { ...process.env, RAPIDR_AI_EVAL_RAPIDR: rapidrBinary(), RAPIDR_AI_KEYSTORE: "memory", RAPIDR_REGISTRY: join(out, "score.reg"), RAPIDR_AI_EVAL_KEY: "", RAPIDR_AI_EVAL_WORKSPACE: "" }, encoding: "utf8", timeout: 600000 });
    const text = redact(`${s.stdout || ""}${s.stderr || ""}`);
    writeFileSync(join(out, "score.log"), text);
    const m = /== SCORE (\d)/.exec(text);
    score = m ? Number(m[1]) : null;
    scoreLine = (/== click test:.*/.exec(text) || [""])[0];
    compiles = /compiles yes/.test(scoreLine) ? true : /compiles no/.test(scoreLine) ? false : null;
  }
  const summary = {
    ...run, stopped, detail, steps, toolCalls, toolErrors, protocolErrors, unknownTools, badArgs, textFallbacks, formatRetries, nativeCallRequests, emptyReplies, maxPromptTokens: maxPrompt,
    inTokens: inTotal, outTokens: outTotal, usd: Number(cost.toFixed(5)), seconds: Math.round((Date.now() - T0) / 1000),
    tools: Object.fromEntries(toolHist), project: folder === studio.project ? "MyApp (the copy of the empty project)" : basename(folder), main, files: files.length, unsavedFiles: unsaved,
    compiles, score, clickTest: scoreLine.replace(/^== click test: /, ""), finalAnswer: short(finalAnswer, 1500), mainSourceLines: mainSource.split("\n").length,
  };
  writeFileSync(join(out, "summary.json"), JSON.stringify(summary, null, 1));
  log({ type: "end", ...summary, studioStatus: short(status.text, 1500) });
  console.log(`   stopped: ${stopped}${detail ? ` (${detail})` : ""} · ${steps} requests · ${toolCalls} tool calls (${toolErrors} errors) · ${inTotal}/${outTotal} tokens · ${summary.seconds}s${cost ? ` · US$${cost.toFixed(4)}` : ""}`);
  console.log(`   ${scoreLine || "(not scored)"}`);
  console.log(`== SCORE ${score ?? "?"}`);
  if (mock) mock.kill();
  // (--dry is the guard that the harness itself still works: the mock "builds" its hello
  // form with create_project; what must hold is the plumbing, not the score)
  if (has("dry")) {
    const problems = [];
    if (stopped !== "model_stopped") problems.push(`stopped: ${stopped} ${detail}`);
    if (!toolHist.get("create_project")) problems.push("the mock's create_project call wasn't made");
    if (toolErrors || protocolErrors) problems.push(`${toolErrors} tool errors, ${protocolErrors} protocol errors`);
    if (!mainSource.includes("CREATE Form")) problems.push("the project Studio has open wasn't read back (no form in its main file)");
    if (compiles !== true) problems.push("the click test didn't run or the program doesn't compile");
    if (!existsSync(join(out, "window.png"))) problems.push("no picture of Studio");
    console.log(problems.length ? `DRY CHECK FAILED: ${problems.join("; ")}` : "DRY CHECK OK");
    process.exit(problems.length ? 1 : 0);
  }
  process.exit(0);
}

main().catch((e) => { console.error(redact(e.stack || e.message)); process.exit(1); });
