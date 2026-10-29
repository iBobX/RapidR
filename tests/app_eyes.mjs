// RapidR Studio's eyes and hands on the program it runs, driven by an AI
// app over Studio's MCP server (ide/mcp.inc, ide/mcpapp.inc): Studio
// started headless with AI apps on (the test step mcp:on — only with
// RAPIDR_TEST_MCP_TOKEN), a small calculator project open
// (tests/fixtures/app_eyes); the program Studio runs stays up for a minute
// (RAPIDR_TEST_PROGRAM_EVENTS: its own GUI test waits), and this script is
// the AI app, in each permission mode:
//
//   Allow everything  app_layout lists the buttons and captions; app_click
//                     by caption and by name (7 + 8 = 15), app_type into
//                     the box beside a label, Hello greets, app_key; a
//                     picture of the window (and one with the pointer
//                     gliding); what isn't there said with what is; another
//                     window can't be asked for; paused in the debugger a
//                     picture still, a click refused with where; going on,
//                     input again; app_wait
//   Ask before changes  the first app_click asks (Allow Once), the next in
//                     the same run doesn't
//   Allow edits       app_click asks (denied here)
//   Read only         the user runs it; app_layout and app_screenshot
//                     work, app_click is refused
//
//   node tests/app_eyes.mjs [all|ask|edits|read …]   (RAPIDR: the rapidr)
//
// Never the user's keychain, registry, printer or AI keys: everything in
// tests/results/app-eyes. Ports 47905-47909.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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

