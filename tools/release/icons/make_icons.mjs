// RapidR's icons from their SVGs (original artwork, MIT): PNGs at the sizes the
// platforms want, Windows' .ico (PNG entries), and on macOS the .icns (iconutil).
//
//   node tools/release/icons/make_icons.mjs
//
// Renders with Playwright's Chromium (tests/node_modules), on a transparent
// background. The outputs are committed beside the SVGs; run this after
// changing an SVG.
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../../..");
const require = createRequire(join(process.env.RAPIDR_NODE_MODULES || join(ROOT, "tests"), "x.js"));
const { chromium } = require("playwright");

const SIZES = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
const browser = await chromium.launch();
const page = await browser.newPage();
async function render(svg, size) {
  const text = readFileSync(join(HERE, `${svg}.svg`), "utf8");
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(`<html><body style="margin:0;background:transparent">${text.replace(/width="1024" height="1024"/, `width="${size}" height="${size}"`)}</body></html>`);
  return await page.screenshot({ omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } });
}

// An .ico of PNG entries (Windows Vista and later read them).
function ico(pngs) {
  const header = Buffer.alloc(6 + 16 * pngs.length);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(pngs.length, 4);
  let offset = header.length;
  pngs.forEach(([size, png], i) => {
    const e = 6 + 16 * i;
    header.writeUInt8(size >= 256 ? 0 : size, e);
    header.writeUInt8(size >= 256 ? 0 : size, e + 1);
    header.writeUInt16LE(1, e + 4);
    header.writeUInt16LE(32, e + 6);
    header.writeUInt32LE(png.length, e + 8);
    header.writeUInt32LE(offset, e + 12);
    offset += png.length;
  });
  return Buffer.concat([header, ...pngs.map(([, png]) => png)]);
}

for (const name of ["rapidr", "rapidr-doc"]) {
  const pngs = {};
  for (const size of SIZES) pngs[size] = await render(name, size);
  for (const size of [48, 128, 256]) writeFileSync(join(HERE, `${name}-${size}.png`), pngs[size]);
  writeFileSync(join(HERE, `${name}.ico`), ico([16, 24, 32, 48, 64, 256].map((s) => [s, pngs[s]])));
  if (process.platform === "darwin") {
    const set = join(HERE, `${name}.iconset`);
    rmSync(set, { recursive: true, force: true });
    mkdirSync(set);
    for (const s of [16, 32, 128, 256, 512]) {
      writeFileSync(join(set, `icon_${s}x${s}.png`), pngs[s]);
      writeFileSync(join(set, `icon_${s}x${s}@2x.png`), pngs[s * 2]);
    }
    execFileSync("iconutil", ["-c", "icns", set, "-o", join(HERE, `${name}.icns`)]);
    rmSync(set, { recursive: true });
  }
  console.log(`${name}: png 48/128/256, .ico${process.platform === "darwin" ? ", .icns" : ""}`);
}
await browser.close();
