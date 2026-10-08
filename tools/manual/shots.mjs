// The manual's screenshots, made again whenever the UI changes
// (docs/manual/images/<topic>/*.png: 2x, light theme, optimized).
//
//   node tools/manual/shots.mjs [scene-file-filter …]
//
// Every tools/manual/scenes/*.mjs exports `scenes`: a list of
//   { topic, name, open?, do?, delay?, setup?(work) → { open?, do? } }
// Each scene runs RapidR Studio on the desktop's headless host through the
// test hooks (`rapidr run ide/studio.rr --do …`, RAPIDR_CAPTURE) at scale 2,
// and its capture becomes docs/manual/images/<topic>/<name>.png (Pillow:
// quantized and optimized to stay small). Only RapidR's own examples and
// fixtures are used. Needs ./rapidr (cargo build --release -p rapidr-cli).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(dirname(HERE));
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "manual");
const filters = process.argv.slice(2);

const files = readdirSync(join(HERE, "scenes")).filter((f) => f.endsWith(".mjs") && (!filters.length || filters.some((x) => f.includes(x))));
let made = 0, failed = 0;
for (const f of files) {
  const { scenes } = await import(pathToFileURL(join(HERE, "scenes", f)).href);
  for (const s of scenes) {
    const work = join(WORK, `${s.topic}-${s.name}`);
    rmSync(work, { recursive: true, force: true });
    mkdirSync(work, { recursive: true });
    const extra = s.setup ? s.setup(work, ROOT) : {};
    const open = extra.open ?? s.open;
    const doList = extra.do ?? s.do;
    const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", s.theme || "rapidr-light"];
    if (doList) args.push("--do", doList);
    if (open) args.push(open);
    const r = spawnSync(RAPIDR, args, {
      cwd: ROOT, encoding: "utf8", timeout: 120000,
      env: {
        ...process.env, RAPIDR_SCALE: "2", RAPIDR_MENU: "window",
        RAPIDR_CAPTURE: join(work, "shot"), RAPIDR_CAPTURE_DELAY: String(s.delay || 5), RAPIDR_TEST_DUMP: "proj.name",
        RAPIDR_PRINT_TO: join(work, "prints"), RAPIDR_REGISTRY: join(work, "registry.reg"),
      },
    });
    const bmp = join(work, "shot-1.bmp");
    const out = join(ROOT, "docs", "manual", "images", s.topic, `${s.name}.png`);
    mkdirSync(dirname(out), { recursive: true });
    if (!existsSync(bmp)) {
      failed++;
      console.log(`✗ ${s.topic}/${s.name}: no capture (${(r.stderr || "").split("\n")[0]})`);
      continue;
    }
    const py = spawnSync("python3", ["-c",
      "import sys\nfrom PIL import Image\nim = Image.open(sys.argv[1]).convert('RGB')\nim.quantize(colors=256, method=Image.Quantize.MEDIANCUT).save(sys.argv[2], optimize=True)",
      bmp, out], { encoding: "utf8" });
    if (py.status !== 0) {
      failed++;
      console.log(`✗ ${s.topic}/${s.name}: ${py.stderr.trim().split("\n").pop()}`);
      continue;
    }
    made++;
    console.log(`✓ ${s.topic}/${s.name}.png`);
  }
}
console.log(`\nmanual shots: ${made} written, ${failed} failed`);
process.exit(failed ? 1 : 0);
