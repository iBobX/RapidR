// RapidR Studio's MCP server (ide/mcp.inc, RMcpServer) driven by an AI app:
// Studio started headless with AI apps switched on (the test step
// mcp:on — only with RAPIDR_TEST_MCP_TOKEN, which stands in for the
// keychain's token), RAI set up against the tests' mock provider, a fixture
// project open; then an MCP client — this script — lists the tools, reads
// the project, is refused outside it and in a hidden folder, proposes a
// change the "user" allows once in the AI Apps pane (a click on Allow Once)
// and one it denies, runs the program and reads its output, asks RAI and
// reads its steps, opens a session of an earlier revision, is refused
// without the token and from a web page. Studio's window is captured at
// the end with the AI Apps pane showing what happened (the activity
// indicator: the status bar's panel), and a second run captures Settings >
// AI Apps.
//
//   node tests/studio_mcp.mjs            (RAPIDR: the rapidr to run; ./rapidr)
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
const PORT = Number(process.env.STUDIO_MCP_PORT || 47931);
const MODERN = "2026-07-28";
const CLIENT = "studio-test";

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
cpSync(join(ROOT, "tests", "fixtures", "studio_mcp"), join(WORK, "project"), { recursive: true });

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

function studio(name, steps, events, dump, open) {
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", steps];
  if (open) args.push(open);
  const child = spawn(RAPIDR, args, { cwd: ROOT, env: { ...env(name), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump }, stdio: ["ignore", "pipe", "pipe"] });
  children.push(child);
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  child.stderr.on("data", (d) => (out += d));
  const done = new Promise((resolve) => child.on("exit", (code) => resolve({ code, out })));
  return { child, done };
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
let nextId = 1;
const meta = { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": { name: CLIENT, version: "1.0" }, "io.modelcontextprotocol/clientCapabilities": {} };

/// One POST: { status, headers, json, text } — an event-stream answer read
/// to its last message.
async function post(body, headers = {}) {
  const r = await fetch(`http://127.0.0.1:${PORT}/mcp`, {
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
    return { text: (res.content || []).map((c) => c.text || "").join("\n") || JSON.stringify(r.json.error || {}), error: !!res.isError || !!r.json.error, raw: r };
  } catch (e) {
    return { text: `(no answer: ${e.message})`, error: true, raw: { progress: [] } };
  }
}

let failures = 0;
function check(what, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${ok ? "" : `  — ${String(detail).slice(0, 600)}`}`);
  if (!ok) failures++;
}

// ---- run 1: an AI app at work ----------------------------------------------------------
const AI_READY = `ai:on,ai:provider=compatible,ai:base=${MOCK_URL},ai:model=mock-small`;
const steps = [AI_READY, "mcp:perm=run:allow", "mcp:perm=rai:allow", `mcp:on=${PORT}`].join(",");
// (the "user": Allow Once on the first change, Deny on the second and on
// the project to open, then the capture once the last call — get_rai_audit
// — shows in the pane)
const events = ["mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", `mcpsub.__until_Caption_${CLIENT}: get_rai_audit`].join(",");
const dump = "mcphead.caption,mcplog.itemcount,status.panelcount,mcpask.visible";
const run1 = studio("work", steps, events, dump, join(WORK, "project", "Hello.rrproj"));

check("Studio listens for AI apps", await listening(PORT));

const d = await call("server/discover");
check("server/discover: the current revision, Studio's name", d.json.result?.supportedVersions?.[0] === MODERN && d.json.result?._meta?.["io.modelcontextprotocol/serverInfo"]?.name === "rapidr-studio", JSON.stringify(d.json));
const list = (await call("tools/list")).json.result?.tools || [];
const names = list.map((t) => t.name);
for (const t of ["studio_status", "list_files", "read_file", "rapidr_describe", "check_code", "search_manual", "find_example", "project_outline", "propose_edit", "write_file", "create_project", "list_documents", "get_problems", "read_output", "open_project", "run_program", "ask_rai", "get_rai_steps", "get_rai_transcript", "get_rai_audit"]) {
  check(`tools/list has ${t}`, names.includes(t), names.join(","));
}
check("read_file is marked read-only, propose_edit isn't", list.find((t) => t.name === "read_file")?.annotations?.readOnlyHint === true && list.find((t) => t.name === "propose_edit")?.annotations?.readOnlyHint === false);

let r = await tool("studio_status");
check("studio_status: the project, what the AI app may do", r.text.includes("Project: Hello") && r.text.includes("change files — the user is asked each time") && r.text.includes("run programs — allowed"), r.text);
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

// (a change: proposed, then waiting for the user — Allow Once in the pane)
r = await tool("propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello from an AI app"' }], summary: "Greets from the AI app." });
check("propose_edit: allowed once by the user, applied in the editor", !r.error && r.text.includes("Applied"), r.text);
check("propose_edit: the AI app heard it waits for the user", r.raw.progress.some((p) => JSON.stringify(p).includes("Waiting for the user")), JSON.stringify(r.raw.progress));
r = await tool("write_file", { path: "notes.txt", text: "A note\n", summary: "A notes file." });
check("write_file: denied by the user", r.error && r.text.includes("didn't allow"), r.text);
r = await tool("list_documents");
check("list_documents: main.rr changed, not saved", r.text.includes("main.rr") && r.text.includes("unsaved changes"), r.text);

r = await tool("run_program", { Action: "start", Wait: 20 });
check("run_program: the changed program ran, its output back", !r.error && r.text.includes("Hello from an AI app") && r.text.includes("Line 2"), r.text);
check("run_program saved the change first (as Run does)", readFileSync(join(WORK, "project", "main.rr"), "utf8").includes("Hello from an AI app"));
r = await tool("read_output");
check("read_output: nothing new", r.text.includes("(no new output)") && r.text.includes("isn't running"), r.text);
r = await tool("get_problems");
check("get_problems: none", !r.error, r.text);
r = await tool("open_project", { Path: "examples/../../etc/hosts" });
check("open_project: only projects and programs, said at once", r.error && r.text.includes("Can't open"), r.text);
r = await tool("open_project", { Path: join(WORK, "project", "Hello.rrproj") });
check("open_project: asked, denied by the user", r.error && r.text.includes("didn't allow"), r.text);

r = await tool("ask_rai", { Prompt: "check the program" });
check("ask_rai: RAI answered, its steps back", !r.error && r.text.includes("Answer:") && r.text.includes("Steps:"), r.text);
check("ask_rai: RAI's progress heard while it worked", r.raw.progress.length > 0 || r.text.includes("RAI: idle"), JSON.stringify(r.raw.progress));
r = await tool("get_rai_steps");
check("get_rai_steps: RAI idle", r.text.includes("RAI: idle"), r.text);
r = await tool("get_rai_transcript");
check("get_rai_transcript: the question in the chat", r.text.includes("check the program"), r.text);

// (an earlier revision's session, beside)
const init = await post({ jsonrpc: "2.0", id: nextId++, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "studio-legacy", version: "0.1" } } });
const sid = init.headers.get("mcp-session-id") || "";
check("initialize (2025-06-18): a session", init.json.result?.protocolVersion === "2025-06-18" && sid.length === 32, JSON.stringify(init.json));
const legacy = await post({ jsonrpc: "2.0", id: nextId++, method: "tools/call", params: { name: "studio_status", arguments: {} } }, { "Mcp-Session-Id": sid, "MCP-Protocol-Version": "2025-06-18" });
check("a session's tools/call", (legacy.json.result?.content || [])[0]?.text?.includes("RapidR Studio"), JSON.stringify(legacy.json).slice(0, 300));
const ended = await fetch(`http://127.0.0.1:${PORT}/mcp`, { method: "DELETE", headers: { Authorization: `Bearer ${TOKEN}`, "Mcp-Session-Id": sid } });
check("DELETE ends the session", ended.status === 200);

// (refused by the server itself)
const noToken = await fetch(`http://127.0.0.1:${PORT}/mcp`, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
check("no token: 401", noToken.status === 401);
const page = await post({ jsonrpc: "2.0", id: 99, method: "ping" }, { Origin: "https://evil.example" });
check("a web page's request: 403", page.status === 403);

r = await tool("get_rai_audit", { Count: 5 });
check("get_rai_audit (the last call)", !r.error, r.text);

const end1 = await Promise.race([run1.done, new Promise((res) => setTimeout(() => res(null), 120000))]);
if (!end1) {
  run1.child.kill();
  check("Studio ended after the capture", false, "it didn't end in 2 minutes");
} else {
  const lines = Object.fromEntries(end1.out.split("\n").filter((l) => /^[a-z0-9]+\.[a-z_]+=/.test(l)).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1)]));
  check("the pane says who is connected", (lines["mcphead.caption"] || "").startsWith(`AI apps: ${CLIENT}`), JSON.stringify(lines));
  check("the pane lists what happened", Number(lines["mcplog.itemcount"]) >= 20, lines["mcplog.itemcount"]);
  check("the status bar has AI apps' panel", lines["status.panelcount"] === "7", lines["status.panelcount"]);
  check("nothing waits for the user", lines["mcpask.visible"] === "0", lines["mcpask.visible"]);
}
const audit = existsSync(join(WORK, "work-audit.jsonl")) ? readFileSync(join(WORK, "work-audit.jsonl"), "utf8") : "";
check("the audit log has the AI app's calls", audit.split("\n").filter((l) => l.includes('"source":"mcp"')).length >= 15, audit.slice(0, 400));
check("the audit log never has the token, nor a file's text", !audit.includes(TOKEN) && !audit.includes("Hello from an AI app"), "");
check("the decisions are in the audit log", audit.includes("denied by you") && audit.includes("applied (you allowed it)"));
if (existsSync(join(WORK, "work-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "work-window-1.bmp"), "--out", join(WORK, "studio-mcp-activity.png")], { stdio: "ignore" });
check("Studio's window captured with the AI Apps pane", existsSync(join(WORK, "studio-mcp-activity.png")));

// ---- run 2: a change waiting for the user (the picture of the pane) ---------------------
cpSync(join(ROOT, "tests", "fixtures", "studio_mcp"), join(WORK, "project2"), { recursive: true });
const PORT3 = PORT + 2;
const run3 = studio("ask", `mcp:on=${PORT3}`, "mcpask.__until_Visible_1,studio.__wait_600", "mcpask.visible,mcpasktext.caption", join(WORK, "project2", "Hello.rrproj"));
check("Studio listens again", await listening(PORT3));
{
  const at = (body, headers = {}) => fetch(`http://127.0.0.1:${PORT3}/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream", Authorization: `Bearer ${TOKEN}`, "MCP-Protocol-Version": MODERN, ...headers }, body: JSON.stringify(body), signal: AbortSignal.timeout(60000) }).then((r) => r.text()).catch(() => "");
  const t = (id, name, args) => at({ jsonrpc: "2.0", id, method: "tools/call", params: { name, arguments: args, _meta: meta } }, { "Mcp-Method": "tools/call", "Mcp-Name": name });
  await t(1, "studio_status", {});
  await t(2, "read_file", { path: "main.rr" });
  await t(3, "check_code", {});
  // (it waits for the user: Studio's window is captured meanwhile, and ends)
  await t(4, "propose_edit", { path: "main.rr", edits: [{ old_text: 'PRINT "Hello from Studio"', new_text: 'PRINT "Hello, world"' }], summary: "Says hello to the world." });
}
const end3 = await Promise.race([run3.done, new Promise((res) => setTimeout(() => res(null), 90000))]);
if (!end3) run3.child.kill();
check("the pane asks the user about the change", (end3?.out || "").includes("mcpasktext.caption=studio-test wants to change main.rr: Says hello to the world."), (end3?.out || "").split("\n").filter((l) => l.startsWith("mcpask")).join(" | "));
if (existsSync(join(WORK, "ask-window-1.bmp"))) spawnSync("sips", ["-s", "format", "png", join(WORK, "ask-window-1.bmp"), "--out", join(WORK, "studio-mcp-ask.png")], { stdio: "ignore" });

// ---- run 3: Settings > AI Apps ---------------------------------------------------------
const run2 = studio("settings", `mcp:on=${PORT + 1},wait,mcp:settings`, "", "mcponbox.checked,mcpaddredit.text,mcpheaderedit.passwordchar", "");
const end2 = await Promise.race([run2.done, new Promise((res) => setTimeout(() => res(null), 90000))]);
if (!end2) run2.child.kill();
const out2 = end2 ? end2.out : "";
check("Settings > AI Apps: on, the address, the token masked", /mcponbox\.checked=(1|-1)/.test(out2) && out2.includes(`mcpaddredit.text=http://127.0.0.1:${PORT + 1}/mcp`) && out2.includes("mcpheaderedit.passwordchar=*"), out2.split("\n").filter((l) => l.startsWith("mcp")).join(" | "));
for (const n of [1, 2]) {
  const bmp = join(WORK, `settings-window-${n}.bmp`);
  if (existsSync(bmp)) spawnSync("sips", ["-s", "format", "png", bmp, "--out", join(WORK, `studio-mcp-settings-${n}.png`)], { stdio: "ignore" });
}
check("Settings captured", existsSync(join(WORK, "studio-mcp-settings-2.png")) || existsSync(join(WORK, "studio-mcp-settings-1.png")));

mockProc.kill();
console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
