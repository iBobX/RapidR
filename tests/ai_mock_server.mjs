// A mock AI provider for RapidR Studio's assistant (tests/studio_flows.mjs'
// ai-* cases, the manual's scenes): no key, no network, no cost. It speaks
// both wire formats RapidR uses — OpenAI-compatible Chat Completions
// (`/v1/chat/completions`, `/v1/models`) and Anthropic's Messages
// (`/v1/messages`, `/v1/models`) — streamed as server-sent events, with
// scripted answers chosen by the conversation's first message:
//
//   "hello …"          a short answer
//   "say hello …"      five words (examples/ai/rai_models.rr's question)
//   "rename …"         read_file the main file, then propose_edit its form's Caption → "AI Notes"
//   "build me …"       create_project: a hello-form program (it compiles)
//   "build a scientific calculator …"  create_project named "Scientific
//                      Calculator" (a title with a space): RapidR's calculator recipe
//   "build me a broken …"  create_project with a compile error it never fixes
//   "pretend …"        first claims the program is made (no tool), then — asked
//                      by Studio — makes it
//   "slowly …"         any of these, each answer after a tool's held 6 s
//   "check …"          check_code, then say what it found
//   "describe …"       rapidr_describe RButton, rapidr_list, then answer
//   "inject …"         try write_file outside its permission (read-only tests)
//
// Each step of a script is the number of tool results so far. CORS is
// open (the web Studio's page calls it from its origin).
//
// It plays the cloud providers too, at the places RapidR sends them when
// RAPIDR_AI_TEST_SERVER names this server (rapidr_ai::provider::
// test_server, a loopback address only): /anthropic/v1, /openai/v1,
// /deepseek — each with made-up models in its own shape (OpenAI's with
// models a chat can't use, which RapidR leaves out), each wanting a key as
// the real one does. The key decides what happens, so a test types a fake
// one into the key box as a user would:
//
//   …bad…      refused: 401, in the provider's own words
//   …fail…     500, every time (RapidR tries once more, then says so)
//   …slow…     no answer at all (RapidR gives up after its time limit)
//   …garbage…  200, but a web page instead of JSON
//   …empty…    a list with no models
//   …org…      (Anthropic) an organisation's key, not a workspace's:
//              refused unless the request names a workspace
//              (anthropic-workspace-id), as Anthropic does
//
// A compatible server (no key) plays the same through its address,
// /s/<bad|fail|slow|garbage|empty>/v1. GET /__log lists what was asked of
// it (the keys' last four characters only).
//
//   node tests/ai_mock_server.mjs [port]      (prints its /v1 URL)

import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { pathToFileURL } from "node:url";

const HELLO_FORM = `' hello_form: a form with a name box and a button that greets.
DECLARE SUB Greet

CREATE Form AS RForm
    Caption = "Hello Form"
    Width = 320: Height = 160
    Center
    CREATE NameEdit AS REdit
        Text = "World"
        Left = 16: Top = 20: Width = 180
    END CREATE
    CREATE GreetButton AS RButton
        Caption = "&Greet"
        Left = 208: Top = 18: Width = 90
        OnClick = Greet
    END CREATE
    CREATE Answer AS RLabel
        Caption = "Type a name, then Greet."
        Left = 16: Top = 64: Width = 280
    END CREATE
END CREATE

SUB Greet
    Answer.Caption = "Hello, " + NameEdit.Text + "!"
END SUB

Form.ShowModal
`;

// (RapidR's own calculator recipe: what a model starting from it makes)
const SCI_CALC = readFileSync(new URL("../examples/recipes/calculator.rr", import.meta.url), "utf8");

// (a program with a form and one compile error: RapidR has no BOOLEAN)
const BROKEN = `' broken: a form, and a type RapidR doesn't have.
DIM ready AS BOOLEAN

CREATE Form AS RForm
    Caption = "Broken Tool"
    Width = 300: Height = 140
    CREATE Info AS RLabel
        Caption = "Not quite there yet."
        Left = 16: Top = 20: Width = 240
    END CREATE
END CREATE

Form.ShowModal
`;

