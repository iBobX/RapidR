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
//   Carried mode        a project an AI app creates or opens takes the mode
//                       of the one open before (Allow edits, Ask), said in
//                       the pane — never Allow everything (Allow edits
//                       instead, said so); a project with its own mode
//                       keeps it; Read only can't be left; nothing open
//                       before: Ask before changes
//   Read only           reading works; a change and a run are refused, said
//                       in plain words
//
// and a waiting change and Settings > AI Apps are captured; the
// first-time question (turning the switch on) answered both ways; the
// activity's Copy; capture refused while Settings shows the token.
//
//   node tests/studio_mcp.mjs [all|ask|edits|carry|read|picture|badge|consent|capture|settings|recipe …]
//   (RAPIDR: the rapidr to run; ./rapidr)
//
// Never the user's keychain (RAPIDR_AI_KEYSTORE=memory), registry, printer
// or AI keys: everything in tests/results/studio-mcp.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
/// the events (the "user"), what to print at the end (`more`: more of the
/// hooks' environment).
// (opts.project false: nothing open; opts.home: RapidR's home as an install
// has it — a new project then goes in the tests' Documents, never in the
// repository)
function studio(name, steps, events, dump, more = {}, opts = {}) {
  cpSync(join(ROOT, "tests", "fixtures", "studio_mcp"), join(WORK, name), { recursive: true });
  // (opts.main: the main file's text before Studio starts — an empty project, say)
  if (opts.main !== undefined) writeFileSync(join(WORK, name, "main.rr"), opts.main);
  let home = ".";
  if (opts.home) {
    home = join(WORK, `${name}-install`);
    cpSync(join(ROOT, "ide", "assets"), join(home, "ide", "assets"), { recursive: true });
    writeFileSync(join(home, "release.toml"), 'version = "0.0.0"\nrust = "1.0.0"\nkind = "sdk"\n');
  }
  const args = ["run", "ide/studio.rr", "--home", home, "--fresh", "--theme", "rapidr-light", "--do", steps, ...(opts.project === false ? [] : [join(WORK, name, "Hello.rrproj")])];
  const child = spawn(RAPIDR, args, { cwd: ROOT, env: { ...env(name), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump, ...more }, stdio: ["ignore", "pipe", "pipe"] });
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
  // (every description whole: BASIC strings take no backslash escapes — a
  // \" once cut set_property's at "(\")
  const cut = list.filter((t) => !/[.!?)]$/.test((t.description || "").trim()) || (t.description || "").includes("\\"));
  check("every tool's description is whole (no stray backslash, ends a sentence)", list.length > 0 && cut.length === 0, cut.map((t) => `${t.name}: …${(t.description || "").slice(-60)}`).join(" | "));
  check("set_property's description names a text value in quotes", /\("OK" for text, 120, &HFF0000, True\)\. One undo step in its file\.$/.test(list.find((t) => t.name === "set_property")?.description || ""), list.find((t) => t.name === "set_property")?.description);
  // (a tool without arguments is listed with "properties": {} — a model server that builds a grammar from
  // the schema, Ollama for some models, answers HTTP 500 "properties must be an object" to a list without)
  check("every tool's inputSchema is an object with its properties (even none)", list.every((t) => t.inputSchema?.type === "object" && t.inputSchema.properties && typeof t.inputSchema.properties === "object"), list.filter((t) => !t.inputSchema?.properties).map((t) => t.name).join(","));
  check("read_file is marked read-only, propose_edit isn't", list.find((t) => t.name === "read_file")?.annotations?.readOnlyHint === true && list.find((t) => t.name === "propose_edit")?.annotations?.readOnlyHint === false);

  // (what Studio tells the AI app on connecting: the recipe-first workflow and a short language card —
  // tools/ai_eval's finding that small models never look at an example, write other languages' syntax and
  // repeat their error; the whole text stays under about 1,200 tokens)
  const instructions = d.json.result?._meta?.["io.modelcontextprotocol/serverInfo"]?.instructions || d.json.result?.instructions || "";
  check("the server's instructions hold the recipe-first workflow", ["find_example", "check_code", "hint line", "The open project IS the user's program", "create_project only makes a separate new project", "stop rewriting", "propose_edit"].every((w) => instructions.includes(w)), instructions.slice(0, 400));
  check("…and a language card (x = 1 not SET, <> not !=, MOD not %, FOR … NEXT, ELSEIF)", ["RapidR in short", "No SET, LET", "not == or !=", "MOD, not %", "NEXT i", "ELSEIF"].every((w) => instructions.includes(w)), instructions.slice(-600));
  check("…under about 1,200 tokens (3.5 characters a token)", instructions.length > 0 && Math.ceil((instructions.length * 2) / 7) <= 1200, `${instructions.length} characters`);
  const legacyInit = await post({ jsonrpc: "2.0", id: next(), method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "studio-legacy-instructions", version: "0.1" } } });
  check("an earlier revision's initialize carries the same instructions", legacyInit.json.result?.instructions === instructions, String(legacyInit.json.result?.instructions || "").slice(0, 200));
  const byName = (n) => list.find((t) => t.name === n) || {};
  check("create_project and the writing tools say the open project IS the user's program", /open project IS the user's program/.test(byName("create_project").description) && /open project \(which IS the user's program\)/.test(byName("propose_edit").description) && /open project \(which IS the user's program\)/.test(byName("write_file").description) && /open project IS the user's program/.test(byName("open_project").description));
  check("every path argument is lower case (path), open_project's too", Object.keys(byName("open_project").inputSchema?.properties || {}).join() === "path" && Object.keys(byName("read_file").inputSchema?.properties || {}).join() === "path", JSON.stringify(byName("open_project").inputSchema));

  let r = await tool("studio_status");
  check("studio_status: the project, the permission", r.text.includes("Project: Hello") && r.text.includes("Permission: Allow everything"), r.text);
  check("studio_status ends with what to do next (this project has code)", /\nNext: read_file what you change/.test(r.text) && !r.text.includes("This project is empty"), r.text);
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
  check("propose_edit: applied at once (Allow everything)", !r.error && r.text.startsWith("Changed main.rr in Studio's editor (not on disk yet") && !r.text.includes("Waiting") && !r.text.includes("Proposed") && r.raw.progress.length === 0, r.text);
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
  check("propose_edit: allowed once by the user, applied in the editor", !r.error && r.text.startsWith("Changed main.rr in Studio's editor (not on disk yet") && !r.text.includes("Waiting"), r.text);
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
  check("Allow edits: propose_edit applied without a question", !r.error && r.text.startsWith("Changed main.rr in Studio's editor (not on disk yet") && r.raw.progress.length === 0, r.text);
  r = await tool("run_program", { Action: "start" });
  check("Allow edits: run_program still asks (denied here)", r.error && r.text.includes("didn't allow"), r.text);
  r = await tool("read_output");
  const end = await run.ended();
  check("Allow edits: the menu says it, no badge", end.lines["mcpmodebox.text"] === "Allow edits" && !shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
}

