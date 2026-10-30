// RapidR Studio's eyes and hands on the program it runs, driven by an AI
// app over Studio's MCP server (ide/mcp.inc, ide/mcpapp.inc): Studio
// started headless with AI apps on (the test step mcp:on — only with
// RAPIDR_TEST_MCP_TOKEN), a small calculator project open
// (tests/fixtures/app_eyes); the program Studio runs stays up for a minute
// (RAPIDR_TEST_PROGRAM_EVENTS: its own GUI test waits), and this script is
// the AI app, in each permission mode:
//
//   Allow everything  nothing asked: app_layout lists the buttons and
//                     captions; app_click by caption and by name (7 + 8 =
//                     15), app_type into the box beside a label, Hello
//                     greets, app_key; a picture of the window (and one with
//                     the pointer gliding); what isn't there said with what
//                     is; another window can't be asked for; paused in the
//                     debugger a picture still, a click refused with where;
//                     going on, input again; app_wait
//   Ask before changes  looking at the program asks like using it: the first
//                     look of a run asks (Allow Once), then pictures, clicks
//                     and typing aren't asked until the run ends; a new run
//                     (restart) asks again
//   Allow edits       asks too: a Deny (of typing) grants nothing — the next
//                     look asks again
//   Read only         the user runs it; every app_* tool that looks at or
//                     uses it is refused, app_wait (time passing) works
//   Session           Allow for This Session: the running program isn't
//                     asked about again in the next run — until another
//                     project opens: then the run and the look ask again
//   Upgrade           a user who confirmed AI apps before they could look
//                     at and use the running program (McpConfirmed, no
//                     McpConsentVersion), a project in Allow everything:
//                     the question once more at the start — Cancel: AI apps
//                     off, the server never started; Turn On: the server
//                     with the app_* tools —, the project set back to Ask
//                     before changes, and not asked again at the next start
//   RAI               RAI, Studio's own assistant (the tests' mock
//                     provider): asked to click 8, it asks the user first
//                     (its chat's card) and is told they didn't allow it;
//                     asked to click 7, it asks again, is allowed for this
//                     run, reads the layout and clicks with the same eyes
//                     and hands, its turn waiting for the program's answer;
//                     its audit: what ran, no digest of what it typed
//
// In the four AI-app modes a canary: CANARY-TYPED-7f3a typed into the name
// box, a click on a caption that isn't there (CANARY-NOCAPTION: the answer
// lists the program's captions, CANARY-BUTTON among them), a picture and
// the layout — then neither the audit log nor the AI Apps pane may hold
// what was typed, what the program shows or a picture, and no picture may
// be left in the temporary folder.
//
//   node tests/app_eyes.mjs [all|ask|edits|read|session|upgrade|rai …]   (RAPIDR: the rapidr)
//
// Never the user's keychain, registry, printer or AI keys: everything in
// tests/results/app-eyes. Ports 47905-47911.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import net from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
// (SEC-24: a release build's test hooks need the test-mode token: tests/test_mode.mjs)
import "./test_mode.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "app-eyes");
const TOKEN = "app-eyes-test-token-0123456789abcdef";
const BASE_PORT = Number(process.env.APP_EYES_PORT || 47905);
const MODERN = "2026-07-28";
const CLIENT = "app-eyes-test";
const only = process.argv.slice(2);

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
const children = [];
// (RAI's provider: the tests' mock, on this machine)
let MOCK_URL = "";
if (only.length === 0 || only.includes("rai")) {
  const mockProc = spawn(process.execPath, [join(HERE, "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
  children.push(mockProc);
  MOCK_URL = await new Promise((resolve) => mockProc.stdout.once("data", (d) => resolve(String(d).trim())));
}
process.on("exit", () => children.forEach((c) => c.kill()));

function env(name, extra = {}) {
  return {
    ...process.env,
    RAPIDR_CAPTURE: join(WORK, `${name}-window`),
    RAPIDR_CAPTURE_DELAY: "1.5",
    RAPIDR_MENU: "window",
    RAPIDR_TEST_MCP_TOKEN: TOKEN,
    // (the program Studio runs: its own GUI test waits a minute, so the AI
    // app has time to use it)
    RAPIDR_TEST_PROGRAM_EVENTS: "form1.__wait_60000",
    RAPIDR_PRINT_TO: join(WORK, "prints"),
    RAPIDR_REGISTRY: join(WORK, `${name}.reg`),
    RAPIDR_DOCUMENTS: join(WORK, "docs"),
    RAPIDR_AI_KEYSTORE: "memory",
    RAPIDR_AI_AUDIT: join(WORK, `${name}-audit.jsonl`),
    RAPIDR_AI_CHATS: join(WORK, "chats"),
    RAPIDR_LOGIN_PATH: "off",
    ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "",
    ...extra,
  };
}

// (opts.project: the project's folder, else a copy of the fixture named
// after the run; opts.fresh false: Studio reads its settings — the run's
// registry — as a user's Studio does)
function studio(name, steps, events, dump, extra, opts = {}) {
  const project = opts.project || join(WORK, name);
  if (!existsSync(join(project, "Calc.rrproj"))) cpSync(join(ROOT, "tests", "fixtures", "app_eyes"), project, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", ...(opts.fresh === false ? [] : ["--fresh"]), "--theme", "rapidr-light", ...(steps ? ["--do", steps] : []), join(project, "Calc.rrproj")];
  const child = spawn(RAPIDR, args, { cwd: ROOT, env: { ...env(name, extra), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump }, stdio: ["ignore", "pipe", "pipe"] });
  children.push(child);
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  child.stderr.on("data", (d) => (out += d));
  const done = new Promise((resolve) => child.on("exit", (code) => resolve({ code, out })));
  const ended = async (ms = 150000) => {
    const r = await Promise.race([done, new Promise((res) => setTimeout(() => res(null), ms))]);
    if (!r) child.kill();
    writeFileSync(join(WORK, `${name}-studio.log`), r?.out || out);
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

const meta = { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": { name: CLIENT, version: "1.0" }, "io.modelcontextprotocol/clientCapabilities": {} };

function client(port) {
  let nextId = 1;
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
    return { status: r.status, json, text, progress };
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
  return { call, tool };
}

let failures = 0;
function check(what, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${ok ? "" : `  — ${String(detail).slice(0, 700)}`}`);
  if (!ok) failures++;
}
const want = (name) => only.length === 0 || only.includes(name);
const until = (what) => `mcpsub.__until_Caption_${CLIENT}: ${what}`;
const png = (r) => r.content.find((c) => c.type === "image");
const isPng = (img) => !!img && img.mimeType === "image/png" && Buffer.from(img.data, "base64").subarray(1, 4).toString() === "PNG";
const save = (img, file) => img && writeFileSync(join(WORK, file), Buffer.from(img.data, "base64"));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const asked = (r) => r.raw.progress.some((p) => JSON.stringify(p).includes("Waiting for the user"));

/// app_layout until the program's window is there (it starts in a moment);
/// `asked`: one of the calls waited for the user.
async function layoutWhenUp(tool) {
  let r = { text: "" };
  let wasAsked = false;
  for (let i = 0; i < 40; i++) {
    r = await tool("app_layout");
    wasAsked ||= asked(r);
    if (r.text.includes("b7 (RButton)")) return { ...r, asked: wasAsked };
    await sleep(250);
  }
  return { ...r, asked: wasAsked };
}

// ---- the canary: what the program shows and what was typed stay out of the log --------
const CANARY_TYPED = "CANARY-TYPED-7f3a";
const CANARY_ASKED = "CANARY-NOCAPTION";
const CANARY_SHOWN = "CANARY-BUTTON";

/// Typing the canary, a click on a caption that isn't there (its answer
/// lists the captions there), a picture and the layout.
async function canaryCalls(mode, tool, allowed) {
  const typed = await tool("app_type", { Label: "Name", Text: CANARY_TYPED });
  const missing = await tool("app_click", { Caption: CANARY_ASKED });
  const shot = await tool("app_screenshot");
  const layout = await tool("app_layout");
  if (allowed) {
    check(`${mode}: the canary typed`, !typed.error, typed.text);
    check(`${mode}: the click on what isn't there lists the canary caption (to the AI app)`, missing.error && missing.text.includes(CANARY_SHOWN), missing.text);
    check(`${mode}: the picture and the layout (with the canary) go to the AI app`, isPng(png(shot)) && layout.text.includes(CANARY_TYPED), layout.text);
  } else {
    check(`${mode}: the canary calls refused`, [typed, missing, shot, layout].every((r) => r.error && !png(r)), [typed, missing, shot, layout].map((r) => r.text).join(" | "));
  }
}

/// The AI Apps pane's lines (the dump's mcplog.items, a line each).
function paneText(out) {
  const at = out.indexOf("mcplog.items=");
  if (at < 0) return "";
  const lines = out.slice(at + "mcplog.items=".length).split("\n");
  const end = lines.findIndex((l, i) => i > 0 && /^[a-z0-9]+\.[a-z_]+=/.test(l));
  return (end < 0 ? lines : lines.slice(0, end)).join("\n");
}

/// Neither the audit log nor the pane holds what was typed, what the
/// program showed or a picture; an error is logged as "error" only; no
/// picture is left in the temporary folder.
function checkTrail(mode, end, pid) {
  const file = join(WORK, `${mode}-audit.jsonl`);
  const audit = existsSync(file) ? readFileSync(file, "utf8") : "";
  const pane = paneText(end.out);
  check(`${mode}: the audit log lists the AI app's app_* calls`, audit.includes('"tool":"app_'), audit.slice(-400));
  check(`${mode}: the pane was read`, pane.includes(CLIENT), end.out.slice(-600));
  for (const c of [CANARY_TYPED, CANARY_SHOWN]) {
    check(`${mode}: ${c} is in neither the audit log nor the pane`, !audit.includes(c) && !pane.includes(c), (audit.includes(c) ? "audit: " : "pane: ") + (audit.includes(c) ? audit : pane).slice(0, 600));
  }
  check(`${mode}: the caption asked for isn't in the audit log`, !audit.includes(CANARY_ASKED), audit.slice(0, 600));
  check(`${mode}: the pane names it only as the AI app's request ("Clicking …"), never the program's answer`, pane.split("\n").filter((l) => l.includes(CANARY_ASKED)).every((l) => l.includes(`Clicking "${CANARY_ASKED}"`) && !/Buttons now|No button/.test(l)), pane);
  check(`${mode}: no picture in the audit log or the pane`, !/iVBOR|data:image/.test(audit + pane));
  const appLines = audit.split("\n").filter((l) => l.includes('"tool":"app_')).map((l) => JSON.parse(l));
  check(`${mode}: each app_* call is logged with its outcome only (ok, error, refused, denied)`, appLines.length > 0 && appLines.every((l) => l.target === "" && /^(ok|error|refused.*|denied by you|allowed.*|asked.*)$/.test(l.decision)), JSON.stringify(appLines.map((l) => l.decision)));
  const left = readdirSync(tmpdir()).filter((f) => f.startsWith(`rapidr-app-${pid}-`) && f.endsWith(".png"));
  check(`${mode}: no picture left in the temporary folder`, left.length === 0, left.join(", "));
}

// ---- Allow everything: the whole tour ----------------------------------------------------
if (want("all")) {
  const PORT = BASE_PORT;
  const run = studio("all", ["mcp:mode=all", `mcp:on=${PORT}`].join(","), until("get_problems"), "mcplog.itemcount,mcplog.items", { RAPIDR_SCALE: "2" });
  check("Studio listens for AI apps", await listening(PORT));
  const { call, tool } = client(PORT);
  const list = (await call("tools/list")).json.result?.tools || [];
  const names = list.map((t) => t.name);
  for (const t of ["app_layout", "app_screenshot", "app_click", "app_type", "app_key", "app_drag", "app_scroll", "app_wait"]) check(`tools/list has ${t}`, names.includes(t), names.join(","));
  const hint = (n) => list.find((t) => t.name === n)?.annotations?.readOnlyHint;
  check("looking at the program isn't read-only (it shows what the program shows), nor is using it; app_wait is", hint("app_screenshot") === false && hint("app_layout") === false && hint("app_click") === false && hint("app_wait") === true, JSON.stringify(list.filter((t) => t.name.startsWith("app_")).map((t) => [t.name, t.annotations])));
  check("each tool's description has an example call", list.filter((t) => t.name.startsWith("app_")).every((t) => t.description.includes("Example")), JSON.stringify(list.filter((t) => t.name.startsWith("app_")).map((t) => t.description)));

  let r = await tool("app_layout");
  check("app_layout before a run: run it first", r.error && r.text.includes("The program isn't running: run it first (run_program)"), r.text);
  r = await tool("run_program", { Action: "start", Debug: true, Wait: 2 });
  check("run_program: it runs (with the debugger)", !r.error && r.text.includes("running"), r.text);

  r = await layoutWhenUp(tool);
  check("app_layout: the window and its buttons with their captions", !r.error && r.text.includes('Window form1 "Calculator" (RForm), inside 240 × 236, in front') && r.text.includes('b7 (RButton) "7" at 10,40 size 50×30') && r.text.includes('bplus (RButton) "+"') && r.text.includes("nameedit (REdit)"), r.text);
  check("app_layout: nothing looks wrong", r.text.includes("No problems seen"), r.text);

  // (the pointer gliding: a picture with it, taken while a click is on its way)
  const gliding = tool("app_click", { Caption: "7" });
  await sleep(140);
  const mid = await tool("app_screenshot", { Window: "Calculator", Cursor: true });
  save(png(mid), "app-eyes-cursor-mid-move.png");
  r = await gliding;
  check("app_click by caption: clicked 7", !r.error && r.text.startsWith('Clicked b7 (RButton "7") in form1 ("Calculator") at 35,55.'), r.text);
  check("a picture taken while the pointer glides", isPng(png(mid)), mid.text);
  r = await tool("app_click", { Caption: "+" });
  check("app_click +", !r.error, r.text);
  r = await tool("app_click", { Name: "B8" });
  check("app_click by name: B8", !r.error && r.text.startsWith('Clicked b8 (RButton "8")'), r.text);
  r = await tool("app_click", { Caption: "=" });
  r = await tool("app_layout", { Window: "form1" });
  check("7 + 8 = 15 on the display", r.text.includes('display (RLabel) "15"'), r.text);
  r = await tool("app_screenshot");
  check("app_screenshot: a PNG of the program's window", !r.error && isPng(png(r)) && r.text.includes("pixel picture"), r.text);
  save(png(r), "app-eyes-calculator-15.png");
  r = await tool("app_screenshot", { Window: "form1", Scale: "1x", Frame: false });
  check("app_screenshot at 1x, its inside only: 240 × 236", r.text.includes("240 × 236 pixel picture"), r.text);

  r = await tool("app_type", { Label: "Name", Text: "Ada" });
  check("app_type beside the label Name", !r.error && r.text.includes('Typed "Ada" into nameedit (REdit) (beside the label "Name:")'), r.text);
  r = await tool("app_click", { Caption: "Hello" });
  r = await tool("read_output");
  check("the program greeted Ada (its own output)", r.text.includes("Hello, Ada!") && r.text.includes("result 15"), r.text);
  r = await tool("app_key", { Name: "NameEdit", Keys: "Ctrl+A Backspace" });
  check("app_key Ctrl+A Backspace", !r.error && r.text.startsWith("Pressed Ctrl+A Backspace in nameedit"), r.text);
  r = await tool("app_layout");
  check("the name box is empty again", !r.text.includes('nameedit (REdit) text "Ada"'), r.text);
  r = await tool("app_key", { Keys: "Hyper+Q" });
  check("a key that isn't one: said, with what to use", r.error && r.text.includes("isn't a modifier"), r.text);

  r = await tool("app_click", { Caption: "Go" });
  check("app_click on what isn't there: the buttons there are", r.error && r.text.includes('No button (or other control) shows "Go" in form1. Buttons now: "7", "8", "9", "+", "=", "C", "Hello", "CANARY-BUTTON"'), r.text);
  r = await tool("app_screenshot", { Window: "Finder" });
  check("app_screenshot of a window the program hasn't got: refused, its windows listed", r.error && r.text.includes('No window named "Finder" in this program. Its windows now: form1 ("Calculator").') && !png(r), r.text);
  r = await tool("app_screenshot", { Window: "RapidR Studio" });
  check("…nor RapidR Studio's own window", r.error && !png(r), r.text);

  // (paused in the debugger: looking yes, input no)
  r = await tool("debug", { Action: "pause", Wait: 5 });
  check("debug pause: paused", r.text.includes("paused"), r.text);
  r = await tool("app_click", { Caption: "C" });
  check("paused: app_click refused with where, and what to do", r.error && /^The program is paused in the debugger at line \d+ of calc\.rr: continue it first/.test(r.text), r.text);
  r = await tool("app_screenshot");
  check("paused: app_screenshot still works", !r.error && isPng(png(r)), r.text);
  r = await tool("debug", { Action: "continue", Wait: 1 });
  r = await tool("app_click", { Caption: "C" });
  check("going on: app_click works again", !r.error && r.text.startsWith('Clicked bclear (RButton "C")'), r.text);
  r = await tool("app_wait", { Ms: 200 });
  check("app_wait", !r.error && r.text.includes("Waited 200 ms"), r.text);
  await canaryCalls("all", tool, true);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Studio ended after the tour", end.ok);
  check("the pane lists the AI app's calls", Number(end.lines["mcplog.itemcount"]) >= 20, end.lines["mcplog.itemcount"]);
  check("the pane says how many characters were typed, never what", paneText(end.out).includes('Typing 3 characters into the box beside "Name" in the running program') && !paneText(end.out).includes("Ada"), paneText(end.out));
  checkTrail("all", end, run.child.pid);
  const shot = join(WORK, "all-window-1.bmp");
  if (existsSync(shot)) spawnSync("sips", ["-s", "format", "png", shot, "--out", join(WORK, "app-eyes-studio.png")], { stdio: "ignore" });
}

// ---- Ask before changes: asked once per run, looking too ---------------------------------
if (want("ask")) {
  const PORT = BASE_PORT + 1;
  // (the run: Allow for This Session; the first look of the run: Allow
  // Once; the first look of the next run (a restart): Allow Once)
  const events = ["mcpask.__until_Visible_1", "mcpsessionbtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", until("get_problems")].join(",");
  const run = studio("ask", `mcp:on=${PORT}`, events, "mcpask.visible,mcplog.items");
  check("Ask before changes: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Ask before changes: the run allowed (for this session)", !r.error, r.text);
  r = await layoutWhenUp(tool);
  check("the first look at the program waited for the user, then answered", r.asked && r.text.includes("b7 (RButton)"), r.text);
  r = await tool("app_click", { Caption: "7" });
  check("using it in the same run isn't asked again", !r.error && r.text.startsWith("Clicked b7") && !asked(r), r.text + JSON.stringify(r.raw.progress));
  r = await tool("app_click", { Caption: "8" });
  check("…nor the next click", !r.error && r.text.startsWith("Clicked b8") && !asked(r), r.text);
  r = await tool("app_screenshot");
  check("…nor a picture", !r.error && isPng(png(r)) && !asked(r), r.text);
  await canaryCalls("ask", tool, true);
  r = await tool("run_program", { Action: "restart", Wait: 2 });
  check("a new run (restart): the run itself isn't asked (Allow for This Session)", !r.error && !asked(r), r.text);
  r = await layoutWhenUp(tool);
  check("…but looking at it asks again: Allow Once lasted the run", r.asked && r.text.includes("b7 (RButton)"), r.text);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Ask before changes: nothing left waiting", end.lines["mcpask.visible"] === "0", JSON.stringify(end.lines));
  checkTrail("ask", end, run.child.pid);
}

// ---- Allow edits: looking and using the program ask -------------------------------------
if (want("edits")) {
  const PORT = BASE_PORT + 2;
  // (the run: Allow Once; the typing: Deny; the next look: Allow Once)
  const events = ["mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", until("get_problems")].join(",");
  const run = studio("edits", `mcp:mode=edits,mcp:on=${PORT}`, events, "mcpmodebox.text,mcplog.items");
  check("Allow edits: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Allow edits: the run allowed once", !r.error, r.text);
  r = await tool("app_type", { Label: "Name", Text: CANARY_TYPED });
  check("Allow edits: app_type asks, like a run (denied here)", r.error && r.text.includes("didn't allow") && asked(r), r.text);
  r = await layoutWhenUp(tool);
  check("a Deny grants nothing: the next look asks again (allowed once here)", r.asked && r.text.includes("b7 (RButton)"), r.text);
  check("the denied typing never reached the program", !r.text.includes(CANARY_TYPED), r.text);
  r = await tool("app_click", { Caption: "7" });
  check("then using it isn't asked in this run", !r.error && !asked(r), r.text);
  await canaryCalls("edits", tool, true);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Allow edits: the menu says it", end.lines["mcpmodebox.text"] === "Allow edits", JSON.stringify(end.lines));
  check("the pane: the denied typing said as a count, denied", paneText(end.out).includes(`Typing ${CANARY_TYPED.length} characters into the box beside "Name" in the running program  — denied by you`), paneText(end.out));
  checkTrail("edits", end, run.child.pid);
}

// ---- Read only: the user runs it; the AI app neither looks at nor uses it -----------------
if (want("read")) {
  const PORT = BASE_PORT + 3;
  const run = studio("read", `mcp:mode=read,mcp:on=${PORT},run.start`, until("get_problems"), "mcpmodebox.text,mcplog.items");
  check("Read only: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  // (app_wait only lets time pass: it works in every mode — here, once the
  // program is up)
  let r = { error: true, text: "" };
  for (let i = 0; i < 40 && r.error; i++) {
    r = await tool("app_wait", { Ms: 100 });
    if (r.error) await sleep(250);
  }
  check("Read only: app_wait works", !r.error && r.text.includes("Waited"), r.text);
  for (const [t, a] of [["app_layout", {}], ["app_screenshot", {}], ["app_click", { Caption: "7" }], ["app_type", { Label: "Name", Text: "x" }], ["app_key", { Keys: "Enter" }], ["app_drag", { Name: "B7", ToX: 5, ToY: 5 }], ["app_scroll", { Name: "B7" }]]) {
    r = await tool(t, a);
    check(`Read only: ${t} refused, in the mode's words`, r.error && r.text.includes("Read only") && !png(r), r.text);
  }
  await canaryCalls("read", tool, false);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Read only: the menu says it", end.lines["mcpmodebox.text"] === "Read only", JSON.stringify(end.lines));
  checkTrail("read", end, run.child.pid);
}

// ---- Allow for This Session: kept across runs, cleared by another project ------------------
if (want("session")) {
  const PORT = BASE_PORT + 5;
  // (the run and the first look: Allow for This Session; then, in another
  // project, opening it, its run and its first look: Allow Once each)
  const events = ["mcpask.__until_Visible_1", "mcpsessionbtn.onclick", "mcpask.__until_Visible_1", "mcpsessionbtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", until("get_problems")].join(",");
  const run = studio("session", `mcp:on=${PORT}`, events, "mcpask.visible");
  check("Session: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Session: the run allowed for this session", !r.error && asked(r), r.text);
  r = await layoutWhenUp(tool);
  check("Session: the first look asked (allowed for this session)", r.asked && r.text.includes("b7 (RButton)"), r.text);
  r = await tool("run_program", { Action: "restart", Wait: 2 });
  r = await layoutWhenUp(tool);
  check("Session: the next run isn't asked about (Allow for This Session)", !r.asked && r.text.includes("b7 (RButton)"), r.text);
  r = await tool("run_program", { Action: "stop", Wait: 2 });
  check("Session: the program stopped (not asked: Allow for This Session)", !r.error && !asked(r), r.text);
  r = await tool("open_project", { Path: join(ROOT, "examples", "gui", "hello_form.rr") });
  check("Session: another project opened (asked: Allow Once)", !r.error && asked(r), r.text);
  r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Session: …its run is asked again (the session's grants went with the project)", !r.error && asked(r), r.text);
  let wasAsked = false;
  for (let i = 0; i < 40; i++) {
    r = await tool("app_layout");
    wasAsked ||= asked(r);
    if (r.text.includes("greetbutton (RButton)")) break;
    await sleep(250);
  }
  check("Session: …and so is looking at it", wasAsked && r.text.includes("greetbutton (RButton)"), r.text);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Session: nothing left waiting", end.lines["mcpask.visible"] === "0", JSON.stringify(end.lines));
}

// ---- Upgrade: confirmed before the running program could be used — asked once more ------
if (want("upgrade")) {
  const PORT = BASE_PORT + 6;
  const project = join(WORK, "upgrade");
  cpSync(join(ROOT, "tests", "fixtures", "app_eyes"), project, { recursive: true });
  // (a user's settings as an earlier Studio kept them: AI apps on and
  // confirmed, this project in Allow everything, no consent version)
  const seed = (file) => writeFileSync(file, [
    "Windows Registry Editor Version 5.00", "",
    "[HKEY_CURRENT_USER]", "", "[HKEY_CURRENT_USER\\Software]", "", "[HKEY_CURRENT_USER\\Software\\RapidR]", "",
    "[HKEY_CURRENT_USER\\Software\\RapidR\\Studio]",
    `"McpOn"="1"`, `"McpPort"="${PORT}"`, `"McpConfirmed"="1"`, `"McpModes"="${join(project, "Calc.rrproj")}?all"`, "",
  ].join("\r\n"));
  const reg = (file) => (existsSync(file) ? readFileSync(file, "utf8") : "");
  const value = (text, name) => (text.match(new RegExp(`"${name}"="([^"]*)"`)) || [])[1];
  const wasListening = async (child) => {
    // (polled while that Studio runs)
    let seen = false;
    while (child.exitCode === null && !seen) {
      seen = await new Promise((resolve) => {
        const so = net.connect({ host: "127.0.0.1", port: PORT }, () => { so.end(); resolve(true); });
        so.on("error", () => resolve(false));
      });
      await sleep(200);
    }
    return seen;
  };

  // Cancel: AI apps off, the server never started
  const regNo = join(WORK, "upgrade-no.reg");
  seed(regNo);
  let run = studio("upgrade-no", "", ["consentdialog.__until_Visible_-1", "consentno.onclick", "form.__wait_1500"].join(","), "consenttitle.caption,mcpmodebox.text", { RAPIDR_REGISTRY: regNo }, { project, fresh: false });
  const listenedNo = await wasListening(run.child);
  let end = await run.ended();
  check("Upgrade: the question once more at the start", end.lines["consenttitle.caption"] === "Let AI apps use RapidR Studio?", JSON.stringify(end.lines) + end.out.slice(-800));
  check("Upgrade, Cancel: the server never started", !listenedNo);
  check("Upgrade, Cancel: AI apps off, kept so", value(reg(regNo), "McpOn") === "0", reg(regNo).slice(-600));
  check("Upgrade: the project's Allow everything set back to Ask before changes", /Calc\.rrproj\?ask/.test(value(reg(regNo), "McpModes") || "") && !/\?all/.test(value(reg(regNo), "McpModes") || ""), value(reg(regNo), "McpModes"));

  // Turn On: the server, with the running program's tools
  const regYes = join(WORK, "upgrade-yes.reg");
  seed(regYes);
  run = studio("upgrade-yes", "", ["consentdialog.__until_Visible_-1", "consentyes.onclick", until("get_problems")].join(","), "consenttitle.caption,mcpmodebox.text", { RAPIDR_REGISTRY: regYes }, { project, fresh: false });
  check("Upgrade, Turn On: Studio listens", await listening(PORT));
  let { call, tool } = client(PORT);
  let names = ((await call("tools/list")).json.result?.tools || []).map((t) => t.name);
  check("Upgrade, Turn On: the app_* tools are there", ["app_layout", "app_screenshot", "app_click", "app_type", "app_wait"].every((t) => names.includes(t)), names.join(","));
  await tool("get_problems");
  end = await run.ended();
  check("Upgrade, Turn On: the project asks before changes now", end.lines["mcpmodebox.text"] === "Ask before changes" && /Calc\.rrproj\?ask/.test(value(reg(regYes), "McpModes") || ""), JSON.stringify(end.lines));
  check("Upgrade, Turn On: the confirmation kept (version 2)", value(reg(regYes), "McpConsentVersion") === "2" && value(reg(regYes), "McpConfirmed") === "1", reg(regYes).slice(-600));

  // The next start: not asked again
  run = studio("upgrade-again", "", until("get_problems"), "consenttitle.caption,mcpmodebox.text", { RAPIDR_REGISTRY: regYes }, { project, fresh: false });
  check("Upgrade, the next start: Studio listens at once", await listening(PORT));
  ({ call, tool } = client(PORT));
  names = ((await call("tools/list")).json.result?.tools || []).map((t) => t.name);
  check("Upgrade, the next start: the app_* tools there", names.includes("app_click"), names.join(","));
  await tool("get_problems");
  end = await run.ended();
  check("Upgrade, the next start: not asked again", !end.lines["consenttitle.caption"], JSON.stringify(end.lines));
}

// ---- RAI: the same eyes and hands, asked once a run ---------------------------------------
if (want("rai")) {
  const PORT = BASE_PORT + 4;
  const AI_READY = `ai:on,ai:provider=compatible,ai:base=${MOCK_URL},ai:model=mock-small`;
  // (RAI's card answered: Don't allow the first time, Allow for this run
  // the second — each step waits until RAI asks)
  const run = studio("rai", [AI_READY, "mcp:mode=all", `mcp:on=${PORT}`, "ai:applink=deny", "ai:applink=allow"].join(","), until("get_problems"), "", { RAPIDR_AI_TEST_SERVER: MOCK_URL.replace(/\/v1$/, "") });
  check("RAI: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("RAI: the program runs", !r.error, r.text);
  await layoutWhenUp(tool);
  r = await tool("ask_rai", { Prompt: "click 8 in the program" });
  check("RAI asked the user first, and was told they didn't allow it", !r.error && r.text.includes("didn't allow you to look at or use their running program"), r.text);
  r = await tool("app_layout");
  check("…so nothing was clicked: the display shows 0", r.text.includes('display (RLabel) "0"'), r.text);
  r = await tool("ask_rai", { Prompt: "click 7 in the program" });
  check("asked again in a new answer and allowed for this run: RAI read the layout and clicked 7 (its turn waited for the program)", !r.error && r.text.includes('The program: Clicked b7 (RButton "7") in form1 ("Calculator")'), r.text);
  check("RAI's steps name what it did", r.text.includes("Reading the running program's layout") || r.text.includes("Clicking"), r.text);
  r = await tool("app_layout");
  check("the program shows 7", r.text.includes('display (RLabel) "7"'), r.text);
  r = await tool("get_rai_transcript", { Chars: 30000 });
  check("RAI's chat shows each answer to its card", r.text.includes("You didn't allow RAI to use your running program") && r.text.includes("You allowed RAI to use your running program for this run"), r.text.slice(-1500));
  r = await tool("get_problems");
  const end = await run.ended();
  check("RAI: Studio ended", end.ok);
  const audit = existsSync(join(WORK, "rai-audit.jsonl")) ? readFileSync(join(WORK, "rai-audit.jsonl"), "utf8") : "";
  const raiTools = audit.split("\n").filter((l) => l.includes('"op":"tool"') && !l.includes('"source":"mcp"')).map((l) => JSON.parse(l)).filter((l) => String(l.tool).startsWith("app_"));
  check("RAI's audit: the refused click not done, the click that ran said so — no digest of what it was asked", raiTools.some((l) => l.tool === "app_click" && l.decision === "not done in your program") && raiTools.some((l) => l.tool === "app_click" && l.decision === "ran in your program") && raiTools.filter((l) => l.risk === "ui").every((l) => l.digest === ""), JSON.stringify(raiTools));
}

console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