function lastUserText(messages) {
  for (const m of messages) {
    if (m.role !== "user") continue;
    if (typeof m.content === "string") return m.content;
    const t = (m.content || []).find((b) => b.type === "text");
    if (t) return t.text;
  }
  return "";
}

// (the tool results so far, in order: { name, content })
function toolResults(format, messages) {
  const calls = new Map();
  const out = [];
  for (const m of messages) {
    if (format === "openai") {
      for (const c of m.tool_calls || []) calls.set(c.id, c.function.name);
      if (m.role === "tool") out.push({ name: calls.get(m.tool_call_id), content: m.content });
    } else if (Array.isArray(m.content)) {
      for (const b of m.content) {
        if (b.type === "tool_use") calls.set(b.id, b.name);
        if (b.type === "tool_result") out.push({ name: calls.get(b.tool_use_id), content: typeof b.content === "string" ? b.content : JSON.stringify(b.content) });
      }
    }
  }
  return out;
}

// (Studio asked the model to make the change it only claimed: its nudge,
// a message of RapidR Studio's own)
function nudged(messages) {
  return messages.some((m) => m.role === "user" && JSON.stringify(m.content).includes("your answer says you made or will make a change"));
}

// What to answer: { text?, tools?: [{ name, input }] }.
function script(first, results, system, messages = []) {
  const step = results.length;
  const q = first.toLowerCase();
  const mainFile = (system.match(/main file ([^;]+);/) || [])[1] || "main.rr";
  // (Robert's chat, 2026-10-18: a title with spaces — the folder is made
  // from it, "Scientific Calculator" → ScientificCalculator)
  if (q.startsWith("build a scientific calculator")) {
    if (step === 0) return { text: "", tools: [{ name: "create_project", input: { name: "Scientific Calculator", main: "main.rr", summary: "A calculator with sqrt, x², sin, cos and tan", files: [{ path: "main.rr", text: SCI_CALC }] } }] };
    return { text: "Press **Run**, then click the keys: **sqrt**, **x²**, **sin**, **cos** and **tan** work on the number shown." };
  }
  // (a program that doesn't compile, and stays so: Studio creates it all
  // the same, and says it has errors)
  if (q.startsWith("build me a broken")) {
    if (step === 0) return { tools: [{ name: "create_project", input: { name: "Broken Tool", main: "main.rr", summary: "Broken", files: [{ path: "main.rr", text: BROKEN } ] } }] };
    return { text: "I couldn't fix the last error: a property RForm doesn't have." };
  }
  // (an answer that claims a program before making it: Studio asks once,
  // then it is made — the claim collapses to one muted line)
  if (q.startsWith("pretend")) {
    if (!nudged(messages)) return { text: "I have created the hello-form project for you. It is ready to run!" };
    if (step === 0) return { tools: [{ name: "create_project", input: { name: "hello-form", main: "main.rr", summary: "A form that greets", files: [{ path: "main.rr", text: HELLO_FORM }] } }] };
    return { text: "Type a name, then press **Greet**." };
  }
  if (q.startsWith("rename")) {
    if (step === 0) return { text: "Let me read the form first.", tools: [{ name: "read_file", input: { path: mainFile } }] };
    if (step === 1) {
      let file = "";
      try { file = JSON.parse(results[0].content).content; } catch { file = ""; }
      const m = file.match(/Caption = "[^"]*"/);
      if (!m) return { text: "I couldn't find the form's caption." };
      return { tools: [{ name: "propose_edit", input: { path: mainFile, summary: "The window's title becomes AI Notes", edits: [{ old_text: m[0], new_text: 'Caption = "AI Notes"' }] } }] };
    }
    return { text: "I proposed a new title for the window, **AI Notes**: review it and apply it if you like it." };
  }
  if (q.startsWith("build me")) {
    if (step === 0) return { text: "Here is a small program with a form.", tools: [{ name: "create_project", input: { name: "hello-form", main: "main.rr", summary: "A form that greets the name typed in it", files: [{ path: "main.rr", text: HELLO_FORM }] } }] };
    return { text: "The project **hello-form** is ready to preview: a form with a name box and a Greet button." };
  }
  if (q.startsWith("check")) {
    if (step === 0) return { tools: [{ name: "check_code", input: {} }] };
    return { text: `The compiler says: ${results[0].content.split("\n")[0]}` };
  }
  if (q.startsWith("describe")) {
    if (step === 0) return { tools: [{ name: "rapidr_describe", input: { type: "RButton" } }, { name: "rapidr_list", input: {} }] };
    return { text: `RButton is ${results[0].content.split("\n")[0].slice(0, 60)}…; the form has ${(results[1].content.match(/"id"/g) || []).length} components.` };
  }
  if (q.startsWith("inject")) {
    if (step === 0) return { tools: [{ name: "write_file", input: { path: "../outside.txt", text: "pwned", summary: "x" } }] };
    return { text: "I can't change files here." };
  }
  // (examples/ai/rai_models.rr's question)
  if (q.startsWith("say hello")) return { text: "Hello, friend, nice meeting you." };
  return { text: "Hello from the mock assistant! Ask me to **rename** the window, **build me** a program, or **check** the code." };
}

