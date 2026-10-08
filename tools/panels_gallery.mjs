// A component's looks, to look at: a GUI fixture run in each theme
// (`Application.Theme` set first) at 1× and 2×, on the desktop (the UI
// kernel's headless host, an interpreted build) and on the web (the kernel
// host in Chromium), each pair saved side by side as PNG — desktop | web —
// with a line saying whether the two are byte-identical.
//
//   node tools/panels_gallery.mjs <fixture.bas> <outdir> [events] [themes]
//
// events: RAPIDR_TEST_EVENTS played before the capture ("" for none);
// themes: comma-separated (default classic,modern,dark,highcontrast).
// Needs ./rapidr built, the web artifacts (tools/build_web_artifacts.sh) and
// the repo served on RAPIDR_URL (default http://localhost:8765). Nothing
// reaches a real printer or the user's registry. Python with Pillow joins
// the pictures.
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "../tests/node_modules/playwright/index.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [fixture, outArg, events = "", themesArg = "classic,modern,dark,highcontrast"] = process.argv.slice(2);
if (!fixture || !outArg) {
  console.log("usage: node tools/panels_gallery.mjs <fixture.bas> <outdir> [events] [themes]");
  process.exit(2);
}
const OUT = resolve(outArg);
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const name = basename(fixture, ".bas");
const themes = themesArg.split(",").map((t) => t.trim()).filter(Boolean);
const src = readFileSync(fixture, "utf8");
mkdirSync(join(OUT, "_work"), { recursive: true });
const safe = { RAPIDR_PRINT_TO: join(OUT, "_work"), RAPIDR_REGISTRY: join(OUT, "_work", "registry.reg") };

const browser = await chromium.launch();
for (const theme of themes) {
  const themed = `Application.Theme = "${theme}"\n` + src;
  const file = join(OUT, "_work", `${name}-${theme}.bas`);
  writeFileSync(file, themed);
  const bin = join(OUT, "_work", `bin-${theme}`);
  execFileSync(join(ROOT, "rapidr"), ["build", file, bin, "--interp"], { cwd: ROOT, stdio: "ignore" });
  const exe = join(bin, `${name}-${theme}`);
  for (const scale of [1, 2]) {
    const tag = `${name}-${theme}@${scale}x`;
    const cap = join(OUT, "_work", tag);
    try {
      execFileSync(exe, [], { cwd: ROOT, timeout: 60000, stdio: "ignore", env: { ...process.env, ...safe, RAPIDR_SCALE: String(scale), RAPIDR_CAPTURE: cap, RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: "" } });
    } catch (e) {
      console.log(`✗ ${tag}: the desktop run failed (${e.message.split("\n")[0]})`);
    }
    const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 1920, height: 1080 } });
    let web = [];
    try {
      await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
      await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
      await page.evaluate(({ source, env }) => {
        const bc = window.rr.compile(source, "fixture", []);
        window.rr.rapidr_set_test_env(env);
        window.rr.rapidr_run_bc(bc);
      }, { source: themed, env: { RAPIDR_CAPTURE: "web", RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: "" } });
      await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: 30000, polling: 100 });
      web = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results())).captures || [];
    } catch (e) {
      console.log(`✗ ${tag}: the web run failed (${String(e.message).split("\n")[0]})`);
    }
    await page.close();
    web.forEach((shot, i) => writeFileSync(join(OUT, "_work", `${tag}-web-${i + 1}.bmp`), Buffer.from(shot.bmp, "base64")));
    const desk = readdirSync(join(OUT, "_work")).filter((f) => f.startsWith(`${tag}-`) && /^.*-\d+\.bmp$/.test(f) && !f.includes("-web-")).sort();
    desk.forEach((d, i) => {
      const w = join(OUT, "_work", `${tag}-web-${i + 1}.bmp`);
      const same = existsSync(w) && readFileSync(w).equals(readFileSync(join(OUT, "_work", d)));
      const png = join(OUT, `${tag}${desk.length > 1 ? `-${i + 1}` : ""}.png`);
      execFileSync("python3", ["-c", `
import sys
from PIL import Image, ImageDraw
a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB") if sys.argv[2] != "-" else Image.new("RGB", a.size, (255, 0, 255))
gap = 12
o = Image.new("RGB", (a.width + b.width + gap, max(a.height, b.height) + 22), (40, 40, 40))
o.paste(a, (0, 22)); o.paste(b, (a.width + gap, 22))
d = ImageDraw.Draw(o)
d.text((6, 5), "desktop", fill=(255, 255, 255)); d.text((a.width + gap + 6, 5), "web  " + sys.argv[4], fill=(255, 255, 255))
o.save(sys.argv[3])
`, join(OUT, "_work", d), existsSync(w) ? w : "-", png, same ? "(byte-identical)" : "(DIFFERENT)"]);
      console.log(`${same ? "✓" : "≠"} ${tag} window ${i + 1}: ${png}${same ? "" : "  desktop and web differ"}`);
    });
  }
}
await browser.close();
