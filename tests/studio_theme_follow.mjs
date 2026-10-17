// RapidR Studio follows the look (Robert, Preview 2 on macOS in Dark:
// "Welcome screen is not responding to theme changes, it's always light").
//
// 1. The guard: Studio started following the system in one look, the
//    system switched to another while it runs (RAPIDR_TEST_SYSTEM_LOOK and
//    a `__look_` step: rapidr_ui_app's testhooks), then captured — must be
//    byte-identical to Studio started in that look. A pixel apart is a
//    colour, an icon or a size Studio took from the old look and kept.
//    The scenes: the Welcome page (and scrolled to its last card by the
//    keyboard), the designer, Settings, New Project and Project Options
//    open while the look changes, the debugger paused (Variables, Watch
//    with values marked changed, a data tip), the save question; on the
//    desktop's headless host and on the web page (the same hooks). The
//    flipped captures' text is judged too (rapidr_ui_kernel::contrast:
//    WCAG AA in every RapidR look).
// 2. The web's own path: the page's colour scheme switched by the browser
//    (playwright's emulateMedia; prefers-color-scheme, forced-colors) on a
//    live page — Studio follows without a reload, the same as a page
//    opened in that scheme.
//
//   tools/build_studio_web.sh
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_theme_follow.mjs [filter…]
//
// STUDIO_WEB_URL (default http://127.0.0.1:18473/) is the page's server;
// STUDIO_FOLLOW_HOSTS=desktop or web runs one host; the captures that
// differ are left in tests/results/studio-follow.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const OUT = join(ROOT, "tests", "results", "studio-follow");
const HOSTS = (process.env.STUDIO_FOLLOW_HOSTS || "desktop,web").split(",");
const filters = process.argv.slice(2);

