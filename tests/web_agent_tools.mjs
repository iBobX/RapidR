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
//   - RapidR Studio for the web (STUDIO_WEB_URL set: tools/build_studio_web.sh,
//     served): its tools listed, the first call asks once ("Allow for This
//     Tab", clicked), a file read (in parts too), a file written, the program
//     run and its output read, each in the AI Apps pane.
//
// RAPIDR_WEB_CHROME=<a Chromium that has the draft's API>: the same calls in
// a real browser, the API turned on by its own flag
// (--enable-features=WebMCP,WebMCPTesting) in a throwaway profile, instead of
// the mock (sections 1, 4 and 5; 3 in a browser without the flag). Never the
// user's own browser or its profile.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_agent_tools.mjs

import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { openRunner } from "./web_run.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

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
      return [...tools.values()].map((t) => ({ name: t.name, title: t.title, description: t.description, inputSchema: t.inputSchema, annotations: t.annotations })).sort((a, b) => a.name.localeCompare(b.name));
    },
    async executeTool(tool, input, options = {}) {
      const name = typeof tool === "string" ? tool : tool && tool.name;
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

const REAL = process.env.RAPIDR_WEB_CHROME || "";
const STUDIO = (process.env.STUDIO_WEB_URL || "").replace(/\/+$/, "");
const browser = await chromium.launch();
// (the real API: the browser's own flag, a throwaway profile)
const real = REAL ? await chromium.launch({ executablePath: REAL, args: ["--enable-features=WebMCP,WebMCPTesting"] }) : null;
if (real) console.log(`(a real browser: ${REAL.split("/").slice(-4, -3)[0] || REAL}, ${await real.version()})`);

async function runner(where) {
  const context = await (real && where === "document" ? real : browser).newContext();
  if (where && !(real && where === "document")) await context.addInitScript(mockModelContext, where);
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
    const tool = (await document.modelContext.getTools()).find((t) => t.name === name);
    const out = await document.modelContext.executeTool(tool || name, input);
    // (an object comes back as its JSON; a text as it is — Chrome 156 — or as
    // the JSON of a string — the draft's words)
    if (typeof out !== "string") return { value: out };
    try {
      const v = JSON.parse(out);
      return { value: typeof v === "object" || typeof v === "string" ? v : out };
    } catch {
      return { value: out };
    }
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
  ok(tools.map((t) => t.name).sort().join(",") === "add,fail,grow,later,long,secret,stop,wait", "every tool registered", JSON.stringify(tools.map((t) => t.name)));
  const add = tools.find((t) => t.name === "add");
  ok(add && add.description === "Adds two numbers." && JSON.stringify(add.inputSchema.required) === '["A","B"]' && add.inputSchema.properties.A.type === "number", "a tool's description and schema", JSON.stringify(add));
  ok(add.annotations.readOnlyHint === true && add.annotations.untrustedContentHint === false && add.annotations.consequentialHint === false, "read: readOnlyHint", JSON.stringify(add.annotations));
  const later = tools.find((t) => t.name === "later"), wait = tools.find((t) => t.name === "wait"), fail = tools.find((t) => t.name === "fail");
  ok(later.annotations.untrustedContentHint === true && later.annotations.readOnlyHint === true, "read untrusted: untrustedContentHint", JSON.stringify(later.annotations));
  ok(fail.annotations.consequentialHint === true && wait.annotations.consequentialHint === true, "change: consequentialHint", JSON.stringify([fail.annotations, wait.annotations]));

  let res = await call(r, "add", { A: 2, B: 40 });
  ok(res.value === "42", "a call answered with text", JSON.stringify(res));
  ok(r.output().includes('call add {"A":2,"B":40} from browser agent'), "OnToolCall: the tool, its arguments, ClientName browser agent", r.output());
  // (the draft before 2026-08 sent the input as a JSON text: the mock only —
  // Chrome 156 refuses it itself)
  res = await call(r, "add", real ? { A: 1, B: "2" } : '{"A": 1, "B": "2"}');
  ok(res.value === "3", "the input as a JSON text (the mock), a number sent as text", JSON.stringify(res));
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
    const tool = (await document.modelContext.getTools()).find((t) => t.name === "wait");
    const p = document.modelContext.executeTool(tool, {}, { signal: c.signal });
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
  const waiting = agent(r, async () => {
    const tool = (await document.modelContext.getTools()).find((t) => t.name === "wait");
    return document.modelContext.executeTool(tool, {}).then((v) => JSON.parse(v), (e) => "threw " + e.name);
  });
  ok(await r.until(() => (r.output().match(/waiting \d+/g) || []).length >= 2), "a second call waits", r.output());
  res = await call(r, "stop", {});
  ok(res.value === "stopping", "a handler stops the server", JSON.stringify(res));
  const told = await waiting;
  ok(told && typeof told.error === "string" && told.error.includes("stopped offering its tools"), "a call still waiting is told the tools went", JSON.stringify(told));
  ok(await r.waitOutput("stopped; listening 0"), "Listening 0 after Stop", r.output());
  const left = await agent(r, () => document.modelContext.getTools());
  const log = await agent(r, () => window.__mcLog || []);
  ok(left.length === 0 && (real || log.filter((l) => l.startsWith("unregister")).length >= 8), "Stop takes every tool away (their signals aborted)", JSON.stringify({ left, log }));
  ok(!r.errors() || !/panicked|RefCell/.test(r.errors()), "no runtime error", r.errors());
  await r.page.context().close();
}

// ---- 2. Chromium 149's place: navigator.modelContext (the mock only)
if (!real) {
  const r = await runner("navigator");
  ok(await r.waitOutput("started 1"), "navigator.modelContext: started", r.output() + r.errors());
  const res = await r.page.evaluate(async () => JSON.parse(await navigator.modelContext.executeTool((await navigator.modelContext.getTools()).find((t) => t.name === "add"), { A: 1, B: 1 })));
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

// ---- 4. the example web app (examples/web/agent_tools.rr): its own tools,
// a change asked first (the browser's question, answered here), Expose
{
  const context = await (real || browser).newContext();
  if (!real) await context.addInitScript(mockModelContext, "document");
  const r = await openRunner(context);
  const asked = [];
  let answer = false;
  r.page.on("dialog", (d) => { asked.push(d.message()); answer ? d.accept() : d.dismiss(); });
  await r.run(readFileSync(join(ROOT, "examples/web/agent_tools.rr"), "utf8"), { name: "agent_tools" });
  ok(await r.until(async () => (await r.prop("Status", "Caption")).startsWith("AI agents in this browser")), "the example offers its tools", await r.prop("Status", "Caption"));
  const names = (await agent(r, () => document.modelContext.getTools())).map((t) => t.name).sort().join(",");
  ok(names === "add_note,app_components,app_get,list_notes", "its tools and Expose's read ones (no app_set)", names);
  let res = await call(r, "list_notes", {});
  ok(res.value === "(no notes yet)", "list_notes", JSON.stringify(res));
  res = await call(r, "add_note", { Text: "Buy milk" });
  ok(res.value && res.value.error && res.value.error.startsWith("The person using the app didn't allow it") && asked.length === 1 && asked[0].includes("add_note"), "a change asks first; no: refused", JSON.stringify({ res, asked }));
  answer = true;
  res = await call(r, "add_note", { Text: "Buy milk" });
  ok(res.value === "Added: Buy milk" && asked.length === 2, "yes: done", JSON.stringify({ res, asked }));
  res = await call(r, "list_notes", {});
  ok(res.value === "Buy milk\n", "the note is there", JSON.stringify(res));
  res = await call(r, "app_components", {});
  ok(typeof res.value === "string" && res.value.toLowerCase().includes("notes (listbox)") && res.value.toLowerCase().includes("status (label)"), "app_components lists the window's components", JSON.stringify(res));
  res = await call(r, "app_get", { name: "Status", property: "Caption" });
  ok(res.value === "AI agents in this browser can read and add notes.", "app_get reads one", JSON.stringify(res));
  ok(asked.length === 2, "reading never asks", JSON.stringify(asked));
  await context.close();
}

// ---- 5. RapidR Studio for the web, used as an agent uses it
if (STUDIO) {
  const context = await (real || browser).newContext({ viewport: { width: 1440, height: 900 } });
  if (!real) await context.addInitScript(mockModelContext, "document");
  const files = ["tests/fixtures/studio_mcp/Hello.rrproj", "tests/fixtures/studio_mcp/main.rr"].map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
  await context.addInitScript((files) => {
    window.RAPIDR_STUDIO_TEST = { RAPIDR_TEST_MCP_TOKEN: "web-agent-test-token-0123456789" };
    // (ENVIRON$ on the web: the page's own table — Studio's test steps read it)
    window.RAPIDR_TEST_ENV = { RAPIDR_TEST_MCP_TOKEN: "web-agent-test-token-0123456789" };
    window.RAPIDR_STUDIO_TEST_FILES = files;
    window.RAPIDR_FILE_SINK = () => {};
  }, files);
  const page = await context.newPage();
  const errs = [];
  page.on("pageerror", (e) => errs.push(e.message));
  const q = new URLSearchParams({ fresh: "", window: "normal", do: "mcp:mode=edits,mcp:on", open: "tests/fixtures/studio_mcp/Hello.rrproj" });
  await page.goto(`${STUDIO}/index.html?${q}`, { waitUntil: "load" });
  const s = { page, output: () => "", errors: () => errs.join("\n") };
  let tools = [];
  for (let i = 0; i < 120 && tools.length < 30; i++) {
    await page.waitForTimeout(500);
    tools = await page.evaluate(() => (document.modelContext ? document.modelContext.getTools() : [])).catch(() => []);
  }
  const names = tools.map((t) => t.name);
  ok(names.length >= 30 && ["studio_status", "read_file", "write_file", "check_code", "run_program", "read_output"].every((n) => names.includes(n)), `web Studio offers its tools (${names.length})`, JSON.stringify(names));
  const rf = tools.find((t) => t.name === "read_file");
  ok(rf && rf.annotations && rf.annotations.readOnlyHint === true && rf.annotations.untrustedContentHint === true && rf.inputSchema.properties.offset, "read_file: read only, untrusted content, offset / limit", JSON.stringify(rf && { a: rf.annotations, p: Object.keys(rf.inputSchema.properties) }));
  const click = async (label) => {
    for (let i = 0; i < 40; i++) {
      const b = page.getByRole("button", { name: label, exact: true });
      if (await b.count()) {
        const box = await b.first().boundingBox();
        if (box) {
          await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
          return true;
        }
      }
      await page.waitForTimeout(250);
    }
    return false;
  };
  // (the first call: asked once, in the pane)
  const first = call(s, "studio_status", {});
  ok(await click("Allow for This Tab"), "the first call asks: Allow for This Tab (clicked)");
  let res = await first;
  ok(typeof res.value === "string" && res.value.includes("main.rr"), "studio_status answered once allowed", JSON.stringify(res).slice(0, 300));
  res = await call(s, "read_file", { path: "main.rr" });
  // (its text marked as data: the untrusted wrapper's JSON)
  ok(JSON.stringify(res.value || "").includes("Hello from Studio") && JSON.stringify(res.value).includes("untrusted"), "read_file: the file's text, marked as data", JSON.stringify(res).slice(0, 200));
  res = await call(s, "read_file", { path: "main.rr", offset: 1, limit: 2 });
  ok(typeof res.value === "string" && res.value.includes("more: call read_file again with offset 3"), "read_file in parts: where the next part starts", JSON.stringify(res).slice(0, 300));
  res = await call(s, "write_file", { path: "notes.txt", text: "A note from an agent\n", summary: "A notes file." });
  ok(typeof res.value === "string" && res.value.includes("notes.txt"), "write_file (Allow edits): made at once", JSON.stringify(res).slice(0, 200));
  const run = call(s, "run_program", { wait: 10 });
  ok(await click("Allow Once"), "run_program asks (Allow edits): Allow Once (clicked)");
  res = await run;
  ok(typeof res.value === "string" && res.value.includes("Hello from Studio"), "run_program: the program ran, its output back", JSON.stringify(res).slice(0, 300));
  res = await call(s, "read_output", {});
  // (run_program gave its output already: read_output has nothing new)
  ok(typeof res.value === "string" && /no new output|Line/.test(res.value), "read_output: answered", JSON.stringify(res).slice(0, 200));
  // (the AI Apps pane's list, as Studio has it)
  // (the pane's head and the status bar name who called and what it did last)
  const shown = await page.evaluate(() => ["mcphead.caption", "mcpsub.caption", "mcp.clients"].map((k) => window.rr.rapidr_get_prop(...k.split("."))).join("\n"));
  ok(/AI apps: browser agent/.test(shown) && /browser agent\t\t\d+\t[5-9]/.test(shown), "the AI Apps pane: browser agent, its calls counted", JSON.stringify(shown).slice(0, 400));
  ok(!/panicked|RefCell/.test(errs.join("\n")), "no runtime error", errs.join("\n"));
  await context.close();
}

if (real) await real.close();
await browser.close();
console.log(failed ? `\n${failed} check(s) failed` : "\nall checks passed");
process.exit(failed ? 1 : 0);