// ---- the two formats, streamed ---------------------------------------------------

function sseOpenAI(res, answer, model) {
  const send = (o) => res.write(`data: ${JSON.stringify(o)}\n\n`);
  const base = { id: "chatcmpl-mock", object: "chat.completion.chunk", model };
  send({ ...base, choices: [{ index: 0, delta: { role: "assistant", content: "" } }] });
  if (answer.text) for (const piece of answer.text.match(/.{1,12}/gs)) send({ ...base, choices: [{ index: 0, delta: { content: piece } }] });
  (answer.tools || []).forEach((t, i) => {
    const args = JSON.stringify(t.input);
    send({ ...base, choices: [{ index: 0, delta: { tool_calls: [{ index: i, id: `call_${Date.now()}_${i}`, type: "function", function: { name: t.name, arguments: "" } }] } }] });
    for (const piece of args.match(/.{1,40}/gs)) send({ ...base, choices: [{ index: 0, delta: { tool_calls: [{ index: i, function: { arguments: piece } }] } }] });
  });
  send({ ...base, choices: [{ index: 0, delta: {}, finish_reason: answer.tools ? "tool_calls" : "stop" }] });
  send({ ...base, choices: [], usage: { prompt_tokens: 1000, completion_tokens: 20 } });
  res.write("data: [DONE]\n\n");
  res.end();
}

function sseAnthropic(res, answer, model) {
  const send = (type, o) => res.write(`event: ${type}\ndata: ${JSON.stringify({ type, ...o })}\n\n`);
  send("message_start", { message: { id: "msg_mock", type: "message", role: "assistant", model, content: [], usage: { input_tokens: 900, cache_read_input_tokens: 100, output_tokens: 1 } } });
  let i = 0;
  if (answer.text) {
    send("content_block_start", { index: i, content_block: { type: "text", text: "" } });
    for (const piece of answer.text.match(/.{1,12}/gs)) send("content_block_delta", { index: i, delta: { type: "text_delta", text: piece } });
    send("content_block_stop", { index: i });
    i++;
  }
  for (const t of answer.tools || []) {
    send("content_block_start", { index: i, content_block: { type: "tool_use", id: `toolu_${Date.now()}_${i}`, name: t.name, input: {} } });
    for (const piece of JSON.stringify(t.input).match(/.{1,40}/gs)) send("content_block_delta", { index: i, delta: { type: "input_json_delta", partial_json: piece } });
    send("content_block_stop", { index: i });
    i++;
  }
  send("message_delta", { delta: { stop_reason: answer.tools ? "tool_use" : "end_turn" }, usage: { output_tokens: 20 } });
  send("message_stop", {});
  res.end();
}