const SHOP = "examples/debugging/shop.rr";
// (a scene: what Studio opens and does; `flips`: [from, to] system looks;
// `window`: which window is captured — 2, a dialog over Studio; `pre`:
// test events before the switch)
const SCENES = [
  { name: "welcome", flips: [["light", "dark"], ["dark", "light"], ["dark", "contrast"], ["contrast", "light"]], delay: 4 },
  { name: "welcome-scrolled", pre: "projecttree.__key_9_256", flips: [["light", "dark"]], delay: 4 },
  { name: "designer", open: "examples/gui/hello_form.rr", do: "wait,view.designer", flips: [["light", "dark"]], delay: 5 },
  { name: "settings", open: "examples/gui/hello_form.rr", do: "view.settings", window: 2, flips: [["light", "dark"], ["dark", "contrast"]], delay: 4 },
  { name: "new-project", do: "file.newProject", window: 2, flips: [["light", "dark"]], delay: 4 },
  {
    name: "project-options", open: "tests/fixtures/studio_app/Notes.rrproj", copyDir: true, do: "project.options", window: 2, flips: [["dark", "contrast"]], delay: 4,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
  },
  { name: "debug-variables", open: SHOP, do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,wait", flips: [["light", "dark"]], delay: 8 },
  {
    name: "debug-watch", open: SHOP, delay: 10, flips: [["dark", "contrast"], ["contrast", "light"]],
    do: "line:76,debug.toggleBreakpoint,watch:total,watch:count,watch:orders(1).Buyer.Name,run.start,wait,wait,wait,run.start,wait,wait,wait,view.watch,wait,changed:report",
  },
  { name: "debug-datatip", open: SHOP, do: "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:76:37,wait,wait,wait,tipopen:1,wait,wait,tipopen:4,wait,wait", flips: [["light", "dark"]], delay: 9 },
  {
    name: "save-prompt", open: "tests/fixtures/studio_save/main.rr", copy: true, do: 'wait,code:"ONE"=>"ONE saved",file.exit', window: 2, flips: [["light", "dark"]], delay: 4,
    webFiles: ["tests/fixtures/studio_save/main.rr", "tests/fixtures/studio_save/other.inc"],
  },
];

mkdirSync(OUT, { recursive: true });
const work = join(OUT, ".work");
mkdirSync(work, { recursive: true });

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

// How two captures differ: "" when identical, else what and where.
function differ(a, b) {
  const A = readBmp(a), B = readBmp(b);
  if (A.w !== B.w || A.h !== B.h) return `${A.w}×${A.h} against ${B.w}×${B.h}`;
  let n = 0, x0 = Infinity, y0 = Infinity, x1 = -1, y1 = -1;
  for (let i = 0; i < A.px.length; i += 3) {
    if (A.px[i] === B.px[i] && A.px[i + 1] === B.px[i + 1] && A.px[i + 2] === B.px[i + 2]) continue;
    n++;
    const p = i / 3, x = p % A.w, y = (p - x) / A.w;
    x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
  }
  return n ? `${n} pixels differ in x ${x0}..${x1}, y ${y0}..${y1}` : "";
}

// (a run's text below WCAG AA, from its contrast report)
function unreadable(report) {
  const windows = report ? JSON.parse(report) : [];
  const checked = windows.reduce((n, w) => n + w.checked, 0);
  const bad = windows.flatMap((w) => w.failures.map((f) => `${w.form}: ${JSON.stringify(f.text)} ${f.color} on ${f.background} ${f.ratio}:1`));
  return { checked, bad };
}

function runDesktop(scene, look, events, tag) {
  const dir = join(work, `${scene.name}-${tag}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr"];
  if (scene.do) args.push("--do", scene.do);
  if (scene.open && scene.copyDir) {
    // (Project Options writes its icon preview beside the project: a copy)
    spawnSync("cp", ["-R", join(ROOT, dirname(scene.open)), join(dir, "project")]);
    args.push(join(dir, "project", scene.open.split("/").pop()));
  } else if (scene.open && scene.copy) {
    for (const f of scene.webFiles || [scene.open]) spawnSync("cp", [join(ROOT, f), dir]);
    args.push(join(dir, scene.open.split("/").pop()));
  } else if (scene.open) args.push(scene.open);
  spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: 120000,
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(scene.delay),
      RAPIDR_SCALE: "1",
      RAPIDR_MENU: "window",
      RAPIDR_TEST_SYSTEM_LOOK: look,
      RAPIDR_TEST_EVENTS: events,
      RAPIDR_TEST_CONTRAST: join(dir, "contrast.json"),
      RAPIDR_PRINT_TO: join(work, "prints"),
      RAPIDR_REGISTRY: join(dir, "registry.reg"),
      RAPIDR_AI_KEYSTORE: "memory",
    },
  });
  const file = join(dir, `window-${scene.window || 1}.bmp`);
  if (!existsSync(file)) throw new Error(`desktop (${tag}): no capture of window ${scene.window || 1}`);
  return { bmp: readFileSync(file), contrast: existsSync(join(dir, "contrast.json")) ? readFileSync(join(dir, "contrast.json"), "utf8") : null };
}

async function runWeb(browser, scene, look, events) {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = (scene.webFiles || []).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      RAPIDR_CAPTURE_DELAY: String(scene.delay),
      RAPIDR_TEST_EVENTS: events,
      RAPIDR_TEST_SYSTEM_LOOK: look,
    });
    const q = new URLSearchParams({ theme: "rapidr", window: "normal", fresh: "" });
    if (scene.open) q.set("open", scene.open);
    if (scene.do) q.set("do", scene.do);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 90000, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    const shot = results.captures[(scene.window || 1) - 1];
    if (!shot) throw new Error(`web: no capture of window ${scene.window || 1} (${results.captures.length} captured)${errors.length ? "; " + errors.join("; ") : ""}`);
    return { bmp: Buffer.from(shot.bmp, "base64"), contrast: results.contrast || null };
  } finally {
    await page.close();
  }
}

let passed = 0, failed = 0;
const record = (name, ok, what) => {
  ok ? passed++ : failed++;
  console.log(`${ok ? "✓" : "✗"} ${name}: ${what}`);
};

const browser = HOSTS.includes("web") ? await chromium.launch() : null;
for (const scene of SCENES) {
  for (const [from, to] of scene.flips) {
    const name = `${scene.name}-${from}-to-${to}`;
    if (filters.length && !filters.some((f) => name.includes(f))) continue;
    const pre = scene.pre ? scene.pre + "," : "";
    for (const host of HOSTS) {
      try {
        const flipped = host === "desktop" ? runDesktop(scene, from, `${pre}studio.__look_${to}`, `${from}-${to}-flipped`) : await runWeb(browser, scene, from, `${pre}studio.__look_${to}`);
        const direct = host === "desktop" ? runDesktop(scene, to, `${pre}studio.__hold_1`, `${to}-direct`) : await runWeb(browser, scene, to, `${pre}studio.__hold_1`);
        const d = differ(flipped.bmp, direct.bmp);
        if (d) {
          writeFileSync(join(OUT, `${name}-${host}-flipped.bmp`), flipped.bmp);
          writeFileSync(join(OUT, `${name}-${host}-direct.bmp`), direct.bmp);
        }
        record(`${name} (${host})`, !d, d ? `the switched Studio isn't the one started in ${to}: ${d}` : `switched to ${to} while it runs, it is exactly Studio started in ${to}`);
        const { checked, bad } = unreadable(flipped.contrast);
        record(`${name} (${host})`, checked > 0 && bad.length === 0, `text contrast after the switch, ${checked} runs${bad.length ? `, below AA:\n    ${bad.slice(0, 8).join("\n    ")}` : " all AA"}`);
      } catch (e) {
        record(`${name} (${host})`, false, e.message.split("\n")[0]);
      }
    }
  }
}

