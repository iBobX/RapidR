// RapidR Studio in Brazilian Portuguese (lane I18N-PT, Preview 6; docs/i18n/pt-glossary.md):
// the scenes of tests/studio_lang.mjs run with --lang pt-BR on the desktop's
// headless host, each judged from the very display list captured
// (rapidr_ui_kernel::contrast's `texts`, RAPIDR_TEST_CONTRAST):
//   (i)  English left: a visible text that is the English of a source string whose
//        Portuguese differs (ide/lang/studio.pot and RapidR's own runtime.pot,
//        read against their pt-BR.po), or a sentence with English function words;
//   (ii) a text that doesn't fit: cut by its box or a clip (Portuguese words run
//        long: nothing may be cut off).
// And no box: every character of a text must be one the UI font draws (Latin
// with accents), judged on the texts themselves.
//
//   node tests/studio_lang_pt.mjs [filter…]       (RAPIDR=path of a debug build: its hooks need no token)
//
// RAPIDR_LANG_OUT (default tests/results/studio-lang-pt): <scene>.bmp per scene.
// RAPIDR_LANG_STRICT_CUT=0 reports cut-off text without failing.

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const OUT = process.env.RAPIDR_LANG_OUT || join(ROOT, "tests", "results", "studio-lang-pt");
const STRICT_CUT = process.env.RAPIDR_LANG_STRICT_CUT !== "0";
const LANG = "pt-BR";
const filters = process.argv.slice(2);
const want = (name) => !filters.length || filters.some((f) => name.includes(f));
mkdirSync(OUT, { recursive: true });
const scratch = join(OUT, "runs");
mkdirSync(scratch, { recursive: true });

// ---- the catalogs read as pairs ----------------------------------------------
function poPairs(file) {
  const text = readFileSync(join(ROOT, file), "utf8");
  const out = [];
  let cur = null, field = null;
  const str = (l) => l.slice(l.indexOf('"') + 1, l.lastIndexOf('"')).replace(/\\(.)/g, (_, c) => ({ n: "\n", t: "\t", r: "\r" })[c] ?? c);
  for (const l of text.split("\n").concat([""])) {
    if (!l.trim()) { if (cur && cur.id !== undefined) out.push(cur); cur = null; field = null; continue; }
    if (l.startsWith("#")) continue;
    cur ||= { ctx: "", id: undefined, plural: undefined, str: [] };
    if (l.startsWith("msgctxt")) { cur.ctx = str(l); field = "ctx"; }
    else if (l.startsWith("msgid_plural")) { cur.plural = str(l); field = "plural"; }
    else if (l.startsWith("msgid")) { cur.id = str(l); field = "id"; }
    else if (l.startsWith("msgstr[")) { cur.str[Number(/\[(\d+)\]/.exec(l)[1])] = str(l); field = ["str", Number(/\[(\d+)\]/.exec(l)[1])]; }
    else if (l.startsWith("msgstr")) { cur.str[0] = str(l); field = ["str", 0]; }
    else if (l.startsWith('"')) {
      if (field === "id") cur.id += str(l);
      else if (field === "plural") cur.plural += str(l);
      else if (field === "ctx") cur.ctx += str(l);
      else if (Array.isArray(field)) cur.str[field[1]] += str(l);
    }
  }
  return out;
}
const noMnemonic = (s) => s.replace(/&&/g, "\u0000").replace(/&/g, "").replace(/\u0000/g, "&");
const escRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const studioPo = new Map(poPairs("ide/lang/pt-BR.po").map((e) => [e.ctx + "\u0001" + e.id, e]));
const EN_EXACT = new Map(), EN_PATTERNS = [];
for (const [pot, po] of [["ide/lang/studio.pot", "ide/lang/pt-BR.po"], ["crates/rapidr-value/lang/runtime.pot", "crates/rapidr-value/lang/pt-BR.po"]]) {
  const tr = new Map(poPairs(po).map((e) => [e.id, e]));
  for (const e of poPairs(pot)) {
    const t = tr.get(e.id);
    if (!t) continue;
    for (const [en, pt] of [[e.id, t.str[0]], ...(e.plural !== undefined ? [[e.plural, t.str[1]]] : [])]) {
      if (!en || !pt) continue;
      const a = noMnemonic(en), b = noMnemonic(pt);
      if (a === b || a.length < 3) continue;
      if (!/\{[^}]*\}/.test(a)) EN_EXACT.set(a, b);
      else if (a.replace(/\{[^}]*\}/g, "").trim().length >= 12) EN_PATTERNS.push([new RegExp(`^${escRe(a).replace(/\\\{[^}]*\\\}/g, ".+?")}$`), new RegExp(`^${escRe(b).replace(/\\\{[^}]*\\\}/g, ".+?")}$`)]);
    }
  }
}
/** The Alt+<letter> key of a menu item's English source, as Portuguese gives it. */
function mnemonicOf(source, ctx = "") {
  const e = studioPo.get(ctx + "\u0001" + source);
  const m = e && /&([^&])/.exec(e.str[0] || "");
  if (!m) throw new Error(`no mnemonic for ${source}`);
  return m[1].toUpperCase().charCodeAt(0);
}
const ALT = (src) => `studio.__key_${mnemonicOf(src, "menubar")}_1`;
const SUB = (src) => `studio.__key_${mnemonicOf(src)}`;