// The models it lists: the tests' two, or (the manual's pictures) the ones
// AI_MOCK_MODELS names, "id:Display name,id2" — made-up names, never a real
// provider's.
const MODELS = process.env.AI_MOCK_MODELS
  ? process.env.AI_MOCK_MODELS.split(",").map((m, i, all) => {
    const [id, name] = m.split(":");
    return { id, object: "model", created: all.length - i, ...(name ? { display_name: name } : {}) };
  })
  : [{ id: "mock-small", object: "model", created: 2, display_name: "Mock Small" }, { id: "mock-large", object: "model", created: 1 }];

// (the cloud providers' lists: made-up models, each in its provider's shape)
const PROVIDER_MODELS = {
  anthropic: { data: [
    { type: "model", id: "claude-mock-opus-1", display_name: "Claude Mock Opus 1", created_at: "2026-09-01T00:00:00Z" },
    { type: "model", id: "claude-mock-sonnet-1", display_name: "Claude Mock Sonnet 1", created_at: "2026-09-01T00:00:00Z" },
    { type: "model", id: "claude-mock-haiku-1", display_name: "Claude Mock Haiku 1", created_at: "2026-09-01T00:00:00Z" },
  ], has_more: false, first_id: "claude-mock-opus-1", last_id: "claude-mock-haiku-1" },
  openai: { object: "list", data: [
    { id: "gpt-mock", object: "model", created: 30, owned_by: "system" },
    { id: "gpt-mock-mini", object: "model", created: 20, owned_by: "system" },
    { id: "gpt-mock-nano", object: "model", created: 10, owned_by: "system" },
    { id: "text-embedding-mock", object: "model", created: 40, owned_by: "system" },
    { id: "whisper-mock", object: "model", created: 5, owned_by: "system" },
    { id: "gpt-mock-realtime", object: "model", created: 35, owned_by: "system" },
  ] },
  deepseek: { object: "list", data: [
    { id: "deepseek-mock-pro", object: "model", owned_by: "deepseek" },
    { id: "deepseek-mock-flash", object: "model", owned_by: "deepseek" },
  ] },
};

// (each provider's refusal of a key, worded as it words it)
const REFUSED = {
  anthropic: { type: "error", error: { type: "authentication_error", message: "invalid x-api-key" } },
  openai: { error: { message: "Incorrect API key provided: mock****. You can find your API key in your account's settings.", type: "invalid_request_error", param: null, code: "invalid_api_key" } },
  deepseek: { error: { message: "Authentication Fails, Your api key: ****mock is invalid", type: "authentication_error", param: null, code: "invalid_request_error" } },
  compatible: { error: { message: "Unauthorized" } },
};
const NO_WORKSPACE = { type: "error", error: { type: "invalid_request_error", message: "This API key is not scoped to a workspace, so this request must include the anthropic-workspace-id header with the ID of the workspace to use." } };

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-headers": "authorization, content-type, x-api-key, anthropic-version, anthropic-workspace-id, anthropic-dangerous-direct-browser-access",
  "access-control-allow-methods": "GET, POST, OPTIONS",
};

// (the key a request carries, either way it's sent)
function keyOf(headers) {
  const bearer = (headers.authorization || "").replace(/^Bearer\s+/i, "");
  return headers["x-api-key"] || bearer || "";
}
const keyHint = (k) => (k ? `…${k.slice(-4)}` : "");
const SCENARIOS = ["bad", "fail", "slow", "garbage", "empty", "org"];

