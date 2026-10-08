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

import { spawnSync } from "node:child_process";
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
// (STUDIO_WEB_URL: each lane serves its own build on its own port)
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-flows");
const filters = process.argv.slice(2);

// (I4) The designer's steps on notepad.bas: the placing tool's click, the
// form's right edge dragged, Button1 dragged.
const DESIGN_STEPS = [
  "__mousedown_100_120", "__mouseup_100_120",
  "__mousedown_491_250", "__mousemove_521_250", "__mousemove_551_250", "__mouseup_551_250",
  "__mousedown_110_130", "__mousemove_130_150", "__mousemove_150_170", "__mouseup_150_170",
].map((e) => `designdoc(0).${e}`).join(",");

// Each case: what Studio opens and does (`do`: its commands; `events`:
// RAPIDR_TEST_EVENTS, input through the kernel), how long it waits before
// the properties are read, and what each must say (a regular expression;
// `same`: equal to a file's text, line ends as the editor keeps them).
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
    dump: { "proj.builtpath": /(Notes\.app|Notes\.AppDir|main\.exe)$/, "outputbox.text": /icon: .*note\.svg[\s\S]*Built /, "proj.building": /^0$/ },
    webDump: { "outputbox.text": /Can't build: Build makes apps in RapidR Studio on the desktop/, "proj.builtpath": /^$/ },
  },
  {
    // Project > Project Options: the app's name, ID, version, icon (previewed)
    name: "app-options",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "project.options",
    delay: 4,
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
      "designdoc(0).statustext": /^Button1 \(QBUTTON\), 128, 88, 75 × 25$/,
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
    dump: { "designdoc(0).statustext": /^Added CheckBox1 \(QCHECKBOX\)/, "codedoc(0).text": /    CREATE CheckBox1 AS QCHECKBOX\n        Caption = "CheckBox1"\n/ },
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
  // (S-PANELS) The toolbox: Enter on QCHECKBOX adds one to the form (its
  // CREATE block in the code), selected in the inspector.
  {
    name: "toolbox-add",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,tool:QCHECKBOX,wait",
    delay: 6,
    dump: { "codedoc(0).text": /CREATE CheckBox1 AS QCHECKBOX/i, "inspector.target": /^CheckBox1$/i },
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
    dump: { "helptitle.caption": /^QLABEL\.Caption$/, "helpwhat.caption": /^Property of QLABEL/ },
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
];

function runDesktop(c) {
  const dir = join(WORK, `${c.name}-desktop`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light"];
  if (c.do) args.push("--do", c.do);
  if (c.open && c.copyDir) {
    // (the project's whole folder: Build writes the app beside it)
    cpSync(join(ROOT, dirname(c.open)), join(dir, "project"), { recursive: true });
    args.push(join(dir, "project", c.open.split("/").pop()));
  } else if (c.open && c.copy) {
    const to = join(dir, c.open.split("/").pop());
    copyFileSync(join(ROOT, c.open), to);
    args.push(to);
  } else if (c.open) {
    args.push(c.open);
  }
  const run = (args, delay) => spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: Math.max(90000, delay * 1000 + 60000),
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(delay),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_DUMP: Object.keys(c.dump).join(","),
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
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
    r = run(again, c.restart.delay || c.delay);
  }
  return parseDump(r.stdout || "", Object.keys(c.dump));
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

async function runWeb(browser, c) {
  if (!c.restart) return runWebPage(await browser.newContext({ viewport: { width: 1920, height: 1080 } }), c, true);
  // (started again in the same browser profile: the page's settings kept)
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  await runWebPage(ctx, c, false);
  return runWebPage(ctx, { ...c, do: c.restart.do, delay: c.restart.delay || c.delay, fresh: false }, true);
}

async function runWebPage(ctx, c, last) {
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = (c.webFiles || []).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_TEST_DUMP: Object.keys(c.webDump || c.dump).join(","),
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: c.folder } : {}),
    });
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal" });
    if (c.fresh !== false) q.set("fresh", "");
    if (c.do) q.set("do", c.do);
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: Math.max(90000, c.delay * 1000 + 60000), polling: 200 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { dump: parseDump(results.dump.join("\n"), Object.keys(c.webDump || c.dump)), errors };
  } finally {
    await page.close();
    if (last) await ctx.close();
  }
}

mkdirSync(WORK, { recursive: true });
const browser = await chromium.launch();
let passed = 0, failed = 0;
const check = (label, dump, c) => {
  for (const [k, re] of Object.entries(c.dump)) {
    const v = dump[k];
    const ok = v !== undefined && re.test(v);
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? "" : `= ${JSON.stringify(v)} (wanted ${re})`}`);
  }
  for (const [k, file] of Object.entries(c.same || {})) {
    const want = readFileSync(join(ROOT, file), "utf8").replace(/\r\n/g, "\n").replace(/\n+$/, "");
    const v = dump[k];
    const ok = v !== undefined && v.replace(/\n+$/, "") === want;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? `equals ${file}` : `differs from ${file}`}`);
  }
};
for (const c of CASES.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  check("desktop", runDesktop(c), c);
  try {
    const web = await runWeb(browser, c);
    check("web", web.dump, c.webDump ? { ...c, dump: c.webDump } : c);
    if (web.errors.length) console.log(`  (page errors: ${web.errors.join("; ")})`);
  } catch (e) {
    failed++;
    console.log(`✗ ${c.name} (web): ${e.message.split("\n")[0]}`);
  }
}
await browser.close();
console.log(`\nRapidR Studio flows: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
