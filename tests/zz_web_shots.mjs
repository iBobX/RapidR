// node scratch/fontvm/web_shots.mjs OUTDIR PREFIX — the spacing case and the
// notepad / menus / pantry examples in the browser (the UI kernel on the
// canvas), page screenshots at devicePixelRatio 1, 1.5 and 2.
import { readFileSync, mkdirSync } from "node:fs";
import { chromium } from "playwright";
const [out, prefix] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const base = process.env.RAPIDR_URL || "http://localhost:8791";
const R = "/Users/roanbema/Programming/rust/RapidR/.claude/worktrees/agent-a6e44cf1b3aea265a/";
const progs = { spacing: "tests/visual/cases/spacing.bas", notepad: "examples/rapidq/notepad.bas", menus: "examples/gui/menus.rr", pantry: "examples/gui/pantry.rr" };
const browser = await chromium.launch();
for (const dpr of [1, 1.5, 2]) {
  for (const [name, path] of Object.entries(progs)) {
    const page = await browser.newPage({ deviceScaleFactor: dpr, viewport: { width: 560, height: 420 } });
    await page.goto(`${base}/tests/web_kernel.html`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
    const source = readFileSync(R + path, "utf8");
    await page.evaluate(({ source, name }) => {
      const bc = window.rr.compile(source, name, {});
      window.rr.rapidr_set_test_env({ RAPIDR_THEME: "classic" });
      window.rr.rapidr_run_bc(bc);
    }, { source, name });
    await page.waitForTimeout(2500);
    await page.screenshot({ path: `${out}/${prefix}-${name}@${dpr}x.png` });
    await page.close();
  }
}
await browser.close();
console.log("done");