function studio(name, steps, events, dump, extra) {
  cpSync(join(ROOT, "tests", "fixtures", "app_eyes"), join(WORK, name), { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", steps, join(WORK, name, "Calc.rrproj")];
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

/// app_layout until the program's window is there (it starts in a moment).
async function layoutWhenUp(tool) {
  let r = { text: "" };
  for (let i = 0; i < 40; i++) {
    r = await tool("app_layout");
    if (r.text.includes("b7 (RButton)")) return r;
    await sleep(250);
  }
  return r;
}

// ---- Allow everything: the whole tour ----------------------------------------------------
if (want("all")) {
  const PORT = BASE_PORT;
  const run = studio("all", ["mcp:mode=all", `mcp:on=${PORT}`].join(","), until("get_problems"), "mcplog.itemcount", { RAPIDR_SCALE: "2" });
  check("Studio listens for AI apps", await listening(PORT));
  const { call, tool } = client(PORT);
  const list = (await call("tools/list")).json.result?.tools || [];
  const names = list.map((t) => t.name);
  for (const t of ["app_layout", "app_screenshot", "app_click", "app_type", "app_key", "app_drag", "app_scroll", "app_wait"]) check(`tools/list has ${t}`, names.includes(t), names.join(","));
  check("app_screenshot and app_layout are read-only, app_click isn't", list.find((t) => t.name === "app_screenshot")?.annotations?.readOnlyHint === true && list.find((t) => t.name === "app_layout")?.annotations?.readOnlyHint === true && list.find((t) => t.name === "app_click")?.annotations?.readOnlyHint === false);
  check("each tool's description has an example call", list.filter((t) => t.name.startsWith("app_")).every((t) => t.description.includes("Example")), JSON.stringify(list.filter((t) => t.name.startsWith("app_")).map((t) => t.description)));

  let r = await tool("app_layout");
  check("app_layout before a run: run it first", r.error && r.text.includes("The program isn't running: run it first (run_program)"), r.text);
  r = await tool("run_program", { Action: "start", Debug: true, Wait: 2 });
  check("run_program: it runs (with the debugger)", !r.error && r.text.includes("running"), r.text);

  r = await layoutWhenUp(tool);
  check("app_layout: the window and its buttons with their captions", !r.error && r.text.includes('Window form1 "Calculator" (RForm), inside 240 × 196, in front') && r.text.includes('b7 (RButton) "7" at 10,40 size 50×30') && r.text.includes('bplus (RButton) "+"') && r.text.includes("nameedit (REdit)"), r.text);
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
  check("app_screenshot at 1x, its inside only: 240 × 196", r.text.includes("240 × 196 pixel picture"), r.text);

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
  check("app_click on what isn't there: the buttons there are", r.error && r.text.includes('No button (or other control) shows "Go" in form1. Buttons now: "7", "8", "9", "+", "=", "C", "Hello"'), r.text);
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
  r = await tool("get_problems");
  const end = await run.ended();
  check("Studio ended after the tour", end.ok);
  check("the pane lists the AI app's calls", Number(end.lines["mcplog.itemcount"]) >= 20, end.lines["mcplog.itemcount"]);
  const shot = join(WORK, "all-window-1.bmp");
  if (existsSync(shot)) spawnSync("sips", ["-s", "format", "png", shot, "--out", join(WORK, "app-eyes-studio.png")], { stdio: "ignore" });
}

// ---- Ask before changes: asked once per run -----------------------------------------------
if (want("ask")) {
  const PORT = BASE_PORT + 1;
  // (Allow Once on the run, Allow Once on the first click; the second click isn't asked)
  const events = ["mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcponcebtn.onclick", until("get_problems")].join(",");
  const run = studio("ask", `mcp:on=${PORT}`, events, "mcpask.visible,mcpasktext.caption");
  check("Ask before changes: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Ask before changes: the run allowed once", !r.error, r.text);
  await layoutWhenUp(tool);
  r = await tool("app_click", { Caption: "7" });
  check("the first app_click waited for the user, then clicked", !r.error && r.text.startsWith("Clicked b7") && r.raw.progress.some((p) => JSON.stringify(p).includes("Waiting for the user")), r.text + JSON.stringify(r.raw.progress));
  r = await tool("app_click", { Caption: "8" });
  check("the next app_click in the same run isn't asked", !r.error && r.text.startsWith("Clicked b8") && !r.raw.progress.some((p) => JSON.stringify(p).includes("Waiting for the user")), r.text + JSON.stringify(r.raw.progress));
  r = await tool("app_screenshot");
  check("looking never asks", !r.error && isPng(png(r)) && r.raw.progress.length === 0, r.text);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Ask before changes: nothing left waiting", end.lines["mcpask.visible"] === "0", JSON.stringify(end.lines));
}

// ---- Allow edits: using the program asks ------------------------------------------------
if (want("edits")) {
  const PORT = BASE_PORT + 2;
  const events = ["mcpask.__until_Visible_1", "mcponcebtn.onclick", "mcpask.__until_Visible_1", "mcpdenybtn.onclick", until("get_problems")].join(",");
  const run = studio("edits", `mcp:mode=edits,mcp:on=${PORT}`, events, "mcpmodebox.text");
  check("Allow edits: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await tool("run_program", { Action: "start", Wait: 2 });
  check("Allow edits: the run allowed once", !r.error, r.text);
  await layoutWhenUp(tool);
  r = await tool("app_click", { Caption: "7" });
  check("Allow edits: app_click asks, like a run (denied here)", r.error && r.text.includes("didn't allow"), r.text);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Allow edits: the menu says it", end.lines["mcpmodebox.text"] === "Allow edits", JSON.stringify(end.lines));
}

// ---- Read only: the user runs it; the AI app only looks ----------------------------------
if (want("read")) {
  const PORT = BASE_PORT + 3;
  const run = studio("read", `mcp:mode=read,mcp:on=${PORT},run.start`, until("get_problems"), "mcpmodebox.text");
  check("Read only: Studio listens", await listening(PORT));
  const { tool } = client(PORT);
  let r = await layoutWhenUp(tool);
  check("Read only: app_layout works", !r.error && r.text.includes('b7 (RButton) "7"'), r.text);
  r = await tool("app_screenshot");
  check("Read only: app_screenshot works", !r.error && isPng(png(r)), r.text);
  r = await tool("app_click", { Caption: "7" });
  check("Read only: app_click refused, in the mode's words", r.error && r.text.includes("Read only"), r.text);
  r = await tool("app_type", { Label: "Name", Text: "x" });
  check("Read only: app_type refused", r.error && r.text.includes("Read only"), r.text);
  r = await tool("get_problems");
  const end = await run.ended();
  check("Read only: the menu says it", end.lines["mcpmodebox.text"] === "Read only", JSON.stringify(end.lines));
}

console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
