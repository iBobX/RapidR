// A file a program deletes with KILL stays deleted on the web — in the page's
// store and, for RapidR Studio, in the browser's saved copy of it (the origin
// private file system, ide/web/studio.js), so it isn't back at the next visit.
//
//   1. On the runtime's own page (tests/web_kernel.html): a program writes a
//      file and KILLs it; the page that keeps files (RAPIDR_FILE_SINK) hears
//      the write, then the removal (RAPIDR_FILE_REMOVED), and the program
//      sees it gone.
//   2. RapidR Studio in the browser, a person's way: the assistant makes a
//      program (the mock provider, tests/ai_mock_server.mjs) — its files are
//      kept in the browser —, the banner's Undo is pressed by the keyboard (Tab,
//      Enter), and the browser's copy of every file of the program is gone,
//      at once and after a reload; Studio's own files that weren't the
//      program's stay.
//
// Usage (repo root, after tools/build_web_artifacts.sh and
// tools/build_studio_web.sh, the repo served on RAPIDR_URL, default
// http://localhost:8765, and Studio on STUDIO_WEB_URL, default its build
// under the repo):  node tests/web_file_store.mjs

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { openRunner, URL_BASE } from "./web_run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const STUDIO_URL = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || `${URL_BASE}/target/studio-web`).replace(/\/+$/, "");
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const browser = await chromium.launch();

// ---- 1. the runtime's page ------------------------------------------------
{
  const r = await openRunner(browser);
  await r.page.addInitScript(() => {
    window.__heard = [];
    window.RAPIDR_FILE_SINK = (path, bytes) => window.__heard.push(`write ${path} ${bytes.length}`);
    window.RAPIDR_FILE_REMOVED = (path) => window.__heard.push(`removed ${path}`);
  });
  await r.run([
    "DIM f AS INTEGER",
    "f = FREEFILE",
    'OPEN "notes.txt" FOR OUTPUT AS #f',
    'PRINT #f, "hello"',
    "CLOSE #f",
    'PRINT "before=" + STR$(FILEEXISTS("notes.txt") <> 0)',
    'KILL "NOTES.TXT"',
    'PRINT "after=" + STR$(FILEEXISTS("notes.txt") <> 0)',
    'PRINT "done"',
  ].join("\n"));
  ok(await r.waitOutput("done"), `the program ran to its end (${r.output().replace(/\n/g, " | ")} ${r.errors()})`);
  const heard = await r.page.evaluate(() => window.__heard);
  ok(/before=\s*-?1/.test(r.output()) && /after=\s*0/.test(r.output()), `the file was there, then gone (${r.output().replace(/\n/g, " | ")})`);
  // (the file made empty by OPEN, then its text; then the removal, under the name KILL was given)
  ok(heard.length >= 2 && heard.slice(0, -1).every((h) => /^write notes\.txt \d+$/.test(h)) && heard.some((h) => h === "write notes.txt 6") && heard.at(-1) === "removed NOTES.TXT", `the page heard the writes, then the removal (${JSON.stringify(heard)})`);
  await r.stop();
}

// ---- 2. Studio: Undo of a new program, then a reload ----------------------
const mock = spawn(process.execPath, [join(HERE, "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
const MOCK_URL = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));

// (the names of the files the page keeps: Studio's own folder in the
// browser's private file system, each path encoded as one name)
const kept = (page) => page.evaluate(async () => {
  try {
    const root = await navigator.storage.getDirectory();
    const dir = await root.getDirectoryHandle("rapidr-studio", { create: true });
    const names = [];
    for await (const [name, h] of dir.entries()) if (h.kind === "file") names.push(decodeURIComponent(name));
    return names.sort();
  } catch (e) {
    return ["(no storage: " + e.message + ")"];
  }
});
const untilKept = async (page, cond, ms = 30000) => {
  for (let waited = 0; waited < ms; waited += 250) {
    if (cond(await kept(page))) return true;
    await page.waitForTimeout(250);
  }
  return cond(await kept(page));
};

try {
  const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const doSteps = `ai:on,ai:provider=compatible,ai:base=${MOCK_URL},ai:model=mock-small,ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait`;
  const q = new URLSearchParams({ theme: "rapidr-light", fresh: "", window: "normal", do: doSteps });
  await page.goto(`${STUDIO_URL}/index.html?${q}`, { waitUntil: "load" });
  const program = (names) => names.filter((n) => /ScientificCalculator/i.test(n));
  const made = await untilKept(page, (names) => program(names).length >= 2);
  const before = await kept(page);
  ok(made, `the assistant's program is kept in the browser (${JSON.stringify(program(before))})`);
  // (the banner is up and Run it has the keys: Tab to Undo, Enter, as the person's keys)
  await page.waitForTimeout(3000);
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  const gone = await untilKept(page, (names) => program(names).length === 0);
  const afterUndo = await kept(page);
  ok(gone, `after Undo none of the program's files is kept (${JSON.stringify(afterUndo)})`);
  // (the queue of saves and removals is done before the page goes)
  await page.waitForTimeout(1000);
  // (a reload without the `do` that made the program: Studio as the person opens it again)
  await page.goto(`${STUDIO_URL}/index.html?${new URLSearchParams({ theme: "rapidr-light", window: "normal" })}`, { waitUntil: "load" });
  await page.waitForTimeout(3000);
  const afterReload = await kept(page);
  ok(program(afterReload).length === 0, `after a reload they are still gone (${JSON.stringify(afterReload)})`);
  ok(errors.length === 0, `no page errors (${errors.join(" | ")})`);
  await context.close();
} finally {
  mock.kill();
  await browser.close();
}
if (failed) { console.log(`\nWeb file store: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb file store: ALL CHECKS PASSED");
