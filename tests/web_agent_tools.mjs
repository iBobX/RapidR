// RMcpServer's browser transport (crates/rapidr-runtime-web/src/agent_tools_web.rs
// over the shared core, crates/rapidr-value/src/agent_tools): a program's
// tools offered to the browser's AI agents through the browser API of the
// WebMCP draft, with `document.modelContext` mocked as the draft (2026-11-02)
// describes it — registerTool(tool, { signal }), abort to take a tool away,
// execute(input, { signal }) — and called as an agent would:
//
//   - the tools registered on Start, with their schemas and the browser's
//     annotations (readOnlyHint, untrustedContentHint, consequentialHint);
//   - a call answered (text), a failure as { error } data, a wrong call told
//     what the tool needs (before the program sees it), a call answered later
//     (a timer), a picture-less long answer cut at MaxResultSize, a key in an
//     answer masked, the input as a JSON text (the draft before 2026-08);
//   - every call from "browser agent" (ClientName, Clients);
//   - a call the agent aborts: rejected (AbortError), the program's
//     OnToolCancel hears it, IsOpen is False;
//   - AddTool / RemoveTool after Start: the browser's list follows;
//   - Stop: every tool taken away, a waiting call told (as data);
//   - navigator.modelContext (Chromium 149's place) works too;
//   - a browser without it: Available = 0, Start fails and says why, the
//     program goes on.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_agent_tools.mjs

import { chromium } from "playwright";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg, extra = "") => {
  console.log(`${cond ? "✓" : "✗"} ${msg}${cond || !extra ? "" : `\n    ${extra}`}`);
  if (!cond) failed++;
};

/// The draft's model context, as a page sees it (no agent: the test is the
/// agent). `where`: "document" (the draft now), "navigator" (Chromium 149).
function mockModelContext(where) {
  const tools = new Map();
  const log = [];
  const mc = {
    registerTool(tool, options = {}) {
      const signal = options.signal;
      if (signal && signal.aborted) return Promise.reject(new DOMException("The signal is aborted.", "AbortError"));
      if (!tool || typeof tool.name !== "string" || typeof tool.description !== "string" || typeof tool.execute !== "function") return Promise.reject(new TypeError("Not a tool."));
      if (tools.has(tool.name)) return Promise.reject(new DOMException(`A tool named ${tool.name} is registered already.`, "InvalidStateError"));
      tools.set(tool.name, tool);
      log.push(`register ${tool.name}`);
      if (signal) signal.addEventListener("abort", () => { tools.delete(tool.name); log.push(`unregister ${tool.name}`); });
      return Promise.resolve();
    },
    async getTools() {
      return [...tools.values()].map((t) => ({ name: t.name, title: t.title, description: t.description, inputSchema: t.inputSchema, annotations: t.annotations }));
    },
    async executeTool(name, input, options = {}) {
      const t = tools.get(name);
      if (!t) throw new DOMException(`No tool named ${name}.`, "NotFoundError");
      const result = await t.execute(input, { signal: options.signal });
      return JSON.stringify(result);
    },
  };
  window.__mcLog = log;
  if (where === "document") Object.defineProperty(Document.prototype, "modelContext", { get() { return mc; }, configurable: true });
  if (where === "navigator") Object.defineProperty(Navigator.prototype, "modelContext", { get() { return mc; }, configurable: true });
}

