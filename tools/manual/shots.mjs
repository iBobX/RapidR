#!/usr/bin/env node
// The manual's screenshots (docs/manual/images/<topic>/<name>.png), made
// again from scenes whenever the UI changes: every tools/manual/scenes/*.mjs
// exports `scenes`, a list of
//
//   { topic, name,              → docs/manual/images/<topic>/<name>.png
//     open,                     a file to open (a repository path)
//     do,                       Studio's --do steps (commands and test steps)
//     delay,                    seconds before the capture (default 6)
//     crop: [x, y, w, h],       the part of Studio's window, in logical
//                               pixels (the image is at 2x); none: all of it
//     host: "desktop" | "web",  (default desktop) — the web: Studio for the
//                               web at STUDIO_WEB_URL, in Chromium
//     program: true,            (desktop) the running program's own window
//                               instead of Studio's
//     webFiles: [paths],        (web) the files the page's store has
//     web: async (page) => {},  (web) what to do on the page before the shot
//     viewport: { width, height } (web) }
//
// Studio runs as the tests run it (tests/studio_flows.mjs): `rapidr run
// ide/studio.rr --fresh --theme rapidr-light --do …` with RAPIDR_SCALE=2 and
// RAPIDR_CAPTURE; light theme, a 1280 × 800 window. Images over 250 KB are
// reported (crop them closer).
//
//   node tools/manual/shots.mjs [filter…]      (a filter matches topic/name)
//
// Needs ./rapidr (or RAPIDR), macOS's sips for the desktop's PNGs, and for
// web scenes Studio for the web served at STUDIO_WEB_URL
// (tools/build_studio_web.sh; python3 -m http.server -d target/studio-web).

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const URL_BASE = (process.env.STUDIO_WEB_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const IMAGES = join(ROOT, "docs", "manual", "images");
const WORK = join(ROOT, "tests", "results", "manual-shots");
const LIMIT = 250 * 1024;
const SCALE = 2;
const filters = process.argv.slice(2);

const sceneDir = join(ROOT, "tools", "manual", "scenes");
const scenes = [];
for (const f of readdirSync(sceneDir).filter((f) => f.endsWith(".mjs")).sort()) {
  const m = await import(pathToFileURL(join(sceneDir, f)).href);
  scenes.push(...(m.scenes || []));
}

function sips(args) {
  const r = spawnSync("sips", args, { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`sips ${args.join(" ")}: ${r.stderr}`);
}

// (the 2x PNG of Studio's window — or the program's — for a desktop scene)
function desktop(s, dir) {
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", s.theme || "rapidr-light"];
  if (s.do) args.push("--do", s.do);
  if (s.open) args.push(s.open);
  const delay = s.delay || 6;
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    encoding: "utf8",
    timeout: delay * 1000 + 60000,
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(delay),
      RAPIDR_MENU: "window",
      RAPIDR_SCALE: String(SCALE),
      RAPIDR_PRINT_TO: join(dir, "prints"),
      RAPIDR_REGISTRY: join(dir, "studio.reg"),
    },
  });
  const bmp = join(dir, s.program ? "window-program-1.bmp" : "window-1.bmp");
  if (!existsSync(bmp)) throw new Error(`no capture (${(r.stderr || "").trim().split("\n").slice(-2).join(" / ")})`);
  const png = join(dir, "whole.png");
  sips(["-s", "format", "png", bmp, "--out", png]);
  return png;
}

async function web(browser, s, dir) {
  const ctx = await browser.newContext({ viewport: s.viewport || { width: 1440, height: 900 }, deviceScaleFactor: SCALE });
  const page = await ctx.newPage();
  try {
    const files = (s.webFiles || (s.open ? [s.open] : [])).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, { RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: String(s.delay || 6), RAPIDR_TEST_DUMP: "" });
    const q = new URLSearchParams({ theme: s.theme || "rapidr-light", fresh: "" });
    if (!s.maximized) q.set("window", "normal");
    if (s.do) q.set("do", s.do);
    if (s.open) q.set("open", s.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: (s.delay || 6) * 1000 + 60000, polling: 200 });
    if (s.web) await s.web(page);
    const png = join(dir, "whole.png");
    await page.screenshot({ path: png });
    return png;
  } finally {
    await ctx.close();
  }
}

mkdirSync(WORK, { recursive: true });
let browser = null;
let made = 0, failed = 0, big = 0;
for (const s of scenes.filter((s) => !filters.length || filters.some((f) => `${s.topic}/${s.name}`.includes(f)))) {
  const id = `${s.topic}/${s.name}`;
  const dir = join(WORK, `${s.topic}-${s.name}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  try {
    let png;
    if (s.host === "web") {
      // (the tests' Playwright: tests/node_modules)
      if (!browser) browser = await (await import(pathToFileURL(join(ROOT, "tests", "node_modules", "playwright", "index.mjs")).href)).chromium.launch();
      png = await web(browser, s, dir);
    } else {
      png = desktop(s, dir);
    }
    const out = join(IMAGES, s.topic, `${s.name}.png`);
    mkdirSync(dirname(out), { recursive: true });
    if (s.crop) {
      const [x, y, w, h] = s.crop.map((v) => v * SCALE);
      sips(["-c", String(h), String(w), "--cropOffset", String(y), String(x), png, "--out", out]);
    } else {
      copyFileSync(png, out);
    }
    const size = statSync(out).size;
    const over = size > LIMIT;
    if (over) big++;
    made++;
    console.log(`${over ? "!" : "✓"} ${id}: ${Math.round(size / 1024)} KB${over ? " (over 250 KB: crop it closer)" : ""}`);
  } catch (e) {
    failed++;
    console.log(`✗ ${id}: ${e.message}`);
  }
}
if (browser) await browser.close();
console.log(`\nManual shots: ${made} made${big ? `, ${big} over 250 KB` : ""}${failed ? `, ${failed} failed` : ""} (docs/manual/images)`);
process.exit(failed ? 1 : 0);
