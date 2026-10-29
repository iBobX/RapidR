// RapidR Studio's MCP server (ide/mcp.inc, RMcpServer) driven by an AI app:
// Studio started headless with AI apps switched on (the test step
// mcp:on — only with RAPIDR_TEST_MCP_TOKEN, which stands in for the
// keychain's token) and a fixture project open; an MCP client — this
// script — works in it under each permission mode:
//
//   Allow everything    the whole tour, nothing asks: discovery, the tools,
//                       reading and the refusals (outside the project, a
//                       hidden folder, a call without its argument), a
//                       change and a new file applied at once, a run and its
//                       output, a breakpoint and the debugger's state, the
//                       layout report, a picture of Studio, RAI asked (the
//                       tests' mock provider) and its steps, a session of an
//                       earlier revision, no token, a web page; the warning
//                       badge; the audit log (never the token or a file's
//                       text)
//   Ask before changes  a change allowed once in the AI Apps pane, a new
//                       file denied, a project outside the Projects folder
//                       denied, a run allowed for the session (the next one
//                       not asked again)
//   Allow edits         a change applied without a question; a run asked
//                       (denied)
//   Read only           reading works; a change and a run are refused, said
//                       in plain words
//
// and a waiting change and Settings > AI Apps are captured.
//
//   node tests/studio_mcp.mjs [all|ask|edits|read|picture|settings …]
//   (RAPIDR: the rapidr to run; ./rapidr)
//
// Never the user's keychain (RAPIDR_AI_KEYSTORE=memory), registry, printer
// or AI keys: everything in tests/results/studio-mcp.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-mcp");
const TOKEN = "studio-mcp-test-token-0123456789abcdef";
const BASE_PORT = Number(process.env.STUDIO_MCP_PORT || 47931);
const MODERN = "2026-07-28";
const CLIENT = "studio-test";
const only = process.argv.slice(2);

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });

const mockProc = spawn(process.execPath, [join(HERE, "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
const MOCK_URL = await new Promise((resolve) => mockProc.stdout.once("data", (d) => resolve(String(d).trim())));
const MOCK_ROOT = MOCK_URL.replace(/\/v1$/, "");
// (whatever happens, the mock and the Studios started here end with the test)
const children = [mockProc];
process.on("exit", () => children.forEach((c) => c.kill()));

function env(name) {
  return {
    ...process.env,
    RAPIDR_CAPTURE: join(WORK, `${name}-window`),
    RAPIDR_CAPTURE_DELAY: "1.5",
    RAPIDR_MENU: "window",
    RAPIDR_TEST_MCP_TOKEN: TOKEN,
    RAPIDR_PRINT_TO: join(WORK, "prints"),
    RAPIDR_REGISTRY: join(WORK, `${name}.reg`),
    RAPIDR_DOCUMENTS: join(WORK, "docs"),
    RAPIDR_AI_KEYSTORE: "memory",
    RAPIDR_AI_TEST_SERVER: MOCK_ROOT,
    RAPIDR_AI_AUDIT: join(WORK, `${name}-audit.jsonl`),
    RAPIDR_AI_CHATS: join(WORK, "chats"),
    RAPIDR_LOGIN_PATH: "off",
    ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "",
  };
}

/// Studio with a copy of the fixture project (`name`'s own), the steps,
/// the events (the "user"), what to print at the end.
function studio(name, steps, events, dump) {
  cpSync(join(ROOT, "tests", "fixtures", "studio_mcp"), join(WORK, name), { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", steps, join(WORK, name, "Hello.rrproj")];
  const child = spawn(RAPIDR, args, { cwd: ROOT, env: { ...env(name), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump }, stdio: ["ignore", "pipe", "pipe"] });
  children.push(child);
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  child.stderr.on("data", (d) => (out += d));
  const done = new Promise((resolve) => child.on("exit", (code) => resolve({ code, out })));
  const ended = async (ms = 150000) => {
    const r = await Promise.race([done, new Promise((res) => setTimeout(() => res(null), ms))]);
    if (!r) child.kill();
    const lines = Object.fromEntries((r?.out || "").split("\n").filter((l) => /^[a-z0-9]+\.[a-z_]+=/.test(l)).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1)]));
    return { ok: !!r, out: r?.out || "", lines };
  };
  return { child, ended };
}

async function listening(port, ms = 60000) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const ok = await new Promise((resolve) => {
      const s = net.connect({ host: "127.0.0.1", port }, () => { s.end(); resolve(true); });
      s.on("error", () => resolve(false));
    });
    if (ok) return true;
    await new Promise((r) => setTimeout(r, 250));
  }
  return false;
}

