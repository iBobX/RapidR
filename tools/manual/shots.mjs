// The user manual's screenshots (docs/manual/images/<topic>/<name>.png),
// made again whenever the UI changes: each scene runs RapidR Studio (or a
// program) on the desktop's headless host through the test hooks — the
// same ones tests/studio_flows.mjs drives (`--do` steps: commands,
// `type:`, `key:`, `focus:`, `wait`) — captures its window at 2×, crops
// it and writes an optimized PNG (under 250 KB).
//
//   cargo build --release -p rapidr-cli && cp target/release/rapidr .
//   node tools/manual/shots.mjs [topic-or-scene …]
//
// Scenes live in tools/manual/scenes/<topic>.mjs, one file a topic (so
// lanes add theirs without touching this script). Each file exports
//
//   export const topic = "studio-editor";      // the images' folder
//   export const scenes = [{
//     name: "completion",                      // <name>.png
//     open: "examples/gui/hello_form.rr",      // what Studio opens
//     do: "focus:codedoc(0),type:form.c",      // Studio's --do steps
//     delay: 6,                                // seconds before the capture
//     crop: [244, 60, 796, 700],               // logical x, y, w, h (optional)
//     theme: "rapidr-light",                   // optional (default light)
//     program: "examples/gui/hello_form.rr",   // optional: run this instead of Studio
//   }];
//
// Programs run with RAPIDR_PRINT_TO and RAPIDR_REGISTRY in a scratch
// folder: nothing reaches a printer or the system's registry. Only
// RapidR's own examples and code belong in shots.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, "rapidr");
const IMAGES = join(ROOT, "docs", "manual", "images");
const WORK = join(ROOT, "tests", "results", "manual-shots");
const SCALE = 2;
const MAX_BYTES = 250 * 1024;
const filters = process.argv.slice(2);

if (!existsSync(RAPIDR)) {
  console.error(`${RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .`);
  process.exit(1);
}

// (BMP → cropped, optimized PNG; Pillow, as tools/visual/gallery.py)
const TO_PNG = `
import sys
from PIL import Image
src, dst, crop, scale, limit = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
im = Image.open(src).convert("RGB")
if crop:
    x, y, w, h = (int(v) * scale for v in crop.split(","))
    im = im.crop((x, y, min(x + w, im.width), min(y + h, im.height)))
im.save(dst, optimize=True)
import os
if os.path.getsize(dst) > limit:
    # (a palette of 256 colours: Studio's flat chrome loses nothing visible)
    im.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(dst, optimize=True)
`;

let made = 0, failed = 0;
const files = readdirSync(join(HERE, "scenes")).filter((f) => f.endsWith(".mjs")).sort();
for (const f of files) {
  const mod = await import(pathToFileURL(join(HERE, "scenes", f)).href);
  for (const s of mod.scenes) {
    if (filters.length && !filters.some((x) => x === mod.topic || x === s.name || `${mod.topic}/${s.name}` === x)) continue;
    const dir = join(WORK, mod.topic, s.name);
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    const args = s.program
      ? ["run", s.program, "--theme", s.theme || "rapidr-light"]
      : ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", s.theme || "rapidr-light", ...(s.open ? [s.open] : []), ...(s.do ? ["--do", s.do] : [])];
    spawnSync(RAPIDR, args, {
      cwd: ROOT,
      timeout: 120000,
      encoding: "utf8",
      env: {
        ...process.env,
        RAPIDR_CAPTURE: join(dir, "window"),
        RAPIDR_CAPTURE_DELAY: String(s.delay || 4),
        RAPIDR_SCALE: String(SCALE),
        RAPIDR_MENU: "window",
        RAPIDR_PRINT_TO: join(WORK, "prints"),
        RAPIDR_REGISTRY: join(WORK, "registry.reg"),
      },
    });
    const bmp = join(dir, `window-${s.window || 1}.bmp`);
    if (!existsSync(bmp)) {
      failed++;
      console.log(`✗ ${mod.topic}/${s.name}: no capture`);
      continue;
    }
    mkdirSync(join(IMAGES, mod.topic), { recursive: true });
    const png = join(IMAGES, mod.topic, `${s.name}.png`);
    const r = spawnSync("python3", ["-c", TO_PNG, bmp, png, (s.crop || []).join(","), String(SCALE), String(MAX_BYTES)], { encoding: "utf8" });
    if (r.status !== 0 || !existsSync(png)) {
      failed++;
      console.log(`✗ ${mod.topic}/${s.name}: ${(r.stderr || "").trim().split("\n").pop()}`);
      continue;
    }
    const kb = Math.round(statSync(png).size / 1024);
    made++;
    console.log(`${kb > 250 ? "✗" : "✓"} ${mod.topic}/${s.name}.png (${kb} KB)`);
    if (kb > 250) failed++;
  }
}
console.log(`\nmanual shots: ${made} written, ${failed} failed`);
process.exit(failed ? 1 : 0);