/** Starts the mock on `port` (0: any); resolves to { url, root, close, requests }. */
export function startMock(port = 0) {
  const requests = [];
  const held = new Set();
  const server = createServer((req, res) => {
    if (req.method === "OPTIONS") {
      res.writeHead(204, CORS).end();
      return;
    }
    let body = "";
    req.on("data", (c) => (body += c));
    req.on("end", () => {
      const path = new URL(req.url, "http://x").pathname;
      const send = (status, obj, type = "application/json") => {
        res.writeHead(status, { "content-type": type, ...CORS });
        res.end(typeof obj === "string" ? obj : JSON.stringify(obj));
      };
      if (path === "/__log") {
        send(200, requests.map((r) => ({ method: r.method, path: r.path, key: keyHint(keyOf(r.headers)), workspace: r.headers["anthropic-workspace-id"] || "", version: r.headers["anthropic-version"] || "", model: r.model })));
        return;
      }
      let json = {};
      try { json = JSON.parse(body || "{}"); } catch { /* not JSON */ }
      requests.push({ method: req.method, path, headers: req.headers, body, model: json.model || "" });
      // (whom it plays: a cloud provider by its place, else a compatible
      // server — a scenario, if any, in its address)
      const who = path.match(/^\/(anthropic|openai|deepseek)(\/|$)/);
      const provider = who ? who[1] : "compatible";
      const key = keyOf(req.headers);
      let scenario = (path.match(/^\/s\/([a-z]+)\//) || [])[1] || "";
      if (who) scenario = key ? SCENARIOS.find((w) => key.toLowerCase().includes(w)) || "" : "bad";
      if (scenario === "slow") {
        // (no answer: the client gives up; the socket goes with the server,
        // or after two minutes)
        held.add(res);
        setTimeout(() => { held.delete(res); if (!res.writableEnded) res.destroy(); }, 120000).unref();
        return;
      }
      if (scenario === "bad") return send(401, REFUSED[provider]);
      if (scenario === "fail") return send(500, { error: { message: "The server had an error while processing your request.", type: "server_error" } });
      if (scenario === "garbage") return send(200, "<html><body><h1>Welcome!</h1><p>A web page, not an API.</p></body></html>", "text/html");
      if (scenario === "org" && provider === "anthropic" && !req.headers["anthropic-workspace-id"]) return send(400, NO_WORKSPACE);
      if (req.method === "GET" && path.endsWith("/models")) {
        if (scenario === "empty") return send(200, provider === "anthropic" ? { data: [], has_more: false } : { object: "list", data: [] });
        // (AI_MOCK_MODELS: the manual's pictures' made-up names, every provider's)
        return send(200, who && !process.env.AI_MOCK_MODELS ? PROVIDER_MODELS[provider] : { object: "list", data: MODELS });
      }
      const anthropic = path.endsWith("/messages");
      if (!anthropic && !path.endsWith("/chat/completions")) {
        res.writeHead(404, { "content-type": "application/json", ...CORS }).end(JSON.stringify({ error: { message: `no ${path} here` } }));
        return;
      }
      if (json.model === "mock-missing") {
        res.writeHead(404, { "content-type": "application/json", ...CORS }).end(JSON.stringify({ error: { message: "The model `mock-missing` does not exist", code: "model_not_found" } }));
        return;
      }
      const messages = json.messages || [];
      const system = anthropic ? (json.system || []).map((b) => b.text).join("") : (messages.find((m) => m.role === "system") || {}).content || "";
      // ("slowly …": the same script, each answer after the first held 6 s
      // — a small local model's pace: the progress checklist shows meanwhile)
      let first = lastUserText(messages);
      const slow = /^slowly /i.test(first);
      if (slow) first = first.replace(/^slowly /i, "");
      const results = toolResults(anthropic ? "anthropic" : "openai", messages);
      const answer = script(first, results, system, messages);
      const reply = () => {
        if (res.writableEnded || res.destroyed) return;
        res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-cache", ...CORS });
        (anthropic ? sseAnthropic : sseOpenAI)(res, answer, json.model || "mock");
      };
      if (slow && results.length > 0) setTimeout(reply, 6000).unref();
      else reply();
    });
  });
  const close = () => {
    for (const r of held) r.destroy();
    server.close();
  };
  return new Promise((resolve) => server.listen(port, "127.0.0.1", () => {
    const root = `http://127.0.0.1:${server.address().port}`;
    resolve({ url: `${root}/v1`, root, close, requests });
  }));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const m = await startMock(Number(process.argv[2] || 0));
  console.log(m.url);
}
