// The user manual's screenshots, made again whenever the UI changes
// (docs/manual/images/<topic>/<name>.png): every tools/manual/scenes/*.mjs
// exports `scenes`, each one RapidR Studio (or a program) driven through
// the test hooks, captured at 2x in RapidR's light look, cropped, made a
// palette PNG (under 250 KB) by tools/manual/png.py.
//
//   node tools/manual/shots.mjs [filter…]
//
// A scene: { topic, name, studio?: { open, do, delay } | run?: { dir, file,
// events, delay, window }, crop?: [x, y, w, h] (logical pixels), window? }.
// `do` may name {dir}: the scene's own folder for a new project (relative,
// so Studio shows a short path). Output: what was written, one line each.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "manual-shots");
const IMAGES = join(ROOT, "docs", "manual", "images");
const filters = process.argv.slice(2);

const scenes = [];
for (const f of readdirSync(join(ROOT, "tools", "manual", "scenes")).filter((f) => f.endsWith(".mjs")).sort()) {
  const mod = await import(pathToFileURL(join(ROOT, "tools", "manual", "scenes", f)).href);
  scenes.push(...(mod.scenes || []));
}

const env = (dir, extra) => ({
  ...process.env,
  RAPIDR_SCALE: "2",
  RAPIDR_MENU: "window",
  RAPIDR_PRINT_TO: join(WORK, "print"),
  RAPIDR_REGISTRY: join(dir, "registry.reg"),
  ...extra,
});

let failed = 0;
for (const s of scenes) {
  const id = `${s.topic}/${s.name}`;
  if (filters.length && !filters.some((f) => id.includes(f))) continue;
  const dir = join(WORK, s.topic, s.name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  let r;
  if (s.studio) {
    // (Studio run in the scene's folder: a new project goes in "Projects")
    const args = ["run", join(ROOT, "ide", "studio.rr"), "--home", ROOT, "--fresh", "--theme", "rapidr-light"];
    if (s.studio.do) args.push("--do", s.studio.do.replaceAll("{dir}", "Projects"));
    if (s.studio.open) args.push(join(ROOT, s.studio.open));
    r = spawnSync(RAPIDR, args, { cwd: dir, encoding: "utf8", timeout: 300000, env: env(dir, { RAPIDR_CAPTURE: join(dir, "shot"), RAPIDR_CAPTURE_DELAY: String(s.studio.delay || 6) }) });
  } else if (s.run) {
    // (a program a scene before made: run in its folder, its windows captured)
    const cwd = join(WORK, s.run.from, s.run.dir);
    r = spawnSync(RAPIDR, ["run", s.run.file], { cwd, encoding: "utf8", timeout: 120000, env: env(dir, { RAPIDR_CAPTURE: join(dir, "shot"), RAPIDR_CAPTURE_DELAY: String(s.run.delay || 2), RAPIDR_TEST_EVENTS: s.run.events || "" }) });
  }
  const shot = join(dir, `shot-${s.window || 1}.bmp`);
  if (!existsSync(shot)) {
    failed++;
    console.log(`✗ ${id}: no capture (${((r && r.stderr) || "").trim().split("\n").pop()})`);
    continue;
  }
  const out = join(IMAGES, s.topic, `${s.name}.png`);
  mkdirSync(dirname(out), { recursive: true });
  const png = spawnSync("python3", [join(ROOT, "tools", "manual", "png.py"), shot, out, ...(s.crop || []).map(String)], { encoding: "utf8" });
  if (png.status !== 0) {
    failed++;
    console.log(`✗ ${id}: ${png.stderr.trim()}`);
    continue;
  }
  console.log(`✓ ${id}: ${png.stdout.trim()}`);
}
process.exit(failed ? 1 : 0);
