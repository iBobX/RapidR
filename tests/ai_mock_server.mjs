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
//   "check …"          check_code, then say what it found
//   "describe …"       rapidr_describe RButton, rapidr_list, then answer
//   "inject …"         try write_file outside its permission (read-only tests)
//
// Each step of a script is the number of tool results so far. CORS is
// open (the web Studio's page calls it from its origin).
//
//   node tests/ai_mock_server.mjs [port]      (prints its URL)

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

// What to answer: { text?, tools?: [{ name, input }] }.
function script(first, results, system) {
  const step = results.length;
  const q = first.toLowerCase();
  const mainFile = (system.match(/main file ([^;]+);/) || [])[1] || "main.rr";
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

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-headers": "authorization, content-type, x-api-key, anthropic-version, anthropic-workspace-id, anthropic-dangerous-direct-browser-access",
  "access-control-allow-methods": "GET, POST, OPTIONS",
};

/** Starts the mock on `port` (0: any); resolves to { url, close, requests }. */
export function startMock(port = 0) {
  const requests = [];
  const server = createServer((req, res) => {
    if (req.method === "OPTIONS") {
      res.writeHead(204, CORS).end();
      return;
    }
    let body = "";
    req.on("data", (c) => (body += c));
    req.on("end", () => {
      const path = new URL(req.url, "http://x").pathname;
      requests.push({ method: req.method, path, headers: req.headers, body });
      if (req.method === "GET" && path.endsWith("/models")) {
        res.writeHead(200, { "content-type": "application/json", ...CORS });
        res.end(JSON.stringify({ object: "list", data: MODELS }));
        return;
      }
      let json = {};
      try { json = JSON.parse(body || "{}"); } catch { /* not JSON */ }
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
      const answer = script(lastUserText(messages), toolResults(anthropic ? "anthropic" : "openai", messages), system);
      res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-cache", ...CORS });
      (anthropic ? sseAnthropic : sseOpenAI)(res, answer, json.model || "mock");
    });
  });
  return new Promise((resolve) => server.listen(port, "127.0.0.1", () => resolve({ url: `http://127.0.0.1:${server.address().port}/v1`, close: () => server.close(), requests })));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const m = await startMock(Number(process.argv[2] || 0));
  console.log(m.url);
}
