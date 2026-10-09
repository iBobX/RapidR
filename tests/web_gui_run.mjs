// The GUI fixtures of tests/gui_parity_cases.mjs in the browser, as
// tests/web_gui_parity.mjs and tests/web_a11y.mjs run them: on the web
// runtime's GUI host, the UI kernel (docs/web-host-plan.md), with the
// desktop's own test hooks.
//
// The fixture runs in tests/web_kernel.html with the desktop's test hooks
// (rapidr_ui_app::script: RAPIDR_TEST_EVENTS fired through the kernel's
// routing as on the desktop, RAPIDR_TEST_DUMP, the resize and the
// splitter, the dialogs' answers), given to the runtime as its environment
// (rapidr_set_test_env). When the script ends the page has the dump lines,
// each window's accessibility tree and its capture (the wasm's pixels, as
// the desktop's RAPIDR_CAPTURE BMP) — rapidr_test_results — and the
// program has ended.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
export const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

// The fixtures' files ($RESOURCE, pictures; QVIDEO's clips in
// fixtures/video) are the program's assets.
function fixtureAssets() {
  const assetDirs = [join(HERE, "fixtures"), join(HERE, "fixtures/video"), join(HERE, "conformance/cases/resource_files")];
  const assets = [];
  for (const dir of assetDirs) {
    for (const f of readdirSync(dir)) {
      if (!statSync(join(dir, f)).isFile() || f.endsWith(".bas")) continue;
      assets.push({ name: f, mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(join(dir, f)).toString("base64") });
    }
  }
  return assets;
}

/// The case's environment for the test hooks, as tests/native_gui_events.mjs
/// gives the desktop's.
export function hookEnv(c) {
  // (RapidQ's look, named: the cases' expected lines are RC.EXE's — as the
  // desktop's harnesses run them)
  const env = { RAPIDR_THEME: process.env.RAPIDR_THEME || "classic", RAPIDR_CAPTURE: "web", RAPIDR_TEST_EVENTS: c.events, RAPIDR_TEST_DUMP: c.dump, RAPIDR_TEST_RESIZE: c.resize || "", RAPIDR_TEST_SPLIT: c.split || "" };
  const opt = { fileDialog: "RAPIDR_TEST_FILE_DIALOG", colorDialog: "RAPIDR_TEST_COLOR_DIALOG", fontDialog: "RAPIDR_TEST_FONT_DIALOG", messageDialog: "RAPIDR_TEST_MESSAGE_DIALOG", dialogHold: "RAPIDR_TEST_DIALOG_HOLD", delay: "RAPIDR_CAPTURE_DELAY", joystick: "RAPIDR_TEST_JOYSTICK", drop: "RAPIDR_TEST_DROP" };
  for (const [k, v] of Object.entries(opt)) if (c[k] !== undefined) env[v] = String(c[k]);
  if (process.env.RAPIDR_TEST_HTTP) env.RAPIDR_TEST_HTTP = process.env.RAPIDR_TEST_HTTP;
  return env;
}

/// The fixtures' files by name, as data URLs (the compiler's assets).
export const assetMap = () => Object.fromEntries(fixtureAssets().map((a) => [a.name, a.dataUrl]));

/// Case `c` on the kernel host, in a page of its own (`dpr`: the screen's
/// scale; the viewport — the screen — the desktop's headless host's,
/// 1920 × 1080): `{ results: {dump, a11y, captures} | null, errors, page }`
/// (the caller closes the page).
export async function runCaseKernel(browser, c, dpr = 1, timeout = 30000) {
  const page = await browser.newPage({ deviceScaleFactor: dpr, viewport: { width: 1920, height: 1080 } });
  const errors = [];
  // (the page's uncaught errors; a 404 a program asks for — a missing file — is its own answer)
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
    const source = readFileSync(join(HERE, "fixtures", c.name + ".bas"), "utf8");
    await page.evaluate(({ source, assets, env }) => {
      window.__rapidr_assets = assets;
      // (QDXJOYSTICK's gamepad: the tests' script, read at each look)
      if (env.RAPIDR_TEST_JOYSTICK !== undefined) window.RAPIDR_TEST_JOYSTICK = env.RAPIDR_TEST_JOYSTICK;
      // (the tests' own HTTP server, ENVIRON$("RAPIDR_TEST_HTTP"))
      if (env.RAPIDR_TEST_HTTP) window.RAPIDR_TEST_ENV = { RAPIDR_TEST_HTTP: env.RAPIDR_TEST_HTTP };
      // (QMIDI: no MIDI output; QWAVE records a scripted tone, never the
      // microphone)
      window.RAPIDR_TEST_MIDI = ""; window.RAPIDR_TEST_WAVE_IN = "tone:440";
      const bc = window.rr.compile(source, "fixture", assets);
      window.rr.rapidr_set_test_env(env);
      window.rr.rapidr_run_bc(bc);
    }, { source, assets: assetMap(), env: hookEnv(c) });
    await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout, polling: 100 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { results, errors, page };
  } catch (e) {
    return { results: null, errors: [...errors, String(e.message).split("\n")[0]], page };
  }
}
