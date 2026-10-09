// RapidR Studio's shell on both hosts (docs/ide-plan.md I1): the same
// bytecode run on the desktop's headless host (`rapidr run ide/studio.rr`,
// RAPIDR_CAPTURE) and on the web page (target/studio-web, the canvas host,
// the same test hooks), in every theme at 1× and 2×; each window's capture
// must be byte-identical and the accessibility trees equal.
//
//   tools/build_studio_web.sh                       (the page)
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_shell.mjs [filter…]
//
// STUDIO_WEB_URL (default http://127.0.0.1:18473/; RAPIDR_STUDIO_URL also read) is the page's server;
// RAPIDR_STUDIO_OUT (default tests/results/studio) gets each capture as
// <scene>-<theme>@<s>x-desktop.bmp / -web.bmp and a side-by-side
// <scene>-<theme>@<s>x.bmp (desktop | web) to look at; RAPIDR_STUDIO_SCALES
// ("1,2") and RAPIDR_STUDIO_THEMES limit the runs.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
// (STUDIO_WEB_URL: each lane serves its own build on its own port)
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const OUT = process.env.RAPIDR_STUDIO_OUT || join(ROOT, "tests", "results", "studio");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const SCALES = (process.env.RAPIDR_STUDIO_SCALES || "1,2").split(",").map(Number);
const THEMES = (process.env.RAPIDR_STUDIO_THEMES || "rapidr-light,rapidr-dark,rapidr-high-contrast,classic").split(",");
const filters = process.argv.slice(2);

// (S-DESIGN-2) Keys typed on the designer and clicks on its form, as in
// tests/studio_flows.mjs.
const KEYS = { Enter: "13", Escape: "27", Tab: "9", "Ctrl+O": "79_16" };
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
const at = (x, y, menu = 0) => `${x + 25}_${y + 54 + menu}`;
const click = (x, y, menu = 0) => `designdoc(0).__mousedown_${at(x, y, menu)},designdoc(0).__mouseup_${at(x, y, menu)}`;

