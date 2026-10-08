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
const URL_BASE = process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473";
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-flows");
const filters = process.argv.slice(2);

// Each case: what Studio opens and does, how long it waits before the
// properties are read, and what each must say (a regular expression).
const CASES = [
  {
    name: "run-console",
    open: "examples/basics/hello.rr",
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /Hello from RapidR![\s\S]*ended, exit code 0/, "session.state": /^stopped$/, "session.exitcode": /^0$/ },
  },
  {
    name: "outline-problems",
    open: "examples/gui/hello_form.rr",
    do: "wait",
    delay: 3,
    dump: { "outlinetree.itemcount": /^[5-9]|1\d$/, "lang.errorcount": /^0$/, "proj.kind": /^file$/, "proj.filecount": /^1$/ },
  },
  {
    name: "theme-and-tabs",
    open: "examples/gui/hello_form.rr",
    do: "view.theme.dark,view.documents.tabs",
    delay: 3,
    dump: { "application.theme": /^dark$/, "dock.documentmode": /^tabs$/ },
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
    dump: { "palette.visible": /^(-1|1|True)$/i, "palettelist.itemcount": /^[1-9]\d+$/ },
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
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: Math.max(90000, c.delay * 1000 + 60000),
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_DUMP: Object.keys(c.dump).join(","),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: join(ROOT, c.folder) } : {}),
    },
  });
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
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = (c.webFiles || []).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_TEST_DUMP: Object.keys(c.webDump || c.dump).join(","),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: c.folder } : {}),
    });
    const q = new URLSearchParams({ theme: "rapidr-light", fresh: "", window: "normal" });
    if (c.do) q.set("do", c.do);
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 90000, polling: 200 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { dump: parseDump(results.dump.join("\n"), Object.keys(c.webDump || c.dump)), errors };
  } finally {
    await page.close();
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
