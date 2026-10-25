// Running a form program from the web Studio must never panic
// (the "RapidR panicked … RefCell already (mutably) borrowed" Robert met on
// 2026-10-24 running a form project). A program's end reaches Studio's
// wasm as a message (the frame's `exited` event, `studio_session_incoming`),
// and Studio stops the frame from inside that call (`RAPIDR_STUDIO_HOST.stop`,
// ide/web/studio.js): the page's own call back into the wasm used to find
// the session's table already borrowed — and every borrow a panic leaves
// behind poisons the page (later calls panic too).
//
//   1. A program that ends by itself, run again after its end, in RUNS fresh
//      Studios (default 20): the project is in the page's file store as
//      Studio keeps it between visits, run with `?do=run.start`, the second
//      run with F5. Any panic, "unreachable", "already borrowed" or
//      uncaught error in the page or the program's frame fails it.
//   2. A page that calls straight back into the runtime from every call the
//      runtime makes to it (run, send, stop, show): the program restarted
//      and stopped over it. Nothing panics; what the page said is in the
//      program's output, in order, once.
//
// Usage (repo root, after tools/build_studio_web.sh, with Studio served as
// for the studio stage of tools/regress.sh: STUDIO_WEB_URL or
// RAPIDR_STUDIO_URL, default http://127.0.0.1:18473):
//   RUNS=20 node tests/studio_run_panic.mjs

import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const STUDIO_URL = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473").replace(/\/+$/, "");
const RUNS = Number(process.env.RUNS || 20);
const FIXTURE = join(HERE, "fixtures", "studio_run_panic");
// (a fixture project's files, as paths in the page's store: Name/main.rr, Name/Name.rrproj)
const project = (dir, name) => ["main.rr", `${name}.rrproj`].map((f) => ({ path: `${name}/${f}`, text: readFileSync(join(FIXTURE, dir, f), "utf8") }));
const ENDS = project(".", "RunPanic");
const STAYS = project("stay", "Stay");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };
const BAD = /panick|unreachable|already (mutably )?borrowed|RuntimeError/i;

const browser = await chromium.launch();
const output = (page) => page.evaluate(() => { try { return String(window.rr.rapidr_get_prop("OutputBox", "text") ?? ""); } catch { return ""; } });
async function until(fn, ms = 20000) {
  for (let waited = 0; waited < ms; waited += 100) {
    if (await fn()) return true;
    await new Promise((r) => setTimeout(r, 100));
  }
  return !!(await fn());
}

/// Studio, fresh, with `files` in the page's file store (as Studio keeps
/// them between visits) and `open` run (`?do=run.start`); what the page and
/// the program's frame report as a panic collects in `seen`.
async function studio(files, open) {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const page = await ctx.newPage();
  const seen = [];
  page.on("console", (m) => { if (BAD.test(m.text())) seen.push(`console.${m.type()}: ${m.text().split("\n").slice(0, 2).join(" ")}`); });
  page.on("pageerror", (e) => seen.push(`pageerror: ${e.message.split("\n")[0]}`));
  await page.goto(`${STUDIO_URL}/run.html`, { waitUntil: "load" });
  await page.evaluate(async (files) => {
    const root = await navigator.storage.getDirectory();
    const dir = await root.getDirectoryHandle("rapidr-studio", { create: true });
    for (const f of files) {
      const h = await dir.getFileHandle(encodeURIComponent(f.path), { create: true });
      const w = await h.createWritable();
      await w.write(new TextEncoder().encode(f.text));
      await w.close();
    }
  }, files);
  const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", open, do: "run.start" });
  await page.goto(`${STUDIO_URL}/index.html?${q}`, { waitUntil: "load" });
  return { ctx, page, seen };
}

// ── 1. A program that ends by itself, and runs again ──
const problems = [];
let ended = 0;
let reran = 0;
for (let run = 0; run < RUNS; run++) {
  const { ctx, page, seen } = await studio(ENDS, "RunPanic/RunPanic.rrproj");
  // (the program prints, shows, and ends by itself: its end is Studio's message)
  if (await until(async () => (await output(page)).includes("bye") || seen.length > 0) && !seen.length) ended++;
  // (and again: the second run of a page whose first has ended)
  if (!seen.length) {
    await page.mouse.click(900, 250);
    await page.keyboard.press("F5");
    if (await until(async () => /bye/.test(await output(page)) || seen.length > 0) && !seen.length) reran++;
  }
  await page.waitForTimeout(300);
  if (seen.length) problems.push(`run ${run}: ${seen.join(" | ")}`);
  await ctx.close();
}
ok(ended === RUNS, `the program ran and ended by itself in ${ended} of ${RUNS} runs`);
ok(reran === RUNS, `and ran again after its end in ${reran} of ${RUNS} runs`);
ok(problems.length === 0, `no panic, no "already borrowed", no uncaught error in ${RUNS} runs${problems.length ? ": " + problems.slice(0, 3).join(" || ") : ""}`);

// ── 2. The page calls straight back from every call the runtime makes to it ──
{
  const { ctx, page, seen } = await studio(STAYS, "Stay/Stay.rrproj");
  ok(await until(async () => (await output(page)).includes("up") || seen.length > 0) && !seen.length, "a program that stays up is running");
  await page.evaluate(() => {
    const h = window.RAPIDR_STUDIO_HOST;
    for (const name of ["run", "send", "stop", "show"]) {
      const real = h[name].bind(h);
      h[name] = (...args) => {
        const result = real(...args);
        // (the page's script answers inside the call: straight back into the wasm)
        window.rr.studio_session_incoming(JSON.stringify({ type: "output", stream: "stdout", text: `[page:${name}]\n` }));
        return result;
      };
    }
  });
  await page.mouse.click(900, 250);
  // (Restart twice — each stops the frame and starts another — then Stop)
  for (let k = 0; k < 2 && !seen.length; k++) {
    await page.keyboard.press("Control+Shift+F5");
    // (the frame loads the runtime again: a moment before the new program says it's up)
    await page.waitForTimeout(1500);
    await until(async () => /\bup\b/.test(await output(page)) || seen.length > 0, 8000);
    await page.waitForTimeout(500);
  }
  await page.keyboard.press("Shift+F5");
  await page.waitForTimeout(800);
  const out = await output(page);
  if (process.env.SHOW_OUTPUT) console.log(JSON.stringify(out));
  ok(seen.length === 0, `no panic when the page calls back from run, send, stop and show${seen.length ? ": " + seen.slice(0, 2).join(" | ") : ""}`);
  ok(/\[page:(run|send)\]/.test(out), "what the page said from inside those calls is in the program's output");
  // (a page that isn't poisoned runs it once more)
  await page.keyboard.press("F5");
  ok(await until(async () => /\bup\b/.test(await output(page))) && seen.length === 0, "and Studio runs the program once more after the Stop");
  await ctx.close();
}

await browser.close();
console.log(failed ? `${failed} check(s) failed` : "ok");
process.exit(failed ? 1 : 0);
