// RapidR Studio's shell on both hosts (docs/ide-plan.md I1): the same
// bytecode run on the desktop's headless host (`rapidr run ide/studio.rr`,
// RAPIDR_CAPTURE) and on the web page (target/studio-web, the canvas host,
// the same test hooks), in every theme at 1× and 2×; each window's capture
// must be byte-identical and the accessibility trees equal.
//
//   tools/build_studio_web.sh                       (the page)
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_shell.mjs [filter…]
//
// RAPIDR_STUDIO_URL (default http://127.0.0.1:18473) is the page's server;
// RAPIDR_STUDIO_OUT (default tests/results/studio) gets each capture as
// <scene>-<theme>@<s>x-desktop.bmp / -web.bmp and a side-by-side
// <scene>-<theme>@<s>x.bmp (desktop | web) to look at; RAPIDR_STUDIO_SCALES
// ("1,2") and RAPIDR_STUDIO_THEMES limit the runs.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const URL_BASE = process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473";
const OUT = process.env.RAPIDR_STUDIO_OUT || join(ROOT, "tests", "results", "studio");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const SCALES = (process.env.RAPIDR_STUDIO_SCALES || "1,2").split(",").map(Number);
const THEMES = (process.env.RAPIDR_STUDIO_THEMES || "rapidr-light,rapidr-dark,rapidr-high-contrast,classic").split(",");
const filters = process.argv.slice(2);

// The scenes: what Studio opens.
const SCENES = [
  { name: "project", open: "examples/gui/hello_form.rr" },
  { name: "welcome", open: "" },
  // (Studio's own dialogs, in the chrome font: the window captured is the
  // dialog, the second shown)
  { name: "newproject", open: "", do: "file.newProject", window: 2 },
  { name: "palette", open: "", do: "view.commandPalette", window: 2 },
  // (the code editor with the completion list open and its docs beside it:
  // typed through the kernel's keys, S-EDITOR)
  { name: "editor", open: "examples/gui/hello_form.rr", do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:dim y as string,key:Escape,key:Enter,type:form.c", delay: 6 },
];

mkdirSync(OUT, { recursive: true });
const scratch = join(OUT, ".work");
mkdirSync(scratch, { recursive: true });

// ---- BMPs ------------------------------------------------------------------
function readBmp(buf) {
  const off = buf.readUInt32LE(10), w = buf.readInt32LE(18), h0 = buf.readInt32LE(22), bpp = buf.readUInt16LE(28) / 8;
  const h = Math.abs(h0), stride = (w * bpp + 3) & ~3, px = Buffer.alloc(w * h * 3);
  for (let y = 0; y < h; y++) {
    const row = h0 > 0 ? h - 1 - y : y;
    for (let x = 0; x < w; x++) {
      const i = off + row * stride + x * bpp, o = (y * w + x) * 3;
      px[o] = buf[i + 2]; px[o + 1] = buf[i + 1]; px[o + 2] = buf[i];
    }
  }
  return { w, h, px };
}

function writeBmp(img) {
  const stride = (img.w * 3 + 3) & ~3, size = 54 + stride * img.h, b = Buffer.alloc(size);
  b.write("BM", 0); b.writeUInt32LE(size, 2); b.writeUInt32LE(54, 10); b.writeUInt32LE(40, 14);
  b.writeInt32LE(img.w, 18); b.writeInt32LE(img.h, 22); b.writeUInt16LE(1, 26); b.writeUInt16LE(24, 28);
  for (let y = 0; y < img.h; y++) for (let x = 0; x < img.w; x++) {
    const o = (y * img.w + x) * 3, i = 54 + (img.h - 1 - y) * stride + x * 3;
    b[i] = img.px[o + 2]; b[i + 1] = img.px[o + 1]; b[i + 2] = img.px[o];
  }
  return b;
}

function sideBySide(a, b, gap = 16) {
  const w = a.w + gap + b.w, h = Math.max(a.h, b.h), px = Buffer.alloc(w * h * 3, 0x80);
  for (const [img, x0] of [[a, 0], [b, a.w + gap]]) for (let y = 0; y < img.h; y++) img.px.copy(px, (y * w + x0) * 3, y * img.w * 3, (y + 1) * img.w * 3);
  return { w, h, px };
}

function diff(a, b) {
  if (a.w !== b.w || a.h !== b.h) return `${a.w}×${a.h} against ${b.w}×${b.h}`;
  let n = 0;
  for (let i = 0; i < a.px.length; i += 3) if (a.px[i] !== b.px[i] || a.px[i + 1] !== b.px[i + 1] || a.px[i + 2] !== b.px[i + 2]) n++;
  return `${n} pixels differ`;
}

// ---- the desktop -------------------------------------------------------------
function runDesktop(scene, theme, scale) {
  const dir = join(scratch, `${scene.name}-${theme}@${scale}x`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", theme];
  if (scene.open) args.push(scene.open);
  if (scene.do) args.push("--do", scene.do);
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 60000,
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      ...(scene.delay ? { RAPIDR_CAPTURE_DELAY: String(scene.delay) } : {}),
      RAPIDR_SCALE: String(scale),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_A11Y: join(dir, "a11y.json"),
      RAPIDR_PRINT_TO: join(scratch, "prints"),
      RAPIDR_REGISTRY: join(scratch, "registry.reg"),
    },
  });
  const file = join(dir, `window-${scene.window || 1}.bmp`);
  if (!existsSync(file)) throw new Error(`desktop: no capture (${(r.stderr || "").trim().split("\n").pop()})`);
  return { bmp: readFileSync(file), a11y: existsSync(join(dir, "a11y.json")) ? readFileSync(join(dir, "a11y.json"), "utf8") : null };
}

