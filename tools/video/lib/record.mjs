// One take: RapidR Studio started headless in demo mode on a profile of its
// own (docs/video-pipeline.md), made so that none of Robert's settings,
// projects, keys or printer is used: its home a fresh folder (links to this
// checkout's ide/ and docs/, a copy of examples/), its registry, Documents,
// Studio data, AI chats and prints in there, AI keys in memory only, and
// only the environment it needs (an allowlist: no token or key of this
// shell reaches it). No window shows on the screen and the real mouse and
// keyboard are never used: demo mode draws the windows itself and plays
// its input through the kernel. Still watch each video whole before posting.
//
// A take that shows RAI uses the scripted mock AI of the tests
// (tests/ai_mock_server.mjs, started here on a free port of 127.0.0.1 and
// stopped after): no key, no network, the same answers every time. `{mock}`
// and `{mockroot}` in the script's text are its addresses.

import { spawn } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { MAIN, ROOT, rapidrBinary, testModeToken } from "./config.mjs";

/**
 * The names the mock's models go by on screen: made up and neutral, never a
 * real product's (the local ones match their sizes: tests/ai_mock_server.mjs
 * LOCAL_MODELS is small 3.2B, chat 1.5B, huge 405B, embedding 137M).
 */
const MOCK_MODELS = "demo-model:Demo model";
const MOCK_LOCAL_NAMES = "small-model-3b,mini-model-1.5b,huge-model-405b,embed-model-137m";

/** The mock AI server of the tests, on a free loopback port (null: this script doesn't use it). */
async function startMock(studio) {
  if (!studio.mock) return null;
  process.env.AI_MOCK_MODELS = MOCK_MODELS;
  process.env.AI_MOCK_LOCAL_NAMES = MOCK_LOCAL_NAMES;
  const mod = await import(pathToFileURL(join(ROOT, "tests", "ai_mock_server.mjs")).href);
  return mod.startMock(0);
}

/** `{mockroot}` and `{mock}` in a text, as the tests write them. */
const withMock = (text, mock) => (mock ? text.split("{mockroot}").join(mock.root).split("{mock}").join(mock.url) : text);

/**
 * The web runtime Run in Browser and Build Web App bundle (rapidrintr.js +
 * rapidrintr_bg.wasm): VIDEO_WEB_RUNTIME, else the release preparation of
 * this checkout's version in the main checkout's dist/ (git-ignored).
 */
function webRuntime() {
  if (process.env.VIDEO_WEB_RUNTIME) return process.env.VIDEO_WEB_RUNTIME;
  let version = "";
  try {
    version = (readFileSync(join(ROOT, "Cargo.toml"), "utf8").match(/^version\s*=\s*"([^"]+)"/m) || [])[1] || "";
  } catch {
    // (no workspace version: the newest one below)
  }
  const dist = join(MAIN, "dist");
  const have = (d) => existsSync(join(d, "rapidrintr_bg.wasm")) && existsSync(join(d, "rapidrintr.js"));
  const exact = join(dist, version, "prep", "web-runtime");
  if (version && have(exact)) return exact;
  const found = existsSync(dist) ? readdirSync(dist).map((v) => join(dist, v, "prep", "web-runtime")).filter(have).sort() : [];
  return found.length ? found[found.length - 1] : null;
}

/** Studio's language tag for a video's language (Studio's catalogs: ide/lang). */
function studioLang(lang) {
  const base = lang.split("-")[0];
  return base === "en" ? "en" : base;
}

/**
 * Records `script` (resolved: one language, lines measured) into `take`.
 * Resolves with `take` when demo mode wrote its frames; rejects with demo
 * mode's own words when it stopped.
 */
