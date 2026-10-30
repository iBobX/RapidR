// RapidR Studio in your language (lane I18N-STUDIO; docs/manual/studio-languages.md):
// Studio's main scenes in the pseudo-language `xx` — every text a catalog
// translates shows accented and a third longer ("[Šàvé ÿöûŕ ŵöŕķ·····]") —
// on the desktop's headless host and the web page, judged from the very
// display list captured (rapidr_ui_kernel::contrast's `texts`, the
// desktop's RAPIDR_TEST_CONTRAST, the page's test results). A scene FAILS on
//   (i)  English left: a visible text with words that are neither
//        translated (no pseudo letter) nor a name — a registry name
//        (RButton, Caption, bsSizeable …), RapidR's product words, a file
//        name, or a word of the program the scene opened;
//   (ii) text that doesn't fit: wider than the box it is laid out in, cut
//        by a clip, or a translated text shortened with "…" (it lost its
//        closing "]").
// Then the language switch, live: Spanish (the ide/lang/es.po sample) from
// the start and by Settings' way (lang:es) — the menu bar in Spanish —, and
// back to English; a compiler error in Problems in Spanish.
//
//   rapidr i18n studio                    (writes ide/lang/xx.po; this test runs it)
//   tools/build_studio_web.sh             (the page, with ide/lang)
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_lang.mjs [filter…]
//
// STUDIO_WEB_URL (default http://127.0.0.1:18473/): the page's server; the
// web runs are skipped when nothing answers there (RAPIDR_LANG_WEB=0 skips
// them always). RAPIDR_LANG_THEMES: "rapidr-light" (default) or a list
// (rapidr-light,rapidr-dark,rapidr-high-contrast: the real walk's
// captures). RAPIDR_LANG_OUT (default tests/results/studio-lang): each
// capture as <scene>-<theme>-<host>.bmp. RAPIDR_LANG_STRICT_CUT=0 reports
// cut-off text without failing.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
// (SEC-24: a release build's test hooks need the test-mode token: tests/test_mode.mjs)
import "./test_mode.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const OUT = process.env.RAPIDR_LANG_OUT || join(ROOT, "tests", "results", "studio-lang");
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const THEMES = (process.env.RAPIDR_LANG_THEMES || "rapidr-light").split(",");
const STRICT_CUT = process.env.RAPIDR_LANG_STRICT_CUT !== "0";
const filters = process.argv.slice(2);
const want = (name) => !filters.length || filters.some((f) => name.includes(f));
mkdirSync(OUT, { recursive: true });
const scratch = join(OUT, "runs");
mkdirSync(scratch, { recursive: true });

// ---- the catalogs: the pseudo-language made from Studio's template ----------
{
  const r = spawnSync(RAPIDR, ["i18n", "studio"], { cwd: ROOT, encoding: "utf8" });
  if (!existsSync(join(ROOT, "ide/lang/xx.po"))) throw new Error(`rapidr i18n studio made no ide/lang/xx.po: ${r.stdout}${r.stderr}`);
}

