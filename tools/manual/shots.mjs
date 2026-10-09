#!/usr/bin/env node
// The user manual's screenshots (docs/manual/images/<topic>/<name>.png),
// made again from scenes whenever the UI changes. One script for every
// topic: each tools/manual/scenes/*.mjs (one file a topic, so a lane adds
// its own without touching this script) exports
//
//   export const topic = "studio-editor";   (optional: the scenes' topic)
//   export const scenes = [{
//     topic, name,              → docs/manual/images/<topic>/<name>.png
//                               (topic: the scene's own, else the file's)
//     open,                     a file Studio opens (a repository path)
//     do,                       Studio's --do steps (commands and the test
//                               steps: type:, key:, focus:, wait …)
//     setup(work, root),        (optional) prepares the scene's work folder
//                               (a copy to import …) → { open?, do? }
//     delay,                    seconds before the capture (default 6)
//     crop: [x, y, w, h],       the part of the window, in logical pixels
//                               (the image is at 2x); none: all of it
//     theme,                    (default "rapidr-light")
//     program: true,            (desktop) the running program's own window,
//                               captured from Studio, instead of Studio's
//     program: "path.rr",       (desktop) that program run on its own
//                               instead of Studio
//     window: n,                (desktop) the n-th window captured (default 1)
//     host: "desktop" | "web",  (default desktop) — the web: Studio for the
//                               web at STUDIO_WEB_URL, in Chromium
//     webFiles: [paths],        (web) the files the page's store has
//     web: async (page) => {},  (web) what to do on the page before the shot
//     viewport: { width, height } (web) }]
//
// Studio runs as the tests run it (tests/studio_flows.mjs): `rapidr run
// ide/studio.rr --fresh --theme rapidr-light --do …` with RAPIDR_SCALE=2 and
// RAPIDR_CAPTURE, a 1280 × 800 window. Programs run with RAPIDR_PRINT_TO and
// RAPIDR_REGISTRY in the scene's work folder: nothing reaches a printer or
// the system's registry. Each PNG is optimized (Pillow; a 256-colour palette
// when it would be over 250 KB) and one still over 250 KB fails (crop it
// closer). Only RapidR's own examples and code belong in shots.
//
//   cargo build --release -p rapidr-cli && cp target/release/rapidr .
//   node tools/manual/shots.mjs [filter…]   (a filter matches a scene's
//                                           topic/name or its scene file)
//
// Needs ./rapidr (or RAPIDR / RAPIDR_BIN), python3 with Pillow, and for web
// scenes Studio for the web served at STUDIO_WEB_URL
// (tools/build_studio_web.sh; python3 -m http.server -d target/studio-web)
// and the tests' Playwright (tests/node_modules). MANUAL_IMAGES=<dir> writes
// the images there instead of docs/manual/images (to try a scene).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const RAPIDR = process.env.RAPIDR || process.env.RAPIDR_BIN || join(ROOT, "rapidr");
const URL_BASE = (process.env.STUDIO_WEB_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const IMAGES = process.env.MANUAL_IMAGES || join(ROOT, "docs", "manual", "images");
const WORK = join(ROOT, "tests", "results", "manual-shots");
const LIMIT = 250 * 1024;
const SCALE = 2;
const filters = process.argv.slice(2);

if (!existsSync(RAPIDR)) {
  console.error(`${RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .`);
  process.exit(1);
}

const scenes = [];
for (const f of readdirSync(join(HERE, "scenes")).filter((f) => f.endsWith(".mjs")).sort()) {
  const m = await import(pathToFileURL(join(HERE, "scenes", f)).href);
  for (const s of m.scenes || []) scenes.push({ ...s, topic: s.topic || m.topic, file: f });
}

// (a capture → the cropped, optimized PNG; Pillow, as tools/visual/gallery.py)
const TO_PNG = `
import os, sys
from PIL import Image
src, dst, crop, scale, limit = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
im = Image.open(src).convert("RGB")
if crop:
    x, y, w, h = (int(v) * scale for v in crop.split(","))
    im = im.crop((x, y, min(x + w, im.width), min(y + h, im.height)))
im.save(dst, optimize=True)
if os.path.getsize(dst) > limit:
    # (a palette of 256 colours: Studio's flat chrome loses nothing visible)
    im.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(dst, optimize=True)
`;

// (the capture of Studio's window — or the program's — for a desktop scene)
function desktop(s, dir, open, steps) {
  const theme = s.theme || "rapidr-light";
  const args = typeof s.program === "string"
    ? ["run", s.program, "--theme", theme]
    : ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", theme, ...(steps ? ["--do", steps] : []), ...(open ? [open] : [])];
  const delay = s.delay || 6;
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    encoding: "utf8",
    timeout: delay * 1000 + 90000,
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
  const bmp = join(dir, s.program === true ? "window-program-1.bmp" : `window-${s.window || 1}.bmp`);
  if (!existsSync(bmp)) throw new Error(`no capture (${(r.stderr || "").trim().split("\n").slice(-2).join(" / ")})`);
  return bmp;
}

async function web(browser, s, dir, open, steps) {
  const ctx = await browser.newContext({ viewport: s.viewport || { width: 1440, height: 900 }, deviceScaleFactor: SCALE });
  const page = await ctx.newPage();
  try {
    const files = (s.webFiles || (open ? [open] : [])).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, { RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: String(s.delay || 6), RAPIDR_TEST_DUMP: "" });
    const q = new URLSearchParams({ theme: s.theme || "rapidr-light", fresh: "" });
    if (!s.maximized) q.set("window", "normal");
    if (steps) q.set("do", steps);
    if (open) q.set("open", open);
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
let made = 0, failed = 0;
for (const s of scenes.filter((s) => !filters.length || filters.some((f) => `${s.topic}/${s.name}`.includes(f) || s.file.includes(f)))) {
  const id = `${s.topic}/${s.name}`;
  const dir = join(WORK, `${s.topic}-${s.name}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  try {
    const extra = s.setup ? s.setup(dir, ROOT) || {} : {};
    const open = extra.open ?? s.open;
    const steps = extra.do ?? s.do;
    let src;
    if (s.host === "web") {
      // (the tests' Playwright: tests/node_modules)
      if (!browser) browser = await (await import(pathToFileURL(join(ROOT, "tests", "node_modules", "playwright", "index.mjs")).href)).chromium.launch();
      src = await web(browser, s, dir, open, steps);
    } else {
      src = desktop(s, dir, open, steps);
    }
    const out = join(IMAGES, s.topic, `${s.name}.png`);
    mkdirSync(dirname(out), { recursive: true });
    const r = spawnSync("python3", ["-c", TO_PNG, src, out, (s.crop || []).join(","), String(SCALE), String(LIMIT)], { encoding: "utf8" });
    if (r.status !== 0 || !existsSync(out)) throw new Error((r.stderr || "").trim().split("\n").pop());
    const size = statSync(out).size;
    const over = size > LIMIT;
    if (over) failed++;
    else made++;
    console.log(`${over ? "✗" : "✓"} ${id}: ${Math.round(size / 1024)} KB${over ? " (over 250 KB: crop it closer)" : ""}`);
  } catch (e) {
    failed++;
    console.log(`✗ ${id}: ${e.message}`);
  }
}
if (browser) await browser.close();
console.log(`\nManual shots: ${made} made${failed ? `, ${failed} failed` : ""} (${IMAGES})`);
process.exit(failed ? 1 : 0);