const PROGRAM = `
DIM Mcp AS RMcpServer
DIM T AS RTimer
DIM Later AS INTEGER
SUB ToolCalled (Tool AS STRING, Arguments AS STRING, CallId AS INTEGER)
    PRINT "call "; Tool; " "; Arguments; " from "; Mcp.ClientName
    SELECT CASE Tool
    CASE "add"
        Mcp.Reply CallId, STR$(VAL(Mcp.Argument("A")) + VAL(Mcp.Argument("B")))
    CASE "fail"
        Mcp.Reply CallId, "That went wrong: call add instead.", 1
    CASE "later"
        Later = CallId
        T.Enabled = 1
    CASE "wait"
        PRINT "waiting "; CallId
    CASE "secret"
        ' (a made-up key, written in two pieces: no key's shape in the repo)
        Mcp.Reply CallId, "the key is sk" + "-ant-api03-abcdefghijklmnopqrstuvwxyz0123"
    CASE "long"
        Mcp.Reply CallId, STRING$(4000, "x")
    CASE "grow"
        Mcp.AddTool "extra", "An extra tool.", "", "read"
        Mcp.RemoveTool "secret"
        Mcp.Reply CallId, "grown"
    CASE "extra"
        Mcp.Reply CallId, "extra here"
    CASE "stop"
        Mcp.Reply CallId, "stopping"
        Mcp.Stop
        PRINT "stopped; listening "; Mcp.Listening
    END SELECT
END SUB
SUB Cancelled (CallId AS INTEGER)
    PRINT "cancelled "; CallId; " open "; Mcp.IsOpen(CallId)
END SUB
SUB Tick
    T.Enabled = 0
    IF Mcp.IsOpen(Later) THEN Mcp.Reply Later, "answered later"
END SUB
T.Interval = 150
T.Enabled = 0
T.OnTimer = Tick
Mcp.OnToolCall = ToolCalled
Mcp.OnToolCancel = Cancelled
Mcp.MaxResultSize = 1000
Mcp.AddTool "add", "Adds two numbers.", "A AS DOUBLE, B AS DOUBLE", "read"
Mcp.AddTool "fail", "Always fails.", "", "change"
Mcp.AddTool "later", "Answers a moment later.", "", "read untrusted"
Mcp.AddTool "wait", "Waits until cancelled.", "", "change consequential"
Mcp.AddTool "secret", "Says a key.", "", "read"
Mcp.AddTool "long", "Says a lot.", "", "read"
Mcp.AddTool "grow", "Adds a tool.", "", "change"
Mcp.AddTool "stop", "Stops offering the tools.", "", "change"
PRINT "available "; Mcp.Available; " transport "; Mcp.Transport
IF Mcp.Start THEN
    PRINT "started "; Mcp.Listening
ELSE
    PRINT "not started: "; Mcp.Error
END IF
' (the program waits in its window, as a RapidR program does)
DIM F AS RForm
F.Caption = "Agent tools"
F.ShowModal
`;

const browser = await chromium.launch();

async function runner(where) {
  const context = await browser.newContext();
  if (where) await context.addInitScript(mockModelContext, where);
  const r = await openRunner(context);
  await r.run(PROGRAM, { name: "agent_tools" });
  return r;
}

const agent = async (r, fn, arg) => {
  try {
    return await r.page.evaluate(fn, arg);
  } catch (e) {
    console.log(`✗ the page failed: ${e.message}\n--- output\n${r.output()}\n--- errors\n${r.errors()}`);
    process.exit(1);
  }
};
const call = (r, name, input) => agent(r, async ({ name, input }) => {
  try {
    return { value: JSON.parse(await document.modelContext.executeTool(name, input)) };
  } catch (e) {
    return { threw: e.name, message: e.message };
  }
}, { name, input });