// ---- what may stay English: names --------------------------------------------
// (RapidR's product words, the keys' names, the code's file types)
const NAMES = new Set(["true", "false", "rapidr", "studio", "rai", "basic", "utf", "crlf", "lf", "cr", "inter", "ms", "sans", "serif", "jetbrains", "mono", "courier", "new", "arial", "rapidq", "github", "anthropic", "claude", "openai", "ollama", "deepseek", "mcp", "api", "csv", "json", "sql", "url", "ctrl", "shift", "alt", "cmd", "enter", "esc", "tab", "del", "rr", "bas", "inc", "rrproj"]);
// (the project's Properties rows are names, like a component's: Theme, Snap, Guides …; "Aa" is the case toggle's icon)
for (const n of ["aa", "ab", "snap", "guides", "grid", "arguments", "targets", "desktop", "web"]) NAMES.add(n);
// (a language is listed by its own name: English, Español; a provider's label says "OpenAI-compatible")
for (const n of ["english", "compatible", "xai", "lm"]) NAMES.add(n);
// (the project the external-changes scene opens is named Outside: a name, as a file's)
NAMES.add("outside");
// (the font dialog lists faces by their own names, and shows a sample text of letters)
for (const n of ["georgia", "tahoma", "times", "roman", "trebuchet", "verdana", "aabbyyzz"]) NAMES.add(n);
{
  const r = spawnSync(RAPIDR, ["lang", "export", "--json"], { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 28 });
  const walk = (v) => {
    // (a constant group's [name, value] pairs)
    if (Array.isArray(v) && v.length === 2 && typeof v[0] === "string" && typeof v[1] === "number") NAMES.add(v[0].toLowerCase());
    if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === "object") {
      for (const [k, x] of Object.entries(v)) {
        if ((k === "name" || k === "display" || k === "rapidq") && typeof x === "string") {
          for (const w of x.split(/[^A-Za-z0-9_$]+/)) if (w) NAMES.add(w.toLowerCase());
        } else walk(x);
      }
    }
  };
  walk(JSON.parse(r.stdout));
}
const PSEUDO = /[àçðéĝĥíĵķĺñöþŕšţûŵÿžÀÇÐÉĜĤÍĴĶĹÑÖÞŔŠŢÛŴÝŽ·]/;
// A text that is plain English: a word of two letters or more that is no
// name, no file's and no word of the scene's own program.
function english(text, programWords) {
  if (PSEUDO.test(text)) return false;
  if (text.startsWith("> ")) return false; // (a --do step's echo in Output)
  // (file names and paths are the user's: hello_form.rr, examples/gui)
  const plain = text.replace(/[\w./-]+\.(rr|bas|inc|rrproj|rqw|md|po|csv|json|txt|png|rtf)\b/gi, " ").replace(/\S*[/\\]\S*/g, " ");
  const words = plain.match(/[A-Za-z][A-Za-z'’]+…?/g) || [];
  return words.some((w) => {
    // (a name shortened by the inspector: "clBtnF…")
    if (w.endsWith("…")) {
      const head = w.slice(0, -1).toLowerCase();
      return ![...NAMES].some((n) => n.startsWith(head)) && ![...programWords].some((n) => n.startsWith(head));
    }
    const k = w.toLowerCase().replace(/['’]s$/, "");
    return !NAMES.has(k) && !programWords.has(k) && !/^[a-z]+_[a-z_]+$/.test(w);
  });
}
// What is left, known and said in the manual: the Project pane's title
// squeezed beside Search's at the left group's default width (the dock
// trims the longest tab first; the whole title is its tooltip and its
// accessible name).
const KNOWN_LEFT = [
  /^\[Þŕöĵéçţ·*…$/,
  // (the Properties tab beside Help's, the left group's tabs beside Versions': the dock trims the longest title first)
  /^\[Þŕöþéŕţíéš·*…$/, /^\[Þŕ…$/, /^\[Šé…$/, /^\[Vé…$/,
  // (the debugging group's tabs — Variables, Watch, Call Stack, Breakpoints — beside each other: the dock trims the longest titles first, the whole of them their tooltips)
  /^\[Vàŕíàbĺéš·*…$/, /^\[Çàĺĺ Šţàçķ·*…$/, /^\[Bŕéàķþöíñţ·*…$/,
  // (the description under the project's rows, longer than its two lines)
  /^RapidR föĺĺöŵš/,
];
// A translated text shortened ("[Šàvé ÿö…"), cut by the layout.
const shortened = (text) => PSEUDO.test(text) && text.includes("…") && !/\][^[]*$/.test(text);
function wordsOf(file) {
  const out = new Set();
  if (!file) return out;
  const text = readFileSync(file.startsWith("/") ? file : join(ROOT, file), "utf8");
  for (const w of text.match(/[A-Za-z][A-Za-z']+/g) || []) out.add(w.toLowerCase().replace(/'s$/, ""));
  // (its file name, as the tree and the tabs show it)
  for (const w of file.split(/[/.]/)) out.add(w.toLowerCase());
  return out;
}

// ---- the scenes ----------------------------------------------------------------
const BAD = join(ROOT, "tests/results/studio-lang-bad/bad.rr");
mkdirSync(dirname(BAD), { recursive: true });
writeFileSync(BAD, '\' a program with a mistake: Problems says it\nDIM b AS RButton\nPRINT "x" THEN\n');
// (a project the scenes change — Versions starts a repository in it — is a copy)
const VERSIONS_COPY = { copy: "tests/fixtures/studio_versions", copyOpen: "Hello.rrproj" };
const V_START = "versions.pane,versions.start,versions:author=Ana,versions:save=first";
const HELP_OPEN = "tests/fixtures/studio_help/main.rr";
const HELPERS = "examples/studio/helpers.rr";
const HELPERS_READY = "focus:codedoc(0),wait,wait,wait,wait";
const HELLO = "examples/gui/hello_form.rr";
const DEBUG = "tests/fixtures/studio_debug/counter.rr";
const CRASH = "tests/fixtures/studio_lang/crash.rr";
const DIALOGS = "tests/fixtures/studio_lang/dialogs.rr";
const NOTES = "tests/fixtures/studio_lang/notes.rr";
const EXT_MAIN = readFileSync(join(ROOT, "tests/fixtures/studio_external/main.rr"), "utf8");
// (a message box Studio shows is waited for as the user's would be, then answered OK)
const MSG_HOLD = { RAPIDR_TEST_DIALOG_HOLD: "9000", RAPIDR_TEST_MESSAGE_DIALOG: "OK" };
// A dialog's capture (window 2) or a bar's (window 1).
function SWEEP_SCENES() {
  return [
    // (the dialogs and bars)
    { name: "keyboard-shortcuts", open: "", do: "view.keyboardShortcuts", window: 2, delay: 5, env: MSG_HOLD },
    { name: "about", open: "", do: "help.about", window: 2, delay: 5, env: MSG_HOLD },
    { name: "goto-line", open: HELLO, do: "wait,view.code,wait,edit.goToLine", delay: 5 },
    { name: "replace-bar", open: HELLO, do: "wait,view.code,wait,edit.replace", delay: 5 },
    { name: "find-bar", open: HELLO, do: "wait,view.code,wait,edit.find", delay: 5 },
    { name: "quick-open", open: HELLO, do: "view.quickOpen", delay: 5 },
    { name: "setup-rai-cloud", open: "", do: "ai.setup,ai:setupcloud,ai:setupnext", window: 2 },
    { name: "setup-rai-local", open: "", do: "ai.setup,ai:setuplocal,ai:setupnext", window: 2 },
    { name: "settings-aiapps", open: "", do: "mcp.settings", window: 2, env: { RAPIDR_TEST_MCP_TOKEN: "lang-test" }, desktopOnly: "AI apps (MCP) are the desktop Studio's" },
      { name: "project-options", open: HELLO, do: "wait,project.options", window: 2 },
    // (a palette row shows a file's description in one line, shortened by design)
    { name: "open-example", open: "", do: "file.openExample", cutOk: [/^\S+\.rr - \[/] },
    { name: "menu-editor", open: "examples/gui/menus.rr", do: "wait,view.designer,wait,designer.menuEditor", delay: 6 },
    { name: "tab-order", open: HELLO, do: "wait,view.designer,wait,designer.tabOrder", delay: 6 },
    // (the context menus: a right click where the person makes it)
    { name: "ctx-code", open: HELLO, do: "wait,view.code,wait", events: "studio.__wait_3500,codedoc(0).__rightclick_220_160", delay: 7 },
    // (the toolbox's and the Properties pane's own menu is the edit menu of the search box they hold: the project tree, the designer and the rows have none)
    { name: "ctx-toolbox", open: HELLO, do: "wait,view.designer", events: "studio.__wait_3000,toolbox.__rightclick_60_22", delay: 6 },
    { name: "ctx-properties", open: HELLO, do: "wait,view.designer", events: "studio.__wait_3000,inspector.__rightclick_100_72", delay: 6 },
    { name: "ctx-output", open: HELLO, do: "wait", events: "studio.__wait_3000,outputbox.__rightclick_100_80", delay: 6 },
    { name: "ctx-immediate", open: HELLO, do: "wait,view.immediate", events: "studio.__wait_3000,immediatebox.__rightclick_100_40", delay: 6 },
    // (a program stopped at a breakpoint: Variables, Watch, Call Stack, Breakpoints)
    { name: "debug-paused", open: DEBUG, do: "line:6,debug.toggleBreakpoint,watch:k,run.start,wait,wait,wait,wait", delay: 11 },
    { name: "ctx-breakpoints", open: DEBUG, do: "line:6,debug.toggleBreakpoint,view.breakpoints,wait", events: "studio.__wait_4500,bptree.__rightclick_60_20", delay: 9 },
    { name: "ctx-variables", open: DEBUG, do: "line:6,debug.toggleBreakpoint,watch:k,run.start,wait,wait,wait,wait", events: "studio.__wait_9000,varstree.__rightclick_60_20", delay: 13 },
    { name: "ctx-watch", open: DEBUG, do: "line:6,debug.toggleBreakpoint,watch:k,run.start,wait,wait,wait,wait,view.watch", events: "studio.__wait_9000,watchtree.__rightclick_60_20", delay: 13 },
    { name: "ctx-call-stack", open: DEBUG, do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait,view.callStack", events: "studio.__wait_9000,stacktree.__rightclick_60_20", delay: 13 },
    // (the dialogs a program shows — the colour dialog, with and without its
    // editor, the font dialog, a message box —: the very program run in each
    // language, its dialogs held open and then answered)
    ...[
      ["colour-dialog", "colorbutton.onclick,colorbutton.__wait_500", { RAPIDR_TEST_COLOR_DIALOG: "255;" }],
      ["colour-dialog-editor", "fullbutton.onclick,fullbutton.__wait_500", { RAPIDR_TEST_COLOR_DIALOG: "255;" }],
      ["font-dialog", "fontbutton.onclick,fontbutton.__wait_500", { RAPIDR_TEST_FONT_DIALOG: ";" }],
      ["message-box", "askbutton.onclick,askbutton.__wait_500", { RAPIDR_TEST_MESSAGE_DIALOG: "Cancel" }],
    ].map(([name, events, env]) => ({ name, program: DIALOGS, events, env: { RAPIDR_TEST_DIALOG_HOLD: "9000", ...env }, window: 2, delay: 3 })),
    // (the language service's words: a note's quick fix, a rename that can't be done)
    // (Output's first line is half scrolled out: it is Studio's history, in the language it was written in)
    { name: "quick-fix", open: NOTES, do: "wait,view.code,wait,focus:codedoc(0),line:14,key:End,edit.quickFix,wait", delay: 8, cutOk: [/^\[RapidR Šţûðíö íš ŕéàðÿ/] },
    { name: "rename-error", open: HELLO, do: "wait,view.code,wait,focus:codedoc(0),line:14,key:Home,key:Right,key:Right,key:Right,key:Right,key:Right,key:F2,wait,type:PRINT,key:Enter,wait", delay: 9 },
    // (the external-changes bar: a file changed outside Studio while it has unsaved changes here)
    { name: "external-bar", open: "", external: { dir: "tests/fixtures/studio_external", open: "Outside.rrproj", steps: [{ write: { "main.rr": EXT_MAIN.replace('PRINT "Done."', 'PRINT "Changed by an agent."') } }] }, do: 'code:PRINT "Done."=>PRINT "Mine.",ext:go=1,ext:await=1,ext:check,wait,wait', delay: 9, desktopOnly: "the web has no disk to watch", cutOk: [/^\[Öþéñéð \//, /\.rr·*\]?$/] },
    // (a tooltip: the pointer rests on a tool bar button, then on the dock's Auto Hide button)
    { name: "tooltip-toolbar", open: HELLO, do: "wait", events: "studio.__wait_3000,toolbar.__mousemove_150_20,toolbar.__mousemove_151_20,toolbar.__wait_2500", delay: 8 },
    { name: "tooltip-dock", open: HELLO, do: "wait", events: "studio.__wait_3000,dock.__mousemove_997_515,dock.__mousemove_998_515,dock.__wait_2500", delay: 8 },
    // (Versions: what changed, the local history)
    // (the Versions pane's own "What Changed?" button and the project's path are not these scenes'; the history's one-line description is as long as its words)
    { name: "versions-changes", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,versions.changes`, delay: 9, cutOk: [/^\[Ŵĥàţ Cĥàñĝéð/, /\/project /, /^[\s·\]]+$/, /·+\]$/], engOk: [/^' Hello: the Versions walk/] },
    { name: "versions-history", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,versions.localHistory`, delay: 9, cutOk: [/^\[Ŵĥàţ Cĥàñĝéð/, /\/project /, /^[\s·\]]+$/, /·+\]$/, /^\[À çöþÿ éàçĥ/, /^' Hello: the Versions walk/], engOk: [/^' Hello: the Versions walk/] },
    // (Settings' lists, one opened on each page)
    ...[["general-language", "view.settings", "languagebox"], ["general-helpers", "view.settings", "helpersbox"], ["general-debug-panes", "view.settings", "debugpanesbox"], ["ai-provider", "ai.settings", "aiprovbox"], ["ai-permission", "ai.settings", "aipermbox"], ["ai-reply", "ai.settings", "aireplybox"]].map(([n, cmd, box]) => ({ name: `settings-open-${n}`, open: "", do: cmd, events: `${box}.__mousedown_40_10,${box}.__mouseup_40_10`, window: 2, delay: 6 })),
    // (a run-time error: the stop, the line in Output, the strip at the line)
    { name: "run-error", open: CRASH, do: "run.start,wait,wait,wait,wait", delay: 11 },
  ];
}
const XX_SCENES = [
  { name: "main", open: "" },
  { name: "project", open: "examples/gui/hello_form.rr", do: "wait,view.designer", delay: 6 },
  { name: "settings", open: "", do: "view.settings", window: 2 },
  { name: "setup-rai", open: "", do: "ai.setup", window: 2 },
  // (Settings > AI, with "RAI answers in:": ES-REFERENCE part 3)
  { name: "settings-ai", open: "", do: "ai.settings", window: 2 },
  // (the first-time question AI apps ask: Studio's test token lets a step ask it)
  { name: "consent", open: "", do: "wait,mcp:consent", window: 2, env: { RAPIDR_TEST_MCP_TOKEN: "lang-test" }, desktopOnly: "AI apps (MCP) are the desktop Studio's" },
  { name: "find", open: "examples/gui/hello_form.rr", do: "wait,edit.findInFiles", delay: 5 },
  { name: "problems", open: BAD, webOpen: "tests/results/studio-lang-bad/bad.rr", do: "wait,view.problems", delay: 5 },
  { name: "palette", open: "", do: "view.commandPalette" },
  // (ES-REFERENCE part 2: the words Rust writes around the docs — the Help
  // pane's start page and its page for a word nothing knows, the code
  // editor's change box, the Theme values in Properties. The pages that
  // carry the registry's docs are judged in Spanish below: the docs have no
  // pseudo-language)
  { name: "help-start", open: HELP_OPEN, do: "wait,view.code,wait,help:", delay: 5 },
  { name: "help-unknown", open: HELP_OPEN, do: "wait,help:RapidR", delay: 5 },
  // (the Versions pane's own "What Changed?" button and the project's path are not this scene's)
  { name: "change-box", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,line:12,versions.showChange`, delay: 8, cutOk: [/^\[Ŵĥàţ Cĥàñĝéð/, /\/project /, /^[\s·\]]+$/, /·+\]$/] },
  { name: "project-theme", open: "examples/gui/hello_form.rr", do: "wait,view.designer,project:select,row:Theme,wait", delay: 6, cutOk: [/^šÿšţém, öŕ çĥööšé/] },
  // (ES-REFERENCE part 2b: the sweep — every dialog, bar, page and menu
  // Studio has, in both languages: no English left, nothing cut off)
  ...SWEEP_SCENES(),
];

function runDesktop(scene, lang, theme) {
  const dir = join(scratch, `${scene.name}-${lang}-${theme}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = scene.program ? ["run", scene.program] : ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", theme, "--lang", lang];
  if (scene.do && !scene.program) args.push("--do", scene.do);
  let writer = null;
  if (scene.external) {
    // (the external-changes bar: a copy of the project, its files changed by
    // another process while Studio has them open — the writer's steps are
    // Studio's own `ext:go=n` / `ext:await=n`)
    cpSync(join(ROOT, scene.external.dir), join(dir, "project"), { recursive: true });
    const meet = join(dir, "meet");
    mkdirSync(meet, { recursive: true });
    writeFileSync(join(dir, "external.json"), JSON.stringify(scene.external.steps));
    writer = spawn(process.execPath, [join(HERE, "ext_writer.mjs"), meet, join(dir, "project"), join(dir, "external.json")], { stdio: "ignore" });
    scene.env = { ...(scene.env || {}), RAPIDR_TEST_EXT: meet };
    args.push(join(dir, "project", scene.external.open));
  } else if (scene.copy) {
    // (a project the scene changes — Versions starts a repository in it —
    // is a copy beside the capture, never in this checkout)
    cpSync(join(ROOT, scene.copy), join(dir, "project"), { recursive: true });
    args.push(join(dir, "project", scene.copyOpen));
  } else if (scene.open) args.push(scene.open);
  spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 90000,
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(scene.delay || 4),
      RAPIDR_SCALE: "1",
      RAPIDR_MENU: "window",
      RAPIDR_TEST_CONTRAST: join(dir, "contrast.json"),
      RAPIDR_TEST_A11Y: join(dir, "a11y.json"),
      RAPIDR_PRINT_TO: join(dir, "prints"),
      RAPIDR_REGISTRY: join(dir, "registry.reg"),
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_KEYSTORE: "memory",
      RAPIDR_AI_CHATS: join(dir, "chats"),
      RAPIDR_DOCUMENTS: join(dir, "documents"),
      // (a program of its own speaks the language the environment says)
      ...(scene.program ? { RAPIDR_LANG: lang } : {}),
      ...(scene.events ? { RAPIDR_TEST_EVENTS: scene.events } : {}),
      ...(scene.env || {}),
    },
  });
  if (writer && writer.exitCode === null) writer.kill();
  const file = join(dir, `window-${scene.window || 1}.bmp`);
  if (!existsSync(file)) throw new Error("desktop: no capture");
  const report = existsSync(join(dir, "contrast.json")) ? JSON.parse(readFileSync(join(dir, "contrast.json"), "utf8")) : [];
  const a11y = existsSync(join(dir, "a11y.json")) ? readFileSync(join(dir, "a11y.json"), "utf8") : "";
  return { bmp: readFileSync(file), report, windows: report.length, a11y };
}

let browser = null;
async function webUp() {
  if (process.env.RAPIDR_LANG_WEB === "0") return false;
  try {
    const r = await fetch(`${URL_BASE}/ide/lang/index.txt`);
    return r.ok;
  } catch {
    return false;
  }
}
async function runWeb(scene, lang, theme) {
  if (!browser) browser = await (await import("playwright")).chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1, viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = scene.open && !scene.open.startsWith("examples/") ? [{ path: scene.webOpen || scene.open, text: readFileSync(scene.open, "utf8") }] : [];
    await page.addInitScript((f) => { window.RAPIDR_STUDIO_TEST_FILES = f; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, { RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: String(scene.delay || 4), ...(scene.env || {}) });
    const q = new URLSearchParams({ theme, window: "normal", fresh: "", lang });
    if (scene.open) q.set("open", scene.webOpen || scene.open);
    if (scene.do) q.set("do", scene.do);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 90000, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    const shot = results.captures[(scene.window || 1) - 1];
    if (!shot) throw new Error(`web: no capture of window ${scene.window || 1} ${errors.join("; ")}`);
    return { bmp: Buffer.from(shot.bmp, "base64"), report: JSON.parse(results.contrast), windows: results.captures.length, a11y: results.a11y || "" };
  } finally {
    await page.close();
  }
}

// The window a scene is about (its capture), and what its texts say.
function judge(run, scene, programWords) {
  const w = run.report[(scene.window || 1) - 1] || run.report[0];
  const texts = (w && w.texts) || [];
  const eng = [...new Set(texts.filter((t) => english(t.text, programWords) && !(scene.engOk || []).some((k) => k.test(t.text))).map((t) => t.text))];
  const cut = [...new Set(texts.filter((t) => (t.cut && PSEUDO.test(t.text)) || shortened(t.text)).map((t) => t.text))].filter((t) => !KNOWN_LEFT.some((k) => k.test(t)) && !(scene.cutOk || []).some((k) => k.test(t)));
  return { form: w ? w.form : "?", count: texts.length, eng, cut, texts };
}

let failed = 0, passed = 0;
const left = [];
function check(name, ok, detail = "") {
  ok ? passed++ : failed++;
  console.log(`${ok ? "✓" : "✗"} ${name}${detail ? `\n    ${detail}` : ""}`);
}

const web = await webUp();
if (!web) console.log(`· the web page isn't served at ${URL_BASE} (tools/build_studio_web.sh, then serve it): desktop only`);
const hosts = web ? ["desktop", "web"] : ["desktop"];

for (const scene of XX_SCENES) for (const theme of THEMES) for (const host of hosts) {
  const name = `xx ${scene.name} ${theme} ${host}`;
  if (!want(name) || (host === "web" && scene.desktopOnly)) continue;
  try {
    const run = host === "desktop" ? runDesktop(scene, "xx", theme) : await runWeb(scene, "xx", theme);
    writeFileSync(join(OUT, `${scene.name}-xx-${theme}-${host}.bmp`), run.bmp);
    const programWords = wordsOf(scene.open || (scene.copy ? `${scene.copy}/main.rr` : ""));
    const j = judge(run, scene, programWords);
    check(`${name}: ${j.count} texts in ${j.form}, none plain English`, j.count > 0 && j.eng.length === 0, j.eng.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
    if (j.cut.length) left.push(`${scene.name} (${host}): ${j.cut.join(" | ")}`);
    if (STRICT_CUT) check(`${name}: nothing cut off`, j.cut.length === 0, j.cut.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
  } catch (e) {
    check(name, false, e.message);
  }
}

// ---- Spanish, the whole interface (ES-STUDIO) ---------------------------------
// The scenes above in Spanish, plus every menu, the Versions pane and Save a
// Version. A drawn text FAILS when it is English left: the very English text of
// a source string whose Spanish differs (ide/lang/studio.pot and RapidR's
// own runtime.pot, read against their es.po), or a sentence with English
// function words (the, and, your …) that is no name and no word of the program
// the scene opened. Cut-off text is reported (and fails like the pseudo-
// language's, RAPIDR_LANG_STRICT_CUT=0 only reports).
function poPairs(file) {
  const text = readFileSync(join(ROOT, file), "utf8");
  const out = [];
  let cur = null, field = null;
  // (a PO string: "…" with C's escapes)
  const str = (l) => {
    const body = l.slice(l.indexOf('"') + 1, l.lastIndexOf('"'));
    return body.replace(/\\(.)/g, (_, c) => ({ n: "\n", t: "\t", r: "\r" })[c] ?? c);
  };
  for (const l of text.split("\n").concat([""])) {
    if (!l.trim()) { if (cur && cur.id !== undefined) out.push(cur); cur = null; field = null; continue; }
    if (l.startsWith("#")) continue;
    cur ||= { id: undefined, plural: undefined, str: [] };
    if (l.startsWith("msgctxt")) field = "ctx";
    else if (l.startsWith("msgid_plural")) { cur.plural = str(l); field = "plural"; }
    else if (l.startsWith("msgid")) { cur.id = str(l); field = "id"; }
    else if (l.startsWith("msgstr[")) { cur.str[Number(/\[(\d+)\]/.exec(l)[1])] = str(l); field = ["str", Number(/\[(\d+)\]/.exec(l)[1])]; }
    else if (l.startsWith("msgstr")) { cur.str[0] = str(l); field = ["str", 0]; }
    else if (l.startsWith('"')) {
      if (field === "id") cur.id += str(l);
      else if (field === "plural") cur.plural += str(l);
      else if (Array.isArray(field)) cur.str[field[1]] += str(l);
    }
  }
  return out;
}
const noMnemonic = (s) => s.replace(/&&/g, "\u0000").replace(/&/g, "").replace(/\u0000/g, "&");
const escRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const EN_EXACT = new Map(), EN_PATTERNS = [];
for (const [pot, po] of [["ide/lang/studio.pot", "ide/lang/es.po"], ["crates/rapidr-value/lang/runtime.pot", "crates/rapidr-value/lang/es.po"]]) {
  const es = new Map(poPairs(po).map((e) => [e.id, e]));
  for (const e of poPairs(pot)) {
    const t = es.get(e.id);
    if (!t) continue;
    const forms = [[e.id, t.str[0]], ...(e.plural !== undefined ? [[e.plural, t.str[1]]] : [])];
    for (const [en, sp] of forms) {
      if (!en || !sp) continue;
      const a = noMnemonic(en), b = noMnemonic(sp);
      if (a === b || a.length < 3) continue;
      if (!/\{[^}]*\}/.test(a)) EN_EXACT.set(a, b);
      else if (a.replace(/\{[^}]*\}/g, "").trim().length >= 12) EN_PATTERNS.push([new RegExp(`^${escRe(a).replace(/\\\{[^}]*\\\}/g, ".+?")}$`), new RegExp(`^${escRe(b).replace(/\\\{[^}]*\\\}/g, ".+?")}$`)]);
    }
  }
}
const EN_WORDS = new Set(["the", "and", "of", "is", "are", "your", "you", "this", "that", "with", "from", "was", "were", "has", "have", "will", "isn't", "can't", "doesn't", "couldn't", "please", "click", "press", "choose", "open", "close", "save", "files", "project", "projects", "new", "add", "remove", "delete", "copy", "paste", "undo", "redo", "stop", "help", "search", "find", "replace", "settings", "tools", "window", "nothing", "there", "here", "after", "before", "when", "then", "all", "any", "not", "no"]);
// What a language-neutral window shows (a key, a name, a code word) is no English left.
const NEUTRAL = /^(enter|esc|escape|tab|space|shift|ctrl|alt|del|backspace|f\d+|[a-z]+\d*)$/i;
function englishLeft(texts, programWords) {
  const out = new Set();
  for (const t of texts) {
    const s = t.text.trim();
    if (!s) continue;
    // (a text made of names — a property's, the program's own words — is no English left)
    const plain = s.replace(/[\w./-]+\.(rr|bas|inc|rrproj|rqw|md|po|csv|json|txt|png|rtf)\b/gi, " ").replace(/\S*[/\\]\S*/g, " ");
    const words = plain.match(/[A-Za-z][A-Za-z']+/g) || [];
    if (words.length && words.every((w) => NAMES.has(w.toLowerCase()) || programWords.has(w.toLowerCase()))) continue;
    if (EN_EXACT.has(s)) { out.add(s); continue; }
    if (EN_PATTERNS.some(([en, sp]) => en.test(s) && !sp.test(s))) { out.add(s); continue; }
    // (a sentence with English function words in it, and nothing of the program's or a name)
    const bad = words.filter((w) => EN_WORDS.has(w.toLowerCase()) && !NAMES.has(w.toLowerCase()) && !programWords.has(w.toLowerCase()) && !(w.length <= 3 && w === w.toUpperCase()));
    if (bad.length >= 1 && words.length >= 2 && bad.some((w) => /^(the|and|your|this|that|with|from|was|were|isn't|can't|couldn't|doesn't)$/i.test(w))) out.add(s);
    else if (words.length >= 3 && bad.length >= 2) out.add(s);
    else if (bad.length && s.length > 25 && words.length >= 4 && words.filter((w) => !NEUTRAL.test(w)).length >= 4 && bad.some((w) => w.length > 3)) out.add(s);
  }
  return [...out];
}
// The Alt+<letter> that opens each menu in Spanish (the menu bar's mnemonics)
const ALT = (c) => `studio.__key_${c.toUpperCase().charCodeAt(0)}_1`;
const ES_SCENES = [
  ...XX_SCENES,
  // (ES-REFERENCE part 2: the pages and popups that carry the registry's
  // docs, in Spanish: a component's page, a statement's, a member's by F1 in
  // the code, an event's, a hover, the completion list)
  { name: "help-component", open: HELP_OPEN, do: "wait,help:RButton", delay: 5 },
  { name: "help-statement", open: HELP_OPEN, do: "wait,help:PRINT", delay: 5 },
  { name: "help-function", open: HELP_OPEN, do: "wait,help:MID$", delay: 5 },
  { name: "help-member", open: "examples/gui/hello_form.rr", do: "wait,view.code,code:Answer.Caption=>Answer.Caption,help.contents", delay: 5 },
  { name: "help-event", open: "examples/gui/hello_form.rr", do: "wait,view.code,code:Answer.Caption=>Answer.OnClick,help.contents", delay: 5 },
  { name: "hover", open: "examples/gui/hello_form.rr", do: 'focus:codedoc(0),key:Ctrl+End,key:Enter,type:ShowMessage "Hi",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait', delay: 6 },
  { name: "completion", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.,wait,wait", delay: 6 },
  ...[["archivo", "A"], ["edicion", "E"], ["ver", "V"], ["proyecto", "P"], ["ejecutar", "J"], ["depurar", "D"], ["formato", "F"], ["herramientas", "H"], ["ventana", "N"], ["ayuda", "U"]].map(([n, k]) => ({ name: `menu-${n}`, open: "examples/gui/hello_form.rr", events: ALT(k), delay: 5 })),
  // (two sub-menus: File > Abrir reciente (R), Edición > Avanzado (V))
  ...[["archivo-reciente", "A", 82], ["edicion-avanzado", "E", 86], ["archivo-fuera", "A", 70], ["ver-tema", "V", 84], ["archivo-versiones", "A", 78]].map(([n, k, sub]) => ({ name: `menu-${n}`, open: "examples/gui/hello_form.rr", events: `${ALT(k)},studio.__key_${sub}`, delay: 5 })),
  // (the AI Assistant pane: on, a server on this computer that isn't there — nothing is asked)
  { name: "assistant", open: "", do: "ai:on,ai:provider=compatible,ai:base=http://127.0.0.1:9,ai:model=mock-small,wait,wait", delay: 6 },
  { name: "versions-pane", ...VERSIONS_COPY, do: V_START, delay: 8 },
  { name: "versions-save", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,versions.save`, window: 2, delay: 8 },
  { name: "debug-panes", open: "examples/gui/hello_form.rr", do: "wait,view.debugPanes", delay: 5 },
  // (the dialogs and pages the scenes above don't reach)
  { name: "help", open: "", do: "wait,help.contents", delay: 6 },
  // (the code helpers' editors: Ctrl+. on a colour, a font, a picture; the
  // line's help after the code is a registry doc — no pseudo-language for
  // those, so Spanish only — and is clipped at the editor's edge by design)
  { name: "helper-colour", open: HELPERS, do: `${HELPERS_READY},line:18,key:End,key:Ctrl+.,wait`, delay: 9, cutOk: [/^El color de fondo/] },
  { name: "helper-font", open: HELPERS, do: `${HELPERS_READY},line:41,key:End,key:Ctrl+.,wait`, delay: 9 },
  { name: "helper-picture", open: HELPERS, do: `${HELPERS_READY},line:44,key:End,key:Ctrl+.,wait`, delay: 9, cutOk: [/^Carga un archivo de imagen/] },
];
{
  for (const scene of ES_SCENES) {
    const name = `es ${scene.name} ${THEMES[0]} desktop`;
    if (!want(name)) continue;
    try {
      const run = runDesktop(scene, "es", THEMES[0]);
      writeFileSync(join(OUT, `es-${scene.name}-${THEMES[0]}-desktop.bmp`), run.bmp);
      const w = run.report[(scene.window || 1) - 1] || run.report[0];
      const texts = (w && w.texts) || [];
      const programWords = wordsOf(scene.open || (scene.copy ? `${scene.copy}/main.rr` : ""));
      const eng = englishLeft(texts, programWords);
      // (a name scrolled out of its list — RMemo at the toolbox's edge — is no text a language cuts)
      const nameOnly = (t) => { const w = t.match(/[A-Za-z][A-Za-z']+/g) || []; return w.length > 0 && w.every((x) => NAMES.has(x.toLowerCase()) || programWords.has(x.toLowerCase())); };
      // (nor is a path of the machine that runs the test, cut short in the Output pane)
      // (nor is a line number half scrolled out of the editor)
      // (nor a sliver of a line — under 4 px tall — at the top of a pane that has scrolled: the Output pane's first
      // line, whose place depends on how long the machine's path is)
      const cut = [...new Set(texts.filter((t) => t.cut && /\p{L}/u.test(t.text) && !nameOnly(t.text) && !/[/\\]/.test(t.text) && !/^[\w .-]+\.(rr|bas|inc|rrproj|rqw|csv|po|md)$/i.test(t.text) && !(t.rect && t.rect[3] < 4) && !(scene.cutOk || []).some((k) => k.test(t.text))).map((t) => t.text))];
      check(`${name}: ${texts.length} texts in ${w ? w.form : "?"}, none English left`, texts.length > 0 && eng.length === 0, eng.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
      if (STRICT_CUT) check(`${name}: nothing cut off`, cut.length === 0, cut.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
      else if (cut.length) left.push(`${name}: ${cut.join(" | ")}`);
    } catch (e) {
      check(name, false, e.message);
    }
  }
}

// ---- Spanish (the es.po sample), from the start and switched live ------------
const SWITCH = [
  // (from the start: the menu bar's sample words)
  { name: "es-start", lang: "es", do: "", expect: ["Archivo", "Edición", "Herramientas", "Ayuda"], none: ["File", "Help"] },
  // (Settings' way, live: English, then Spanish — the menu bar and the
  // status bar's "Ready" follow at once)
  { name: "en-to-es", lang: "en", do: "wait,lang:es,wait", expect: ["Archivo", "Ejecutar", "Listo"], none: ["File", "Ready"] },
  // (and back to English, at once)
  { name: "es-to-en", lang: "es", do: "wait,lang:en,wait", expect: ["File", "Run", "Ready"], none: ["Archivo"] },
  // (the pseudo-language live: what the runtime translates again, what
  // Studio makes again on OnLanguageChange)
  { name: "en-to-xx", lang: "en", do: "wait,lang:xx,wait", pseudo: true },
  // (Settings in Spanish: its Language row)
  { name: "es-settings", lang: "es", do: "view.settings", window: 2, expect: ["Idioma"] },
  // (RAI in Studio's language: Settings > AI's "RAI answers in:" row and the
  // empty assistant's greeting, ES-REFERENCE part 3)
  { name: "es-settings-ai", lang: "es", do: "ai.settings", window: 2, expect: ["RAI responde en:", "El idioma de Studio", "Puede hacer:"], none: ["RAI answers in", "Studio's language"] },
  { name: "es-assistant", lang: "es", do: "ai:on,ai:provider=compatible,ai:base=http://127.0.0.1:9,ai:model=mock-small,wait,wait", delay: 6, expect: ["Pregunta por este proyecto", "Nuevo chat"], none: ["Ask about this project"] },
  // (the manual's first page in the Help pane: docs/manual/es/getting-started.md)
  { name: "es-help", lang: "es", do: "wait,manual:getting-started.md", expect: ["Primeros pasos", "En tres líneas"] },
  // (the manual's other most-read pages: docs/manual/es/, whole)
  { name: "es-help-debugging", lang: "es", do: "wait,manual:debugging.md", expect: ["Depuración en RapidR Studio", "El ejemplo"], none: ["No traducido aún"] },
  { name: "es-help-troubleshooting", lang: "es", do: "wait,manual:troubleshooting.md", expect: ["Solución de problemas", "Instalar y empezar"], none: ["No traducido aún"] },
  // (every page of the manual has its Spanish now: the language chapter in Spanish, no "not translated yet" note)
  { name: "es-help-language", lang: "es", do: "wait,manual:language.md", expect: ["El lenguaje"], none: ["No traducido aún"] },
  // (ES-REFERENCE part 2: the Help pane's own words around an entry, the
  // hover's and the completion's, the change box, the Theme values)
  { name: "es-help-component", lang: "es", open: HELP_OPEN, do: "wait,help:RButton", delay: 5, expect: ["Propiedades (", "Métodos (", "Eventos (", "Insertar en el código", "Su evento predeterminado es"], none: ["Properties (", "Insert into the code"] },
  { name: "es-help-statement", lang: "es", open: HELP_OPEN, do: "wait,help:PRINT", delay: 5, expect: ["Instrucción", "Insertar en el código"], none: ["Insert into the code"] },
  { name: "es-help-member", lang: "es", open: "examples/gui/hello_form.rr", do: "wait,view.code,code:Answer.Caption=>Answer.Caption,help.contents", delay: 5, expect: ["Miembro de", "Propiedad de RLabel"] },
  { name: "es-help-start", lang: "es", open: HELP_OPEN, do: "wait,view.code,wait,help:", delay: 5, expect: ["Para empezar", "Primeros pasos"], none: ["Start with"] },
  { name: "es-help-unknown", lang: "es", open: HELP_OPEN, do: "wait,help:RapidR", delay: 5, expect: ["No hay entrada para RapidR"] },
  { name: "es-hover", lang: "es", open: "examples/gui/hello_form.rr", do: 'focus:codedoc(0),key:Ctrl+End,key:Enter,type:ShowMessage "Hi",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait', delay: 6, expect: ["SHOWMESSAGE"] },
  { name: "es-change-box", lang: "es", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,line:12,versions.showChange`, delay: 8, expect: ["Deshacer este cambio", "Cambiado desde la última versión"], none: ["Undo This Change", "Changed since the last version"] },
  { name: "es-project-theme", lang: "es", open: "examples/gui/hello_form.rr", do: "wait,view.designer,project:select,row:Theme,prop:Theme=RapidR Dark,wait,wait", delay: 6, expect: ["Oscuro"], none: ["RapidR Dark", "RapidR oscuro"] },
  { name: "en-help", lang: "en", do: "wait,manual:getting-started.md", expect: ["Getting started", "In three lines"], none: ["Primeros pasos"] },
  // (a compiler error in Problems, in Spanish: the compiler's sample)
  { name: "es-problems", lang: "es", open: BAD, webOpen: "tests/results/studio-lang-bad/bad.rr", do: "wait,view.problems", delay: 5, expect: ["Se esperaba el fin de la línea, pero se encontró THEN"] },
];
for (const s of SWITCH) for (const host of hosts) {
  const name = `${s.name} ${host}`;
  if (!want(name)) continue;
  try {
    const run = host === "desktop" ? runDesktop(s, s.lang, THEMES[0]) : await runWeb(s, s.lang, THEMES[0]);
    writeFileSync(join(OUT, `${s.name}-${host}.bmp`), run.bmp);
    const j = judge(run, s, wordsOf(s.open));
    // (what the window draws, and what its accessibility tree says: a
    // Markdown page's text — the Help pane's — is in the tree)
    const shown = j.texts.map((t) => t.text.replace(/&/g, "")).concat([run.a11y || ""]);
    if (s.pseudo) {
      // (Output keeps its lines in the language they were written in: history)
      const eng = j.eng.filter((t) => t !== "RapidR Studio is ready.");
      check(`${name}: switched live, none plain English`, j.count > 0 && eng.length === 0, eng.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
      continue;
    }
    const missing = (s.expect || []).filter((e) => !shown.some((t) => t.includes(e)));
    const there = (s.none || []).filter((e) => shown.some((t) => t === e));
    check(`${name}: ${(s.expect || []).join(", ")}${s.none ? ` (not ${s.none.join(", ")})` : ""}`, missing.length === 0 && there.length === 0, `missing ${JSON.stringify(missing)}, still there ${JSON.stringify(there)}`);
  } catch (e) {
    check(name, false, e.message);
  }
}

// ---- the CLI's errors in the user's language --------------------------------
if (want("cli")) {
  const r = spawnSync(RAPIDR, ["check", BAD], { cwd: ROOT, encoding: "utf8", env: { ...process.env, RAPIDR_LANG: "es" } });
  check("RAPIDR_LANG=es rapidr check: the error in Spanish, RC.EXE's words beside it", r.status === 1 && r.stderr.includes("Se esperaba el fin de la línea, pero se encontró THEN [Expected end-of-line but got THEN]"), r.stderr.trim());
  const en = spawnSync(RAPIDR, ["check", BAD], { cwd: ROOT, encoding: "utf8", env: { ...process.env, RAPIDR_LANG: "en" } });
  check("RAPIDR_LANG=en rapidr check: as the compiler says it", en.stderr.includes("error: Expected end-of-line but got THEN") && !en.stderr.includes("Se esperaba"), en.stderr.trim());
}

if (browser) await browser.close();
if (left.length) console.log(`\nCut off in the pseudo-language (a longer language won't fit there):\n  ${left.join("\n  ")}`);
console.log(`\nRapidR Studio in your language: ${passed} passed, ${failed} failed (captures in ${OUT})`);
process.exit(failed ? 1 : 0);
