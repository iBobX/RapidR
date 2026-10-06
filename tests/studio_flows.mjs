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
import { existsSync, mkdirSync, rmSync } from "node:fs";
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
  if (c.open) args.push(c.open);
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 90000,
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_DUMP: Object.keys(c.dump).join(","),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
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
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_TEST_DUMP: Object.keys(c.dump).join(","),
    });
    const q = new URLSearchParams({ theme: "rapidr-light", fresh: "" });
    if (c.do) q.set("do", c.do);
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 90000, polling: 200 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { dump: parseDump(results.dump.join("\n"), Object.keys(c.dump)), errors };
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
    check("web", web.dump, c);
    if (web.errors.length) console.log(`  (page errors: ${web.errors.join("; ")})`);
  } catch (e) {
    failed++;
    console.log(`✗ ${c.name} (web): ${e.message.split("\n")[0]}`);
  }
}
await browser.close();
console.log(`\nRapidR Studio flows: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
