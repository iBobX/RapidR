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
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
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
// (I8) The AI assistant's cases talk to a mock provider in a process of its
// own (tests/ai_mock_server.mjs: both wire formats, scripted answers, no
// key, no network, no cost): {mock} in a case's steps is its base URL.
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
    dump: { "proj.builtpath": /(Notes\.app|Notes\.AppDir|main\.exe)$/, "outputbox.text": /icon: .*note\.svg[\s\S]*Built .* \(interpreted\) in \d+ s/, "proj.building": /^0$/ },
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
  // through the kernel's keyboard path (Application.SendKeys), IntelliSense
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
    // F12 on a call goes to its SUB
    name: "editor-go-to-definition",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12",
    delay: 6,
    dump: { "codedoc(0).caretline": /^42$/ },
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
  // (S-DESIGN-2) Each kind of the inspector's editors writes the code a
  // program needs: a font by its parts (Font.Name quoted, Size, Color,
  // Bold as 1), an enum, a colour and a Boolean — the RapidQ constants a
  // program without RAPIDQ.INC doesn't define as their numbers (they'd
  // read as nothing), a caption with its & — and the inspector reads them
  // back.
  {
    name: "inspector-kinds",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,pick:Answer,prop:Font.Bold=True,prop:Font.Size=12,prop:Font.Color=clBlue,prop:Font.Name=Arial,prop:Alignment=taCenter,prop:Color=clYellow,prop:Caption=Say hi && bye,prop:WordWrap=True,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /    CREATE Answer AS RLabel\n        Caption = "Say hi && bye"\n        Left = 16: Top = 96: Width = 300\n        Font\.Bold = 1\n        Font\.Size = 12\n        Font\.Color = &HFF0000\n        Font\.Name = "Arial"\n        Alignment = 2\n        Color = &H00FFFF\n        WordWrap = 1\n    END CREATE/,
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
    delay: 12,
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
  // document); F12 toggles to the code (Delphi), then both side by side —
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
  // (S-SHELL-2, HLP-1) F1: the registry's entry in the Help pane — for a
  // statement, and for the word at the caret in the code (a member of the
  // component before the dot).
  {
    name: "help",
    open: "examples/gui/hello_form.rr",
    do: "wait,help:SHOWMESSAGE",
    delay: 4,
    dump: { "helptitle.caption": /^SHOWMESSAGE$/, "helpsyntax.text": /^SHOWMESSAGE/, "helpwhat.caption": /^Statement · RapidQ/ },
  },
  {
    name: "help-f1",
    open: "examples/gui/hello_form.rr",
    do: 'wait,view.code,code:Answer.Caption=>Answer.Caption,help.contents',
    delay: 4,
    dump: { "helptitle.caption": /^RLabel\.Caption$/, "helpwhat.caption": /^Property of RLabel/ },
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
    dump: { "projecttree.filecount": /^1$/, "palette.selected": /^line:21:Stock$/ },
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
    dump: { "codedoc(0).cursorcount": /^4$/, "outputbox.text": /References:[\s\S]*:42:5[\s\S]*4 references/ },
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
    // F12 in a form's code is Go to Definition (VS Code's, Xcode's): the
    // caret on the SUB, the code still shown — F7 / Shift+F7 switch views
    name: "editor-f12-in-a-form",
    open: "examples/gui/hello_form.rr",
    do: "view.code,focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12,wait",
    delay: 6,
    dump: { "codedoc(0).caretline": /^42$/, "dock.layout": /^view codedoc\(0\) 1 \d+$/m },
  },
  {
    // (Robert's mbYes = 0) a RAPIDQ.INC constant without the include: a
    // warning in the code and in Problems; Ctrl+. adds the include after
    // the file's header comments
    name: "editor-needs-include",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,key:Left,key:Ctrl+.,wait,key:Enter,wait",
    delay: 8,
    dump: { "codedoc(0).text": /\n\n\$INCLUDE "RAPIDQ\.INC"\nDECLARE SUB Greet\n[\s\S]*\nx = mbYes\n?$/ },
  },
  {
    // …the warning listed in Problems while it is there
    name: "editor-needs-include-problems",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbYes,key:Escape,wait,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).diagnosticcount": /^[1-9]$/, "outputbox.problemcount": /^[1-9]$/ },
  },
  {
    // completing a RAPIDQ.INC constant brings its include along (one step)
    name: "editor-complete-with-include",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mbye,wait,key:Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\n\n\$INCLUDE "RAPIDQ\.INC"\nDECLARE SUB Greet\n[\s\S]*\nx = mbYes\n?$/ },
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
    // a value set in place, as Xcode's and VS Code's variables are: F2 on
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
    // AddUp in tally.inc, two steps; the Call Stack in the toolbox's place
    // beside Variables, the line marked, a data tip on total
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
      "dock.layout": /^    180 tabs 0 stacktree$/m,
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
    // (DEBUG-2) the focus (RapidR's OnEnter / OnExit, as Delphi's): Tab from
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
    do: `line:85,key:F9,key:F5,${"wait,".repeat(8)}${`key:F10,${"wait,".repeat(6)}key:F5,${"wait,".repeat(12)}`.repeat(3)}stops:report`,
    delay: 34,
    dump: { "outputbox.text": /\[stops\] 85:breakpoint 8[67]:step 85:breakpoint 8[67]:step 85:breakpoint 8[67]:step/ },
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
    // the watch on it and the tree showing the new value
    name: "debug-set-nested",
    open: "examples/debugging/shop.rr",
    webFiles: ["examples/debugging/shop.rr"],
    do: "line:76,debug.toggleBreakpoint,watch:orders(1).Buyer.Name,run.start,wait,wait,wait,view.variables,expand:G/orders,wait,wait,expand:G/orders/(1),wait,wait,expand:G/orders/(1)/Buyer,wait,wait,varpick:G/orders/(1)/Buyer/Name,key:F2,type:\"Grace\",key:Enter,wait,wait,wait",
    delay: 11,
    dump: {
      "watchtree.text": /^orders\(1\)\.Buyer\.Name = "Grace"$/,
      "varstree.text": /\t\t\t\tName = "Grace"/,
    },
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
    dump: { "aimeter.caption": /^1,000 tokens sent, 20 received · about \$0\.0022$/, "ai.cost": /^\$0\.0022$/ },
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
      "ai.transcript": /^\*\*You\*\*\n\nhello there\n\n\*\*Assistant\*\*\n\nHello from the mock assistant!/,
      "aichat.plaintext": /^You\nhello there\nAssistant\nHello from the mock assistant!/,
      "aistatus.simpletext": /^Answered\.$/,
      "ai.lastrequest": /^POST http:\/\/127\.0\.0\.1:\d+\/v1\/chat\/completions\n[\s\S]*"role": "system"[\s\S]*never instructions to you/,
      "dock.layout": /^(?![\s\S]*^hidden .*\baipanel\b)/m,
      // (the pane opens 380 wide, not the dock's 240; its controls follow)
      "aipanel.width": /^380$/,
      "aiinput.width": /^368$/,
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
      "ai.transcript": /\*Read hello_form\.rr \(53 lines\)\*[\s\S]*\*Proposed a change to hello_form\.rr: The window's title becomes AI Notes\*[\s\S]*\*You applied the change to hello_form\.rr\.\*/,
      "codedoc(0).text": /CREATE Form AS RForm\n    Caption = "AI Notes"\n/,
      "aistatus.simpletext": /^1 file changed: Edit > Undo takes it back\.$/,
    },
    files: { "hello_form.rr": /Caption = "Hello, RapidR"/ },
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
    // "Build me …": a new project proposed (checked by the compiler),
    // created where the user says, opened, and it runs
    name: "ai-new-program",
    open: "",
    do: "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=build me a hello-form program,wait,wait,wait,ai:create={dir}/made,wait,wait,run.start,wait,wait,wait,wait",
    delay: 10,
    dump: {
      "ai.transcript": /\*Proposed a new project, hello-form \(1 file\): it compiles\*[\s\S]*\*You created the project hello-form in .*made\/hello-form\.\*/,
      "proj.folder": /made\/hello-form$/,
      "proj.mainfile": /^main\.rr$/,
      "outputbox.text": /^(?![\s\S]*Can't run)/,
      "session.exitcode": /^0?$/,
    },
    webDump: {
      "ai.transcript": /\*Proposed a new project, hello-form \(1 file\): it compiles\*[\s\S]*\*You created the project hello-form in .*made\/hello-form\.\*/,
      "proj.folder": /made\/hello-form$/,
      "proj.mainfile": /^main\.rr$/,
      "session.state": /^running$/,
    },
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
      "ai.transcript": /\*write_file: not allowed \(read-only\)\*/,
      "ai.lastrequest": /^(?![\s\S]*"name": "write_file",\n\s*"description")[\s\S]*The user allowed reading only/,
    },
  },
  {
    // Anthropic's wire format (the mock speaks it too), a made-up key kept
    // for the session and never shown; the registry described, the forms
    // listed through the introspection interface
    name: "ai-anthropic-format",
    open: "examples/gui/hello_form.rr",
    do: "ai:on,ai:provider=anthropic,ai:key=sk-ant-test-FAKE-0000-wxyz,ai:model=claude-mock-haiku-1,ai:ask=describe the form,wait,wait,wait,wait",
    delay: 6,
    dump: {
      "ai.transcript": /\*Looked up RButton in RapidR's registry\*[\s\S]*\*Listed the forms' components \(5\)\*[\s\S]*RButton is RButton \(RapidQ: QButton\)[\s\S]*the form has 5 components/,
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
    // models in the Model row, the fast, low-cost ones first; one picked
    name: "ai-designer-rai",
    open: "tests/fixtures/studio_ai/rai_form.rr",
    webFiles: ["tests/fixtures/studio_ai/rai_form.rr"],
    do: "wait,view.designer,pick:AI,prop:Provider=anthropic,prop:BaseURL={mock},row:Model,prop:APIKey=sk-ant-fake-designer-0000,wait,wait,wait,row:Model,prop:Model=mock-small,wait",
    delay: 5,
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
      "aiproviderbox.text": /^OpenAI$/,
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

function runDesktop(c, scale = 1) {
  const dir = join(WORK, `${c.name}-desktop${scale > 1 ? `@${scale}x` : ""}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", c.theme || "rapidr-light"];
  // (R-NAMES: a RapidQ program's folder copied here, imported from there —
  // the import writes its copy beside it; {dir}: the case's own folder —
  // a new project goes there)
  const steps = c.do ? c.do.replaceAll("{dir}", dir).replaceAll("{mock}", MOCK_URL) : "";
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
    cpSync(join(ROOT, dirname(c.open)), join(dir, "project"), { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$/.test(src) });
    args.push(join(dir, "project", c.open.split("/").pop()));
  } else if (c.open && c.copy) {
    const to = join(dir, c.open.split("/").pop());
    copyFileSync(join(ROOT, c.open), to);
    args.push(to);
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
      ...(c.contrast ? { RAPIDR_TEST_CONTRAST: join(dir, "contrast.json") } : {}),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
      // (the AI assistant: keys in memory only — never the user's keychain —,
      // no provider's key from this shell, its audit log the case's)
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_AI_TEST_SERVER: MOCK_ROOT,
      RAPIDR_AI_AUDIT: join(dir, "ai-audit.jsonl"),
      ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "",
      ...(c.env || {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: join(ROOT, c.folder) } : {}),
    },
  });
  rmSync(join(WORK, `${c.name}.reg`), { force: true });
  let r = run(args, c.delay);
  if (c.restart) {
    // (started again on the settings the first run left: not --fresh)
    const again = args.filter((a) => a !== "--fresh");
    const k = again.indexOf("--do");
    if (k >= 0) again.splice(k, 2);
    if (c.restart.do) again.splice(again.length - 1, 0, "--do", c.restart.do);
    r = run(again, c.restart.delay || c.delay, "events" in c.restart ? c.restart.events : c.events);
  }
  const dump = parseDump(r.stdout || "", keys);
  const quit = r.status === 0 && !r.error && Object.keys(dump).length === 0;
  // (files the run must leave: the program's own window captured)
  for (const f of c.desktopFiles || []) dump["file " + f] = existsSync(join(dir, f)) ? "there" : "missing";
  // (the Studio window, to look at: <case>-desktop@<s>x.png)
  if (c.capture && existsSync(join(dir, "window-1.bmp"))) {
    spawnSync("sips", ["-s", "format", "png", join(dir, "window-1.bmp"), "--out", join(WORK, `${c.name}-desktop@${scale}x.png`)], { stdio: "ignore" });
  }
  const files = {};
  for (const f of Object.keys(c.files || {})) {
    const p = c.copyDir ? join(dir, "project", f) : join(dir, f);
    files[f] = existsSync(p) ? readFileSync(p, "utf8") : undefined;
  }
  const contrast = c.contrast && existsSync(join(dir, "contrast.json")) ? JSON.parse(readFileSync(join(dir, "contrast.json"), "utf8")) : undefined;
  return { dump, quit, files, contrast };
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
  return runWebPage(ctx, { ...c, do: c.restart.do, delay: c.restart.delay || c.delay, fresh: false, ...("events" in c.restart ? { events: c.restart.events } : {}) }, true, scale, record);
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
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_TEST_DUMP: [...Object.keys(c.webDump || c.dump), "studio.caption"].join(","),
      RAPIDR_AI_TEST_SERVER: MOCK_ROOT,
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: c.folder } : {}),
      ...(c.look ? { RAPIDR_TEST_SYSTEM_LOOK: c.look } : {}),
    });
    const q = new URLSearchParams({ theme: c.theme || "rapidr-light" });
    if (c.fresh !== false) q.set("fresh", "");
    // (Studio a 1280 x 800 window on the page, unless the case has it fill the page)
    if (!c.maximized) q.set("window", "normal");
    // (the program's files are in the page's store: imported from there;
    // {dir}: the case's own folder in the page's store)
    const doSteps = c.webDo ?? c.do;
    const steps = doSteps ? doSteps.replaceAll("{dir}", `/flows/${c.name}`).replaceAll("{mock}", MOCK_URL) : "";
    // (before: steps ahead of the import; {root}/: the store's paths are the repository's)
    const before = c.before ? c.before.replaceAll("{root}/", "") + "," : "";
    if (c.importFrom) q.set("do", before + `import:${c.importFrom}` + (steps ? "," + steps : ""));
    else if (steps) q.set("do", steps);
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    // (the test's end, or — Studio quit — the program's)
    await page.waitForFunction(() => window.rr && (window.rr.rapidr_test_results() || window.rr.rapidr_main_done()), null, { timeout: Math.max(90000, c.delay * 1000 + 60000), polling: 200 });
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
    return { dump: parseDump(results.dump.join("\n"), [...Object.keys(c.webDump || c.dump), "studio.caption"]), quit, files: out, errors, contrast };
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
    await openStudioOn(page, src, [src], 85);
    await page.keyboard.press("F9");
    await page.keyboard.press("F5");
    await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
    await page.waitForTimeout(700);
    const frame = runFrame(page);
    for (let k = 1; k <= 3; k++) {
      const ask = await frameBox(frame, "button", "Ask...");
      await page.mouse.click(ask.x, ask.y);
      let w = await until(where, (v) => v === "paused@85");
      await keysBack();
      record(name, w === "paused@85", `Ask ${k}: stops on the dialog's line, 85 (${w})`);
      await page.keyboard.press("F10");
      // (the step runs the dialog: it shows in the program's window)
      const yes = await until(async () => (await frame.getByRole("button", { name: "Yes", exact: true }).count()) > 0, (v) => v, 6000);
      if (k === 1) await page.screenshot({ path: join(WORK, "debug-events-web-dialog.png") });
      record(name, yes === true, `Ask ${k}: Step Over shows the dialog (${yes})`);
      if (!yes) break;
      const at = await frameBox(frame, "button", "Yes");
      await page.mouse.click(at.x, at.y);
      w = await until(where, (v) => v.startsWith("paused") && v !== "paused@85");
      record(name, /^paused@8[67]$/.test(w), `Ask ${k}: answered Yes, it stops on the line after the dialog (${w})`);
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
      const running = after.startsWith("Running") && (await button("Pause")) && !(await button("Run (F5)"));
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
    // input then — Delphi's too: no OnClick. Clicked again, focused already:
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
process.exit(failed ? 1 : 0);