// ---- what may stay English: names -----------------------------------------------
const NAMES = new Set(["true", "false", "rapidr", "studio", "rai", "basic", "utf", "crlf", "lf", "cr", "inter", "ms", "sans", "serif", "jetbrains", "mono", "courier", "new", "arial", "rapidq", "github", "anthropic", "claude", "openai", "ollama", "deepseek", "mcp", "api", "csv", "json", "sql", "url", "ctrl", "shift", "alt", "cmd", "enter", "esc", "escape", "tab", "del", "rr", "bas", "inc", "rrproj", "include", "includes", "layout", "design", "designer", "chat", "chats", "token", "tokens", "link", "regex", "hex", "web", "desktop", "app", "apps", "mdi", "console", "status", "checkpoint", "git", "commit", "diff", "log", "revert", "branch", "rust", "mouse", "beta", "ok", "backspace", "blob", "array", "assets", "asset", "bytecode", "dump", "pt", "px"]);
for (const n of ["aa", "ab", "snap", "guides", "grid", "arguments", "targets", "english", "compatible", "xai", "lm", "outside", "georgia", "tahoma", "times", "roman", "trebuchet", "verdana", "aabbyyzz", "bundle", "workspace", "provider", "model", "quiz", "player", "pixels", "frame", "data", "flags", "shell", "path", "playground"]) NAMES.add(n);
{
  const r = spawnSync(RAPIDR, ["lang", "export", "--json"], { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 28 });
  const walk = (v) => {
    if (Array.isArray(v) && v.length === 2 && typeof v[0] === "string" && typeof v[1] === "number") NAMES.add(v[0].toLowerCase());
    if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === "object") for (const [k, x] of Object.entries(v)) {
      if ((k === "name" || k === "display" || k === "rapidq") && typeof x === "string") {
        for (const w of x.split(/[^A-Za-z0-9_$]+/)) if (w) NAMES.add(w.toLowerCase());
      } else walk(x);
    }
  };
  try { walk(JSON.parse(r.stdout)); } catch { /* the registry's names are a convenience */ }
}
const EN_WORDS = new Set(["the", "and", "of", "is", "are", "your", "you", "this", "that", "with", "from", "was", "were", "has", "have", "will", "isn't", "can't", "doesn't", "couldn't", "please", "click", "press", "choose", "open", "close", "save", "cancel", "file", "folder", "project", "settings", "help", "error", "errors", "warning", "warnings", "version", "versions", "change", "changes", "undo", "redo", "run", "stop", "pause", "continue", "search", "find", "replace", "name", "type", "value", "list", "none", "all", "new", "edit", "copy", "paste", "delete", "remove", "add", "show", "hide", "back", "next", "line", "lines", "when", "for", "not", "no", "yes", "or", "to", "in", "on", "at", "by", "as", "it", "its", "an", "a"]);
const STRONG = /^(the|and|your|this|that|with|from|was|were|isn't|can't|couldn't|doesn't|please)$/i;
function wordsOf(file) {
  const out = new Set();
  if (!file) return out;
  const p = file.startsWith("/") ? file : join(ROOT, file);
  if (!existsSync(p)) return out;
  for (const w of readFileSync(p, "utf8").match(/[A-Za-z][A-Za-z']+/g) || []) out.add(w.toLowerCase().replace(/'s$/, ""));
  for (const w of file.split(/[/.]/)) out.add(w.toLowerCase());
  return out;
}
function englishLeft(texts, programWords) {
  const out = new Set();
  for (const t of texts) {
    const s = t.text.trim();
    if (!s) continue;
    const plain = s.replace(/[\w./-]+\.(rr|bas|inc|rrproj|rqw|md|po|csv|json|txt|png|rtf)\b/gi, " ").replace(/\S*[/\\]\S*/g, " ");
    const words = plain.match(/[A-Za-z][A-Za-z']+/g) || [];
    if (words.length && words.every((w) => NAMES.has(w.toLowerCase()) || programWords.has(w.toLowerCase()))) continue;
    if (EN_EXACT.has(s)) { out.add(s); continue; }
    if (EN_PATTERNS.some(([en, pt]) => en.test(s) && !pt.test(s))) { out.add(s); continue; }
    const strong = words.filter((w) => STRONG.test(w) && !programWords.has(w.toLowerCase()));
    if (strong.length >= 1 && words.length >= 3) out.add(s);
  }
  return [...out];
}
// A character the UI font can't draw shows as a box: Latin text with accents needs none of these.
const BOXY = /[□▯�]/;

// ---- the scenes -------------------------------------------------------------------
const BAD = join(ROOT, "tests/results/studio-lang-bad/bad.rr");
mkdirSync(dirname(BAD), { recursive: true });
writeFileSync(BAD, '\' a program with a mistake: Problems says it\nDIM b AS RButton\nPRINT "x" THEN\n');
const VERSIONS_COPY = { copy: "tests/fixtures/studio_versions", copyOpen: "Hello.rrproj" };
const V_START = "versions.pane,versions.start,versions:author=Ana,versions:save=first";
const HELP_OPEN = "tests/fixtures/studio_help/main.rr";
const HELLO = "examples/gui/hello_form.rr";
const DEBUG = "tests/fixtures/studio_debug/counter.rr";
const CRASH = "tests/fixtures/studio_lang/crash.rr";
const DIALOGS = "tests/fixtures/studio_lang/dialogs.rr";
const NOTES = "tests/fixtures/studio_lang/notes.rr";
const MSG_HOLD = { RAPIDR_TEST_DIALOG_HOLD: "9000", RAPIDR_TEST_MESSAGE_DIALOG: "OK" };
const SCENES = [
  { name: "main", open: "" },
  { name: "project", open: HELLO, do: "wait,view.designer", delay: 6 },
  { name: "code", open: HELLO, do: "wait,view.code", delay: 6 },
  { name: "settings", open: "", do: "view.settings", window: 2 },
  // (the legal review's R6: the key's place carries its preposition — "guardada na memória", never "em a memória" —;
  // the setup's cost notes name the age — "quem tem 18 anos ou mais", not "adultos" —, a professor for a child)
  { name: "settings-ai", open: "", do: "ai.settings", window: 2, expect: ["ela fica guardada na memória, só nesta sessão, nunca em um projeto."], none: ["guardada em ", "em a memória", "memory, for this session"] },
  { name: "setup-rai", open: "", do: "ai.setup", window: 2 },
  { name: "setup-rai-cloud", open: "", do: "ai.setup,ai:setupcloud,ai:setupnext", window: 2 },
  { name: "setup-rai-local", open: "", do: "ai.setup,ai:setuplocal,ai:setupnext", window: 2 },
  { name: "school-lock", open: "", do: "tools.schoolLock", window: 2 },
  { name: "consent", open: "", do: "wait,mcp:consent", window: 2, env: { RAPIDR_TEST_MCP_TOKEN: "lang-test" }, expect: ["são só para quem tem 18 anos ou mais: se você tem menos de 18 anos, configure isto com um responsável ou um professor."], none: ["para adultos", "docente"] },
  // (the APIKey row of an RAI in the designer: where the key is, in words — never the code word "environment")
  { name: "designer-key-kept", open: "tests/fixtures/studio_ai/rai_form.rr", do: "wait,view.designer,pick:AI,prop:Provider=anthropic,prop:BaseURL=http://127.0.0.1:9,prop:APIKey=sk-ant-fake-designer-0000,row:APIKey,wait", delay: 8, expect: ["guardada na memória, só nesta"], none: ["guardada em ", "em a memória", "is kept in"] },
  { name: "designer-key-env", open: "tests/fixtures/studio_ai/rai_form.rr", do: "wait,view.designer,pick:AI,prop:Provider=anthropic,prop:BaseURL=http://127.0.0.1:9,prop:APIKeySource=env:ANTHROPIC_API_KEY,row:APIKey,wait", delay: 8, env: { ANTHROPIC_API_KEY: "sk-ant-fake-env-0000" }, expect: ["em uma variável de ambiente: o"], none: ["em environment", "found in the", "no the"] },
  { name: "settings-aiapps", open: "", do: "mcp.settings", window: 2, env: { RAPIDR_TEST_MCP_TOKEN: "lang-test" } },
  // (the Details link's row touches the pane's bottom edge in this window: its last pixel row is clipped
  // in English and Spanish too — the golden's ai-panel — and nothing of its letters is lost)
  { name: "assistant", open: "", do: "ai:on,ai:provider=compatible,ai:base=http://127.0.0.1:9,ai:model=mock-small,wait,wait", delay: 6, cutOk: [/^Detalhes$/] },
  { name: "problems", open: BAD, do: "wait,view.problems", delay: 5 },
  { name: "run-error", open: CRASH, do: "run.start,wait,wait,wait,wait", delay: 11 },
  { name: "debug-paused", open: DEBUG, do: "line:6,debug.toggleBreakpoint,watch:k,run.start,wait,wait,wait,wait", delay: 11 },
  { name: "palette", open: "", do: "view.commandPalette" },
  { name: "find", open: HELLO, do: "wait,edit.findInFiles", delay: 5 },
  { name: "replace-bar", open: HELLO, do: "wait,view.code,wait,edit.replace", delay: 5 },
  { name: "goto-line", open: HELLO, do: "wait,view.code,wait,edit.goToLine", delay: 5 },
  { name: "about", open: "", do: "help.about", window: 2, delay: 5, env: MSG_HOLD },
  { name: "keyboard-shortcuts", open: "", do: "view.keyboardShortcuts", window: 2, delay: 5, env: MSG_HOLD },
  { name: "project-options", open: HELLO, do: "wait,project.options", window: 2 },
  { name: "open-example", open: "", do: "file.openExample" },
  { name: "help-start", open: HELP_OPEN, do: "wait,view.code,wait,help:", delay: 5 },
  { name: "help-unknown", open: HELP_OPEN, do: "wait,help:RapidR", delay: 5 },
  { name: "help", open: "", do: "wait,help.contents", delay: 6 },
  { name: "quick-fix", open: NOTES, do: "wait,view.code,wait,focus:codedoc(0),line:14,key:End,edit.quickFix,wait", delay: 8 },
  { name: "versions-pane", ...VERSIONS_COPY, do: V_START, delay: 8 },
  { name: "versions-save", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,versions.save`, window: 2, delay: 8 },
  { name: "versions-changes", ...VERSIONS_COPY, do: `${V_START},code:PRINT "Done."=>PRINT "All done!",file.save,versions.changes`, delay: 9 },
  { name: "debug-panes", open: HELLO, do: "wait,view.debugPanes", delay: 5 },
  { name: "tab-order", open: HELLO, do: "wait,view.designer,wait,designer.tabOrder", delay: 6 },
  ...[
    ["colour-dialog", "colorbutton.onclick,colorbutton.__wait_500", { RAPIDR_TEST_COLOR_DIALOG: "255;" }],
    ["font-dialog", "fontbutton.onclick,fontbutton.__wait_500", { RAPIDR_TEST_FONT_DIALOG: ";" }],
    ["message-box", "askbutton.onclick,askbutton.__wait_500", { RAPIDR_TEST_MESSAGE_DIALOG: "Cancel" }],
  ].map(([name, events, env]) => ({ name, program: DIALOGS, events, env: { RAPIDR_TEST_DIALOG_HOLD: "9000", ...env }, window: 2, delay: 3 })),
  // (the right-click menus of Studio, CONTEXT-MENUS 2a: the Project tree's file, the designer's component, the code
  // editor's under the caret (Shift+F10), a property's — the manual's own recipes, tools/manual/scenes/context-menus.mjs)
  { name: "ctx-project-file", open: "tests/fixtures/studio_search/Counter.rrproj", do: "view.project,wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,projecttree.__rightclick_110_102,projecttree.__wait_500" }, expect: ["Renomear", "Copiar caminho", "Perguntar à RAI sobre este arquivo..."] },
  { name: "ctx-designer", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__rightclick_180_118,designdoc(0).__wait_500" }, expect: ["Editar manipulador de evento", "Perguntar à RAI sobre este componente...", "Centralizar no formulário"] },
  { name: "ctx-code", open: "tests/fixtures/studio_search/Counter.rrproj", do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,codedoc(0).__mousedown_120_212,codedoc(0).__mouseup_120_212,codedoc(0).__key_121_256,codedoc(0).__wait_500" }, expect: ["Perguntar à RAI sobre este código..."] },
  { name: "ctx-inspector", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__mousedown_180_118,designdoc(0).__mouseup_180_118,inspector.__wait_300,inspector.__rightclick_60_230,inspector.__wait_500" }, expect: ["Editar valor", "Copiar como código", "Perguntar à RAI sobre"] },
  // (every menu, by its Portuguese Alt letter, and the sub-menus)
  ...[["File", "File"], ["Edit", "Edit"], ["View", "View"], ["Project", "Project"], ["Run", "Run"], ["Debug", "Debug"], ["Format", "Format"], ["Tools", "Tools"], ["Window", "Window"], ["Help", "Help"]]
    .map(([n]) => ({ name: `menu-${n.toLowerCase()}`, open: HELLO, events: ALT({ File: "&File", Edit: "&Edit", View: "&View", Project: "&Project", Run: "&Run", Debug: "&Debug", Format: "F&ormat", Tools: "&Tools", Window: "&Window", Help: "&Help" }[n]), delay: 5 })),
  ...[["file-recent", "&File", "Open &Recent", 82], ["edit-advanced", "&Edit", "Ad&vanced", 86], ["file-versions", "&File", "Versio&ns", 78], ["view-theme", "&View", "T&heme", 84]]
    .map(([n, menu, item]) => ({ name: `menu-${n}`, open: HELLO, events: `${ALT(menu)},${SUB(item)}`, delay: 5 })),
];

function runDesktop(scene) {
  const dir = join(scratch, scene.name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = scene.program ? ["run", scene.program] : ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", process.env.RAPIDR_LANG_THEME || "rapidr-light", "--lang", LANG];
  if (scene.do && !scene.program) args.push("--do", scene.do);
  if (scene.copy) {
    cpSync(join(ROOT, scene.copy), join(dir, "project"), { recursive: true });
    args.push(join(dir, "project", scene.copyOpen));
  } else if (scene.open) args.push(scene.open);
  spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 120000,
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
      RAPIDR_CONFIG_DIR: join(dir, "config"),
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_KEYSTORE: "memory",
      RAPIDR_AI_CHATS: join(dir, "chats"),
      RAPIDR_DOCUMENTS: join(dir, "documents"),
      ...(scene.program ? { RAPIDR_LANG: LANG } : {}),
      ...(scene.events ? { RAPIDR_TEST_EVENTS: scene.events } : {}),
      ...(scene.env || {}),
    },
  });
  const file = join(dir, `window-${scene.window || 1}.bmp`);
  if (!existsSync(file)) throw new Error("desktop: no capture");
  const report = existsSync(join(dir, "contrast.json")) ? JSON.parse(readFileSync(join(dir, "contrast.json"), "utf8")) : [];
  const a11y = existsSync(join(dir, "a11y.json")) ? readFileSync(join(dir, "a11y.json"), "utf8") : "";
  return { bmp: readFileSync(file), report, a11y };
}

let failed = 0, passed = 0;
function check(name, ok, detail = "") {
  ok ? passed++ : failed++;
  console.log(`${ok ? "✓" : "✗"} ${name}${detail ? `\n    ${detail}` : ""}`);
}

for (const scene of SCENES) {
  const name = `pt-BR ${scene.name}`;
  if (!want(name)) continue;
  try {
    const run = runDesktop(scene);
    writeFileSync(join(OUT, `${scene.name}.bmp`), run.bmp);
    const w = run.report[(scene.window || 1) - 1] || run.report[0];
    const texts = (w && w.texts) || [];
    const programWords = wordsOf(scene.open || scene.program || (scene.copy ? `${scene.copy}/main.rr` : ""));
    const eng = englishLeft(texts, programWords);
    const nameOnly = (t) => { const ws = t.match(/[A-Za-z][A-Za-z']+/g) || []; return ws.length > 0 && ws.every((x) => NAMES.has(x.toLowerCase()) || programWords.has(x.toLowerCase())); };
    const cut = [...new Set(texts.filter((t) => t.cut && /\p{L}/u.test(t.text) && !nameOnly(t.text) && !/[/\\]/.test(t.text) && !/^[\w .-]+\.(rr|bas|inc|rrproj|rqw|csv|po|md)$/i.test(t.text) && !(t.rect && t.rect[3] < 4) && !(scene.cutOk || []).some((k) => k.test(t.text))).map((t) => t.text))];
    const boxes = texts.filter((t) => BOXY.test(t.text)).map((t) => t.text);
    // (what a scene must say, and must not: the lines drawn and the accessibility tree's whole texts)
    const shown = texts.map((t) => t.text.replace(/&/g, "")).concat([run.a11y || ""]);
    if (scene.expect || scene.none) {
      const missing = (scene.expect || []).filter((e) => !shown.some((t) => t.includes(e)));
      const there = (scene.none || []).filter((e) => shown.some((t) => t.includes(e)));
      check(`${name}: ${(scene.expect || []).join(" | ")}`, missing.length === 0 && there.length === 0, `missing ${JSON.stringify(missing)}, still there ${JSON.stringify(there)}`);
    }
    check(`${name}: ${texts.length} texts in ${w ? w.form : "?"}, none English left`, texts.length > 0 && eng.length === 0, eng.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
    if (STRICT_CUT) check(`${name}: nothing cut off`, cut.length === 0, cut.slice(0, 20).map((t) => JSON.stringify(t)).join("\n    "));
    else if (cut.length) console.log(`  cut: ${cut.join(" | ")}`);
    check(`${name}: no box characters`, boxes.length === 0, boxes.slice(0, 10).map((t) => JSON.stringify(t)).join("\n    "));
  } catch (e) {
    check(name, false, e.message);
  }
}

// ---- the CLI's errors in Portuguese ------------------------------------------------
if (want("cli")) {
  const r = spawnSync(RAPIDR, ["check", BAD], { cwd: ROOT, encoding: "utf8", env: { ...process.env, RAPIDR_LANG: LANG } });
  check("RAPIDR_LANG=pt-BR rapidr check: the error in Portuguese, RC.EXE's words beside it", r.status === 1 && r.stderr.includes("Esperava-se o fim da linha, mas veio THEN [Expected end-of-line but got THEN]"), r.stderr.trim());
  for (const sys of ["pt_BR.UTF-8", "pt"]) {
    const s = spawnSync(RAPIDR, ["check", BAD], { cwd: ROOT, encoding: "utf8", env: { ...process.env, RAPIDR_LANG: "", LC_ALL: sys, LANG: sys } });
    check(`the system's ${sys}: Portuguese too`, s.stderr.includes("Esperava-se o fim da linha"), s.stderr.trim());
  }
}

console.log(`\nRapidR Studio in Brazilian Portuguese: ${passed} passed, ${failed} failed (captures in ${OUT})`);
process.exit(failed ? 1 : 0);