// ---- the web -------------------------------------------------------------------
async function runWeb(browser, scene, theme, scale) {
  const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    await page.addInitScript((delay) => { window.RAPIDR_STUDIO_TEST = { RAPIDR_CAPTURE: "web", ...(delay ? { RAPIDR_CAPTURE_DELAY: String(delay) } : {}) }; }, scene.delay || 0);
    const q = new URLSearchParams({ theme, window: "normal", fresh: "" });
    if (scene.open) q.set("open", scene.open);
    if (scene.do) q.set("do", scene.do);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 60000, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { results, errors };
  } finally {
    await page.close();
  }
}

// ---- the runs ------------------------------------------------------------------
const browser = await chromium.launch();
let same = 0, differ = 0, failed = 0, a11ySame = 0, a11yDiffer = 0;
for (const scene of SCENES) for (const theme of THEMES) for (const scale of SCALES) {
  const name = `${scene.name}-${theme}@${scale}x`;
  if (filters.length && !filters.some((f) => name.includes(f))) continue;
  try {
    const desk = runDesktop(scene, theme, scale);
    const web = await runWeb(browser, scene, theme, scale);
    if (!web.results || !web.results.captures.length) throw new Error(`web: no capture ${web.errors.join("; ")}`);
    const shot = web.results.captures[(scene.window || 1) - 1];
    if (!shot) throw new Error(`web: no capture of window ${scene.window} (${web.results.captures.length} captured)`);
    const wb = Buffer.from(shot.bmp, "base64");
    const d = readBmp(desk.bmp), w = readBmp(wb);
    writeFileSync(join(OUT, `${name}-desktop.bmp`), desk.bmp);
    writeFileSync(join(OUT, `${name}-web.bmp`), wb);
    writeFileSync(join(OUT, `${name}.bmp`), writeBmp(sideBySide(d, w)));
    const identical = desk.bmp.equals(wb);
    identical ? same++ : differ++;
    console.log(`${identical ? "✓" : "≠"} ${name}: ${identical ? "byte-identical" : diff(d, w)}${web.errors.length ? ` (page errors: ${web.errors.join("; ")})` : ""}`);
    if (desk.a11y !== null) {
      const eq = desk.a11y === web.results.a11y;
      eq ? a11ySame++ : a11yDiffer++;
      if (!eq) {
        writeFileSync(join(OUT, `${name}-desktop.a11y.json`), desk.a11y);
        writeFileSync(join(OUT, `${name}-web.a11y.json`), web.results.a11y);
      }
      console.log(`${eq ? "✓" : "≠"} ${name}: accessibility trees ${eq ? "equal" : "differ"}`);
    }
  } catch (e) {
    failed++;
    console.log(`✗ ${name}: ${e.message}`);
  }
}
await browser.close();
console.log(`\nRapidR Studio shell: ${same} captures byte-identical desktop / web, ${differ} differ, ${failed} failed; accessibility trees equal ${a11ySame}, differ ${a11yDiffer} (captures in ${OUT})`);
process.exit(differ || failed || a11yDiffer ? 1 : 0);