export async function record({ script, lang, take, log = () => {}, timeoutMin = 15 }) {
  const rapidr = rapidrBinary();
  if (!rapidr) throw new Error("No rapidr to run Studio with: build it (cargo build --release -p rapidr-cli) or set RAPIDR.");
  for (const d of ["frames", "inbox", "app"]) rmSync(join(take, d), { recursive: true, force: true });
  for (const f of ["timeline.json", "done", "error.txt", "last.png"]) rmSync(join(take, f), { force: true });
  // (outside any repository: Studio rightly won't start versions inside a
  // folder that has some — this checkout's; projects are ./projects, so the
  // video shows no path of this machine)
  // (a take that shows a build's output: its paths are under /tmp, a place
  // that says nothing about this computer, not the user's temporary folder)
  const profile = mkdtempSync(join(script.studio?.web ? "/tmp" : tmpdir(), "rapidr-video-"));
  const home = join(profile, "home");
  mkdirSync(home, { recursive: true });
  for (const d of ["docs", "studio", "chats", "prints"]) mkdirSync(join(profile, d), { recursive: true });
  // (Studio's home: what it reads beside it; projects go to ./projects here)
  symlinkSync(join(ROOT, "ide"), join(home, "ide"));
  symlinkSync(join(ROOT, "docs"), join(home, "docs"));
  // (Run in Browser and Build Web App: the web runtime where a checkout's
  // rapidr looks for it, and the Rust toolchain the third-party notices are
  // made with — read only, the user's own, named by RUSTUP_HOME / CARGO_HOME)
  let rust = {};
  if (script.studio?.web) {
    const runtime = webRuntime();
    if (!runtime) throw new Error("No web runtime for Run in Browser: set VIDEO_WEB_RUNTIME to a folder with rapidrintr.js and rapidrintr_bg.wasm.");
    mkdirSync(join(home, "target"), { recursive: true });
    symlinkSync(runtime, join(home, "target", "web"));
    // (a build runs in the project's folder, where target/web isn't: a checkout's
    // rapidr looks in its own root too — target/ is git-ignored)
    const rootWeb = join(ROOT, "target", "web");
    if (!existsSync(rootWeb)) {
      mkdirSync(join(ROOT, "target"), { recursive: true });
      symlinkSync(runtime, rootWeb);
    }
    rust = { RUSTUP_HOME: process.env.RUSTUP_HOME || join(homedir(), ".rustup"), CARGO_HOME: process.env.CARGO_HOME || join(homedir(), ".cargo") };
  }
  cpSync(join(ROOT, "examples"), join(home, "examples"), { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$|big(_\d+)?\.csv$/.test(src) });
  const studio = script.studio || {};
  // (a small project to start from: the tests' fixtures, copied into the
  // home — `fixtures`: { "tests/fixtures/studio_coach": "projects/Counter" })
  for (const [from, to] of Object.entries(studio.fixtures || {})) cpSync(join(ROOT, from), join(home, to), { recursive: true, filter: (src) => !/(^|\/)\.git(\/|$)/.test(src) });
  const mock = await startMock(studio);
  const file = join(take, "script.json");
  writeFileSync(file, withMock(JSON.stringify(script, null, 2), mock));
  const setup = [withMock(studio.setup || "", mock), "demo:poll"].filter(Boolean).join(",");
  const args = ["run", "ide/studio.rr", "--home", "./", "--fresh", "--maximized", "--theme", studio.theme || "rapidr-light", "--lang", studioLang(lang), "--do", setup];
  if (studio.open) args.push(studio.open);
  const token = testModeToken();
  // (an allowlist, never this shell's whole environment)
  const keep = Object.fromEntries(["PATH", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "SHELL"].filter((k) => process.env[k]).map((k) => [k, process.env[k]]));
  const env = {
    ...keep,
    HOME: home,
    ...rust,
    ...(token ? { RAPIDR_TEST_MODE: token } : {}),
    RAPIDR_DEMO: file,
    RAPIDR_DEMO_OUT: take,
    RAPIDR_DEMO_LANG: lang,
    RAPIDR_SCALE: String(script.scale || 1),
    RAPIDR_MENU: "window",
    RAPIDR_REGISTRY: join(profile, "registry.reg"),
    RAPIDR_DOCUMENTS: join(profile, "docs"),
    RAPIDR_STUDIO_DATA: join(profile, "studio"),
    RAPIDR_AI_CHATS: join(profile, "chats"),
    RAPIDR_AI_AUDIT: join(profile, "ai-audit.jsonl"),
    RAPIDR_AI_KEYSTORE: "memory",
    RAPIDR_PRINT_TO: join(profile, "prints"),
    RAPIDR_LOGIN_PATH: "off",
    // (Run in Browser serves the page but opens no browser on this Mac)
    RAPIDR_NO_BROWSER: "1",
    RAPIDR_BUILD_CACHE: join(profile, "build-cache"),
    // (the name a saved version is by: a learner's, never this machine's
    // account; no Git identity of this machine read either)
    RAPIDR_VERSIONS_NAME: studio.user || "Ana",
    USER: studio.user || "Ana",
    LOGNAME: studio.user || "Ana",
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
  };
  log(`   record ${rapidr} ${args.slice(0, 2).join(" ")} … (headless, ${script.screen?.join(" × ") || "1280 × 720"} at ${script.scale || 1}×)`);
  const t0 = Date.now();
  const code = await new Promise((resolve) => {
    const p = spawn(rapidr, args, { cwd: home, env, stdio: ["ignore", "pipe", "pipe"] });
    let err = "";
    p.stderr.on("data", (d) => (err += d));
    p.stdout.on("data", () => {});
    const timer = setTimeout(() => p.kill("SIGTERM"), timeoutMin * 60000);
    p.on("close", (c) => {
      clearTimeout(timer);
      writeFileSync(join(take, "studio-stderr.txt"), err);
      if (!process.env.VIDEO_KEEP_PROFILE) rmSync(profile, { recursive: true, force: true });
      if (mock) mock.close();
      resolve(c);
    });
  });
  if (existsSync(join(take, "error.txt"))) throw new Error(`demo mode stopped: ${readFileSync(join(take, "error.txt"), "utf8").trim()}`);
  if (!existsSync(join(take, "done"))) {
    const tail = existsSync(join(take, "studio-stderr.txt")) ? readFileSync(join(take, "studio-stderr.txt"), "utf8").trim().split("\n").slice(-4).join("\n") : "";
    throw new Error(`Studio ended (code ${code}) before the recording was done.${/test hooks are off|token was refused/.test(tail) ? " Demo mode needs a development build or the signed test-mode token (docs/security/test-mode.md)." : ""}\n${tail}`);
  }
  const frames = readdirSync(join(take, "frames")).filter((f) => f.endsWith(".png")).length;
  log(`   record done in ${((Date.now() - t0) / 1000).toFixed(0)} s (${frames} distinct frames)`);
  return take;
}
