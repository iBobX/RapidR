// RapidR Studio's measured budgets (docs/studio-wow.md §3.15, docs/ide-plan.md
// §6.2), on the desktop's headless host and on the web page:
//   PERF-1  cold start to an interactive Studio (desktop ≤ 1.0 s; the web's
//           cached start ≤ 1.2 s — its first visit: tests/studio_perf_web.mjs)
//   PERF-8  F5 to the program's first form (desktop ≤ 300 ms, web ≤ 500 ms)
//   DBG     a step to its line shown with the locals (≤ 100 ms; studio-wow §2
//           step 11: "Stop → highlight and locals")
//   PERF-9  Run in Browser to the form on the page (≤ 1.5 s): tests/run_in_browser.mjs
// Each is the median of three runs. A budget missed fails; so does a
// measure more than 20 % (and 25 ms) over tests/studio_perf_baseline.json —
// RAPIDR_PERF_BLESS=1 writes the baseline from this run.
//
//   tools/build_studio_web.sh
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_perf.mjs

import { spawnSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-perf");
const BASELINE = join(HERE, "studio_perf_baseline.json");
const RUNS = 3;
mkdirSync(WORK, { recursive: true });

const BUDGETS = {
  "desktop.start": 1000,
  "desktop.f5-form": 300,
  "desktop.step": 100,
  "web.start-cached": 1200,
  "web.f5-form": 500,
  "web.step": 100,
};

const median = (a) => [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)];
// (the local clock's seconds since midnight, as BASIC's TIMER reads it)
const secondsToday = () => {
  const d = new Date();
  return (d - new Date(d.getFullYear(), d.getMonth(), d.getDate())) / 1000;
};
const perfOf = (text, key) => {
  const m = (text || "").match(new RegExp(`\\[perf\\][^\\n]*\\b${key}=(-?\\d+)`));
  return m ? Number(m[1]) : NaN;
};

const env = (extra) => ({
  ...process.env,
  RAPIDR_MENU: "window",
  RAPIDR_PRINT_TO: join(WORK, "prints"),
  RAPIDR_REGISTRY: join(WORK, "studio.reg"),
  ...extra,
});

// (Studio on the desktop's headless host, its commands run, its Output read)
function desktop(open, doList, delay) {
  const r = spawnSync(RAPIDR, ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", doList, open], {
    cwd: ROOT,
    timeout: 120000,
    encoding: "utf8",
    env: env({ RAPIDR_CAPTURE: join(WORK, "window"), RAPIDR_CAPTURE_DELAY: String(delay), RAPIDR_TEST_DUMP: "outputbox.text" }),
  });
  return r.stdout || "";
}

// (the desktop's cold start: spawn to the first turn of Studio's loop)
function desktopStart() {
  return new Promise((resolve) => {
    const t0 = secondsToday();
    const child = spawn(RAPIDR, ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light"], {
      cwd: ROOT,
      env: env({ RAPIDR_STUDIO_PERF: "1", RAPIDR_CAPTURE: join(WORK, "start"), RAPIDR_CAPTURE_DELAY: "1" }),
    });
    let out = "";
    const done = (v) => {
      child.kill();
      resolve(v);
    };
    child.stdout.on("data", (d) => {
      out += d;
      const m = out.match(/\[perf\] ready\s+([\d.]+)/);
      if (m) done(Math.round((Number(m[1]) - t0) * 1000));
    });
    child.on("exit", () => done(NaN));
    setTimeout(() => done(NaN), 60000);
  });
}

async function web(browser, open, files, doList, delay) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  try {
    const texts = files.map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, texts);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, { RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: String(delay), RAPIDR_TEST_DUMP: "outputbox.text" });
    const q = new URLSearchParams({ theme: "rapidr-light", fresh: "", window: "normal", do: doList, open });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 120000, polling: 200 });
    return JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results())).dump.join("\n");
  } finally {
    await page.close();
  }
}

async function webStart(browser) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  try {
    // (once to fill the cache, then measured)
    const load = async () => {
      await page.goto(`${URL_BASE}/index.html?fresh&theme=rapidr-light`, { waitUntil: "load" });
      await page.waitForFunction(() => window.RAPIDR_STUDIO_STARTED !== undefined, null, { timeout: 60000, polling: 10 });
      return page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(performance.now())))));
    };
    await load();
    return Math.round(await load());
  } finally {
    await page.close();
  }
}

const HELLO = "examples/gui/hello_form.rr";
const COUNTER = "tests/fixtures/studio_debug/counter.rr";
const COUNTER_FILES = [COUNTER, "tests/fixtures/studio_debug/tally.inc"];
const F5 = "run.start,wait,wait,wait,perf:report";
const STEP = "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,debug.stepOver,wait,wait,debug.stepOver,wait,wait,perf:report";

const results = {};
const add = (k, v) => (results[k] ||= []).push(v);
for (let i = 0; i < RUNS; i++) {
  add("desktop.start", await desktopStart());
  add("desktop.f5-form", perfOf(desktop(HELLO, F5, 4), "f5-form"));
  add("desktop.step", perfOf(desktop(COUNTER, STEP, 7), "step"));
}
const browser = await chromium.launch();
try {
  for (let i = 0; i < RUNS; i++) {
    add("web.start-cached", await webStart(browser));
    add("web.f5-form", perfOf(await web(browser, HELLO, [], F5, 4), "f5-form"));
    add("web.step", perfOf(await web(browser, COUNTER, COUNTER_FILES, STEP, 7), "step"));
  }
} finally {
  await browser.close();
}

const baseline = existsSync(BASELINE) ? JSON.parse(readFileSync(BASELINE, "utf8")) : {};
let failed = 0;
const medians = {};
for (const [k, budget] of Object.entries(BUDGETS)) {
  const runs = results[k] || [];
  const m = median(runs.filter(Number.isFinite));
  medians[k] = m;
  const base = baseline[k];
  const overBudget = !Number.isFinite(m) || m > budget;
  const regressed = Number.isFinite(base) && m > Math.max(base * 1.2, base + 25);
  if (overBudget || regressed) failed++;
  console.log(`${overBudget || regressed ? "✗" : "✓"} ${k}: ${Number.isFinite(m) ? m + " ms" : "not measured"} (runs ${runs.join(", ")}; budget ${budget} ms${Number.isFinite(base) ? `, baseline ${base} ms` : ""})${regressed ? " — more than 20 % over the baseline" : ""}`);
}
if (process.env.RAPIDR_PERF_BLESS) {
  writeFileSync(BASELINE, JSON.stringify(medians, null, 2) + "\n");
  console.log(`(baseline written: ${BASELINE})`);
}
console.log(`\nStudio perf: ${Object.keys(BUDGETS).length - failed} within budget, ${failed} not`);
process.exit(failed ? 1 : 0);