// ---- 1. the draft's place: document.modelContext
{
  const r = await runner("document");
  ok(await r.waitOutput("started 1"), "Start registers the tools (Listening 1)", r.output() + r.errors());
  ok(r.output().includes("available 1 transport browser"), "Available 1, Transport browser", r.output());
  const tools = await agent(r, () => document.modelContext.getTools());
  ok(tools.map((t) => t.name).join(",") === "add,fail,later,wait,secret,long,grow,stop", "every tool registered, in order", JSON.stringify(tools.map((t) => t.name)));
  const add = tools.find((t) => t.name === "add");
  ok(add && add.description === "Adds two numbers." && JSON.stringify(add.inputSchema.required) === '["A","B"]' && add.inputSchema.properties.A.type === "number", "a tool's description and schema", JSON.stringify(add));
  ok(JSON.stringify(add.annotations) === '{"readOnlyHint":true,"untrustedContentHint":false,"consequentialHint":false}', "read: readOnlyHint", JSON.stringify(add.annotations));
  const later = tools.find((t) => t.name === "later"), wait = tools.find((t) => t.name === "wait"), fail = tools.find((t) => t.name === "fail");
  ok(later.annotations.untrustedContentHint === true && later.annotations.readOnlyHint === true, "read untrusted: untrustedContentHint", JSON.stringify(later.annotations));
  ok(fail.annotations.consequentialHint === true && wait.annotations.consequentialHint === true, "change: consequentialHint", JSON.stringify([fail.annotations, wait.annotations]));

  let res = await call(r, "add", { A: 2, B: 40 });
  ok(res.value === "42", "a call answered with text", JSON.stringify(res));
  ok(r.output().includes('call add {"A":2,"B":40} from browser agent'), "OnToolCall: the tool, its arguments, ClientName browser agent", r.output());
  res = await call(r, "add", '{"A": 1, "B": "2"}');
  ok(res.value === "3", "the input as a JSON text (the draft before 2026-08), a number sent as text", JSON.stringify(res));
  res = await call(r, "add", { A: 1 });
  ok(res.value && typeof res.value.error === "string" && res.value.error.startsWith("Call add again: it needs `B`"), "a wrong call: told what the tool needs, as data", JSON.stringify(res));
  ok(!r.output().includes('call add {"A":1}'), "a wrong call never reaches the program", r.output());
  res = await call(r, "fail", {});
  ok(res.value && res.value.error === "That went wrong: call add instead.", "a failure is { error } data, not an exception", JSON.stringify(res));
  res = await call(r, "later", {});
  ok(res.value === "answered later", "a call answered later (by a timer)", JSON.stringify(res));
  res = await call(r, "secret", {});
  ok(res.value === "the key is (an API key, not shown)", "a key in an answer is masked", JSON.stringify(res));
  res = await call(r, "long", {});
  ok(typeof res.value === "string" && [...res.value].length <= 1000 && /\(\d+ more characters not shown: ask for a smaller part\)$/.test(res.value), "a long answer cut at MaxResultSize, saying so", JSON.stringify(res).slice(0, 120));
  const clients = await r.prop("Mcp", "Clients");
  ok(/^browser agent\t\t\d+\t\d+$/.test(clients), "Clients: one browser agent", JSON.stringify(clients));

  // (the agent gives up)
  const aborted = await agent(r, async () => {
    const c = new AbortController();
    const p = document.modelContext.executeTool("wait", {}, { signal: c.signal });
    await new Promise((res) => setTimeout(res, 300));
    c.abort();
    try {
      await p;
      return "resolved";
    } catch (e) {
      return e.name;
    }
  });
  ok(aborted === "AbortError", "an aborted call is rejected with its signal's reason", aborted);
  ok(await r.waitOutput(/cancelled \d+ open 0/), "OnToolCancel hears it; IsOpen is False", r.output());
  ok((await r.prop("Mcp", "PendingCount")) === "0", "nothing waits after it", await r.prop("Mcp", "PendingCount"));

  // (the tool list changes while offered)
  res = await call(r, "grow", {});
  ok(res.value === "grown", "a handler adds and removes tools", JSON.stringify(res));
  await r.page.waitForTimeout(100);
  const names = (await agent(r, () => document.modelContext.getTools())).map((t) => t.name);
  ok(names.includes("extra") && !names.includes("secret"), "the browser's list follows AddTool / RemoveTool", JSON.stringify(names));
  res = await call(r, "extra", {});
  ok(res.value === "extra here", "the new tool is called", JSON.stringify(res));

  // (a call waiting when the tools go: told, as data)
  const waiting = agent(r, () => document.modelContext.executeTool("wait", {}).then((v) => JSON.parse(v), (e) => "threw " + e.name));
  ok(await r.until(() => (r.output().match(/waiting \d+/g) || []).length >= 2), "a second call waits", r.output());
  res = await call(r, "stop", {});
  ok(res.value === "stopping", "a handler stops the server", JSON.stringify(res));
  const told = await waiting;
  ok(told && typeof told.error === "string" && told.error.includes("stopped offering its tools"), "a call still waiting is told the tools went", JSON.stringify(told));
  ok(await r.waitOutput("stopped; listening 0"), "Listening 0 after Stop", r.output());
  const left = await agent(r, () => document.modelContext.getTools());
  const log = await agent(r, () => window.__mcLog);
  ok(left.length === 0 && log.filter((l) => l.startsWith("unregister")).length >= 8, "Stop takes every tool away (their signals aborted)", JSON.stringify({ left, log }));
  ok(!r.errors() || !/panicked|RefCell/.test(r.errors()), "no runtime error", r.errors());
  await r.page.context().close();
}

// ---- 2. Chromium 149's place: navigator.modelContext
{
  const r = await runner("navigator");
  ok(await r.waitOutput("started 1"), "navigator.modelContext: started", r.output() + r.errors());
  const res = await r.page.evaluate(async () => JSON.parse(await navigator.modelContext.executeTool("add", { A: 1, B: 1 })));
  ok(res === "2", "navigator.modelContext: a call answered", JSON.stringify(res));
  await r.page.context().close();
}

// ---- 3. a browser without it
{
  const r = await runner(null);
  ok(await r.waitOutput("not started:"), "no model context: Start fails", r.output() + r.errors());
  ok(r.output().includes("available 0 transport browser"), "Available 0", r.output());
  ok(r.output().includes("not started: This browser doesn't give web pages a way to offer tools to AI agents."), "Error says why, in plain words", r.output());
  ok((await r.prop("Mcp", "Listening")) === "0", "Listening 0", await r.prop("Mcp", "Listening"));
  ok(!/panicked|RefCell/.test(r.errors()), "the program goes on, no runtime error", r.errors());
  await r.page.context().close();
}

await browser.close();
console.log(failed ? `\n${failed} check(s) failed` : "\nall checks passed");
process.exit(failed ? 1 : 0);