// (2) The browser switches the page's colour scheme: a live page (no test
// script: the page's own media queries, followed as they change), against
// pages opened in that scheme.
if (browser && (!filters.length || filters.some((f) => "web-color-scheme".includes(f)))) {
  const name = "web-color-scheme";
  const prop = (page, c, p) => page.evaluate(([c, p]) => String(window.rr.rapidr_get_prop(c, p)), [c, p]);
  const open = async (scheme) => {
    const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 }, colorScheme: scheme.colorScheme || "light", forcedColors: scheme.forcedColors || "none" });
    const page = await ctx.newPage();
    await page.addInitScript(() => { window.RAPIDR_STUDIO_TEST = {}; });
    const q = new URLSearchParams({ theme: "rapidr", window: "normal", fresh: "" });
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && String(window.rr.rapidr_get_prop("studio", "caption")) === "RapidR Studio", null, { timeout: 90000, polling: 200 });
    await page.waitForTimeout(1500);
    return { ctx, page };
  };
  const until = async (page, what, want, ms = 8000) => {
    const t0 = Date.now();
    let v = "";
    while (Date.now() - t0 < ms) {
      v = await what();
      if (v === want) return v;
      await page.waitForTimeout(150);
    }
    return v;
  };
  try {
    const live = await open({ colorScheme: "light" });
    const look = () => prop(live.page, "application", "theme");
    record(name, (await look()) === "rapidr light", `a page in the light scheme opens Studio light (${await look()})`);
    const steps = [
      { media: { colorScheme: "dark" }, want: "rapidr dark", ref: { colorScheme: "dark" } },
      { media: { forcedColors: "active" }, want: "rapidr high contrast", ref: { colorScheme: "dark", forcedColors: "active" } },
      { media: { forcedColors: "none", colorScheme: "light" }, want: "rapidr light", ref: { colorScheme: "light" } },
    ];
    for (const s of steps) {
      await live.page.emulateMedia(s.media);
      const got = await until(live.page, look, s.want);
      record(name, got === s.want, `the browser switched to ${JSON.stringify(s.media)}: Studio follows without a reload (${got})`);
      await live.page.waitForTimeout(800);
      const ref = await open(s.ref);
      const [a, b] = [await prop(live.page, "welcome", "color"), await prop(ref.page, "welcome", "color")];
      const [ca, cb] = [await prop(live.page, "excard(0)", "color"), await prop(ref.page, "excard(0)", "color")];
      record(name, a === b && ca === cb, `the Welcome page and its cards are in the colours of a page opened that way (page ${a} / ${b}, card ${ca} / ${cb})`);
      const [sa, sb] = [await live.page.screenshot(), await ref.page.screenshot()];
      const same = sa.equals(sb);
      if (!same) {
        writeFileSync(join(OUT, `${name}-${s.want.replace(/ /g, "-")}-live.png`), sa);
        writeFileSync(join(OUT, `${name}-${s.want.replace(/ /g, "-")}-opened.png`), sb);
      }
      record(name, same, `the whole page is pixel for pixel the one opened in ${s.want}`);
      await ref.ctx.close();
    }
    await live.page.screenshot({ path: join(OUT, `${name}-back-to-light.png`) });
    await live.ctx.close();
  } catch (e) {
    record(name, false, e.message.split("\n")[0]);
  }
}
if (browser) await browser.close();
console.log(`\nStudio follows the look: ${passed} passed, ${failed} failed (differences in ${OUT})`);
process.exit(failed ? 1 : 0);