// The scenes: what Studio opens.
const SCENES = [
  { name: "project", open: "examples/gui/hello_form.rr" },
  { name: "welcome", open: "" },
  // (Studio's own dialogs, in the chrome font: the window captured is the
  // dialog, the second shown)
  { name: "newproject", open: "", do: "file.newProject", window: 2 },
  // (the palette is over Studio's own window: RCOMMANDPALETTE)
  { name: "palette", open: "", do: "view.commandPalette" },
  // (S-PANELS: the panels on a form program — the project tree, the
  // toolbox, the inspector following the designer, the console)
  { name: "panels", open: "examples/gui/pantry.rr", do: "wait,view.designer,pick:AddBtn", delay: 6 },
  { name: "events", open: "examples/gui/pantry.rr", do: "wait,view.designer,pick:NameEdit,page:events", delay: 6 },
  { name: "toolbox-search", open: "examples/gui/pantry.rr", do: "wait,search:chart", delay: 5 },
  { name: "palette-line", open: "examples/gui/pantry.rr", do: "wait,palette::12", delay: 5 },
  // (I4) The designer: notepad.bas's form after its right edge was dragged,
  // a QBUTTON placed and moved — real input through the kernel (selection,
  // handles, anchor pins, the form's grips drawn)
  {
    name: "designer",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.place.QBUTTON",
    delay: 4,
    events: [
      "__mousedown_112_132", "__mouseup_112_132",
      "__mousedown_503_262", "__mousemove_533_262", "__mousemove_563_262", "__mouseup_563_262",
      "__mousedown_122_142", "__mousemove_142_162", "__mousemove_162_182", "__mouseup_162_182",
    ].map((e) => `designdoc(0).${e}`).join(","),
  },
  // (S-DESIGN-2) The menu editor on hello_form's new menu bar: File made,
  // its menu open, Open… being given its ShortCut
  { name: "designer-menu", open: "examples/gui/hello_form.rr", do: "wait,view.designer,designer.menuEditor", delay: 4, events: typed("&File{Enter}&Open...{Tab}{Ctrl+O}") },
  // (S-DESIGN-2) A component placed with the toolbox's tool: its 100 ms
  // settling over by the capture, on both hosts
  { name: "designer-drop", open: "examples/gui/hello_form.rr", do: "wait,view.designer,designer.place.QBUTTON", delay: 4, events: click(40, 120) },
  // (S-DESIGN-2) The Tab-order editor: the badges, GreetButton clicked first
  { name: "designer-taborder", open: "examples/gui/hello_form.rr", do: "wait,view.designer,designer.tabOrder", delay: 4, events: click(150, 60) },
  // (S-DESIGN-2) A caption edited in place (a slow click, then typing)
  { name: "designer-caption", open: "examples/gui/hello_form.rr", do: "wait,view.designer", delay: 4, events: `${click(150, 60)},${click(150, 60)},${typed("Say &hi")}` },
  // (S-DESIGN-2) Smart guides while Answer is held
  { name: "designer-guides", open: "examples/gui/hello_form.rr", do: "wait,view.designer", delay: 4, events: [`__mousedown_${at(100, 100)}`, `__mousemove_${at(104, 104)}`, `__mousemove_${at(103, 106)}`].map((e) => `designdoc(0).${e}`).join(",") },
  // (S-DESIGN-2) Zoomed to 150 %, notepad's dialogs in its tray, SaveDialog selected
  { name: "designer-zoom", open: "examples/rapidq/notepad.bas", do: "wait,view.designer,designer.zoomIn,designer.zoomIn,designer.zoomIn,pick:SaveDialog,wait", delay: 6 },
  // (S-DESIGN-2) A console program's designer: "Add a Form", then one added
  { name: "designer-empty", open: "examples/basics/hello.rr", do: "wait,view.designer", delay: 4 },
  { name: "designer-addform", open: "examples/basics/hello.rr", do: "wait,view.designer,project.addForm,designer.add.QBUTTON,wait", delay: 6 },
  // (the code editor with the completion list open and its docs beside it:
  // typed through the kernel's keys, S-EDITOR)
  { name: "editor", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:dim y as string,key:Escape,key:Enter,type:form.c", delay: 6 },
  // (S-DESIGN-2) A form added to a new program (Project > Add Form):
  // Form2.rr on its designer with an RLabel, an REdit and an RButton, the
  // button selected in the inspector; then the program's main file with the
  // $INCLUDE and Form1's handler that shows Form2. ({dir}: the same project
  // path on both hosts, under tests/results)
  { name: "addform-design", open: "", project: true, do: "newproject:gui|{dir}|Multi,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,tool:RLABEL,prop:Caption=Hello from Form2,tool:REDIT,prop:Text=Type here,tool:RBUTTON,prop:Caption=Close,wait", delay: 10 },
  { name: "addform-code", open: "", project: true, do: "newproject:gui|{dir}|Multi,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,tool:RLABEL,prop:Caption=Hello from Form2,tool:REDIT,prop:Text=Type here,tool:RBUTTON,prop:Caption=Close,event:OnClick,wait,type:Form2.Close,key:Escape,open:main.rr,wait,view.designer,wait,tool:RBUTTON,prop:Caption=Show Form2,event:OnClick,wait,type:Form2.Show,key:Escape,wait", delay: 12 },
  // (S-EDITOR) a misspelt member squiggled, in Problems too; a hover; the
  // signature after `(`; Tab at a line's start (blanks, never a glyph)
  { name: "editor-squiggle", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,view.problems", delay: 8 },
  { name: "editor-hover", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,type:ShowMessage \"Hi\",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait", delay: 7 },
  { name: "editor-signature", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,type:x$ = MID$(,wait", delay: 6 },
  { name: "editor-tab", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Tab,type:clicks = 0,key:Escape,key:Enter,key:Tab,key:Tab,type:x,key:Escape", delay: 6 },
  // (Robert's IntelliSense pass) F12 on a call: the caret on its SUB, the
  // code view kept; mbYes without RAPIDQ.INC squiggled and in Problems;
  // completing it with the include it brings; Ctrl+Space's list
  { name: "editor-f12", open: "examples/gui/hello_form.rr", do: "view.code,focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12,wait", delay: 6 },
  { name: "editor-needs-include", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:r = MessageDlg(\"Save?\",key:Comma,type: mtWarning,key:Comma,type: mbYes OR mbNo,key:Comma,type: 0),key:Escape,wait,wait,wait,view.problems", delay: 8 },
  { name: "editor-complete-include", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x = mby,wait", delay: 6 },
  { name: "editor-ctrl-space", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Tab,key:Ctrl+Space,wait", delay: 6 },
  // (S-SHELL-2) Documents as tabs: a form's file side by side (Design |
  // Code), another file in a second group on the right; Find in Files'
  // results; F1's Help pane.
  // (examples without $INCLUDE: on the web the language service and the
  // designer don't read includes from the page's store yet)
  {
    name: "workspace",
    open: "examples/gui/hello_form.rr",
    do: "wait,view:Split,open:menus.rr,view.splitVertically",
    delay: 5,
  },
  {
    name: "design-tab",
    open: "examples/gui/hello_form.rr",
    delay: 4,
  },
  {
    name: "search",
    open: "examples/gui/hello_form.rr",
    do: "wait,find:Greet,help:QBUTTON",
    delay: 5,
  },
];

mkdirSync(OUT, { recursive: true });
const scratch = join(OUT, ".work");
mkdirSync(scratch, { recursive: true });

// ---- BMPs ------------------------------------------------------------------
function readBmp(buf) {
  const off = buf.readUInt32LE(10), w = buf.readInt32LE(18), h0 = buf.readInt32LE(22), bpp = buf.readUInt16LE(28) / 8;
  const h = Math.abs(h0), stride = (w * bpp + 3) & ~3, px = Buffer.alloc(w * h * 3);
  for (let y = 0; y < h; y++) {
    const row = h0 > 0 ? h - 1 - y : y;
    for (let x = 0; x < w; x++) {
      const i = off + row * stride + x * bpp, o = (y * w + x) * 3;
      px[o] = buf[i + 2]; px[o + 1] = buf[i + 1]; px[o + 2] = buf[i];
    }
  }
  return { w, h, px };
}

function writeBmp(img) {
  const stride = (img.w * 3 + 3) & ~3, size = 54 + stride * img.h, b = Buffer.alloc(size);
  b.write("BM", 0); b.writeUInt32LE(size, 2); b.writeUInt32LE(54, 10); b.writeUInt32LE(40, 14);
  b.writeInt32LE(img.w, 18); b.writeInt32LE(img.h, 22); b.writeUInt16LE(1, 26); b.writeUInt16LE(24, 28);
  for (let y = 0; y < img.h; y++) for (let x = 0; x < img.w; x++) {
    const o = (y * img.w + x) * 3, i = 54 + (img.h - 1 - y) * stride + x * 3;
    b[i] = img.px[o + 2]; b[i + 1] = img.px[o + 1]; b[i + 2] = img.px[o];
  }
  return b;
}

function sideBySide(a, b, gap = 16) {
  const w = a.w + gap + b.w, h = Math.max(a.h, b.h), px = Buffer.alloc(w * h * 3, 0x80);
  for (const [img, x0] of [[a, 0], [b, a.w + gap]]) for (let y = 0; y < img.h; y++) img.px.copy(px, (y * w + x0) * 3, y * img.w * 3, (y + 1) * img.w * 3);
  return { w, h, px };
}

function diff(a, b) {
  if (a.w !== b.w || a.h !== b.h) return `${a.w}×${a.h} against ${b.w}×${b.h}`;
  let n = 0;
  for (let i = 0; i < a.px.length; i += 3) if (a.px[i] !== b.px[i] || a.px[i + 1] !== b.px[i + 1] || a.px[i + 2] !== b.px[i + 2]) n++;
  return `${n} pixels differ`;
}

// (a scene's project folder, relative to the repository: the same on both
// hosts, so what Studio says about its files is the same)
const projectDir = (scene) => `tests/results/studio-projects/${scene.name}`;
const sceneDo = (scene) => scene.do.replaceAll("{dir}", projectDir(scene));

// ---- the desktop -------------------------------------------------------------
function runDesktop(scene, theme, scale) {
  const dir = join(scratch, `${scene.name}-${theme}@${scale}x`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", theme];
  if (scene.open) args.push(scene.open);
  // (a new project: made afresh each run, at the path the web uses too)
  if (scene.project) rmSync(join(ROOT, projectDir(scene)), { recursive: true, force: true });
  if (scene.do) args.push("--do", sceneDo(scene));
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 60000,
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      ...(scene.delay ? { RAPIDR_CAPTURE_DELAY: String(scene.delay) } : {}),
      ...(scene.events ? { RAPIDR_TEST_EVENTS: scene.events } : {}),
      RAPIDR_SCALE: String(scale),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_A11Y: join(dir, "a11y.json"),
      RAPIDR_PRINT_TO: join(scratch, "prints"),
      RAPIDR_REGISTRY: join(scratch, "registry.reg"),
    },
  });
  const file = join(dir, `window-${scene.window || 1}.bmp`);
  if (!existsSync(file)) throw new Error(`desktop: no capture (${(r.stderr || "").trim().split("\n").pop()})`);
  return { bmp: readFileSync(file), a11y: existsSync(join(dir, "a11y.json")) ? readFileSync(join(dir, "a11y.json"), "utf8") : null };
}

// ---- the web -------------------------------------------------------------------
async function runWeb(browser, scene, theme, scale) {
  const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = (scene.webFiles || []).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      ...(scene.delay ? { RAPIDR_CAPTURE_DELAY: String(scene.delay) } : {}),
      ...(scene.events ? { RAPIDR_TEST_EVENTS: scene.events } : {}),
    });
    const q = new URLSearchParams({ theme, window: "normal", fresh: "" });
    if (scene.open) q.set("open", scene.open);
    if (scene.do) q.set("do", sceneDo(scene));
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 60000, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { results, errors };
  } finally {
    await page.close();
  }
}

// ---- the runs ------------------------------------------------------------------
const browser = await chromium.launch();
let same = 0, differ = 0, failed = 0, a11ySame = 0, a11yDiffer = 0;
for (const scene of SCENES) for (const theme of THEMES) for (const scale of SCALES) {
  const name = `${scene.name}-${theme}@${scale}x`;
  if (filters.length && !filters.some((f) => name.includes(f))) continue;
  try {
    const desk = runDesktop(scene, theme, scale);
    const web = await runWeb(browser, scene, theme, scale);
    if (!web.results || !web.results.captures.length) throw new Error(`web: no capture ${web.errors.join("; ")}`);
    const shot = web.results.captures[(scene.window || 1) - 1];
    if (!shot) throw new Error(`web: no capture of window ${scene.window} (${web.results.captures.length} captured)`);
    const wb = Buffer.from(shot.bmp, "base64");
    const d = readBmp(desk.bmp), w = readBmp(wb);
    writeFileSync(join(OUT, `${name}-desktop.bmp`), desk.bmp);
    writeFileSync(join(OUT, `${name}-web.bmp`), wb);
    writeFileSync(join(OUT, `${name}.bmp`), writeBmp(sideBySide(d, w)));
    const identical = desk.bmp.equals(wb);
    identical ? same++ : differ++;
    console.log(`${identical ? "✓" : "≠"} ${name}: ${identical ? "byte-identical" : diff(d, w)}${web.errors.length ? ` (page errors: ${web.errors.join("; ")})` : ""}`);
    if (desk.a11y !== null) {
      const eq = desk.a11y === web.results.a11y;
      eq ? a11ySame++ : a11yDiffer++;
      if (!eq) {
        writeFileSync(join(OUT, `${name}-desktop.a11y.json`), desk.a11y);
        writeFileSync(join(OUT, `${name}-web.a11y.json`), web.results.a11y);
      }
      console.log(`${eq ? "✓" : "≠"} ${name}: accessibility trees ${eq ? "equal" : "differ"}`);
    }
  } catch (e) {
    failed++;
    console.log(`✗ ${name}: ${e.message}`);
  }
}
await browser.close();
console.log(`\nRapidR Studio shell: ${same} captures byte-identical desktop / web, ${differ} differ, ${failed} failed; accessibility trees equal ${a11ySame}, differ ${a11yDiffer} (captures in ${OUT})`);
process.exit(differ || failed || a11yDiffer ? 1 : 0);
