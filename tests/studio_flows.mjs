// RapidR Studio's flows on both hosts (docs/ide-plan.md I1): the shell
// driven through its own commands (`--do`, the `do` parameter on the web)
// on the desktop's headless host and on the web page, the same properties
// read on both (RAPIDR_TEST_DUMP) — open an example, run it (in its own
// process on the desktop, a sandboxed frame on the web) and read its
// output, the outline and problems from the language service, the palette,
// a theme switch, the document mode, a project saved and opened again.
//
//   tools/build_studio_web.sh
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_flows.mjs [filter…]
//
// A case runs in RapidR light unless it names `themes` (each run is
// `<name>-<dark|contrast|light>`); a case with `contrast: true` has every
// text run of Studio's windows judged against what it's drawn on, on both
// hosts (rapidr_ui_kernel::contrast, from the very list captured: the
// desktop's RAPIDR_TEST_CONTRAST, the page's test results) — below WCAG AA
// (4.5:1, 3:1 for large and disabled text) fails.

import { spawn, spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { createServer } from "node:http";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
// (STUDIO_WEB_URL: each lane serves its own build on its own port)
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-flows");
const filters = process.argv.slice(2);
// (STUDIO_FLOWS_HOSTS=desktop or web: one host only)
const HOSTS = (process.env.STUDIO_FLOWS_HOSTS || "desktop,web").split(",");
// (the save prompts' project: a file and the one it includes)
const SAVE_FILES = ["tests/fixtures/studio_save/main.rr", "tests/fixtures/studio_save/other.inc"];
// (Preview 4 S1/S2: the seven-file project, three forms in ui.rr)
// (Preview 4 S3/S5: four files, a global in each, a desktop-only component)
const SEARCH_FILES = ["Counter.rrproj", "main.rr", "globals.rr", "server.rr", "ui.rr", "notes.txt"].map((f) => `tests/fixtures/studio_search/${f}`);
// (S3: a project file gone stale — tools.rr listed as a form, window.rr as a
// module; the code says the opposite)
const STALE_FILES = ["Stale.rrproj", "main.rr", "tools.rr", "window.rr"].map((f) => `tests/fixtures/studio_stale/${f}`);
// (S5: 26 warnings and, on the web, a note: 27 problems)
const PROBLEM_FILES = ["Problems.rrproj", "main.rr", "a.rr", "b.rr", "net.rr"].map((f) => `tests/fixtures/studio_problems/${f}`);
const SHELF_FILES = ["Shelf.rrproj", "main.rr", "globals.rr", "store.rr", "ui.rr", "lists.rr", "prices.rr", "report.rr"].map((f) => `tests/fixtures/studio_forms/${f}`);
// (WEB-OPEN: a project of three files, three forms in ui.rr, a desktop-only component)
const WEB_OPEN_FILES = ["Kiosk.rrproj", "main.rr", "globals.rr", "ui.rr"];
// (PROJECT-PROPS: two files, a form and a module; the main file names no look)
const PROJPROPS_FILES = ["RenMany.rrproj", "main.rr", "helpers.rr"].map((f) => `tests/fixtures/studio_projprops/${f}`);
// (I8) The AI assistant's cases talk to a mock provider in a process of its
// own (tests/ai_mock_server.mjs: both wire formats, scripted answers, no
// key, no network, no cost): {mock} in a case's steps is its base URL,
// {mockroot} its root (a scenario's address: {mockroot}/s/drop/v1).
const mockProc = spawn(process.execPath, [join(HERE, "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
const MOCK_URL = await new Promise((resolve) => mockProc.stdout.once("data", (d) => resolve(String(d).trim())));
// (the cloud providers — Anthropic, OpenAI, DeepSeek — played by the same
// mock: RAPIDR_AI_TEST_SERVER sends them there, a loopback address only;
// the fake key typed decides what it answers: tests/ai_mock_server.mjs)
const MOCK_ROOT = MOCK_URL.replace(/\/v1$/, "");

// (I4) The designer's steps on notepad.bas: the placing tool's click, the
// form's right edge dragged, Button1 dragged (the form sits 24 px in on the
// surface's backdrop).
const DESIGN_STEPS = [
  "__mousedown_112_132", "__mouseup_112_132",
  "__mousedown_503_262", "__mousemove_533_262", "__mousemove_563_262", "__mouseup_563_262",
  "__mousedown_122_142", "__mousemove_142_162", "__mousemove_162_182", "__mouseup_162_182",
].map((e) => `designdoc(0).${e}`).join(",");

// (the web) The running program's windows float over the whole page, as
// the desktop's are windows of their own: its window dragged by its title
// bar (real mouse input) to each edge of the page lands there whole —
// Studio's frame shows every pixel of it (the clip covers it, the corners
// hit the program), a point beside it still hits Studio, the whole frame
// shows while the button is held. A capture at each edge.
async function floatingWindows(page, scale, record) {
  const vp = page.viewportSize();
  const frame = () => page.frames().find((f) => f.url().endsWith("/run.html"));
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForTimeout(500);
  const win = async () => frame().evaluate(() => {
    const r = document.querySelector(".rr-kwin").getBoundingClientRect();
    return { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) };
  });
  const hit = (x, y) => page.evaluate(([x, y]) => {
    const el = document.elementFromPoint(x, y);
    return el && el.tagName === "IFRAME" ? "program" : "studio";
  }, [x, y]);
  const first = await win();
  record("", first.w > 100 && first.h > 100 && first.x > 0 && first.y > 0, `the program's window shows on the page, centred (${JSON.stringify(first)})`);
  const edges = {
    left: (w) => ({ x: 0, y: w.y }),
    right: (w) => ({ x: vp.width - w.w, y: w.y }),
    top: (w) => ({ x: w.x, y: 0 }),
    bottom: (w) => ({ x: w.x, y: vp.height - w.h }),
  };
  for (const [edge, to] of Object.entries(edges)) {
    const w = await win();
    const target = to(w);
    // (the title bar's middle, then the mouse moved by as much as the window should)
    const from = { x: w.x + Math.round(w.w / 2), y: w.y + 12 };
    await page.mouse.move(from.x, from.y);
    await page.mouse.down();
    const steps = 8;
    for (let i = 1; i <= steps; i++) {
      await page.mouse.move(from.x + ((target.x - w.x) * i) / steps, from.y + ((target.y - w.y) * i) / steps);
      if (i === steps / 2) {
        const clip = await page.evaluate(() => document.querySelector("#studio-run iframe").style.clipPath);
        record("", clip === "none", `${edge}: while dragging the whole frame shows (clip ${clip})`);
      }
    }
    await page.mouse.up();
    await page.waitForTimeout(400);
    const now = await win();
    record("", Math.abs(now.x - target.x) <= 2 && Math.abs(now.y - target.y) <= 2, `${edge}: the window is at the edge (${JSON.stringify(now)}, wanted ${JSON.stringify(target)})`);
    const inside = now.x >= 0 && now.y >= 0 && now.x + now.w <= vp.width && now.y + now.h <= vp.height;
    const corners = [[now.x + 3, now.y + 3], [now.x + now.w - 4, now.y + 3], [now.x + 3, now.y + now.h - 4], [now.x + now.w - 4, now.y + now.h - 4]];
    const hits = await Promise.all(corners.map(([x, y]) => hit(x, y)));
    record("", inside && hits.every((h) => h === "program"), `${edge}: nothing of it is cut off (corners: ${hits.join(", ")})`);
    // (a point beside the window: Studio's)
    const beside = edge === "right" ? [now.x - 40, now.y + now.h / 2] : edge === "bottom" ? [now.x + now.w / 2, now.y - 40] : edge === "left" ? [now.x + now.w + 40, now.y + now.h / 2] : [now.x + now.w / 2, now.y + now.h + 40];
    record("", (await hit(beside[0], beside[1])) === "studio", `${edge}: Studio gets the clicks beside it`);
    await page.screenshot({ path: join(WORK, `program-windows-${edge}@${scale}x.png`) });
  }
}

// (the web) A program window bigger than the page (tests/fixtures/
// studio_debug/big_window.rr, 1200 x 800, centred, on a 1280 x 720 page):
// nothing of it is ever out of reach. Shown with its title bar on the page
// (a window is never placed or dragged above or left of the page's
// corner); the program's desktop scrolls — its scroll bar along the page's
// edge, the wheel over the title bar — to its bottom corners; dragged past
// the page's right and bottom edges, both bars, their thumbs dragged to the
// end show its far corner; maximized, it fits the page and the bars go.
// On a page it fits (1920 x 1080), all of it shows at once, no bar. A
// capture at each step.
async function bigWindow(page, scale, record) {
  const vp = page.viewportSize();
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForTimeout(600);
  const frame = () => page.frames().find((f) => f.url().endsWith("/run.html"));
  const win = async () => frame().evaluate(() => {
    const r = document.querySelector(".rr-kwin").getBoundingClientRect();
    const bars = [...document.querySelectorAll(".rr-desk-bar")].filter((b) => b.style.display !== "none").map((b) => {
      const t = b.querySelector(".thumb").getBoundingClientRect();
      return { dir: b.classList.contains("v") ? "v" : "h", x: Math.round(t.left + t.width / 2), y: Math.round(t.top + t.height / 2) };
    });
    return { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height), bars };
  });
  const corners = (w) => [[w.x + 3, w.y + 3], [w.x + w.w - 4, w.y + 3], [w.x + 3, w.y + w.h - 4], [w.x + w.w - 4, w.y + w.h - 4]];
  const onPage = ([x, y]) => x >= 0 && y >= 0 && x < vp.width && y < vp.height;
  const hits = async (pts) => Promise.all(pts.map(([x, y]) => hitPage(page, x, y)));
  const shot = (step) => page.screenshot({ path: join(WORK, `program-window-big-${vp.width}x${vp.height}-${step}@${scale}x.png`) });
  let w = await win();
  if (w.w <= vp.width && w.h <= vp.height) {
    // (it fits: all of it at once, no scroll bar)
    const h = await hits(corners(w));
    record("", w.x >= 0 && w.y >= 0 && w.x + w.w <= vp.width && w.y + w.h <= vp.height && h.every((x) => x === "program") && !w.bars.length, `${vp.width}x${vp.height}: the 1200 x 800 window shows whole, no scroll bar (${JSON.stringify(w)}; corners ${h.join(", ")})`);
    await shot("whole");
    return;
  }
  record("", w.x >= 0 && w.y === 0 && w.w === 1200 && w.h === 800, `shown with its title bar on the page, not above it (${JSON.stringify({ x: w.x, y: w.y, w: w.w, h: w.h })})`);
  record("", (await hitPage(page, w.x + 200, w.y + 12)) === "program" && w.bars.some((b) => b.dir === "v"), `its title bar is the program's, and the desktop's scroll bar shows (${JSON.stringify(w.bars)})`);
  await shot("shown");
  // (the wheel over its title bar scrolls the desktop; on over the scroll
  // bar, once the title bar has moved from under the mouse: to its bottom)
  await page.mouse.move(w.x + 200, w.y + 12);
  await page.mouse.wheel(0, 40);
  await page.waitForTimeout(300);
  const y0 = w.y;
  w = await win();
  record("", w.y < y0 && w.y >= y0 - 40, `the wheel over the title bar scrolls the desktop (y ${y0} → ${w.y})`);
  await page.mouse.move(vp.width - 6, Math.round(vp.height / 2));
  for (let i = 0; i < 4; i++) await page.mouse.wheel(0, 100);
  await page.waitForTimeout(400);
  w = await win();
  const bottom = [[w.x + 3, w.y + w.h - 4], [w.x + w.w - 4, w.y + w.h - 4]];
  let h = await hits(bottom);
  record("", Math.abs(w.y + w.h - vp.height) <= 1 && bottom.every(onPage) && h.every((x) => x === "program"), `scrolled to the end, its bottom corners show (${JSON.stringify({ y: w.y, bottom: w.y + w.h })}; ${h.join(", ")})`);
  await shot("scrolled");
  // (back up with the wheel over the scroll bar, then the title bar dragged
  // past the page's top left corner: kept at it)
  await page.mouse.move(vp.width - 6, Math.round(vp.height / 2));
  await page.mouse.wheel(0, -1000);
  await page.waitForTimeout(300);
  w = await win();
  record("", w.y === 0, `the wheel over the scroll bar scrolls back up (y ${w.y})`);
  const from = { x: w.x + 300, y: w.y + 12 };
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  for (let i = 1; i <= 8; i++) await page.mouse.move(from.x - 60 * i, from.y - 40 * i);
  await page.mouse.up();
  await page.waitForTimeout(400);
  w = await win();
  record("", w.x === 0 && w.y === 0 && (await hitPage(page, 300, 12)) === "program", `dragged past the page's corner, it stays at it, its title bar on the page (${w.x}, ${w.y})`);
  await shot("corner");
  // (dragged past the right and bottom edges: both bars; their thumbs to the end)
  await page.mouse.move(300, 12);
  await page.mouse.down();
  for (let i = 1; i <= 8; i++) await page.mouse.move(300 + 50 * i, 12 + 30 * i);
  await page.mouse.up();
  await page.waitForTimeout(400);
  w = await win();
  record("", w.x === 400 && w.y === 240 && w.bars.length === 2, `dragged past the right and bottom edges: the desktop grows, two scroll bars (${JSON.stringify(w)})`);
  for (const b of w.bars) {
    await page.mouse.move(b.x, b.y);
    await page.mouse.down();
    await page.mouse.move(b.dir === "v" ? b.x : b.x + vp.width, b.dir === "v" ? b.y + vp.height : b.y, { steps: 6 });
    await page.mouse.up();
  }
  await page.waitForTimeout(400);
  w = await win();
  const far = [w.x + w.w - 4, w.y + w.h - 4];
  record("", onPage(far) && (await hitPage(page, far[0], far[1])) === "program" && Math.abs(w.x + w.w - (vp.width - 12)) <= 1 && Math.abs(w.y + w.h - (vp.height - 12)) <= 1, `the thumbs dragged to the end show its far corner, beside the bars (${JSON.stringify({ right: w.x + w.w, bottom: w.y + w.h })})`);
  await shot("bars");
  // (maximized — a double click on its title bar —: the page, no bar)
  if (w.y + 12 >= 0) await page.mouse.dblclick(w.x + 300, w.y + 12);
  else {
    await page.mouse.move(vp.width - 6, Math.round(vp.height / 2));
    await page.mouse.wheel(0, -2000);
    await page.waitForTimeout(300);
    w = await win();
    await page.mouse.dblclick(Math.max(w.x, 0) + 300, w.y + 12);
  }
  await page.waitForTimeout(500);
  w = await win();
  record("", w.x === 0 && w.y === 0 && w.w === vp.width && w.h === vp.height && !w.bars.length, `maximized, it fits the page and the scroll bars go (${JSON.stringify(w)})`);
  await shot("maximized");
}

// (the web) The inspector's rows as a screen reader gets them: the case
// inspector-read-only.
async function inspectorAccess(page, scale, record) {
  const cdp = await page.context().newCDPSession(page);
  const { nodes } = await cdp.send("Accessibility.getFullAXTree");
  const byId = new Map(nodes.map((n) => [n.nodeId, n]));
  const prop = (n, k) => (n.properties || []).find((p) => p.name === k)?.value?.value;
  const grid = nodes.find((n) => n.role?.value === "treegrid" && n.name?.value === "Properties");
  record("", !!grid, `the inspector is a tree grid named Properties (${grid ? "yes" : nodes.filter((n) => /grid/.test(n.role?.value || "")).map((n) => n.role.value + " " + n.name?.value).join(", ")})`);
  const rows = nodes.filter((n) => n.role?.value === "row").map((r) => ({ r, cells: (r.childIds || []).map((id) => byId.get(id)).filter((c) => c && /cell|rowheader/.test(c.role?.value || "")) }));
  const caption = rows.find(({ cells }) => cells[0]?.name?.value === "Caption");
  record("", caption && caption.r.name?.value === "Caption &Add to shelf" && caption.cells[0].role.value === "rowheader" && caption.cells[1]?.role?.value === "gridcell" && !prop(caption.cells[1], "readonly") && !/read-only/.test(caption.r.description?.value || ""),
    `an editable row is named by its cells, its value with it ("${caption?.r.name?.value}"), not read-only`);
  const fixed = rows.filter(({ r }) => /(^|, )read-only$/.test(r.description?.value || ""));
  const allCells = fixed.every(({ cells }) => cells.length === 2 && prop(cells[1], "readonly") === true);
  record("", fixed.length > 0 && allCells, `a read-only row says "read-only" (announced with it) and its value's cell is read-only (${fixed.map(({ r }) => `"${r.name?.value}"`).join(", ")})`);
  // (the categories are no properties: no cells, never read-only)
  const category = rows.find(({ r }) => r.name?.value === "Appearance");
  record("", category && category.cells.length === 0 && !/read-only/.test(category.r.description?.value || ""), `a category's row has no cells and isn't read-only ("${category?.r.description?.value}")`);
  await cdp.detach();
}

// (DEBUG-RUN) cmds:report's line for a state: each Run / Debug command's
// menu item, tool button and palette entry (1 on, 0 off, - none), then the
// Run item's caption and the ▶ button's hint (the desktop's Build last)
const CMDS = ["run.start", "run.withoutDebugging", "run.stop", "run.restart", "run.continue", "run.pause", "debug.showProgram", "debug.stepInto", "debug.stepOver", "debug.stepOut", "debug.runToCursor", "debug.setNextStatement", "debug.stepIntoSpecific", "debug.toggleBreakpoint"];
const CMD_STATES = {
  idle: "111 111 000 0-0 0-0 000 0-0 111 111 000 1-1 0-0 0-0 111",
  running: "000 000 111 1-1 0-0 111 1-1 000 000 000 0-0 0-0 0-0 111",
  paused: "111 000 111 1-1 1-1 000 1-1 111 111 111 1-1 1-1 1-1 111",
};
const cmdsLine = (state, kind) => {
  const f = CMD_STATES[kind].split(" ");
  const run = kind === "paused" ? "&Continue \\| Continue \\(F5\\)" : "&Run \\| Run \\(F5\\)";
  return `\\[cmds\\] ${state} ` + CMDS.map((id, i) => `${id.replace(/\./g, "\\.")}=${f[i]}`).join(" ") + `( run\\.build=111)? \\| ${run}`;
};
const cmdsSeq = (...lines) => new RegExp(lines.join("[\\s\\S]*"));

// (DEBUG-RUN, the debugger's events) Robert's scenarios 1–6
// (docs/manual/debugging.md "Debugging events"): a breakpoint in a
// handler, the event triggered again and again — it stops every time,
// after Continue (F5) and after a Step Over out of the handler (F10).
// The program's own input comes from its test script
// (RAPIDR_TEST_PROGRAM_EVENTS: Studio passes it on as the program's
// RAPIDR_TEST_EVENTS, as any RAPIDR_TEST_PROGRAM_<X> as its
// RAPIDR_TEST_<X>), which waits while the program is stopped, so each
// trigger lands after Studio has gone on. `stops:report` lists the run's
// stops, line and reason.
const EVENTS_FILE = "tests/fixtures/studio_debug/events.rr";
// (the program's script: `n` times `trigger`, a moment of other input
// between them — moves over the Click button, which handles none)
const triggers = (trigger, n, gap = 2) => Array.from({ length: n }, () => [trigger, ...Array(gap).fill("btn.__mousemove_5_5")].join(",")).join(",");
// (Studio's side: the breakpoint on `line`, F5; then each stop answered —
// F5 twice, F10 twice (out of the handler: the second F10 does nothing
// once the program runs on), F5 for the rest)
const eventSteps = (line, after = "") => `line:${line},key:F9,key:F5,${"wait,".repeat(8)}key:F5,${"wait,".repeat(4)}key:F5,${"wait,".repeat(4)}key:F10,wait,wait,key:F10,${"wait,".repeat(4)}key:F5,${"wait,".repeat(4)}${after}stops:report`;
// ("[stops] L:breakpoint L:breakpoint L:breakpoint (L+1:step)? L:breakpoint L:breakpoint")
const stopsAt = (line) => new RegExp(`\\[stops\\] ${line}:breakpoint ${line}:breakpoint ${line}:breakpoint( ${line + 1}:step)? ${line}:breakpoint ${line}:breakpoint`);

// (S-DEBUG: the debugger's program and the file it includes)
const DEBUG_FILES = ["tests/fixtures/studio_debug/counter.rr", "tests/fixtures/studio_debug/tally.inc"];
// (E0–E2) The code's helpers' example: its picture beside it. Typing pauses
// a moment before the helpers come (the language service's), so the steps
// wait for them first.
const HELPERS_FILES = ["examples/studio/helpers.rr", "examples/studio/logo.svg"];
const HELPERS_READY = "focus:codedoc(0),wait,wait,wait,wait";
// (2.121.1, HLP-1) The Help pane's program: a variable of its own, a SUB, a
// component, a colour set on a font; and the workspace's version, which
// Help > About says
const HELP_FILES = ["tests/fixtures/studio_help/main.rr"];
const HELP_OPEN = "tests/fixtures/studio_help/main.rr";
const WORKSPACE_VERSION = readFileSync(join(ROOT, "Cargo.toml"), "utf8").match(/\[workspace\.package\][^[]*?version = "([^"]+)"/)[1];
const NO = /^(0|False)$/i;
const YES = /^(1|True|-1)$/i;
// (a long question: the chat's cards come well below the first screenful)
const LONG_ASK = Array(14).fill("alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa").join(" ") + ".";
// (an a11y check's test) The node is inside the assistant's chat as it shows
// now: its top and bottom within the conversation's own
const SHOWN_IN_CHAT = (n, nodes) => {
  const chat = nodes.find((x) => x.role === "document" && x.name === "Conversation with the assistant");
  return !!chat && n.bounds[1] >= chat.bounds[1] && n.bounds[1] + n.bounds[3] <= chat.bounds[1] + chat.bounds[3];
};
// (2.121.1) The node is inside the Help page's view as it shows now: its top
// and bottom within the view's own
const SHOWN_IN_HELP = (n, nodes) => {
  const page = nodes.find((x) => x.role === "document" && x.name === "Help page");
  return !!page && n.bounds[1] >= page.bounds[1] && n.bounds[1] + n.bounds[3] <= page.bounds[1] + page.bounds[3];
};
// (DARK-FIX) RapidR's three looks, for the cases that run in each
const LOOKS = ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"];
// (IMPORT-INC) A RapidQ program in several files: its includes in lib/ and forms/
const SHAPES = ["shapes.bas", "lib/geometry.inc", "lib/units.inc", "forms/shapesform.inc"].map((f) => `examples/rapidq/shapes/${f}`);
const INC_FIXTURE = ["tests/fixtures/rapidq_import_inc/main.bas", "tests/fixtures/rapidq_import_lib/inc/tools.inc"];

// (the web) Modal dialogs stay on top. The program's modal message is over
// its own form, whole over Studio and answered with Enter (programModal).
// Studio's own modal dialog (File > New Project, from the case's commands)
// opened while the program's window floats over Studio is over that window
// (studioModal: the frame stacks under Studio's dialogs).
const hitPage = (page, x, y) => page.evaluate(([x, y]) => {
  const el = document.elementFromPoint(x, y);
  if (!el) return "none";
  if (el.tagName === "IFRAME") return "program";
  const w = el.closest(".rr-kwin");
  return w ? "studio:" + w.getAttribute("data-rr-form") : "studio";
}, [x, y]);
const runFrame = (page) => page.frames().find((f) => f.url().endsWith("/run.html"));
const programWindows = (page) => runFrame(page).evaluate(() => [...document.querySelectorAll(".rr-kwin")].filter((w) => getComputedStyle(w).display !== "none").map((w) => {
  const r = w.getBoundingClientRect();
  return { form: w.getAttribute("data-rr-form"), z: Number(getComputedStyle(w).zIndex) || 0, x: r.left, y: r.top, w: r.width, h: r.height };
}));

async function programModal(page, scale, record) {
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForTimeout(800);
  const hitFrame = (x, y) => runFrame(page).evaluate(([x, y]) => {
    const el = document.elementFromPoint(x, y);
    const w = el && el.closest(".rr-kwin");
    return w ? w.getAttribute("data-rr-form") : "none";
  }, [x, y]);
  let list = await programWindows(page);
  record("", list.length === 2, `the program shows its form and its message (${list.map((w) => w.form).join(", ")})`);
  const top = [...list].sort((a, b) => b.z - a.z)[0];
  const form = list.find((w) => w !== top);
  const cx = Math.round(top.x + top.w / 2), cy = Math.round(top.y + top.h / 2);
  record("", form && top.z > form.z && (await hitFrame(cx, cy)) === top.form, `the message is over the program's form (z ${top.z} > ${form && form.z})`);
  const corners = [[top.x + 3, top.y + 3], [top.x + top.w - 4, top.y + top.h - 4]];
  const hits = await Promise.all(corners.map(([x, y]) => hitPage(page, x, y)));
  record("", hits.every((h) => h === "program"), `the message shows whole over Studio (corners: ${hits.join(", ")})`);
  await page.screenshot({ path: join(WORK, `program-modal@${scale}x.png`) });
  await page.mouse.click(cx, cy);
  await page.keyboard.press("Enter");
  await page.waitForTimeout(600);
  list = await programWindows(page);
  record("", list.length === 1, `Enter closed the message (${list.map((w) => w.form).join(", ")})`);
}

// (the web) Studio opens maximized on the page: it fills the page (not
// just the restore icon), one double click on its title bar restores it,
// another maximizes it again (Robert, 2026-10-16: it took two clicks).
async function studioStartsMaximized(page, scale, record) {
  const vp = page.viewportSize();
  const sel = 'body > .rr-kwin[data-rr-form="studio"]';
  await page.waitForSelector(sel, { timeout: 30000 });
  await page.waitForTimeout(800);
  const rect = () => page.evaluate((s) => {
    const r = document.querySelector(s).getBoundingClientRect();
    return [Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)];
  }, sel);
  const start = await rect();
  record("", start[0] === 0 && start[1] === 0 && start[2] === vp.width && start[3] === vp.height, `Studio fills the page at start (${start})`);
  await page.mouse.dblclick(300, 12);
  await page.waitForTimeout(600);
  const restored = await rect();
  record("", restored[2] < vp.width || restored[3] < vp.height, `one double click restores it (${restored})`);
  await page.mouse.dblclick(restored[0] + 200, restored[1] + 12);
  await page.waitForTimeout(600);
  const again = await rect();
  record("", JSON.stringify(again) === JSON.stringify(start), `the next maximizes it again (${again})`);
}

async function studioModal(page, scale, record) {
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForFunction(() => {
    const w = document.querySelector('body > .rr-kwin[data-rr-form="newdialog"]');
    return w && getComputedStyle(w).display !== "none";
  }, null, { timeout: 30000 });
  await page.waitForTimeout(500);
  const [prog] = await programWindows(page);
  const dialog = await page.evaluate(() => {
    const r = document.querySelector('body > .rr-kwin[data-rr-form="newdialog"]').getBoundingClientRect();
    return { x: r.left, y: r.top, w: r.width, h: r.height };
  });
  // (a point of the dialog over the program's window)
  const ox = Math.max(dialog.x, prog.x) + 6, oy = Math.max(dialog.y, prog.y) + 6;
  const overlaps = ox < Math.min(dialog.x + dialog.w, prog.x + prog.w) - 6 && oy < Math.min(dialog.y + dialog.h, prog.y + prog.h) - 6;
  record("", overlaps, `Studio's New Project dialog and the program's window overlap (${JSON.stringify(dialog)} / ${JSON.stringify(prog)})`);
  const hit = await hitPage(page, ox, oy);
  record("", hit === "studio:newdialog", `Studio's dialog is over the program's window where they overlap (${hit})`);
  // (and the program's window still over Studio's main window beside the dialog)
  const beside = [prog.x + 4, prog.y + prog.h - 4];
  const inDialog = beside[0] >= dialog.x && beside[0] < dialog.x + dialog.w && beside[1] >= dialog.y && beside[1] < dialog.y + dialog.h;
  if (!inDialog) record("", (await hitPage(page, beside[0], beside[1])) === "program", "the program's window is still over Studio's window");
  await page.screenshot({ path: join(WORK, `studio-modal-over-program@${scale}x.png`) });
}

// (S-DESIGN-2) Keys pressed on the designer: "Ab&" is Shift+A, B, Shift+7
// (a US keyboard, as the hosts' test hooks type them); {Enter} and the
// like by name.
const KEYS = { Enter: "13", Escape: "27", Tab: "9", F2: "113", "Ctrl+O": "79_16", "Ctrl+=": "187_16", "Ctrl+-": "189_16", "Ctrl+0": "48_16" };
function typed(text) {
  const out = [];
  for (const m of text.matchAll(/\{([^}]+)\}|(.)/g)) {
    if (m[1]) { out.push(KEYS[m[1]]); continue; }
    const c = m[2];
    if (/[a-z]/.test(c)) out.push(String(c.toUpperCase().charCodeAt(0)));
    else if (/[A-Z]/.test(c)) out.push(`${c.charCodeAt(0)}_256`);
    else if (/[0-9 ]/.test(c)) out.push(String(c.charCodeAt(0)));
    else if (")!@#$%^&*(".includes(c)) out.push(`${48 + ")!@#$%^&*(".indexOf(c)}_256`);
    else out.push({ "-": "189", ".": "190" }[c]);
  }
  return out.map((k) => `designdoc(0).__key_${k}`).join(",");
}
// (the mouse on the designed form: its client area's (x, y), the form's
// frame 24 px in, its title bar 29 px, a menu bar 28 px)
const at = (x, y, menu = 0) => `${x + 25}_${y + 54 + menu}`;
const click = (x, y, menu = 0) => `designdoc(0).__mousedown_${at(x, y, menu)},designdoc(0).__mouseup_${at(x, y, menu)}`;

// Each case: what Studio opens and does (`do`: its commands; `events`:
// RAPIDR_TEST_EVENTS, input through the kernel), how long it waits before
// the properties are read, and what each must say (a regular expression;
// `same`: equal to a file's text, line ends as the editor keeps them).
// (I8 AI-REAL: the settings dialog's clicks — typeInto and idle below)
const AI_TAB = "settingstabs.__mousedown_75_12,settingstabs.__mouseup_75_12";
const FETCH = "aifetchbtn.__mousedown_60_14,aifetchbtn.__mouseup_60_14";
const OK = "settingsokbtn.__mousedown_40_14,settingsokbtn.__mouseup_40_14";
const CANCEL = "settingsno.__mousedown_40_14,settingsno.__mouseup_40_14";
// (Preview 4 R1–R6) the assistant set up on the mock, and a click (press,
// release) at (x, y) of a component
const AI_READY = "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small";
const CLICK = (comp, x, y) => `${comp}.__mousedown_${x}_${y},${comp}.__mouseup_${x}_${y}`;
// (COPY-ANYWHERE) the program's output in Output, then "line of" dragged over
const COPY_WAIT = "outputbox.__wait_4000";
const COPY_DRAG = "outputbox.__mousedown_62_60,outputbox.__mousemove_80_60,outputbox.__mousemove_100_60,outputbox.__mousemove_117_60,outputbox.__mouseup_117_60";
// (Set up RAI — a modal window: its steps are input once it shows — "a
// cloud service" picked, Next, Anthropic in the provider list)
const SETUP_CLOUD = [CLICK("aisucloudradio", 8, 12), idle(2), CLICK("aisunextbtn", 30, 14), idle(2), "aisuprovbox.__item_0", idle(2)].join(",");
// (VERSIONS-G3) The walk's project — a copy of tests/fixtures/studio_versions
// (on the web, its files in the page's store) — and its steps: started and
// "first" saved by Ana; main.rr's last line changed and saved; "second"
const VERSIONS_FILES = ["Hello.rrproj", "main.rr", "greet.rr"].map((f) => `tests/fixtures/studio_versions/${f}`);
const V_OPEN = { open: "tests/fixtures/studio_versions/Hello.rrproj", copyDir: true, webFiles: VERSIONS_FILES, fsck: true };
const V_FIRST = "versions.pane,versions.start,versions:author=Ana,versions:save=first";
const V_EDIT = 'code:PRINT "Done."=>PRINT "All done!",file.save';
const V_SECOND = `${V_FIRST},${V_EDIT},versions:save=second`;
// (a key written in the code: the secret check's line; not a real key)
const V_SECRET = 'code:PRINT "Done."=>key$ = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH",file.save';

// (EXTERNAL-CHANGES) The walk's project — a copy of tests/fixtures/
// studio_external outside this repository — and what another process writes
// into it (tests/ext_writer.mjs): main.rr changed, typed in first, twelve
// files rewritten at once.
const X_OPEN = { open: "tests/fixtures/studio_external/Outside.rrproj", copyDir: true, fsck: true };
const X_DIR = "tests/fixtures/studio_external";
const X_MAIN = readFileSync(join(ROOT, X_DIR, "main.rr"), "utf8");
const X_AGENT = X_MAIN.replace('PRINT "Done."', 'PRINT "Changed by an agent."');
const X_CHANGE = "ext:go=1,ext:await=1";
const X_MINE = 'code:PRINT "Done."=>PRINT "Mine."';
const X_WINDOW = readFileSync(join(ROOT, X_DIR, "window.rr"), "utf8").replace('Caption = "Hello"', 'Caption = "Changed outside"').replace("Left = 24", "Left = 140");
const X_TWELVE = Object.fromEntries(["main.rr", "greet.rr", "window.rr", ...Array.from({ length: 9 }, (_, i) => `part${i + 1}.rr`)].map((f) => [f, readFileSync(join(ROOT, X_DIR, f), "utf8") + "' (rewritten by an agent)\n"]));
const CASES = [
  {
    name: "run-console",
    open: "examples/basics/hello.rr",
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /Hello from RapidR![\s\S]*ended, exit code 0/, "session.state": /^stopped$/, "session.exitcode": /^0$/ },
  },
  {
    // (the old web IDE's console suite) CLS, COLOR and LOCATE in Output as
    // on a terminal: CLS cleared "one" / "two"; LOCATE 1, 7 overwrote row 1
    // from column 7; the next PRINT went on row 2 over "yellow on blue"; no
    // escape sequence left in the text (the colours: rapidr-value's
    // panels::console::screen tests)
    name: "run-ansi",
    open: "tests/fixtures/studio_console_ansi.bas",
    webFiles: ["tests/fixtures/studio_console_ansi.bas"],
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /^(?![\s\S]*(\x1b|\bone\b|\btwo\b))[\s\S]*^first LINE\nrow2ow on blue$/m, "session.exitcode": /^0$/ },
  },
  {
    name: "outline-problems",
    open: "examples/gui/hello_form.rr",
    do: "wait",
    delay: 3,
    dump: { "outlinetree.itemcount": /^[5-9]|1\d$/, "lang.errorcount": /^0$/, "proj.kind": /^file$/, "proj.filecount": /^1$/ },
  },
  {
    // A program with errors: Problems lists them, and Run runs nothing
    name: "problems",
    open: "tests/fixtures/studio_problems.bas",
    webFiles: ["tests/fixtures/studio_problems.bas"],
    do: "wait,run.start,wait,wait",
    delay: 4,
    dump: { "lang.errorcount": /^[1-9]\d*$/, "outputbox.problemcount": /^[1-9]\d*$/, "outputbox.page": /^problems$/, "session.state": /^stopped$/, "outputbox.text": /^(?![\s\S]*fine)/ },
  },
  {
    name: "theme-and-tabs",
    open: "examples/gui/hello_form.rr",
    do: "view.theme.dark",
    delay: 3,
    dump: { "application.theme": /^rapidr dark$/, "dock.documentmode": /^tabs$/ },
  },
  // (THEME-LIVE, Robert: "Welcome screen is not responding to theme
  // changes, it's always light") Studio following a light system (its
  // default, View > Theme > RapidR); the system switches to dark while it
  // runs (`look` and the __look_ step: the test hooks): the Welcome page,
  // its cards, the look Studio reads — dark, its text AA. Then high
  // contrast with Settings open; and back to light.
  {
    name: "theme-follows-system-dark",
    theme: "rapidr",
    look: "light",
    events: "studio.__look_dark",
    delay: 4,
    contrast: true,
    dump: { "application.theme": /^rapidr dark$/, "welcome.color": /^2627858$/, "welcomepage.color": /^2627858$/, "excard(0).color": /^3812133$/, "excard(15).color": /^3812133$/ },
  },
  {
    name: "theme-follows-system-contrast-settings",
    theme: "rapidr",
    look: "dark",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: "studio.__look_contrast",
    delay: 4,
    contrast: true,
    dump: { "application.theme": /^rapidr high contrast$/, "settingsdialog.__shown": /^1$/, "welcome.color": /^0$/ },
  },
  {
    name: "theme-follows-system-light",
    theme: "rapidr",
    look: "contrast",
    events: "studio.__look_light",
    delay: 4,
    contrast: true,
    dump: { "application.theme": /^rapidr light$/, "welcome.color": /^16777215$/, "excard(0).color": /^16777215$/ },
  },
  // (a look View > Theme named stays when the system switches)
  {
    name: "theme-named-stays",
    theme: "rapidr",
    look: "dark",
    do: "view.theme.light",
    events: "studio.__look_dark",
    delay: 4,
    dump: { "application.theme": /^rapidr light$/, "welcome.color": /^16777215$/ },
  },
  // (THEME-LIVE) The Welcome page's cards from the keyboard: Shift+Tab from
  // the Project pane (where the keyboard starts) reaches the last card,
  // the page scrolled to it; again: the one before; Enter opens it
  {
    name: "welcome-cards-keyboard",
    look: "dark",
    theme: "rapidr",
    events: "projecttree.__key_9_256,studio.__key_9_256,studio.__key_13",
    delay: 4,
    dump: { "studio.caption": /^notepad - RapidR Studio$/, "welcome.vertposition": /^[1-9]\d*$/, "excard(14).tabstop": /^1$/ },
  },
  // (the final audit) The Welcome page's Start rows are keyboard stops
  // with names, as the cards: Enter on "New Project..." opens its dialog
  {
    name: "welcome-start-keyboard",
    look: "dark",
    theme: "rapidr",
    events: "startrow(0).__key_13",
    delay: 4,
    dump: { "startrow(0).tabstop": /^1$/, "startrow(0).accessiblename": /^New Project\.\.\.$/, "startrow(4).tabstop": /^1$/, "newdialog.visible": /^(1|-1|True)$/ },
  },
  // (EXAMPLES-DATA) RapidR's examples where RapidR is installed (a
  // read-only home, as /usr/lib/rapidr or the macOS app): the Welcome
  // page's card clicked (the mouse, on its title) opens the user's copy in
  // Documents/RapidR/Examples/<name> — made with its data, said in the
  // Output with "Copy again from the original"; a copy that's there opens
  // as the user left it (its missing files copied again); File > Copy
  // Example Again asks first — Copy Again puts the original back, Cancel
  // changes nothing; the same from the command line (Recent's way: a path)
  {
    name: "example-readonly-home",
    readOnlyHome: true,
    desktopOnly: true,
    events: "extitle(1).__mousedown_5_5,extitle(1).__mouseup_5_5",
    delay: 4,
    dump: { "proj.folder": /docs\/RapidR\/Examples\/linked_data$/, "proj.examplecopy": /^made$/, "studio.caption": /^linked_data - RapidR Studio$/,
      "outputbox.text": /Made your copy of the example linked_data\.rr \(the installed examples can't be changed\), in:\n {2}\S*docs\/RapidR\/Examples\/linked_data\n {2}Copy again from the original/ },
    desktopFiles: ["docs/RapidR/Examples/linked_data/linked_data.rr", "docs/RapidR/Examples/linked_data/staff.csv"],
  },
  {
    // (an install the user could write: copied all the same — nothing is
    // written inside RapidR, an update would take it away)
    name: "example-installed-writable-home",
    readOnlyHome: true,
    installedWritable: true,
    desktopOnly: true,
    events: "extitle(1).__mousedown_5_5,extitle(1).__mouseup_5_5",
    delay: 4,
    dump: { "proj.folder": /docs\/RapidR\/Examples\/linked_data$/, "proj.examplecopy": /^made$/ },
  },
  {
    name: "example-readonly-home-kept",
    readOnlyHome: true,
    desktopOnly: true,
    seed: { "RapidR/Examples/linked_data/linked_data.rr": "' mine\nPRINT \"mine\"\n" },
    events: "extitle(1).__mousedown_5_5,extitle(1).__mouseup_5_5",
    delay: 4,
    dump: { "proj.examplecopy": /^kept$/, "outputbox.text": /Your copy of the example linked_data\.rr, as you left it, in:/ },
    files: { "docs/RapidR/Examples/linked_data/linked_data.rr": /^' mine\nPRINT "mine"\n$/ },
    desktopFiles: ["docs/RapidR/Examples/linked_data/staff.csv"],
  },
  {
    name: "example-copy-again",
    readOnlyHome: true,
    desktopOnly: true,
    seed: { "RapidR/Examples/linked_data/linked_data.rr": "' mine\nPRINT \"mine\"\n" },
    open: "examples/data/linked_data.rr",
    do: "file.exampleAgain",
    events: "promptsave.__key_13",
    delay: 6,
    dump: { "prompttitle.caption": /^Copy linked_data\.rr again from the original\?$/, "promptsave.caption": /^&Copy Again$/, "promptdont.visible": /^0$/, "saveprompt.visible": /^0$/,
      "outputbox.text": /Copied linked_data\.rr again from the original, in:/, "proj.mainfile": /^linked_data\.rr$/ },
    files: { "docs/RapidR/Examples/linked_data/linked_data.rr": /^' linked_data\.rr: linked data components/ },
  },
  {
    name: "example-copy-again-cancel",
    readOnlyHome: true,
    desktopOnly: true,
    seed: { "RapidR/Examples/linked_data/linked_data.rr": "' mine\nPRINT \"mine\"\n" },
    open: "examples/data/linked_data.rr",
    do: "file.exampleAgain",
    events: "promptcancel.__mousedown_12_12,promptcancel.__mouseup_12_12",
    delay: 6,
    dump: { "saveprompt.visible": /^0$/, "proj.examplecopy": /^kept$/ },
    files: { "docs/RapidR/Examples/linked_data/linked_data.rr": /^' mine\nPRINT "mine"\n$/ },
  },
  {
    // (the desktop works on a copy: Save All writes the project file)
    name: "save-project",
    open: "examples/gui/hello_form.rr",
    copy: true,
    do: "file.saveAll,wait",
    delay: 3,
    dump: { "proj.kind": /^project$/, "proj.filename": /hello_form\.rrproj$/, "proj.filecount": /^1$/ },
  },
  {
    name: "open-folder",
    open: "",
    folder: "examples/gui",
    // (the web: the folder's files in the page's store, as showDirectoryPicker leaves them)
    webFiles: ["examples/gui/dialogs.rr", "examples/gui/hello_form.rr", "examples/gui/menus.rr"],
    do: "file.openFolder,wait,wait",
    delay: 4,
    dump: { "proj.mainfile": /^dialogs\.rr$/, "studio.caption": /^dialogs - RapidR Studio$/ },
  },
  // (WEB-OPEN) File > Open in a browser gives Studio the one file picked:
  // a project file (.rrproj) alone, not the files it lists. Studio no
  // longer opens the project with empty files (2026-10-22: main.rr empty,
  // or a closed document's code under its name, no forms); it offers the
  // project's folder. Cancel (Escape): nothing opened, the Output says how.
  {
    name: "web-open-project-alone",
    webOnly: true,
    open: "",
    folder: "Kiosk.rrproj",
    webFiles: ["tests/fixtures/studio_web_open/Kiosk.rrproj=>Kiosk.rrproj"],
    do: "file.open,wait",
    events: "promptsave.__key_27",
    delay: 4,
    dump: {
      "prompttitle.caption": /^Open the folder Kiosk\.rrproj is in\?$/,
      "promptsave.caption": /^&Open Folder\.\.\.$/,
      "promptdont.visible": /^0$/,
      "saveprompt.visible": /^0$/,
      "proj.kind": /^$/,
      "outputbox.text": /Kiosk\.rrproj: the browser gave RapidR Studio this file only, not the project's files in its folder\n {2}File > Open Folder\.\.\. opens the project with its files$/,
      "studio.caption": /^RapidR Studio$/,
    },
  },
  // (WEB-OPEN) …the folder (as showDirectoryPicker leaves it in the page's
  // store): the project with its files — main.rr in the editor, ui.rr's
  // three forms in the tree — and, with Targets = Web, one note for the
  // desktop-only component.
  {
    name: "web-open-project-folder",
    webOnly: true,
    open: "",
    folder: "kiosk",
    webFiles: WEB_OPEN_FILES.map((f) => `tests/fixtures/studio_web_open/${f}=>kiosk/${f}`),
    do: "file.openFolder,wait,wait,targets:web,view.problems,wait",
    delay: 5,
    a11y: [
      ["ui.rr", "treeitem", null, (n) => /3 forms/.test(n.description || "")],
      ["MainForm, RForm", "treeitem"],
      ["NewForm, RForm", "treeitem"],
      ["CfgForm, RForm", "treeitem"],
    ],
    dump: {
      "proj.filename": /^kiosk\/Kiosk\.rrproj$/,
      "projecttree.filecount": /^3$/,
      "codedoc(0).text": /^' The web Studio's "project file picked alone" fixture/,
      "outputbox.problemcount": /^1$/,
      "outputbox.text": /globals\.rr:3:\d+: info: RServerSocket works on the desktop only: the web runtime doesn't have it/,
    },
  },
  // (the program's windows over the whole page: floatingWindows, at 1x and
  // 2x; on the desktop they are windows of their own — the program's
  // window is shown and captured there, then it ends)
  {
    name: "program-windows-float",
    open: "examples/gui/themes.rr",
    do: "run.start",
    delay: 4,
    maximized: true,
    viewport: { width: 1440, height: 900 },
    scales: [1, 2],
    web: floatingWindows,
    desktopFiles: ["window-program-1.bmp"],
    dump: { "session.exitcode": /^0$/, "session.error": /^$/ },
  },
  // (a program window bigger than the page: bigWindow — on a 1280 x 720
  // page, and on 1920 x 1080 where it fits)
  {
    name: "program-window-big",
    open: "tests/fixtures/studio_debug/big_window.rr",
    webFiles: ["tests/fixtures/studio_debug/big_window.rr"],
    do: "run.start",
    delay: 4,
    maximized: true,
    viewport: { width: 1280, height: 720 },
    scales: [1, 2],
    web: bigWindow,
    desktopFiles: ["window-program-1.bmp"],
    dump: { "session.error": /^$/ },
  },
  {
    name: "program-window-big-fits",
    open: "tests/fixtures/studio_debug/big_window.rr",
    webFiles: ["tests/fixtures/studio_debug/big_window.rr"],
    do: "run.start",
    delay: 4,
    maximized: true,
    viewport: { width: 1920, height: 1080 },
    scales: [1],
    web: bigWindow,
    // (the desktop's windows are the system's: the first case shows it there)
    webOnly: true,
    dump: { "session.error": /^$/ },
  },
  {
    // modal dialogs stay on top: the program's over its form, Studio's
    // over the program's windows (modalsOnTop); the desktop's are windows
    name: "program-modal-on-top",
    open: "tests/fixtures/studio_debug/modal.rr",
    webFiles: ["tests/fixtures/studio_debug/modal.rr"],
    do: "run.start",
    delay: 4,
    maximized: true,
    viewport: { width: 1440, height: 900 },
    scales: [1, 2],
    web: programModal,
    dump: { "session.error": /^$/ },
  },
  {
    // Studio's own modal dialog opened while the program runs: over the
    // program's window (the desktop's are windows of their own)
    name: "studio-modal-over-program",
    open: "examples/gui/themes.rr",
    do: "run.start,wait,wait,wait,wait,wait,file.newProject",
    delay: 5,
    maximized: true,
    viewport: { width: 1440, height: 900 },
    scales: [1, 2],
    web: studioModal,
    dump: { "session.error": /^$/ },
  },
  {
    // Studio itself maximized on the page from the start (studioStartsMaximized)
    name: "studio-starts-maximized",
    open: "examples/gui/hello_form.rr",
    webOnly: true,
    delay: 3,
    maximized: true,
    viewport: { width: 1440, height: 900 },
    scales: [1],
    web: studioStartsMaximized,
    dump: {},
  },
  {
    // Run > Build: the app for this system (interpreted: the project says),
    // with the project's own icon; on the web Build says it's the desktop's
    name: "build-app",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "run.build",
    // (cargo checks the runner first: a minute on a busy machine)
    delay: 90,
    dump: { "proj.builtpath": /(Notes\.app|Notes\.AppDir|Notes\.exe)$/, "outputbox.text": /icon: .*note\.svg[\s\S]*Built .* \(interpreted\) in \d+ s/, "proj.building": /^0$/ },
    webDump: { "outputbox.text": /Can't build: Build Native App and Build Interpreted App make apps in RapidR Studio on the desktop/, "proj.builtpath": /^$/ },
  },
  // Run > Build Web App: the program as a web app, <project>-web.zip (the
  // desktop runs `rapidr build --web --interp`; the web page zips it itself
  // and downloads it). The web case checks the download: its files, then
  // the unzipped app served and run.
  {
    name: "build-web-app",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "run.buildWeb",
    delay: 25,
    dump: { "proj.builtpath": /Notes-web\.zip$/, "outputbox.text": /Web: .*Notes-web\.zip[\s\S]*Built Notes-web\.zip — unzip on any web host/, "proj.building": /^0$/ },
    webDump: { "outputbox.text": /✓ Built Notes-web\.zip — unzip on any web host/, "proj.builtpath": /^Notes-web\.zip$/, "proj.building": /^0$/ },
    web: (page, scale, record) => builtWebApp(page, record, "Notes-web.zip", ["main.rrbc"]),
  },
  // (an example with its data file: the CSV the program loads goes in the zip)
  {
    name: "build-web-app-data",
    open: "examples/data/dataframe.rr",
    copyDir: true,
    webFiles: ["examples/data/dataframe.rr", "examples/data/staff.csv"],
    do: "run.buildWeb",
    delay: 25,
    dump: { "proj.builtpath": /dataframe-web\.zip$/ },
    webDump: { "outputbox.text": /✓ Built dataframe-web\.zip — unzip on any web host/ },
    web: (page, scale, record) => builtWebApp(page, record, "dataframe-web.zip", ["dataframe.rrbc", "rapidr-assets.js"]),
  },
  // (T1b) A program that translates: its lang/*.po go in the zip beside the
  // page, with their list (lang/index.txt), as the CLI's builds put them
  {
    name: "build-web-app-translations",
    open: "examples/translations/languages.rr",
    copyDir: true,
    webFiles: ["examples/translations/languages.rr", "examples/translations/lang/es.po", "examples/translations/lang/pt.po", "examples/translations/lang/zh-Hans.po"],
    do: "run.buildWeb",
    delay: 25,
    dump: { "proj.builtpath": /languages-web\.zip$/, "outputbox.text": /Translations beside the page: lang\/es\.po, lang\/pt\.po, lang\/zh-Hans\.po/ },
    webDump: { "outputbox.text": /lang\/index\.txt[\s\S]*lang\/es\.po[\s\S]*✓ Built languages-web\.zip — unzip on any web host/ },
    web: (page, scale, record) => builtWebApp(page, record, "languages-web.zip", ["languages.rrbc", "lang/index.txt", "lang/es.po", "lang/pt.po", "lang/zh-Hans.po"]),
  },
  // (NO-RUST) A computer without Rust (RAPIDR_NO_RUST=1 says so): Build
  // Native App asks first (prompts.inc) — Enter is Build Interpreted Instead
  // (the app, "(interpreted)" in the log), Cancel builds nothing, Install
  // Rust... runs `rapidr setup` and the log says it ended; Run > Build with
  // a native project asks the same, and with an interpreted one just builds.
  {
    name: "build-native-no-rust-interpreted",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_NO_RUST: "1" },
    do: "run.buildNative",
    events: "promptsave.__key_13",
    // (the events fire at the end of the delay and the dump follows at once:
    // the build has begun, as interpreted; its end, "(interpreted) in N s",
    // is build-app's)
    delay: 8,
    dump: { "prompttitle.caption": /^Native builds need Rust/, "promptsave.caption": /^&Build Interpreted Instead$/, "promptdont.caption": /^&Install Rust\.\.\.$/, "saveprompt.visible": /^0$/, "outputbox.text": /^Building main\.rr: interpreted, release/ },
  },
  {
    name: "build-native-no-rust-cancel",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_NO_RUST: "1" },
    do: "run.buildNative,wait",
    events: "promptcancel.__mousedown_12_12,promptcancel.__mouseup_12_12",
    delay: 6,
    dump: { "prompttitle.caption": /^Native builds need Rust/, "saveprompt.visible": /^0$/, "proj.builtpath": /^$/, "proj.building": /^0$/, "outputbox.text": /^(?![\s\S]*(Built|Building))/ },
  },
  {
    name: "build-native-no-rust-install",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_NO_RUST: "1" },
    do: "run.buildNative",
    events: "promptdont.__mousedown_12_12,promptdont.__mouseup_12_12",
    delay: 8,
    // (a source checkout: setup only reports the Rust the machine has)
    dump: { "proj.builtpath": /^$/, "outputbox.text": /^RapidR \d/ },
  },
  {
    name: "build-default-native-no-rust",
    open: "tests/fixtures/studio_app_native/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_NO_RUST: "1" },
    do: "run.build,wait",
    events: "promptcancel.__mousedown_12_12,promptcancel.__mouseup_12_12",
    delay: 6,
    dump: { "prompttitle.caption": /^Native builds need Rust/, "proj.buildkind": /^native$/, "proj.builtpath": /^$/, "proj.building": /^0$/ },
  },
  {
    // Project Options: the Compiled choice says Rust is missing
    name: "app-options-no-rust",
    open: "tests/fixtures/studio_app_native/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_NO_RUST: "1" },
    do: "project.options",
    delay: 4,
    dump: { "appkind.text": /^Compiled \(Rust not installed\)$/, "appkindnote.caption": /^Rust is free/ },
  },
  {
    // Project Options: Rust is there (this machine's): the Compiled choice as it was
    name: "app-options-with-rust",
    open: "tests/fixtures/studio_app_native/Notes.rrproj",
    copyDir: true,
    desktopOnly: true,
    do: "project.options",
    delay: 4,
    dump: { "appkind.text": /^Compiled \(native: needs Rust\)$/ },
  },
  {
    // Project > Project Options: the app's name, ID, version, icon (previewed)
    name: "app-options",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "project.options",
    delay: 4,
    // (DARK-FIX: the dialog's text judged in every look, on both hosts)
    themes: LOOKS,
    contrast: true,
    dump: { "appnameedit.text": /^Notes$/, "appversionedit.text": /^1\.2\.0$/, "appiconedit.text": /^note\.svg$/, "appiconnote.caption": /every size/ },
  },
  {
    name: "palette",
    open: "",
    do: "view.commandPalette",
    delay: 3,
    dump: { "palette.count": /^[1-9]\d+$/, "palette.commandcount": /^[1-9]\d+$/ },
  },
  // (I4) The designer on the source: notepad.bas's form at its own size;
  // its right edge dragged 60 px (Width written), a QBUTTON placed from the
  // toolbox's tool (Button1's CREATE block written), then moved by 40, 40
  // (snapped); every step real input through the kernel (the mouse at
  // surface coordinates: the form's frame starts 12 px in).
  {
    name: "designer",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.place.QBUTTON",
    events: DESIGN_STEPS,
    delay: 4,
    dump: {
      "designdoc(0).formname": /^Form$/,
      "designdoc(0).statustext": /^Button1 \(RButton\), 128, 88, 75 × 25$/,
      "codedoc(0).text": /Width = 540\n    Height = 340[\s\S]*    CREATE Button1 AS QBUTTON\n        Caption = "Button1"\n        Left = 128\n        Top = 88\n        Width = 75\n        Height = 25\n    END CREATE\nEND CREATE/,
      // (POLISH-2: the designer's changes mark the document changed)
      "studio.modified": /^(1|-1|True)$/i,
      "codedoc(0).modified": /^(1|-1|True)$/i,
    },
  },
  // (I4) Enter on a toolbox item: AddComponent through the program (a
  // method's edits heard as OnSourceEdit on both hosts), named after the
  // registry's spelling (CheckBox1).
  {
    name: "designer-add",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.add.QCHECKBOX",
    delay: 3,
    dump: { "designdoc(0).statustext": /^Added CheckBox1 \(RCheckBox\)/, "codedoc(0).text": /    CREATE CheckBox1 AS QCHECKBOX\n        Caption = "CheckBox1"\n/ },
  },
  // (I4) The same, then Ctrl+Z three times: the exact text back.
  {
    name: "designer-undo",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.place.QBUTTON",
    events: DESIGN_STEPS + ",designdoc(0).__key_90_16,designdoc(0).__key_90_16,designdoc(0).__key_90_16",
    delay: 4,
    dump: { "designdoc(0).canundo": /^(0|False)$/i, "codedoc(0).text": /CREATE Form AS QFORM/ },
    same: { "codedoc(0).text": "examples/rapidq/notepad.bas" },
  },
  // ---- the code editor (S-EDITOR; docs/studio-wow.md ED-1 … ED-8): typed
  // through the kernel's keyboard path (Application.SendKeys), completion
  // from RapidR's language service ----
  {
    // `form.` lists QFORM's members: properties, then methods, then events,
    // each A–Z
    name: "editor-completion",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.,wait,wait",
    delay: 6,
    dump: { "codedoc(0).completionitems": /^AccessibleDescription\n[\s\S]*\nCaption\n[\s\S]*\nWidth\n[\s\S]*\nShowModal\n[\s\S]*\nOnClick\n/ },
  },
  {
    // typing narrows it, best match first: on the word starts (`sm` →
    // ShowModal) before letters anywhere
    name: "editor-completion-fuzzy",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.sm,wait",
    delay: 6,
    dump: { "codedoc(0).completionselected": /^ShowModal$/ },
  },
  {
    // Tab accepts the selected item; the language's words in upper case as
    // they're typed (rapidr.keywordCase = upper)
    name: "editor-accept-and-case",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.capt,key:Tab,type: = \"Hi\",key:Enter,type:dim y as string,key:Escape,key:Enter",
    delay: 8,
    dump: { "codedoc(0).text": /\nForm\.Caption = "Hi"\nDIM y AS STRING\n?$/, "codedoc(0).completionitems": /^$/ },
  },
  {
    // Tab on a selected block indents every line by the file's unit (4
    // spaces); the selection stays
    name: "editor-tab-indent",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Shift+Up,key:Shift+Up,key:Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nEND SUB\n\n {4}NameEdit\.SetFocus\n {4}Form\.ShowModal\n?$/, "codedoc(0).sellength": /^3[0-9]$/ },
  },
  {
    // Shift+Tab takes it back out: the text as it was
    name: "editor-tab-outdent",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Shift+Up,key:Shift+Up,key:Tab,key:Shift+Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nEND SUB\n\nNameEdit\.SetFocus\nForm\.ShowModal\n?$/, "codedoc(0).canundo": /^(-1|1|True)$/i },
  },
  {
    // a misspelt member: squiggled once typing pauses (RapidQ's compiler's
    // words), in Problems too; Ctrl+. offers the fix, Enter applies it
    name: "editor-diagnostic",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,key:Ctrl+.,wait",
    delay: 8,
    dump: { "codedoc(0).diagnosticcount": /^1$/, "codedoc(0).completionitems": /^Change to Text$/ },
  },
  {
    name: "editor-quick-fix",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,key:Ctrl+.,wait,key:Enter,wait,wait,wait",
    delay: 11,
    dump: { "codedoc(0).text": /\nx\$ = NameEdit\.Text\n?$/, "codedoc(0).diagnosticcount": /^0$/ },
  },
  {
    // (K4b, docs/rai-mistake-ledger.md) `NOT FILEEXISTS(f)` is always true —
    // FILEEXISTS gives 1, NOT 1 is -2: a warning in the code, and Ctrl+.
    // offers `FILEEXISTS(f) = 0`
    name: "editor-not-fileexists",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:IF NOT FILEEXISTS(\"notes.txt\") THEN PRINT 1,key:Escape,wait,wait,wait,key:Home,key:Right,key:Right,key:Right,key:Right,key:Ctrl+.,wait",
    delay: 8,
    dump: { "codedoc(0).diagnosticcount": /^1$/, "codedoc(0).completionitems": /^Write FILEEXISTS\("notes\.txt"\) = 0$/ },
  },
  {
    // …Enter applies it: the warning goes
    name: "editor-not-fileexists-fix",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:IF NOT FILEEXISTS(\"notes.txt\") THEN PRINT 1,key:Escape,wait,wait,wait,key:Home,key:Right,key:Right,key:Right,key:Right,key:Ctrl+.,wait,key:Enter,wait,wait,wait",
    delay: 11,
    dump: { "codedoc(0).text": /\nIF FILEEXISTS\("notes\.txt"\) = 0 THEN PRINT 1\n?$/, "codedoc(0).diagnosticcount": /^0$/ },
  },
  {
    // F12 on a call goes to its SUB
    name: "editor-go-to-definition",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12",
    delay: 6,
    dump: { "codedoc(0).caretline": /^42$/ },
  },
  {
    // (POLISH-2) Go to Line (Ctrl+G; Cmd+G on a Mac): the box keeps its
    // ":" and the number is typed after it; the caret goes there and the
    // status bar's line with it
    name: "editor-go-to-line",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+G,wait,type:12,key:Enter,wait,wait",
    delay: 5,
    dump: { "codedoc(0).caretline": /^12$/, "status.panel(2).caption": /^Ln 12, Col 1$/ },
  },
  {
    // (POLISH-2) Undone back to the text on disk, the document isn't
    // marked changed any more (its tab's dot, the window's)
    name: "editor-undo-to-saved",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,type:x,wait,key:Ctrl+Z,wait,wait",
    delay: 5,
    dump: { "studio.modified": /^(0|False)$/i, "codedoc(0).modified": /^(0|False)$/i },
  },
  {
    // …and redone away from it, marked again
    name: "editor-redo-from-saved",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,type:x,wait,key:Ctrl+Z,wait,key:Ctrl+Y,wait,wait",
    delay: 5,
    dump: { "studio.modified": /^(1|-1|True)$/i, "codedoc(0).modified": /^(1|-1|True)$/i },
  },
  // (E0–E2, docs/code-helpers-plan.md) The code's helpers on
  // examples/studio/helpers.rr, by the keys a user presses: Ctrl+. on a
  // value opens its editor, what it picks is written back in the code's own
  // form as one undo step; the helpers read with the line (HelperText,
  // Ctrl+K H's words); Quiet hides the note; a paused program's values at
  // the lines' ends.
  {
    name: "helpers-colour-open",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:18,key:End,key:Ctrl+.,wait`,
    delay: 8,
    capture: true,
    dump: {
      "codedoc(0).helpereditortext": /^.+, 250 246 238 \(was .+, 250 246 238\)$/,
      "codedoc(0).helpertext": /^Color = RGB\(250, 246, 238\); .+, 250 246 238/,
    },
  },
  {
    // Red 0, Green 0, Blue 255 typed, Enter: RGB(…) stays RGB(…)
    name: "helpers-colour-write",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:18,key:End,key:Ctrl+.,wait,key:Tab,key:Tab,type:0,key:Tab,type:0,key:Tab,type:255,key:Enter,wait`,
    delay: 9,
    dump: { "codedoc(0).text": /\n {4}Color = RGB\(0, 0, 255\)\n/, "codedoc(0).helpereditortext": /^$/ },
  },
  {
    // …and one Ctrl+Z puts it back
    name: "helpers-colour-undo",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:18,key:End,key:Ctrl+.,wait,key:Tab,key:Tab,type:0,key:Tab,type:0,key:Tab,type:255,key:Enter,wait,key:Ctrl+Z,wait`,
    delay: 9,
    dump: { "codedoc(0).text": /\n {4}Color = RGB\(250, 246, 238\)\n/ },
  },
  {
    // &H stays &H: the Hex field typed
    name: "helpers-hex",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:26,key:End,key:Ctrl+.,wait,key:Tab,key:Tab,key:Tab,key:Tab,key:Tab,type:#102030,key:Enter,wait`,
    delay: 9,
    dump: { "codedoc(0).text": /\n {8}FontColor = &H302010\n/ },
  },
  {
    // Esc: the code as it was, the editor gone
    name: "helpers-escape",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:18,key:End,key:Ctrl+.,wait,key:Right,key:Right,key:Escape,wait`,
    delay: 8,
    dump: { "codedoc(0).text": /\n {4}Color = RGB\(250, 246, 238\)\n/, "codedoc(0).helpereditortext": /^$/, "codedoc(0).canundo": /^(0|False)$/i },
  },
  {
    // a choice: Align's constants, Down picks the next
    name: "helpers-choice",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:35,key:End,key:Ctrl+.,wait,key:Down,key:Enter,wait`,
    delay: 8,
    capture: true,
    dump: { "codedoc(0).text": /\n {8}Align = alTop\n/ },
  },
  {
    // an event: the list opens on its SUB, Create handler at the top (Home)
    // — the new SUB before the form, linked, one undo step
    name: "helpers-create-handler",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:36,key:End,key:Ctrl+.,wait,key:Home,key:Enter,wait`,
    delay: 8,
    dump: { "codedoc(0).text": /\nSUB AgainClick\n {4}\nEND SUB\n\nCREATE Form AS RForm\n[\s\S]*\n {8}OnClick = AgainClick\n/ },
  },
  {
    // the picture's mark: there…
    name: "helpers-picture",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:44,key:End`,
    delay: 7,
    dump: { "codedoc(0).helpertext": /^Logo\.Load "logo\.svg"; picture logo\.svg; Loads a picture/ },
  },
  {
    // the picture editor (Ctrl+. on it): the project's pictures, the one in
    // the code picked, its preview inside the editor (the real walk once
    // found it drawn past the editor's bottom)
    name: "helpers-picture-editor",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:44,key:End,key:Ctrl+.,wait`,
    delay: 9,
    capture: true,
    dump: { "codedoc(0).helpereditortext": /logo\.svg/ },
  },
  {
    // the font editor: the font in the code picked, its preview line clear
    // of Apply and Cancel; Esc leaves the code as it was
    name: "helpers-font-editor",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:41,key:End,key:Ctrl+.,wait`,
    delay: 9,
    capture: true,
    dump: { "codedoc(0).helpereditortext": /^Font: Inter/ },
  },
  {
    name: "helpers-font-escape",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:41,key:End,key:Ctrl+.,wait,key:Down,key:Escape,wait`,
    delay: 9,
    dump: { "codedoc(0).text": /\nTitleFont\.Name = "Inter"\n/, "codedoc(0).helpereditortext": /^$/ },
  },
  {
    // …and a name typed wrong: the red mark, said "missing"
    name: "helpers-picture-missing",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:44,key:End,key:Left,type:x,key:Escape,key:Down,wait,wait,wait,wait,key:Up,key:End`,
    delay: 9,
    dump: { "codedoc(0).helpertext": /picture logo\.svgx, missing/ },
  },
  {
    // the line's help, read with it; Quiet: the colours only
    name: "helpers-note",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `${HELPERS_READY},line:48,key:End`,
    delay: 7,
    capture: true,
    dump: { "codedoc(0).helpertext": /RGB\(255, 130, 20\); orange, 255 130 20; .*rectangle/i, "codedoc(0).helpers": /^helpful$/ },
  },
  {
    name: "helpers-quiet",
    open: "examples/studio/helpers.rr",
    webFiles: HELPERS_FILES,
    do: `helpers:quiet,${HELPERS_READY},line:48,key:End`,
    delay: 7,
    dump: { "codedoc(0).helpertext": /^(?![\s\S]*rectangle)[\s\S]*orange, 255 130 20$/i, "codedoc(0).helpers": /^quiet$/ },
  },
  {
    // the debugger stopped on line 6: the values of the names each line
    // above uses, at its end (Variables' values)
    name: "helpers-values-paused",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    dump: { "session.state": /^paused$/, "codedoc(0).linevaluestext": /^i = 1$/ },
  },
  {
    // …gone once it ends
    name: "helpers-values-gone",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,debug.toggleBreakpoint,run.start,wait,wait,wait",
    delay: 9,
    dump: { "session.state": /^stopped$/, "codedoc(0).linevaluestext": /^$/ },
  },
  // (S-PANELS) The inspector on the designer: pantry's AddBtn selected,
  // its Caption and Width set in the inspector — the code shows them as the
  // smallest edit (the values on their line) and the inspector reads them
  // back.
  {
    name: "inspector-edits-code",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Caption=Go,prop:Width=120,wait",
    delay: 6,
    dump: {
      "inspector.target": /^AddBtn$/,
      "inspector.rows": /^Caption=Go$[\s\S]*^Width=120$/m,
      "codedoc(0).text": /    CREATE AddBtn AS RButton\n        Caption = "Go": Left = 314: Top = 252: Width = 120\n        OnClick = AddItem\n/,
    },
  },
  // (C-CURSORS) The mouse pointer every part shows by default (Robert: the
  // inspector's divider didn't say it drags): `comp.__cursor_x_y` in the dump
  // is the pointer's CSS name at (x, y) of the component, the mouse moved
  // there as the user's — the inspector's divider between the name and value
  // columns, its search box, the dock's splitters (left | documents, documents
  // above output), the designer's selected component and its handle.
  {
    name: "pointers",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,wait",
    delay: 6,
    dump: {
      "inspector.__cursor_120_246": /^col-resize$/,
      "inspector.__cursor_60_246": /^default$/,
      "inspector.__cursor_120_80": /^text$/,
      "dock.__cursor_241_200": /^col-resize$/,
      "dock.__cursor_600_499": /^row-resize$/,
      // (AddBtn at 314, 252 of the form's client, which S-DESIGN-2's canvas
      // puts at 25, 54 of the surface: its body, its top-left handle)
      "designdoc(0).__cursor_355_306": /^move$/,
      "designdoc(0).__cursor_339_306": /^nwse-resize$/,
    },
  },
  // (the web) A screen reader on the inspector: Chrome's accessibility
  // tree (what VoiceOver, NVDA and JAWS read) has a tree grid of rows, each
  // named by its cells — the property's name (a row header) and its value
  // — and a read-only property says so: "read-only" in its row's
  // description (announced with the row) and its value's cell read-only
  // (ARIA has no read-only row). inspectorAccess.
  {
    name: "inspector-read-only",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,wait",
    delay: 6,
    webOnly: true,
    web: inspectorAccess,
    dump: { "studio.caption": /pantry/ },
  },
  // (S-PANELS) …then Undo twice on the designer: the exact text back.
  {
    name: "inspector-undo",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Caption=Go,prop:Width=120,wait,edit.undo,edit.undo,wait",
    delay: 7,
    dump: { "designdoc(0).canundo": /^(0|False)$/i, "inspector.rows": /^Caption=&Add to shelf$[\s\S]*^Width=110$/m, "codedoc(0).text": /CREATE AddBtn AS RButton/ },
    same: { "codedoc(0).text": "examples/gui/pantry.rr" },
  },
  // (S-PANELS) The code edited (a Caption typed over): the designer reads
  // it and the inspector shows it.
  {
    name: "code-edits-inspector",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,code:"&Add to shelf"=>"Store it",wait,wait,wait',
    delay: 7,
    dump: { "inspector.rows": /^Caption=Store it$/m, "designdoc(0).source": /Caption = "Store it": Left = 314/ },
  },
  // (S-PANELS) An event's row double-clicked in the inspector: its SUB
  // written with the registry's parameters (a DECLARE beside pantry's, the
  // SUB at the end), bound in the CREATE block, the caret inside it.
  {
    name: "inspector-event-handler",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:NameEdit,page:events,event:OnKeyDown,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /DECLARE SUB AddItem\nDECLARE SUB NameEditKeyDown \(Key AS WORD, Shift AS INTEGER\)\n[\s\S]*OnKeyDown = NameEditKeyDown\n[\s\S]*\nSUB NameEditKeyDown \(Key AS WORD, Shift AS INTEGER\)\n    \nEND SUB\n?$/,
      "inspector.rows": /^OnKeyDown=NameEditKeyDown$/m,
    },
  },
  // (S-PANELS) Typed values and a reset: Default as RapidQ writes a
  // Boolean, a colour constant, Width put back to its default (its
  // assignment taken out of the line); the Events page offers the file's
  // SUBs.
  {
    name: "inspector-typed",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Default=True,prop:Color=clRed,reset:Width,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /    CREATE AddBtn AS RButton\n        Caption = "&Add to shelf": Left = 314: Top = 252\n        OnClick = AddItem\n        Default = 1\n        Color = clRed\n/,
      "inspector.rows": /^Default=True$[\s\S]*^Width=75$/m,
    },
  },
  // (PROJECT-PROPS) The project's name at the top of the Project pane,
  // selected: the Properties pane shows the project — "RenMany  Project" in
  // its header, its look (Theme), name, main file and targets, how Run
  // starts it, the form designer's grid —, in every look of Studio.
  {
    name: "project-props",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,wait",
    delay: 6,
    capture: true,
    contrast: true,
    themes: LOOKS,
    dump: {
      "inspector.target": /^$/,
      "inspector.rows": /^\[Appearance\]\nTheme=RapidR\n\[Designer\]\nGrid=8\nGuides=True\nSnap=True\n\[General\]\nMainFile=main\.rr\nName=RenMany\nTargets=desktop\n\[Run\]\nArguments=$/,
      "projecttree.selectedkind": /^project$/,
    },
    // (a screen reader: "RenMany, project" selected in the Project pane; the
    // inspector's rows are rows like a component's, each with its value)
    a11y: [
      [/^RenMany$/, "treeitem", "selected", (n) => n.description === "project"],
      ["Theme", "row", null, (n) => n.value === "RapidR"],
      ["MainFile", "row", null, (n) => n.value === "main.rr"],
    ],
  },
  // …a row selected shows what it does under the rows
  {
    name: "project-props-doc",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,row:Theme,wait",
    delay: 6,
    capture: true,
    dump: {
      "inspector.selected": /^Theme$/,
      "inspector.doc": /^The look the program is drawn in: RapidR follows the system, or choose Light, Dark, High Contrast or Classic\. It is the \$THEME line at the top of the main file/,
    },
  },
  // Theme set to RapidR Dark with Studio light: the main file says it — one
  // line after $APPTYPE, before the includes, its tab marked changed — and
  // the designer draws the form dark; Studio stays light. The search finds
  // the row.
  {
    name: "project-theme-dark",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,prop:Theme=RapidR Dark,wait,wait",
    delay: 7,
    capture: true,
    dump: {
      "application.theme": /^rapidr light$/,
      "designdoc(0).showntheme": /^rapidr dark$/,
      "codedoc(0).text": /\$APPTYPE GUI\n\$THEME RapidR Dark\n\$INCLUDE "helpers\.rr"\n/,
      "codedoc(0).modified": /^(1|True|-1)$/i,
      "inspector.rows": /^\[Appearance\]\nTheme=RapidR Dark\n/,
      "inspector.target": /^$/,
    },
  },
  // …and with Studio dark: a look the project names is the designer's whatever
  // Studio has (RapidR Light on a dark Studio); naming none (RapidR), the
  // designer shows Studio's own look
  {
    name: "project-theme-light-while-studio-dark",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    theme: "rapidr-dark",
    do: "wait,view.designer,wait,project:select,prop:Theme=RapidR Light,wait,wait",
    delay: 7,
    capture: true,
    dump: { "application.theme": /^rapidr dark$/, "designdoc(0).showntheme": /^rapidr light$/, "inspector.rows": /^\[Appearance\]\nTheme=RapidR Light\n/ },
  },
  {
    name: "project-theme-rapidr-in-studio-dark",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    theme: "rapidr-dark",
    do: "wait,view.designer,wait",
    delay: 6,
    dump: { "application.theme": /^rapidr dark$/, "designdoc(0).showntheme": /^rapidr dark$/ },
  },
  // Theme back to RapidR: the line is gone, the text exactly the first
  // (one Undo each way too); Studio's own look untouched
  {
    name: "project-theme-back",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,prop:Theme=RapidR Dark,wait,prop:Theme=RapidR,wait,wait",
    delay: 7,
    dump: {
      "application.theme": /^rapidr light$/,
      "designdoc(0).showntheme": /^rapidr light$/,
      "codedoc(0).text": /^[^$]*\$APPTYPE GUI\n\$INCLUDE "helpers\.rr"\n/,
      "inspector.rows": /^\[Appearance\]\nTheme=RapidR\n/,
    },
    same: { "codedoc(0).text": "tests/fixtures/studio_projprops/main.rr" },
  },
  // …Undo takes the line out again, as one step: the designer, the project's
  // Theme row and the main file's mark follow it back
  {
    name: "project-theme-undo",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,prop:Theme=RapidR Dark,wait,edit.undo,wait,wait,wait",
    delay: 8,
    dump: {
      "designdoc(0).showntheme": /^rapidr light$/,
      "inspector.rows": /^\[Appearance\]\nTheme=RapidR\n/,
      "codedoc(0).modified": /^(0|False)$/i,
      "codedoc(0).text": /^' main\.rr: [^$]*\$APPTYPE GUI\n\$INCLUDE "helpers\.rr"\n/,
    },
    same: { "codedoc(0).text": "tests/fixtures/studio_projprops/main.rr" },
  },
  // …with the main file's tab closed it opens for the line, and the project's
  // properties stay (the tab that opens doesn't take them away)
  {
    name: "project-theme-main-closed",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,file.close,project:select,prop:Theme=RapidR High Contrast,wait,wait,wait",
    delay: 8,
    dump: {
      "codedoc(0).text": /\$APPTYPE GUI\n\$THEME RapidR High Contrast\n\$INCLUDE "helpers\.rr"\n/,
      "designdoc(0).showntheme": /^rapidr high contrast$/,
      "inspector.rows": /^\[Appearance\]\nTheme=RapidR High Contrast\n/,
    },
  },
  // …the line edited by hand: the designer follows, the project's Theme row too
  {
    name: "project-theme-by-hand",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: 'wait,view.designer,code:$INCLUDE "helpers.rr"=>$THEME Classic,wait,wait,wait,project:select,wait',
    delay: 8,
    dump: {
      "designdoc(0).showntheme": /^classic$/,
      "inspector.rows": /^\[Appearance\]\nTheme=Classic\n/,
    },
  },
  // Name, main file, targets, arguments, the designer's grid: each saved to
  // the project file; the Project pane's top row says the new name
  {
    name: "project-props-saved",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: 'wait,view.designer,project:select,prop:Name=Renamer,prop:MainFile=helpers.rr,prop:MainFile=main.rr,prop:Targets=web,prop:Arguments=--dry "a b",prop:Grid=16,prop:Snap=False,prop:Guides=False,wait',
    delay: 8,
    capture: true,
    dump: {
      "projecttree.projectname": /^Renamer$/,
      "projecttree.selectedkind": /^project$/,
      "inspector.rows": /Name=Renamer\nTargets=web\n[\s\S]*Arguments=--dry "a b"/,
      "designdoc(0).gridsize": /^16$/,
      "designdoc(0).snaptogrid": /^(0|False)$/i,
      "designdoc(0).showguides": /^(0|False)$/i,
    },
    files: { "RenMany.rrproj": /name = "Renamer"\nmain = "main\.rr"[\s\S]*runs_on = "web"[\s\S]*args = \[\s*"--dry",\s*"a b",?\s*\][\s\S]*\[designer\]\ngrid = 16\nsnap = false\nguides = false/ },
  },
  // A file chosen as the main file that isn't one of the project's: refused,
  // the row put back; selecting a form's component returns to the component
  // inspector, and the project's name selected again brings the project back.
  {
    name: "project-props-leave",
    open: "tests/fixtures/studio_projprops/RenMany.rrproj",
    copyDir: true,
    webFiles: PROJPROPS_FILES,
    do: "wait,view.designer,project:select,prop:MainFile=nothere.rr,wait,pick:GoBtn,wait",
    delay: 7,
    dump: { "inspector.target": /^GoBtn$/, "inspector.rows": /^\[Appearance\]|^Caption=/m },
  },
  // (S-DESIGN-2) Each kind of the inspector's editors writes the code a
  // program needs: a font by its parts (Font.Name quoted, Size, Color,
  // Bold as 1), an enum, a colour and a Boolean — the RapidQ constants by
  // their names, built into a RapidR program (M0-CONST; a RapidQ program
  // without RAPIDQ.INC gets their numbers: they'd read as nothing there), a
  // caption with its & — and the inspector reads them back.
  {
    name: "inspector-kinds",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,pick:Answer,prop:Font.Bold=True,prop:Font.Size=12,prop:Font.Color=clBlue,prop:Font.Name=Arial,prop:Alignment=taCenter,prop:Color=clYellow,prop:Caption=Say hi && bye,prop:WordWrap=True,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /    CREATE Answer AS RLabel\n        Caption = "Say hi && bye"\n        Left = 16: Top = 96: Width = 300\n        Font\.Bold = 1\n        Font\.Size = 12\n        Font\.Color = clBlue\n        Font\.Name = "Arial"\n        Alignment = taCenter\n        Color = clYellow\n        WordWrap = 1\n    END CREATE/,
      "inspector.rows": /^Alignment=taCenter$[\s\S]*^Caption=Say hi && bye$[\s\S]*^Color=(clYellow|&H00FFFF)$[\s\S]*^WordWrap=True$[\s\S]*^Font=Arial, 12 pt, Bold$/m,
    },
  },
  // (S-PANELS) The toolbox: Enter on QCHECKBOX adds one to the form (its
  // CREATE block in the code), selected in the inspector.
  // (R-NAMES) The toolbox shows RapidR's names and gives RButton's kind of
  // name whatever was asked (QCHECKBOX here); the designer writes it in the
  // file's own style: pantry.rr writes RapidR's names, so RCheckBox.
  {
    name: "toolbox-add",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,tool:QCHECKBOX,wait",
    delay: 6,
    dump: { "codedoc(0).text": /    CREATE CheckBox1 AS RCheckBox\n(?![\s\S]*QCHECKBOX)/, "inspector.target": /^CheckBox1$/i, "toolbox.shownames": /^rapidr$/, "toolbox.selected": /^RCheckBox$/ },
  },
  // (R-NAMES) RapidQ's other source extensions open as they are: a .rqw
  // (RapidQ's names, an include in a folder of its own) — its form in the
  // designer, the language service reading the include (on the web too).
  {
    name: "open-rqw",
    open: "tests/fixtures/rapidq_import/greeter.rqw",
    webFiles: ["tests/fixtures/rapidq_import/greeter.rqw", "tests/fixtures/rapidq_import/include/shapes.inc"],
    do: "wait,view.designer,wait",
    delay: 5,
    dump: { "proj.kind": /^file$/, "proj.mainfile": /^greeter\.rqw$/, "proj.filecount": /^2$/, "designdoc(0).formname": /^Form$/, "lang.errorcount": /^0$/ },
  },
  {
    name: "run-rq",
    open: "tests/fixtures/rapidq_import/count.rq",
    webFiles: ["tests/fixtures/rapidq_import/count.rq"],
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /1: one\n2: two\n3: three[\s\S]*ended, exit code 0/, "session.exitcode": /^0$/, "proj.mainfile": /^count\.rq$/ },
  },
  // (R-NAMES) File > Import RapidQ Project or File: a copy of a RapidQ
  // program (a .rqw with an include in a folder of its own) with RapidR's
  // names, proved to compile to the same bytecode, opened as a project with
  // the RapidQ-compatible setting off, its report beside the code.
  {
    name: "import-rapidq",
    importFrom: "tests/fixtures/rapidq_import/greeter.rqw",
    webFiles: ["tests/fixtures/rapidq_import/greeter.rqw", "tests/fixtures/rapidq_import/include/shapes.inc"],
    do: "wait",
    delay: 6,
    dump: {
      "proj.importsummary": /^1 program\(s\), 2 source file\(s\) \(2 changed, 7 name\(s\)\)[\s\S]*: 1 identical, 0 different/,
      "proj.mainfile": /^greeter\.rqw$/,
      "proj.filecount": /^2$/,
      "proj.compatmode": /^$/,
      "proj.importreport": /greeter-rapidr\/rapidr-import-report\.md$/,
      "codedoc(0).text": /^(?![\s\S]*AS Q[A-Z])[\s\S]*DECLARE SUB SayHello \(Sender AS RButton\)[\s\S]*CREATE Form AS RForm[\s\S]*CREATE NameEdit AS REdit/,
      "codedoc(1).text": /^# RapidQ import: greeter\.rqw[\s\S]*\| `greeter\.rqw` \| yes: identical bytecode \|[\s\S]*`QBUTTON` → `RButton`/,
      "lang.errorcount": /^0$/,
    },
  },
  // (IMPORT-INC) A RapidQ program in several files opens with its includes:
  // `Lib\Geometry.INC` is lib/geometry.inc (any case, \ or /), the nested
  // `Lib\Units.inc` found from the program's folder as RC.EXE finds it —
  // the project tree lists the four files, the language service reads them.
  {
    name: "open-includes",
    open: "examples/rapidq/shapes/shapes.bas",
    webFiles: SHAPES,
    do: "wait",
    delay: 5,
    dump: { "proj.kind": /^file$/, "proj.filecount": /^4$/, "projecttree.filecount": /^4$/, "lang.errorcount": /^0$/ },
  },
  // (IMPORT-INC) Imported: every include copied with it, names converted,
  // the copy a project of its four files, the report's Include files table.
  {
    name: "import-includes",
    importFrom: "examples/rapidq/shapes/shapes.bas",
    webFiles: SHAPES,
    do: "wait",
    delay: 7,
    dump: {
      "proj.importsummary": /^1 program\(s\), 4 source file\(s\) \(1 changed, 9 name\(s\)\)[\s\S]*: 1 identical, 0 different[\s\S]*include files: 3 copied, 1 RapidR's built-in, 0 not found$/,
      "proj.filecount": /^4$/,
      "projecttree.filecount": /^4$/,
      "proj.importmissing": /^$/,
      "codedoc(1).text": /^## Include files$[\s\S]*^\| `Lib\\Geometry\.INC` \| `shapes\.bas`, line 17 \| in the program's folder: `lib\/geometry\.inc` \| `lib\/geometry\.inc` \|$[\s\S]*^\| `Lib\\Units\.inc` \| `lib\/geometry\.inc`, line 4 \|/m,
      "lang.errorcount": /^0$/,
    },
  },
  // (IMPORT-INC) An include found nowhere is never passed over: Output says
  // which and what to do (ImportMissing, the status line) …
  {
    name: "import-missing-include",
    importFrom: "tests/fixtures/rapidq_import_inc/main.bas",
    webFiles: INC_FIXTURE,
    do: "wait",
    delay: 6,
    dump: {
      "proj.importsummary": /include files: 0 copied, 1 RapidR's built-in, 1 not found$/,
      "proj.importmissing": /main\.bas:5\tInc\\Tools\.INC\tadd the folder that holds Tools\.INC in RapidR Studio > Tools > Settings > Include folders/,
      "outputbox.text": /main\.bas:5: include file not found: Inc\\Tools\.INC - add the folder that holds Tools\.INC/,
    },
  },
  // … and found once its folder is one of Studio's include folders
  // (Tools > Settings > Include folders; the test step sets it): copied
  // into the copy as it was in that folder (inc/tools.inc).
  {
    name: "import-include-folder",
    importFrom: "tests/fixtures/rapidq_import_inc/main.bas",
    before: "includes:{root}/tests/fixtures/rapidq_import_lib",
    webFiles: INC_FIXTURE,
    do: "wait",
    delay: 6,
    dump: {
      "proj.importsummary": /: 1 identical, 0 different[\s\S]*include files: 1 copied, 1 RapidR's built-in, 0 not found$/,
      "proj.importmissing": /^$/,
      "proj.includefolders": /tests\/fixtures\/rapidq_import_lib$/,
      "proj.filecount": /^2$/,
      "outputbox.text": /Include folders: [^\n]*tests\/fixtures\/rapidq_import_lib/,
    },
  },
  // (IMPORT-INC, the web) File > Import RapidQ Project or File with several
  // files picked at once — the program and its include, side by side at the
  // top of the page's store as the browser gives them: the program
  // imported, `Inc\Tools.INC` found by its name and copied with it.
  {
    name: "import-several-files",
    webOnly: true,
    webFiles: ["tests/fixtures/rapidq_import_lib/inc/tools.inc=>tools.inc", "tests/fixtures/rapidq_import_inc/main.bas=>main.bas"],
    folder: "tools.inc;main.bas",
    do: "file.importRapidQ,wait",
    delay: 6,
    dump: {
      "proj.importsummary": /: 1 identical, 0 different[\s\S]*include files: 1 copied, 1 RapidR's built-in, 0 not found$/,
      "proj.importmissing": /^$/,
      "proj.mainfile": /^main\.bas$/,
      "proj.filecount": /^2$/,
    },
  },
  // (S-DESIGN-2) The menu editor, on the form's own menu bar: Format >
  // Menu Editor gives hello_form a QMAINMENU; typing on its Type Here makes
  // File (its & mnemonic), Enter goes into its menu: Open… with Ctrl+O typed
  // in the ShortCut field (Tab), a separator, Exit — each item a QMENUITEM
  // CREATE block, one undo step.
  {
    name: "designer-menu",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,designer.menuEditor",
    events: typed("&File{Enter}&Open...{Tab}{Ctrl+O}{Enter}-{Enter}E&xit{Enter}{Escape}"),
    delay: 4,
    dump: {
      "codedoc(0).text": /    CREATE MainMenu1 AS RMainMenu\n        CREATE File1 AS RMenuItem\n            Caption = "&File"\n            CREATE Open1 AS RMenuItem\n                Caption = "&Open\.\.\."\n                ShortCut = "Ctrl\+O"\n            END CREATE\n            CREATE N1 AS RMenuItem\n                Caption = "-"\n            END CREATE\n            CREATE Exit1 AS RMenuItem\n                Caption = "E&xit"\n            END CREATE\n        END CREATE\n    END CREATE\n/,
    },
  },
  // (S-DESIGN-2) The Tab-order editor: GreetButton clicked first, then
  // NameEdit — only GreetButton's TabOrder written (TabOrder = 0 puts it
  // first, the others after it in their order, as RapidQ's TabOrder does).
  {
    name: "designer-taborder",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,designer.tabOrder",
    events: `${click(150, 60)},${click(150, 22)}`,
    delay: 4,
    dump: {
      "designdoc(0).tabordermode": /^(-1|1|True)$/i,
      "designdoc(0).statustext": /^NameEdit: Tab order 1$/,
      "codedoc(0).text": /    CREATE NameEdit AS REdit\n        Text = "World"\n        Left = 112: Top = 16: Width = 200\n        OnChange = NameChanged\n    END CREATE\n    CREATE GreetButton AS RButton\n[\s\S]*        OnClick = Greet\n        TabOrder = 0\n    END CREATE/,
    },
  },
  // (S-DESIGN-2) A caption edited in place: GreetButton clicked, then
  // clicked again (a slow click) — its caption typed over, Enter writes it.
  {
    name: "designer-caption",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer",
    events: `${click(150, 60)},${click(150, 60)},${typed("Say &hi{Enter}")}`,
    delay: 4,
    dump: { "designdoc(0).editing": /^(0|False)$/i, "codedoc(0).text": /    CREATE GreetButton AS RButton\n        Caption = "Say &hi"\n/ },
  },
  // (S-DESIGN-2) Smart guides while dragging: Answer held and moved a
  // little — its left edge lines up with NameLabel's (the capture shows the
  // guide; the button isn't let go).
  {
    name: "designer-guides",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer",
    events: [`__mousedown_${at(100, 100)}`, `__mousemove_${at(104, 104)}`, `__mousemove_${at(103, 106)}`].map((e) => `designdoc(0).${e}`).join(","),
    delay: 4,
    dump: { "designdoc(0).guides": /^(edge|centre|baseline|margin|spacing \d+) [xy] -?\d+/ },
  },
  // (S-DESIGN-2) Zoom: View > Zoom In twice, then Ctrl+− and Ctrl+= on the
  // designer (110 %, 125 %, 110 %, 125 %).
  {
    name: "designer-zoom",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,designer.zoomIn,designer.zoomIn",
    events: typed("{Ctrl+-}{Ctrl+=}"),
    delay: 4,
    dump: { "designdoc(0).zoom": /^125$/, "designdoc(0).statustext": /^Zoom 125 %$/ },
  },
  // (S-DESIGN-2) notepad.bas makes its OpenDialog and SaveDialog outside
  // the form: they show in its tray, and selecting one inspects it.
  {
    name: "designer-tray",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,pick:SaveDialog,wait",
    delay: 4,
    dump: { "inspector.target": /^SaveDialog$/, "designdoc(0).statustext": /SaveDialog \(QSAVEDIALOG\)/ },
  },
  // (S-DESIGN-2) A console program has no form: Project > Add Form gives it
  // one (its CREATE block and ShowModal at the end), designed at once.
  {
    name: "designer-addform",
    open: "examples/basics/hello.rr",
    do: "wait,view.designer,project.addForm,designer.add.QBUTTON",
    delay: 4,
    dump: {
      "designdoc(0).formname": /^Form1$/,
      "codedoc(0).text": /\nCREATE Form1 AS RForm\n    Caption = "Form1"\n    Width = 320\n    Height = 240\n    CREATE Button1 AS RButton\n[\s\S]*END CREATE\n\nForm1\.ShowModal\n?$/,
    },
  },
  // (S-DESIGN-2) One undo history for the file: a designer change, a
  // change typed in the code, another designer change; Undo (from the
  // code) takes back only the last, in the order they were made.
  {
    name: "designer-undo-interleave",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,pick:AddBtn,prop:Left=320,wait,view.code,edit.undo,wait',
    delay: 7,
    dump: { "codedoc(0).text": /    CREATE AddBtn AS RButton\n        Caption = "Store it": Left = 314: Top = 252: Width = 120\n/ },
  },
  // (S-DESIGN-2) …and three Undos: the file's exact text; Redo twice: the
  // designer's Width and the typed Caption back, in order.
  {
    name: "designer-undo-all",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,pick:AddBtn,prop:Left=320,wait,edit.undo,edit.undo,edit.undo,wait',
    delay: 8,
    dump: { "codedoc(0).text": /CREATE AddBtn AS RButton/ },
    same: { "codedoc(0).text": "examples/gui/pantry.rr" },
  },
  {
    name: "designer-redo",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,edit.undo,edit.undo,edit.redo,edit.redo,wait',
    delay: 8,
    dump: { "codedoc(0).text": /    CREATE AddBtn AS RButton\n        Caption = "Store it": Left = 314: Top = 252: Width = 120\n/ },
  },
  // (S-DESIGN-2, Robert: "keep going until I can add new forms") A new
  // form program; Project > Add Form (Form2.rr, named in the project tree:
  // Enter) — the main file includes it, it opens on its designer; an
  // RLabel, an REdit and an RButton dropped on it with their captions, the
  // button's OnClick written (it closes Form2); Form1 gets a button whose
  // OnClick shows Form2. Saved: the program's two files, RapidR's names, no
  // errors (tests/studio_add_form.mjs runs what was made).
  {
    name: "add-form",
    open: "",
    do: [
      "newproject:gui|{dir}|Multi", "wait", "wait",
      "project.addForm", "wait", "key:Enter", "wait", "wait", "wait",
      "tool:RLABEL", "prop:Caption=Hello from Form2",
      "tool:REDIT", "prop:Text=Type here",
      "tool:RBUTTON", "prop:Caption=Close", "event:OnClick", "wait", "type:Form2.Close",
      "open:main.rr", "wait", "view.designer", "wait",
      "tool:RBUTTON", "prop:Caption=Show Form2", "event:OnClick", "wait", "type:Form2.Show",
      "file.saveAll", "wait", "wait", "wait",
    ].join(","),
    // (the web's analysis of the new form can take a few seconds on a
    // loaded machine: the count settles after it)
    delay: 16,
    dump: {
      "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Form2\.rr"\n[\s\S]*SUB Button2Click\n    Form2\.Show\nEND SUB[\s\S]*CREATE Form1 AS RForm[\s\S]*    CREATE Button2 AS RButton\n        Caption = "Show Form2"[\s\S]*OnClick = Button2Click/,
      "codedoc(1).text": /SUB Button1Click\n    Form2\.Close\nEND SUB[\s\S]*CREATE Form2 AS RForm[\s\S]*    CREATE Label1 AS RLabel\n        Caption = "Hello from Form2"[\s\S]*    CREATE Edit1 AS REdit\n[\s\S]*Text = "Type here"[\s\S]*    CREATE Button1 AS RButton\n        Caption = "Close"[\s\S]*OnClick = Button1Click/,
      "proj.filecount": /^2$/,
      "lang.errorcount": /^0$/,
    },
  },
  // (S-DESIGN-2, Robert's report) RForm in the toolbox is Project > Add
  // Form (a form is a document, not a component); RFormMDI adds an MDI
  // main window.
  {
    name: "toolbox-form",
    open: "",
    do: "newproject:gui|{dir}|Tb,wait,wait,tool:RFORM,wait,key:Enter,wait,wait,wait,tool:RFORMMDI,wait,key:Enter,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Form2\.rr"\n\$INCLUDE "Form3\.rr"\n/, "codedoc(1).text": /\nCREATE Form2 AS RForm\n/, "codedoc(2).text": /\nCREATE Form3 AS RFormMDI\n/, "proj.filecount": /^3$/ },
  },
  // (S-DESIGN-2) RForm dragged onto a designed form: never nested — a new
  // window, and the status bar says why; onto an RFormMDI: one of its child
  // windows, RapidQ's way.
  {
    name: "toolbox-form-drop",
    open: "",
    do: "newproject:gui|{dir}|Dr,wait,wait,drop:RFORM|designdoc(0),wait,key:Enter,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(1).text": /\nCREATE Form2 AS RForm\n/, "outputbox.text": /A form can't go inside a form: Form2 was added as a new window\. For child windows, make Form1 an RFormMDI\./ },
  },
  {
    name: "toolbox-form-drop-mdi",
    open: "",
    do: "newproject:mdi|{dir}|Md,wait,wait,drop:RFORM|designdoc(0),wait,wait,wait",
    delay: 7,
    // (onto the MDI template's window, Main: a child window, RapidQ's way —
    // a panel on Main and Main.AddChild after it)
    dump: { "codedoc(0).text": /\n    CREATE Form1 AS RPanel\n        Left = 0\n        Top = 0\n        Width = 320\n        Height = 240\n    END CREATE\nEND CREATE\nMain\.AddChild\(Form1\.Handle, "Form1", 0, 0, 0, 0, 0, 1\)\n/, "outputbox.text": /Form1 is a child window of Main/, "proj.filecount": /^1$/ },
  },
  // (S-DESIGN-2, Robert's report) The form itself (nothing selected) in
  // the inspector, with its events: OnShow's handler made, bound and
  // written as a component's is.
  {
    name: "form-events",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,wait,page:events,event:OnShow,wait",
    delay: 6,
    dump: { "inspector.target": /^Form$/, "codedoc(0).text": /^(?=[\s\S]*\n        OnShow = FormShow\n|[\s\S]*\n    OnShow = FormShow\n)(?=[\s\S]*\nSUB FormShow\b)/ },
  },
  // (S-DESIGN-2) Project > Add Module: Module1.rr, named in the tree,
  // included by the main file, opened on its code.
  {
    name: "add-module",
    open: "",
    do: "newproject:gui|{dir}|Mods,wait,wait,project.addModule,wait,key:Enter,wait,wait,wait",
    delay: 6,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Module1\.rr"\n\nCREATE Form1 AS RForm\n/, "codedoc(1).text": /^' Module1\.rr: SUBs and FUNCTIONs the program's files share$/, "proj.filecount": /^2$/, "lang.errorcount": /^0$/ },
  },
  // (S-DESIGN-2) A form's file renamed in the project tree (F2): the main
  // file's $INCLUDE follows it; then taken out of the project (Delete,
  // confirmed with Enter): the main file no longer includes it.
  {
    name: "rename-form",
    open: "",
    do: "newproject:gui|{dir}|Ren,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,rename:Form2.rr|About.rr,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "About\.rr"\n/, "proj.filecount": /^2$/, "lang.errorcount": /^0$/ },
  },
  {
    name: "remove-form",
    open: "",
    do: "newproject:gui|{dir}|Rem,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,remove:Form2.rr,wait,key:Enter,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\nCREATE Form1 AS RForm\n/, "proj.filecount": /^1$/, "lang.errorcount": /^0$/ },
  },
  // (S-DESIGN-2) A program with an $INCLUDE on the web: the language
  // service and the designer read the included file from the page's store
  // (rapidr_preprocessor's source reader), as the desktop reads the disk —
  // no "missing include" error, the form designed, its button's SUB known.
  {
    name: "include-web",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,wait,designer.add.QCHECKBOX,wait,wait",
    delay: 5,
    dump: { "lang.errorcount": /^0$/, "outputbox.problemcount": /^0$/, "designdoc(0).formname": /^Form$/, "codedoc(0).text": /    CREATE CheckBox1 AS QCHECKBOX\n/ },
  },
  // (S-SHELL-2) Documents are tabs, never windows: a form's file is one
  // tab with the Design | Code switch (no MDI window, no "[Design]"
  // document); F12 toggles to the code, then both side by side —
  // the dock's layout says each view.
  {
    name: "tabs-design-code",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.toggleDesigner,wait",
    delay: 4,
    dump: { "dock.documentmode": /^tabs$/, "dock.documentcount": /^1$/, "dock.layout": /^documents 0 codedoc\(0\)\n[\s\S]*^view codedoc\(0\) 1 500$/m },
  },
  {
    name: "side-by-side",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.sideBySide,wait",
    delay: 4,
    dump: { "dock.layout": /^view codedoc\(0\) split 500$/m, "designdoc(0).formname": /^Form$/ },
  },
  // (S-SHELL-2) A tab moved to a new group on the right (Window > Split
  // Right, as dragging it to the right edge): two groups side by side.
  {
    name: "split-groups",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,view:Split,open:greeting.inc,view.splitVertically,wait",
    delay: 4,
    dump: { "dock.documentgroupcount": /^2$/, "dock.layout": /^groups split row\n  1000 group 0 codedoc\(0\)\n  1000 group 0 codedoc\(1\)$/m, "dock.activedocument": /^codedoc\(1\)$/ },
  },
  // (S-SHELL-2) …and Studio started again on the same settings (not
  // --fresh): the project's files open again, the groups and the
  // side-by-side view as they were.
  {
    name: "restore-layout",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,view:Split,open:greeting.inc,view.splitVertically,wait,wait,wait,wait,wait,wait",
    delay: 5,
    restart: { do: "wait,wait", delay: 5 },
    dump: { "dock.documentgroupcount": /^2$/, "dock.documentcount": /^2$/, "dock.layout": /^groups split row\n[\s\S]*^view codedoc\(0\) split 500$/m },
  },
  // (Preview 4 S1, Robert's seven-file program) Forms come from the code:
  // the project file says main.rr is the form and ui.rr a module, but
  // ui.rr makes three forms and main.rr none. The tree (what a screen
  // reader is told) lists ui.rr under Forms with its three forms, main.rr
  // under Modules; the keyboard walks to NewForm (Home, Down × 4) and Enter
  // opens it in ui.rr's designer — the designer says which form of three.
  {
    name: "forms-from-code",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    events: "projecttree.__key_36,projecttree.__key_40,projecttree.__key_40,projecttree.__key_40,projecttree.__key_40,projecttree.__key_13",
    delay: 6,
    a11y: [
      ["ui.rr", "treeitem", null, (n) => n.description === "form, 3 forms"],
      ["Form, RForm", "treeitem", null, (n) => n.description === "form"],
      ["NewForm, RForm", "treeitem", "selected"],
      ["CfgForm, RForm", "treeitem"],
      ["main.rr", "treeitem", null, (n) => n.description === "module, the main file"],
      [/^Designing NewForm, form 2 of 3 in this file$/, "status"],
    ],
    dump: { "projecttree.selected": /^ui\.rr#NewForm$/, "designdoc(1).formname": /^NewForm$/, "designdoc(1).formnames": /^Form\nNewForm\nCfgForm$/, "designdoc(0).formname": /^$/, "dock.activedocument": /^codedoc\(1\)$/, "inspector.target": /^NewForm$/ },
  },
  // (S1) …and Save (Ctrl+S on any file) writes the project file as the
  // code says: main.rr a module, ui.rr a form file; the Output says so.
  {
    name: "forms-follow-save",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait,file.save,wait",
    delay: 5,
    files: { "Shelf.rrproj": /path = "main\.rr"\nkind = "module"[\s\S]*path = "ui\.rr"\nkind = "form"/ },
    dump: { "outputbox.text": /main\.rr makes no form: now under Modules\nui\.rr makes a form: now under Forms\nSaved \S*Shelf\.rrproj/ },
  },
  // (Preview 4 S2) File > Open Recent: Studio started again (as from the
  // Dock: no project named), the menu opened from the keyboard (Alt+F, R)
  // lists the project with its folder — its items are MI(14) … MI(23),
  // "No Recent Projects or Files" MI(24), Clear Recent MI(26) — and the
  // Welcome page offers to reopen it, the keyboard on the offer.
  {
    name: "open-recent",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait,open:ui.rr,wait,wait",
    delay: 4,
    restart: { noOpen: true, events: "studio.__key_70_1,studio.__key_82", delay: 4 },
    // (the open menu is in the tree: File's menu with its items — checked,
    // disabled, with their keys — and no node but the chosen item has the focus)
    a11y: [
      [/^Reopen Shelf, and the 2 files you had open$/, "button"],
      ["File", "menuitem", "expanded"],
      ["File", "menu", null, (n) => (n.children || []).some((i) => i.role === "menuitem" && /Open Recent/.test(i.name) && (i.states || []).includes("expanded"))],
      [/Open Recent/, "menuitem", "expanded"],
      [/^1 {2}Shelf\.rrproj {2}— {2}/, "menuitem", "focused"],
      ["New Project...", "menuitem", null, (n) => n.accelerator === "Ctrl+N"],
      ["Save All", "menuitem", null, (n) => n.accelerator === "Ctrl+Shift+S"],
    ],
    dump: { "mi(13).caption": /^Open &Recent$/, "mi(14).caption": /^&1 {2}Shelf\.rrproj {2}— {2}\S+$/, "mi(14).visible": /^(1|-1)$/, "mi(15).visible": /^0$/, "mi(24).visible": /^0$/, "mi(26).caption": /^&Clear Recent$/, "mi(26).enabled": /^(1|-1)$/, "reopenrow.visible": /^(1|-1)$/, "recentnone.visible": /^0$/, "studio.caption": /^RapidR Studio$/ },
  },
  // (S2) …its first item picked from the keyboard (Alt+F, R, 1): open.
  {
    name: "open-recent-pick",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait",
    delay: 3,
    restart: { noOpen: true, events: "studio.__key_70_1,studio.__key_82,studio.__key_49", delay: 4 },
    dump: { "studio.caption": /^Shelf - RapidR Studio$/, "reopenrow.visible": /^0$/ },
  },
  // (S2) …Enter on the offer: the project and the two files open again.
  {
    name: "reopen-last",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait,open:ui.rr,wait,wait",
    delay: 4,
    restart: { noOpen: true, events: "reopenrow.__key_13", delay: 4 },
    dump: { "studio.caption": /^Shelf - RapidR Studio$/, "dock.documentcount": /^2$/, "reopenrow.visible": /^0$/ },
  },
  // (A11Y) The Welcome page's Recent links are keyboard stops (a label
  // given TabStop is a link). With the offer to reopen the last project
  // there, Shift+Tab from it reaches the project's link — a screen reader's
  // focus is on a link named by the file —…
  {
    name: "welcome-recent-link-keyboard",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait",
    delay: 3,
    restart: { noOpen: true, events: "studio.__key_9_256", delay: 4 },
    // (the ring round the focused link, in every look, the page's text judged too)
    themes: LOOKS,
    contrast: true,
    capture: true,
    a11y: [["Shelf", "link", "focused", (n) => (n.actions || []).includes("click") && (n.description || "").endsWith("Shelf.rrproj")], [/^Reopen Shelf/, "button"]],
    dump: { "studio.caption": /^RapidR Studio$/, "recentlink(0).tabstop": /^(1|-1)$/, "recentlink(0).visible": /^(1|-1)$/ },
  },
  // …and without it (turned off in Settings), Shift+Tab from the Project
  // pane walks back over the example cards (17) to the same link (18 times); Enter on
  // it opens the project, as a click does.
  {
    name: "welcome-recent-link-enter",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait,view.settings,wait",
    events: "reopenbox.__mousedown_6_6,reopenbox.__mouseup_6_6,settingsokbtn.__key_13",
    delay: 4,
    restart: { noOpen: true, do: "wait", delay: 4, events: [...Array(18).fill("studio.__key_9_256"), "studio.__key_13"].join(",") },
    dump: { "studio.caption": /^Shelf - RapidR Studio$/, "reopenrow.visible": /^0$/ },
  },
  // (S2) …Clear Recent (Alt+F, R, C): the list is empty — the menu says
  // so, the Welcome page too; the offer stays (the files are there).
  {
    name: "clear-recent",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait",
    delay: 3,
    restart: { noOpen: true, events: "studio.__key_70_1,studio.__key_82,studio.__key_67,studio.__key_70_1,studio.__key_82", delay: 4 },
    dump: { "recentnone.visible": /^(1|-1)$/, "outputbox.text": /Open Recent cleared/, "mi(14).visible": /^0$/, "mi(24).visible": /^(1|-1)$/, "mi(24).enabled": /^0$/, "mi(26).enabled": /^0$/ },
  },
  // (S2) The offer turned off (Tools > Settings > General): Studio starts
  // on the Welcome page as it always did, the Project pane focused.
  {
    name: "reopen-off",
    open: "tests/fixtures/studio_forms/Shelf.rrproj",
    copyDir: true,
    webFiles: SHELF_FILES,
    do: "wait,view.settings,wait",
    events: "reopenbox.__mousedown_6_6,reopenbox.__mouseup_6_6,settingsokbtn.__key_13",
    delay: 4,
    restart: { noOpen: true, do: "wait", delay: 3, events: "" },
    dump: { "reopenrow.visible": /^0$/, "studio.caption": /^RapidR Studio$/ },
  },
  // (Preview 4 S6) Run gets your login shell's PATH: a stand-in shell
  // (it prints a greeting around it, as startup files do) — the program's
  // ENVIRON$("PATH") is its PATH, its SHELL finds sh; Tools > Settings
  // shows the folders and where they came from.
  {
    name: "run-login-path",
    open: "tests/fixtures/studio_path/path_check.rr",
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_LOGIN_PATH: "", RAPIDR_LOGIN_SHELL: join(ROOT, "tests/fixtures/studio_path/login_shell.sh") },
    do: "wait,run.start,wait,wait,wait,view.settings,wait",
    delay: 6,
    dump: {
      "outputbox.text": /PATH=\/opt\/rapidr-test\/bin:\/usr\/bin:\/bin:\/usr\/sbin:\/sbin[^\n]*\n(brew: not found|\S*brew)\n\/bin\/sh\n/,
      "pathlist.itemcount": /^([5-9]|\d\d+)$/,
      "pathnote.caption": /^Programs run from Studio, and their SHELL and RUN commands, find commands in these folders, in this order\. From your login shell \(\S*login_shell\.sh\), as Terminal has it\.$/,
      "pathrefresh.visible": /^(1|-1)$/,
    },
  },
  // (S6) A login shell that never answers: given three seconds, then
  // Studio's own PATH and the usual folders — Settings says why.
  {
    name: "run-login-path-hung",
    open: "tests/fixtures/studio_path/path_check.rr",
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_LOGIN_PATH: "", RAPIDR_LOGIN_SHELL: join(ROOT, "tests/fixtures/studio_path/hung_shell.sh") },
    do: "wait,view.settings,wait",
    delay: 7,
    dump: { "pathnote.caption": /Your login shell couldn't say \(\S*hung_shell\.sh didn't answer in 3 seconds\): Studio's own PATH and the usual folders that are on this computer\.$/ },
  },
  // (S-SHELL-2, CMD-3) Find in Files over a project and the file it
  // includes: the results by file, the open editor's text and the disk's.
  {
    name: "find-in-files",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,find:SayGreeting",
    delay: 4,
    dump: { "searchinfo.caption": /^3 results in 2 files$/, "searchtree.itemcount": /^5$/ },
  },
  // (Preview 4 S3) Search you can see: the Search tab beside Project —
  // Ctrl+Shift+F, the word typed, Enter — the whole project's files (a
  // data file too), each result file:line:col with the match marked; a
  // result picked with the keyboard (Down to it, Enter) opens its file
  // with the match selected
  {
    name: "search-tab",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    themes: LOOKS,
    contrast: true,
    capture: true,
    do: "wait,key:Ctrl+Shift+F,type:Counter,key:Enter,wait",
    events: "searchtree.__node_6,searchtree.__key_13",
    delay: 4,
    dump: {
      "searchinfo.caption": /^9 results in 5 files$/,
      "searchtree.itemcount": /^14$/,
      // (2.121.1: rows with room, as the other trees)
      "searchtree.itemheight": /^24$/,
      "dock.activedocument": /^codedoc\(1\)$/,
      "codedoc(1).filename": /server\.rr$/,
      "codedoc(1).seltext": /^Counter$/,
    },
  },
  // (S3) The scope: the modules only (main.rr, globals.rr, server.rr —
  // not the form's file, not the notes), then the forms only
  {
    name: "search-scope-modules",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,key:Ctrl+Shift+F,type:Counter,key:Enter,wait",
    events: "searchscope.__item_2",
    delay: 4,
    dump: { "searchinfo.caption": /^6 results in 3 files$/, "searchscope.itemindex": /^2$/ },
  },
  {
    name: "search-scope-forms",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,key:Ctrl+Shift+F,type:Counter,key:Enter,wait",
    events: "searchscope.__item_3",
    delay: 4,
    dump: { "searchinfo.caption": /^2 results in 1 file$/, "searchtree.itemcount": /^3$/ },
  },
  // (S3) "A folder...": the folder picked (the test's file dialog answers
  // tests/fixtures/studio_split), its text files searched — another
  // project's
  {
    name: "search-scope-folder",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    desktopOnly: true,
    folder: "tests/fixtures/studio_split",
    do: "wait,key:Ctrl+Shift+F,type:SayGreeting,key:Enter,wait",
    events: "searchscope.__item_6",
    delay: 4,
    dump: { "searchinfo.caption": /^3 results in 2 files$/, "searchscope.itemindex": /^6$/ },
  },
  // (S3) A stale project file: the results' icons are the Project pane's,
  // by the code (window.rr makes the form: a form's icon; tools.rr makes
  // none), and the Forms and Modules scopes go by the code too
  {
    name: "search-stale-kinds",
    open: "tests/fixtures/studio_stale/Stale.rrproj",
    webFiles: STALE_FILES,
    do: "wait,key:Ctrl+Shift+F,type:Total,key:Enter,wait,icons:",
    delay: 4,
    dump: {
      "searchinfo.caption": /^10 results in 3 files$/,
      "outputbox.text": /icon main\.rr=file:[^\n]*\nicon tools\.rr=file:[^\n]*\nicon window\.rr=kind:form/,
    },
  },
  {
    name: "search-stale-forms",
    open: "tests/fixtures/studio_stale/Stale.rrproj",
    webFiles: STALE_FILES,
    do: "wait,key:Ctrl+Shift+F,type:Total,key:Enter,wait",
    events: "searchscope.__item_3",
    delay: 4,
    dump: { "searchinfo.caption": /^5 results in 1 file$/, "searchtree.itemcount": /^6$/ },
  },
  {
    name: "search-stale-modules",
    open: "tests/fixtures/studio_stale/Stale.rrproj",
    webFiles: STALE_FILES,
    do: "wait,key:Ctrl+Shift+F,type:Total,key:Enter,wait",
    events: "searchscope.__item_2",
    delay: 4,
    dump: { "searchinfo.caption": /^5 results in 2 files$/, "searchtree.itemcount": /^7$/ },
  },
  // (S3) Find References (Shift+F12) on a global in a module: every use in
  // every file of the program — the language service's, not a text search
  // (the comment and the form's caption that say "Counter" aren't uses) —
  // in the Search tab, by file
  {
    name: "find-references",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:ui.rr,view:Code,wait,caret:STR$(Coun,key:Shift+F12,wait",
    delay: 4,
    dump: { "searchinfo.caption": /^References to Counter: 6 in 4 files$/, "searchtree.itemcount": /^10$/ },
  },
  // (S3) Go to Definition (F12) from a module jumps to the DIM in another
  // file; Navigate Back returns to where it was asked. (Back and Forward
  // are Alt+Left / Alt+Right, on a Mac — these tests' — Cmd+Option+Left /
  // Right: `key:Ctrl+…` is Cmd there)
  {
    name: "go-to-definition",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:ui.rr,view:Code,wait,caret:STR$(Coun,key:F12,wait",
    delay: 4,
    dump: { "dock.activedocument": /^codedoc\(2\)$/, "codedoc(2).filename": /globals\.rr$/, "codedoc(2).caretline": /^2$/ },
  },
  {
    name: "navigate-back",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:ui.rr,view:Code,wait,caret:STR$(Coun,key:F12,wait,key:Ctrl+Alt+Left,wait",
    delay: 4,
    dump: { "dock.activedocument": /^codedoc\(1\)$/, "codedoc(1).filename": /ui\.rr$/, "codedoc(1).caretline": /^18$/ },
  },
  // (S3) …and in the same file: F12 on the handler's name in its CREATE
  // goes to its SUB (line 16); Alt+Left comes back to the CREATE's line
  // (12), Alt+Right goes to the SUB again
  {
    name: "navigate-back-in-file",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:ui.rr,view:Code,wait,focus:codedoc(1),caret:OnClick = ShowCo,key:F12,wait,key:Ctrl+Alt+Left,wait,key:Ctrl+Alt+Right,wait",
    delay: 7,
    dump: { "dock.activedocument": /^codedoc\(1\)$/, "codedoc(1).caretline": /^16$/ },
  },
  // (S3) On a Mac, Option+Left still moves the caret by words in the code
  // (Back isn't on it there): after F12 to the SUB (line 16), the caret
  // stays on line 16
  {
    name: "option-left-moves-by-words",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:ui.rr,view:Code,wait,focus:codedoc(1),caret:OnClick = ShowCo,key:F12,wait,key:Alt+Left,wait",
    delay: 6,
    dump: { "codedoc(1).caretline": /^16$/ },
  },
  // (S5) Problems that tell the truth: a module open in the editor is
  // checked as part of its project (no errors where the program compiles),
  // and a desktop project hears nothing about what works on the desktop only
  {
    name: "problems-project",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    do: "wait,open:server.rr,wait,view.problems,wait",
    delay: 4,
    dump: { "lang.errorcount": /^0$/, "outputbox.problemcount": /^0$/, "lang.targets": /^desktop$/ },
  },
  // (S5) Targets = Web (Project Options, picked and OK'd with the keys):
  // one note for RServerSocket with how many uses, not one per call
  {
    name: "problems-targets-web",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    copyDir: true,
    webFiles: SEARCH_FILES,
    do: "wait,project.options",
    events: "apptargets.__item_1,appdialog.__key_13",
    delay: 5,
    dump: { "proj.targets": /^web$/, "lang.targets": /^web$/, "outputbox.problemcount": /^1$/ },
    files: { "Counter.rrproj": /\nruns_on = "web"\n/ },
  },
  // (S5) …and what that one note says, on the Problems page: under its
  // file's heading, at the first use, with how many uses
  {
    name: "problems-web-note",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    webFiles: SEARCH_FILES,
    themes: LOOKS,
    contrast: true,
    capture: true,
    do: "wait,targets:web,view.problems,wait",
    delay: 4,
    dump: {
      "outputbox.problemcount": /^1$/,
      "outputbox.text": /^\S*globals\.rr: 1 note\n\S*globals\.rr:3:\d+: info: RServerSocket works on the desktop only: the web runtime doesn't have it \(3 uses\)$/,
    },
  },
  // (S5) Problems opens at its top: 27 problems, more than the page shows —
  // the first row (the first file's heading) in view, not the last
  {
    name: "problems-top",
    open: "tests/fixtures/studio_problems/Problems.rrproj",
    webFiles: PROBLEM_FILES,
    do: "wait,targets:web,view.problems,wait",
    delay: 5,
    dump: { "outputbox.problemcount": /^27$/, "outputbox.topline": /^0$/, "outputbox.text": /^\S*a\.rr: 13 warnings\n/ },
  },
  // (S5) Errors first, grouped by file: an error typed into the form's
  // module comes before the notes of the project's other files
  {
    name: "problems-errors-first",
    open: "tests/fixtures/studio_search/Counter.rrproj",
    copyDir: true,
    webFiles: SEARCH_FILES,
    do: "wait,targets:web,open:ui.rr,view:Code,wait,code:StopServer=>StopServr,wait,view.problems,wait",
    delay: 5,
    dump: {
      "lang.errorcount": /^1$/,
      "outputbox.text": /^\S*ui\.rr: 1 error\n\S*ui\.rr:\d+:\d+: error: [^\n]*StopServr[^\n]*\n\S*globals\.rr: 1 note\n/,
    },
  },
  // (S-SHELL-2, HLP-1; 2.121.1) F1: the registry's entry in the Help pane — a
  // search box on top (the word asked in it), one page below: the title,
  // what it is, the syntax with Insert under it, what it does, a
  // component's members as links. For a statement, and for the word at the
  // caret in the code (a member of the component before the dot).
  {
    name: "help",
    open: "examples/gui/hello_form.rr",
    do: "wait,help:SHOWMESSAGE",
    delay: 4,
    dump: {
      "helppage.plaintext": /^SHOWMESSAGE\nStatement · RapidQ-compatible[^\n]*\n[^\n]*\nInsert into the code\n/,
      "helpsearch.text": /^SHOWMESSAGE$/,
      "helpsuggest.visible": NO,
    },
  },
  {
    name: "help-f1",
    open: "examples/gui/hello_form.rr",
    do: 'wait,view.code,code:Answer.Caption=>Answer.Caption,help.contents',
    delay: 4,
    dump: { "helppage.plaintext": /^RLabel\.Caption\nA member of RLabel\.\nProperty of RLabel/, "helpsearch.text": /^Caption$/ },
  },
  {
    // a component: its members under headings, each name a link
    name: "help-component",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,help:RButton",
    delay: 4,
    dump: {
      "helppage.plaintext": /^RButton\nComponent · RapidQ name: QBUTTON · desktop and web\nIts default event is OnClick\.\nCREATE Button1 AS RButton\n[\s\S]*\nInsert into the code\nA push button[^\n]*\nProperties \(\d+\)\nAlign · [^\n]*Caption · [^\n]*\nMethods \(\d+\)\n[^\n]*\nEvents \(\d+\)\nOnClick · /,
    },
  },
  {
    // nothing at the caret, nothing asked: how to use Help, and where to start
    name: "help-start",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    // (the search box's grey "Search the help" too: in each look, AA)
    themes: LOOKS,
    contrast: true,
    capture: true,
    do: "wait,view.code,wait,help:",
    delay: 5,
    dump: {
      "helppage.plaintext": /^Help\nPut the caret on a word in your code and press F1, or type a word in the box above\.[^\n]*\nStart with\n• RForm: a window\n• RButton[^\n]*\n[\s\S]*• IF: choose what to do\n[\s\S]*The whole manual is at the RapidR manual\.$/,
      "helpsearch.text": /^$/,
    },
  },
  {
    // the pane reached by its tab before any F1 has the start page, not a stray line
    name: "help-pane-first",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,focus:helppane",
    delay: 4,
    dump: { "helppage.plaintext": /^Help\nPut the caret on a word in your code/ },
  },
  {
    // a word nothing knows: said plainly, not "Nothing in the language is called"
    name: "help-unknown",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,help:RapidR",
    delay: 4,
    dump: { "helppage.plaintext": /^No entry for RapidR\nNothing in the language, and nothing in this program, is called RapidR\./ },
  },
  {
    // F1 on a variable of the program: what it is, how it was declared,
    // where, then its type's help; looks: light, dark, high contrast
    name: "help-own-variable",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    themes: LOOKS,
    contrast: true,
    capture: true,
    do: "wait,view.code,wait,caret:    PickedFont,help.contents,wait",
    delay: 6,
    dump: {
      "helppage.plaintext": /^PickedFont\nA variable in this program · global variable\nDIM PickedFont AS RFont\nDeclared in main\.rr, line 5 · used once\nFind References\nIts type is RFont\.\nRFont\nComponent · RapidQ name: QFONT[^\n]*\n[^\n]*\nProperties \(\d+\)\n/,
      "helpsearch.text": /^PickedFont$/,
    },
  },
  {
    // …its declaration is a link: Tab to it in the page, Enter, the caret is there
    name: "help-own-variable-goto",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,caret:    PickedFont,help.contents,wait,helpfocus:page,key:Tab,key:Enter,wait",
    delay: 7,
    dump: { "codedoc(0).caretline": /^5$/, "dock.activedocument": /^codedoc\(0\)$/ },
  },
  {
    // a SUB: its signature, its declaration, Find References
    name: "help-own-sub",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,caret:    Greet,help.contents,wait",
    delay: 6,
    dump: { "helppage.plaintext": /^Greet\nA SUB in this program\nSUB Greet\(Who AS STRING\)\nDeclared in main\.rr, line 28 · used once\nFind References$/ },
  },
  {
    // a component of the form: the form's own name, its type below
    name: "help-own-component",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,caret:    Hello,help.contents,wait",
    delay: 6,
    dump: { "helppage.plaintext": /^Hello\nA component in this program\nCREATE Hello AS RButton\nDeclared in main\.rr, line 13 · used once\nFind References\nIts type is RButton\.\nRButton\n/ },
  },
  {
    // a property after a dot of a dot: RFont's Color, not "no entry"
    name: "help-chain",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,caret:Hello.Font.Color,help.contents,wait",
    delay: 6,
    dump: { "helppage.plaintext": /^RFont\.Color\nA member of RFont\.\nProperty of RFont · color/, "helpsearch.text": /^Color$/ },
  },
  {
    // typing in the box (real keys): after a pause the names it may mean show
    // under it, the first one's page below
    name: "help-type",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,helpfocus:search,type:rbut,wait,wait",
    delay: 8,
    dump: {
      "helpsearch.text": /^rbut$/,
      "helpsuggest.visible": YES,
      "helpsuggest.itemcount": /^1$/,
      "helpsuggest.text": /^RButton {2}· {2}component\s*$/,
      "helppage.plaintext": /^RButton\nComponent/,
    },
  },
  {
    // …Enter opens the one chosen: the box says its name, the list is gone
    name: "help-type-enter",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,helpfocus:search,type:rbut,wait,wait,key:Enter,wait",
    delay: 8,
    dump: { "helpsearch.text": /^RButton$/, "helpsuggest.visible": NO, "helppage.plaintext": /^RButton\nComponent/ },
  },
  {
    // …a click on one of the names (pri: PRINT first, the shortest; then Printer)
    name: "help-type-click",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,helpfocus:search,type:pri,wait,wait",
    events: "helpsuggest.__item_1",
    delay: 9,
    dump: { "helpsearch.text": /^Printer$/, "helpsuggest.visible": NO, "helppage.plaintext": /^Printer\nObject · / },
  },
  {
    // the names of the program come with the language's
    name: "help-type-own",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,helpfocus:search,type:pick,wait,wait",
    delay: 8,
    dump: { "helpsuggest.text": /^PickedFont {2}· {2}your variable/, "helppage.plaintext": /^PickedFont\nA variable in this program/ },
  },
  {
    // a link in the page (Tab to it, Enter) opens that entry: the default event
    name: "help-link",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,help:RButton,helpfocus:page,key:Tab,key:Enter,wait",
    delay: 7,
    dump: { "helppage.plaintext": /^RButton\.OnClick\nA member of RButton\.\nEvent of RButton/, "helpsearch.text": /^OnClick$/ },
  },
  {
    // a page opened from one scrolled to its end starts at its title (the view
    // keeps its scroll as its text changes: the pane puts it back)
    name: "help-new-page-at-top",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    a11y: [["Help page", "document", ""], [/^RForm$/, "heading", "", SHOWN_IN_HELP]],
    do: "wait,help:RButton,helpfocus:page,key:End,wait,help:RForm,wait",
    delay: 7,
    dump: { "helppage.plaintext": /^RForm\n/ },
  },
  {
    // Insert (a link under the syntax) types the syntax into the code at the caret
    name: "help-insert",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,view.code,wait,caret:    PRINT,help.contents,wait,helpfocus:page,key:Tab,key:Enter,wait",
    delay: 7,
    dump: { "codedoc(0).text": /\n {4}PRINTPRINT[^\n]* Count\n/, "dock.activedocument": /^codedoc\(0\)$/ },
  },
  {
    // Help > About: the version of the build — the workspace's
    name: "about-version",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: "wait,about:text",
    delay: 4,
    dump: { "outputbox.text": new RegExp(`RapidR Studio \\(RapidR ${WORKSPACE_VERSION.replaceAll(".", "\\.")}\\)\\r?\\nBASIC is powerful again`) },
  },
  {
    // the code helpers: a colour of a sub-object, Font.Color in a CREATE body,
    // has its swatch and its editor (Ctrl+.), written back as it was (&H)
    name: "helpers-subobject-colour",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: `${HELPERS_READY},line:12,key:End,key:Ctrl+.,wait,key:Tab,key:Tab,key:Tab,key:Tab,key:Tab,type:#102030,key:Enter,wait`,
    delay: 9,
    dump: { "codedoc(0).text": /\n {4}Font\.Color = &H302010\n/, "codedoc(0).helpereditortext": /^$/ },
  },
  {
    name: "helpers-subobject-colour-swatch",
    open: HELP_OPEN,
    webFiles: HELP_FILES,
    do: `${HELPERS_READY},line:12,key:End,wait`,
    delay: 7,
    dump: { "codedoc(0).helpertext": /^Font\.Color = &H800000; / },
  },
  // (S-BUILD) The questions before changes would be lost (prompts.inc),
  // each button: one document changed and Studio quit — Save (Enter)
  // writes it and quits, Don't Save quits and leaves the file as it was,
  // Cancel (Escape) keeps Studio open with the change; several changed —
  // Save All (Enter), Discard Changes, Review Changes... (each asked:
  // Save, then Don't Save), Cancel (Escape); a document closed — Don't
  // Save closes it, Studio stays. `quit`: Studio must have quit (the
  // desktop's process ended, the web page's program ended) before the
  // test's dump; `files`: what each file holds afterwards.
  {
    name: "quit-save",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE saved",file.exit',
    events: "promptsave.__key_13",
    delay: 4,
    quit: true,
    dump: {},
    files: { "main.rr": /PRINT "ONE saved"/ },
  },
  {
    name: "quit-dont-save",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE lost",file.exit',
    events: "promptdont.__mousedown_12_12,promptdont.__mouseup_12_12",
    delay: 4,
    quit: true,
    dump: {},
    files: { "main.rr": /^(?![\s\S]*"ONE lost")[\s\S]*PRINT "ONE"/ },
  },
  {
    name: "quit-cancel",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE kept",file.exit',
    events: "promptsave.__key_27",
    delay: 4,
    dump: { "saveprompt.visible": /^0$/, "studio.modified": /^(1|-1|True)$/i, "codedoc(0).text": /PRINT "ONE kept"/, "dock.documentcount": /^1$/ },
    files: { "main.rr": /^(?![\s\S]*"ONE kept")[\s\S]*PRINT "ONE"/ },
  },
  {
    name: "quit-many-save-all",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE all",open:other.inc,wait,code:"TWO"=>"TWO all",file.exit',
    events: "promptsave.__key_13",
    delay: 5,
    quit: true,
    dump: {},
    files: { "main.rr": /PRINT "ONE all"/, "other.inc": /PRINT "TWO all"/ },
  },
  {
    name: "quit-many-discard",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE gone",open:other.inc,wait,code:"TWO"=>"TWO gone",file.exit',
    events: "promptdont.__mousedown_12_12,promptdont.__mouseup_12_12",
    delay: 5,
    quit: true,
    dump: {},
    files: { "main.rr": /^(?![\s\S]*"ONE gone")[\s\S]*PRINT "ONE"/, "other.inc": /^(?![\s\S]*"TWO gone")[\s\S]*PRINT "TWO"/ },
  },
  {
    name: "quit-many-review",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE reviewed",open:other.inc,wait,code:"TWO"=>"TWO dropped",file.exit',
    events: "promptreview.__mousedown_12_12,promptreview.__mouseup_12_12,promptsave.__key_13,promptdont.__mousedown_12_12,promptdont.__mouseup_12_12",
    delay: 5,
    quit: true,
    dump: {},
    files: { "main.rr": /PRINT "ONE reviewed"/, "other.inc": /^(?![\s\S]*"TWO dropped")[\s\S]*PRINT "TWO"/ },
  },
  {
    name: "quit-many-cancel",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE stays",open:other.inc,wait,code:"TWO"=>"TWO stays",file.exit',
    events: "promptcancel.__mousedown_12_12,promptcancel.__mouseup_12_12",
    delay: 5,
    dump: { "saveprompt.visible": /^0$/, "dock.documentcount": /^2$/, "studio.modified": /^(1|-1|True)$/i, "prompttitle.caption": /^You have 2 documents with unsaved changes/ },
    files: { "main.rr": /^(?![\s\S]*"ONE stays")/, "other.inc": /^(?![\s\S]*"TWO stays")/ },
  },
  {
    name: "close-dont-save",
    open: "tests/fixtures/studio_save/main.rr",
    copyDir: true,
    webFiles: SAVE_FILES,
    do: 'wait,code:"ONE"=>"ONE closed",file.close',
    events: "promptdont.__mousedown_12_12,promptdont.__mouseup_12_12",
    delay: 4,
    dump: { "dock.documentcount": /^0$/, "studio.modified": /^(0|False)$/i, "prompttitle.caption": /^Do you want to save the changes you made to main\.rr\?$/ },
    files: { "main.rr": /^(?![\s\S]*"ONE closed")[\s\S]*PRINT "ONE"/ },
  },
  // (MD-VIEW) A Markdown file opens formatted: its tab shows Preview (the
  // switch's first view: no "view" line in the layout), the RMarkdownView
  // has its text read (headings, links), the outline lists its headings,
  // and the language service leaves it alone.
  {
    name: "markdown-preview",
    open: "tests/fixtures/markdown/rapidr-import-report.md",
    webFiles: ["tests/fixtures/markdown/rapidr-import-report.md"],
    do: "wait",
    delay: 4,
    dump: {
      "dock.layout": /^(?![\s\S]*^view codedoc\(0\))[\s\S]*^documents \d+ (welcome )?codedoc\(0\)$/m,
      "mddoc(0).plaintext": /^RapidQ import: greeter\.rqw\nA copy of greeter\/greeter\.rqw in [\s\S]*^Program\tCompiled as the original\ngreeter\.rqw\tyes: identical bytecode$/m,
      "mddoc(0).headings": /^1\tRapidQ import: greeter\.rqw\trapidq-import-greeterrqw\t1\n2\tPrograms\tprograms\t9\n/,
      "outlinetree.itemcount": /^7$/,
      "lang.errorcount": /^0$/,
    },
  },
  // (MD-VIEW) The switch: F7 (View > Code) shows the source, an edit there
  // shows in the preview, Shift+F7 (View > Designer) goes back to Preview;
  // a link to another file opens it, in Preview too.
  {
    name: "markdown-switch",
    open: "tests/fixtures/markdown/sample.md",
    webFiles: ["tests/fixtures/markdown/sample.md", "tests/fixtures/markdown/notes.md"],
    do: "wait,view.code,code:## Lists=>## Shopping lists,wait,view.designer,wait",
    events: "mddoc(0).__item_2",
    delay: 5,
    dump: {
      "codedoc(0).text": /^## Shopping lists$/m,
      "mddoc(0).headings": /^2\tShopping lists\tshopping-lists\t7$/m,
      "dock.layout": /^documents \d+ (welcome )?codedoc\(0\) codedoc\(1\)$(?![\s\S]*^view codedoc\(0\) 1)/m,
      "dock.activedocument": /^codedoc\(1\)$/,
      "mddoc(1).plaintext": /^Notes\nA second Markdown file/,
    },
  },
  // (DEMO-CSV) An example with a data file beside it: the file is the
  // project's (the tree lists staff.csv), and the program runs — on the
  // web its $RESOURCE is built in from it and the file goes with the
  // program into its frame (Robert: "$RESOURCE STAFF_CSV: file not found").
  {
    name: "run-with-data",
    open: "examples/data/dataframe.rr",
    copyDir: true,
    do: "run.start,wait,wait,wait",
    delay: 6,
    // (the desktop: its own process, which under the capture test ends
    // itself — exit code 0 — before Studio is read)
    dump: { "projecttree.filecount": /^2$/, "session.exitcode": /^0?$/, "outputbox.text": /^(?![\s\S]*Can't run)/ },
    webDump: { "projecttree.filecount": /^2$/, "session.state": /^running$/, "outputbox.text": /^(?![\s\S]*Can't run)/ },
  },
  {
    name: "run-csv-explorer",
    open: "examples/data/csv_explorer.rr",
    copyDir: true,
    do: "run.start,wait,wait,wait",
    delay: 6,
    // (the desktop: its own process, which under the capture test ends
    // itself — exit code 0 — before Studio is read)
    dump: { "projecttree.filecount": /^2$/, "session.exitcode": /^0?$/, "outputbox.text": /^(?![\s\S]*Can't run)/ },
    webDump: { "projecttree.filecount": /^2$/, "session.state": /^running$/, "outputbox.text": /^(?![\s\S]*Can't run)/ },
  },
  // (S-PANELS) The project tree lists the form's components; the palette
  // finds a symbol of the file.
  {
    name: "tree-and-search",
    open: "examples/gui/pantry.rr",
    do: "wait,palette:stock",
    delay: 4,
    dump: { "projecttree.filecount": /^1$/, "palette.selected": /^line:20:Stock$/ },
  },
  // (SHOWCASE) the showcase programs are in Open an Example (the palette),
  // not cards on the Welcome page: "visualizer" typed finds one, Enter opens
  // it as the user's own copy, with its song beside it
  {
    name: "palette-showcase",
    open: "",
    do: "wait,palette:visualizer",
    events: "palette.__key_13",
    delay: 6,
    dump: { "palette.selected": /^more:\d+$/, "studio.caption": /^midi_visualizer - RapidR Studio$/ },
  },
  // (REQUEST-001) the background queries example too: "background" typed
  // finds it, Enter opens it as the user's own copy
  {
    name: "palette-million-rows-async",
    open: "",
    do: "wait,palette:in the background",
    events: "palette.__key_13",
    delay: 6,
    dump: { "palette.selected": /^more:\d+$/, "studio.caption": /^million_rows_async - RapidR Studio$/ },
  },
  // (RECIPES) the recipes too: "sqrt" typed finds the calculator, Enter
  // opens it as the user's own copy
  {
    name: "palette-recipe",
    open: "",
    do: "wait,palette:sqrt",
    events: "palette.__key_13",
    delay: 6,
    dump: { "palette.selected": /^more:\d+$/, "studio.caption": /^calculator - RapidR Studio$/ },
  },
  {
    // a snippet: `sub` and Tab — the skeleton, its name selected; Tab again
    // to the parameters
    name: "editor-snippet",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:sub,wait,key:Tab,type:Hello,key:Tab,type:n AS INTEGER",
    delay: 6,
    dump: { "codedoc(0).text": /\nSUB Hello\(n AS INTEGER\)\n {4}\nEND SUB\n?$/ },
  },
  {
    // F2 renames from the language service's references: the DECLARE, the
    // SUB, OnClick = and the calls
    name: "editor-rename",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F2,wait,key:Ctrl+A,type:SayHi,key:Enter,wait",
    delay: 7,
    dump: { "codedoc(0).text": /DECLARE SUB SayHi\n[\s\S]*OnClick = SayHi\n[\s\S]*\nSUB SayHi\n[\s\S]*\nSayHi\n?$/ },
  },
  {
    // Ctrl+F with a regular expression (Alt+R): found as it is typed, the
    // first match selected
    name: "editor-find-regex",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+F,wait,key:Alt+R,type:Show\\w+,wait",
    delay: 6,
    dump: { "codedoc(0).seltext": /^ShowModal$/ },
  },
  {
    // Edit > Undo takes the typing back (a word at a time), Redo again
    name: "editor-undo",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,type:one two,edit.undo,wait,edit.undo,edit.redo,wait",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\none ?\n?$/, "codedoc(0).canredo": /^(-1|1|True)$/i },
  },
  {
    // Tab at a line's start: the file's unit (4 spaces, never a tab
    // glyph); Shift+Tab takes it back
    name: "editor-tab-line-start",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Tab,type:x,key:Escape",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\n\n {4}x\n?$/ },
  },
  {
    name: "editor-shift-tab-line-start",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Tab,type:x,key:Escape,key:Shift+Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\n\nx\n?$/ },
  },
  {
    // hover: the registry's syntax and doc for a RapidQ statement
    name: "editor-hover",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:ShowMessage \"Hi\",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait",
    delay: 6,
    dump: { "codedoc(0).hovertext": /SHOWMESSAGE|ShowMessage/ },
  },
  {
    // signature help after `(`: the parameters
    name: "editor-signature",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x$ = MID$(,wait",
    delay: 6,
    dump: { "codedoc(0).signaturetext": /MID\$\(/i },
  },
  {
    // Shift+F12: every use selected here (DECLARE, OnClick =, the SUB, the
    // call) and listed in Output
    name: "editor-references",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:Shift+F12,wait",
    delay: 6,
    // (the references are in the Search tab since S3: one file's row and its 4)
    dump: { "codedoc(0).cursorcount": /^4$/, "searchinfo.caption": /^References to greet: 4 in 1 file$/, "searchtree.itemcount": /^5$/ },
  },
  {
    // Edit > Advanced > Fold All: the CREATE blocks and SUBs folded
    name: "editor-fold",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),edit.foldAll,wait",
    delay: 5,
    dump: { "codedoc(0).foldcount": /^[3-9]$/ },
  },
  {
    // the find box's search, then Edit > Find Next (F3): the next one
    name: "editor-find-next",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+F,wait,type:Caption,wait,key:Escape,edit.findNext,wait",
    delay: 6,
    dump: { "codedoc(0).caretline": /^22$/, "codedoc(0).seltext": /^Caption$/ },
  },
  {
    // (Robert: "Tab inserted a weird character") Tab takes the selected
    // member — never a TAB in the text — and closes the list; Enter then
    // writes the line's names as declared
    name: "editor-tab-accept",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x$ = nameedit.te,key:Tab,key:Enter",
    delay: 6,
    dump: { "codedoc(0).text": /^[^\t]*\nx\$ = NameEdit\.Text\n?$/, "codedoc(0).completionitems": /^$/ },
  },
  {
    // Ctrl+Space on an empty line: the program's own names (its SUBs and
    // components) with the language's
    name: "editor-ctrl-space",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Ctrl+Space,wait",
    delay: 6,
    dump: { "codedoc(0).completionitems": /^Greet$[\s\S]*^NameEdit$[\s\S]*^ShowMessage$/im },
  },
  {
    // F12 in a form's code is Go to Definition: the
    // caret on the SUB, the code still shown — F7 / Shift+F7 switch views
    name: "editor-f12-in-a-form",
    open: "examples/gui/hello_form.rr",
    do: "view.code,focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12,wait",
    delay: 6,
    dump: { "codedoc(0).caretline": /^42$/, "dock.layout": /^view codedoc\(0\) 1 \d+$/m },
  },
  {
    // (Robert's mbYes = 0) in a RapidQ program (.bas: hello_form as one), a
    // RAPIDQ.INC constant without the include: a warning in the code and in
    // Problems; Ctrl+. adds the include after the file's header comments
    name: "editor-needs-include",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,key:Left,key:Ctrl+.,wait,key:Enter,wait",
    delay: 8,
    dump: { "codedoc(0).text": /\n\n\$INCLUDE "RAPIDQ\.INC"\nDECLARE SUB Greet\n[\s\S]*\nx = mbYes\n?$/ },
  },
  {
    // …the warning listed in Problems while it is there
    name: "editor-needs-include-problems",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).diagnosticcount": /^[1-9]$/, "outputbox.problemcount": /^[1-9]$/ },
  },
  {
    // (A11Y) The debugging layout's narrow group — Variables, Watch, Call
    // Stack, Breakpoints (2) — keeps every tab's name whole in every look,
    // high contrast too (the tabs' padding gives way before a name is cut;
    // a name that still can't fit has its tooltip and its full accessible
    // name): the shown tab is a screen reader's "Breakpoints (2)"
    name: "tabs-fit-names",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,line:147,debug.toggleBreakpoint,bpcond:count > 1,run.start,wait,wait,wait,view.breakpoints,wait",
    delay: 9,
    themes: LOOKS,
    contrast: true,
    capture: true,
    a11y: [["Breakpoints (2)", "tab", "selected"], ["Variables", "tab"], ["Watch", "tab"], ["Call Stack", "tab"]],
    dump: { "dock.activepane": /^bppanel$/ },
  },
  {
    // (A11Y) …and the Problems page wraps a long message onto more rows, so
    // it reads whole (WordWrap, on unless turned off) — captured in every
    // look, the page's text judged against what it is drawn on
    name: "problems-wrap",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,wait,view.problems,view.debugPanes,wait,wait",
    delay: 8,
    themes: LOOKS,
    contrast: true,
    capture: true,
    dump: { "outputbox.page": /^problems$/, "outputbox.wordwrap": /^(1|-1|True)$/, "outputbox.problemcount": /^[1-9]$/ },
  },
  {
    // (A11Y) A long line of the program's output wraps in the Output pane —
    // one logical line (the pane's text keeps it whole), several rows to look at
    name: "output-wrap-long-line",
    open: "tests/fixtures/studio_long_output.bas",
    webFiles: ["tests/fixtures/studio_long_output.bas"],
    do: "run.start,wait,wait,wait,wait",
    delay: 8,
    themes: LOOKS,
    contrast: true,
    capture: true,
    dump: { "outputbox.page": /^output$/, "outputbox.text": /^Shipping the order to Ada Lovelace, 12 Market Street, Manchester, M1 1AA, United Kingdom, by courier on Tuesday the fourteenth of March, signed for at the front desk; the next delivery is the week after that\.\ndone/, "outputbox.wordwrap": /^(1|-1|True)$/ },
  },
  {
    // (A11Y) View > Wrap Long Output Lines from the keyboard (Alt+V, L): the
    // menu item says it is checked while it wraps (a screen reader reads a
    // check item), and it turns the wrapping off — the long message cut with
    // "…" and a tooltip
    name: "output-wrap-menu",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    events: "studio.__key_86_1",
    delay: 4,
    a11y: [["Wrap Long Output Lines", "menuitemcheckbox", "checked"], ["View", "menuitem", "expanded"]],
    dump: { "outputbox.wordwrap": /^(1|-1|True)$/ },
  },
  {
    name: "output-wrap-off",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,wait,view.problems,view.debugPanes,wait,wait,view.outputWrap,wait",
    delay: 8,
    capture: true,
    dump: { "outputbox.page": /^problems$/, "outputbox.wordwrap": /^(0|False)$/, "outputbox.problemcount": /^[1-9]$/ },
  },
  {
    // …and Studio remembers it: started again, it is still off
    name: "output-wrap-remembered",
    open: "tests/fixtures/hello_form.bas",
    copy: true,
    do: "view.outputWrap,wait",
    delay: 3,
    restart: { events: "", delay: 3 },
    dump: { "outputbox.wordwrap": /^(0|False)$/ },
  },
  {
    // a file that opens with a warning shows its squiggle as it opens — no
    // edit, no key: the code's problem count is Problems', from the start
    name: "editor-squiggles-on-open",
    open: "tests/fixtures/studio_open_warnings.bas",
    webFiles: ["tests/fixtures/studio_open_warnings.bas"],
    do: "wait,wait,wait",
    delay: 6,
    capture: true,
    dump: { "codedoc(0).diagnosticcount": /^[1-9]$/, "outputbox.problemcount": /^[1-9]$/, "codedoc(0).text": /^' RapidR Studio's squiggles on open/ },
  },
  {
    // completing a RAPIDQ.INC constant brings its include along (one step)
    name: "editor-complete-with-include",
    open: "tests/fixtures/hello_form.bas",
    webFiles: ["tests/fixtures/hello_form.bas"],
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbye,wait,key:Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\n\n\$INCLUDE "RAPIDQ\.INC"\nDECLARE SUB Greet\n[\s\S]*\nx = mbYes\n?$/ },
  },
  {
    // (M0-CONST) a RapidR program (.rr) has RAPIDQ.INC's constants built
    // in: mbYes, mtInformation, mbOK, clRed typed with no include are no
    // problem — nothing in the code, nothing in Problems
    name: "editor-builtin-constants",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes + mtInformation + mbOK + clRed,key:Escape,wait,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).text": /\nx = mbYes \+ mtInformation \+ mbOK \+ clRed\n?$/, "codedoc(0).diagnosticcount": /^0$/, "outputbox.problemcount": /^0$/ },
  },
  {
    // …and completing one adds no include (one step: the name)
    name: "editor-complete-builtin",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbye,wait,key:Tab",
    delay: 6,
    dump: { "codedoc(0).text": /^' hello_form\.rr(?![\s\S]*RAPIDQ\.INC)[\s\S]*\nx = mbYes\n?$/ },
  },
  {
    // (S-DESIGN ↔ S-EDITOR) the designer's two additions came to the code as
    // two undo steps of the editor's (ApplyPatches): Ctrl+Z twice in the
    // code gives the file back exactly, and nothing is left to undo
    name: "designer-code-undo",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,designer.add.QCHECKBOX,designer.add.QBUTTON,wait,view.code,focus:codedoc(0),key:Ctrl+Z,key:Ctrl+Z,wait",
    delay: 6,
    dump: { "codedoc(0).canundo": /^(0|False)$/i, "codedoc(0).text": /CREATE Form AS QFORM/ },
    same: { "codedoc(0).text": "examples/rapidq/notepad.bas" },
  },
  // ---- S8 / S9: the panes at start-up; the editing and debugging layouts ----
  // (S8) Studio started maximized (the window's size given after the panes
  // were placed, as a restored or scaled window at start-up): the
  // Breakpoints pane's list and its strip fill the pane, the Welcome page
  // its document — before any resize
  {
    name: "layout-startup-fills",
    desktopArgs: ["--maximized"],
    maximized: true,
    viewport: { width: 1600, height: 1000 },
    do: "wait,focus:bppanel,wait",
    delay: 4,
    capture: true,
    dump: { "bppanel.width": /^\d+$/, "bptree.width": /^\d+$/, "bpstrip.width": /^\d+$/, "bppanel.height": /^\d+$/, "bpstrip.top": /^\d+$/, "bpstrip.height": /^\d+$/, "welcome.width": /^\d+$/, "welcomepage.width": /^\d+$/, "studio.width": /^\d+$/ },
    checks: [
      (d) => [+d["bptree.width"] === +d["bppanel.width"] && +d["bpstrip.width"] === +d["bppanel.width"], `Breakpoints' list and strip fill its width (${d["bptree.width"]}, ${d["bpstrip.width"]} of ${d["bppanel.width"]})`],
      (d) => [+d["bpstrip.top"] + +d["bpstrip.height"] === +d["bppanel.height"], `its strip at its bottom (${d["bpstrip.top"]} + ${d["bpstrip.height"]} of ${d["bppanel.height"]})`],
      (d) => [+d["bppanel.width"] > 900, `the bottom panel spans the maximized window's documents (${d["bppanel.width"]} wide of ${d["studio.width"]})`],
      (d) => [Math.abs(+d["welcomepage.width"] - +d["welcome.width"]) <= 17, `the Welcome page fills its document (${d["welcomepage.width"]} of ${d["welcome.width"]})`],
    ],
  },
  {
    // (S9) Editing: the bottom panel is one group the documents' width —
    // Output, Immediate, Breakpoints —, the debug panes hidden
    name: "layout-editing",
    do: "wait",
    delay: 3,
    dump: { "dock.layout": /tabs \d+ outputbox immediatebox bppanel\n[\s\S]*hidden [^\n]*varstree[^\n]*watchpanel[^\n]*stacktree/, "outputbox.width": /^\d+$/ },
    checks: [(d) => [+d["outputbox.width"] >= 780, `Output the documents' whole width (${d["outputbox.width"]})`]],
  },
  {
    // (S9) A run with the debugger: the debugging layout — Variables,
    // Watch, Call Stack, Breakpoints beside Output —, Variables in front at
    // the pause, the editor at the paused line
    name: "layout-debug-paused",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    dump: {
      "session.state": /^paused$/,
      "dock.layout": /split row\n\s+\d+ tabs \d+ outputbox immediatebox\n\s+\d+ tabs 0 varstree watchpanel stacktree bppanel\n/,
      "varstree.visible": /^-1$/,
      "watchpanel.visible": /^0$/,
      "varstree.text": /^Locals/,
    },
  },
  {
    // (S9) …and when the program ends, the editing layout again; View >
    // Debug Panes (Ctrl+Shift+D) brings them back by hand, and they stay
    name: "layout-debug-ends",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,wait",
    delay: 8,
    dump: {
      "session.state": /^stopped$/,
      "dock.layout": /tabs \d+ outputbox immediatebox bppanel\n[\s\S]*hidden [^\n]*varstree/,
      "outputbox.text": /ended, exit code 0/,
    },
  },
  {
    // (S9) Each layout keeps its own sizes, across a restart: the bottom
    // panel 260 high while editing; 330 high, the debug panes 420 wide
    // while debugging
    name: "layout-sizes-kept",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    desktopOnly: true,
    do: "extent:outputbox:height:260,line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,extent:outputbox:height:330,extent:varstree:width:420,wait,wait,wait,run.stop,wait,wait,wait,wait",
    delay: 9,
    restart: { do: "wait,wait,wait,run.start,wait,wait,wait,wait", delay: 6 },
    dump: {
      "session.state": /^paused$/,
      "dock.layout": /^\s+330 split row\n\s+\d+ tabs \d+ outputbox immediatebox\n\s+420 tabs 0 varstree/m,
    },
  },
  {
    // (S9) …and the editing layout's, started again
    name: "layout-sizes-kept-editing",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    desktopOnly: true,
    do: "extent:outputbox:height:260,line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,extent:outputbox:height:330,wait,wait,wait,run.stop,wait,wait,wait,wait",
    delay: 9,
    restart: { do: "wait,wait", delay: 3 },
    dump: { "dock.layout": /^\s+260 tabs \d+ outputbox immediatebox bppanel$/m },
  },
  // ---- S-DEBUG: running and debugging (docs/studio-wow.md RUN / DBG) ----
  // counter.rr includes tally.inc (AddUp, the SUB stepped into); oops.rr
  // divides by zero in a SUB. `line:N` puts the caret on line N, F9 sets a
  // breakpoint there; captures at 1x and 2x (desktop: the Studio window; web:
  // the page).
  {
    // F9, F5: paused at the breakpoint, the globals in Variables, the stack
    name: "debug-breakpoint",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait",
    delay: 6,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.state": /^paused$/,
      "session.currentline": /^6$/,
      "session.stopreason": /^breakpoint$/,
      "varstree.text": /^Locals\n\t\(none\)\nGlobals\n\ti = 1\n\ttotal = 0\nComponents$/,
      "stacktree.text": /^\(the program\)\s+counter\.rr:6$/,
      "bptree.text": /^counter\.rr:6$/,
    },
  },
  {
    // a component (a handler's Sender) in Variables opens to its
    // properties, as the program reads them
    name: "debug-component-properties",
    open: "tests/fixtures/studio_debug/sender.rr",
    webFiles: ["tests/fixtures/studio_debug/sender.rr"],
    do: "line:13,debug.toggleBreakpoint,run.start,wait,wait,wait,expand:L/Sender,wait,wait",
    delay: 7,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.currentline": /^13$/,
      "varstree.text": /^Locals\n\tSender = "Greet" \(RButton\)\n\t\t[\s\S]*Caption = "Greet"[\s\S]*Width = 120/,
    },
  },
  {
    // a click in the gutter's marker column (line 6: 100–120 px down the
    // editor) sets a breakpoint there, as F9 does (the test's events come
    // after the commands: running from it is debug-breakpoint's)
    name: "debug-gutter-click",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    events: "codedoc(0).__mousedown_12_110,codedoc(0).__mouseup_12_110",
    do: "wait",
    delay: 3,
    dump: { "bptree.text": /^counter\.rr:6$/, "session.state": /^stopped$/ },
  },
  {
    // F11 into AddUp: tally.inc opens at its line (the editor follows the
    // program into another file); F10 twice: the local k and the watch on it
    // change; the stack has both files
    name: "debug-step-watch",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,watch:k,watch:total + k,run.start,wait,wait,wait,debug.stepInto,wait,wait,debug.stepOver,wait,debug.stepOver,wait,debug.stepOver,wait,wait",
    delay: 9,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.state": /^paused$/,
      "session.currentfile": /tally\.inc$/,
      "session.currentline": /^5$/,
      "varstree.text": /^Locals\n\tn = 1\n\tk = 2\nGlobals\n\ti = 1\n\ttotal = 0\nComponents$/,
      "watchtree.text": /^k = 2\ntotal \+ k = 2$/,
      "stacktree.text": /^AddUp\s+tally\.inc:5\n\(the program\)\s+counter\.rr:6$/,
      "codedoc(1).caretline": /^5$/,
    },
  },
  {
    // a condition (i = 2): it stops once, at the second time round; "? i"
    // in Immediate says 2; F5 goes on to the end
    name: "debug-condition-continue",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,bpcond:i = 2,run.start,wait,wait,wait,wait,focus:immediatebox,type:? i * 100,key:Enter,wait,wait,run.start,wait,wait,wait",
    delay: 9,
    dump: {
      "bptree.text": /^counter\.rr:6  when i = 2$/,
      "immediatebox.text": /\? i \* 100\s*\n\s*200/,
      "session.state": /^stopped$/,
      "session.exitcode": /^0$/,
      "outputbox.text": /total12[\s\S]*ended, exit code 0/,
    },
  },
  {
    // a .rr program has RapidQ's constants built in, and so does Immediate
    // while it is stopped: "? mbYes" says 1 and "? mrYes" 6 (RapidQ's own
    // values), the program's own i still 1 (counter.rr has no include of
    // RAPIDQ.INC)
    name: "debug-immediate-built-in-constants",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,focus:immediatebox,type:? mbYes,key:Enter,wait,wait,type:? mrYes + mbYes * 10,key:Enter,wait,wait,type:? i,key:Enter,wait,wait",
    delay: 9,
    dump: {
      "session.state": /^paused$/,
      "immediatebox.text": /\? mbYes\s*\n\s*1\s*\n[\s\S]*\? mrYes \+ mbYes \* 10\s*\n\s*16\s*\n[\s\S]*\? i\s*\n\s*1\s*$/,
    },
  },
  {
    // a logpoint prints and goes on: the program runs to its end
    name: "debug-logpoint",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,bplog:adding {i},run.start,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "session.state": /^stopped$/,
      "session.exitcode": /^0$/,
      "outputbox.text": /adding 1\s*\n\s*adding 2\s*\n\s*adding 3[\s\S]*total12/,
      "bptree.text": /^counter\.rr:6  log "adding \{i\}"$/,
    },
  },
  {
    // a hit count: it stops on the third hit only
    name: "debug-hit-count",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,bphit:3,run.start,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "session.state": /^paused$/,
      "session.currentline": /^6$/,
      "varstree.text": /\ti = 3\n\ttotal = 6\nComponents$/,
      "bptree.text": /^counter\.rr:6  hit 3$/,
    },
  },
  {
    // Run to Cursor from a stopped program: it starts and stops there
    name: "debug-run-to-cursor",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:8,debug.runToCursor,wait,wait,wait,wait",
    delay: 6,
    dump: { "session.state": /^paused$/, "session.currentline": /^8$/, "varstree.text": /total = 12/, "bptree.text": /^$/ },
  },
  {
    // a data tip: the value under a resting mouse while paused; a caller's
    // frame picked in the Call Stack shows its line and its locals
    name: "debug-hover-frame",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,debug.stepInto,wait,wait,frame:1,wait,wait,hover:6:9,wait,wait",
    delay: 8,
    dump: { "codedoc(0).datatiptext": /^i = 1$/, "session.frame": /^1$/, "varstree.text": /^Locals\n\t\(none\)\nGlobals/ },
  },
  {
    // Pause while the program waits for its events (its ShowModal): it
    // stops at once, at the line that waits, the note "Waiting for events"
    // there and its variables shown (desktop: the program's wait turns
    // under the debugger; web: the frame's VM waits, nothing runs). Going on
    // from there is rapidr-session's a_pause_while_the_program_waits_… (a
    // desktop program captured under a test ends once it has been seen)
    name: "debug-pause-waiting",
    open: "tests/fixtures/studio_debug/waits.rr",
    webFiles: ["tests/fixtures/studio_debug/waits.rr"],
    do: "run.start,wait,wait,wait,run.pause,wait,wait",
    delay: 6,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.state": /^paused$/,
      "session.stopreason": /^pause$/,
      "session.currentline": /^17$/,
      "session.stopmessage": /^Waiting for events$/,
      "varstree.text": /^Locals\n\t\(none\)\nGlobals\n\tclicks = 0\nComponents$/,
      "stacktree.text": /^\(the program\)\s+waits\.rr:17$/,
      "codedoc(0).caretline": /^17$/,
    },
  },
  {
    // breakpoints on lines without code: the running program places them —
    // the comment's moves to the next line with code (it stops there), the
    // one after the last code never stops and says so
    name: "debug-placed",
    open: "tests/fixtures/studio_debug/placed.rr",
    webFiles: ["tests/fixtures/studio_debug/placed.rr"],
    do: "line:5,debug.toggleBreakpoint,line:8,debug.toggleBreakpoint,run.start,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "session.state": /^paused$/,
      "session.currentline": /^6$/,
      "bptree.text": /^placed\.rr:6\nplaced\.rr:8  \(no code here: never stops\)$/,
    },
  },
  {
    // the project's breakpoints and watches kept: Studio started again on
    // the same settings has them as they were
    name: "debug-kept",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,bpcond:i = 2,watch:total * 10,wait,wait,wait,wait",
    delay: 5,
    restart: { do: "wait,wait,run.start,wait,wait,wait,wait", delay: 6 },
    dump: {
      "bptree.text": /^counter\.rr:6  when i = 2$/,
      "watchtree.text": /^total \* 10 = 20$/,
      "session.currentline": /^6$/,
    },
  },
  {
    // a value set in place, as debuggers' variables are: F2 on
    // Globals' total, 100 typed, Enter — the paused program has it (the
    // values and a watch fetched again), and it runs on with it
    name: "debug-set-value",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,watch:total + 1,run.start,wait,wait,wait,varpick:G/total,key:F2,type:100,key:Enter,wait,wait,wait,line:6,debug.toggleBreakpoint,run.start,wait,wait,wait",
    delay: 9,
    dump: {
      "outputbox.text": /total112[\s\S]*ended, exit code 0/,
    },
  },
  {
    // …and while it's still paused: the tree and the watch show it
    name: "debug-set-value-shown",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,watch:total + 1,run.start,wait,wait,wait,varpick:G/total,key:F2,type:100,key:Enter,wait,wait,wait",
    delay: 7,
    dump: {
      "varstree.text": /^Locals\n\t\(none\)\nGlobals\n\ti = 1\n\ttotal = 100\nComponents$/,
      "watchtree.text": /^total \+ 1 = 101$/,
    },
  },
  {
    // the whole paused session, to look at (docs/studio-wow.md DBG): into
    // AddUp in tally.inc, two steps; the Call Stack one of the debug panes
    // beside Output (S9's debugging layout), the line marked, a data tip on
    // total
    name: "debug-paused-session",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,watch:total + k,run.start,wait,wait,wait,debug.stepInto,wait,debug.stepOver,wait,debug.stepOver,wait,view.variables,hover:5:5,wait,wait",
    delay: 9,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.currentline": /^4$/,
      "codedoc(1).datatiptext": /^total = 0$/,
      "stacktree.text": /^AddUp\s+tally\.inc:4\n\(the program\)\s+counter\.rr:6$/,
      "varstree.text": /^Locals\n\tn = 1\n\tk = 0\nGlobals\n\ti = 1\n\ttotal = 0\nComponents$/,
      "dock.layout": /^\s+\d+ tabs 0 varstree watchpanel stacktree bppanel$/m,
    },
  },
  {
    // a run-time error stops at its line, its message at the line's end
    name: "debug-runtime-error",
    open: "tests/fixtures/studio_debug/oops.rr",
    webFiles: ["tests/fixtures/studio_debug/oops.rr"],
    do: "run.start,wait,wait,wait,view.variables",
    delay: 6,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.state": /^paused$/,
      "session.stopreason": /^exception$/,
      "session.currentline": /^4$/,
      "session.stopmessage": /Division by zero/,
      "varstree.text": /^Locals\n\tAmount = 10\nGlobals\n\tz = 0\nComponents$/,
      "outputbox.text": /before\s*\n\s*each gets\s*\n\s*oops\.rr:4: run-time error: Division by zero/,
    },
  },
  // ---- DEBUG-RUN: Run's and Debug's commands in every state of the
  // program (Robert, Preview 2: "debug disables the play icon, if the user
  // decides to just continue the execution it can't"). Each step is a real
  // key through Studio's menus (key:F5 …: a disabled item never runs) and
  // cmds:report writes what the person sees then — each command's menu
  // item, tool button and palette entry (1 enabled, 0 not, - none), the Run
  // item's caption and the ▶ button's hint: "Continue (F5)" while paused.
  {
    // F9 F5: paused at 6 — Continue enabled, F5 goes on (stops at 6 again,
    // the next time round), F10 steps (over AddUp: the loop comes back to
    // the breakpoint), Shift+F5 stops; F11 from idle starts at the first
    // line; Run again after a Stop
    name: "debug-commands-keys",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "cmds:report,line:6,key:F9,key:F5,wait,wait,wait,wait,cmds:report,key:F5,wait,wait,wait,cmds:report,key:F10,wait,wait,wait,cmds:report,key:Shift+F5,wait,wait,wait,cmds:report,key:F11,wait,wait,wait,wait,cmds:report,key:Shift+F5,wait,wait,wait,key:F5,wait,wait,wait,wait,cmds:report,cmds:log",
    delay: 30,
    dump: {
      "outputbox.text": cmdsSeq(cmdsLine("stopped", "idle"), cmdsLine("paused@6:breakpoint", "paused"), cmdsLine("paused@6:breakpoint", "paused"), cmdsLine("paused@[67]:(step|breakpoint)", "paused"), cmdsLine("stopped", "idle"), cmdsLine("paused@\\d+:entry", "paused"), cmdsLine("paused@6:breakpoint", "paused")),
      "varstree.text": /\ti = 1\n/,
    },
  },
  {
    // paused at 6: a real click on the tool bar's ▶ (enabled, "Continue
    // (F5)") goes on — the program stops at 6 again, the next time round;
    // the same click while it waits for its events (running: ▶ disabled)
    // does nothing
    name: "debug-commands-toolbar",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,key:F9,key:F5,wait,wait,wait,wait",
    delay: 6,
    events: "toolbar.__mousemove_357_18,toolbar.__mousedown_357_18,toolbar.__mouseup_357_18,toolbar.__mousemove_400_18,toolbar.__mousemove_401_18,toolbar.__mousemove_402_18",
    dump: { "session.state": /^paused$/, "session.currentline": /^6$/, "varstree.text": /\ti = 2\n/ },
  },
  {
    // Restart (Ctrl+Shift+F5) while paused: the program ends and runs again
    // from its start — stopped at 6 the first time round again (i = 1)
    name: "debug-commands-restart",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,key:F9,key:F5,wait,wait,wait,wait,key:F5,wait,wait,wait,cmds:report,key:Ctrl+Shift+F5,wait,wait,wait,wait,wait,cmds:report,cmds:log",
    delay: 12,
    dump: {
      "outputbox.text": cmdsSeq(cmdsLine("paused@6:breakpoint", "paused"), cmdsLine("paused@6:breakpoint", "paused")),
      "varstree.text": /\ti = 1\n/,
    },
  },
  {
    // a run-time error: paused at it (Continue enabled), F5 lets the error
    // end the program (exit code 1: the "error" state) — and F5 runs it
    // again from there; Shift+F5 stops it
    name: "debug-commands-error",
    open: "tests/fixtures/studio_debug/oops.rr",
    webFiles: ["tests/fixtures/studio_debug/oops.rr"],
    do: "cmds:report,key:F5,wait,wait,wait,wait,cmds:report,key:F5,wait,wait,wait,wait,cmds:report,key:F5,wait,wait,wait,wait,cmds:report,key:Shift+F5,wait,wait,wait,cmds:report,cmds:log",
    delay: 16,
    dump: {
      "outputbox.text": cmdsSeq(cmdsLine("stopped", "idle"), cmdsLine("paused@4:exception", "paused"), cmdsLine("error", "idle"), cmdsLine("paused@4:exception", "paused"), cmdsLine("stopped", "idle")),
    },
  },
  {
    // a program that waits for its events runs: Pause, Stop and Show
    // Program Window on, Run and the steps off; paused while it waits, the
    // whole paused set; Continue lets it wait again (running)
    name: "debug-commands-running",
    open: "tests/fixtures/studio_debug/waits.rr",
    webFiles: ["tests/fixtures/studio_debug/waits.rr"],
    do: "view.code,key:F5,wait,cmds:report,wait,run.pause,wait,wait,cmds:report,key:F5,wait,cmds:report,cmds:log",
    delay: 8,
    dump: {
      "outputbox.text": cmdsSeq(cmdsLine("running", "running"), cmdsLine("paused@17:pause", "paused"), cmdsLine("running", "running")),
    },
  },
  {
    // stepping over a line that takes seconds: the program runs meanwhile
    // (F5 and the steps off, Pause on), then it stops on the next line
    name: "debug-commands-stepping",
    open: "tests/fixtures/studio_debug/busy.rr",
    webFiles: ["tests/fixtures/studio_debug/busy.rr"],
    do: "line:12,key:F9,key:F5,wait,wait,wait,wait,key:F10,wait,cmds:report,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,cmds:report,key:F5,wait,wait,wait,cmds:report,cmds:log",
    delay: 16,
    dump: {
      "outputbox.text": cmdsSeq(cmdsLine("running", "running"), cmdsLine("paused@13:step", "paused"), cmdsLine("stopped", "idle")),
    },
  },
  // ---- DEBUG-RUN, the debugger's events (Robert: "users will debug events
  // of course"): each handler's breakpoint stops on every event — three
  // times with Continue between, again after a Step Over out of the
  // handler (scenarios 1, 2, 5, 6; the program's input from its script)
  {
    // 1. hello_form.rr's one-line OnChange: every key typed stops at 49
    name: "debug-events-onchange",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("nameedit.__key_65", 6) },
    do: eventSteps(49),
    delay: 16,
    dump: { "outputbox.text": stopsAt(49) },
  },
  {
    // 2. a click (Click's OnClick)
    name: "debug-events-click",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("btn.__mousedown_10_10,btn.__mouseup_10_10", 6) },
    do: eventSteps(58),
    delay: 16,
    dump: { "outputbox.text": stopsAt(58) },
  },
  {
    // 2. a key (the edit's OnKeyPress)
    name: "debug-events-keypress",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("box.__key_65", 6) },
    do: eventSteps(62),
    delay: 16,
    dump: { "outputbox.text": stopsAt(62) },
  },
  {
    // 2. the focus (the list's OnEnter: clicked, then the edit, again)
    name: "debug-events-focus",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("list.__mousedown_10_10,list.__mouseup_10_10,box.__mousedown_10_10,box.__mouseup_10_10", 6) },
    do: eventSteps(66),
    delay: 16,
    dump: { "outputbox.text": stopsAt(66) },
  },
  {
    // 2. a timer's OnTimer (every second while the program runs)
    name: "debug-events-timer",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: Array(160).fill("btn.__mousemove_5_5").join(",") },
    do: eventSteps(79),
    delay: 20,
    dump: { "outputbox.text": stopsAt(79) },
  },
  {
    // 2. the mouse moving over the form (OnMouseMove): it stops on every
    // move, and Continue always gets the program going again
    name: "debug-events-mousemove",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("form.__mousemove_300_160", 6) },
    do: eventSteps(83),
    delay: 16,
    dump: { "outputbox.text": stopsAt(83) },
  },
  {
    // 2. a resize (the form's OnResize): the test's resize stops at 70
    name: "debug-events-resize",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: Array(20).fill("btn.__mousemove_5_5").join(","), RAPIDR_TEST_RESIZE: "420,260" },
    do: `line:70,key:F9,key:F5,${"wait,".repeat(8)}stops:report,key:F5,wait,wait`,
    delay: 9,
    dump: { "outputbox.text": /\[stops\]( 70:breakpoint)+/ },
  },
  {
    // 5. nested: Grow widens the form, whose OnResize runs inside it — the
    // breakpoint in Resized (set while the program runs: it takes effect
    // at the next event) stops, the call stack has both
    name: "debug-events-nested",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: Array(40).fill("btn.__mousemove_5_5").join(",") + "," + triggers("growbtn.__mousedown_10_10,growbtn.__mouseup_10_10", 2) },
    do: `key:F5,wait,wait,line:70,key:F9,${"wait,".repeat(10)}stops:report`,
    delay: 9,
    dump: { "outputbox.text": /\[stops\] 70:breakpoint/, "stacktree.text": /^Resized\s+events\.rr:70\nGrow\s+events\.rr:91\n/ },
  },
  {
    // 6. a run-time error in a handler: it stops at the line (Continue
    // enabled); Shift+F5 ends it; run again, the error again: Continue
    // (F5) does what the program does without the debugger — RapidQ's
    // (RC.EXE, DEBUG-2): the handler ends there, the program runs on —, so
    // the next click stops at the error again, three times; the error in
    // Output each time; the program still running after the last
    name: "debug-events-error",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("oopsbtn.__mousedown_10_10,oopsbtn.__mouseup_10_10", 3, 40) },
    do: `key:F5,${"wait,".repeat(8)}cmds:report,key:Shift+F5,wait,wait,wait,cmds:report,key:F5,${"wait,".repeat(8)}cmds:report,${`key:F5,${"wait,".repeat(8)}cmds:report,`.repeat(2)}key:F5,wait,wait,wait,cmds:report,cmds:log`,
    delay: 30,
    // (Output keeps its last lines: the stops and the errors of the second
    // run, then the whole log of the states)
    dump: {
      "outputbox.text": cmdsSeq(
        cmdsLine("paused@95:exception", "paused"),
        "run-time error: Division by zero \\(at events\\.rr line 95\\)",
        cmdsLine("paused@95:exception", "paused"),
        "run-time error: Division by zero \\(at events\\.rr line 95\\)",
        cmdsLine("paused@95:exception", "paused"),
        "run-time error: Division by zero \\(at events\\.rr line 95\\)",
        cmdsLine("running", "running"),
        "\\[cmds-log\\]",
        cmdsLine("paused@95:exception", "paused"),
        cmdsLine("stopped", "idle"),
        cmdsLine("paused@95:exception", "paused"),
        cmdsLine("paused@95:exception", "paused"),
        cmdsLine("paused@95:exception", "paused"),
        cmdsLine("running", "running"),
      ),
    },
  },
  {
    // (DEBUG-2) the focus (RapidR's OnEnter / OnExit): Tab from
    // the name to Greet stops in the name's OnExit (41), then, F5, in
    // Greet's OnEnter (45) — three rounds, Shift+Tab back between them (its
    // own OnExit / OnEnter have no breakpoint): each stops every time, in
    // that order
    name: "debug-events-focus-order",
    open: "tests/fixtures/studio_debug/focus.rr",
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: Array.from({ length: 3 }, () => ["nameedit.__key_9", ...Array(4).fill("otherbutton.__mousemove_5_5"), "greetbutton.__key_9_256", ...Array(4).fill("otherbutton.__mousemove_5_5")].join(",")).join(",") },
    do: `line:41,key:F9,line:45,key:F9,key:F5,${"wait,".repeat(8)}${"key:F5,wait,wait,wait,wait,".repeat(6)}stops:report`,
    delay: 22,
    dump: { "outputbox.text": /\[stops\] 41:breakpoint 45:breakpoint 41:breakpoint 45:breakpoint 41:breakpoint 45:breakpoint$/m },
  },
  {
    // 2. the close box (the form's OnClose, the close cancelled: Action = 0)
    name: "debug-events-close",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("form.__close", 6) },
    do: eventSteps(74),
    delay: 16,
    dump: { "outputbox.text": stopsAt(74) },
  },
  {
    // 4. a handler that opens a dialog (dialogs.rr's Ask: MESSAGEDLG): it
    // stops on the dialog's line, Step Over shows the dialog (held open
    // a moment, then answered Yes, as the user's click would) and stops on
    // the line after it — three times, F5 between. (The script's next
    // click waits out the dialog: a script's events aren't held back by a
    // modal dialog as the user's are)
    name: "debug-events-dialog",
    open: "examples/gui/dialogs.rr",
    copy: true,
    desktopOnly: true,
    env: {
      RAPIDR_TEST_PROGRAM_EVENTS: Array.from({ length: 3 }, () => ["askbtn.__mousedown_10_10", "askbtn.__mouseup_10_10", ...Array(70).fill("answer.__mousemove_5_5")].join(",")).join(","),
      RAPIDR_TEST_PROGRAM_MESSAGE_DIALOG: "Yes;Yes;Yes;",
      RAPIDR_TEST_PROGRAM_DIALOG_HOLD: "800",
    },
    do: `line:84,key:F9,key:F5,${"wait,".repeat(8)}${`key:F10,${"wait,".repeat(6)}key:F5,${"wait,".repeat(12)}`.repeat(3)}stops:report`,
    delay: 34,
    dump: { "outputbox.text": /\[stops\] 84:breakpoint 8[56]:step 84:breakpoint 8[56]:step 84:breakpoint 8[56]:step/ },
  },
  // ---- DEBUG-2: Robert's scenarios 7, 9, 10, 11, 13 (stepping, loops,
  // recursion, breakpoints while running, a breakpoint in an included file;
  // 8, 12, 14, 16 and 17 are the flows above: debug-events-* (F10 out of a
  // handler), debug-condition-continue / -hit-count / -logpoint,
  // debug-placed, debug-hover-frame / debug-datatip-* / debug-set-value,
  // debug-step-watch / debug-changed and Immediate)
  {
    // 9. loops: a breakpoint on a FOR loop's body stops at every pass (40:
    // i = 1, 2, 3); one on a loop written on one line (38) stops once, when
    // the line is reached (the loop then runs round inside it)
    name: "debug-steps-loop",
    open: "tests/fixtures/studio_debug/steps.rr",
    webFiles: ["tests/fixtures/studio_debug/steps.rr", "tests/fixtures/studio_debug/tally.inc"],
    do: `line:38,key:F9,line:40,key:F9,key:F5,${"wait,".repeat(5)}${"key:F5,wait,wait,wait,".repeat(4)}stops:report`,
    delay: 12,
    dump: { "outputbox.text": /\[stops\] 38:breakpoint 40:breakpoint 40:breakpoint 40:breakpoint$/m },
  },
  {
    // 10. recursion: a breakpoint in a recursive FUNCTION stops at every
    // level (Fact(4) → 4, 3, 2, 1); at the deepest the Call Stack lists
    // them all, the program's line under them, and Locals is that level's
    name: "debug-steps-recursion",
    open: "tests/fixtures/studio_debug/steps.rr",
    webFiles: ["tests/fixtures/studio_debug/steps.rr", "tests/fixtures/studio_debug/tally.inc"],
    do: `line:11,key:F9,key:F5,${"wait,".repeat(5)}${"key:F5,wait,wait,wait,".repeat(3)}stops:report`,
    delay: 10,
    dump: {
      "outputbox.text": /\[stops\] 11:breakpoint 11:breakpoint 11:breakpoint 11:breakpoint$/m,
      "stacktree.text": /^Fact\s+steps\.rr:11\nFact\s+steps\.rr:14\nFact\s+steps\.rr:14\nFact\s+steps\.rr:14\n\(the program\)\s+steps\.rr:42$/,
      "varstree.text": /^Locals\n\tn = 1\n/,
    },
  },
  {
    // 13. a breakpoint in an $INCLUDEd file (tally.inc, opened from the
    // project, the breakpoint set there, the file closed again): it stops
    // there, and the editor opens the file at the line
    name: "debug-steps-include",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: `open:tally.inc,wait,line:5,key:F9,wait,file.close,wait,key:F5,${"wait,".repeat(6)}`,
    delay: 9,
    dump: {
      "session.state": /^paused$/,
      "session.stopreason": /^breakpoint$/,
      "session.currentfile": /tally\.inc$/,
      "session.currentline": /^5$/,
      "codedoc(1).caretline": /^5$/,
      "stacktree.text": /^AddUp\s+tally\.inc:5\n\(the program\)\s+counter\.rr:6$/,
    },
  },
  {
    // 7. stepping inside a handler (Go, the button's OnClick): its
    // breakpoint (33), Step Into Twice (its SUB line, 18, as VB's), Step
    // Out back in Go (the next line, 34), Run to Cursor (35), Set Next
    // Statement back to 34, Step Over (35); Continue — the next click stops
    // at 33 again
    name: "debug-steps-handler",
    open: "tests/fixtures/studio_debug/steps.rr",
    copyDir: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("gobtn.__mousedown_10_10,gobtn.__mouseup_10_10", 2, 20) },
    do: `line:33,key:F9,key:F5,${"wait,".repeat(8)}key:F11,wait,wait,wait,key:Shift+F11,wait,wait,wait,line:35,debug.runToCursor,wait,wait,wait,goto:34,wait,wait,wait,key:F10,wait,wait,wait,key:F5,${"wait,".repeat(8)}stops:report`,
    delay: 22,
    dump: { "outputbox.text": /\[stops\] 33:breakpoint 18:step 34:step 35:breakpoint 34:goto 35:step 33:breakpoint$/m },
  },
  {
    // 11. a breakpoint taken off while the program runs: the click after it
    // doesn't stop (the first did); one put back stops the next click
    name: "debug-steps-remove-running",
    open: EVENTS_FILE,
    copy: true,
    desktopOnly: true,
    env: { RAPIDR_TEST_PROGRAM_EVENTS: triggers("btn.__mousedown_10_10,btn.__mouseup_10_10", 8, 30) },
    do: `line:58,key:F9,key:F5,${"wait,".repeat(8)}key:F5,wait,line:58,key:F9,${"wait,".repeat(8)}line:58,key:F9,${"wait,".repeat(8)}stops:report,cmds:report`,
    delay: 16,
    dump: { "outputbox.text": /\[stops\] 58:breakpoint 58:breakpoint\n[\s\S]*\[cmds\] paused@58/ },
  },
  {
    // (DEBUG-2) a file name too long for the status bar: stopped, the bar
    // says where with the name shortened in its middle — "Breakpoint:",
    // its start, "…", its end and the line kept —, the whole in the bar's
    // tooltip
    name: "debug-status-long-name",
    open: "tests/fixtures/studio_debug/a_program_whose_name_is_far_too_long_for_the_status_bar.rr",
    webFiles: ["tests/fixtures/studio_debug/a_program_whose_name_is_far_too_long_for_the_status_bar.rr"],
    do: "line:6,key:F9,key:F5,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    // (POLISH-2: a narrow Studio, where even the whole bar is too short —
    // the desktop's window resized; the page's Studio is 1280 wide, where
    // the state says it all: debug-status-long-name-room)
    env: { RAPIDR_TEST_RESIZE: "800,700" },
    desktopOnly: true,
    dump: {
      "session.currentline": /^6$/,
      // (the hint is set only when a panel is shortened; the bar itself in
      // the capture)
      "status.hint": /^Breakpoint: a_program_whose_name_is_far_too_long_for_the_status_bar\.rr:6( - .*)?$/,
      "status.panel(0).caption": /^Breakpoint: a_prog[^…]*…[^…]*_bar\.rr:6$/,
      "status.accessibledescription": /^Breakpoint: a_program_whose_name_is_far_too_long_for_the_status_bar\.rr:6( - .*)?$/,
    },
  },
  {
    // (POLISH-2) …where the bar has the room — Studio's own size —, the
    // state takes it and says it all; the project's name gives way
    name: "debug-status-long-name-room",
    open: "tests/fixtures/studio_debug/a_program_whose_name_is_far_too_long_for_the_status_bar.rr",
    webFiles: ["tests/fixtures/studio_debug/a_program_whose_name_is_far_too_long_for_the_status_bar.rr"],
    do: "line:6,key:F9,key:F5,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "session.currentline": /^6$/,
      "status.panel(0).caption": /^Breakpoint: a_program_whose_name_is_far_too_long_for_the_status_bar\.rr:6$/,
      "status.panel(2).width": /^110$/,
    },
  },
  // ---- DEBUG-RUN, scenario 15: Variables > Globals — the program's own
  // variables first, RAPIDQ.INC's hundreds of constants after them under
  // one item, closed; opened, its names
  {
    name: "debug-globals-include",
    open: "tests/fixtures/studio_debug/include.rr",
    webFiles: ["tests/fixtures/studio_debug/include.rr"],
    do: "line:9,key:F9,key:F5,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    dump: { "varstree.text": /^Locals\n\t\(none\)\nGlobals\n\ttotal = 3\n\twho = "Ann"\n\t\$INCLUDE RAPIDQ\.INC \(\d{3} names\)\nComponents$/ },
  },
  {
    name: "debug-globals-include-open",
    open: "tests/fixtures/studio_debug/include.rr",
    webFiles: ["tests/fixtures/studio_debug/include.rr"],
    do: "line:9,key:F9,key:F5,wait,wait,wait,wait,expand:G/$INCLUDE RAPIDQ.INC,wait,wait,wait",
    delay: 6,
    dump: { "varstree.text": /\n\twho = "Ann"\n\t\$INCLUDE RAPIDQ\.INC \(\d{3} names\)\n(\t\t\w+ = -?\d+\n){20}/ },
  },
  // ---- DEBUG-VM: the values inspector, Set Next Statement, Step Into
  // Specific (examples/debugging/shop.rr: nested TYPEs, an array of them, an
  // RDataFrame, an RNum, a form). Line 76 adds a LineTotal in AddUp's loop.
  {
    // the data tip opens like a tree: orders → (1) → Buyer → Home
    name: "debug-datatip-tree",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:76:37,wait,wait,wait,tipopen:1,wait,wait,tipopen:4,wait,wait,tipopen:7,wait,wait",
    delay: 9,
    scales: [1, 2],
    capture: true,
    dump: {
      "session.currentline": /^76$/,
      "codedoc(0).datatiptext": /^orders = Array\(1 TO 10\)\n\t\(1\) = Order\n\t\tItem = "tea"\n\t\tQty = 2\n\t\tBuyer = Customer\n\t\t\tName = "Ada"\n\t\t\tSince = 2021\n\t\t\tHome = Address\n\t\t\t\tStreet = "10 Market Street"\n\t\t\t\tCity = "London"\n\t\(2\) = Order/i,
    },
  },
  {
    // an RDataFrame in a data tip: a table — its columns, its rows
    name: "debug-datatip-frame",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:140:2,wait,wait,wait,wait",
    delay: 8,
    scales: [1, 2],
    capture: true,
    dump: {
      "codedoc(0).datatiptext": /^week = "week" \(RDataFrame: 5 rows × 3 columns\)\n\tday \| orders \| amount\n\t\(0\) Mon \| 12 \| 340\.5\n\t\(1\) Tue \| 9 \| 251[\s\S]*\(4\) Fri \| 21 \| 610\.75$/,
    },
  },
  {
    // a hovered member through an element (orders(i).Qty) is evaluated —
    // and a call never is: Variables marks what changed since the last
    // stop, an RNum and a frame open in it too
    name: "debug-datatip-member",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:76:46,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).datatiptext": /^orders\(i\)\.Qty = 2$/ },
  },
  {
    // what the last run changed is marked: from one pass of AddUp's loop to
    // the next, its i and the total (and the watch on the total) — the
    // order's own values, the same, aren't
    name: "debug-changed",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,watch:total,watch:count,run.start,wait,wait,wait,run.start,wait,wait,wait,changed:report",
    delay: 9,
    dump: { "outputbox.text": /\[changed\] L\/i G\/total W1$/m },
  },
  {
    // a nested field set in place (F2 in Variables): orders(1).Buyer.Name,
    // the watch on it and the tree showing the new value — (S11) text typed
    // as it is, no quotes: its whole text fetched first, then the editor
    name: "debug-set-nested",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,watch:orders(1).Buyer.Name,run.start,wait,wait,wait,view.variables,expand:G/orders,wait,wait,expand:G/orders/(1),wait,wait,expand:G/orders/(1)/Buyer,wait,wait,varpick:G/orders/(1)/Buyer/Name,key:F2,wait,wait,type:Grace,key:Enter,wait,wait,wait",
    delay: 11,
    dump: {
      "watchtree.text": /^orders\(1\)\.Buyer\.Name = "Grace"$/,
      "varstree.text": /\t\t\t\tName = "Grace"/,
    },
  },
  // ---- (S11) values set as their kind wants ----------------------------------------
  {
    // text where a whole number goes: refused, Output says why in plain
    // words, the status bar points there; the value stays
    name: "debug-set-refused",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,varpick:G/total,key:F2,type:abc,key:Enter,wait,wait,wait",
    delay: 7,
    dump: {
      "varstree.text": /\ttotal = 0\n/,
      "outputbox.text": /^total holds a whole number, and abc isn't one\. Type a number such as 42\.$/m,
    },
    a11y: [[/Can't set total: see Output/, "label"]],
  },
  {
    // set, marked changed; Ctrl+Z in Variables (the tree's OnUndo) puts it
    // back while paused
    name: "debug-set-undo",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,varpick:G/total,key:F2,type:7,key:Enter,wait,wait,wait,changed:report,key:Ctrl+Z,wait,wait,wait",
    delay: 9,
    dump: {
      "varstree.text": /\ttotal = 0\n/,
      "outputbox.text": /^\[changed\][^\n]* G\/total/m,
    },
    a11y: [[/total is back to 0/, "label"]],
  },
  {
    // …and Edit > Undo, Variables in front: the same
    name: "debug-set-undo-menu",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,varpick:G/total,key:F2,type:7,key:Enter,wait,wait,wait,edit.undo,wait,wait,wait",
    delay: 9,
    dump: { "varstree.text": /\ttotal = 0\n/ },
    a11y: [[/total is back to 0/, "label"]],
  },
  {
    // Immediate's `name = value`: the same checks — refused in plain words
    // there —, answered only when what it holds isn't what was typed (7.9
    // into a whole number is 7), and Variables shows it
    name: "debug-immediate-set",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,view.immediate,focus:immediatebox,type:total = 100,key:Enter,wait,wait,wait,type:total = \"many\",key:Enter,wait,wait,wait,type:total = 7.9,key:Enter,wait,wait,wait,view.variables,wait",
    delay: 11,
    dump: {
      "immediatebox.text": /^total = 100\r?\ntotal = "many"\r?\ntotal holds a whole number, and "many" isn't one\. Type a number such as 42\.\r?\ntotal = 7\.9\r?\ntotal = 7\r?\n?$/,
      "varstree.text": /\ttotal = 7\n/,
    },
  },
  {
    // a long text with line breaks: its whole text in a window of its own
    name: "debug-set-text-window",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,varpick:G/notes,key:F2,wait,wait,wait",
    delay: 8,
    dump: {
      "valueeditor.__shown": /^1$/,
      "valueeditorbox.text": /^Opening hours: 8 to 18\.\r?\nClosed on Sundays\.\r?\nAsk about the cake of the day!$/,
    },
  },
  {
    // …and Set: the text with its new lines, stored as CHR$(10) between them
    name: "debug-set-text-lines",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,varpick:G/notes,key:F2,wait,wait,wait,valueset:Open 9 to 5.\\nClosed on Mondays.,wait,wait,wait",
    delay: 9,
    dump: {
      "valueeditor.__shown": /^0$/,
      "varstree.text": /\tnotes = "Open 9 to 5\.\\nClosed on Mondays\."\n/,
    },
  },
  {
    // a property's constants: a menu, one picked; a read-only property and
    // a whole array refused in plain words
    name: "debug-set-choice",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait,choice:crHandPoint,varpick:C/Form/Cursor,key:F2,wait,wait,wait,varpick:C/Form/Handle,key:F2,wait,wait",
    delay: 12,
    dump: { "outputbox.text": /^Handle can't be changed: it's read-only\.$/m },
    a11y: [[/^Cursor, constant crHandPoint, changed$/, "treeitem"], [/Can't set Handle: see Output/, "label"]],
  },
  {
    // (POLISH-2) Ctrl+C — ⌘C on a Mac, the tree's OnCopy — in Variables:
    // Edit > Copy, the selected value copied (the tests' own clipboard)
    name: "debug-copy-value",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,focus:codedoc(0),wait,varpick:G/i,key:Ctrl+C,wait,wait,wait",
    delay: 8,
    dump: { "clipboard.text": /^1$/, "status.panel(0).caption": /Copied the value \(1 character\)/ },
  },
  {
    // (COPY-ANYWHERE) Ctrl+C in Call Stack: the selected call as shown
    name: "copy-call-stack",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,focus:stacktree",
    events: "stacktree.__wait_4000,stacktree.__key_67_16,stacktree.__wait_300",
    delay: 8,
    dump: { "clipboard.text": /counter\.rr:6$/, "status.panel(0).caption": /^Copied the call$/ },
  },
  {
    // (COPY-ANYWHERE) …and in Breakpoints: the breakpoint as listed
    name: "copy-breakpoints",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "line:6,debug.toggleBreakpoint,wait,focus:bppanel",
    events: "bptree.__wait_3000,bptree.__key_40,bptree.__key_67_16,bptree.__wait_300",
    delay: 6,
    dump: { "clipboard.text": /^counter\.rr:6$/, "status.panel(0).caption": /^Copied the breakpoint$/ },
  },
  {
    // (POLISH-2) F5 after a pause, the program running on (a line that
    // takes seconds): a moment later Call Stack and Variables say it's
    // running — the last pause's values aren't the program's any more —;
    // at the next pause (Pause) they're filled again
    name: "debug-running-stale",
    open: "tests/fixtures/studio_debug/runs_on.rr",
    webFiles: ["tests/fixtures/studio_debug/runs_on.rr"],
    do: "line:12,key:F9,key:F5,wait,wait,wait,wait,panes:report,key:F5,wait,wait,panes:report,run.pause,wait,wait,wait,panes:report",
    delay: 12,
    dump: {
      "outputbox.text": /\[panes\] \(the program\)[^\n]* \| Locals\n[\s\S]*\[panes\] Running… \| Running… the values show when it pauses[^\n]*\n[\s\S]*\[panes\] (?!Running)[^\n]* \| Locals/,
    },
  },
  {
    // (POLISH-2) a form's Enabled and Visible are on / off as every other
    // component's (the registry's boolean kind): "1 · on"
    name: "debug-form-on-off",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait",
    delay: 10,
    dump: { "session.state": /^paused$/ },
    // (not shown yet at line 76: Visible is 0, off)
    a11y: [[/^Enabled, -?1, on$/, "treeitem"], [/^Visible, 0, off$/, "treeitem"]],
  },
  {
    // …the status bar names the constant picked
    name: "debug-set-choice-said",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait,choice:crHandPoint,varpick:C/Form/Cursor,varmenu:set,wait,wait,wait",
    delay: 11,
    dump: { "session.state": /^paused$/ },
    a11y: [[/Form\.Cursor = crHandPoint/, "label"]],
  },
  {
    // the menu itself: the property's constants, the one it has ticked
    name: "debug-set-choice-menu",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait,varpick:C/Form/Cursor,key:F2,wait,wait",
    delay: 10,
    dump: {
      "choiceitem(0).caption": /^crDefault$/,
      "choiceitem(0).checked": /^(1|-1)$/,
      "choiceitem(1).checked": /^0$/,
      "choiceitem(21).caption": /^crHandPoint$/,
    },
  },
  {
    // an on / off property: its number, then a quiet "off"; `on` typed sets 1
    name: "debug-set-on-off",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait,varpick:C/Form/KeyPreview,wait,key:F2,type:on,key:Enter,wait,wait,wait",
    delay: 12,
    dump: { "varstree.text": /\t\tKeyPreview = 1\n/ },
    a11y: [[/^KeyPreview, 1, on/, "treeitem"]],
  },
  {
    // a watch's own place set from Watch (Set Value…): checked the same way,
    // the watch and Variables showing it
    name: "debug-set-watch",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,watch:count,run.start,wait,wait,wait,view.watch,watchpick:W1,watchmenu:set,wait,type:5,key:Enter,wait,wait,wait",
    delay: 9,
    dump: {
      "watchtree.text": /^count = 5$/,
      "varstree.text": /\tcount = 5\n/,
    },
  },
  {
    // a whole array: refused; open it and set an element
    name: "debug-set-array",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,varpick:G/orders,key:F2,wait,wait",
    delay: 8,
    dump: { "outputbox.text": /^orders is an array: open it and set its elements one at a time\.$/m },
  },
  {
    // Set Next Statement: the arrow carried back to a line runs it again;
    // carried forward, the lines between are skipped (counter.rr: i = 1)
    name: "debug-set-next",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,debug.stepInto,wait,wait,wait,debug.stepOver,wait,wait,wait,debug.stepOver,wait,wait,wait,debug.stepOver,wait,wait,wait,goto:4,wait,wait,wait",
    delay: 11,
    dump: {
      "session.state": /^paused$/,
      "session.stopreason": /^goto$/,
      "session.currentline": /^4$/,
      "session.currentfile": /tally\.inc$/,
      "varstree.text": /^Locals\n\tn = 1\n\tk = 2\nGlobals\n\ti = 1\n\ttotal = 0\nComponents$/,
    },
  },
  {
    // Ctrl+Shift+F10 at a run-time error: the value fixed (F2), the line run
    // again — the program goes on instead of ending
    name: "debug-error-retry",
    open: "tests/fixtures/studio_debug/oops.rr",
    webFiles: ["tests/fixtures/studio_debug/oops.rr"],
    do: "run.start,wait,wait,wait,view.variables,varpick:G/z,key:F2,type:2,key:Enter,wait,wait,line:4,debug.setNextStatement,wait,wait,run.start,wait,wait,wait",
    delay: 9,
    dump: {
      "session.state": /^stopped$/,
      "session.exitcode": /^0$/,
      "outputbox.text": /each gets\s*5\s*\n\s*after[\s\S]*ended, exit code 0/,
    },
  },
  {
    // Step Into Specific: the line's calls (LineTotal, PriceOf) — into
    // PriceOf, past LineTotal's argument
    name: "debug-step-into-specific",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,into:PriceOf,wait,wait,wait",
    delay: 8,
    dump: {
      "session.state": /^paused$/,
      "stacktree.text": /^PriceOf\s+shop\.rr:\d+\nAddUp\s+shop\.rr:76\n/,
      "varstree.text": /^Locals\n\tItem = "tea"/,
    },
  },
  // ---- DARK-FIX: the debugger's panes read in every look (Robert's
  // screenshots: shop.rr paused in RapidR dark — Variables' rows, the Call
  // Stack's rows not selected, the Breakpoints pane's labels were black or
  // dark grey on the dark ground). Each in light, dark and high contrast,
  // its text judged (contrast: WCAG AA) on both hosts.
  {
    // Variables at line 147: Locals, Globals, values unchanged
    name: "debug-look-variables",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,wait",
    delay: 8,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "session.currentline": /^147$/,
      "varstree.text": /^Locals\n\t\(none\)\nGlobals\n\tcount = 3\n\tMAXORDERS = 10\n/,
      "stacktree.text": /^\(the program\)\s+shop\.rr:147$/,
    },
  },
  {
    // the Call Stack with its second row picked: the others not selected
    name: "debug-look-stack",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,debug.stepInto,wait,wait,frame:1,wait,view.callStack,wait",
    delay: 9,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "session.frame": /^1$/,
      "stacktree.text": /^PriceOf\s+shop\.rr:\d+\nAddUp\s+shop\.rr:76\n\(the program\)\s+shop\.rr:147$/,
    },
  },
  {
    // the Breakpoints pane: two breakpoints, the second's condition in
    // its strip (When, Hit, Log, On)
    name: "debug-look-breakpoints",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,line:147,debug.toggleBreakpoint,bpcond:count > 1,run.start,wait,wait,wait,view.breakpoints,wait",
    delay: 9,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "bptree.text": /^shop\.rr:76\nshop\.rr:147  when count > 1$/,
      "bpcondedit.text": /^count > 1$/,
    },
  },
  {
    // Watch and Variables marking what changed since the last stop
    name: "debug-look-watch",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,watch:total,watch:count,watch:orders(1).Buyer.Name,run.start,wait,wait,wait,run.start,wait,wait,wait,view.watch,wait,changed:report",
    delay: 10,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "watchtree.text": /^total = \d+\ncount = 3\norders\(1\)\.Buyer\.Name = "Ada"$/,
      "outputbox.text": /\[changed\] L\/i G\/total W1$/m,
    },
  },
  {
    // a data tip opened like a tree
    name: "debug-look-datatip",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:76:37,wait,wait,wait,tipopen:1,wait,wait,tipopen:4,wait,wait",
    delay: 9,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "codedoc(0).datatiptext": /^orders = Array\(1 TO 10\)\n\t\(1\) = Order\n/i },
  },
  {
    // a run-time error's line and its message
    name: "debug-look-error",
    open: "tests/fixtures/studio_debug/oops.rr",
    webFiles: ["tests/fixtures/studio_debug/oops.rr"],
    do: "run.start,wait,wait,wait,view.variables",
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "session.stopreason": /^exception$/ },
  },
  {
    // (S10) a breakpoint of each kind — not bound to code (the empty line
    // 3), a condition, a hit count, a log message, plain, off —: each its
    // own mark in the margin and in the Breakpoints pane, and its name read
    name: "debug-look-breakpoint-states",
    open: "tests/fixtures/studio_debug/states.rr",
    webFiles: ["tests/fixtures/studio_debug/states.rr"],
    do: "line:14,debug.toggleBreakpoint,line:6,debug.toggleBreakpoint,bpcond:n = 2,line:7,debug.toggleBreakpoint,bphit:3,line:10,debug.toggleBreakpoint,bplog:i is {i},line:12,debug.toggleBreakpoint,bpoff:x,line:11,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,view.breakpoints,wait",
    delay: 16,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "session.currentline": /^11$/, "bptree.text": /^states\.rr:14  \(no code here: never stops\)\nstates\.rr:6  when n = 2\nstates\.rr:7  hit 3\nstates\.rr:10  log "i is \{i\}"\nstates\.rr:12  \(off\)\nstates\.rr:11$/ },
    a11y: [
      [/^states\.rr:14 .*, breakpoint, not bound to code yet$/, "treeitem"],
      [/^states\.rr:6 .*, conditional breakpoint$/, "treeitem"],
      [/^states\.rr:7 .*, breakpoint with a hit count$/, "treeitem"],
      [/^states\.rr:10 .*, log point$/, "treeitem"],
      [/^states\.rr:12 .*, breakpoint, off$/, "treeitem"],
      ["states.rr:11, breakpoint", "treeitem"],
    ],
  },
  {
    // (S10) values read as sentences — their kind, a colour by its name,
    // an array's count — and the call the program is in
    name: "debug-values-read",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,expand:C,wait,wait,expand:C/Form,wait,wait",
    delay: 9,
    dump: { "varstree.text": /\tColor = -2147483633\n/ },
    a11y: [
      ["Locals", "treeitem"],
      ["count, whole number 3", "treeitem"],
      [/^notes, text Opening hours/, "treeitem"],
      ["orders, array 1 to 10, 10 items", "treeitem"],
      ["Color, colour clBtnFace", "treeitem"],
      ["Caption, text Shop", "treeitem"],
    ],
  },
  // (DARK-FIX, Robert) The designer's anchor pins on a component clicked —
  // filled where it's anchored (Left, Top), hollow where it isn't — in
  // every look; and dataframe.rr's linked data live in the designer: the
  // grid's rows and the chart's bars read from staff.csv (its $RESOURCE)
  {
    name: "designer-pins",
    open: "examples/data/dataframe.rr",
    webFiles: ["examples/data/dataframe.rr", "examples/data/staff.csv"],
    do: "wait,view.designer",
    events: "designdoc(0).__mousedown_225_154,designdoc(0).__mouseup_225_154",
    delay: 7,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "designdoc(0).pins": /^Left on, Top on, Right off, Bottom off$/,
      "inspector.target": /Grid/,
    },
  },
  {
    // Run in Browser: the desktop serves the web build on 127.0.0.1 (the
    // page itself: tests/run_in_browser.mjs); on the web it runs here
    name: "run-in-browser",
    open: "tests/fixtures/studio_debug/counter.rr",
    webFiles: DEBUG_FILES,
    do: "run.browser,wait,wait,wait,wait",
    delay: 8,
    env: { RAPIDR_NO_BROWSER: "1" },
    dump: { "session.browserurl": /^http:\/\/127\.0\.0\.1:\d+\/[0-9a-f]{32}\/$/, "outputbox.text": /Serving http:\/\/127\.0\.0\.1/ },
    webDump: { "session.browserurl": /^$/, "outputbox.text": /Studio runs in a browser already[\s\S]*total12/ },
  },
  // ---- I8: the AI assistant (docs/manual/ai-assistant.md), against the mock provider ----
  {
    // Turned on, but no model chosen yet: still hidden — no pane, no AI
    // command, nothing sent
    name: "ai-hidden-until-ready",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=compatible,ai:base={mock},wait",
    delay: 3,
    dump: { "dock.layout": /^hidden .*\baipanel\b/m, "ai.model": /^$/, "ai.lastrequest": /^$/ },
  },
  {
    // A token meter: the session's tokens, and their cost from the prices
    // the user gave
    name: "ai-meter",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:prices=2/10,ai:ask=hello there,wait,wait,wait",
    delay: 5,
    capture: true,
    // (one line: 18 pixels, as it always was)
    dump: { "aimeter.caption": /^1,000 tokens sent, 20 received · about \$0\.0022$/, "ai.cost": /^\$0\.0022$/, "aimeter.height": /^18$/ },
  },
  {
    // The meter with cached tokens and prices is longer than a line of the
    // pane: it wraps, and its label is as tall as its lines — "· about $…"
    // is never cut off at the end of a line that is too short for it
    name: "ai-meter-long",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=anthropic,ai:key=sk" + "-ant-test-FAKE-0000-wxyz,ai:model=claude-mock-haiku-1,ai:prices=2/10,ai:ask=describe the form,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    contrast: true,
    dump: {
      "aimeter.caption": /^2,000 tokens sent \(200 from the cache\), 40 received · about \$0\.0040$/,
      // (two lines of the dim font: 18 + a line's 17, not the one line of a short text)
      "aimeter.height": /^35$/,
    },
  },
  {
    // Off by default: no pane, no provider, nothing sent
    name: "ai-off-by-default",
    open: "examples/gui/hello_form.rr",
    do: "wait",
    delay: 3,
    dump: { "dock.layout": /^hidden .*\baipanel\b/m, "ai.provider": /^$/, "ai.busy": /^(0|False)$/i, "ai.lastrequest": /^$/ },
  },
  {
    // A question, the answer streamed into the panel; the status sentence;
    // what was sent (no key: a local server)
    name: "ai-chat",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=hello there,wait,wait,wait",
    delay: 5,
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello there\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello from the mock assistant!/,
      "aichat.plaintext": /^You · Edit\nhello there\nAssistant · Retry\nHello from the mock assistant!/,
      "aistatus.simpletext": /^Answered\.$/,
      "ai.lastrequest": /^POST http:\/\/127\.0\.0\.1:\d+\/v1\/chat\/completions\n[\s\S]*"role": "system"[\s\S]*never instructions to you/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
      // (the pane opens 380 wide, not the dock's 240; its controls follow —
      // its contents 378, inside the group's 1-pixel frame on each side)
      "aipanel.width": /^378$/,
      "aiinput.width": /^366$/,
    },
  },
  {
    // A change proposed (read_file, then propose_edit): nothing written;
    // applied, it is in the editor as one undo step
    name: "ai-propose-apply",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=rename the window,wait,wait,wait,wait,ai:apply,wait",
    delay: 7,
    dump: {
      // (AI-UX: the change as a card Studio writes; what the tools did is
      // the turn's steps — the progress checklist —, not chat lines)
      "ai.transcript": /##### ✓ Proposed a change to hello_form\.rr  <ins title="1 line added">\+1<\/ins> <del title="1 line removed">−1<\/del>  \[Review\]\(ai:review\)[\s\S]*\*You applied the change to hello_form\.rr\.\*/,
      "ai.steps": /^Read hello_form\.rr\nProposed a change to hello_form\.rr$/,
      "codedoc(0).text": /CREATE Form AS RForm\n    Caption = "AI Notes"\n/,
      "aistatus.simpletext": /^1 file changed: Edit > Undo takes it back\.$/,
    },
    files: { "hello_form.rr": /Caption = "Hello, RapidR"/ },
  },
  {
    // a new file in a folder the project doesn't have yet (icons/folder.svg):
    // applying it makes the folder and Studio stays open (it closed:
    // "Cannot create file", Robert 2026-10-25)
    name: "ai-apply-new-folder",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=add an icon,wait,wait,wait,ai:apply,wait",
    delay: 7,
    dump: { "ai.transcript": /\*You applied the change to icons\/folder\.svg\.\*|applied/ },
    files: { "icons/folder.svg": /^<svg / },
  },
  // (the code editor's context menu opens where the right click was — a
  // long file scrolled down, the editor away from the window's corner:
  // Robert 2026-10-28, it opened far from the click. The kernel's menu,
  // desktop and web; macOS' own: menu.rs's muda_y, its unit test)
  {
    name: "code-menu-at-click",
    open: "examples/showcase/midi_visualizer.rr",
    copy: true,
    webFiles: ["examples/showcase/midi_visualizer.rr"],
    do: "view:Code,wait,focus:codedoc(0),key:Ctrl+End,wait,wait",
    events: "studio.__wait_3500,codedoc(0).__rightclick_220_160",
    capture: true,
    delay: 7,
    dump: {
      "codedoc(0).__popupat": /^220,160$/,
      "codedoc(0).caretline": /^(?!0$)\d+$/,
    },
  },
  // ---- a project as RAI left Robert's ShopInvoices: the main file a module
  // that main.rr includes; Run ran it, it ended at once, nothing said ----
  {
    name: "project-main-included",
    open: "tests/fixtures/studio_shop/Shop.rrproj",
    copyDir: true,
    desktopOnly: true,
    do: "wait,wait",
    delay: 5,
    dump: {
      "proj.mainfile": /^lang\.rr$/,
      "outputbox.text": /The main file lang\.rr is included by main\.rr: the program probably starts in main\.rr\. To run it from there, select the project's name in the Project pane, then MainFile in Properties\./,
    },
  },
  {
    // (…Run there: it ends at once, and says what to look at)
    name: "project-main-runs-module",
    open: "tests/fixtures/studio_shop/Shop.rrproj",
    copyDir: true,
    desktopOnly: true,
    do: "wait,run.start,wait,wait,wait,wait,wait,wait",
    delay: 9,
    dump: {
      "outputbox.text": /\[lang\.rr ended, exit code 0\]\nlang\.rr ran and ended at once: it showed no window and printed nothing\. If the program starts in another file, make that the main file/,
    },
  },
  {
    // (…the project's properties fix it: MainFile main.rr, written to the
    // project file; Run starts there)
    name: "project-main-fixed",
    open: "tests/fixtures/studio_shop/Shop.rrproj",
    copyDir: true,
    desktopOnly: true,
    do: "wait,project:select,prop:MainFile=main.rr,wait,run.start,wait,wait,wait,wait,wait,wait",
    delay: 10,
    dump: {
      "proj.mainfile": /^main\.rr$/,
      "outputbox.text": /Shop open\n\[main\.rr ended, exit code 0\](?![\s\S]*ended at once)/,
    },
    files: { "Shop.rrproj": /^main = "main\.rr"$/m },
  },
  {
    // (Run with nothing open says so)
    name: "run-nothing-open",
    open: "",
    desktopOnly: true,
    do: "run.start,wait",
    delay: 4,
    dump: { "outputbox.text": /Nothing to run: open a project or a program first/ },
  },
  // (the project tree's selection shows its file: lang/pt-BR.po selected
  // opens in the preview tab, the keys stay in the tree; lang/es.po
  // selected takes the preview tab over — never another file's text on
  // show, Robert 2026-10-29)
  {
    name: "project-tree-preview",
    open: "tests/fixtures/studio_assets/Langs.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_assets/Langs.rrproj", "tests/fixtures/studio_assets/main.rr", "tests/fixtures/studio_assets/lang/es.po", "tests/fixtures/studio_assets/lang/pt-BR.po"],
    do: "wait,focus:projecttree,key:Home,key:Down,key:Down,key:Down,key:Down,key:Down,key:Down,wait,wait,wait,key:Up,wait,wait,wait",
    delay: 9,
    dump: {
      "projecttree.selected": /^lang\/es\.po$/,
      "dock.activedocument": /^codedoc\(1\)$/,
      "codedoc(1).text": /Language: es\\n[\s\S]*Velocidad"/,
      "outputbox.text": /Opened .*lang\/pt-BR\.po\n> key:Up\nOpened .*lang\/es\.po/,
    },
  },
  // ---- Studio keeps your work (safety.inc; Robert 2026-10-28: Studio quit
  // on a run-time error in its own code, the unsaved work and the chat's
  // proposals with it) ----
  {
    // (a run-time error in Studio's own code — a file written into a folder
    // that isn't there —: Studio goes on, says it in a dialog and the Output,
    // logs it with its file and line, keeps a copy of the unsaved work)
    name: "studio-error-safe",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:PRINT 42,wait,safety:fail,wait,wait,wait",
    delay: 8,
    capture: true,
    dump: {
      "failnote.__shown": /^1$/,
      "failtitle.caption": /^Something went wrong in Studio$/,
      "failtext.caption": /Cannot create file …\/studio\/no-such-folder\/studio-test\.txt\. \(safety\.inc line \d+\)\n\nYour work is safe: nothing was lost, and Studio goes on/,
      "outputbox.text": /Something went wrong in Studio: Cannot create file .* \(safety\.inc line \d+\)\. Your work is safe; Studio goes on\./,
      "codedoc(0).text": /PRINT 42/,
      "codedoc(0).modified": /^(-1|1|True)$/i,
    },
    files: { "studio/studio-errors.log": /RapidR Studio \(RapidR [\d.]+\): Cannot create file .* \(at Studio's safety\.inc line \d+\)/ },
  },
  {
    // (…in the browser: a division by zero in Studio's own code — the
    // page's store has no folders to be missing —, said the same way)
    name: "studio-error-safe-web",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    webOnly: true,
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:PRINT 42,wait,safety:divide,wait,wait,wait",
    delay: 8,
    dump: {
      "failnote.__shown": /^1$/,
      "failtitle.caption": /^Something went wrong in Studio$/,
      "failtext.caption": /^Division by zero \(safety\.inc line \d+\)\n\nYour work is safe/,
      "outputbox.text": /Something went wrong in Studio: Division by zero \(safety\.inc line \d+\)\. Your work is safe; Studio goes on\./,
      "codedoc(0).text": /PRINT 42/,
    },
  },
  {
    // (unsaved work kept, Studio ended without closing — killed: the test's
    // end —; started again: "Restore your unsaved work?", Enter restores it
    // in its editor, not saved)
    name: "studio-restore-unsaved",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:PRINT 42,wait,safety:keep,wait",
    delay: 6,
    restart: { noOpen: true, do: "wait,wait,wait,key:Enter,wait,wait,wait", delay: 8 },
    dump: {
      "codedoc(0).text": /Form\.ShowModal\n+PRINT 42/,
      "codedoc(0).modified": /^(-1|1|True)$/i,
      "outputbox.text": /Restored the unsaved work of 1 file \(not saved yet\)\./,
      "saveprompt.__shown": /^0$/,
    },
    files: { "hello_form.rr": /^(?![\s\S]*PRINT 42)/ },
  },
  {
    // (a new file with Chinese in it: on disk as the proposal has it, UTF-8
    // — the old Apply wrote a byte a character, "??? 3D")
    name: "ai-apply-utf8",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=add a chinese translation,wait,wait,wait,ai:apply,wait",
    delay: 7,
    dump: { "aistatus.simpletext": /^1 file changed/ },
    files: { "lang/zh-Hans.po": /^msgid ""\nmsgstr ""\n"Content-Type: text\/plain; charset=UTF-8\\n"\n"Language: zh-Hans\\n"\n\nmsgid "Solar System 3D"\nmsgstr "太阳系 3D"\n\nmsgid "Speed"\nmsgstr "速度"\n$/ },
  },
  {
    // (an Apply that can't be done whole changes nothing: the rename, the
    // icon in a new folder and a file that can't be made — none applied, the
    // folder not made, why said; the changes still wait to be reviewed)
    name: "ai-apply-all-or-nothing",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=add three things,wait,wait,wait,wait,ai:apply,wait",
    delay: 8,
    desktopGone: ["icons"],
    dump: {
      "aistatus.simpletext": /^Nothing was changed: hello_form\.rr\/blocked\.txt: its folder can't be made: .*hello_form\.rr is a file\. The changes still wait \(Review\)\.$/,
      "aireviewbar.__shown": /^1$/,
      "codedoc(0).text": /Caption = "Hello, RapidR"/,
      "codedoc(0).modified": /^(0|False)$/i,
      "outputbox.text": /AI: not applied — hello_form\.rr\/blocked\.txt: its folder can't be made/,
    },
    same: { "codedoc(0).text": "examples/gui/hello_form.rr" },
  },
  {
    // (changes left waiting are kept with the chat: Studio started again —
    // the project's chat opened —, they are still there to apply)
    name: "ai-proposals-kept",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=rename the window,wait,wait,wait,wait,wait",
    delay: 7,
    restart: { do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,wait,wait,ai:apply,wait", delay: 7 },
    dump: {
      "aireviewbar.__shown": /^0$/,
      "codedoc(0).text": /Caption = "AI Notes"/,
      "aistatus.simpletext": /^1 file changed: Edit > Undo takes it back\.$/,
    },
  },
  {
    name: "ai-apply-undo",
    open: "examples/gui/hello_form.rr",
    copy: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=rename the window,wait,wait,wait,wait,ai:apply,wait,focus:codedoc(0),edit.undo,wait",
    delay: 8,
    dump: { "codedoc(0).canundo": /^(0|False)$/i, "codedoc(0).text": /Caption = "Hello, RapidR"/ },
    same: { "codedoc(0).text": "examples/gui/hello_form.rr" },
  },
  {
    // "Build me …" with Review first on (Tools > Settings > AI: "Ask
    // before the assistant creates or changes files"): a new project
    // proposed (checked by the compiler), previewed, created where the
    // user says, opened, and it runs
    name: "ai-new-program",
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:reviewfirst=1,ai:ask=build me a hello-form program,wait,wait,wait,ai:create={dir}/made,wait,wait,run.start,wait,wait,wait,wait",
    delay: 10,
    dump: {
      "ai.transcript": /##### ✓ Created \*hello-form\*  main\.rr · compiles  \[Open\]\(ai:open\/hello-form\) · \[Undo\]\(ai:undo\/hello-form\)/,
      "proj.folder": /made\/hello-form$/,
      "proj.mainfile": /^main\.rr$/,
      "outputbox.text": /^(?![\s\S]*Can't run)/,
      "session.exitcode": /^0?$/,
    },
    webDump: {
      "ai.transcript": /##### ✓ Created \*hello-form\*  main\.rr · compiles/,
      "proj.folder": /made\/hello-form$/,
      "proj.mainfile": /^main\.rr$/,
      "session.state": /^running$/,
    },
  },
  {
    // ... Review first: nothing is created until Create — the answer
    // leaves the Preview button, the program waits
    name: "ai-review-first",
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:reviewfirst=1,ai:ask=build me a hello-form program,wait,ai:wait,wait",
    delay: 6,
    dump: {
      "aireviewbtn.caption": /^Preview the New Program\.\.\.$/,
      "aireviewbar.__shown": /^1$/,
      "aibanner.__shown": /^0$/,
      "aistatus.simpletext": /^Waiting for you — The assistant made a new program: preview it, then create it\.$/,
      "aitools.createdfolder": /^$/,
      "proj.folder": /^$/,
    },
  },
  // ---- Robert's evening with DeepSeek (docs/preview4-plan.md R3, R10, R11),
  // against the mock; tests/fixtures/studio_ai_sites is a small three-file
  // project (main.rr includes server.rr and a 297-line sites.rr) ----
  {
    // R3: Send while the assistant works (the answer held 6 s) queues the
    // message — typed with real keys, Enter — shown above the box as
    // "Queued: …"; nothing stops, nothing is sent yet
    name: "ai-r3-queued",
    open: "examples/gui/hello_form.rr",
    copy: true,
    // (the queued line in every look: its text readable, WCAG AA)
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    contrast: true,
    capture: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=slowly rename the window,wait,wait,ai.assistant,type:and add a Clear button,key:Enter,wait",
    delay: 4,
    webDelay: 7,
    dump: {
      "aiqueuebar.__shown": /^1$/,
      "aiqueuetext.caption": /^Queued: and add a Clear button$/,
      "aiinput.text": /^$/,
      "ai.busy": /^(-1|True)$/i,
      "aisendbtn.enabled": /^(-1|1|True)$/i,
      "aistopbtn.enabled": /^(-1|1|True)$/i,
      "ai.transcript": /^(?![\s\S]*Stopped)(?![\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nand add a Clear button)[\s\S]*slowly rename the window/,
    },
  },
  {
    // ... and sent when the answer ends: the second question and its
    // answer in the chat, the queued line gone
    name: "ai-r3-queued-sent",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=slowly rename the window,wait,wait,ai.assistant,type:and add a Clear button,key:Enter,wait",
    delay: 14,
    webDelay: 22,
    dump: {
      "aiqueuebar.__shown": /^0$/,
      "ai.busy": /^(0|False)$/i,
      "ai.transcript": /^(?![\s\S]*Stopped)\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nslowly rename the window[\s\S]*Proposed a change to hello_form\.rr[\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nand add a Clear button\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nA \*\*Clear\*\* button next to Greet would empty the name box/,
    },
  },
  {
    // ... × (the mouse) takes it back: the line goes, the status says it
    // won't be sent; the answer goes on
    name: "ai-r3-queued-cancel",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=slowly rename the window,wait,wait,ai.assistant,type:and add a Clear button,key:Enter,wait",
    events: "aiqueuecancel.__mousedown_14_13,aiqueuecancel.__mouseup_14_13,aiqueuetext.onclick,aiqueuetext.onclick",
    delay: 5,
    dump: {
      "aiqueuebar.__shown": /^0$/,
      "aistatus.simpletext": /^The queued message won't be sent\.$/,
      "ai.busy": /^(-1|True)$/i,
      "ai.transcript": /^(?![\s\S]*and add a Clear button)/,
    },
  },
  {
    // ... Stop (the mouse) ends the answer and gives the queued message
    // back to the box, unsent
    name: "ai-r3-queued-stop",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=slowly rename the window,wait,wait,ai.assistant,type:and add a Clear button,key:Enter,wait",
    events: "aistopbtn.__mousedown_20_12,aistopbtn.__mouseup_20_12,aiqueuetext.onclick,aiqueuetext.onclick",
    delay: 5,
    dump: {
      "aiqueuebar.__shown": /^0$/,
      "aiinput.text": /^and add a Clear button$/,
      "aistatus.simpletext": /^Stopped\. Your queued message is back in the box, not sent\.$/,
      "ai.busy": /^(0|False)$/i,
      "ai.transcript": /Stopped\.(?![\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nand add a Clear button)/,
    },
  },
  {
    // R10: the reply's language follows the user's latest message, even
    // after the conversation is summarised (forced: SummaryAt 1). The
    // mock's polyglot model answers in the language its instructions name,
    // else in the conversation's first message's — after a summary, the
    // summary's, which it writes in German unless told: the drift of the
    // evening. English stays English; a question in Spanish gets Spanish.
    name: "ai-r10-language",
    capture: true,
    open: "tests/fixtures/studio_ai_sites/main.rr",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_ai_sites/main.rr", "tests/fixtures/studio_ai_sites/server.rr", "tests/fixtures/studio_ai_sites/sites.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-polyglot,ai:summaryat=1,ai:ask=What does the form show?,ai:wait,ai:ask=And the list?,ai:wait,ai:ask=Is that all of it?,ai:wait,ai:ask=and the button?,ai:wait,ai:ask=¿Y el botón Start?,ai:wait",
    delay: 12,
    dump: {
      "ai.transcript": /^(?![\s\S]*Das Formular)[\s\S]*The conversation so far was summarised[\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nand the button\?\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nThe form has a list of sites[\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\n¿Y el botón Start\?\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nEl formulario tiene una lista de sitios/,
      "ai.userlanguage": /^Spanish$/,
      // (the last request: the chat summarised again, now in Spanish)
      "ai.lastrequest": /Write the summary in Spanish \(español\) \(the language the user writes in\)/,
    },
  },
  {
    // R11: a change to a module waits for the user (not reviewed), the
    // next question's change to main.rr calls what only that change has:
    // checked together, it compiles (it was refused); each card is its own
    // answer's. Applied: "You applied" once per file — a review applied
    // again says nothing more (it said "You rejected" for the same files)
    name: "ai-r11-waiting-together",
    capture: true,
    open: "tests/fixtures/studio_ai_sites/main.rr",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_ai_sites/main.rr", "tests/fixtures/studio_ai_sites/server.rr", "tests/fixtures/studio_ai_sites/sites.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=add StopServer to the server,ai:wait,ai:ask=now add a Stop button to main.rr,ai:wait,wait,wait,ai:apply,wait,ai:apply,wait",
    delay: 8,
    dump: {
      "ai.steps": /^Proposed a change to main\.rr$/,
      "ai.transcript": /##### ✓ Proposed a change to server\.rr  <ins title="5 lines added">\+5<\/ins> <del title="0 lines removed">−0<\/del>[\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nnow add a Stop button to main\.rr\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\n(?:\[✓ \d+ steps? · show\]\(rai:steps\/\d+\)\n\n)?##### ✓ Proposed a change to main\.rr  <ins title="10 lines added">\+10<\/ins> <del title="0 lines removed">−0<\/del>  \[Review\]\(ai:review\)\n\nPress \*\*Stop\*\*(?![\s\S]*server\.rr ·)[\s\S]*\*You applied the change to server\.rr\.\*\n\n\*You applied the change to main\.rr\.\*$/,
      "aitools.proposalcount": /^0$/,
      // (both files changed in their editors: main.rr open, server.rr opened)
      "codedoc(0).text": /StopServer/,
      "codedoc(1).text": /SUB StopServer/,
    },
  },
  {
    // ... applied once: both changes in their editors, and the program
    // has no error (the module, changed and not saved, counted as its
    // editor has it — "1 error" was said of a program that had none)
    name: "ai-r11-applied-status",
    open: "tests/fixtures/studio_ai_sites/main.rr",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_ai_sites/main.rr", "tests/fixtures/studio_ai_sites/server.rr", "tests/fixtures/studio_ai_sites/sites.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=add StopServer to the server,ai:wait,ai:ask=now add a Stop button to main.rr,ai:wait,wait,wait,ai:apply,wait,wait",
    delay: 8,
    dump: {
      "aistatus.simpletext": /^2 files changed: Edit > Undo in each takes it back\.$/,
      "aibanner.__shown": /^0$/,
    },
  },
  {
    // R11: a two-line fix sent as the whole 297-line file is refused (the
    // model is told to send the parts); the two edits are proposed: the
    // card counts the two lines
    name: "ai-r11-hunks",
    capture: true,
    open: "tests/fixtures/studio_ai_sites/main.rr",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_ai_sites/main.rr", "tests/fixtures/studio_ai_sites/server.rr", "tests/fixtures/studio_ai_sites/sites.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=name sites 7 and 8,ai:wait",
    delay: 6,
    dump: {
      "ai.steps": /^Read sites\.rr\nRefused a whole-file rewrite of sites\.rr: only a few lines change\nProposed a change to sites\.rr$/,
      "ai.transcript": /##### ✓ Proposed a change to sites\.rr  <ins title="2 lines added">\+2<\/ins> <del title="2 lines removed">−2<\/del>  \[Review\]\(ai:review\)\n\nSites 7 and 8 are called seven and eight now\.$/,
      "aitools.proposalcount": /^1$/,
      "aistatus.simpletext": /^Waiting for you — The assistant proposed a change: review it\.$/,
    },
  },
  {
    // R11: a file cut off at `END FUNCTI` is refused in plain words —
    // nothing to review, a card says so
    name: "ai-r11-cut-off",
    capture: true,
    open: "tests/fixtures/studio_ai_sites/main.rr",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_ai_sites/main.rr", "tests/fixtures/studio_ai_sites/server.rr", "tests/fixtures/studio_ai_sites/sites.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=finish the count,ai:wait",
    delay: 6,
    dump: {
      "ai.transcript": /##### × Refused sites\.rr: it is cut off at the end\n\nMy change was cut off before its end/,
      "aitools.proposalcount": /^0$/,
      "aireviewbar.__shown": /^0$/,
      "aistatus.simpletext": /^Answered\.$/,
    },
  },
  {
    // R11: the name not found (DNS) on the first try — the walk's stand-in
    // for "Couldn't reach api.deepseek.com" —: tried again 2 s later, the
    // answer comes, no error shown
    name: "ai-r11-retry-dns",
    desktopOnly: true,
    open: "examples/gui/hello_form.rr",
    env: { RAPIDR_AI_TEST_DNS_FAIL_ONCE: "1" },
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=hello there,ai:wait",
    delay: 6,
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello there\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello from the mock assistant!(?![\s\S]*Error)/,
      "aistatus.simpletext": /^Answered\.$/,
    },
  },
  {
    // ... a connection closed before any answer, every other time (the
    // mock's `drop`): each request tried once more, the answer comes
    name: "ai-r11-retry-drop",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mockroot}/s/drop/v1,ai:model=mock-small,ai:ask=hello there,ai:wait",
    delay: 9,
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello there\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello from the mock assistant!(?![\s\S]*Error)/,
      "aistatus.simpletext": /^Answered\.$/,
    },
  },
  // ---- Preview 4 R1–R6, K8 (docs/preview4-plan.md): chats kept, a sent
  // question edited, the live status line, the context meter and the /
  // commands, the effort and "Thought for N s", one model list with ↻,
  // Set up RAI, the lite set for a small window ----
  {
    // R1: Studio closed and started again on the same project — its chat
    // is there, as it was (kept in Studio's data folder: here the case's)
    name: "ai-r1-kept-restart",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello there,ai:wait,wait`,
    delay: 4,
    restart: { do: "wait,wait", delay: 4 },
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello there\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello from the mock assistant!/,
      "aichattitle.caption": /^hello there$/,
      "ai.questioncount": /^1$/,
    },
  },
  {
    // R1: New Chat keeps the old one; Chats (the mouse) lists both, the
    // older picked and opened — its conversation back, the newer kept
    name: "ai-r1-chats-switch",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai.newChat,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichatsbtn", 20, 12), "aichatlist.__item_1", CLICK("aichatsopenbtn", 20, 12), idle(4)].join(","),
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello one\n\n(?![\s\S]*hello two)/,
      "aichattitle.caption": /^hello one$/,
      "aichatspanel.__shown": /^0$/,
      "aichat.__shown": /^1$/,
      "aistatus.simpletext": /^Opened "hello one"\.$/,
    },
  },
  {
    // ... the list itself: both chats, the newest first and marked open;
    // where they are kept said under it (never the project's folder)
    name: "ai-r1-chats-list",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai.newChat,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichatsbtn", 20, 12), idle(4)].join(","),
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aichatlist.itemcount": /^2$/,
      // (the list's items, a line each)
      "aichatlist.text": /^hello two — 1 question, \d{4}-\d\d-\d\d \(open\)\r?\nhello one — 1 question, \d{4}-\d\d-\d\d\s*$/,
      "aichatsnote.caption": /^Kept on this computer, in Studio's own folder — never in the project\. Delete removes a chat for good\.$/,
      "aichatsnote.hint": /chats$|^this browser's storage for the page$/,
      "aibottom.__shown": /^0$/,
      "aichatsbtn.caption": /^Back$/,
    },
  },
  {
    // R1: Delete (the mouse) removes a chat — Undo offered beside the
    // list; Undo (the mouse) puts it back
    name: "ai-r1-chats-delete",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai.newChat,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichatsbtn", 20, 12), "aichatlist.__item_1", CLICK("aichatsdeletebtn", 20, 12), idle(4)].join(","),
    delay: 5,
    dump: {
      "aichatlist.itemcount": /^1$/,
      "aichatundobar.__shown": /^1$/,
      "aichatundotext.caption": /^Deleted "hello one"\.$/,
      "aistatus.simpletext": /^Deleted "hello one"\. Undo puts it back\.$/,
      "ai.transcript": /hello two/,
    },
  },
  {
    name: "ai-r1-chats-undo",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai.newChat,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichatsbtn", 20, 12), "aichatlist.__item_1", CLICK("aichatsdeletebtn", 20, 12), idle(2), CLICK("aichatundobtn", 20, 12), idle(4)].join(","),
    delay: 5,
    dump: {
      "aichatlist.itemcount": /^2$/,
      "aichatundobar.__shown": /^0$/,
      "aistatus.simpletext": /^Put back: "hello one"\.$/,
    },
  },
  {
    // R1: Search (typed) finds a chat by what was said, not only its
    // title; Rename (the mouse, a name typed, Enter) names it
    name: "ai-r1-chats-search-rename",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai.newChat,ai:ask=check the code,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichatsbtn", 20, 12), typeInto("aichatsearch", "rename"), idle(2), CLICK("aichatsrenamebtn", 20, 12), "airenameedit.__key_65_16", typeInto("airenameedit", "Greetings"), "airenameedit.__key_13", idle(4)].join(","),
    delay: 5,
    dump: {
      "aichatlist.itemcount": /^1$/,
      "aichatlist.text": /^Greetings — 1 question, .*: "Hello from the mock assistant! Ask me to \*\*rename\*\*/,
      "aistatus.simpletext": /^Renamed: "Greetings"\.$/,
    },
  },
  {
    // R2: Edit under the first question (the keyboard: Tab to the link,
    // Enter) — the bar says what Send will do; nothing changed yet
    name: "ai-r2-edit-bar",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: ["aichat.__key_9", "aichat.__key_13", idle(4)].join(","),
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aieditbar.__shown": /^1$/,
      "aiedittext.caption": /^Editing your question\. Send replaces it: its answer and the 1 question after it move to a saved copy of this chat \(in Chats\)\.$/,
      "aiinput.text": /^hello one$/,
      "ai.questioncount": /^2$/,
    },
  },
  {
    // ... the question changed and sent (typed, Enter): the replies below
    // it moved to a saved copy — nothing lost —, the new one asked
    name: "ai-r2-edit-send",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: ["aichat.__key_9", "aichat.__key_13", idle(2), "aiinput.__key_65_16", typeInto("aiinput", "say hello please"), "aiinput.__key_13", "ai.__until_busy_0", idle(2), CLICK("aichatsbtn", 20, 12), idle(4)].join(","),
    delay: 5,
    dump: {
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nsay hello please\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello, friend, nice meeting you\.(?![\s\S]*hello two)/,
      "aichatlist.itemcount": /^2$/,
      "aichatlist.text": /hello one \(before the edit\) — 2 questions, /,
      "outputbox.text": /AI: the chat before the edit is kept as "hello one \(before the edit\)" \(Chats\)\./,
      "aieditbar.__shown": /^0$/,
    },
  },
  {
    // R2: Retry under the last answer: asked again, the answer before kept
    name: "ai-r2-retry",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,wait,ai:retry,ai:wait,wait,ai:chats`,
    delay: 6,
    dump: {
      "ai.questioncount": /^1$/,
      "aichatlist.itemcount": /^2$/,
      "aichatlist.text": /hello one \(before Retry\) — 1 question, /,
      "outputbox.text": /AI: the answer before Retry is kept in "hello one \(before Retry\)" \(Chats\)\./,
    },
  },
  {
    // R4: the live status line while the model writes (paced: a piece
    // every 250 ms) — what it does now first, the steps done after, the
    // seconds beside Stop
    name: "ai-r4-status",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=trickle rename the window,wait,wait,wait,wait,wait,wait`,
    delay: 3.5,
    webDelay: 6,
    capture: true,
    dump: {
      "aistatus.simpletext": /^(Reading hello_form\.rr|Writing hello_form\.rr|Thinking|Writing the answer)…( · ✓ |$)/,
      "aielapsed.caption": /^\d+ s$/,
      "ai.busy": /^(-1|True)$/i,
    },
  },
  {
    // RAI shows its work (2.121.2): the strip under the conversation while
    // a long turn goes (paced: a piece every 250 ms) — three dots, what it
    // does now in words, its seconds, the tokens so far, the newest steps
    // ✓ — and a capture of it
    name: "ai-live-status",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:reviewfirst=1,ai:motion=1,ai:ask=trickle build a long list app,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait`,
    delay: 9,
    webDelay: 12,
    capture: true,
    // (the strip judged in every look and at both scales: its text and the dots' ground)
    themes: LOOKS,
    scales: [1, 2],
    contrast: true,
    // (what a screen reader is told: the strip's text, and no node for the
    // decoration — the dots — beside it)
    a11y: [[/^What the assistant is doing now$/, "", "", (n, nodes) => !nodes.some((x) => x.role === "canvas" && x.bounds && n.bounds && Math.abs(x.bounds[1] - n.bounds[1]) < 14 && x.bounds[0] < n.bounds[0])]],
    dump: {
      "ailive.__shown": /^(1|-1|True)$/i,
      "ailivetext.caption": /^(Thinking|Reading the help of RListBox|Looking at the form|Writing the answer|Continuing \(part 2\)|Writing Time List( \(\+\d+ lines\))?|Checking \(compiling\)|Fixing 1 problem|Fixing the errors)…$/,
      // (each thing said once: the tokens in the meter's line, the seconds in
      // the strip, the status line hidden while the strip shows, nothing beside Stop)
      "aimeter.caption": /^This answer: [\d,]+ in( \([\d,]+ cached\))? · [\d,]+ out( · part 2 of 5)?$/,
      "ailivetime.caption": /^\d+ s$/,
      "aistatus.__shown": /^(0|False)$/i,
      "aielapsed.__shown": /^(0|False)$/i,
      "ailivesteps.caption": /^((\d+ earlier steps\n)?(✓ [^\n]+\n?)*)$/,
      "ai.busy": /^(-1|True)$/i,
    },
  },
  {
    // RAI shows its work: the whole long turn — help read, the form looked
    // at, an answer cut at its length limit that goes on by itself (one
    // muted line, in order with the replies, the open code block closed),
    // a program with an error fixed — leaves the answer with its steps
    // counted ("✓ N steps · show"), its RapidR code coloured (a block with no
    // language that looks like RapidR's is labelled so) and Copy · Insert
    // under each block; the strip is gone
    name: "ai-live-long-turn",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:reviewfirst=1,ai:ask=build a long list app,ai:wait,wait,wait`,
    delay: 6,
    capture: true,
    // (RapidR code in the chat judged in every look: the editor's colours)
    themes: LOOKS,
    contrast: true,
    dump: {
      "ai.transcript": /\[✓ \d+ steps · show\]\(rai:steps\/1\)[\s\S]*Answer was long: continuing \(part 2 of up to 5\)\*[\s\S]*```rapidr\nDIM Shown AS INTEGER[\s\S]*\[Copy\]\(rai:copy\/3\) · \[Insert\]\(rai:insert\/3\)/,
      "ai.part": /^2$/,
      "ai.busy": /^(0|False)$/i,
      "ailive.__shown": /^(0|False)$/i,
      // (the status line is back, the meter has the session's totals again)
      "aistatus.__shown": /^(1|-1|True)$/i,
      "aimeter.caption": /^[\d,]+ tokens sent, [\d,]+ received/,
      "aichat.codelanguage": /^rapidr$/,
    },
  },
  {
    // RAI shows its work: an answer that keeps reaching its length limit —
    // RAI goes on by itself four times, one muted line each, then says
    // plainly why it stopped and offers "Continue in smaller steps", which
    // the keyboard follows (Tab through the links to the last, Enter) and which
    // sends that request
    name: "ai-live-endless",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=write an endless answer,ai:wait,wait,ai.assistant`,
    events: [...Array(14).fill("aichat.__key_9"), "aichat.__key_13", "ai.__until_busy_0", idle(4)].join(","),
    delay: 8,
    capture: true,
    dump: {
      "ai.transcript": /part 2 of up to 5[\s\S]*part 3 of up to 5[\s\S]*part 4 of up to 5[\s\S]*part 5 of up to 5[\s\S]*RAI stopped here: the answer kept reaching its length limit \(4,096 tokens\), 5 parts in a row[\s\S]*\[Continue in smaller steps\]\(rai:continue\)[\s\S]*\*\*You\*\*[^\n]*\n\nContinue where you stopped, in smaller steps[\s\S]*Done in smaller steps\./,
      "ai.busy": /^(0|False)$/i,
    },
  },
  {
    // RAI shows its work: a tool call cut off at the length limit (its
    // arguments not whole) is never carried out — the model is told to write
    // it again in smaller parts, the chat says the answer was long, and only
    // the small file that follows is proposed
    name: "ai-live-cut-call",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=write a cut file,ai:wait,wait,wait`,
    delay: 5,
    dump: {
      "ai.transcript": /Answer was long: continuing \(part 2 of up to 5\)\*[\s\S]*Done: a small `notes\.rr`\./,
      "ai.steps": /write_file: cut off, to be written again in smaller parts[\s\S]*notes\.rr/i,
      "aitools.proposalcount": /^1$/,
    },
  },
  {
    // RAI shows its work: the system asks for reduced motion
    // (RAPIDR_REDUCED_MOTION on the desktop, prefers-reduced-motion on the
    // page): the dots stand still — the pulse's timer never runs — and
    // Application.ReducedMotion says so; with the pulse (the case before) it runs
    name: "ai-live-reduced-motion",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    reducedMotion: true,
    do: `${AI_READY},ai:ask=trickle build a long list app,wait,wait,wait,wait,wait`,
    delay: 5,
    webDelay: 7,
    capture: true,
    dump: {
      "ailive.__shown": /^(1|-1|True)$/i,
      "aibeat.enabled": /^(0|False)$/i,
      "ai.busy": /^(-1|True)$/i,
    },
  },
  {
    // RAI shows its work: with the pulse (no reduced motion) its timer runs while it works
    name: "ai-live-pulse",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=trickle build a long list app,wait,wait,wait,wait,wait`,
    delay: 5,
    webDelay: 7,
    dump: {
      "ailive.__shown": /^(1|-1|True)$/i,
      "aibeat.enabled": /^(-1|1|True)$/i,
    },
  },
  {
    // RAI shows its work: the card of a proposed change reads as one design —
    // the step icon, the file, the counts as a group of their own (+ in the
    // diff's green, − in its red, in the code font, the signs kept), Review
    // — judged in every look; a screen reader hears "Proposed a change to
    // hello_form.rr, 1 line added, 1 line removed, Review"
    name: "ai-card-proposed",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=rename the window,ai:wait,wait,wait`,
    delay: 5,
    capture: true,
    themes: LOOKS,
    scales: [1, 2],
    contrast: true,
    a11y: [[/^Proposed a change to hello_form\.rr, 1 line added, 1 line removed, Review$/, "heading"]],
    dump: {
      "ai.transcript": /##### ✓ Proposed a change to hello_form\.rr\u2003\u2003<ins title="1 line added">\+1<\/ins> <del title="1 line removed">−1<\/del>\u2003\u2003\[Review\]\(ai:review\)/,
    },
  },
  {
    // RAI shows its work: Stop (the mouse) while it works — the strip goes,
    // the status line is back saying so, the token meter has the session's
    // totals again (not "This answer"), nothing keeps ticking
    name: "ai-live-stop",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:motion=0,ai:ask=trickle build a long list app,wait,wait,wait,wait,wait`,
    events: [CLICK("aistopbtn", 20, 12), idle(3)].join(","),
    delay: 6,
    webDelay: 9,
    capture: true,
    dump: {
      "ailive.__shown": /^(0|False)$/i,
      "aistatus.__shown": /^(1|-1|True)$/i,
      "aistatus.simpletext": /^Stopped\.$/,
      "aimeter.caption": /^[\d,]+ tokens sent|^$/,
      "ai.busy": /^(0|False)$/i,
    },
  },
  {
    // RAI shows its work: a provider that fails after its tools (the mock's
    // "then fail"): the strip goes with the turn, the error is said where the
    // status line is, and the next question works
    name: "ai-live-error",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=then fail rename the window,ai:wait,wait,wait`,
    delay: 5,
    capture: true,
    dump: {
      "ailive.__shown": /^(0|False)$/i,
      "aistatus.__shown": /^(1|-1|True)$/i,
      "aistatus.simpletext": /ran out of memory|interrupted/i,
      "ai.busy": /^(0|False)$/i,
    },
  },
  {
    // RAI shows its work: the smart chat — RapidR words in an answer, inline
    // and in a code block, coloured as the editor colours them
    name: "ai-smart-chat",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=show me the words,ai:wait,wait,ai.assistant`,
    delay: 5,
    capture: true,
    dump: {
      "ai.transcript": /Use a `RListBox` and `PRINT` it:\n\n```basic\nSHOWMESSAGE "Hi"/,
    },
  },
  {
    // RAI shows its work: the mouse over a word of RapidR code in an answer
    // (inline `RListBox`) rests there: its one-line help is the tooltip
    name: "ai-smart-hover",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=show me the words,ai:wait,wait,ai.assistant`,
    events: ["aichat.__mousemove_40_14", "aichat.__mousemove_69_14", idle(3)].join(","),
    delay: 5,
    capture: true,
    dump: {
      "aichat.__tip": /^RListBox — Component: .+/,
    },
  },
  {
    // RAI shows its work: a click on that word opens its entry in the Help
    // pane (its search box says the word, the page is the component's)
    name: "ai-smart-click",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=show me the words,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichat", 69, 14), idle(4)].join(","),
    delay: 5,
    capture: true,
    dump: {
      "helpsearch.text": /^RListBox$/,
      "helppage.text": /RListBox/,
    },
  },
  {
    // RAI shows its work: a click on a keyword in a code block (SHOWMESSAGE)
    // opens its entry too
    name: "ai-smart-click-block",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=show me the words,ai:wait,wait,ai.assistant`,
    events: [CLICK("aichat", 61, 51), idle(4)].join(","),
    delay: 5,
    capture: true,
    dump: {
      "helpsearch.text": /^SHOWMESSAGE$/,
    },
  },
  {
    // RAI shows its work: never a silent wait — the provider says nothing
    // (the mock's `slow`), and the strip says what it waits for, with the
    // seconds (here after 2 s: the tests' threshold; 10 s as the user has it)
    name: "ai-live-waiting",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mockroot}/s/slow/v1,ai:model=mock-small,ai:waitafter=2,ai:motion=1,ai:ask=hello there,wait,wait,wait,wait,wait,wait,wait",
    delay: 6,
    capture: true,
    dump: {
      "ailivetext.caption": /^Waiting for the server's answer… \d+ s$/,
      "ailive.__shown": /^(1|-1|True)$/i,
    },
  },
  {
    // R4: a provider that says nothing (the mock's `slow`): the status
    // says so (here after 2 s: the tests' threshold), wait or Stop
    name: "ai-r4-quiet",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mockroot}/s/slow/v1,ai:model=mock-small,ai:quietafter=2,ai:ask=hello there,wait,wait,wait,wait,wait,wait,wait,wait,wait",
    delay: 5,
    dump: {
      "aistatus.simpletext": /^No answer from the model for \d+ s: wait, or Stop\.$/,
      "aistopbtn.enabled": /^(-1|1|True)$/i,
    },
  },
  {
    // R5: the context meter, and Compact (the mouse): the chat summarised,
    // said in it and in the status; its questions before can't be edited
    name: "ai-r5-compact",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai:ask=hello two,ai:wait,wait,ai.assistant`,
    events: [CLICK("aicompactbtn", 20, 12), "ai.__until_busy_0", idle(4)].join(","),
    delay: 5,
    dump: {
      "ai.transcript": /\*The chat was compacted: what each question sends of it went from about [\d,]+ tokens to about [\d,]+\.\*(?![\s\S]*rai:edit)/,
      "aistatus.simpletext": /^The chat was compacted: what each question sends of it went from about [\d,]+ tokens to about [\d,]+\.$/,
      "aicontexttext.caption": /^Context: [\d,]+ of 11,468 \(\d+%\)$/,
    },
  },
  {
    // R5: "/" typed lists the commands; "/c" narrows them; Escape closes
    name: "ai-r5-slash-list",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,wait,ai.assistant,type:/c`,
    delay: 4,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aislashbar.__shown": /^1$/,
      "aislashlist.itemcount": /^2$/,
      "aislashlist.text": /^\/compact {3}Summarise this chat now[^\n]*\r?\n\/chats {3}This project's chats/,
    },
  },
  {
    // ... /compact and Enter: the chat compacted, the box empty, nothing
    // sent to the model as a question
    name: "ai-r5-slash-compact",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello one,ai:wait,ai:ask=hello two,ai:wait,wait,ai.assistant,type:/compact,key:Enter,ai:wait,wait`,
    delay: 6,
    dump: {
      "ai.transcript": /The chat was compacted(?![\s\S]*\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\n\/compact)/,
      "aiinput.text": /^$/,
      "aislashbar.__shown": /^0$/,
    },
  },
  {
    // R5: Effort High (the list, the mouse) with Anthropic (the mock): the
    // request asks it to think, the answer shows "Thought for N s"; its
    // link (the keyboard) shows the reasoning
    name: "ai-r5-effort-thought",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `ai:on,ai:provider=anthropic,ai:key=mock-good-key-1,ai:fetch,wait,wait,ai:model=claude-mock-haiku-1,ai.assistant,wait`,
    events: ["aieffortbox.__item_3", idle(2), typeInto("aiinput", "hello"), "aiinput.__key_13", "ai.__until_busy_0", idle(2), "aichat.__key_9", "aichat.__key_9", "aichat.__key_9", "aichat.__key_13", idle(4)].join(","),
    delay: 4,
    capture: true,
    dump: {
      "ai.lastrequest": /"thinking": \{\s*"type": "adaptive",\s*"display": "summarized"\s*\},\s*"output_config": \{\s*"effort": "high"/,
      "ai.transcript": /\[Thought for \d+ s · hide\]\(rai:thought\/1\)\n\n> The user wants a small change\./,
      "aieffortbox.text": /^Effort: High$/,
      // (the view stays where the link was clicked: it follows the end
      // again at the next question)
      "aichat.followend": /^(0|False)$/i,
    },
  },
  {
    // R6: one model list — the provider is Settings' — and ↻ (the mouse)
    // asks for the models again: the list has them
    name: "ai-r6-refresh",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},wait,ai.assistant`,
    events: [CLICK("airefreshbtn", 12, 12), "ai.__until_busy_0", idle(4)].join(","),
    delay: 4,
    dump: {
      "aimodelbox.itemcount": /^2$/,
      "aimodelbox.text": /^mock-small$/,
      "airefreshbtn.enabled": /^(-1|1|True)$/i,
      "aistatus.simpletext": /^The server lists 2 models\.$/,
      "aimodelbox.accessibledescription": /^A model of the server at http:\/\/127\.0\.0\.1:\d+\/v1\. The provider is chosen in Tools > Settings > AI, or Set up RAI\.$/,
    },
  },
  {
    // R6 + LOCAL-AI (docs/local-ai-plan.md): Set up RAI, free on this
    // computer — the models of a local AI app found (the tests' addresses:
    // the mock's local AI app, a closed port; the memory the tests'
    // 16 GB with 12 free — never this computer's or the user's servers),
    // said in plain words with no product named
    name: "ai-r6-setup-found",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:setupprobes={mockroot}/s/local/v1|http://127.0.0.1:9/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8)].join(","),
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    window: 2,
    dump: {
      "aisetup.visible": /^(-1|1|True)$/i,
      "aisufoundtext.caption": /^✓ Found 3 models in a local AI app on this computer\.$/,
      "aisulocal.checked": /^(-1|1|True)$/i,
      "aisulocaltext.caption": /local AI app(?![\s\S]*(Ollama|LM Studio|MLX))/,
    },
    // (the web: a browser can't look around this computer — the address
    // typed is asked: the first one, as the tests type it; every model
    // listed, none described)
    webDump: {
      "aisetup.visible": /^(-1|1|True)$/i,
      "aisufoundtext.caption": /^✓ Found 4 models in a local AI app on this computer\.$/,
      "aisulocal.checked": /^(-1|1|True)$/i,
    },
  },
  {
    // ... Next (the mouse): the models, the best match first and chosen,
    // each with its chip and its line for a student — mock-small fits
    // (16K for RAI), mock-chat takes no tools, mock-huge is too big, the
    // embedding model can't chat; its licence as reported, the note for
    // apps to sell; Check My Models' estimate
    name: "local-ai-models",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/local/v1|http://127.0.0.1:9/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(6)].join(","),
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    window: 2,
    dump: {
      "aisumodels.itemcount": /^4$/,
      "aisumodels.itemindex": /^0$/,
      "aisumodels.text": /^mock-small\. Fits\. Best match for this computer by its size: Check My Models to be sure\.\r?\n/,
      "aisumemtext.caption": /^This computer: 16 GB of memory; 1[01](\.\d)? GB free for a model now \(13 GB with other apps closed\)\.$/,
      "aisudetail.caption": /^mock-small · 1\.9 GB · 3\.2B parameters · Q4_K_M · takes tools · RAI gets a 16K context\.\nLicence, as its local AI app reports it: Mock Model Licence 1\.0\. Building apps to sell\? Check this model's licence\.$/,
      "aisuchecknote.caption": /^Optional, about \d+ minutes?: Studio tries each model that fits here/,
      "aisucheckbtn.enabled": /^(-1|1|True)$/i,
      "aisulicbtn.enabled": /^(-1|1|True)$/i,
    },
  },
  {
    // ... each row said in words (a screen reader reads the list's items):
    // the model with no tools
    name: "local-ai-rows",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/local/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), "aisumodels.__item_1", idle(2)].join(","),
    delay: 5,
    dump: {
      "aisumodels.text": /\nmock-chat\. Fits\. Can chat, but can't build apps with RAI\.\r?\n/,
      "aisudetail.caption": /no tools[\s\S]*Its local AI app doesn't say its licence\. Building apps to sell\? Check this model's licence\.$/,
      "aisulicbtn.enabled": /^(0|False)$/i,
    },
  },
  {
    // ... a computer too small (4 GB, 1 free): every model shown, none
    // picked — Finish says so and stays
    name: "local-ai-toobig",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/local/v1,ai:setupfacts=total=4096&free=1024,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), "aisumodels.__item_0", idle(2), CLICK("aisunextbtn", 30, 14), idle(2)].join(","),
    delay: 5,
    capture: true,
    window: 2,
    dump: {
      "aisetup.visible": /^(-1|1|True)$/i,
      "aisumodels.text": /^mock-small\. Too big for this computer\. Too big for this computer: it needs [\d.]+ GB; a model can have 1 GB here at most\.\r?\n/,
      "aisuchecknote.caption": /^mock-small is too big for this computer: choose another model\.$/,
      "aisucheckbtn.enabled": /^(0|False)$/i,
      "ai.provider": /^$/,
    },
  },
  {
    // ... none found (a closed port): what a local AI app is, how to add
    // one (the manual), another address — and one of another computer
    // refused before anything is asked
    name: "local-ai-none",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes=http://127.0.0.1:9/v1,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), typeInto("aisuaddredit", "192.168.1.20:11434"), CLICK("aisuaddrbtn", 30, 12), idle(4)].join(","),
    delay: 5,
    capture: true,
    window: 2,
    dump: {
      "aisufoundtext.caption": /^No local AI app found running on this computer: Next says what one is and how to add one\.$/,
      "aisumodels.visible": /^(0|False)$/i,
      "aisunone.visible": /^(-1|1|True)$/i,
      "aisunone.caption": /^No AI model answered on this computer\. A local AI app is a free program you install/,
      "aisunonelink.visible": /^(-1|1|True)$/i,
      "aisuchecknote.caption": /^Studio only asks this computer: 192\.168\.1\.20 isn't this computer/,
    },
  },
  {
    // ... Check My Models (the mouse): one model at a time against the
    // mock's local AI app — mock-small calls the tool and builds the Hello
    // app, so it is the best match "builds apps"; mock-chat writes its call
    // as text: it can chat, not build
    name: "local-ai-check",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/local/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), CLICK("aisucheckbtn", 30, 14), "aisucheckbtn.__until_Caption_&Check My Models", idle(4)].join(","),
    delay: 12,
    themes: LOOKS,
    capture: true,
    contrast: true,
    window: 2,
    dump: {
      "aisumodels.text": /^mock-small\. Fits\. Best match for this computer: builds apps, answers in about \d+ s\.\r?\nmock-chat\. Fits\. Can chat, but can't build apps with RAI: its tool calls didn't work here\.\r?\n/,
      "aisuchecknote.caption": /^Every model that fits is checked\. Choose one, then Finish\.$/,
      "aisucheckbtn.caption": /^&Check My Models$/,
    },
  },
  {
    // ... and Finish (the mouse): RAI on with that model, the context and
    // the mode its check chose (it built the app, fast, with tools: the
    // full mode, 16K); its pane shown
    name: "local-ai-finish",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/local/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), CLICK("aisucheckbtn", 30, 14), "aisucheckbtn.__until_Caption_&Check My Models", idle(2), CLICK("aisunextbtn", 30, 14), idle(4)].join(","),
    delay: 12,
    dump: {
      "aisetup.visible": /^(0|False)$/i,
      "ai.provider": /^compatible$/,
      "ai.model": /^mock-small$/,
      "ai.baseurl": /^http:\/\/127\.0\.0\.1:\d+\/s\/local\/v1$/,
      "ai.smallmodel": /^0$/,
      "ai.contextsize": /^16384$/,
      "aistatus.simpletext": /^RAI is ready: mock-small\.$/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
    },
  },
  {
    // ... a check stopped (Stop Checking): the model being checked is left
    // unchecked, said — the slow local AI app holds each answer 5 s
    name: "local-ai-cancel",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/localslow/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), CLICK("aisucheckbtn", 30, 14), idle(4), CLICK("aisucheckbtn", 30, 14), "aisucheckbtn.__until_Caption_&Check My Models", idle(2)].join(","),
    delay: 6,
    dump: {
      "aisuchecknote.caption": /^The check stopped\. 2 models aren't checked yet: Check My Models goes on with them/,
      "aisumodels.text": /^mock-small\. Fits\. Best match for this computer by its size: Check My Models to be sure\.\r?\n/,
      "aisucheckbtn.caption": /^&Check My Models$/,
    },
  },
  {
    // ... in Spanish: the page, the chips, the lines
    name: "local-ai-es",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: "ai:setupprobes={mockroot}/s/local/v1,ai:setupfacts=total=16384&free=12288,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(6)].join(","),
    delay: 6,
    capture: true,
    window: 2,
    dump: {
      "aisumodels.text": /^mock-small\. Cabe\. /,
      "aisucheckbtn.caption": /^&Comprobar mis modelos$/,
    },
  },
  {
    // ... the web: a browser can't look around this computer — the
    // address typed is asked (the mock's local AI app), its models listed
    // with what the browser can tell; no check there
    name: "local-ai-web",
    open: "examples/gui/hello_form.rr",
    webOnly: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:setupaddr={mockroot}/s/local/v1,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(6)].join(","),
    delay: 6,
    capture: true,
    window: 2,
    dump: {
      "aisumodels.itemcount": /^4$/,
      "aisumodels.text": /^mock-small\. Size not known\. /,
      "aisumemtext.caption": /^A browser can't tell how much memory this computer has free/,
      "aisucheckbtn.enabled": /^(0|False)$/i,
    },
  },
  {
    // ... while School Lock turns RAI off: nothing is looked for — the
    // mock's local AI app isn't asked anything
    name: "local-ai-locked",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "lock:set=maestra1,wait,ai:setupprobes={mockroot}/s/local/v1,ai:setup,wait",
    events: [idle(8)].join(","),
    delay: 5,
    mockLog: /^\[\]$/,
    dump: {
      "locksustrip.visible": /^1$/,
      "aisufoundtext.caption": /^$/,
      "aisunextbtn.enabled": /^0$/,
    },
  },
  {
    // (POLISH-2) …a server whose list says each model's window (the mock's
    // `tiny`: 8,192 tokens): known at Finish, without ↻ — the context
    // meter's limit is the model's
    name: "ai-r6-setup-local-window",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "ai:setupprobes={mockroot}/s/tiny/v1,ai:setup,wait",
    events: [idle(8), CLICK("aisunextbtn", 30, 14), idle(4), CLICK("aisunextbtn", 30, 14), idle(6)].join(","),
    delay: 6,
    dump: {
      "aisetup.visible": /^(0|False)$/i,
      "aistatus.simpletext": /^RAI is ready: mock-small\.$/,
      "ai.modelcount": /^[1-9]\d*$/,
      "ai.contextwindow": /^8192$/,
    },
  },
  {
    // R6: a cloud key — a refused one said plainly; another typed, Test
    // (the mouse): the models listed, the fast one first; Finish keeps the
    // key (the tests' memory store) and turns the assistant on
    name: "ai-r6-setup-cloud",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:setupprobes=http://127.0.0.1:9/v1|http://127.0.0.1:9/v1|http://127.0.0.1:9/v1,ai:setup",
    events: [SETUP_CLOUD, typeInto("aisukeyedit", "mock-bad-key-1"), CLICK("aisutestbtn", 30, 14), "aisucloud.__until_busy_0", idle(4), "aisukeyedit.__key_65_16", typeInto("aisukeyedit", "mock-good-key-1"), CLICK("aisutestbtn", 30, 14), "aisucloud.__until_busy_0", idle(4), CLICK("aisunextbtn", 30, 14), idle(4)].join(","),
    delay: 5,
    dump: {
      "ai.provider": /^anthropic$/,
      "ai.model": /^claude-mock-haiku-1$/,
      "ai.haskey": /^(-1|1|True)$/i,
      "aisetup.visible": /^(0|False)$/i,
      // (POLISH-2: the models asked for at Finish, as ↻)
      "ai.modelcount": /^[1-9]\d*$/,
    },
  },
  {
    // ... the refused key's words, before another is tried
    name: "ai-r6-setup-badkey",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:setupprobes=http://127.0.0.1:9/v1|http://127.0.0.1:9/v1|http://127.0.0.1:9/v1,ai:setup",
    events: [SETUP_CLOUD, typeInto("aisukeyedit", "mock-bad-key-1"), CLICK("aisutestbtn", 30, 14), "aisucloud.__until_busy_0", idle(6)].join(","),
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    window: 2,
    dump: {
      "aisutestnote.caption": /refused the API key \(401\)/,
      "aisutestbtn.enabled": /^(-1|1|True)$/i,
      "ai.provider": /^$/,
    },
  },
  {
    // K8: a model of an 8,192-token window (the mock's `tiny`) gets the
    // lite instructions and tools — it starts, and answers
    name: "ai-k8-lite",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mockroot}/s/tiny/v1,ai:model=mock-small,ai:ask=hello there,ai:wait,wait",
    delay: 5,
    dump: {
      "ai.lite": /^(-1|True)$/i,
      "ai.transcript": /^\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nhello there\n\n\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\nHello from the mock assistant!(?![\s\S]*takes 8,192 tokens)/,
      "aitools.lite": /^(1|-1|True)$/i,
    },
  },
  // ---- AI-UX (docs/ai-ux-plan.md §1–3): a new program straight into the
  // IDE, a banner to run it, Undo, the chat's cards ----
  {
    // Robert's chat (2026-10-18), a model naming the program "Scientific
    // Calculator": no question, no dialog — the project ScientificCalculator
    // made in Documents/RapidR/Projects (an install's), opened with its form
    // in the designer, the banner's Run it has the keys; Enter (a real key)
    // runs it. The chat: the cards, then the short reply.
    name: "ai-new-direct",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait",
    delay: 8,
    a11y: [["Run it", "button", "focused"], ["Undo", "button"], ["What did it do?", "button"], ["The assistant made this program for you.", "label"]],
    dump: {
      "proj.folder": /\/docs\/RapidR\/Projects\/ScientificCalculator$/,
      "proj.name": /^Scientific Calculator$/,
      "proj.mainfile": /^main\.rr$/,
      "designdoc(0).__shown": /^1$/,
      "aibanner.__shown": /^1$/,
      "aibannertext.caption": /^The assistant made this program for you\.$/,
      "aibannerrun.caption": /^▶  Run It$/,
      "aibannerfix.__shown": /^0$/,
      "aistatus.simpletext": /^Made Scientific Calculator: Run It is at the top\.$/,
      "ai.transcript": /\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\n(?:\[✓ \d+ steps? · show\]\(rai:steps\/\d+\)\n\n)?##### ✓ Started from the recipe \*Calculator \(basic and scientific\)\*\n\n##### ✓ Created \*Scientific Calculator\*  main\.rr · compiles  \[Open\]\(ai:open\/ScientificCalculator\) · \[Undo\]\(ai:undo\/ScientificCalculator\)\n\nPress \*\*Run\*\*/,
      "aireviewbar.__shown": /^0$/,
    },
    desktopFiles: ["docs/RapidR/Projects/ScientificCalculator/main.rr", "docs/RapidR/Projects/ScientificCalculator/ScientificCalculator.rrproj"],
    webDump: {
      "proj.folder": /ScientificCalculator$/,
      "proj.name": /^Scientific Calculator$/,
      "aibanner.__shown": /^1$/,
      "aibannerrun.caption": /^▶  Run It$/,
      "ai.transcript": /##### ✓ Created \*Scientific Calculator\*  main\.rr · compiles/,
    },
  },
  {
    // ... Enter on the banner (Run it has the keys): the program runs
    name: "ai-new-direct-run",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,key:Enter,wait,wait,wait,wait,wait",
    delay: 10,
    // (the desktop: the program's own process, which under the capture
    // test ends itself — exit code 0 — before Studio is read)
    dump: { "session.program": /ScientificCalculator\/main\.rr$/, "outputbox.text": /^(?![\s\S]*Can't run)/, "aibanner.__shown": /^1$/ },
    webDump: { "session.state": /^running$/, "aibanner.__shown": /^1$/ },
  },
  {
    // ... but Enter elsewhere — the designer clicked — never runs it: Run
    // It is the default button only while it has the keys
    name: "ai-new-direct-enter-elsewhere",
    readOnlyHome: true,
    installedWritable: true,
    desktopOnly: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait",
    // (the mouse in the designer, beside the form; then Enter)
    events: "designdoc(0).__mousedown_560_420,designdoc(0).__mouseup_560_420,designdoc(0).__key_13," + Array(20).fill("aibannertext.onclick").join(","),
    delay: 9,
    dump: { "session.program": /^$/, "aibannerrun.default": /^(0|False)$/i, "aibanner.__shown": /^1$/ },
  },
  {
    // ... the mouse on Run it: the same
    name: "ai-new-direct-click",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait",
    events: "aibannerrun.__mousedown_30_15,aibannerrun.__mouseup_30_15,aibannertext.onclick,aibannertext.onclick",
    delay: 8,
    // (the run started: its process running, the Output cleared for it)
    dump: { "session.program": /ScientificCalculator\/main\.rr$/, "outputbox.text": /^(?![\s\S]*Can't run)/ },
    desktopOnly: true,
  },
  {
    // ... Undo (the banner's, by the mouse): the program closed, the files
    // Studio wrote removed and its folder with them; Recent forgets it;
    // the chat says it was undone
    name: "ai-new-direct-undo",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,key:Tab,key:Enter,wait,wait",
    delay: 8,
    a11y: [[/^Welcome$/]],
    dump: {
      "proj.folder": /^$/,
      "aibanner.__shown": /^0$/,
      "aitools.createdfolder": /^$/,
      "aistatus.simpletext": /^Undone: Scientific Calculator was removed\.$/,
      "ai.transcript": /\*Undone: the new program \*Scientific Calculator\* was removed\.\*/,
      // (Recent forgets it: the Welcome page's list is empty again)
      "recentlink(0).__shown": /^0$/,
      "recentnone.__shown": /^1$/,
    },
    // (the folder gone; the projects folder it was in stays: not Studio's to remove)
    desktopGone: ["docs/RapidR/Projects/ScientificCalculator"],
    desktopFiles: ["docs/RapidR/Projects"],
    webDump: { "proj.folder": /^$/, "aibanner.__shown": /^0$/, "aitools.createdfolder": /^$/, "aistatus.simpletext": /^Undone: Scientific Calculator was removed\.$/ },
  },
  // ---- the chat's cards by the keyboard: Open and Undo are links in the
  // conversation (a markdown view), each a Tab stop in it — after the text,
  // before the box — with a ring round the link, a name for a screen reader,
  // Enter or Space to follow. From the box, Shift+Tab comes back to the
  // last link first.
  {
    // Shift+Tab, Shift+Tab: from the box to the last card's Undo, then its
    // Open; Tab: on to Undo again (the order in the chat)
    name: "ai-card-keys",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai.assistant,key:Shift+Tab,key:Shift+Tab,key:Tab",
    delay: 9,
    // (the ring round the link in each look, the text round it still AA)
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [["Undo", "link", "focused"], ["Open", "link"]],
    dump: { "aibanner.__shown": /^1$/, "ai.transcript": /\[Open\]\(ai:open\/ScientificCalculator\) · \[Undo\]\(ai:undo\/ScientificCalculator\)/ },
    webDump: { "aibanner.__shown": /^1$/ },
  },
  {
    // ... Tab once more leaves the chat for the box: its text box has the keys
    name: "ai-card-keys-leave",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai.assistant,key:Shift+Tab,key:Shift+Tab,key:Tab,key:Tab",
    delay: 9,
    a11y: [["Ask the assistant", "", "focused"], ["Undo", "link"]],
    dump: { "aibanner.__shown": /^1$/ },
    webDump: { "aibanner.__shown": /^1$/ },
  },
  {
    // ... Enter on Open opens the program again (its project was closed)
    name: "ai-card-keys-open",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,file.closeProject,wait,ai.assistant,key:Shift+Tab,key:Shift+Tab,key:Enter,wait,wait,wait",
    delay: 10,
    dump: { "proj.folder": /\/docs\/RapidR\/Projects\/ScientificCalculator$/, "proj.name": /^Scientific Calculator$/, "designdoc(0).__shown": /^1$/ },
    webDump: { "proj.folder": /ScientificCalculator$/, "proj.name": /^Scientific Calculator$/ },
  },
  {
    // ... Enter on Undo takes the program back, as the mouse's click does
    name: "ai-card-keys-undo",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai.assistant,key:Shift+Tab,key:Enter,wait,wait",
    delay: 9,
    dump: {
      "proj.folder": /^$/,
      "aibanner.__shown": /^0$/,
      "aitools.createdfolder": /^$/,
      "aistatus.simpletext": /^Undone: Scientific Calculator was removed\.$/,
    },
    desktopGone: ["docs/RapidR/Projects/ScientificCalculator"],
    webDump: { "proj.folder": /^$/, "aibanner.__shown": /^0$/, "aitools.createdfolder": /^$/, "aistatus.simpletext": /^Undone: Scientific Calculator was removed\.$/ },
  },
  {
    // The banner's "What Did It Do?": the Assistant pane, the chat scrolled
    // to the turn's first card (the chat was scrolled back to its top
    // first, the card out of sight) and the keyboard on the chat. (Robert's
    // walk, 2026-10-18: it did nothing.)
    name: "ai-banner-what",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please. " + LONG_ASK + ",wait,ai:wait,wait,wait,wait,ai.assistant,key:Shift+Tab,key:Home,wait",
    events: "aibannerwhat.onclick",
    delay: 9,
    a11y: [["Conversation with the assistant", "document", "focused"], [/^Started from the recipe/, "heading", "", SHOWN_IN_CHAT]],
    dump: { "aibanner.__shown": /^1$/ },
    webDump: { "aibanner.__shown": /^1$/ },
  },
  {
    // An edit applied that leaves the program with an error (a property the
    // form doesn't have): the status says so — Problems has it — , not only
    // that a file changed
    name: "ai-edit-errors",
    open: "examples/gui/hello_form.rr",
    copy: true,
    webFiles: ["examples/gui/hello_form.rr"],
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=break the window,wait,wait,wait,wait,ai:apply,wait",
    delay: 7,
    dump: {
      "codedoc(0).text": /DIM ready AS BOOLEAN\nCREATE Form AS RForm/,
      "aistatus.simpletext": /^1 file changed: Edit > Undo takes it back\. The program has 1 error: Problems lists them\.$/,
    },
  },
  {
    // ... and under the banner of a program the assistant made: the banner
    // goes from "made this program for you" to the errors variant (Run
    // Anyway, Ask It to Fix) — Robert's walk, 2026-10-18
    name: "ai-banner-edit-errors",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai.newChat,ai:ask=break the window,wait,wait,wait,wait,ai:apply,wait",
    delay: 12,
    a11y: [["Ask the assistant to fix the errors", "button"], ["Run anyway", "button"]],
    dump: {
      "aibanner.__shown": /^1$/,
      "aibannertext.caption": /^The assistant made this program — it still has 1 error\.$/,
      "aibannerrun.caption": /^▶  Run Anyway$/,
      "aibannerfix.__shown": /^1$/,
      "aistatus.simpletext": /^1 file changed: Edit > Undo takes it back\. The program has 1 error: Problems lists them\.$/,
    },
    webDump: { "aibannertext.caption": /still has 1 error/, "aibannerfix.__shown": /^1$/ },
  },
  {
    // ... a file the user added to its folder since: Undo asks first;
    // Cancel keeps everything
    name: "ai-new-direct-undo-asks",
    readOnlyHome: true,
    installedWritable: true,
    desktopOnly: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai:touch=notes.txt,wait,ai:undo",
    events: "promptcancel.__mousedown_12_12,promptcancel.__mouseup_12_12",
    delay: 8,
    dump: {
      "prompttitle.caption": /^Undo the new program Scientific Calculator\?$/,
      "prompttext.caption": /notes\.txt was added[\s\S]*files you added stay/,
      "promptsave.caption": /^&Undo Anyway$/,
      "saveprompt.visible": /^0$/,
      "proj.folder": /ScientificCalculator$/,
      "aitools.createdfolder": /ScientificCalculator$/,
    },
    desktopFiles: ["docs/RapidR/Projects/ScientificCalculator/main.rr", "docs/RapidR/Projects/ScientificCalculator/notes.txt"],
  },
  {
    // ... Undo Anyway: the assistant's files go, the user's file stays
    // (and its folder with it)
    name: "ai-new-direct-undo-anyway",
    readOnlyHome: true,
    installedWritable: true,
    desktopOnly: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,ai:touch=notes.txt,wait,ai:undo",
    events: "promptsave.__key_13",
    delay: 8,
    dump: {
      "proj.folder": /^$/,
      "aistatus.simpletext": /^Undone: the assistant's files of Scientific Calculator were removed; the files you added stay in .*ScientificCalculator\.$/,
    },
    desktopFiles: ["docs/RapidR/Projects/ScientificCalculator/notes.txt"],
    desktopGone: ["docs/RapidR/Projects/ScientificCalculator/main.rr", "docs/RapidR/Projects/ScientificCalculator/ScientificCalculator.rrproj"],
  },
  {
    // A program that still doesn't compile after the assistant's tries:
    // opened all the same, the banner says so — Run anyway, Ask it to fix
    name: "ai-new-direct-errors",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=build me a broken tool,wait,ai:wait,wait,wait,wait",
    delay: 8,
    a11y: [["Run anyway", "button", "focused"], ["Ask the assistant to fix the errors", "button"]],
    dump: {
      "proj.folder": /BrokenTool$/,
      "aibanner.__shown": /^1$/,
      "aibannertext.caption": /^The assistant made this program — it still has 1 error\.$/,
      "aibannerrun.caption": /^▶  Run Anyway$/,
      "aibannerfix.__shown": /^1$/,
      "ai.transcript": /##### ✓ Created \*Broken Tool\*  main\.rr · 1 error left  \[Open\]/,
    },
    webDump: { "aibannertext.caption": /^The assistant made this program — it still has 1 error\.$/, "aibannerfix.__shown": /^1$/ },
  },
  {
    // The model made the program, then the server failed on its last words
    // (a local server short of memory, Robert's walk): the program is made
    // all the same — opened, the banner on top; the chat says the error
    name: "ai-new-direct-after-error",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=then fail Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait,wait,wait",
    delay: 10,
    dump: {
      "proj.folder": /ScientificCalculator$/,
      "aibanner.__shown": /^1$/,
      "ai.transcript": /##### ✓ Created \*Scientific Calculator\*[\s\S]*> \*Error: /,
    },
  },
  {
    // ... Ask It to Fix (the mouse): the assistant is asked, in the chat,
    // with the program open — its files are the project's now
    name: "ai-new-direct-fix",
    readOnlyHome: true,
    installedWritable: true,
    desktopOnly: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=build me a broken tool,wait,ai:wait,wait,wait,wait",
    events: "aibannerfix.__mousedown_30_15,aibannerfix.__mouseup_30_15," + Array(30).fill("aibannertext.onclick").join(","),
    delay: 8,
    dump: {
      "ai.transcript": /\*\*You\*\*(?: · \[Edit\]\(rai:edit\/\d+\))?\n\nPlease fix the errors that are left\./,
      "ai.lastrequest": /The open project: Broken Tool; main file main\.rr/,
    },
  },
  {
    // A reply that claimed the program before any tool made it: Studio
    // asked the model to make it; the chat shows one muted line for that
    // draft, never the claim — then the card and the true reply
    name: "ai-claim-collapsed",
    readOnlyHome: true,
    installedWritable: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=pretend you made a hello program,wait,ai:wait,wait,wait,wait",
    delay: 8,
    dump: {
      "ai.transcript": /\*\*Assistant\*\*(?: · \[Retry\]\(rai:retry\))?\n\n(?:\[✓ \d+ steps? · show\]\(rai:steps\/\d+\)\n\n)?> \*An earlier draft said this was done before it was — Studio asked it to actually do it\.\*\n\n##### ✓ Created \*hello-form\*  main\.rr · compiles[^\n]*\n\nType a name, then press \*\*Greet\*\*\.$/,
      "aichat.plaintext": /^(?![\s\S]*I have created)/,
      "aibanner.__shown": /^1$/,
    },
    webDump: { "ai.transcript": /> \*An earlier draft said this was done before it was[\s\S]*##### ✓ Created \*hello-form\*/, "aichat.plaintext": /^(?![\s\S]*I have created)/ },
  },
  {
    // While it works (a slow model: the mock holds its next answer 6 s):
    // the progress checklist under the chat — what the tools did, ✓, and
    // what goes on now —, its newest line the one status, announced
    name: "ai-progress",
    readOnlyHome: true,
    installedWritable: true,
    capture: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=slowly Build a scientific calculator with graphics please.,wait,wait,wait,wait",
    delay: 4,
    dump: {
      "ai.busy": /^(-1|1|True)$/i,
      "ai.steps": /^Wrote the program$/,
      "aistatus.simpletext": /^Thinking… · ✓ Wrote the program$/,
      "ai.transcript": /##### ✓ New program \*Scientific Calculator\*  main\.rr · compiles/,
      "aibanner.__shown": /^0$/,
    },
  },
  {
    // The banner, the chat's cards and the one status in each look: every
    // text against its ground (WCAG AA), captured for the manual
    name: "ai-banner",
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    readOnlyHome: true,
    installedWritable: true,
    capture: true,
    contrast: true,
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait",
    delay: 8,
    dump: { "aibanner.__shown": /^1$/, "aibanner.top": /^4\d$/ },
  },
  {
    // Read only: the proposing tools aren't offered, a call to one is
    // refused, nothing is proposed; a path outside the project is refused
    name: "ai-read-only",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:permission=read-only,ai:ask=inject a file,wait,wait,wait",
    delay: 5,
    dump: {
      "aitools.proposalcount": /^0$/,
      "ai.steps": /^write_file: not allowed \(read-only\)$/,
      "ai.lastrequest": /^(?![\s\S]*"name": "write_file",\n\s*"description")[\s\S]*The user allowed reading only/,
    },
  },
  {
    // Anthropic's wire format (the mock speaks it too), a made-up key kept
    // for the session and never shown; the registry described, the forms
    // listed through the introspection interface
    name: "ai-anthropic-format",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=anthropic,ai:key=sk" + "-ant-test-FAKE-0000-wxyz,ai:model=claude-mock-haiku-1,ai:ask=describe the form,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "ai.steps": /^Looked up RButton\nLooked at the form$/,
      "ai.transcript": /RButton is RButton \(RapidQ: QButton\)[\s\S]*the form has 5 components/,
      "ai.lastrequest": /^POST http:\/\/127\.0\.0\.1:\d+\/anthropic\/v1\/messages\n[\s\S]*x-api-key: \(your API key, not shown\)\nanthropic-version: 2023-06-01\n(?![\s\S]*FAKE-0000)/,
      "ai.keyhint": /^…wxyz$/,
      "ai.usage": /^2,000 tokens sent \(200 from the cache\), 40 received$/,
    },
  },
  {
    // Fetch Models; a missing model explained in plain words
    name: "ai-models-and-errors",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:fetch,wait,wait,ai:model=mock-missing,ai:ask=hello,wait,wait",
    delay: 5,
    dump: {
      // (no model chosen for the user; the fast, low-cost ones first, by the
      // words of their names)
      "ai.models": /^mock-small\tMock Small\tfast · low cost\nmock-large\t\t$/,
      "ai.model": /^mock-missing$/,
      "ai.error": /^The model "mock-missing" isn't available to your key \(404\)/,
      "aistatus.simpletext": /isn't available to your key/,
    },
  },
  {
    // An RAI on a form, live in the designer: no models until it has a
    // key; the key typed goes to the keychain (here the test's memory
    // store) and the code says only where it is; then the provider's own
    // models in the Model row, the fast, low-cost ones first; one picked.
    // (ai:designwait waits for the list, not a fixed time; the capture
    // comes after the last step even when the page starts slowly — 13
    // steps 400 ms apart outlasted the old 5 s on a cold web start, and the
    // Model pick seemed not to stick)
    name: "ai-designer-rai",
    open: "tests/fixtures/studio_ai/rai_form.rr",
    webFiles: ["tests/fixtures/studio_ai/rai_form.rr"],
    do: "wait,view.designer,pick:AI,prop:Provider=anthropic,prop:BaseURL={mock},row:Model,prop:APIKey=sk" + "-ant-fake-designer-0000,ai:designwait,row:Model,prop:Model=mock-small,wait",
    delay: 8,
    dump: {
      "inspector.target": /^AI$/,
      "inspector.selected": /^Model$/,
      "inspector.doc": /^2 models from Anthropic \(Claude\), fast, low-cost ones first: pick one\./,
      "inspector.rows": /^Model=mock-small$/m,
      // (the key never in the code; where it is, and "ask" the program's
      // user wherever the keychain doesn't have it)
      "codedoc(0).text": /^(?![\s\S]*sk-fake)[\s\S]*APIKeySource = "keychain:RapidR\/rai_form\/AI\|ask"[\s\S]*Model = "mock-small"/,
      "designai.haskey": /^(1|-1|True)$/i,
    },
  },
  {
    // ... and before it has a key: nothing fetched, the row says why
    name: "ai-designer-rai-no-key",
    open: "tests/fixtures/studio_ai/rai_form.rr",
    webFiles: ["tests/fixtures/studio_ai/rai_form.rr"],
    do: "wait,view.designer,pick:AI,prop:Provider=anthropic,prop:BaseURL={mock},row:Model,wait",
    delay: 3,
    dump: {
      "inspector.doc": /^Type the API key in APIKey \(kept in your keychain, never in the code\): the provider's models are listed here then\./,
      "designai.modelcount": /^0$/,
      "codedoc(0).text": /^(?![\s\S]*APIKey)/,
    },
  },
  // ---- AI-REAL: Tools > Settings > AI used as a person uses it (Robert's
  // Preview 2 findings, 2026-10-16) — the dialog opened from its menu
  // command, then clicks and keys through the kernel's input (the mouse on
  // the tab, the Fetch button, OK, Cancel; a fake key typed key by key), the
  // cloud providers played by the mock (RAPIDR_AI_TEST_SERVER): what the
  // user sees — the list's items, the button enabled again, the note —
  // read back. A real walk of the same paths on the screen is in the
  // lane's report; these keep them from coming back.
  {
    // A refused key (401): said in plain words under the list, all of it;
    // Fetch Models can be pressed again at once (it stayed greyed until
    // Studio was restarted)
    name: "ai-settings-401",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_1", typeInto("aikeyedit", "mock-bad-key-1"), FETCH, idle(40)].join(","),
    delay: 4,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aiprovbox.text": /^OpenAI$/,
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "aifetchnote.caption": /^OpenAI refused the API key \(401\) \(OpenAI says: "Incorrect API key provided[\s\S]*Check the key: it may be mistyped, revoked, or a key of another provider\.$/,
      // (the note as tall as its four lines: nothing cut)
      "aifetchnote.height": /^(6\d|7\d|8\d)$/,
      "aimodelcombo.itemcount": /^0$/,
    },
  },
  {
    // ... then another key typed over it and Fetch Models again: the list
    // comes, the fast, low-cost ones first — the dropdown has the items
    name: "ai-settings-retry",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_1", typeInto("aikeyedit", "mock-bad-key-1"), FETCH, idle(40), "aikeyedit.__key_65_16", typeInto("aikeyedit", "mock-good-key-1"), FETCH, idle(40), "aimodelcombo.__item_0"].join(","),
    delay: 4,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "aimodelcombo.itemcount": /^3$/,
      "aimodelcombo.text": /^gpt-mock-mini {3}fast · low cost$/,
      "aifetchnote.caption": /^OpenAI lists 3 models\./,
      // (the assistant itself untouched until OK)
      "ai.provider": /^$/,
      "ai.model": /^$/,
    },
  },
  {
    // A model picked, OK: the assistant has it (its pane shows, Send
    // enabled), and the dialog opened again shows it chosen, the list kept
    name: "ai-settings-ok-reopen",
    open: "examples/gui/hello_form.rr",
    do: "view.settings,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,view.settings",
    events: [AI_TAB, "aionbox.__mousedown_8_10,aionbox.__mouseup_8_10", "aiprovbox.__item_1", typeInto("aikeyedit", "mock-good-key-1"), FETCH, idle(40), "aimodelcombo.__item_0", OK, idle(160), AI_TAB, idle(8)].join(","),
    delay: 4,
    dump: {
      "settingsdialog.visible": /^(-1|1|True)$/i,
      "aiprovbox.text": /^OpenAI$/,
      "aimodelcombo.text": /^gpt-mock-mini {3}fast · low cost$/,
      "aimodelcombo.itemcount": /^3$/,
      "ai.provider": /^openai$/,
      "ai.model": /^gpt-mock-mini$/,
      "aisendbtn.enabled": /^(-1|1|True)$/i,
      "aimodelbox.itemcount": /^3$/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
    },
  },
  {
    // ... and kept: Studio started again, Settings shows the provider and
    // the model chosen (the key was a test's, in memory: gone with it)
    name: "ai-settings-saved",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aionbox.__mousedown_8_10,aionbox.__mouseup_8_10", "aiprovbox.__item_1", typeInto("aikeyedit", "mock-good-key-1"), FETCH, idle(40), "aimodelcombo.__item_1", OK, idle(20)].join(","),
    delay: 4,
    // (the second run: only the dialog opened, nothing typed or clicked)
    restart: { do: "view.settings", delay: 4, events: "" },
    dump: {
      "settingsdialog.visible": /^(-1|1|True)$/i,
      "aiprovbox.text": /^OpenAI$/,
      "aimodelcombo.text": /^gpt-mock-nano$/,
      "aionbox.checked": /^(-1|1|True)$/i,
    },
  },
  {
    // Cancel leaves the assistant exactly as it was: another provider
    // browsed, a key typed and tried, a list fetched — none of it reaches
    // the assistant (it used to switch it, clearing its key and models)
    name: "ai-settings-cancel",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=openai,ai:key=mock-good-key-1,ai:fetch,wait,wait,ai:model=gpt-mock-mini,wait,view.settings",
    events: [AI_TAB, "aiprovbox.__item_2", typeInto("aikeyedit", "mock-good-key-2"), FETCH, idle(40), "aimodelcombo.__item_0", CANCEL, idle(8)].join(","),
    delay: 5,
    dump: {
      "settingsdialog.visible": /^(0|False)$/i,
      "ai.provider": /^openai$/,
      "ai.model": /^gpt-mock-mini$/,
      "ai.modelcount": /^3$/,
      "ai.haskey": /^(-1|1|True)$/i,
      // (R6: no provider row in the pane — the model list says whose)
      "aimodelbox.accessibledescription": /^A model of OpenAI\. /,
      "aimodelbox.text": /^gpt-mock-mini$/,
      "aisendbtn.enabled": /^(-1|1|True)$/i,
    },
  },
  {
    // A provider that doesn't answer: Fetch Models waits greyed; another
    // provider picked meanwhile gives it back at once (the old request is
    // dropped), as closing the dialog does
    name: "ai-settings-slow-switch",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_2", typeInto("aikeyedit", "mock-slow-key-1"), FETCH, idle(20), "aiprovbox.__item_1", idle(8)].join(","),
    delay: 4,
    dump: {
      "aiprovbox.text": /^OpenAI$/,
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "setai.busy": /^(0|False)$/i,
      "aifetchnote.caption": /^$/,
    },
  },
  {
    // ... or the dialog closed while it waits (Cancel): opened again,
    // Fetch Models is ready, nothing of the silent provider kept
    name: "ai-settings-slow-close",
    open: "examples/gui/hello_form.rr",
    do: "view.settings,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,wait,view.settings",
    events: [AI_TAB, "aiprovbox.__item_2", typeInto("aikeyedit", "mock-slow-key-1"), FETCH, idle(20), CANCEL, idle(160), AI_TAB, idle(8)].join(","),
    delay: 4,
    dump: {
      "settingsdialog.visible": /^(-1|1|True)$/i,
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "setai.busy": /^(0|False)$/i,
      "aifetchnote.caption": /^$/,
      "ai.provider": /^$/,
    },
  },
  {
    // A server that isn't there (nothing listens at the address typed): said
    // in words, Fetch Models back at once
    name: "ai-settings-unreachable",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_3", "aibasebox.__mousedown_60_12,aibasebox.__mouseup_60_12,aibasebox.__key_65_16", typeInto("aibasebox", "http://127.0.0.1:9/v1"), FETCH, idle(60)].join(","),
    delay: 4,
    dump: {
      "aibasebox.text": /^http:\/\/127\.0\.0\.1:9\/v1$/,
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "aifetchnote.caption": /^Couldn't reach 127\.0\.0\.1:9: [\s\S]*Is the local server running/,
    },
  },
  {
    // ... and left waiting: after 30 seconds RapidR gives up and says so;
    // the button is back
    name: "ai-settings-timeout",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_2", typeInto("aikeyedit", "mock-slow-key-1"), FETCH, "aionnote.__hold_36000", idle(8)].join(","),
    delay: 4,
    desktopOnly: true,
    dump: {
      "aifetchbtn.enabled": /^(-1|1|True)$/i,
      "aifetchnote.caption": /^Couldn't reach 127\.0\.0\.1:\d+: no answer came in 30 seconds\./,
    },
  },
  {
    // (the web: a browser's fetch has no time limit of its own — RapidR's:
    // the page is watched until the button comes back)
    name: "ai-settings-timeout-web",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_2", typeInto("aikeyedit", "mock-slow-key-1"), FETCH, idle(8)].join(","),
    delay: 4,
    webOnly: true,
    dump: { "aifetchbtn.enabled": /^(0|False)$/i, "setai.busy": /^(-1|1|True)$/i },
    web: async (page, scale, record) => {
      const prop = (c, p) => page.evaluate(([c, p]) => window.rr.rapidr_get_prop(c, p), [c, p]);
      const t0 = Date.now();
      let enabled = "";
      while (Date.now() - t0 < 60000) {
        enabled = String(await prop("aifetchbtn", "enabled"));
        if (/^(-1|1|true)$/i.test(enabled)) break;
        await page.waitForTimeout(1000);
      }
      const note = String(await prop("aifetchnote", "caption"));
      record("", /^(-1|1|true)$/i.test(enabled) && /no answer came in \d+ seconds/.test(note), `after ${Math.round((Date.now() - t0) / 1000)} s Fetch Models is back and says why ("${note}")`);
    },
  },
  {
    // An Anthropic key of an organisation, not of a workspace: Anthropic
    // wants to know which workspace — said, the Workspace ID box ready;
    // typed, the list comes, the header sent
    name: "ai-settings-workspace",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_0", typeInto("aikeyedit", "mock-org-key-1"), FETCH, idle(40), typeInto("aiwsedit", "wrk_flow_01"), FETCH, idle(40)].join(","),
    delay: 4,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "aiprovbox.text": /^Anthropic \(Claude\)$/,
      "aiwsedit.text": /^wrk_flow_01$/,
      "aimodelcombo.itemcount": /^3$/,
      "aifetchnote.caption": /^Anthropic lists 3 models\./,
    },
    mockLog: /"path":"\/anthropic\/v1\/models","key":"…ey-1","workspace":"","[\s\S]*"path":"\/anthropic\/v1\/models","key":"…ey-1","workspace":"wrk_flow_01"/,
  },
  {
    // Each provider's rows, and only those: Anthropic's workspace,
    // OpenAI's and DeepSeek's none (no empty gap where a row is hidden),
    // a compatible server's address
    name: "ai-settings-rows",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    events: [AI_TAB, "aiprovbox.__item_1", idle(4)].join(","),
    delay: 4,
    dump: {
      "aiwsedit.__shown": /^0$/,
      "aibasebox.__shown": /^0$/,
      // (the Model row a row's step under the key's note)
      "aimodelcombo.top": /^22\d$/,
    },
    // (the web: the key's note a line longer, and Remember on this device)
    webDump: {
      "aiwsedit.__shown": /^0$/,
      "aibasebox.__shown": /^0$/,
      "aimodelcombo.top": /^2[5-7]\d$/,
      "airemember.__shown": /^1$/,
    },
  },
  // ---- COPY-ANYWHERE: copying from the Output pane (tests/fixtures/studio_copy) ----
  // (Output's lines from y 30 down, 20 pixels each in Studio's monospaced
  // font; "second line of output"'s "line of" from x 62 to 117 — its
  // characters 7 to 14 — on line 2, y 60; "third line" at y 80)
  {
    // a drag over the middle of line 2 selects "line of"; Ctrl+C (⌘C)
    // copies exactly that (the tests' own clipboard); the selection read
    // in every look (contrast judged, the selected text with it)
    name: "copy-output-drag",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "run.start,wait,wait,wait",
    events: [COPY_WAIT, COPY_DRAG, "outputbox.__key_67_16", "outputbox.__wait_300"].join(","),
    delay: 7,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "outputbox.seltext": /^line of$/, "outputbox.hasselection": /^1$/, "clipboard.text": /^line of$/, "outputbox.text": /^first line\nsecond line of output\nthird line\n/ },
    // (the web: the kernel's Ctrl+C fills the page's copy event — the real
    // ⌘C in Chrome below; here the selection)
    webDump: { "outputbox.seltext": /^line of$/, "outputbox.hasselection": /^1$/ },
  },
  {
    // the right-click menu over "third line", "line of" selected: Copy
    // (enabled), Copy Line, Copy All, Select All, —, Wrap Long Lines ✓,
    // Clear — captured open, light and dark
    name: "copy-output-menu",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    do: "run.start,wait,wait,wait",
    events: [COPY_WAIT, COPY_DRAG, "outputbox.__rightclick_100_80", "outputbox.__wait_300"].join(","),
    delay: 7,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    dump: {
      "outputbox.clickedline": /^2$/,
      "outputbox.seltext": /^line of$/,
      "outputmenucopy.enabled": /^1$/, "outputmenucopy.caption": /^&Copy$/,
      "outputmenucopyline.enabled": /^1$/, "outputmenucopyline.visible": /^(1|-1)$/, "outputmenucopyline.caption": /^Copy &Line$/,
      "outputmenucopyall.caption": /^Copy &All$/, "outputmenuselall.caption": /^&Select All$/,
      "outputmenuwrap.checked": /^(1|-1)$/, "outputmenuclear.caption": /^Cl&ear$/,
    },
  },
  {
    // the menu's Copy (Down, Enter): the selection on the clipboard, said
    // on the status bar
    name: "copy-output-menu-copy",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "run.start,wait,wait,wait",
    events: [COPY_WAIT, COPY_DRAG, "outputbox.__rightclick_100_80", "outputbox.__key_40", "outputbox.__key_13", "outputbox.__wait_300"].join(","),
    delay: 7,
    dump: { "clipboard.text": /^line of$/, "status.panel(0).caption": /^Copied 7 characters$/ },
  },
  {
    // nothing selected: Copy disabled; Copy Line (Down past Copy, Down,
    // Enter) copies the line right-clicked
    name: "copy-output-menu-line",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "run.start,wait,wait,wait",
    events: [COPY_WAIT, "outputbox.__rightclick_100_80", "outputbox.__key_40", "outputbox.__key_40", "outputbox.__key_13", "outputbox.__wait_300"].join(","),
    delay: 7,
    dump: { "outputbox.hasselection": /^0$/, "outputmenucopy.enabled": /^0$/, "clipboard.text": /^third line$/, "status.panel(0).caption": /^Copied 10 characters$/ },
  },
  {
    // Copy All: the whole page, its last line Studio's own
    name: "copy-output-menu-all",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "run.start,wait,wait,wait",
    events: [COPY_WAIT, "outputbox.__rightclick_100_80", "outputbox.__key_40", "outputbox.__key_40", "outputbox.__key_40", "outputbox.__key_13", "outputbox.__wait_300"].join(","),
    delay: 7,
    dump: { "clipboard.text": /^first line\nsecond line of output\nthird line\n\[three\.rr ended, exit code 0\]$/, "status.panel(0).caption": /^Copied 4 lines$/ },
  },
  {
    // Edit > Copy and Select All while Output has the keyboard (no
    // document needed): Select All, Copy — every line
    name: "copy-output-edit-menu",
    open: "tests/fixtures/studio_copy/three.rr",
    webFiles: ["tests/fixtures/studio_copy/three.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "run.start,wait,wait,wait,focus:outputbox,edit.selectAll,edit.copy",
    delay: 7,
    dump: { "clipboard.text": /^first line\nsecond line of output\nthird line\n\[three\.rr ended, exit code 0\]\n> focus:outputbox\n> edit\.selectAll$/, "status.panel(0).caption": /^Copied 6 lines$/ },
  },
  {
    // the Problems page: an error's row clicked, the next added with
    // Shift+Down, Ctrl+C — each as `file:line:col: severity: message`; the menu's Copy
    // All says Problems
    name: "copy-problems",
    open: "tests/fixtures/studio_copy/problems.rr",
    webFiles: ["tests/fixtures/studio_copy/problems.rr"],
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "wait,run.start,wait,wait",
    events: [COPY_WAIT, "outputbox.__mousedown_300_60", "outputbox.__mouseup_300_60", "outputbox.__wait_300", "outputbox.__key_40_256", "outputbox.__key_67_16", "outputbox.__rightclick_300_60", "outputbox.__wait_300"].join(","),
    delay: 8,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    dump: {
      "outputbox.page": /^problems$/,
      "clipboard.text": /^problems\.rr:8:1: error: Unknown SUB or FUNCTION 'Missing'[^\n]*\nproblems\.rr:9:1: error: Unknown SUB or FUNCTION 'Other'[^\n]*$/,
      "outputmenucopy.enabled": /^1$/, "outputmenucopyline.visible": /^0$/, "outputmenucopyall.caption": /^Copy &All Problems$/,
    },
    webDump: { "outputbox.page": /^problems$/, "outputbox.seltext": /^problems\.rr:8:1: error: [^\n]*\nproblems\.rr:9:1: error: [^\n]*$/, "outputmenucopyall.caption": /^Copy &All Problems$/ },
  },
  // (VERSIONS-G3) Versions in Studio, walked as a person would (docs/
  // versions-plan.md; the acceptance in the lane's brief): a copy of a small
  // project (tests/fixtures/studio_versions) — Start Versioning, Save a
  // Version "first", main.rr changed: the gutter's marks, the Project
  // pane's M, the status bar's "1 change not saved as a version"; the
  // change shown under its line; Save a Version "second" through the
  // dialog; the timeline; What Changed? of "first"; Go Back to "first"
  // (asked; a third version "Went back to “first”"); Undo. A key in a file:
  // the dialog says so, saves only once ticked. A project never versioned
  // keeps a local history of every save. The web: no versions (said), the
  // local history works.
  {
    name: "versions-empty",
    ...V_OPEN,
    do: "versions.pane",
    delay: 4,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "versub.caption": /^No versions yet$/, "verstartbtn.visible": /^1$/, "verlocalbtn.visible": /^1$/, "repo.versioned": NO },
    webDump: { "versub.caption": /^Not in a browser$/, "verstartbtn.visible": /^0$/, "verlocalbtn.visible": /^1$/, "repo.available": NO },
  },
  {
    name: "versions-first",
    ...V_OPEN,
    desktopOnly: true,
    do: V_FIRST,
    delay: 5,
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [["Timeline: every version, newest first", null], [/^first, saved by Ana, just now$/, null]],
    dump: { "repo.versioned": YES, "vertimeline.itemcount": /^1$/, "versub.caption": /^main · all saved$/, "repo.experiment": /^main$/ },
    desktopFiles: ["project/.gitignore", "project/.git/HEAD"],
  },
  {
    name: "versions-changed",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},view.project,versions:report=main.rr`,
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [["main.rr", "treeitem", null, (n) => /changed since the last version/.test(n.description || "")]],
    dump: {
      "codedoc(0).changessince": /^version$/, "codedoc(0).changecount": /^1$/, "repo.changecount": /^1$/,
      "outputbox.text": /\[versions\] versioned=1 changes=1 timeline=1 experiment=main badge=M status=Hello {2}· {2}main · 1 change not saved as a version/,
    },
  },
  {
    name: "versions-peek",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},line:12,versions.showChange`,
    delay: 6,
    themes: LOOKS,
    capture: true,
    dump: { "codedoc(0).changepeek": /^12$/, "codedoc(0).changecount": /^1$/ },
  },
  {
    // (Undo This Change: the line as the last version has it, one Ctrl+Z step)
    name: "versions-undo-change",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},line:12,versions.undoChange`,
    delay: 6,
    dump: { "codedoc(0).changecount": /^0$/, "codedoc(0).text": /PRINT "Done\."$/, "codedoc(0).modified": YES },
  },
  {
    name: "versions-save-dialog",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},versions.save`,
    events: typeInto("vermsgedit", "second"),
    delay: 6,
    themes: LOOKS,
    capture: true,
    captureAll: true,
    contrast: true,
    dump: { "versavedialog.__shown": /^1$/, "vermsgedit.text": /^second$/, "verfileslist.itemcount": /^1$/, "versaveok.enabled": /^1$/, "vernameedit.text": /^Ana$/ },
  },
  {
    name: "versions-save-second",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},versions.save,wait,versions.pane`,
    events: `${typeInto("vermsgedit", "second")},${CLICK("versaveok", 40, 14)}`,
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "versavedialog.__shown": /^0$/, "vertimeline.itemcount": /^2$/, "repo.changecount": /^0$/, "outputbox.text": /Saved the version “second”/ },
  },
  {
    // (a key written in the code: not saved, the line said — masked —, Save
    // only once "save anyway" is ticked)
    name: "versions-secrets",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_SECRET},versions.save`,
    events: `${typeInto("vermsgedit", "keys")},${CLICK("versaveok", 40, 14)}`,
    delay: 6,
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    capture: true,
    captureAll: true,
    dump: { "verwarn.visible": /^1$/, "veranyway.visible": /^1$/, "versaveok.enabled": /^0$/, "verwarnlines.caption": /^main\.rr, line 12: an API key[\s\S]*sk-a/, "vertimeline.itemcount": /^1$/ },
  },
  {
    name: "versions-secrets-anyway",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_SECRET},versions.save`,
    events: `${typeInto("vermsgedit", "keys")},${CLICK("versaveok", 40, 14)},${CLICK("veranyway", 8, 10)},${CLICK("versaveok", 40, 14)}`,
    delay: 6,
    dump: { "versavedialog.__shown": /^0$/, "vertimeline.itemcount": /^2$/, "outputbox.text": /Saved the version “keys”/ },
  },
  {
    // ("first" picked: what it changed — every file new)
    name: "versions-what-changed-first",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions:pick=first,versions.whatChanged`,
    delay: 6,
    dump: { "verdifflist.itemcount": /^4$/, "verdiffhead.caption": /^What changed in “first”$/, "verdiff.readonly": YES },
  },
  {
    name: "versions-what-changed",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions:pick=second,versions.whatChanged`,
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "verdifflist.itemcount": /^1$/, "verdiffhead.caption": /^What changed in “second”$/, "verdiff.changecount": /^1$/, "verdiff.summary": /changed/ },
  },
  {
    // (Go Back to "first": asked first — Cancel the default —, then a third
    // version with first's files; main.rr back in the editor)
    name: "versions-go-back",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions:pick=first,versions.goBack,wait,versions.pane`,
    events: `${CLICK("consentyes", 40, 14)},verpanel.__wait_2500`,
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "vertimeline.itemcount": /^3$/, "outputbox.text": /Went back to “first”: a new version/, "codedoc(0).text": /PRINT "Done\."$/, "verundonote.caption": /^Undo: Went back to “first”/ },
    files: { "main.rr": /PRINT "Done\."\n$/ },
  },
  {
    name: "versions-go-back-asks",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions:pick=first,versions.goBack`,
    delay: 5,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    captureAll: true,
    dump: { "consentdialog.__shown": /^1$/, "consenttitle.caption": /^Go back to “first”\?$/, "consentno.default": YES },
  },
  {
    name: "versions-undo",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions:pick=first,versions.goBack,wait,versions.undo,wait,versions.pane`,
    events: `${CLICK("consentyes", 40, 14)},verpanel.__wait_2500`,
    delay: 7,
    capture: true,
    dump: { "vertimeline.itemcount": /^2$/, "outputbox.text": /Undid: Went back to “first”/, "codedoc(0).text": /PRINT "All done!"$/ },
    files: { "main.rr": /PRINT "All done!"\n$/ },
  },
  {
    // (a project never versioned: every save kept — what the disk had, then
    // each save —, the newest first, one compared with the editor)
    name: "versions-local-history",
    ...V_OPEN,
    do: 'code:PRINT "Done."=>PRINT "One",file.save,code:PRINT "One"=>PRINT "Two",file.save,code:PRINT "Two"=>PRINT "Three",file.save,versions.localHistory',
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "verdifflist.itemcount": /^4$/, "repo.versioned": NO, "verdiffhead.caption": /^Local History: main\.rr$/, "verdiff.changecount": /^0$/ },
    webDump: { "verdifflist.itemcount": /^[34]$/, "repo.versioned": NO, "verdiffhead.caption": /^Local History: main\.rr$/ },
  },
  {
    // (an AI's checkpoint on the timeline: smaller and lighter, said as one)
    name: "versions-checkpoint",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_FIRST},${V_EDIT},versions:checkpoint=make the button bigger,versions:save=second,versions.pane`,
    delay: 6,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    a11y: [[/^AI checkpoint: make the button bigger, just now$/, null]],
    dump: { "vertimeline.itemcount": /^3$/ },
  },
  {
    // (keyboard only: View > Versions gives the pane the keys; Down picks
    // "first" in the timeline, Tab reaches What Changed?, Space presses it)
    name: "versions-keyboard",
    ...V_OPEN,
    desktopOnly: true,
    do: `${V_SECOND},versions.pane,wait,key:Down,wait,key:Tab,wait,key:Space,wait`,
    delay: 7,
    a11y: [[/^What changed in “first”$|^Files that changed$/, null]],
    dump: { "vertimeline.itemindex": /^1$/, "verdiffhead.caption": /^What changed in “first”$/ },
  },
  {
    // (the oldest copy picked — the file as it was before the first save —
    // and Restore This Copy: its text in the editor, unsaved, one Ctrl+Z
    // step; the editor's text kept first)
    name: "versions-local-restore",
    ...V_OPEN,
    do: 'code:PRINT "Done."=>PRINT "One",file.save,code:PRINT "One"=>PRINT "Two",file.save,versions.localHistory',
    events: `verdifflist.__item_2,${CLICK("verdiffact", 40, 12)},verdiffpane.__wait_1500`,
    delay: 6,
    dump: { "codedoc(0).text": /PRINT "Done\."$/, "codedoc(0).modified": YES, "verdiff.changecount": /^0$/ },
  },
  // (EXTERNAL-CHANGES) Files changed outside Studio — another AI agent, an
  // editor, git — walked as a person sees it (the lane's acceptance): a
  // copy of tests/fixtures/studio_external outside this repository, its
  // files changed by another process (tests/ext_writer.mjs) between Studio's
  // steps. main.rr changed with nothing unsaved: loaded again, the status
  // bar says so, Ctrl+Z brings the old text back, the local history has a
  // "before reload" copy — on a look (ext:check, as Studio coming to the
  // front) and by the two-second timer. With unsaved changes: the strip
  // above the editor (Compare, Use Disk Version, Keep Mine); Save after Keep
  // Mine asks first, Cancel writing nothing. greet.rr deleted: its tab
  // marked, its text kept. Twelve files at once: one note. Studio's own
  // Save: no notice. The web: no disk to watch, nothing said.
  {
    name: "ext-reload",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_CHANGE},ext:check,wait,ext:report`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: {
      "codedoc(0).text": /PRINT "Changed by an agent\."$/, "codedoc(0).modified": NO, "extbar.visible": /^0$/,
      "outputbox.text": /\[ext\] status=main\.rr was updated outside Studio bar= \| main\.rr:main\.rr:dirty=0:ask=0:gone=0[\s\S]*history=\d+\s+\d+\s+before-reload\s+\d+/,
    },
  },
  {
    // (Ctrl+Z: the text before the reload, one step — marked changed, as it
    // differs from the disk's)
    name: "ext-reload-undo",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_CHANGE},ext:check,wait,focus:codedoc(0),key:Ctrl+Z,wait,wait`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 6,
    dump: { "codedoc(0).text": /PRINT "Done\."$/, "codedoc(0).modified": YES },
  },
  {
    // (no look asked for: the two-second timer finds it)
    name: "ext-timer",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_CHANGE},wait,wait,wait,wait,wait,wait,wait,ext:report`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 7,
    dump: { "codedoc(0).text": /PRINT "Changed by an agent\."$/, "outputbox.text": /main\.rr was updated outside Studio/ },
  },
  {
    // (typed in, not saved, then changed on disk: the question above the
    // editor; the editor's text untouched)
    name: "ext-ask",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,ext:report`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [[/^main\.rr changed outside Studio, and you have unsaved changes in it: Compare, Use Disk Version or Keep Mine\.$/, null], ["Compare", "button"], ["Use Disk Version", "button"], ["Keep Mine", "button"]],
    dump: {
      "extbar.visible": /^1$/, "extbartext.caption": /^main\.rr changed outside Studio\.$/, "codedoc(0).text": /PRINT "Mine\."$/, "codedoc(0).modified": YES,
      "outputbox.text": /bar=ask \| main\.rr:main\.rr:dirty=1:ask=1:gone=0/,
    },
  },
  {
    // (in Spanish: the strip, its buttons, the status bar)
    name: "ext-ask-es",
    ...X_OPEN,
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: `${X_MINE},${X_CHANGE},ext:check,wait`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 6,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    contrast: true,
    a11y: [[/^main\.rr cambió fuera de Studio, y tienes cambios sin guardar en él: Comparar, Usar la versión del disco o Conservar la mía\.$/, null], ["Comparar", "button"], ["Usar la versión del disco", "button"], ["Conservar la mía", "button"]],
    dump: { "extbar.visible": /^1$/, "extbartext.caption": /^main\.rr cambió fuera de Studio\.$/, "extdisk.caption": /^&Usar la versión del disco$/ },
  },
  {
    name: "ext-many-es",
    ...X_OPEN,
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: `ext:openall,wait,${X_CHANGE},ext:check,wait`,
    external: [{ write: X_TWELVE }],
    delay: 9,
    themes: ["rapidr-light"],
    capture: true,
    dump: { "extbartext.caption": /^12 archivos se actualizaron fuera de Studio\.$/, "extshow.caption": /^&Mostrar$/ },
  },
  {
    name: "ext-keep-mine-asks-es",
    ...X_OPEN,
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.keepMine,wait,file.save`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 7,
    themes: ["rapidr-light"],
    capture: true,
    captureAll: true,
    dump: { "consenttitle.caption": /^¿Guardar main\.rr de todos modos\?$/, "consentyes.caption": /^&Guardar de todos modos$/, "consentno.default": YES },
  },
  {
    // (Compare: the disk's version beside the editor's)
    name: "ext-compare",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.compareDisk`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 6,
    themes: LOOKS,
    capture: true,
    contrast: true,
    dump: { "verdiffhead.caption": /^main\.rr: the disk's version and yours$/, "verdifflist.itemcount": /^1$/, "verdiff.changecount": /^1$/, "verdiffact.caption": /^&Use Disk Version$/ },
  },
  {
    // (Use Disk Version, the strip's button clicked: the disk's text, saved;
    // the editor's kept in the local history; Ctrl+Z brings it back)
    name: "ext-use-disk",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait`,
    external: [{ write: { "main.rr": X_AGENT } }],
    events: `extbar.__until_visible_1,${CLICK("extdisk", 40, 12)},codedoc(0).__wait_1000`,
    delay: 6,
    dump: { "codedoc(0).text": /PRINT "Changed by an agent\."$/, "codedoc(0).modified": NO, "extbar.visible": /^0$/, "outputbox.text": /main\.rr changed outside Studio, and you have unsaved changes in it/ },
  },
  {
    name: "ext-use-disk-undo",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.useDisk,wait,focus:codedoc(0),key:Ctrl+Z,wait,wait,ext:report`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 7,
    dump: { "codedoc(0).text": /PRINT "Mine\."$/, "codedoc(0).modified": YES, "outputbox.text": /history=\d+\s+\d+\s+before-reload\s+\d+/ },
  },
  {
    // (Keep Mine, then Ctrl+S: asked — Cancel the default — before the
    // disk's newer text is replaced)
    name: "ext-keep-mine-asks",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.keepMine,wait,focus:codedoc(0),key:Ctrl+S`,
    external: [{ write: { "main.rr": X_AGENT } }],
    delay: 7,
    themes: LOOKS,
    capture: true,
    captureAll: true,
    dump: { "consentdialog.__shown": /^1$/, "consenttitle.caption": /^Save main\.rr anyway\?$/, "consentlead.caption": /^main\.rr changed outside Studio since you opened it\. Saving replaces/,"consentno.default": YES, "consentyes.caption": /^&Save Anyway$/, "extbar.visible": /^0$/ },
    files: { "main.rr": /PRINT "Changed by an agent\."\n$/ },
  },
  {
    name: "ext-keep-mine-cancel",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.keepMine,wait,file.save`,
    external: [{ write: { "main.rr": X_AGENT } }],
    events: `${CLICK("consentno", 40, 14)},codedoc(0).__wait_1000`,
    delay: 7,
    dump: { "consentdialog.__shown": /^0$/, "codedoc(0).modified": YES, "codedoc(0).text": /PRINT "Mine\."$/ },
    files: { "main.rr": /PRINT "Changed by an agent\."\n$/ },
  },
  {
    // (Save Anyway: written; the disk's version kept in the local history first)
    name: "ext-keep-mine-save",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},${X_CHANGE},ext:check,wait,file.keepMine,wait,file.save,wait,ext:report`,
    external: [{ write: { "main.rr": X_AGENT } }],
    events: `${CLICK("consentyes", 40, 14)},codedoc(0).__wait_1500`,
    delay: 7,
    dump: { "codedoc(0).modified": NO, "outputbox.text": /history=\d+\s+\d+\s+saved\s+\d+;\d+\s+\d+\s+on-disk\s+\d+/ },
    files: { "main.rr": /PRINT "Mine\."\n?$/ },
  },
  {
    // (greet.rr deleted outside: its tab stays, marked, its text kept)
    name: "ext-deleted",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:open=greet.rr,wait,${X_CHANGE},ext:check,wait,ext:report`,
    external: [{ remove: ["greet.rr"] }],
    delay: 7,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    a11y: [["greet.rr (deleted on disk)", "tab"]],
    dump: { "codedoc(1).text": /FUNCTION Greeting/, "outputbox.text": /greet\.rr was deleted outside Studio: its text is kept here[\s\S]*greet\.rr:greet\.rr \(deleted on disk\):dirty=1:ask=0:gone=1 \| history=\d+\s+\d+\s+deleted-outside\s+\d+/ },
  },
  {
    // (deleted, then there again as Studio has it — a branch switched and
    // back: no mark, nothing to save)
    name: "ext-deleted-back",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:open=greet.rr,wait,${X_CHANGE},ext:check,wait,ext:go=2,ext:await=2,ext:check,wait,ext:report`,
    external: [{ remove: ["greet.rr"] }, { write: { "greet.rr": readFileSync(join(ROOT, X_DIR, "greet.rr"), "utf8") } }],
    delay: 8,
    dump: { "codedoc(1).modified": NO, "outputbox.text": /greet\.rr:greet\.rr:dirty=0:ask=0:gone=0/ },
  },
  {
    // (Save on a file deleted outside: asked — Save Again —, then made again)
    name: "ext-deleted-save",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:open=greet.rr,wait,${X_CHANGE},ext:check,wait,file.save,wait,ext:report`,
    external: [{ remove: ["greet.rr"] }],
    events: `consenttitle.__until_caption_Save greet.rr again?,${CLICK("consentyes", 40, 14)},codedoc(1).__wait_1000`,
    delay: 7,
    dump: { "outputbox.text": /greet\.rr:greet\.rr:dirty=0:ask=0:gone=0/ },
    files: { "greet.rr": /FUNCTION Greeting/ },
  },
  {
    // (a form's file changed outside with nothing unsaved: the designer
    // shows the new form)
    name: "ext-form-reload",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:open=window.rr,wait,wait,${X_CHANGE},ext:check,wait,wait,ext:report`,
    external: [{ write: { "window.rr": X_WINDOW } }],
    delay: 7,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    dump: { "codedoc(1).text": /Caption = "Changed outside"[\s\S]*Left = 140/, "extbar.visible": /^0$/, "outputbox.text": /window\.rr was updated outside Studio/ },
  },
  {
    // (changed in the designer, not saved, then outside: the question above
    // the designer too)
    name: "ext-form-ask",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:open=window.rr,wait,wait,pick:Hello,prop:Caption=Mine,wait,${X_CHANGE},ext:check,wait,ext:report`,
    external: [{ write: { "window.rr": X_WINDOW } }],
    delay: 7,
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    capture: true,
    contrast: true,
    dump: { "extbar.visible": /^1$/, "extbartext.caption": /^window\.rr changed outside Studio\.$/, "codedoc(1).text": /Caption = "Mine"/, "outputbox.text": /window\.rr:window\.rr:dirty=1:ask=1:gone=0/ },
  },
  {
    // (twelve open files rewritten at once — git checkout, an agent: one
    // note, Show lists them in the Output)
    name: "ext-many",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:openall,wait,${X_CHANGE},ext:check,wait,ext:report`,
    external: [{ write: X_TWELVE }],
    delay: 9,
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [[/^12 files were updated outside Studio\.$/, null], ["Show", "button"]],
    dump: { "extbar.visible": /^1$/, "extbartext.caption": /^12 files were updated outside Studio\.$/, "outputbox.text": /\[ext\] status=12 files were updated outside Studio bar=note/ },
  },
  {
    name: "ext-many-show",
    ...X_OPEN,
    desktopOnly: true,
    do: `ext:openall,wait,${X_CHANGE},ext:check,wait`,
    external: [{ write: X_TWELVE }],
    events: `extbar.__until_visible_1,${CLICK("extshow", 30, 12)},outputbox.__wait_800`,
    delay: 9,
    dump: { "extbar.visible": /^0$/, "outputbox.text": /12 files were updated outside Studio:\n {2}greet\.rr\n {2}main\.rr\n {2}part1\.rr[\s\S]* {2}window\.rr/ },
  },
  {
    // (Studio's own writes: Save, then a look — nothing said)
    name: "ext-own-save",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_MINE},file.save,wait,ext:check,wait,ext:report`,
    delay: 6,
    dump: { "extbar.visible": /^0$/, "outputbox.text": /\[ext\] status=(?![^\n]*outside)[^\n]*bar= \| main\.rr:main\.rr:dirty=0:ask=0:gone=0/ },
    files: { "main.rr": /PRINT "Mine\."\n?$/ },
  },
  {
    // (RAI's change applied, then saved: Studio's own — nothing said)
    name: "ext-own-ai-apply",
    open: "examples/gui/hello_form.rr",
    copy: true,
    desktopOnly: true,
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=rename the window,wait,wait,wait,wait,ai:apply,wait,file.save,wait,ext:check,wait,ext:report",
    delay: 9,
    dump: { "codedoc(0).text": /Caption = "AI Notes"/, "extbar.visible": /^0$/, "outputbox.text": /\[ext\] status=(?![^\n]*outside)[^\n]*bar= \| hello_form\.rr:hello_form\.rr:dirty=0:ask=0:gone=0/ },
    files: { "hello_form.rr": /Caption = "AI Notes"/ },
  },
  {
    // (the project file rewritten outside — a file added by an agent: the
    // Project pane follows quietly)
    name: "ext-project-file",
    ...X_OPEN,
    desktopOnly: true,
    do: `${X_CHANGE},ext:check,wait,view.project`,
    external: [{ write: { "notes.rr": "' notes.rr: added by an agent.\n", "Outside.rrproj": readFileSync(join(ROOT, "tests/fixtures/studio_external/Outside.rrproj"), "utf8") + '\n[[files]]\npath = "notes.rr"\nkind = "module"\n' } }],
    delay: 6,
    dump: { "proj.filecount": /^13$/, "outputbox.text": /Outside\.rrproj was updated outside Studio: the Project pane follows it/ },
  },
  {
    // (File > Reopen Project: the project read again from the disk)
    name: "ext-reopen",
    ...X_OPEN,
    desktopOnly: true,
    do: "file.reopenProject,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /Opened Outside\.rrproj again from the disk/, "codedoc(0).text": /PRINT "Done\."$/ },
  },
  {
    // (the web: files in the page's store change only through Studio — a
    // look finds nothing, says nothing)
    name: "ext-web",
    open: "tests/fixtures/studio_versions/Hello.rrproj",
    webFiles: VERSIONS_FILES,
    webOnly: true,
    do: "ext:check,wait,ext:report",
    delay: 4,
    dump: {},
    webDump: { "extbar.visible": /^0$/, "outputbox.text": /\[ext\] status=[^\n]*bar= \| main\.rr:main\.rr:dirty=0:ask=0:gone=0/ },
  },  // ---- SCHOOL-LOCK (Preview 5): a teacher turns Studio's AI off behind a
  // password — ide/schoollock.inc, rapidr-studio's school_lock.rs, the
  // manual's "For teachers". The password "maestra1" (a test's); the
  // computer's policy is tests/fixtures/school_lock (RAPIDR_SCHOOL_POLICY:
  // never the real /Library, %ProgramData% or /etc file). Each password
  // costs 600,000 PBKDF2 rounds: the delays leave room for a busy machine.
  // (A dialog's events fire at the end of the delay, the dump at once: no
  // step runs after them.) ----
  {
    // Tools > School Lock…, the password typed twice with the keyboard,
    // Enter: locked — the status bar's chip, RAI's card, said
    name: "school-lock-set",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "tools.schoolLock",
    events: [typeInto("lockpwedit", "maestra1"), typeInto("lockpw2edit", "maestra1"), "lockpw2edit.__key_13"].join(","),
    delay: 6,
    capture: true,
    a11y: [[/^School Lock is on: RAI and AI apps are off in RapidR Studio on this computer\.$/]],
    dump: {
      "lockdialog.visible": /^(0|False)$/i,
      "lockcard.visible": /^1$/,
      "lockchipbox.visible": /^1$/,
      "lockchiptext.caption": /^School Lock$/,
      "outputbox.text": /> tools\.schoolLock\nSchool Lock is on: RAI and AI apps are off in RapidR Studio on this computer\.$/,
    },
  },
  {
    // Locked: RAI's pane shows the teacher's card (View > AI Assistant),
    // the AI commands disabled with the reason in the menu and the palette,
    // AI apps off — in the three looks, every text readable
    name: "school-lock-card",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,ai.assistant,wait,lock:state",
    delay: 8,
    capture: true,
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    contrast: true,
    a11y: [[/^School Lock is on: RAI and AI apps are off in RapidR Studio on this computer\.$/], [/^AI is turned off by your teacher$/], [/^Unlock\.\.\.$/, "button"]],
    dump: {
      "lockcard.visible": /^1$/,
      "lockcardtitle.caption": /^AI is turned off by your teacher$/,
      "lockcardbody.caption": /^RAI and AI apps are off in RapidR Studio on this computer\. Everything else works as usual\.$/,
      "lockcardbtn.caption": /^&Unlock\.\.\.$/,
      "aichat.visible": /^0$/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
      "outputbox.text": /School Lock state: on=1 unlocked=0 policy=0 features=ai,ai_apps ai=1 ai_apps=1 wait=0 left=5 mcp=0\nSchool Lock commands: newchat palette=0 menu=0 hint=turned off by your teacher/,
    },
  },
  {
    // Escape is Cancel: nothing locked, nothing kept
    name: "school-lock-cancel",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "tools.schoolLock",
    events: [typeInto("lockpwedit", "maestra1"), "lockpwedit.__key_27"].join(","),
    delay: 4,
    dump: {
      "lockdialog.visible": /^(0|False)$/i,
      "lockchipbox.visible": /^0$/,
      "outputbox.text": /> tools\.schoolLock$/,
    },
  },
  {
    // Two passwords that differ: said under the field, nothing locked
    name: "school-lock-set-refused",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "tools.schoolLock",
    events: [typeInto("lockpwedit", "maestra1"), typeInto("lockpw2edit", "maestra2"), "lockpw2edit.__key_13"].join(","),
    delay: 4,
    dump: {
      "lockhint.caption": /^The two passwords differ: type the same one twice\.$/,
      "lockchipbox.visible": /^0$/,
    },
  },
  {
    // Settings > AI read-only: the strip with Unlock…, every control greyed
    name: "school-lock-settings",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,wait,ai.settings",
    delay: 7,
    capture: true,
    captureAll: true,
    themes: ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"],
    contrast: true,
    dump: {
      "locksetstrip.visible": /^1$/,
      "locksethead.caption": /^AI is turned off by your teacher\.$/,
      "locksetbody.caption": /^These settings can't be changed while School Lock is on\.$/,
      "aionbox.enabled": /^0$/,
      "aisetupbtn.enabled": /^0$/,
      "aiprovbox.enabled": /^0$/,
      "aikeyedit.enabled": /^0$/,
      "aifetchbtn.enabled": /^0$/,
      "aionnote.visible": /^0$/,
    },
  },
  {
    // Settings > AI Apps: the switch off and greyed, the rest read-only;
    // the server never started (the tests' token stands in for the
    // keychain's)
    name: "school-lock-apps",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_TEST_MCP_TOKEN: "school-lock-test-token-0123456789abcdef" },
    do: "lock:set=maestra1,mcp:on=47395,lock:state,mcp.settings",
    delay: 8,
    capture: true,
    captureAll: true,
    dump: {
      "locksetstrip.visible": /^1$/,
      "mcponbox.enabled": /^0$/,
      "mcponbox.checked": /^0$/,
      "mcpresetbtn.enabled": /^0$/,
      "outputbox.text": /AI apps can't start: School Lock is on \(Tools > School Lock\.\.\.\)\.[\s\S]*School Lock state: on=1 [^\n]* ai_apps=1 [^\n]* mcp=0/,
    },
  },
  {
    // AI apps on, then locked: the server stops (every AI app
    // disconnected); unlocked, it starts again as the user had it
    name: "school-lock-apps-running",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_TEST_MCP_TOKEN: "school-lock-test-token-0123456789abcdef" },
    do: "mcp:on=47396,lock:state,lock:set=maestra1,lock:state,lock:unlock=maestra1,lock:state",
    delay: 10,
    dump: {
      "outputbox.text": /mcp=1\n[\s\S]*AI apps: stopped, School Lock is on\. Every AI app was disconnected\.[\s\S]*School Lock state: on=1 unlocked=0 [^\n]* mcp=0[\s\S]*AI apps can connect at http:\/\/127\.0\.0\.1:47396\/mcp[\s\S]*School Lock state: on=1 unlocked=1 [^\n]* mcp=1/,
    },
  },
  {
    // Set Up RAI read-only: the strip on its first page, nothing looked for
    name: "school-lock-setup",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,wait,ai.setup",
    delay: 7,
    capture: true,
    captureAll: true,
    dump: {
      "locksustrip.visible": /^1$/,
      "locksubody.caption": /^RAI can't be set up while School Lock is on\.$/,
      "aisulocal.enabled": /^0$/,
      "aisucloudradio.enabled": /^0$/,
      "aisunextbtn.enabled": /^0$/,
      "aisufoundtext.caption": /^$/,
    },
  },
  {
    // RAI set up and asked once; locked: its pane keeps its place, the card
    // over the chat, a question isn't sent
    name: "school-lock-ai-ready",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello there,ai:wait,wait,lock:set=maestra1,wait,ai:ask=hello again,wait`,
    delay: 10,
    capture: true,
    dump: {
      "lockcard.visible": /^1$/,
      "aichat.visible": /^0$/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
      "ai.questioncount": /^1$/,
      "aistatus.simpletext": /^AI is turned off by your teacher$/,
    },
  },
  {
    // …and unlocked: the chat is back as it was
    name: "school-lock-ai-unlocked",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: `${AI_READY},ai:ask=hello there,ai:wait,wait,lock:set=maestra1,wait,lock:unlock=maestra1,wait`,
    delay: 10,
    dump: {
      "lockcard.visible": /^0$/,
      "aichat.visible": /^1$/,
      "aitop.visible": /^1$/,
      "ai.questioncount": /^1$/,
      "lockchiptext.caption": /^School Lock: unlocked$/,
    },
  },
  {
    // Unlock… > five wrong passwords (Enter each time): the tries left said,
    // then a minute's wait counted down — the right one isn't even checked
    name: "school-lock-wrong",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,tools.schoolLock",
    events: [1, 2, 3, 4, 5].map((k) => `${typeInto("lockpwedit", `nope${k}`)},lockpwedit.__key_13`).concat([typeInto("lockpwedit", "maestra1"), "lockpwedit.__key_13"]).join(","),
    delay: 7,
    capture: true,
    captureAll: true,
    dump: {
      "lockhint.caption": /^Too many wrong passwords\. Try again in (0:5\d|1:00)\.$/,
      "lockmainbtn.enabled": /^0$/,
      "lockoffbtn.enabled": /^0$/,
      "lockdialog.visible": /^(1|-1|True)$/i,
      "lockchiptext.caption": /^School Lock$/,
    },
  },
  {
    // One wrong (the tries left said), then the right one with the
    // keyboard: unlocked until Studio closes — the chip says so
    name: "school-lock-unlock",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,lock:unlock=wrong-one,tools.schoolLock",
    events: [typeInto("lockpwedit", "maestra1"), "lockpwedit.__key_13"].join(","),
    delay: 7,
    capture: true,
    a11y: [[/^School Lock is unlocked until Studio closes$/]],
    dump: {
      "lockdialog.visible": /^(0|False)$/i,
      "lockchiptext.caption": /^School Lock: unlocked$/,
      "lockcard.visible": /^0$/,
      "outputbox.text": /School Lock answered: 0 — That isn't the password\. 4 more tries before a one-minute wait\.[\s\S]*> tools\.schoolLock\nSchool Lock is unlocked until Studio closes\.$/,
    },
  },
  {
    // Unlocked, Studio closed and started again: locked again
    name: "school-lock-restart",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "lock:set=maestra1,lock:unlock=maestra1,lock:state",
    delay: 7,
    restart: { do: "lock:state", delay: 5 },
    dump: {
      "lockchiptext.caption": /^School Lock$/,
      "outputbox.text": /School Lock state: on=1 unlocked=0 policy=0 features=ai,ai_apps ai=1 ai_apps=1/,
    },
  },
  {
    // The wrong tries are kept: five, Studio closed and started again —
    // still waiting
    name: "school-lock-wait-kept",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "lock:set=maestra1,lock:unlock=a1b2c3,lock:unlock=a1b2c4,lock:unlock=a1b2c5,lock:unlock=a1b2c6,lock:unlock=a1b2c7,lock:state",
    delay: 12,
    restart: { do: "lock:unlock=maestra1,lock:state", delay: 6 },
    dump: {
      "outputbox.text": /School Lock answered: -1 — Too many wrong passwords\. Try again in (0:[1-5]\d|1:00)\.\n[\s\S]*School Lock state: on=1 unlocked=0 [^\n]* wait=[1-6]\d? left=5/,
    },
  },
  {
    // Turn School Lock Off: the password again (a wrong one refused); off
    // for good — kept so (the next start has no lock)
    name: "school-lock-off",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    do: "lock:set=maestra1,lock:off=maestra2,lock:off=maestra1,lock:state",
    delay: 8,
    restart: { do: "lock:state", delay: 5 },
    dump: {
      "lockchipbox.visible": /^0$/,
      "outputbox.text": /School Lock state: on=0 unlocked=0 policy=0 features= ai=0 ai_apps=0/,
    },
  },
  {
    // (the same, on both hosts: what Turn Off says)
    name: "school-lock-off-said",
    open: "examples/gui/hello_form.rr",
    webFiles: ["examples/gui/hello_form.rr"],
    do: "lock:set=maestra1,lock:off=maestra2,lock:off=maestra1,lock:state",
    delay: 8,
    dump: {
      "lockchipbox.visible": /^0$/,
      "outputbox.text": /School Lock answered: 0 — That isn't the password\.[\s\S]*School Lock is off\.\nSchool Lock answered: 1 — \n[\s\S]*School Lock state: on=0/,
    },
  },
  {
    // The computer's policy (IT's file): locked from the start, the
    // school's words; Turn Off refused and not offered — only unlocked for
    // this run, with IT's password
    name: "school-lock-policy",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_SCHOOL_POLICY: join(ROOT, "tests/fixtures/school_lock/policy.toml") },
    do: "lock:state,lock:off=it-secret-1,ai.assistant,wait,tools.schoolLock",
    delay: 8,
    capture: true,
    captureAll: true,
    dump: {
      "lockcardtitle.caption": /^AI is turned off by your school$/,
      "lockoffbtn.visible": /^0$/,
      "lockfoot.caption": /^Your school's IT set this for every account on this computer\. It can be unlocked until Studio closes, but only IT can turn it off\.$/,
      "outputbox.text": /School Lock state: on=1 unlocked=0 policy=1 features=ai,ai_apps ai=1 ai_apps=1[\s\S]*School Lock answered: -2 — Your school's IT set this lock: only IT can turn it off\./,
    },
  },
  {
    // …unlocked with IT's password until Studio closes (the dialog then
    // offers only Lock Again); started again: locked again
    name: "school-lock-policy-unlock",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_SCHOOL_POLICY: join(ROOT, "tests/fixtures/school_lock/policy.toml") },
    do: "lock:unlock=it-secret-1,lock:state,tools.schoolLock",
    delay: 8,
    capture: true,
    captureAll: true,
    dump: {
      "lockmainbtn.caption": /^Lock &Again$/,
      "lockoffbtn.visible": /^0$/,
      "lockpwedit.visible": /^0$/,
      "outputbox.text": /School Lock state: on=1 unlocked=1 policy=1 [^\n]* ai=0 ai_apps=0/,
    },
  },
  {
    name: "school-lock-policy-restart",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_SCHOOL_POLICY: join(ROOT, "tests/fixtures/school_lock/policy.toml") },
    do: "lock:unlock=it-secret-1,lock:state",
    delay: 6,
    restart: { do: "lock:state", delay: 5 },
    dump: {
      "outputbox.text": /School Lock state: on=1 unlocked=0 policy=1 features=ai,ai_apps ai=1 ai_apps=1/,
    },
  },
  {
    // A policy file that can't be read: everything locked, said once, no
    // password unlocks it
    name: "school-lock-policy-broken",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    env: { RAPIDR_SCHOOL_POLICY: join(ROOT, "tests/fixtures/school_lock/broken.toml") },
    do: "lock:unlock=it-secret-1,lock:state",
    delay: 6,
    dump: {
      "outputbox.text": /School Lock: this computer's policy \([^)]*broken\.toml\) can't be read: it isn't TOML[^\n]*AI stays off until your school's IT fixes it\.[\s\S]*School Lock answered: -3 — No password can unlock this lock: ask your school's IT\.[\s\S]*School Lock state: on=1 unlocked=0 policy=1 features=ai,ai_apps ai=1 ai_apps=1/,
    },
  },
  {
    // A crash while School Lock is on (crash.inc's CrashSchoolLock): at the
    // next start no crash strip, no Report on GitHub — a GitHub account is
    // for 13 and over
    name: "school-lock-crash",
    desktopOnly: true,
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "lock:set=maestra1,wait,crash:panic",
    delay: 6,
    restart: { noOpen: true, do: "wait,wait,wait,crash:check,crash:report,lock:state", delay: 9 },
    dump: {
      "crashbar.visible": /^0$/,
      "crashgitb.__shown": /^0$/,
      "lockchiptext.caption": /^School Lock$/,
      "outputbox.text": /\[crash\] bar=0 [\s\S]*School Lock state: on=1 unlocked=0/,
    },
  },
  {
    // …and once a teacher turned it off, the strip and Report on GitHub
    // are offered again (the same crash)
    name: "school-lock-crash-off",
    desktopOnly: true,
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    do: "lock:set=maestra1,lock:off=maestra1,wait,crash:panic",
    delay: 7,
    restart: { noOpen: true, do: "wait,wait,wait,crash:report", delay: 6 },
    dump: {
      "crashbar.visible": /^1$/,
      "crashgitb.__shown": /^1$/,
      "outputbox.text": /\[crash\] bar=1 /,
    },
  },
  {
    // In Spanish: the card, the chip and the dialog
    name: "school-lock-es",
    open: "examples/gui/hello_form.rr",
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: "lock:set=maestra1,ai.assistant,wait,tools.schoolLock",
    delay: 8,
    capture: true,
    captureAll: true,
    themes: ["rapidr-light", "rapidr-dark"],
    dump: {
      "lockcardtitle.caption": /^Tu docente desactivó la IA$/,
      "lockchiptext.caption": /^Bloqueo escolar$/,
      "lockmainbtn.caption": /^&Desbloquear hasta cerrar Studio$/,
    },
  },
  // (CRASH-REPORTS) RapidR's crash and hang reports, walked as a person
  // meets them: Studio crashed for real (crash:panic, a Rust panic in a
  // component's call — the process ends, 101), started again on the same
  // folders — the strip under the tool bar says it closed unexpectedly,
  // with View Report, Copy Report, Save Report..., Report on GitHub... and
  // Dismiss, read by a screen reader; any answer and it isn't offered
  // again. The report is the case's (RAPIDR_STUDIO_DATA/crashes), never
  // the user's; Report on GitHub opens nothing in a test (the address is
  // said instead). A UI hang (crash:hang: Studio's thread asleep 8 s)
  // writes a hang report within 6 s and, answered again, is no crash at
  // the next start. A program RapidR crashes in its timer's handler:
  // "Your program stopped: …, at main.rr line 24, in SUB Tick (OnTimer of
  // Ticker)." with its report. The desktop only (the web writes none yet).
  {
    name: "crash-banner",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:report", delay: 6 },
    themes: LOOKS,
    capture: true,
    contrast: true,
    a11y: [[/^RapidR Studio closed unexpectedly last time\. A report of what happened stays on this computer; nothing was sent\./, null], ["View Report", "button"], ["Copy Report", "button"], ["Save Report...", "button"], ["Report on GitHub...", "button"], ["Dismiss", "button"]],
    dump: {
      "crashbar.visible": /^1$/, "crashbartext.caption": /^RapidR Studio closed unexpectedly last time\.$/,
      "outputbox.text": /\[crash\] bar=1 report=\d{8}-\d{6}-studio-\d+-crash\.txt kind=crash pending=\d{8}-\d{6}-studio-\d+-crash\.txt /,
    },
  },
  {
    // (in Spanish: the strip, its buttons)
    name: "crash-banner-es",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait", delay: 6 },
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    contrast: true,
    a11y: [[/^La última vez, RapidR Studio se cerró de forma inesperada\. Un informe de lo que pasó queda en esta computadora; no se envió nada\./, null], ["Ver informe", "button"], ["Copiar informe", "button"], ["Guardar informe...", "button"], ["Informar en GitHub...", "button"], ["Ocultar", "button"]],
    dump: { "crashbar.visible": /^1$/, "crashbartext.caption": /^La última vez, RapidR Studio se cerró de forma inesperada\.$/, "crashviewb.caption": /^&Ver informe$/ },
  },
  {
    // (Copy Report: the report's text on the clipboard; seen — not offered
    // at the next start)
    name: "crash-copy",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:copy,wait,crash:report", delay: 6 },
    dump: { "outputbox.text": /\[crash\] bar=1 report=\S+-crash\.txt kind=crash pending= copied=\d{4,} /, "clipboard.text": /^RapidR crash report\n===================\n\nRapidR Studio stopped unexpectedly[\s\S]*\nWhere: crash\.inc line \d+, in FUNCTION CrashStep \(OnTimer of Macro\)\n[\s\S]*\nPrivacy\n/ },
  },
  {
    // (Save Report...: a copy where the person picks — the test's dialog answers)
    name: "crash-save",
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:save,wait", delay: 6 },
    env: { RAPIDR_TEST_CLIPBOARD: "1", RAPIDR_TEST_FILE_DIALOG: join(WORK, "crash-save-desktop", "saved-report.txt") },
    desktopFiles: ["saved-report.txt"],
    dump: { "outputbox.text": /Saved the report as \S*saved-report\.txt\./ },
  },
  {
    // (Report on GitHub..., yes: the prefilled new-issue address — title,
    // the report in the body —; nothing opened in a test)
    name: "crash-github",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:github=yes,wait,crash:report", delay: 6 },
    dump: { "outputbox.text": /url=https:\/\/github\.com\/iBobX\/RapidR\/issues\/new\?title=RapidR%20Studio%3A%20RapidR%27s%20crash%20test[^&\s]*%282\.\d+\.\d+%29&body=%2A%2AWhat%20happened%2A%2A[^\s]*%60%60%60text%0ARapidR%20crash%20report/ },
  },
  {
    // (…and its question first: what opens, what it shows, a GitHub
    // account and 13+, public — Cancel the default)
    name: "crash-github-ask",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:github", delay: 6 },
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    captureAll: true,
    contrast: true,
    a11y: [[/^You need: A GitHub account\. GitHub's terms ask that you be 13 or older/, null], ["Open GitHub", "button"], ["Cancel", "button"]],
    dump: { "consentdialog.visible": YES, "consenttitle.caption": /^Report this on GitHub\?$/ },
  },
  {
    // (Dismiss: gone, seen; the next start says nothing)
    name: "crash-dismiss",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "wait,wait,wait,crash:dismiss,wait,crash:check,crash:report", delay: 6 },
    dump: { "crashbar.visible": /^0$/, "outputbox.text": /\[crash\] bar=0 report=\S+ kind=crash pending= / },
  },
  {
    // (School Lock on — the other lane's hook, CrashSchoolLock: no strip)
    name: "crash-locked",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,crash:panic",
    delay: 5,
    restart: { noOpen: true, do: "crash:lock,wait,wait,wait,crash:check,crash:report", delay: 6 },
    dump: { "crashbar.visible": /^0$/, "outputbox.text": /\[crash\] bar=0 / },
  },
  {
    // (a UI hang: the report within 6 s, how long it lasted once Studio
    // answers again; started again, no strip — it wasn't a crash)
    name: "crash-hang",
    env: { RAPIDR_TEST_CLIPBOARD: "1" },
    desktopOnly: true,
    do: "wait,wait,crash:hang,wait,wait,crash:report",
    delay: 13,
    restart: { noOpen: true, do: "wait,wait,wait,crash:report", delay: 6 },
    dump: { "crashbar.visible": /^0$/, "outputbox.text": /\[crash\] bar=0 report= kind= pending= copied=0 program= list=\d{8}-\d{6}-studio-\d+-hang\.txt outcome=Answered again after [6-9] s/ },
  },
  {
    // (a program RapidR crashes in its timer's handler: the Output says
    // where, plainly, with links to the line and the report)
    name: "crash-program",
    desktopOnly: true,
    open: "tests/fixtures/studio_crash/main.rr",
    copy: true,
    env: { RAPIDR_TEST_CLIPBOARD: "1", RAPIDR_TEST_PROGRAM_CRASH: "panic@main.rr:24" },
    do: "run.start,wait,wait,wait,wait,wait,wait,wait,wait,crash:report",
    delay: 9,
    themes: ["rapidr-light", "rapidr-dark"],
    capture: true,
    contrast: true,
    dump: {
      "outputbox.text": /\[main\.rr ended, exit code 101\]\nYour program stopped: RapidR's crash test \(RAPIDR_TEST_CRASH\): a panic, as a bug in RapidR would make one, at main\.rr line 24, in SUB Tick \(OnTimer of Ticker\)\.\nRapidR stopped it: a problem in RapidR, not in your code\. A report stays on this computer: View Report[\s\S]*program=\d{8}-\d{6}-program-\d+-crash\.txt/,
      "status.panel(0).caption": /^Your program stopped: a problem in RapidR$/,
    },
  },
  {
    name: "crash-program-es",
    desktopOnly: true,
    desktopArgs: ["--lang", "es"],
    open: "tests/fixtures/studio_crash/main.rr",
    copy: true,
    env: { RAPIDR_TEST_CLIPBOARD: "1", RAPIDR_TEST_PROGRAM_CRASH: "overflow@main.rr:24" },
    do: "run.start,wait,wait,wait,wait,wait,wait,wait,wait,crash:report",
    delay: 9,
    capture: true,
    dump: { "outputbox.text": /Tu programa se detuvo: se desbordó la pila \(las llamadas fueron demasiado profundas\), en main\.rr línea 24, en SUB Tick \(OnTimer de Ticker\)\.\nRapidR lo detuvo: es un problema de RapidR, no de tu código\. Un informe queda en esta computadora: Ver informe/ },
  },
];



// (I8 AI-REAL) The settings dialog's input: the AI tab clicked, Fetch
// Models, OK and Cancel pressed with the mouse; text typed key by key into
// a component (a US keyboard's keys); `idle(n)`: n turns of the loop with
// nothing done (a label's click it doesn't handle), for an answer to come.
function typeInto(comp, text) {
  const out = [];
  for (const c of text) {
    if (/[a-z]/.test(c)) out.push(String(c.toUpperCase().charCodeAt(0)));
    else if (/[A-Z]/.test(c)) out.push(`${c.charCodeAt(0)}_256`);
    else if (/[0-9 ]/.test(c)) out.push(String(c.charCodeAt(0)));
    else out.push({ "-": "189", ".": "190", "_": "189_256", "/": "191", ":": "186_256" }[c]);
  }
  return out.map((k) => `${comp}.__key_${k}`).join(",");
}
function idle(n) {
  return Array(n).fill("aionnote.onclick").join(",");
}

// (the web) Settings > AI with the browser's mouse and keyboard, where the
// controls are on the page (their accessibility elements' boxes): the AI
// tab, OpenAI picked in the provider list (Down, Enter), a refused key
// typed, Fetch Models — back, enabled, the refusal said; the key typed
// again over it, Fetch Models — the list has the models; dropped, captured.
async function settingsByMouse(page, scale, record) {
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const center = async (sel) => {
    const b = await page.locator(sel).first().boundingBox();
    return b && { x: b.x + b.width / 2, y: b.y + b.height / 2, b };
  };
  const click = async (sel, dx = 0) => {
    const c = await center(sel);
    if (!c) throw new Error(`nothing at ${sel}`);
    await page.mouse.click(c.x + dx, c.y);
    await page.waitForTimeout(250);
  };
  const until = async (what, test, ms = 15000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(200);
    }
    return v;
  };
  try {
    const tab = page.getByRole("tab", { name: "AI", exact: true });
    if (await tab.count()) {
      const b = await tab.first().boundingBox();
      await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
    } else {
      const s = await center("#rr-settingstabs");
      await page.mouse.click(s.b.x + 75, s.b.y + 12);
    }
    await page.waitForTimeout(300);
    record("", (await prop("settingstabs", "tabindex")) === "1", "the AI tab, clicked");
    await click("#rr-aiprovbox");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(300);
    record("", (await prop("aiprovbox", "text")) === "OpenAI", `OpenAI picked from the list with the keyboard (${await prop("aiprovbox", "text")})`);
    await click("#rr-aikeyedit");
    await page.keyboard.type("mock-bad-key-1");
    await click("#rr-aifetchbtn");
    const note = await until(() => prop("aifetchnote", "caption"), (v) => /401/.test(v));
    const disabled = await page.locator("#rr-aifetchbtn").getAttribute("aria-disabled");
    record("", /refused the API key \(401\)/.test(note) && disabled !== "true", `a refused key: said ("${note.slice(0, 60)}…"), Fetch Models enabled for a screen reader too (aria-disabled ${disabled})`);
    await click("#rr-aikeyedit");
    await page.keyboard.press("ControlOrMeta+A");
    await page.keyboard.type("mock-good-key-1");
    await click("#rr-aifetchbtn");
    const n = await until(() => prop("aimodelcombo", "itemcount"), (v) => v === "3");
    record("", n === "3", `the key typed again, Fetch Models again: the Model list has the provider's ${n} models`);
    const combo = await center("#rr-aimodelcombo");
    await page.mouse.click(combo.b.x + combo.b.width - 10, combo.y);
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(WORK, `ai-settings-web-mouse-list@${scale}x.png`) });
    await page.keyboard.press("Escape");
  } catch (e) {
    record("", false, `the dialog used with the mouse: ${e.message.split("\n")[0]}`);
  }
}

// (EXAMPLES-DATA) A folder and everything in it writable again (true) or
// read-only (false): an install's examples, as /usr/lib/rapidr or the macOS
// app has them.
function writableTree(path, writable) {
  if (!existsSync(path)) return;
  const st = statSync(path);
  if (st.isDirectory()) {
    if (writable) chmodSync(path, 0o755);
    for (const f of readdirSync(path)) writableTree(join(path, f), writable);
    if (!writable) chmodSync(path, 0o555);
  } else chmodSync(path, writable ? 0o644 : 0o444);
}

function runDesktop(c, scale = 1) {
  const dir = join(WORK, `${c.name}-desktop${scale > 1 ? `@${scale}x` : ""}`);
  writableTree(join(dir, "install"), true);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  // (EXAMPLES-DATA: `readOnlyHome` — RapidR's home as an install has it, its
  // examples where they can't be changed; the Documents folder the case's
  // own, with `seed`'s files in it first)
  let home = ".";
  if (c.readOnlyHome) {
    home = join(dir, "install");
    cpSync(join(ROOT, "examples"), join(home, "examples"), { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$|big(_\d+)?\.csv$/.test(src) });
    cpSync(join(ROOT, "ide", "assets"), join(home, "ide", "assets"), { recursive: true });
    for (const [rel, text] of Object.entries(c.seed || {})) {
      mkdirSync(dirname(join(dir, "docs", rel)), { recursive: true });
      writeFileSync(join(dir, "docs", rel), text);
    }
    // (`installedWritable`: an install the user can write — a macOS app
    // dragged to Applications —, known by its release.toml)
    if (c.installedWritable) writeFileSync(join(home, "release.toml"), 'version = "0.0.0"\nrust = "1.0.0"\nkind = "sdk"\n');
    else writableTree(home, false);
  }
  const args = ["run", "ide/studio.rr", "--home", home, "--fresh", "--theme", c.theme || "rapidr-light", ...(c.desktopArgs || [])];
  // (R-NAMES: a RapidQ program's folder copied here, imported from there —
  // the import writes its copy beside it; {dir}: the case's own folder —
  // a new project goes there)
  const steps = c.do ? c.do.replaceAll("{dir}", dir).replaceAll("{mockroot}", MOCK_ROOT).replaceAll("{mock}", MOCK_URL) : "";
  if (c.importFrom) {
    cpSync(join(ROOT, dirname(c.importFrom)), join(dir, "src"), { recursive: true });
    // (before: steps ahead of the import — Studio's include folders; {root}: the repository)
    const before = c.before ? c.before.replaceAll("{root}", ROOT) + "," : "";
    args.push("--do", before + `import:${join(dir, "src", c.importFrom.split("/").pop())}` + (steps ? "," + steps : ""));
  } else if (steps) args.push("--do", steps);
  if (c.open && c.copyDir) {
    // (the project's whole folder: Build writes the app beside it — not the
    // *.tmp.* files an example run left in it: they are no part of the
    // project, and the web build leaves them out too)
    // (a versions case's copy lives outside this repository — Studio rightly
    // refuses to start versions inside a folder that already has them —
    // reached through a link from the case's folder)
    const copyTo = c.fsck ? mkdtempSync(join(tmpdir(), "rapidr-versions-")) : join(dir, "project");
    cpSync(join(ROOT, dirname(c.open)), copyTo, { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$/.test(src) });
    if (c.fsck) symlinkSync(copyTo, join(dir, "project"));
    args.push(join(dir, "project", c.open.split("/").pop()));
  } else if (c.open && c.copy) {
    const to = join(dir, c.open.split("/").pop());
    copyFileSync(join(ROOT, c.open), to);
    args.push(to);
  } else if (c.open && c.readOnlyHome) {
    // (an example as the install has it: its path there)
    args.push(join(home, c.open));
  } else if (c.open) {
    args.push(c.open);
  }
  // (studio.caption: always asked for — no line of the dump at all means
  // Studio had quit before it)
  const keys = [...Object.keys(c.dump), "studio.caption"];
  const run = (args, delay, events = c.events) => spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: Math.max(90000, delay * 1000 + 60000),
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(delay),
      RAPIDR_MENU: "window",
      ...(scale > 1 ? { RAPIDR_SCALE: String(scale) } : {}),
      ...(c.env || {}),
      RAPIDR_TEST_DUMP: keys.join(","),
      ...(events ? { RAPIDR_TEST_EVENTS: events } : {}),
      // (THEME-LIVE: how the system looks to the test)
      ...(c.look ? { RAPIDR_TEST_SYSTEM_LOOK: c.look } : {}),
      // (the user asked the system for reduced motion)
      ...(c.reducedMotion ? { RAPIDR_REDUCED_MOTION: "1" } : {}),
      ...(c.contrast ? { RAPIDR_TEST_CONTRAST: join(dir, "contrast.json") } : {}),
      ...(c.a11y ? { RAPIDR_TEST_A11Y: join(dir, "a11y.json") } : {}),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
      // (an example's copy, a build that can't go beside its program: the
      // case's own Documents folder — never the user's)
      RAPIDR_DOCUMENTS: join(dir, "docs"),
      // (the AI assistant: keys in memory only — never the user's keychain —,
      // no provider's key from this shell, its audit log the case's)
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_AI_TEST_SERVER: MOCK_ROOT,
      RAPIDR_AI_AUDIT: join(dir, "ai-audit.jsonl"),
      // (R1: the case's kept chats in its own folder — kept across a
      // restart, never another case's)
      RAPIDR_AI_CHATS: join(dir, "chats"),
      // (Studio's own folder — the copies of unsaved work, its log: the
      // case's, kept across a restart, never another case's)
      RAPIDR_STUDIO_DATA: join(dir, "studio"),
      // (S6: the user's login shell isn't read by the tests — the PATH
      // cases give one of their own, RAPIDR_LOGIN_SHELL)
      RAPIDR_LOGIN_PATH: "off",
      ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "",
      ...(c.env || {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: join(ROOT, c.folder) } : {}),
    },
  });
  rmSync(join(WORK, `${c.name}.reg`), { force: true });
  // (EXTERNAL-CHANGES: another process changing the project's files while
  // Studio has them open — tests/ext_writer.mjs, step n when Studio's
  // `ext:go=n` asks, Studio's `ext:await=n` waiting for it; its folder of
  // meeting files is RAPIDR_TEST_EXT)
  let writer;
  if (c.external) {
    const meet = join(dir, "meet");
    mkdirSync(meet, { recursive: true });
    writeFileSync(join(dir, "external.json"), JSON.stringify(c.external));
    writer = spawn(process.execPath, [join(HERE, "ext_writer.mjs"), meet, join(dir, "project"), join(dir, "external.json")], { stdio: "ignore" });
    c.env = { ...(c.env || {}), RAPIDR_TEST_EXT: meet };
  }
  let r = run(args, c.delay);
  if (writer && writer.exitCode === null) writer.kill();
  if (c.restart) {
    // (started again on the settings the first run left: not --fresh)
    const again = args.filter((a) => a !== "--fresh");
    const k = again.indexOf("--do");
    if (k >= 0) again.splice(k, 2);
    // (noOpen: started without the project — as from the Dock — S2's offer)
    if (c.restart.noOpen && c.open) again.pop();
    if (c.restart.do) {
      if (c.restart.noOpen) again.push("--do", c.restart.do);
      else again.splice(again.length - 1, 0, "--do", c.restart.do);
    }
    r = run(again, c.restart.delay || c.delay, "events" in c.restart ? c.restart.events : c.events);
  }
  // (the install's copy writable again: the results folder can be cleared)
  if (c.readOnlyHome) writableTree(home, true);
  const dump = parseDump(r.stdout || "", keys);
  const quit = r.status === 0 && !r.error && Object.keys(dump).length === 0;
  // (VERSIONS-G3: the repository the run wrote, checked by the system's git
  // — an independent judge, isolated from this machine's configuration;
  // Studio never runs git)
  if (c.fsck && existsSync(join(dir, "project", ".git"))) {
    const g = spawnSync("git", ["fsck", "--full", "--strict", "--no-dangling"], { cwd: join(dir, "project"), encoding: "utf8", env: { ...process.env, GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: "/dev/null", LC_ALL: "C" } });
    dump.__fsck = g.error ? "no git" : g.status === 0 ? "ok" : `${g.stdout}${g.stderr}`.trim();
  }
  // (files the run must leave: the program's own window captured)
  for (const f of c.desktopFiles || []) dump["file " + f] = existsSync(join(dir, f)) ? "there" : "missing";
  for (const f of c.desktopGone || []) dump["gone " + f] = existsSync(join(dir, f)) ? "there" : "gone";
  // (the Studio window, to look at: <case>-desktop@<s>x.png; a dialog open
  // over it — `captureAll` — <case>-desktop-<n>@<s>x.png)
  if (c.capture && existsSync(join(dir, "window-1.bmp"))) {
    spawnSync("sips", ["-s", "format", "png", join(dir, "window-1.bmp"), "--out", join(WORK, `${c.name}-desktop@${scale}x.png`)], { stdio: "ignore" });
    for (let n = 2; c.captureAll && existsSync(join(dir, `window-${n}.bmp`)); n++) {
      spawnSync("sips", ["-s", "format", "png", join(dir, `window-${n}.bmp`), "--out", join(WORK, `${c.name}-desktop-${n}@${scale}x.png`)], { stdio: "ignore" });
    }
  }
  const files = {};
  for (const f of Object.keys(c.files || {})) {
    const p = c.copyDir ? join(dir, "project", f) : join(dir, f);
    files[f] = existsSync(p) ? readFileSync(p, "utf8") : undefined;
  }
  const contrast = c.contrast && existsSync(join(dir, "contrast.json")) ? JSON.parse(readFileSync(join(dir, "contrast.json"), "utf8")) : undefined;
  const a11y = c.a11y && existsSync(join(dir, "a11y.json")) ? readFileSync(join(dir, "a11y.json"), "utf8") : undefined;
  return { dump, quit, files, contrast, a11y };
}

// "name=value" lines, a value running on to the next "name=" line.
function parseDump(text, names) {
  const out = {};
  const lines = text.split("\n");
  let cur = null;
  for (const line of lines) {
    const k = names.find((n) => line.toLowerCase().startsWith(n.toLowerCase() + "="));
    if (k) {
      cur = k;
      out[k] = line.slice(k.length + 1);
    } else if (cur && !line.startsWith("[rapidr]")) {
      out[cur] += "\n" + line;
    }
  }
  for (const k of Object.keys(out)) out[k] = out[k].replace(/\n+$/, "");
  return out;
}

// (Build Web App) The download the page made: sound, with the files a web
// host needs; unzipped and served, the app runs.
async function builtWebApp(page, record, zipName, wantFiles) {
  const got = await Promise.all(page.__downloads || []);
  record("", got.length === 1 && got[0].name === zipName, `Build Web App downloads ${zipName} (got: ${got.map((g) => g.name).join(", ") || "nothing"})`);
  if (got.length !== 1) return;
  const zip = got[0].path;
  const t = spawnSync("unzip", ["-t", zip], { encoding: "utf8" });
  record("", t.status === 0, "the zip is sound (unzip -t: every file's checksum)");
  const names = spawnSync("unzip", ["-Z1", zip], { encoding: "utf8" }).stdout.split("\n").filter(Boolean);
  const want = ["index.html", "loader.js", "rapidrintr.js", "rapidrintr_bg.wasm", "THIRD-PARTY-NOTICES.txt", "fonts/index.json", "_headers", ...wantFiles];
  const missing = want.filter((n) => !names.includes(n));
  record("", missing.length === 0, `the zip has the page, the runtime, the program's bytecode and its files (${names.length} files${missing.length ? "; missing: " + missing.join(", ") : ""})`);
  const dir = join(WORK, "served-" + zipName.replace(/\W/g, ""));
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  spawnSync("unzip", ["-q", zip, "-d", dir]);
  const html = existsSync(join(dir, "index.html")) ? readFileSync(join(dir, "index.html"), "utf8") : "";
  record("", /Content-Security-Policy/.test(html) && /rapidr-program/.test(html), "its page carries the program's strict policy");
  // (served as any static web host would, on a port of its own)
  const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json", ".txt": "text/plain" };
  const server = createServer((req, res) => {
    const path = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/\.\.+/g, ""));
    if (!existsSync(path) || !path.startsWith(dir) || path === dir) {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, { "content-type": types[extname(path)] || "application/octet-stream" }).end(readFileSync(path));
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const app = await page.context().newPage();
  const errors = [];
  app.on("pageerror", (e) => errors.push(e.message));
  try {
    await app.goto(`http://127.0.0.1:${server.address().port}/index.html`, { waitUntil: "load" });
    await app.waitForFunction(() => document.getElementById("rapidr-status")?.textContent === "" && document.querySelector(".rr-kwin"), null, { timeout: 60000, polling: 200 });
    await app.waitForTimeout(500);
    await app.screenshot({ path: join(WORK, zipName.replace(/\.zip$/, "") + "-served.png") });
    record("", errors.length === 0, `the unzipped app runs from a web server: its window is on the page${errors.length ? " (errors: " + errors.join("; ") + ")" : ""}`);
  } catch (e) {
    const status = await app.evaluate(() => document.getElementById("rapidr-status")?.textContent).catch(() => "");
    record("", false, `the unzipped app runs from a web server (${e.message.split("\n")[0]}; page says: ${status})`);
  } finally {
    await app.close();
    server.close();
  }
}

async function runWeb(browser, c, scale = 1, record = () => {}) {
  const opts = { viewport: c.viewport || { width: 1920, height: 1080 }, deviceScaleFactor: scale };
  if (!c.restart) return runWebPage(await browser.newContext(opts), c, true, scale, record);
  // (started again in the same browser profile: the page's settings kept)
  const ctx = await browser.newContext(opts);
  await runWebPage(ctx, c, false, scale, record);
  return runWebPage(ctx, { ...c, do: c.restart.do, delay: c.restart.delay || c.delay, fresh: false, ...("events" in c.restart ? { events: c.restart.events } : {}), ...(c.restart.noOpen ? { open: "" } : {}) }, true, scale, record);
}

async function runWebPage(ctx, c, last, scale, record) {
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  // (what the page offers as downloads: Build Web App's zip)
  page.__downloads = [];
  page.on("download", (d) => page.__downloads.push((async () => ({ name: d.suggestedFilename(), path: await d.path() }))()));
  try {
    // (`repo/path=>name`: the file at that name in the page's store)
    const files = (c.webFiles || []).map((f) => f.split("=>")).map(([from, to]) => ({ path: to || from, text: readFileSync(join(ROOT, from), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    // (what Studio writes to the page's store, kept for the test: the
    // last text of each file)
    await page.addInitScript(() => {
      window.__rrWrites = {};
      window.RAPIDR_FILE_SINK = (path, bytes) => { window.__rrWrites[path] = new TextDecoder().decode(bytes); };
    });
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      // (webDelay: a page's Studio starts later than the desktop's — a case
      // timed against an answer the mock holds says when it looks there)
      RAPIDR_CAPTURE_DELAY: String(c.webDelay ?? c.delay),
      RAPIDR_TEST_DUMP: [...Object.keys(c.webDump || c.dump), "studio.caption"].join(","),
      RAPIDR_AI_TEST_SERVER: MOCK_ROOT,
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: c.folder } : {}),
      ...(c.look ? { RAPIDR_TEST_SYSTEM_LOOK: c.look } : {}),
      ...(c.reducedMotion ? { RAPIDR_REDUCED_MOTION: "1" } : {}),
    });
    const q = new URLSearchParams({ theme: c.theme || "rapidr-light" });
    if (c.fresh !== false) q.set("fresh", "");
    // (Studio a 1280 x 800 window on the page, unless the case has it fill the page)
    if (!c.maximized) q.set("window", "normal");
    // (the program's files are in the page's store: imported from there;
    // {dir}: the case's own folder in the page's store)
    const doSteps = c.webDo ?? c.do;
    const steps = doSteps ? doSteps.replaceAll("{dir}", `/flows/${c.name}`).replaceAll("{mockroot}", MOCK_ROOT).replaceAll("{mock}", MOCK_URL) : "";
    // (before: steps ahead of the import; {root}/: the store's paths are the repository's)
    const before = c.before ? c.before.replaceAll("{root}/", "") + "," : "";
    if (c.importFrom) q.set("do", before + `import:${c.importFrom}` + (steps ? "," + steps : ""));
    else if (steps) q.set("do", steps);
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    // (the test's end, or — Studio quit — the program's)
    await page.waitForFunction(() => window.rr && (window.rr.rapidr_test_results() || window.rr.rapidr_main_done()), null, { timeout: Math.max(90000, (c.webDelay ?? c.delay) * 1000 + 60000), polling: 200 });
    const raw = await page.evaluate(() => window.rr.rapidr_test_results());
    const quit = !raw && (await page.evaluate(() => window.rr.rapidr_main_done()));
    const results = raw ? JSON.parse(raw) : { dump: [] };
    // (what the case does on the page itself: real mouse input)
    if (c.web) await c.web(page, scale, record);
    if (c.capture) await page.screenshot({ path: join(WORK, `${c.name}-web@${scale}x.png`) });
    const writes = await page.evaluate(() => window.__rrWrites);
    const out = {};
    for (const f of Object.keys(c.files || {})) {
      const path = (c.webFiles || []).find((w) => w.endsWith("/" + f) || w === f);
      const written = Object.entries(writes).find(([p]) => p.endsWith("/" + f) || p === f);
      out[f] = written ? written[1] : path ? readFileSync(join(ROOT, path), "utf8") : undefined;
    }
    const contrast = c.contrast && results.contrast ? JSON.parse(results.contrast) : undefined;
    return { dump: parseDump(results.dump.join("\n"), [...Object.keys(c.webDump || c.dump), "studio.caption"]), quit, files: out, errors, contrast, a11y: results.a11y };
  } finally {
    await page.close();
    if (last) await ctx.close();
  }
}

mkdirSync(WORK, { recursive: true });
const browser = await chromium.launch();
let passed = 0, failed = 0;
const check = (label, run, c) => {
  const dump = run.dump;
  if (c.quit !== undefined || c.files) {
    const ok = !!run.quit === !!c.quit;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): Studio ${c.quit ? "quit" : "stayed open"}${ok ? "" : ` (it ${run.quit ? "quit" : "didn't quit"})`}`);
  }
  for (const [f, re] of Object.entries(c.files || {})) {
    const v = run.files[f];
    const ok = v !== undefined && re.test(v.replace(/\r\n/g, "\n"));
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${f} ${ok ? "" : `= ${JSON.stringify(v)} (wanted ${re})`}`);
  }
  for (const f of label === "desktop" ? c.desktopFiles || [] : []) {
    const ok = dump["file " + f] === "there";
    ok ? passed++ : failed++;
    console.log((ok ? "✓ " : "✗ ") + c.name + " (desktop): " + f + (ok ? " written" : " missing"));
  }
  // (VERSIONS-G3: git fsck --strict on the repository the run wrote)
  if (c.fsck && label === "desktop" && dump.__fsck !== undefined && dump.__fsck !== "no git") {
    const ok = dump.__fsck === "ok";
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (desktop): git fsck --strict ${ok ? "passes" : dump.__fsck}`);
  }
  // (AI-UX: files the run must have removed — Undo's)
  for (const f of label === "desktop" ? c.desktopGone || [] : []) {
    const ok = dump["gone " + f] === "gone";
    ok ? passed++ : failed++;
    console.log((ok ? "✓ " : "✗ ") + c.name + " (desktop): " + f + (ok ? " gone" : " still there"));
  }
  for (const [k, re] of Object.entries(c.dump)) {
    // (a tree's Text: its lines end CR LF, as LoadFromFile reads them)
    const v = k.endsWith("tree.text") && dump[k] !== undefined ? dump[k].replace(/\r/g, "").replace(/\n$/, "") : dump[k];
    const ok = v !== undefined && re.test(v);
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? "" : `= ${JSON.stringify(v)} (wanted ${re})`}`);
  }
  if (c.contrast) {
    // (every window's text against its ground: WCAG AA in RapidR's looks)
    const windows = run.contrast || [];
    const checked = windows.reduce((n, w) => n + w.checked, 0);
    const failures = windows.flatMap((w) => w.failures.map((f) => `${w.form}: ${JSON.stringify(f.text)} ${f.color} on ${f.background} ${f.ratio}:1 (needs ${f.need})`));
    const ok = windows.length > 0 && checked > 0 && failures.length === 0;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): text contrast, ${checked} runs in ${windows.length} window(s)${failures.length ? ` — ${failures.length} below AA:\n    ${failures.slice(0, 12).join("\n    ")}` : windows.length ? " all AA" : " (no report)"}`);
  }
  if (c.a11y) {
    // (AI-UX) What a screen reader is told: the windows' accessibility
    // trees (RAPIDR_TEST_A11Y's, the same on both hosts) — each check a
    // [name, role, state, test] the tree must have a node for
    const nodes = [];
    const walk = (n) => { if (!n) return; nodes.push(n); (n.children || []).forEach(walk); };
    try {
      JSON.parse((run.a11y || "[]").replace(/"id":\d+/g, '"id":0')).forEach(walk);
    } catch {
      /* no tree: every check fails below */
    }
    // (a fourth, optional: a test of the node and the tree — where it is, say)
    for (const [name, role, state, extra] of c.a11y) {
      const hit = nodes.find((n) => (name instanceof RegExp ? name.test(n.name) : n.name === name) && (!role || n.role === role) && (!state || (n.states || []).includes(state)) && (!extra || extra(n, nodes)));
      const ok = !!hit;
      ok ? passed++ : failed++;
      console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): a11y ${role || "node"} ${name}${state ? ` (${state})` : ""}${ok ? "" : ` — not in the tree (${nodes.length} nodes; ${nodes.filter((n) => (n.states || []).includes("focused")).map((n) => `${n.role} "${n.name}"`).join(", ") || "nothing"} focused)`}`);
    }
  }
  // (S8 / S9: what the dumped values say together — a pane's controls
  // filling it, a size kept — each `(dump) => [ok, what]`)
  for (const check of c.checks || []) {
    let ok = false;
    let what = String(check);
    try {
      [ok, what] = check(dump);
    } catch (e) {
      what = `${what}: ${e.message}`;
    }
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${what}`);
  }
  for (const [k, file] of Object.entries(c.same || {})) {
    const want = readFileSync(join(ROOT, file), "utf8").replace(/\r\n/g, "\n").replace(/\n+$/, "");
    const v = dump[k];
    const ok = v !== undefined && v.replace(/\n+$/, "") === want;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? `equals ${file}` : `differs from ${file}`}`);
  }
};
// (AI-REAL) What the mock provider was asked during a case (its own log:
// the keys' last four characters only), as one JSON text to match
const mockLog = async () => {
  // (a fresh connection each time: a kept-alive one the mock closed meanwhile is reset)
  for (let i = 0; ; i++) {
    try {
      return await (await fetch(`${MOCK_ROOT}/__log`, { headers: { connection: "close" } })).json();
    } catch (e) {
      if (i >= 3) throw e;
      await new Promise((r) => setTimeout(r, 300));
    }
  }
};
function checkMockLog(name, entries, re) {
  const text = JSON.stringify(entries);
  record(name, re.test(text), `the provider was asked as it should be (${re})${re.test(text) ? "" : `: ${text.slice(0, 600)}`}`);
}
const record = (name, ok, what) => {
  ok ? passed++ : failed++;
  console.log(`${ok ? "✓" : "✗"} ${name}: ${what}`);
};
// (docs/studio-wow.md §4.3: no Run or Debug command falls through to
// RunCommand's "(… : not there yet)": each id of the command table's Run and
// Debug menus has its CASE in debug.inc's DebugCommand or shell.inc's
// RunCommand)
if (!filters.length || filters.some((f) => "commands-handled".includes(f))) {
  const table = readFileSync(join(ROOT, "ide/commands.inc"), "utf8");
  const handlers = readFileSync(join(ROOT, "ide/debug.inc"), "utf8") + readFileSync(join(ROOT, "ide/shell.inc"), "utf8");
  const ids = [...table.matchAll(/AddCmd "([^"]+)", "[^"]*", "(Run|Debug)"/g)].map((m) => m[1]);
  const missing = ids.filter((id) => !new RegExp(`CASE "${id.replace(/\./g, "\\.")}"`).test(handlers));
  record("commands-handled", ids.length >= 15 && missing.length === 0, `every Run / Debug command is handled (${ids.length} commands${missing.length ? "; missing: " + missing.join(", ") : ""})`);
}

// (a case with `themes`: one run per look, named for it)
const SHORT = { "rapidr-light": "light", "rapidr-dark": "dark", "rapidr-high-contrast": "contrast", classic: "classic" };
const RUNS = CASES.flatMap((c) => (c.themes ? c.themes.map((theme) => ({ ...c, theme, name: `${c.name}-${SHORT[theme] || theme}` })) : [c]));
for (const c of RUNS.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  if (HOSTS.includes("desktop") && !c.webOnly) {
    for (const scale of c.capture ? c.scales || [1] : [1]) {
      const seen = c.mockLog ? (await mockLog()).length : 0;
      check(c.capture && scale > 1 ? `desktop @${scale}x` : "desktop", runDesktop(c, scale), c);
      if (c.mockLog) checkMockLog(`${c.name} (desktop)`, (await mockLog()).slice(seen), c.mockLog);
    }
  }
  if (!HOSTS.includes("web") || c.desktopOnly) continue;
  for (const scale of c.scales || [1]) {
    const label = c.scales ? `web @${scale}x` : "web";
    try {
      const seen = c.mockLog ? (await mockLog()).length : 0;
      const web = await runWeb(browser, c, scale, (name, ok, what) => record(`${c.name} (${label})`, ok, what));
      check(label, web, c.webDump ? { ...c, dump: c.webDump } : c);
      if (c.mockLog) checkMockLog(`${c.name} (${label})`, (await mockLog()).slice(seen), c.mockLog);
      if (web.errors.length) console.log(`  (page errors: ${web.errors.join("; ")})`);
    } catch (e) {
      failed++;
      console.log(`✗ ${c.name} (${label}): ${e.message.split("\n")[0]}`);
    }
  }
}

// (S-BUILD) The web page asks before it's left (the browser's own "Leave
// site?": beforeunload) while a document has changes not saved —
// Studio.Modified — and not once they're saved. The page is used for real
// first (a click: browsers ask only on a page the user touched).
async function beforeUnload(label, steps, want) {
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  try {
    const files = SAVE_FILES.map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => {
      window.RAPIDR_STUDIO_TEST_FILES = files;
      // (no test script: Studio runs as the user's)
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_FILE_SINK = () => {};
    }, files);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: steps, open: SAVE_FILES[0] });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && /ONE x/.test(window.rr.rapidr_get_prop("codedoc(0)", "text")), null, { timeout: 60000, polling: 200 });
    await page.waitForTimeout(1500);
    const modified = await page.evaluate(() => window.rr.rapidr_get_prop("studio", "modified"));
    await page.mouse.click(700, 300);
    let asked = false;
    page.on("dialog", async (d) => {
      if (d.type() === "beforeunload") asked = true;
      await d.dismiss();
    });
    await page.close({ runBeforeUnload: true });
    await new Promise((r) => setTimeout(r, 1500));
    const ok = asked === want;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} beforeunload-${label} (web): leaving the page ${want ? "asks first" : "doesn't ask"}${ok ? "" : ` (asked: ${asked}, Studio.Modified = ${modified})`}`);
  } catch (e) {
    failed++;
    console.log(`✗ beforeunload-${label} (web): ${e.message.split("\n")[0]}`);
  } finally {
    await ctx.close();
  }
}
// (AI-REAL, the web) Settings > AI used with the browser's own mouse and
// keyboard: Studio runs as the user's (no test script — under one, the
// kernel leaves user input alone), the mock provider standing in.
// (DEBUG-RUN) Studio in the browser, driven by the browser's own keys and
// mouse (no test script: nothing drops them), as the person uses it: F9 F5
// paused at 6 — the ▶ button, as a screen reader reads it, "Continue (F5)"
// and enabled; F5 goes on (the page isn't reloaded: Studio keeps the key),
// a click on ▶ goes on too, F10 steps, Shift+F5 stops, F5 runs again, F11
// steps into tally.inc.
async function debugByRealKeys() {
  const name = "debug-commands-web-keys (web)";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, test, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(150);
    }
    return v;
  };
  const where = async () => `${await prop("session", "state")}@${await prop("session", "currentline")}`;
  const runButton = async () => {
    for (const label of ["Continue (F5)", "Run (F5)"]) {
      const b = page.getByRole("button", { name: label, exact: true });
      if (await b.count()) return { label, enabled: (await b.first().getAttribute("aria-disabled")) !== "true", box: await b.first().boundingBox() };
    }
    return { label: null };
  };
  try {
    const files = DEBUG_FILES.map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => {
      // (no test script: the files only — the person's keys and mouse do the rest)
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_STUDIO_TEST_FILES = files;
      window.RAPIDR_FILE_SINK = () => {};
    }, files);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "wait,line:6", open: DEBUG_FILES[0] });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("codedoc(0)", "caretline")) === "6", null, { timeout: 60000, polling: 200 });
    await page.evaluate(() => { window.__notReloaded = true; });
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    let w = await until(where, (v) => v === "paused@6");
    record(name, w === "paused@6", `F9, F5: paused at line 6 (${w})`);
    let b = await until(runButton, (v) => v.label === "Continue (F5)");
    record(name, b.label === "Continue (F5)" && b.enabled, `paused: the ▶ button reads "Continue (F5)" and is enabled (${b.label}, ${b.enabled})`);
    await page.screenshot({ path: join(WORK, "debug-commands-web-paused.png") });
    await page.keyboard.press("F5");
    const vars = await until(() => prop("varstree", "text"), (v) => /i = 2/.test(v));
    w = await where();
    record(name, w === "paused@6" && /i = 2/.test(vars) && (await page.evaluate(() => window.__notReloaded === true)), `F5 while paused goes on — stopped at 6 the next time round, i = 2, the page not reloaded (${w})`);
    b = await runButton();
    await page.mouse.click(b.box.x + b.box.width / 2, b.box.y + b.box.height / 2);
    const vars3 = await until(() => prop("varstree", "text"), (v) => /i = 3/.test(v));
    record(name, /i = 3/.test(vars3), "a click on ▶ (Continue) goes on: i = 3");
    // (i = 3, the last time round: over AddUp, out of the loop, to the PRINT)
    await page.keyboard.press("F10");
    w = await until(where, (v) => v === "paused@8");
    record(name, w === "paused@8", `F10 steps over AddUp, out of the loop: paused at 8 (${w})`);
    await page.keyboard.press("Shift+F5");
    w = await until(() => prop("session", "state"), (v) => v === "stopped");
    b = await until(runButton, (v) => v.label === "Run (F5)");
    record(name, w === "stopped" && b.label === "Run (F5)" && b.enabled, `Shift+F5 stops it: ▶ "Run (F5)" enabled (${w}, ${b.label}, ${b.enabled})`);
    await page.keyboard.press("F5");
    w = await until(where, (v) => v === "paused@6");
    const vars1 = await prop("varstree", "text");
    record(name, w === "paused@6" && /i = 1/.test(vars1), `F5 runs it again after the Stop: paused at 6, i = 1 (${w})`);
    await page.keyboard.press("F11");
    const file = await until(() => prop("session", "currentfile"), (v) => /tally\.inc$/i.test(v));
    record(name, /tally\.inc$/i.test(file), `F11 steps into AddUp, in tally.inc (${file})`);
    await page.keyboard.press("Shift+F5");
    w = await until(() => prop("session", "state"), (v) => v === "stopped");
    record(name, w === "stopped", `Shift+F5 ends it (${w})`);
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN) The program's window while it's paused, in the browser: it
// stays on the page, its clicks are dropped (not run later), Show Program
// Window gives it the keyboard; going on, its clicks work again.
async function pausedProgramByMouse() {
  const name = "debug-commands-web-paused-window (web)";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, test, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(150);
    }
    return v;
  };
  const press = async (label, role = "button") => {
    const b = page.getByRole(role, { name: label, exact: true }).first();
    const box = await b.boundingBox();
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  };
  // (the program's "Again" button, clicked by the mouse where it shows)
  const again = async () => {
    const r = await runFrame(page).evaluate(() => {
      const el = [...document.querySelectorAll("[role=button]")].find((e) => /Again/.test(e.getAttribute("aria-label") || e.textContent || ""));
      if (!el) return null;
      const b = el.getBoundingClientRect();
      return { x: b.left + b.width / 2, y: b.top + b.height / 2 };
    });
    if (!r) throw new Error("no Again button in the program's window");
    await page.mouse.click(r.x, r.y);
    await page.waitForTimeout(400);
  };
  const clicks = async () => (/clicks = (\d+)/.exec(await prop("varstree", "text")) || [])[1];
  try {
    const src = "tests/fixtures/studio_debug/waits.rr";
    await page.addInitScript((files) => {
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_STUDIO_TEST_FILES = files;
      window.RAPIDR_FILE_SINK = () => {};
    }, [{ path: src, text: readFileSync(join(ROOT, src), "utf8") }]);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "wait,view.code", open: src });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("session", "state")) === "stopped", null, { timeout: 60000, polling: 200 });
    await page.waitForTimeout(800);
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(600);
    await press("Pause");
    let st = await until(() => prop("session", "state"), (v) => v === "paused");
    record(name, st === "paused" && (await clicks()) === "0", `Pause (the tool button, clicked): paused while it waits, clicks = ${await clicks()}`);
    const rects = await page.evaluate(() => window.RAPIDR_STUDIO_RUN_RECTS.length);
    record(name, rects > 0, `paused: the program's window is still on the page (${rects})`);
    await again();
    await again();
    await page.screenshot({ path: join(WORK, "debug-commands-web-paused-window.png") });
    st = await prop("session", "state");
    record(name, st === "paused" && (await clicks()) === "0", `paused: two clicks on its Again button run nothing (${st}, clicks = ${await clicks()})`);
    // (Show Program Window, from the command palette: Ctrl+Shift+P, typed, Enter)
    await page.keyboard.press("Control+Shift+P");
    await page.waitForTimeout(300);
    await page.keyboard.type("Show Program Window");
    await page.waitForTimeout(300);
    await page.keyboard.press("Enter");
    await page.waitForTimeout(400);
    const focused = await page.evaluate(() => document.activeElement && document.activeElement.tagName);
    record(name, focused === "IFRAME", `Show Program Window gives the program's frame the keyboard (${focused})`);
    // (on, then paused again: the clicks made while paused were dropped, not kept for later)
    await press("Continue (F5)");
    await until(() => prop("session", "state"), (v) => v === "running");
    await page.waitForTimeout(500);
    await press("Pause");
    await until(() => prop("session", "state"), (v) => v === "paused");
    record(name, (await clicks()) === "0", `going on runs none of the clicks made while paused (clicks = ${await clicks()})`);
    await press("Continue (F5)");
    await until(() => prop("session", "state"), (v) => v === "running");
    await page.waitForTimeout(400);
    await again();
    await press("Pause");
    await until(() => prop("session", "state"), (v) => v === "paused");
    record(name, (await clicks()) === "1", `running again, its click works (clicks = ${await clicks()})`);
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN) Robert's walk, in the browser: examples/gui/themes.rr with a
// breakpoint in SUB Switch (line 94); a click on the program's Dark stops
// there — Studio has the keyboard back, the ▶ button "Continue (F5)" and
// enabled, the program's window still on the page; F5 goes on: the
// program's window gets the keyboard and the theme turns dark.
async function themesPausedWeb() {
  const name = "debug-commands-web-themes (web)";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, test, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(150);
    }
    return v;
  };
  // (the program's controls, by what a screen reader reads, in its frame)
  const programLabel = (re) => runFrame(page).evaluate((src) => {
    const re = new RegExp(src);
    for (const e of document.querySelectorAll("[aria-label],[role]")) {
      const t = e.getAttribute("aria-label") || e.textContent || "";
      if (re.test(t)) {
        const b = e.getBoundingClientRect();
        return { text: t, x: b.left + b.width / 2, y: b.top + b.height / 2 };
      }
    }
    return null;
  }, re.source);
  try {
    const src = "examples/gui/themes.rr";
    await page.addInitScript((files) => {
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_STUDIO_TEST_FILES = files;
      window.RAPIDR_FILE_SINK = () => {};
    }, [{ path: src, text: readFileSync(join(ROOT, src), "utf8") }]);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "wait,view.code,line:94", open: src });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("codedoc(0)", "caretline")) === "94", null, { timeout: 60000, polling: 200 });
    await page.waitForTimeout(500);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(800);
    const dark = await programLabel(/^Dark$/);
    if (!dark) throw new Error("no Dark radio button in the program's window");
    await page.mouse.click(dark.x, dark.y);
    const w = await until(async () => `${await prop("session", "state")}@${await prop("session", "currentline")}`, (v) => v === "paused@94");
    record(name, w === "paused@94", `a click on the program's Dark stops at line 94 (${w})`);
    const active = await page.evaluate(() => document.activeElement && document.activeElement.tagName);
    record(name, active !== "IFRAME", `paused: Studio has the keyboard back (${active})`);
    const run = page.getByRole("button", { name: "Continue (F5)", exact: true });
    const enabled = (await run.count()) > 0 && (await run.first().getAttribute("aria-disabled")) !== "true";
    record(name, enabled, "paused: the ▶ button reads \"Continue (F5)\" and is enabled");
    const rects = await page.evaluate(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length);
    record(name, rects > 0, `paused: the program's window is still on the page (${rects})`);
    await page.screenshot({ path: join(WORK, "debug-commands-web-themes-paused.png") });
    await page.keyboard.press("F5");
    const st = await until(() => prop("session", "state"), (v) => v === "running");
    // (what the program's label says, read in its own frame)
    const programProp = (c, p) => runFrame(page).evaluate(([c, p]) => String(window.__rapidr_rt.rapidr_get_prop(c, p)), [c, p]);
    const now = await until(() => programProp("now", "caption"), (v) => /dark/i.test(v));
    record(name, st === "running" && /dark/i.test(now), `F5 goes on: the theme turns dark (${st}, ${now})`);
    await page.waitForTimeout(700);
    const focused = await page.evaluate(() => document.activeElement && document.activeElement.tagName);
    record(name, focused === "IFRAME", `going on, the program's window has the keyboard — and keeps it (${focused})`);
    await page.screenshot({ path: join(WORK, "debug-commands-web-themes-continued.png") });
    // (typed into its edit, slowly — Studio's frames meanwhile, a caret's
    // blink: the keys are the program's, Studio's code untouched)
    const edit = await runFrame(page).evaluate(() => {
      const e = document.querySelector("input");
      const r = e.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    });
    await page.mouse.click(edit.x, edit.y);
    await page.keyboard.press("End");
    for (const ch of " typed") {
      await page.keyboard.type(ch);
      await page.waitForTimeout(250);
    }
    const typed = await until(() => programProp("edit", "text"), (v) => /typed$/.test(v), 3000);
    const code = await prop("codedoc(0)", "text");
    record(name, /Text to edit typed$/.test(typed) && !code.includes("typed"), `typing in the running program's edit goes to it, not to Studio's code (${JSON.stringify(typed)})`);
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN, the debugger's events in the browser) Studio and its program's
// frame driven by the browser's own mouse and keys: a breakpoint in a
// handler, the event triggered — it stops (Studio has the keyboard back),
// F5, again, three times; F10 out of the handler, again: it stops again.
// `trigger(page, frame)` makes the event happen as a person would.
function webDebug(page) {
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, test, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(120);
    }
    return v;
  };
  const where = async () => `${await prop("session", "state")}@${await prop("session", "currentline")}`;
  // (going on is the program's word: Studio's status says Running once the
  // program has heard it — the session's own state says so as soon as it's
  // asked; a click before that reaches a stopped program, and is dropped,
  // as it should be)
  const status = async () => (await prop("status", "panel(0).caption")).trim();
  // (the stop shown — its line, its values; Studio's status no longer
  // Running: a key pressed the moment the session has the stop, before
  // Studio has shown it, finds ▶ still off —: the keyboard is Studio's)
  const keysBack = async () => {
    await until(status, (v) => !v.startsWith("Running"), 3000);
    return until(() => page.evaluate(() => document.activeElement && document.activeElement.tagName), (v) => v !== "IFRAME", 3000);
  };
  const button = async (label) => {
    const b = page.getByRole("button", { name: label, exact: true });
    return (await b.count()) > 0 && (await b.first().getAttribute("aria-disabled")) !== "true";
  };
  return { prop, until, where, status, keysBack, button };
}

async function openStudioOn(page, src, files, line) {
  await page.addInitScript((files) => {
    window.RAPIDR_STUDIO_TEST = {};
    window.RAPIDR_STUDIO_TEST_FILES = files;
    window.RAPIDR_FILE_SINK = () => {};
  }, files.map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") })));
  const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: `wait,view.code,line:${line}`, open: src });
  await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
  await page.waitForFunction((l) => window.rr && String(window.rr.rapidr_get_prop("codedoc(0)", "caretline")) === String(l), line, { timeout: 60000, polling: 200 });
  await page.waitForTimeout(400);
}

async function webEventRepeats(name, src, line, trigger, { afterStep = 1, files = [src] } = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { prop, until, where, status, keysBack } = webDebug(page);
  const label = `${name} (web)`;
  try {
    await openStudioOn(page, src, files, line);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    for (let k = 1; k <= 3; k++) {
      await trigger(page, frame, k);
      const w = await until(where, (v) => v === `paused@${line}`);
      const keys = await keysBack();
      record(label, w === `paused@${line}` && keys !== "IFRAME", `trigger ${k}: stops at ${line}, Studio has the keyboard (${w}, ${keys})`);
      if (k === 1) await page.screenshot({ path: join(WORK, `${name}-web-paused.png`) });
      await page.keyboard.press(k < 3 ? "F5" : "F10");
      await until(async () => `${await status()}|${await where()}`, (v) => v.startsWith("Running") || (v.split("|")[1].startsWith("paused") && v.split("|")[1] !== `paused@${line}`), 3000);
    }
    // (out of the handler: F10 once more if the step stopped at its END SUB)
    for (let i = 0; i < afterStep && (await prop("session", "state")) === "paused"; i++) {
      await page.keyboard.press("F10");
      await until(status, (v) => v.startsWith("Running"), 3000);
    }
    await until(status, (v) => v.startsWith("Running"), 3000);
    const st = await prop("session", "state");
    record(label, st === "running", `Step Over out of the handler: the program runs on (${st})`);
    await trigger(page, frame, 4);
    const w = await until(where, (v) => v === `paused@${line}`);
    record(label, w === `paused@${line}`, `after the Step Over, the next event stops at ${line} again (${w})`);
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(label, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (a control of the program's frame, by its role and name: its centre on the page)
async function frameBox(frame, role, name) {
  const loc = name === undefined ? frame.locator(role).first() : frame.getByRole(role, { name, exact: true }).first();
  const b = await loc.boundingBox();
  if (!b) throw new Error(`no ${role} ${name ?? ""} in the program's window`);
  return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
}

async function webEventFlows() {
  const EV = "tests/fixtures/studio_debug/events.rr";
  // 1. hello_form.rr's one-line OnChange: a letter typed into the name (End
  // first: the edit, given the keyboard again, has its text selected — a
  // letter typed over a selected "a" would change nothing)
  await webEventRepeats("debug-events-web-onchange", "examples/gui/hello_form.rr", 49, async (page, frame) => {
    const at = await frameBox(frame, "input");
    await page.mouse.click(at.x, at.y);
    await page.keyboard.press("End");
    await page.keyboard.type("a");
  });
  // 2. a click
  await webEventRepeats("debug-events-web-click", EV, 58, async (page, frame) => {
    const at = await frameBox(frame, "button", "Click");
    await page.mouse.click(at.x, at.y);
  });
  // 2. a key (OnKeyPress)
  await webEventRepeats("debug-events-web-keypress", EV, 62, async (page, frame) => {
    const at = await frameBox(frame, "input");
    await page.mouse.click(at.x, at.y);
    await page.keyboard.press("End");
    await page.keyboard.type("k");
  });
  // 2. the focus (the list's OnEnter): the edit, then the list
  await webEventRepeats("debug-events-web-focus", EV, 66, async (page, frame) => {
    const box = await frameBox(frame, "input");
    await page.mouse.click(box.x, box.y);
    await page.waitForTimeout(150);
    const list = await frameBox(frame, "listbox", "");
    await page.mouse.click(list.x, list.y + 20);
  });
  // 2. the mouse moving over the form
  await webEventRepeats("debug-events-web-mousemove", EV, 83, async (page, frame, k) => {
    const form = await frameBox(frame, "dialog", "Events");
    await page.mouse.move(form.x + 120 + k * 3, form.y + 60);
    await page.mouse.move(form.x + 124 + k * 3, form.y + 64);
  });
  // 2. the close box (OnClose, the close cancelled: the window stays)
  await webEventRepeats("debug-events-web-close", EV, 74, async (page, frame) => {
    const r = await frame.evaluate(() => {
      const b = document.querySelector(".rr-kwin").getBoundingClientRect();
      return { x: b.right - 16, y: b.top + 15 };
    });
    await page.mouse.click(r.x, r.y);
  }, { afterStep: 2 });
  // 2. a timer (OnTimer, every second)
  await webEventRepeats("debug-events-web-timer", EV, 79, async (page) => {
    await page.waitForTimeout(200);
  });
}

// (DEBUG-RUN, scenario 3) Typed into the program's window while it's
// stopped (hello_form.rr, at the OnChange line 49): the keys are dropped —
// the name stays as it was, nothing crashes, no stop waits behind them —;
// ▶ Continue clicked: it runs on, no second stop; the next key typed stops
// again.
async function webTypingWhilePaused() {
  const name = "debug-events-web-dropped (web)";
  const src = "examples/gui/hello_form.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { until, where, status, keysBack } = webDebug(page);
  try {
    await openStudioOn(page, src, [src], 49);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    const edit = frame.locator("input").first();
    const at = await frameBox(frame, "input");
    await page.mouse.click(at.x, at.y);
    await page.keyboard.press("End");
    await page.keyboard.type("a");
    let w = await until(where, (v) => v === "paused@49");
    await keysBack();
    const before = await edit.inputValue();
    // (typed into the stopped program: the click gives its frame the keys)
    await page.mouse.click(at.x, at.y);
    await page.keyboard.type("xyz");
    await page.waitForTimeout(600);
    const during = await edit.inputValue();
    const still = await where();
    record(name, w === "paused@49" && still === "paused@49" && during === before, `keys typed into the stopped program are dropped (${w}, then ${still}; ${JSON.stringify(before)} → ${JSON.stringify(during)})`);
    // (▶ on Studio's tool bar, clicked: the keys are the program's frame's)
    const go = await page.getByRole("button", { name: "Continue (F5)", exact: true }).first().boundingBox();
    await page.mouse.click(go.x + go.width / 2, go.y + go.height / 2);
    const running = await until(status, (v) => v.startsWith("Running"), 3000);
    const again = await until(where, (v) => v.startsWith("paused"), 1500);
    const after = await edit.inputValue();
    record(name, running.startsWith("Running") && again === "running@0" && after === before, `▶ Continue: it runs on, no stop queued behind the dropped keys (${running}, ${again}, ${JSON.stringify(after)})`);
    const at2 = await frameBox(frame, "input");
    await page.mouse.click(at2.x, at2.y);
    await page.keyboard.press("End");
    await page.keyboard.type("b");
    w = await until(where, (v) => v === "paused@49");
    record(name, w === "paused@49", `the next key typed stops at 49 again (${w})`);
    await keysBack();
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN, scenario 20) The stopped program's window closed by its close
// box: the program ends, as Stop ends it — Studio Ready, ▶ Run (F5)
// enabled —, and F5 runs it again.
async function webCloseWhilePaused() {
  const name = "debug-commands-web-close-paused (web)";
  const src = "examples/gui/hello_form.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { until, where, status, keysBack, button } = webDebug(page);
  try {
    await openStudioOn(page, src, [src], 49);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    const at = await frameBox(frame, "input");
    await page.mouse.click(at.x, at.y);
    await page.keyboard.press("End");
    await page.keyboard.type("a");
    const w = await until(where, (v) => v === "paused@49");
    await keysBack();
    const r = await frame.evaluate(() => {
      const b = document.querySelector(".rr-kwin").getBoundingClientRect();
      return { x: b.right - 16, y: b.top + 15 };
    });
    await page.mouse.click(r.x, r.y);
    const st = await until(status, (v) => v === "Ready", 5000);
    const run = await button("Run (F5)");
    record(name, w === "paused@49" && st === "Ready" && run, `its window closed while stopped at 49: it ends — ${st}, ▶ Run (F5) ${run ? "enabled" : "off"} (${w})`);
    await page.keyboard.press("F5");
    const again = await until(status, (v) => v.startsWith("Running"), 8000);
    record(name, again.startsWith("Running"), `F5 runs it again (${again})`);
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN, scenario 4) A handler that opens a dialog (dialogs.rr's Ask:
// MESSAGEDLG): paused on that line, Step Over (F10) — the dialog shows,
// answered with the mouse, the program stops on the line after it — three
// times.
async function webDialogStep() {
  const name = "debug-events-web-dialog (web)";
  const src = "examples/gui/dialogs.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { until, where, status, keysBack } = webDebug(page);
  try {
    await openStudioOn(page, src, [src], 84);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    for (let k = 1; k <= 3; k++) {
      const ask = await frameBox(frame, "button", "Ask...");
      await page.mouse.click(ask.x, ask.y);
      let w = await until(where, (v) => v === "paused@84");
      await keysBack();
      record(name, w === "paused@84", `Ask ${k}: stops on the dialog's line, 84 (${w})`);
      await page.keyboard.press("F10");
      // (the step runs the dialog: it shows in the program's window)
      const yes = await until(async () => (await frame.getByRole("button", { name: "Yes", exact: true }).count()) > 0, (v) => v, 6000);
      if (k === 1) await page.screenshot({ path: join(WORK, "debug-events-web-dialog.png") });
      record(name, yes === true, `Ask ${k}: Step Over shows the dialog (${yes})`);
      if (!yes) break;
      const at = await frameBox(frame, "button", "Yes");
      await page.mouse.click(at.x, at.y);
      w = await until(where, (v) => v.startsWith("paused") && v !== "paused@84");
      record(name, /^paused@8[56]$/.test(w), `Ask ${k}: answered Yes, it stops on the line after the dialog (${w})`);
      await keysBack();
      await page.keyboard.press("F5");
      await until(status, (v) => v.startsWith("Running"), 3000);
    }
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-RUN, scenarios 5 and 6 in the browser) A handler that fires
// another (Grow widens the form: OnResize inside it) stops in the inner
// one, the call stack has both — the breakpoint in Resized before the run,
// so the form's first sizes stop there too and are gone on from; a run-time
// error in a handler stops at its line, and on Continue Studio follows the
// program.
async function webNestedAndError() {
  const name = "debug-events-web-nested (web)";
  const src = "tests/fixtures/studio_debug/events.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { prop, until, where, status, keysBack, button } = webDebug(page);
  try {
    await openStudioOn(page, src, [src], 70);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    // (the form's first sizes as it shows — OnResize, OnShow, OnResize —
    // stop there first: each stop gives Studio the keyboard, though the
    // program's window had just come up and taken it, and F5 goes on)
    const firstStops = [];
    let firstOk = true;
    for (let i = 0; i < 4; i++) {
      const v = await until(where, (v) => v.startsWith("paused"), 2500);
      if (!v.startsWith("paused")) break;
      const keys = await keysBack();
      await page.keyboard.press("F5");
      const st = await until(status, (v) => v.startsWith("Running"), 3000);
      firstStops.push(`${v} ${keys} → ${st}`);
      firstOk = firstOk && v === "paused@70" && keys !== "IFRAME" && st.startsWith("Running");
    }
    record(name, firstStops.length > 0 && firstOk, `the form's first sizes stop in OnResize, Studio has the keyboard and F5 goes on (${firstStops.join("; ")})`);
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(500);
    const frame = runFrame(page);
    const grow = await frameBox(frame, "button", "Grow");
    await page.mouse.click(grow.x, grow.y);
    // (Grow's new width: OnResize inside Grow — the call stack has both, as
    // on the desktop —, then Continue: the program goes on, Grow's click
    // over, and no other stop)
    const w = await until(where, (v) => v === "paused@70");
    const keys = await keysBack();
    const stack = (await prop("stacktree", "text")).replace(/\r/g, "");
    record(name, w === "paused@70" && keys !== "IFRAME" && /^Resized\s+events\.rr:70\nGrow\s+events\.rr:91\n/.test(stack), `Grow's OnResize stops at its breakpoint, inside Grow; Studio has the keyboard (${w}, ${keys}, ${JSON.stringify(stack.slice(0, 80))})`);
    await page.keyboard.press("F5");
    const going = await until(status, (v) => v.startsWith("Running"), 3000);
    const again = await until(where, (v) => v.startsWith("paused"), 1500);
    record(name, going.startsWith("Running") && again === "running@0", `Continue: the program goes on, no other stop (${going}, ${again})`);
    await page.keyboard.press("Shift+F5");
    await until(status, (v) => !v.startsWith("Breakpoint"), 3000);
    // (6: a fresh run, no breakpoint: the error in Oops's handler)
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const oops = await frameBox(runFrame(page), "button", "Oops");
    await page.mouse.click(oops.x, oops.y);
    const e = await until(where, (v) => v === "paused@95");
    await keysBack();
    const reason = await prop("session", "stopreason");
    record("debug-events-web-error (web)", e === "paused@95" && reason === "exception", `a run-time error in a handler stops at its line (${e}, ${reason})`);
    // (Continue: what the program does without the debugger — RapidQ's
    // (RC.EXE, DEBUG-2): the handler ends there and the program runs on, as
    // on the desktop: running, ▶ off and ❙❙ on; the next click stops at the
    // error again, three times)
    for (let k = 1; k <= 3; k++) {
      await page.keyboard.press("F5");
      const after = await until(status, (v) => v.startsWith("Running") || /exit code/.test(v), 5000);
      // (the status text says Running a pump before the tool bar's buttons follow — ❙❙ on, ▶ off: wait for them, as a person looks at them, instead of reading them in that gap: this check failed one run in five)
      const running = after.startsWith("Running") && (await until(async () => (await button("Pause")) && !(await button("Run (F5)")), (v) => v === true, 3000)) === true;
      record("debug-events-web-error (web)", running, `Continue ${k} after the error: the handler ends, the program runs on (${after})`);
      if (k === 3) break;
      const again = await frameBox(runFrame(page), "button", "Oops");
      await page.mouse.click(again.x, again.y);
      const e2 = await until(where, (v) => v === "paused@95");
      await keysBack();
      record("debug-events-web-error (web)", e2 === "paused@95", `click ${k + 1}: the error stops at its line again (${e2})`);
    }
    // (the program answers: a click on Click runs its handler, no stop)
    const click = await frameBox(runFrame(page), "button", "Click");
    await page.mouse.click(click.x, click.y);
    await page.waitForTimeout(600);
    const st = await prop("session", "state");
    record("debug-events-web-error (web)", st === "running", `after the errors the program still answers its events (${st})`);
    await page.screenshot({ path: join(WORK, "debug-events-web-error-continued.png") });
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-2) The focus on the web (RapidR's OnEnter / OnExit): focus.rr with
// breakpoints in the name's OnExit (41) and Greet's OnEnter (45); in the
// program's window, Tab from the name stops at 41, F5, then at 45, F5 — three
// rounds, Shift+Tab back between them; then a click on Greet from the name
// stops the same way, before its OnClick.
async function webFocusOrder() {
  const name = "debug-events-web-focus-order (web)";
  const src = "tests/fixtures/studio_debug/focus.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { prop, until, where, status, keysBack } = webDebug(page);
  try {
    await openStudioOn(page, src, [src], 41);
    await page.keyboard.press("F9");
    // (the caret to 45 as a person moves it: four lines down)
    for (let i = 0; i < 4; i++) await page.keyboard.press("ArrowDown");
    await page.keyboard.press("F9");
    record(name, (await prop("bptree", "text")).replace(/\r/g, "").trim() === "focus.rr:41\nfocus.rr:45", `breakpoints at 41 and 45 (${JSON.stringify(await prop("bptree", "text"))})`);
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    const stops = [];
    const going = async () => {
      await page.keyboard.press("F5");
      await until(status, (v) => v.startsWith("Running"), 3000);
    };
    for (let k = 1; k <= 3; k++) {
      // (the name has the focus: a click in it first, then Tab)
      const ed = await frameBox(frame, "input");
      await page.mouse.click(ed.x, ed.y);
      await page.waitForTimeout(200);
      await page.keyboard.press("Tab");
      const a = await until(where, (v) => v.startsWith("paused"));
      await keysBack();
      if (k === 1) await page.screenshot({ path: join(WORK, "debug-events-web-focus-onexit.png") });
      await going();
      const b = await until(where, (v) => v.startsWith("paused"));
      await keysBack();
      await going();
      stops.push(`${a} ${b}`);
      record(name, a === "paused@41" && b === "paused@45", `Tab ${k}: OnExit (41) then OnEnter (45) (${a}, ${b})`);
    }
    // (a click on Greet from the name: the same two stops, then its OnClick)
    const ed = await frameBox(frame, "input");
    await page.mouse.click(ed.x, ed.y);
    await page.waitForTimeout(300);
    const greet = await frameBox(frame, "button", "Greet");
    await page.mouse.click(greet.x, greet.y);
    const a = await until(where, (v) => v.startsWith("paused"));
    await keysBack();
    await going();
    const b = await until(where, (v) => v.startsWith("paused"));
    await keysBack();
    await going();
    record(name, a === "paused@41" && b === "paused@45", `a click on Greet: OnExit (41) then OnEnter (45) (${a}, ${b})`);
    // (the button let go while the program was stopped: dropped, as all its
    // input then — RapidQ's too: no OnClick. Clicked again, focused already:
    // no OnExit / OnEnter, its OnClick runs)
    await page.waitForTimeout(400);
    await page.mouse.click(greet.x, greet.y);
    await page.waitForTimeout(600);
    record(name, (await where()).startsWith("running"), `Greet clicked again: no stop (${await where()})`);
    // (the program's own label, read in its frame)
    const steps = await frame.evaluate(() => String(window.__rapidr_rt.rapidr_get_prop("steps", "caption")));
    record(name, / -name \+greet click$/.test(steps), `then its OnClick ran: the trail ends "-name +greet click" (${JSON.stringify(steps)})`);
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}

// (DEBUG-2, scenario 7 on the web) Stepping inside a handler with the
// keyboard: steps.rr's Go (the button's OnClick) stops at its breakpoint
// (33); F11 into Twice (its SUB line, 18), Shift+F11 back in Go (34), the
// caret a line down and Ctrl+F10 (Run to Cursor: 35), a line up and
// Ctrl+Shift+F10 (Set Next Statement: 34), F10 (35); F5 — running, and the
// next click stops at 33 again. Then (scenario 11) the breakpoint taken off
// while the program runs: the next click doesn't stop.
async function webHandlerSteps() {
  const name = "debug-steps-web-handler (web)";
  const src = "tests/fixtures/studio_debug/steps.rr";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  const { until, where, status, keysBack } = webDebug(page);
  try {
    await openStudioOn(page, src, [src, "tests/fixtures/studio_debug/tally.inc"], 33);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const click = async () => {
      const go = await frameBox(runFrame(page), "button", "Go");
      await page.mouse.click(go.x, go.y);
    };
    const step = async (keys, want, what) => {
      for (const k of keys) await page.keyboard.press(k);
      const w = await until(where, (v) => v === `paused@${want}`, 4000);
      await keysBack();
      record(name, w === `paused@${want}`, `${what}: paused at ${want} (${w})`);
    };
    await click();
    const w = await until(where, (v) => v === "paused@33");
    await keysBack();
    record(name, w === "paused@33", `the click stops at Go's breakpoint, 33 (${w})`);
    await step(["F11"], 18, "Step Into Twice");
    await step(["Shift+F11"], 34, "Step Out, back in Go");
    await step(["ArrowDown", "Control+F10"], 35, "Run to Cursor a line down");
    await step(["ArrowUp", "Control+Shift+F10"], 34, "Set Next Statement a line up");
    await step(["F10"], 35, "Step Over");
    await page.keyboard.press("F5");
    await until(status, (v) => v.startsWith("Running"), 3000);
    await click();
    const again = await until(where, (v) => v === "paused@33");
    await keysBack();
    record(name, again === "paused@33", `F5, the next click stops at 33 again (${again})`);
    // (11: Continue, then the breakpoint off while it runs — a click in
    // Studio's editor gives Studio the keyboard back (the program has it
    // while it runs), Go to Line 33, F9)
    await page.keyboard.press("F5");
    await until(status, (v) => v.startsWith("Running"), 3000);
    const studioKeys = async () => {
      // (a moment after Continue, as a person's: Continue hands the
      // program's frame the keyboard —; the mouse onto Studio's editor and a
      // click there: Studio has the keyboard again; then a click on line 33
      // itself, its lines 20 pixels apart)
      const before = await until(() => page.evaluate(() => document.activeElement && document.activeElement.tagName), (v) => v === "IFRAME", 3000);
      await page.waitForTimeout(400);
      await page.mouse.move(600, 300, { steps: 5 });
      await page.waitForTimeout(200);
      await page.mouse.click(600, 300);
      await page.waitForTimeout(150);
      const keys = await page.evaluate(() => document.activeElement && document.activeElement.tagName);
      record(name, keys !== "IFRAME", `a click in Studio's editor while the program runs gives Studio the keyboard (${before} → ${keys})`);
      const at = Number(await page.evaluate(() => String(window.rr.rapidr_get_prop("codedoc(0)", "caretline"))));
      await page.mouse.click(600, 300 + (33 - at) * 20);
      await page.waitForTimeout(200);
    };
    await studioKeys();
    await page.keyboard.press("F9");
    await page.waitForTimeout(300);
    const bpOff = (await page.evaluate(() => String(window.rr.rapidr_get_prop("bptree", "text")))).trim();
    await click();
    await page.waitForTimeout(800);
    const off = await where();
    record(name, off.startsWith("running") && bpOff === "", `the breakpoint taken off while it runs: the next click doesn't stop (${off}, breakpoints ${JSON.stringify(bpOff)})`);
    await studioKeys();
    await page.keyboard.press("F9");
    await page.waitForTimeout(300);
    await click();
    const on = await until(where, (v) => v === "paused@33");
    record(name, on === "paused@33", `put back while it runs: the next click stops (${on})`);
    await page.keyboard.press("Shift+F5");
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "debug-steps-web-handler".includes(f) || f === "debug-steps"))) {
  await webHandlerSteps();
}

// (COPY-ANYWHERE, the web) Real input in Chrome, on a page without the
// tests' hooks (they hold the user's input off): three.rr run, "line of"
// dragged over with the mouse, ⌘C (Ctrl+C) — the page's copy event carries
// exactly it (Chromium's headless clipboard, never the system's).
async function outputCopyWebMouse() {
  const name = "copy-output-web-mouse (web)";
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await ctx.newPage();
  try {
    const files = [{ path: "tests/fixtures/studio_copy/three.rr", text: readFileSync(join(ROOT, "tests/fixtures/studio_copy/three.rr"), "utf8") }];
    await page.addInitScript((files) => {
      window.RAPIDR_STUDIO_TEST = {};
      window.RAPIDR_STUDIO_TEST_FILES = files;
      window.RAPIDR_FILE_SINK = () => {};
    }, files);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "run.start", open: "tests/fixtures/studio_copy/three.rr" });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && /third line/.test(String(window.rr.rapidr_get_prop("outputbox", "text"))), null, { timeout: 60000, polling: 200 });
    await page.waitForTimeout(800);
    const lines = (await page.evaluate(() => String(window.rr.rapidr_get_prop("outputbox", "text")))).split("\n");
    const row = lines.indexOf("second line of output");
    const box = await page.locator("#rr-outputbox").first().boundingBox();
    record(name, !!box && row >= 0, `Output shows the program's lines (line ${row + 1}: "second line of output")`);
    if (!box || row < 0) return;
    await page.evaluate(() => {
      window.__copied = null;
      document.addEventListener("copy", (e) => { window.__copied = e.clipboardData ? e.clipboardData.getData("text/plain") : null; });
    });
    const y = box.y + 30 + row * 20 + 10;
    await page.mouse.move(box.x + 62, y);
    await page.mouse.down();
    for (const x of [80, 100, 117]) await page.mouse.move(box.x + x, y);
    await page.mouse.up();
    await page.waitForTimeout(300);
    const sel = await page.evaluate(() => String(window.rr.rapidr_get_prop("outputbox", "seltext")));
    record(name, sel === "line of", `the mouse's drag selects "line of" (${JSON.stringify(sel)})`);
    await page.keyboard.press("ControlOrMeta+c");
    await page.waitForTimeout(300);
    const copied = await page.evaluate(() => window.__copied);
    record(name, copied === "line of", `⌘C in Chrome puts exactly it on the clipboard (${JSON.stringify(copied)})`);
    await page.screenshot({ path: join(WORK, "copy-output-web-mouse.png") });
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "copy-output-web-mouse".includes(f)))) {
  await outputCopyWebMouse();
}

async function aiSettingsByMouse() {
  const name = "ai-settings-web-mouse (web)";
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  try {
    await page.addInitScript((root) => {
      window.RAPIDR_STUDIO_TEST = { RAPIDR_AI_TEST_SERVER: root };
      window.RAPIDR_FILE_SINK = () => {};
    }, MOCK_ROOT);
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "view.settings", open: "examples/gui/hello_form.rr" });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("settingsdialog", "visible")).match(/^(-1|1|true)$/i), null, { timeout: 60000, polling: 200 });
    await page.waitForTimeout(800);
    await settingsByMouse(page, 1, (n, ok, what) => record(name, ok, what));
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "ai-settings-web-mouse".includes(f) || f === "ai-"))) {
  await aiSettingsByMouse();
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "debug-commands-web-keys".includes(f) || f === "debug-commands"))) {
  await debugByRealKeys();
  await pausedProgramByMouse();
  await themesPausedWeb();
  await webCloseWhilePaused();
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "debug-events-web".includes(f) || f === "debug-events"))) {
  await webEventFlows();
  await webTypingWhilePaused();
  await webDialogStep();
  await webNestedAndError();
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "debug-events-web-focus-order".includes(f) || f === "debug-events"))) {
  await webFocusOrder();
}
// (THEME-LIVE) The browser switches the page's colour scheme (playwright's
// emulateMedia) on a live page — no test script: the page's own media
// queries, followed as they change. Studio follows without a reload: the
// look it reads, its Welcome page's colours (a page opened in that scheme's),
// dark, high contrast (forced colours), light again; a capture of each.
// (USE-AUDIT) The manual's way to add a component — select the toolbox item, press
// Enter — and then, as a person does, a click on a component of the form to select
// it: it selects it. It used to place another component of the item's kind (the
// click that picked the item had armed the placing tool, and Enter didn't spend it).
// Real mouse and key events on the page, the result read from the accessibility mirror.
async function webToolboxEnterThenClick() {
  const name = "toolbox-enter-then-click (web)";
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await ctx.newPage();
  try {
    await page.addInitScript(() => { window.RAPIDR_STUDIO_TEST = {}; });
    const src = "examples/gui/hello_form.rr";
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: "wait,view.designer", open: src });
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; window.RAPIDR_FILE_SINK = () => {}; }, [{ path: src, text: readFileSync(join(ROOT, src), "utf8") }]);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("studio", "caption")).includes("hello_form"), null, { timeout: 90000, polling: 200 });
    await page.waitForTimeout(1500);
    const click = async (locator) => {
      const b = await locator.boundingBox();
      await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
      await page.waitForTimeout(500);
    };
    const labels = () => page.locator("[aria-label*='(RLabel)'][role=option]").count();
    const before = await labels();
    await click(page.getByRole("treeitem", { name: "RLabel — Label", exact: true }));
    await page.keyboard.press("Enter");
    await page.waitForTimeout(700);
    const added = await labels();
    record(name, added === before + 1, `select the toolbox item and press Enter: one label more (${before} -> ${added})`);
    await click(page.locator("[aria-label^='GreetButton (RButton)'][role=option]").first());
    const after = await labels();
    const sel = await page.locator("[aria-label^='GreetButton (RButton)'][role=option][aria-selected=true]").count();
    record(name, after === added && sel === 1, `a click on the Greet button selects it and places nothing (${added} -> ${after} labels, Greet selected: ${sel === 1})`);
    await page.screenshot({ path: join(WORK, "toolbox-enter-then-click-web.png") });
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "toolbox-enter-then-click".includes(f) || f === "toolbox"))) {
  await webToolboxEnterThenClick();
}
// (WEB-OPEN, the web) A project opened as a person opens it (2026-10-22:
// RapidRWebServer opened "empty" — main.rr blank, no forms): the Welcome
// page's Open Project or File... clicked, the browser's file picker answers
// Kiosk.rrproj alone (Chromium's pickers replaced by ones over the
// fixture's files: headless Chromium can't show the system's sheet);
// Studio asks instead of opening the project with empty files; Open
// Folder... clicked — the folder picker, asked to start where the project
// file is, answers its folder; the project opens with its files and forms.
// Nothing on the page's console is an error.
async function webOpenPickedProject() {
  const name = "web-open-project-picked (web)";
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await ctx.newPage();
  const errors = [];
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  page.on("pageerror", (e) => errors.push(e.message));
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, test, ms = 15000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (test(v)) return v;
      await page.waitForTimeout(100);
    }
    return v;
  };
  // (a real click where the control is: its accessibility element's box)
  const click = async (sel) => {
    const b = await page.locator(sel).first().boundingBox();
    if (!b) throw new Error(`nothing at ${sel}`);
    await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
  };
  try {
    const files = Object.fromEntries(WEB_OPEN_FILES.map((f) => [f, readFileSync(join(ROOT, "tests/fixtures/studio_web_open", f), "utf8")]));
    await page.addInitScript(() => { window.RAPIDR_STUDIO_TEST = {}; window.RAPIDR_FILE_SINK = () => {}; });
    await page.addInitScript((files) => {
      const file = (name) => ({ kind: "file", name, async getFile() { return new File([files[name]], name); } });
      window.__rrPicked = [];
      window.showOpenFilePicker = async (opts) => { window.__rrPicked.push({ open: !!opts.multiple }); return [file("Kiosk.rrproj")]; };
      window.showDirectoryPicker = async (opts) => {
        window.__rrPicked.push({ folder: opts.startIn ? opts.startIn.name : "" });
        return { kind: "directory", name: "kiosk", async *values() { for (const n of Object.keys(files)) yield file(n); } };
      };
    }, files);
    await page.goto(`${URL_BASE}/index.html?${new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "" })}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && /^RapidR Studio$/.test(String(window.rr.rapidr_get_prop("studio", "caption"))), null, { timeout: 90000, polling: 200 });
    await page.waitForTimeout(1500);
    const t0 = Date.now();
    await click("#rr-startrow\\(1\\)");
    const shown = await until(() => prop("saveprompt", "visible"), (v) => /^(1|-1|true)$/i.test(v));
    const title = await prop("prompttitle", "caption");
    record(name, /^(1|-1|true)$/i.test(shown) && /^Open the folder Kiosk\.rrproj is in\?$/.test(title), `Open Project or File..., Kiosk.rrproj picked alone: Studio asks to open its folder ("${title}")`);
    record(name, (await prop("proj", "kind")) === "", "no project opened with empty files meanwhile");
    await page.screenshot({ path: join(WORK, "web-open-project-picked-ask.png") });
    await click("#rr-promptsave");
    const file = await until(() => prop("proj", "filename"), (v) => v === "kiosk/Kiosk.rrproj");
    const ms = Date.now() - t0;
    const picked = await page.evaluate(() => window.__rrPicked);
    record(name, picked.length === 2 && picked[1].folder === "Kiosk.rrproj", `Open Folder...: the folder picker starts where Kiosk.rrproj is (${JSON.stringify(picked)})`);
    const text = await until(() => prop("codedoc(0)", "text"), (v) => v.startsWith("' The web Studio's"));
    record(name, file === "kiosk/Kiosk.rrproj" && text.startsWith("' The web Studio's"), `the project opens from its folder with main.rr's code (${file || "none"}, ${ms} ms)`);
    record(name, (await prop("projecttree", "filecount")) === "3", "the tree lists its 3 files");
    const forms = await page.getByRole("treeitem", { name: /^(MainForm|NewForm|CfgForm), RForm$/ }).count();
    record(name, forms === 3, `ui.rr's three forms in the tree (${forms})`);
    await page.waitForTimeout(500);
    await page.screenshot({ path: join(WORK, "web-open-project-picked.png") });
    record(name, errors.length === 0, `no errors on the page's console (${errors.length ? errors.join("; ") : "none"})`);
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "web-open-project-picked".includes(f)))) {
  await webOpenPickedProject();
}
async function webColorScheme() {
  const name = "theme-web-color-scheme (web)";
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 }, colorScheme: "light" });
  const page = await ctx.newPage();
  const prop = (c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const until = async (what, want, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (v === want) return v;
      await page.waitForTimeout(150);
    }
    return v;
  };
  try {
    await page.addInitScript(() => { window.RAPIDR_STUDIO_TEST = {}; });
    const q = new URLSearchParams({ theme: "rapidr", window: "normal", fresh: "" });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("studio", "caption")) === "RapidR Studio", null, { timeout: 90000, polling: 200 });
    const look = () => prop("application", "theme");
    record(name, (await until(look, "rapidr light")) === "rapidr light", `a page in the light scheme opens Studio light (${await look()})`);
    for (const [media, want, page_, card] of [
      [{ colorScheme: "dark" }, "rapidr dark", "2627858", "3812133"],
      [{ forcedColors: "active" }, "rapidr high contrast", "0", "0"],
      [{ forcedColors: "none", colorScheme: "light" }, "rapidr light", "16777215", "16777215"],
    ]) {
      await page.emulateMedia(media);
      const got = await until(look, want);
      const colors = `${await prop("welcome", "color")}/${await prop("excard(0)", "color")}`;
      record(name, got === want && colors === `${page_}/${card}`, `the browser switched to ${JSON.stringify(media)}: Studio is ${got}, the Welcome page and its cards ${colors} (wanted ${page_}/${card}), no reload`);
      await page.waitForTimeout(400);
      if (media.forcedColors === "active") {
        // (forced colours — Windows' High Contrast —: the browser repaints every element in the system's colours unless told not to; the screen readers' mirror, laid over the canvas in transparent text, then showed every label twice, a serif face on top of the canvas's)
        const adjust = await page.evaluate(() => {
          const el = document.querySelector(".rr-a11y [role=button], .rr-a11y [role=menuitem], .rr-a11y [role=treeitem]");
          return el ? getComputedStyle(el).forcedColorAdjust + "/" + getComputedStyle(el).color : "no mirror element";
        });
        record(name, adjust === "none/rgba(0, 0, 0, 0)", `in forced colours the screen readers' mirror stays invisible (forced-color-adjust/colour: ${adjust})`);
      }
      await page.screenshot({ path: join(WORK, `theme-web-color-scheme-${want.replace(/ /g, "-")}.png`) });
    }
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  } finally {
    await ctx.close();
  }
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "theme-web-color-scheme".includes(f) || f === "theme-"))) {
  await webColorScheme();
}
if (HOSTS.includes("web") && (!filters.length || filters.some((f) => "beforeunload".includes(f)))) {
  await beforeUnload("changed", 'wait,code:"ONE"=>"ONE x"', true);
  await beforeUnload("saved", 'wait,code:"ONE"=>"ONE x",file.save', false);
}
// (a browser that died — the system short of memory — fails its cases, and
// the mock server still goes)
await browser.close().catch(() => {});
mockProc.kill();
console.log(`\nRapidR Studio flows: ${passed} checks passed, ${failed} failed`);
// (a filter that names no flow — a case's full name, `debug-events-web-nested`, is longer than the filter the web groups match, `debug-events-web` — ran nothing and said "0 passed, 0 failed", exit 0: a typo looked like a green run)
if (filters.length && passed + failed === 0) {
  console.error(`no flow matches ${JSON.stringify(filters)}: nothing ran`);
  process.exit(1);
}
process.exit(failed ? 1 : 0);