// ---- the client ---------------------------------------------------------------------
const meta = { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": { name: CLIENT, version: "1.0" }, "io.modelcontextprotocol/clientCapabilities": {} };

/// An MCP client of the Studio on `port`.
function client(port) {
  let nextId = 1;
  /// One POST: { status, headers, json, text, progress } — an event-stream
  /// answer read to its last message.
  async function post(body, headers = {}) {
    const r = await fetch(`http://127.0.0.1:${port}/mcp`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream", Authorization: `Bearer ${TOKEN}`, ...headers },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(180000),
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
    return { status: r.status, headers: r.headers, json, text, progress };
  }
  async function call(method, params = {}) {
    const name = params.name || params.uri;
    return post({ jsonrpc: "2.0", id: nextId++, method, params: { ...params, _meta: meta } }, { "MCP-Protocol-Version": MODERN, "Mcp-Method": method, ...(name ? { "Mcp-Name": name } : {}) });
  }
  async function tool(name, args = {}) {
    try {
      const r = await call("tools/call", { name, arguments: args });
      const res = r.json.result || {};
      return { text: (res.content || []).map((c) => c.text || "").join("\n") || JSON.stringify(r.json.error || {}), error: !!res.isError || !!r.json.error, content: res.content || [], raw: r };
    } catch (e) {
      return { text: `(no answer: ${e.message})`, error: true, content: [], raw: { progress: [] } };
    }
  }
  return { post, call, tool, next: () => nextId++ };
}

let failures = 0;
function check(what, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${ok ? "" : `  — ${String(detail).slice(0, 600)}`}`);
  if (!ok) failures++;
}
const want = (name) => only.length === 0 || only.includes(name);
const until = (what) => `mcpsub.__until_Caption_${CLIENT}: ${what}`;
const shown = (v) => v === "1" || v === "-1";

// ---- Allow everything: the whole tour, nothing asks -------------------------------------
if (want("all")) {
  const PORT = BASE_PORT;
  const AI_READY = `ai:on,ai:provider=compatible,ai:base=${MOCK_URL},ai:model=mock-small`;
  const run = studio("all", [AI_READY, "mcp:mode=all", `mcp:on=${PORT}`].join(","), until("get_rai_audit"), "mcphead.caption,mcplog.itemcount,status.panelcount,mcpask.visible,mcpbadge.visible,mcpmodebox.text");
  check("Studio listens for AI apps", await listening(PORT));
  const { post, call, tool, next } = client(PORT);

  const d = await call("server/discover");
  check("server/discover: the current revision, Studio's name", d.json.result?.supportedVersions?.[0] === MODERN && d.json.result?._meta?.["io.modelcontextprotocol/serverInfo"]?.name === "rapidr-studio", JSON.stringify(d.json));
  const list = (await call("tools/list")).json.result?.tools || [];
  const names = list.map((t) => t.name);
  for (const t of ["studio_status", "list_files", "read_file", "rapidr_describe", "rapidr_list", "rapidr_get", "check_code", "search_manual", "find_example", "project_outline", "propose_edit", "write_file", "create_project", "list_documents", "get_problems", "read_output", "open_project", "set_property", "run_program", "debug", "layout_report", "capture", "ask_rai", "get_rai_steps", "get_rai_transcript", "get_rai_audit"]) {
    check(`tools/list has ${t}`, names.includes(t), names.join(","));
  }
  check("read_file is marked read-only, propose_edit isn't", list.find((t) => t.name === "read_file")?.annotations?.readOnlyHint === true && list.find((t) => t.name === "propose_edit")?.annotations?.readOnlyHint === false);

  let r = await tool("studio_status");
  check("studio_status: the project, the permission", r.text.includes("Project: Hello") && r.text.includes("Permission: Allow everything"), r.text);
  r = await tool("list_files");
  check("list_files: main.rr", r.text.includes("main.rr") && !r.error, r.text);
  r = await tool("read_file", { path: "main.rr" });
  check("read_file main.rr", r.text.includes("Hello from Studio") && !r.error, r.text);
  r = await tool("read_file", { path: "../outside.txt" });
  check("read_file outside the project: refused", r.error && r.text.includes("outside the project"), r.text);
  r = await tool("read_file", { path: ".secret/key.txt" });
  check("read_file in a hidden folder: refused", r.error && r.text.includes("hidden") && !r.text.includes("not for AI apps"), r.text);
  r = await tool("read_file", {});
  check("read_file without its path: told what it needs", r.error && r.text.includes("Call read_file again: it needs `path`"), r.text);
  r = await tool("check_code");
  check("check_code: no errors", !r.error && /no (errors|problems)/i.test(r.text), r.text);
  // (a small model's tool-call layer sends a list as JSON text and a flag as "true": the schema says
  // what is wanted, so the value is taken — found by tools/ai_eval's outside-AI-app runs)
  r = await tool("read_output", { All: "true" });
  check("a flag sent as the string \"true\" is the flag", !r.error && r.text.includes("(no new output)"), r.text);
  r = await tool("propose_edit", { path: "main.rr", edits: JSON.stringify([{ old_text: "this text is not in the file", new_text: "x" }]), summary: "Nothing." });
  check("a list sent as JSON text is the list (the edit is looked at: its text isn't there)", r.error && r.text.includes("isn't in main.rr") && !r.text.includes("must be a list"), r.text);
  r = await tool("propose_edit", { path: "main.rr", edits: '[{"old_text": "a"}]\n}', summary: "Nothing." });
  check("JSON text that isn't a list is still refused, in words", r.error && r.text.includes("`edits` must be a list"), r.text);

  r = await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello from an AI app"' }], summary: "Greets from the AI app." });
  check("propose_edit: applied at once (Allow everything)", !r.error && r.text.includes("Applied") && r.raw.progress.length === 0, r.text);
  r = await tool("write_file", { path: "notes.txt", text: "A note\n", summary: "A notes file." });
  check("write_file: a new file made at once", !r.error && r.text.includes("notes.txt") && existsSync(join(WORK, "all", "notes.txt")), r.text);
  r = await tool("list_documents");
  check("list_documents: main.rr changed, not saved", r.text.includes("main.rr") && r.text.includes("unsaved changes"), r.text);
  // (a new program would open over unsaved changes: Studio would ask its user to save them and answer
  // nothing, to any call, until they did — found by tools/ai_eval's runs: now said at once, as open_project does)
  r = await tool("create_project", { name: "Second", main: "main.rr", files: [{ path: "main.rr", text: 'PRINT "second"\n' }], summary: "A second program." });
  check("create_project over unsaved changes: told at once, nothing opened", r.text.includes("unsaved changes") && !r.text.includes("Created and opened"), r.text);
  r = await tool("studio_status");
  check("Studio still answers, with the same project", r.text.includes("Project: Hello"), r.text);

  r = await tool("run_program", { Action: "start", Wait: 20 });
  check("run_program: run at once, the changed program's output back", !r.error && r.text.includes("Hello from an AI app") && r.text.includes("Line 2"), r.text);
  check("run_program saved the change first (as Run does)", readFileSync(join(WORK, "all", "main.rr"), "utf8").includes("Hello from an AI app"));
  r = await tool("read_output");
  check("read_output: nothing new", r.text.includes("(no new output)") && r.text.includes("isn't running"), r.text);
  r = await tool("get_problems");
  check("get_problems: none", !r.error, r.text);
  r = await tool("open_project", { Path: "examples/../../etc/hosts" });
  check("open_project: only projects and programs, said at once", r.error && r.text.includes("Can't open"), r.text);

  // (the debugger: a breakpoint, a run that stops there, its state, on)
  r = await tool("debug", { Action: "breakpoint", File: "main.rr", Line: 6 });
  check("debug breakpoint: set", !r.error && r.text.includes("main.rr:6"), r.text);
  r = await tool("run_program", { Action: "start", Debug: true, Wait: 20 });
  check("run_program with the debugger: paused at the breakpoint, the variables", r.text.includes("paused") && r.text.includes("main.rr:6") && r.text.includes("Variables:"), r.text);
  r = await tool("debug", { Action: "continue", Wait: 10 });
  check("debug continue: stops again in the loop", r.text.includes("paused") || r.text.includes("isn't running"), r.text);
  r = await tool("debug", { Action: "breakpoint", File: "main.rr", Line: 6, Remove: true });
  check("debug breakpoint removed", r.text.includes("Breakpoints: none"), r.text);
  r = await tool("debug", { Action: "continue", Wait: 10 });
  check("debug continue: the program ends", r.text.includes("isn't running"), r.text);

  r = await tool("layout_report");
  check("layout_report: this program makes no form", r.text.includes("no form"), r.text);
  r = await tool("capture");
  const img = r.content.find((c) => c.type === "image");
  check("capture: a PNG of Studio", !r.error && img?.mimeType === "image/png" && Buffer.from(img.data, "base64").subarray(1, 4).toString() === "PNG", r.text);

  r = await tool("ask_rai", { Prompt: "check the program" });
  check("ask_rai: RAI answered, its steps back", !r.error && r.text.includes("Answer:") && r.text.includes("Steps:"), r.text);
  r = await tool("get_rai_steps");
  check("get_rai_steps: RAI idle", r.text.includes("RAI: idle"), r.text);
  r = await tool("get_rai_transcript");
  check("get_rai_transcript: the question in the chat", r.text.includes("check the program"), r.text);

  // (an earlier revision's session, beside)
  const init = await post({ jsonrpc: "2.0", id: next(), method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "studio-legacy", version: "0.1" } } });
  const sid = init.headers.get("mcp-session-id") || "";
  check("initialize (2025-06-18): a session", init.json.result?.protocolVersion === "2025-06-18" && sid.length === 32, JSON.stringify(init.json));
  const legacy = await post({ jsonrpc: "2.0", id: next(), method: "tools/call", params: { name: "studio_status", arguments: {} } }, { "Mcp-Session-Id": sid, "MCP-Protocol-Version": "2025-06-18" });
  check("a session's tools/call", (legacy.json.result?.content || [])[0]?.text?.includes("RapidR Studio"), JSON.stringify(legacy.json).slice(0, 300));
  const ended = await fetch(`http://127.0.0.1:${PORT}/mcp`, { method: "DELETE", headers: { Authorization: `Bearer ${TOKEN}`, "Mcp-Session-Id": sid } });
  check("DELETE ends the session", ended.status === 200);
  const noToken = await fetch(`http://127.0.0.1:${PORT}/mcp`, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
  check("no token: 401", noToken.status === 401);
  const page = await post({ jsonrpc: "2.0", id: 99, method: "ping" }, { Origin: "https://evil.example" });
  check("a web page's request: 403", page.status === 403);

  r = await tool("get_rai_audit", { Count: 5 });
  check("get_rai_audit (the last call)", !r.error, r.text);
  const end = await run.ended();
  check("Studio ended after the capture", end.ok);
  check("the pane says who is connected", (end.lines["mcphead.caption"] || "").startsWith(`AI apps: ${CLIENT}`), JSON.stringify(end.lines));
  check("the pane lists what happened", Number(end.lines["mcplog.itemcount"]) >= 25, end.lines["mcplog.itemcount"]);
  check("the status bar has AI apps' panel", end.lines["status.panelcount"] === "7", end.lines["status.panelcount"]);
  check("Allow everything: the warning badge shows, the menu says it", shown(end.lines["mcpbadge.visible"]) && end.lines["mcpmodebox.text"] === "Allow everything", JSON.stringify(end.lines));
  const audit = existsSync(join(WORK, "all-audit.jsonl")) ? readFileSync(join(WORK, "all-audit.jsonl"), "utf8") : "";
  check("the audit log has the AI app's calls and the mode", audit.split("\n").filter((l) => l.includes('"source":"mcp"')).length >= 25 && audit.includes("Allow everything"), audit.slice(0, 400));
  check("the audit log never has the token, nor a file's text", !audit.includes(TOKEN) && !audit.includes("Hello from an AI app"), "");
  if (existsSync(join(WORK, "all-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "all-window-1.bmp"), "--out", join(WORK, "studio-mcp-allow-everything.png")], { stdio: "ignore" });
  check("Studio's window captured with the AI Apps pane", existsSync(join(WORK, "studio-mcp-allow-everything.png")));
}

// ---- Ask before changes (the default): the user decides in the pane -------------------
if (want("ask")) {
  const PORT = BASE_PORT + 1;
  // (Allow Once on the change, Deny on the new file and on the project
  // outside the Projects folder, Allow for This Session on the run)
  const events = ["mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", "mcpask.__until_Visible_1", "mcpsessionbtn.onclick", until("read_output")].join(",");
  const run = studio("ask", `mcp:on=${PORT}`, events, "mcpask.visible,mcpbadge.visible,mcpmodebox.text");
  check("Ask before changes: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("studio_status");
  check("studio_status: Ask before changes, the default", r.text.includes("Permission: Ask before changes"), r.text);
  r = await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello from an AI app"' }], summary: "Greets from the AI app." });
  check("propose_edit: allowed once by the user, applied in the editor", !r.error && r.text.includes("Applied"), r.text);
  check("propose_edit: the AI app heard it waits for the user", r.raw.progress.some((p) => JSON.stringify(p).includes("Waiting for the user")), JSON.stringify(r.raw.progress));
  r = await tool("write_file", { path: "notes.txt", text: "A note\n", summary: "A notes file." });
  check("write_file: denied by the user", r.error && r.text.includes("didn't allow") && !existsSync(join(WORK, "ask", "notes.txt")), r.text);
  cpSync(join(ROOT, "tests", "fixtures", "studio_mcp"), join(WORK, "ask-other"), { recursive: true });
  r = await tool("open_project", { Path: join(WORK, "ask-other", "Hello.rrproj") });
  check("open_project outside the Projects folder: asked, denied", r.error && r.text.includes("didn't allow"), r.text);
  r = await tool("run_program", { Action: "start", Wait: 20 });
  check("run_program: allowed for the session, it ran", !r.error && r.text.includes("Hello from an AI app"), r.text);
  r = await tool("run_program", { Action: "start", Wait: 20 });
  check("run_program again: not asked again this session", !r.error && r.text.includes("Line 2") && r.raw.progress.length === 0, r.text);
  r = await tool("read_output");
  const end = await run.ended();
  check("Ask before changes: nothing left waiting, no badge", end.lines["mcpask.visible"] === "0" && !shown(end.lines["mcpbadge.visible"]) && end.lines["mcpmodebox.text"] === "Ask before changes", JSON.stringify(end.lines));
  const audit = existsSync(join(WORK, "ask-audit.jsonl")) ? readFileSync(join(WORK, "ask-audit.jsonl"), "utf8") : "";
  check("the decisions are in the audit log", audit.includes("denied by you") && audit.includes("applied (you allowed it)"), audit.slice(0, 300));
}

// ---- Allow edits: changes at once, a run asks -------------------------------------------
if (want("edits")) {
  const PORT = BASE_PORT + 2;
  const run = studio("edits", `mcp:mode=edits,mcp:on=${PORT}`, ["mcpask.__until_Visible_1", "mcpdenybtn.onclick", until("read_output")].join(","), "mcpbadge.visible,mcpmodebox.text");
  check("Allow edits: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hi"' }], summary: "Says hi." });
  check("Allow edits: propose_edit applied without a question", !r.error && r.text.includes("Applied") && r.raw.progress.length === 0, r.text);
  r = await tool("run_program", { Action: "start" });
  check("Allow edits: run_program still asks (denied here)", r.error && r.text.includes("didn't allow"), r.text);
  r = await tool("read_output");
  const end = await run.ended();
  check("Allow edits: the menu says it, no badge", end.lines["mcpmodebox.text"] === "Allow edits" && !shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
}

// ---- Read only: looking only -----------------------------------------------------------
if (want("read")) {
  const PORT = BASE_PORT + 3;
  const run = studio("read", `mcp:mode=read,mcp:on=${PORT}`, until("read_output"), "mcpmodebox.text");
  check("Read only: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("read_file", { path: "main.rr" });
  check("Read only: read_file works", !r.error && r.text.includes("Hello from Studio"), r.text);
  r = await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hi"' }], summary: "Says hi." });
  check("Read only: a change is refused, in plain words", r.error && r.text.includes("Read only") && r.text.includes("only look"), r.text);
  r = await tool("run_program", { Action: "start" });
  check("Read only: a run is refused", r.error && r.text.includes("Read only"), r.text);
  check("Read only: the file is as it was", readFileSync(join(WORK, "read", "main.rr"), "utf8").includes("Hello from Studio"));
  r = await tool("read_output");
  const end = await run.ended();
  check("Read only: the menu says it", end.lines["mcpmodebox.text"] === "Read only", JSON.stringify(end.lines));
}

// ---- a change waiting for the user (the picture of the pane) ----------------------------
if (want("picture")) {
  const PORT = BASE_PORT + 4;
  const run = studio("picture", `mcp:on=${PORT}`, "mcpask.__until_Visible_1,studio.__wait_600", "mcpask.visible,mcpasktext.caption");
  check("Studio listens again", await listening(PORT));
  const { tool } = client(PORT);
  await tool("studio_status");
  await tool("read_file", { path: "main.rr" });
  await tool("check_code");
  // (it waits for the user: Studio's window is captured meanwhile, and ends)
  await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello, world"' }], summary: "Says hello to the world." });
  const end = await run.ended(90000);
  check("the pane asks the user about the change", (end.lines["mcpasktext.caption"] || "") === "studio-test wants to change main.rr: Says hello to the world.", JSON.stringify(end.lines));
  if (existsSync(join(WORK, "picture-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "picture-window-1.bmp"), "--out", join(WORK, "studio-mcp-ask.png")], { stdio: "ignore" });
}

// ---- Allow everything's badge (the picture: the pane in front, its list) ----------------
if (want("badge")) {
  const PORT = BASE_PORT + 6;
  const run = studio("badge", `mcp:mode=all,mcp:on=${PORT}`, until("Checking (compiling)"), "mcpbadge.__shown");
  check("Allow everything (picture): Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  await tool("studio_status");
  await tool("read_file", { path: "main.rr" });
  await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello, world"' }], summary: "Says hello to the world." });
  await tool("check_code");
  const end = await run.ended(90000);
  check("Allow everything: the badge shows in the pane in front", end.lines["mcpbadge.__shown"] === "1", JSON.stringify(end.lines));
  if (existsSync(join(WORK, "badge-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "badge-window-1.bmp"), "--out", join(WORK, "studio-mcp-allow-everything-pane.png")], { stdio: "ignore" });
}

// ---- Settings > AI Apps ---------------------------------------------------------------
if (want("settings")) {
  const PORT = BASE_PORT + 5;
  const run = studio("settings", `mcp:on=${PORT},wait,mcp:settings`, "", "mcponbox.checked,mcpaddredit.text,mcpheaderedit.passwordchar,mcppagemodebox.text");
  const end = await run.ended(90000);
  check("Settings > AI Apps: on, the address, the token masked, the project's mode", /mcponbox\.checked=(1|-1)/.test(end.out) && end.out.includes(`mcpaddredit.text=http://127.0.0.1:${PORT}/mcp`) && end.out.includes("mcpheaderedit.passwordchar=*") && end.out.includes("mcppagemodebox.text=Ask before changes"), end.out.split("\n").filter((l) => l.startsWith("mcp")).join(" | "));
  for (const n of [1, 2]) {
    const bmp = join(WORK, `settings-window-${n}.bmp`);
    if (existsSync(bmp)) spawnSync("sips", ["-s", "format", "png", bmp, "--out", join(WORK, `studio-mcp-settings-${n}.png`)], { stdio: "ignore" });
  }
  check("Settings captured", existsSync(join(WORK, "studio-mcp-settings-2.png")));
}

console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