// ---- A new project takes the mode of the one before (Robert, 2026-10-29) -----------------
// (never Allow everything: it starts with Allow edits — the legal review's Addendum 2)
if (want("carry")) {
  const NEW = (name) => ({ name, main: "main.rr", files: [{ path: "main.rr", text: 'PRINT "new"\n' }], summary: "A new program." });
  const NOT_PASSED = "New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it.";
  const status = async (tool, want, tries = 40) => {
    let r = { text: "" };
    for (let i = 0; i < tries; i++) {
      r = await tool("studio_status");
      if (r.text.includes(want)) break;
      await new Promise((res) => setTimeout(res, 250));
    }
    return r;
  };

  // Allow edits -> create_project: Allow edits
  let PORT = BASE_PORT + 10;
  let run = studio("carry", `mcp:mode=edits,mcp:on=${PORT}`, until("studio_status"), "mcpmodebox.text,mcplog.items", {}, { home: true });
  check("Carried mode: Studio listens", await listening(PORT));
  let { tool } = client(PORT);
  let r = await tool("create_project", NEW("Carried"));
  check("Carried mode: create_project made and opened it at once (Allow edits)", !r.error && r.text.includes("Created and opened") && r.raw.progress.length === 0, r.text);
  check("Carried mode: the new project in the tests' Documents, not the repository", existsSync(join(WORK, "docs", "RapidR", "Projects", "Carried")) && !existsSync(join(ROOT, "projects", "Carried")), r.text);
  r = await tool("studio_status");
  check("Carried mode: the new project is in Allow edits, as the one before", r.text.includes("Project: Carried") && r.text.includes("Permission: Allow edits"), r.text);
  let end = await run.ended();
  check("Carried mode: the menu says it", end.lines["mcpmodebox.text"] === "Allow edits", JSON.stringify(end.lines));
  check("Carried mode: the pane says it", end.out.includes("New project: same permission as before (Allow edits)"), end.out.slice(-1200));

  // Allow everything -> create_project: Allow edits, said; no badge
  PORT = BASE_PORT + 12;
  run = studio("carry-all", `mcp:mode=all,mcp:on=${PORT}`, until("studio_status"), "mcpmodebox.text,mcpbadge.visible,mcplog.items", {}, { home: true });
  check("Allow everything, create_project: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("create_project", NEW("NotAll"));
  check("Allow everything, create_project: made and opened at once", !r.error && r.text.includes("Created and opened"), r.text);
  r = await tool("studio_status");
  check("Allow everything is never passed on: the new project is in Allow edits", r.text.includes("Project: NotAll") && r.text.includes("Permission: Allow edits") && !r.text.includes("Permission: Allow everything"), r.text);
  end = await run.ended();
  check("Allow everything, create_project: the menu says Allow edits, no badge", end.lines["mcpmodebox.text"] === "Allow edits" && !shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
  check("Allow everything, create_project: the pane says it isn't passed on", end.out.includes(NOT_PASSED), end.out.slice(-1200));

  // Allow everything -> open_project of an example: Allow edits, said
  PORT = BASE_PORT + 13;
  // (the example is asked about first, as a project from elsewhere: Allow Once)
  run = studio("carry-open", `mcp:mode=all,mcp:on=${PORT}`, ["mcpask.__until_Visible_1", "mcponcebtn.onclick", until("studio_status")].join(","), "mcpmodebox.text,mcpbadge.visible,mcplog.items");
  check("Allow everything, open_project: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("open_project", { Path: join(ROOT, "examples", "gui", "hello_form.rr") });
  check("Allow everything, open_project: opened", !r.error && r.text.includes("Opened"), r.text);
  r = await tool("studio_status");
  check("Allow everything is never passed on: the project opened is in Allow edits", r.text.includes("Permission: Allow edits") && !r.text.includes("Permission: Allow everything"), r.text);
  end = await run.ended();
  check("Allow everything, open_project: the menu says Allow edits, no badge", end.lines["mcpmodebox.text"] === "Allow edits" && !shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
  check("Allow everything, open_project: the pane says it isn't passed on", end.out.includes(NOT_PASSED), end.out.slice(-1200));

  // A project with its own mode keeps it: Hello in Allow edits (its own);
  // an AI app makes Kept (carried: Allow edits), the user sets Kept to Ask
  // before changes, the AI app opens Hello again (asked: outside the
  // Projects folder; Allow Once): Hello is in Allow edits, its own — not
  // Ask, Kept's — and nothing is said about a mode carried
  PORT = BASE_PORT + 14;
  run = studio("carry-kept", `mcp:mode=edits,mcp:on=${PORT}`, ["mcpsub.__until_Caption_New project: same permission as before (Allow edits)", "mcpmodebox.__item_1", "mcpask.__until_Visible_1", "mcponcebtn.onclick", until("get_problems")].join(","), "mcpmodebox.text,mcplog.items", {}, { home: true });
  check("Own mode: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("create_project", NEW("Kept"));
  check("Own mode: Kept made (carried Allow edits)", !r.error && r.text.includes("Created and opened"), r.text);
  await new Promise((res) => setTimeout(res, 1500));
  r = await status(tool, "Permission: Ask before changes");
  check("Own mode: the user set Kept to Ask before changes", r.text.includes("Project: Kept") && r.text.includes("Permission: Ask before changes"), r.text);
  r = await tool("open_project", { Path: join(WORK, "carry-kept", "Hello.rrproj") });
  check("Own mode: Hello opened (asked: outside the Projects folder)", !r.error && r.text.includes("Opened"), r.text);
  r = await tool("studio_status");
  check("Own mode: Hello keeps its own Allow edits (not Kept's Ask before changes)", r.text.includes("Project: Hello") && r.text.includes("Permission: Allow edits"), r.text);
  await tool("get_problems");
  end = await run.ended();
  const said = (end.out.match(/same permission as before/g) || []).length;
  check("Own mode: a mode carried is said once (Kept), never for Hello", said === 1, end.out.slice(-1500));

  // Ask before changes -> create_project (asked: Allow Once): Ask before changes
  PORT = BASE_PORT + 15;
  run = studio("carry-ask", `mcp:on=${PORT}`, ["mcpask.__until_Visible_1", "mcponcebtn.onclick", until("studio_status")].join(","), "mcpmodebox.text,mcplog.items", {}, { home: true });
  check("Ask carried: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("create_project", NEW("Asked"));
  check("Ask carried: create_project asked, then made", !r.error && r.text.includes("Created and opened"), r.text);
  r = await tool("studio_status");
  check("Ask carried: the new project asks before changes", r.text.includes("Project: Asked") && r.text.includes("Permission: Ask before changes"), r.text);
  end = await run.ended();
  check("Ask carried: the pane says it", end.out.includes("New project: same permission as before (Ask before changes)"), end.out.slice(-1200));

  // Read only: an AI app can't create or open a project, so it can't leave it
  PORT = BASE_PORT + 16;
  run = studio("carry-read", `mcp:mode=read,mcp:on=${PORT}`, until("studio_status"), "mcpmodebox.text", {}, { home: true });
  check("Read only carried: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("create_project", NEW("ReadNew"));
  check("Read only: create_project refused", r.error && r.text.includes("Read only"), r.text);
  r = await tool("open_project", { Path: join(ROOT, "examples", "gui", "hello_form.rr") });
  check("Read only: open_project refused", r.error && r.text.includes("Read only"), r.text);
  r = await tool("studio_status");
  check("Read only: still the same project, Read only", r.text.includes("Project: Hello") && r.text.includes("Permission: Read only"), r.text);
  end = await run.ended();
  check("Read only: the menu says it", end.lines["mcpmodebox.text"] === "Read only", JSON.stringify(end.lines));

  // (nothing open before: the default)
  PORT = BASE_PORT + 11;
  run = studio("carry-none", `mcp:mode=edits,mcp:on=${PORT}`, until("studio_status"), "mcpmodebox.text,mcplog.items", {}, { project: false, home: true });
  check("Carried mode, nothing open: Studio listens", await listening(PORT));
  ({ tool } = client(PORT));
  r = await tool("create_project", NEW("Fresh"));
  check("Carried mode, nothing open: create_project made it", !r.error && r.text.includes("Created and opened"), r.text);
  r = await tool("studio_status");
  check("Carried mode, nothing open: the new project asks before changes", r.text.includes("Project: Fresh") && r.text.includes("Permission: Ask before changes"), r.text);
  end = await run.ended();
  check("Carried mode, nothing open: nothing said about a mode carried", !end.out.includes("same permission as before"), end.out.slice(-1200));
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
  const run = studio("badge", `mcp:mode=all,mcp:on=${PORT}`, `${until("Checking (compiling)")},mcplogitem(0).onclick`, "mcpbadge.__shown,clipboard.text", { RAPIDR_TEST_CLIPBOARD: "1" });
  check("Allow everything (picture): Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  await tool("studio_status");
  await tool("read_file", { path: "main.rr" });
  await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello, world"' }], summary: "Says hello to the world." });
  await tool("check_code");
  const end = await run.ended(90000);
  check("Allow everything: the badge shows in the pane in front", end.lines["mcpbadge.__shown"] === "1", JSON.stringify(end.lines));
  check("the activity's Copy: its last line as plain text", /^\d\d:\d\d:\d\d  studio-test  Checking \(compiling\)  — ok$/.test(end.lines["clipboard.text"] || ""), JSON.stringify(end.lines));
  if (existsSync(join(WORK, "badge-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "badge-window-1.bmp"), "--out", join(WORK, "studio-mcp-allow-everything-pane.png")], { stdio: "ignore" });
}

// ---- the first-time question (turning the switch on): Cancel by Enter, Turn On ---------
if (want("consent")) {
  const no = studio("consent-no", "mcp:consent", "consentno.__key_13", "mcponbox.checked,consenttitle.caption,consentyes.caption,consentno.default");
  let end = await no.ended(90000);
  check("the first-time question: its title, Turn On, Cancel the default", end.lines["consenttitle.caption"] === "Let AI apps use RapidR Studio?" && end.lines["consentyes.caption"] === "&Turn On" && shown(end.lines["consentno.default"]), JSON.stringify(end.lines));
  check("Enter answers Cancel: the switch stays off", end.lines["mcponbox.checked"] === "0", JSON.stringify(end.lines));
  const yes = studio("consent-yes", "mcp:consent", "consentyes.onclick", "mcponbox.checked");
  end = await yes.ended(90000);
  check("Turn On: the switch on", shown(end.lines["mcponbox.checked"]), JSON.stringify(end.lines));
  // (the pane's menu set to Allow everything: its question, Cancel by Enter, then Allow Everything)
  const keep = studio("all-no", `mcp:on=${BASE_PORT + 7},mcp:pane`, "mcpmodebox.__item_3,consentno.__key_13", "mcpmodebox.text,mcpbadge.visible,consenttitle.caption");
  end = await keep.ended(90000);
  check("Allow everything asked first; Enter keeps Ask before changes", end.lines["consenttitle.caption"] === "Allow everything for AI apps in this project?" && end.lines["mcpmodebox.text"] === "Ask before changes" && !shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
  const all = studio("all-yes", `mcp:on=${BASE_PORT + 8},mcp:pane`, "mcpmodebox.__item_3,consentyes.onclick", "mcpmodebox.text,mcpbadge.visible");
  end = await all.ended(90000);
  check("Allow Everything: the mode and its chip", end.lines["mcpmodebox.text"] === "Allow everything" && shown(end.lines["mcpbadge.visible"]), JSON.stringify(end.lines));
}

// ---- capture refused while a dialog is open (docs/legal/review-2026-10-29-mcp.md,
// addendum B): Settings open, the token shown, an AI app asks for a picture ----
if (want("capture")) {
  const PORT = BASE_PORT + 9;
  const run = studio("capture", `mcp:on=${PORT},wait,mcp:settings`, `mcpaddredit.__until_Text_http://127.0.0.1:${PORT}/mcp,mcpshowtokenbtn.onclick,mcpheaderedit.__until_PasswordChar_,${until("A picture of Studio")}`, "mcpheaderedit.passwordchar,mcpheaderedit.text");
  check("capture guard: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  // (until Studio says Settings is open: studio_status names an open dialog)
  let status = "";
  for (let i = 0; i < 60 && !/Open in Studio: Settings/.test(status); i++) {
    status = (await tool("studio_status")).text;
    if (!/Open in Studio: Settings/.test(status)) await new Promise((res) => setTimeout(res, 500));
  }
  check("studio_status names the open dialog", /Open in Studio: Settings \(no picture of Studio until it closes\)/.test(status), status);
  // (a moment for the user's Show: the token on screen when the AI app asks)
  await new Promise((res) => setTimeout(res, 2000));
  const r = await tool("capture");
  const raw = JSON.stringify(r.raw.json || {});
  check("capture refused while Settings is open, said plainly", r.error && /Settings is open/.test(r.text) && !r.content.some((c) => c.type === "image"), r.text);
  check("capture: the token appears nowhere in the answer", !raw.includes(TOKEN), raw.slice(0, 300));
  const end = await run.ended(90000);
  check("capture guard: the token was on screen (Show pressed)", end.lines["mcpheaderedit.passwordchar"] === "" && (end.lines["mcpheaderedit.text"] || "").includes(TOKEN), JSON.stringify(end.lines));
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

// ---- Recipe first: an empty project, the same refusal twice, the paths ------------------------
// (tools/ai_eval's findings for small models: they never looked at an example, rewrote the whole file
// after the same error, and named paths with the project folder before them)
if (want("recipe")) {
  const PORT = BASE_PORT + 17;
  const run = studio("recipe", `mcp:mode=all,mcp:on=${PORT}`, until("read_file"), "mcphead.caption", {}, { main: "' MyApp: an empty program.\n" });
  check("recipe: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("studio_status");
  check("an empty project's first status says: call find_example first", !r.error && r.text.includes("This project is empty: call find_example first"), r.text);
  r = await tool("find_example", { query: "a stopwatch with start stop and reset" });
  check("find_example: a working program", !r.error && r.text.includes("```basic"), r.text);

  // (the same refusal twice: the second one changes the advice, and gives the nearest recipe's lines)
  const broken = (more) => `' a stopwatch with Start, Stop and Reset buttons and a timer\nDIM x AS INTEGER\nSET x = 1\n${more}`;
  r = await tool("write_file", { path: "main.rr", text: broken(""), summary: "A first try." });
  check("a SET statement: refused, with the hint under the compiler's message", r.error && r.text.includes("Syntax error in SET statement") && r.text.includes("hint main.rr:3: RapidR has no SET: write x = 1") && !r.text.includes("Same error again"), r.text);
  r = await tool("write_file", { path: "main.rr", text: broken("PRINT x\n"), summary: "A second try." });
  check("the same refusal again: stop rewriting, find_example, the nearest recipe", r.error && r.text.includes("Same error again") && r.text.includes("Stop rewriting the whole file") && r.text.includes("Call find_example('") && r.text.includes("```basic"), r.text);

  // (paths: the project folder before the file's name, ./, the absolute path inside the project)
  const folder = join(WORK, "recipe");
  r = await tool("write_file", { path: "project/main.rr", text: 'PRINT "recipe first"\n', summary: "A program that compiles." });
  check("project/main.rr is the project's main.rr, and a change that is taken forgets the refusals", !r.error && r.text.startsWith("Changed main.rr in Studio's editor (not on disk yet") && !r.text.includes("Waiting"), r.text);
  r = await tool("read_file", { path: "./main.rr" });
  check("./main.rr is read", !r.error && r.text.includes("recipe first"), r.text);
  r = await tool("read_file", { path: `${folder}/main.rr` });
  check("the absolute path inside the project is read", !r.error && r.text.includes("recipe first"), r.text);
  r = await tool("read_file", { Path: "recipe/main.rr" });
  check("the folder's name before it, and the old capitalised Path, are taken", !r.error && r.text.includes("recipe first"), r.text);
  r = await tool("read_file", { path: "/etc/hosts" });
  check("a path outside the project: refused, with the project's files", r.error && r.text.includes("outside the project") && r.text.includes("The project's files") && r.text.includes("main.rr"), r.text);
  r = await tool("read_file", { path: "nothing.rr" });
  check("a file the project doesn't have: the project's files listed", r.error && r.text.includes("is not a file of the project") && r.text.includes("main.rr"), r.text);
  r = await tool("open_project", { path: "examples/../../etc/hosts" });
  check("open_project takes a lower case path (and says why it can't)", r.error && r.text.includes("Can't open"), r.text);
  r = await tool("studio_status");
  check("with code in it the status no longer says the project is empty", !r.text.includes("This project is empty") && r.text.includes("Next: read_file"), r.text);
  r = await tool("read_file", { path: "main.rr" });
  const end = await run.ended();
  check("recipe: Studio ended", end.ok);
}

console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
