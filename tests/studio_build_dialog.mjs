// RapidR Studio's Build dialog (design/build-dialog, the approved design;
// ide/build.inc), on the desktop's headless host: every step in light, dark
// and high contrast, in English and Spanish, and narrow (one page), each
// captured (tests/results/build-dialog/<scene>.png) to be put next to the
// design's frames; its state read back (RAPIDR_TEST_DUMP) and checked; then
// a real build of the project's own system and the web from the dialog's
// choices (Build Again's path), the zips read back.
//
//   node tests/studio_build_dialog.mjs [filter…]      (RAPIDR: the rapidr to use)
//
// Studio is driven through its own commands and the dialog's test steps
// (build:open=N, build:step=N, build:tick=…, build:phase=…: ide/build.inc's
// BuildStep). Nothing leaves this computer: the build's runners are the
// checkout's own (this computer's and the web's).

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./test_mode.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const OUT = join(ROOT, "tests", "results", "build-dialog");
const WORK = join(OUT, "work");
const FIXTURE = join(ROOT, "tests", "fixtures", "studio_build");
const filters = process.argv.slice(2);
const LOOKS = { light: "rapidr-light", dark: "rapidr-dark", hc: "rapidr-high-contrast" };

let failed = 0;
let passed = 0;
const ok = (cond, what) => {
  if (cond) passed++;
  else failed++;
  console.log(`${cond ? "ok  " : "FAIL"} ${what}`);
};

// Each scene: the steps before the dialog opens (it is modal: the steps wait
// while it shows), the step it opens on, what the dump must say.
const PICK = "wait,wait,build:tick=windows-x64,build:tick=windows-x86,build:tick=macos-universal,build:tick=web,build:untick=linux-x64";
const BUILDING = "build:phase=building,build:state=windows-x64~built~0,build:state=windows-x86~download~19.0 MB,build:state=macos-universal~working,build:state=web~working";
const DONE = "build:state=windows-x64~built~0,build:state=windows-x86~built~19677149,build:state=macos-universal~built~16693150,build:state=web~built~23304209,build:phase=done";
const SCENES = [
  { name: "01-where-it-runs", steps: `${PICK},build:open=1`, dump: { "bdnext.caption": /Next|Siguiente/, "proj.buildfor": /windows-x64.*web/ } },
  { name: "02-your-app", steps: `${PICK},build:open=2`, dump: { "bdname.text": /^Tip Calculator$/, "bdid.text": /^com\.northfield\.tipcalculator$/ } },
  { name: "02b-your-app-needs-care", steps: `${PICK},build:icon=small.png,build:field=id~org.northfield.tip calculator,build:open=2`, dump: { "bdidnote.caption": /space|espacio/ } },
  { name: "03-what-goes-in", steps: `${PICK},build:open=3`, dump: { "bddatahead.caption": /3/ } },
  { name: "04-ready-to-build", steps: `${PICK},build:open=4`, dump: { "bdnext.caption": /4/ } },
  { name: "05-building", steps: `${PICK},${BUILDING},build:open=5`, dump: { "bdnext.caption": /Stop|Detener/ } },
  { name: "06-done", steps: `${PICK},${DONE},build:open=5`, dump: { "bdnext.caption": /Show in|Mostrar en/ } },
];

function capture(scene, look, lang, narrow, opts = {}) {
  // (BD_SCALE=2: the manual's pictures, at twice the size)
  const scale = Number(process.env.BD_SCALE || 1);
  const tag = `${scene.name}-${lang}-${look}${narrow ? "-narrow" : ""}${scale > 1 ? `@${scale}x` : ""}`;
  const dir = join(WORK, tag);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  cpSync(FIXTURE, join(dir, "Projects", "TipCalculator"), { recursive: true });
  const keys = [...Object.keys(scene.dump), "studio.caption", "builddialog.clientwidth", "builddialog.clientheight"];
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", LOOKS[look], "--lang", lang, "--do", scene.steps, join(dir, "Projects", "TipCalculator", "Tip Calculator.rrproj")];
  const r = spawnSync(RAPIDR, args, {
    cwd: ROOT,
    encoding: "utf8",
    timeout: Math.max(120000, (opts.delay || 7) * 1000 + 90000),
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(opts.delay || 7),
      ...(opts.events ? { RAPIDR_TEST_EVENTS: opts.events } : {}),
      ...(opts.a11y ? { RAPIDR_TEST_A11Y: join(dir, "a11y.json") } : {}),
      // (the runner a checkout builds for this computer: the lane's own cargo target)
      ...(process.env.CARGO_TARGET_DIR ? { CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR } : {}),
      RAPIDR_MENU: "window",
      ...(scale > 1 ? { RAPIDR_SCALE: String(scale) } : {}),
      RAPIDR_TEST_DUMP: keys.join(","),
      ...(narrow ? { RAPIDR_TEST_SCREEN: "470x880" } : { RAPIDR_TEST_SCREEN: "1440x900" }),
      RAPIDR_PRINT_TO: join(dir, "prints"),
      RAPIDR_REGISTRY: join(dir, "registry.reg"),
      RAPIDR_CONFIG_DIR: join(dir, "config"),
      RAPIDR_DOCUMENTS: join(dir, "docs"),
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_STUDIO_DATA: join(dir, "studio"),
      RAPIDR_LOGIN_PATH: "off",
      // (the project in ~/Projects/TipCalculator, as a person keeps one: the
      // pictures show that path, not this test's folder)
      HOME: dir,
      CARGO_HOME: process.env.CARGO_HOME || join(homedir(), ".cargo"),
      RUSTUP_HOME: process.env.RUSTUP_HOME || join(homedir(), ".rustup"),
    },
  });
  const dump = {};
  for (const line of (r.stdout || "").split("\n")) {
    const m = line.match(/^([\w.()]+)=(.*)$/);
    if (m) dump[m[1].toLowerCase()] = m[2];
  }
  // (the dialog is the second window captured; its PNG kept under the scene's name)
  const shots = readdirSync(dir).filter((f) => /^window-\d+\.bmp$/.test(f)).sort();
  const dialog = shots.length > 1 ? shots[shots.length - 1] : null;
  if (dialog) {
    spawnSync("sips", ["-s", "format", "png", join(dir, dialog), "--out", join(OUT, `${tag}.png`)], { encoding: "utf8" });
  }
  if (!opts.noDialog) ok(!!dialog, `${tag}: the dialog was captured${dialog ? "" : ` (status ${r.status}; ${(r.stderr || "").slice(-400)})`}`);
  for (const [k, re] of Object.entries(scene.dump)) {
    ok(re.test(dump[k] ?? ""), `${tag}: ${k} = ${JSON.stringify(dump[k])}`);
  }
  return { dump, dir };
}

mkdirSync(OUT, { recursive: true });
const want = (name) => filters.length === 0 || filters.some((f) => name.includes(f));
for (const scene of SCENES) {
  for (const lang of ["en", "es"]) {
    for (const look of Object.keys(LOOKS)) {
      if (!want(`${scene.name}-${lang}-${look}`)) continue;
      capture(scene, look, lang, false);
    }
  }
}
// (narrow: one page, the footer pinned)
for (const lang of ["en", "es"]) {
  if (!want(`narrow-${lang}`) && !want("narrow")) continue;
  capture({ name: "09-narrow", steps: `${PICK},build:open=1`, dump: { "bdnext.caption": /4/ } }, "light", lang, true);
}

// The keyboard and a screen reader (the design's frame 11): step 1's
// controls named as the design says, the first system's box focused as the
// dialog opens, Tab going on in reading order.
if (want("a11y")) {
  const nodesOf = (dir) => {
    const nodes = [];
    const walk = (n) => { if (!n) return; nodes.push(n); (n.children || []).forEach(walk); };
    try {
      JSON.parse(readFileSync(join(dir, "a11y.json"), "utf8")).forEach(walk);
    } catch {
      /* no tree */
    }
    return nodes;
  };
  // (the dialog's focus: Studio's window behind it has its own)
  const focusedName = (nodes) => nodes.filter((n) => (n.states || []).includes("focused") && n.name !== "Tip Calculator").map((n) => n.name).join(" | ");
  const tabs = (n) => Array(n).fill("builddialog.__key_9").join(",");
  for (const [n, want] of [[0, /^Windows$/], [1, /^64-bit \(x64\)$/], [4, /^macOS$/], [5, /^Linux$/]]) {
    const { dir } = capture({ name: `a11y-tab${n}`, steps: `${PICK},build:open=1`, dump: {} }, "light", "en", false, { a11y: true, events: n ? tabs(n) : "bdfootnote.onclick", delay: 9 });
    const nodes = nodesOf(dir);
    ok(want.test(focusedName(nodes)), `a11y: after ${n} Tab${n === 1 ? "" : "s"} the focus is on ${want} (${focusedName(nodes) || "nothing"})`);
    if (n === 0) {
      for (const name of ["Steps", "All Desktop", "Everything", "Windows", "64-bit (x64)", "ARM64", "32-bit (x86)", "macOS", "Linux", "Web", "More", "Next"]) {
        const hit = nodes.find((x) => (x.name || "").replace(/&/g, "") === name);
        ok(!!hit, `a11y: "${name}" is named for a screen reader${hit ? ` (${hit.role}${hit.description ? `; ${hit.description}` : ""})` : ""}`);
      }
      const win = nodes.find((x) => x.name === "Windows");
      ok(!!win && /64-bit \(x64\), 32-bit \(x86\) chosen/.test(win.description || ""), `a11y: Windows says which processors are chosen (${win && win.description})`);
    }
  }
}

// A real build, with the checkout's runners (this computer's system and the
// web): Build Again's path (no dialog), then the dialog's own Build button
// clicked, its Done page captured; the zips and SHA256SUMS read back.
const HERE_SYSTEM = process.platform === "darwin" ? "macos-universal" : process.platform === "win32" ? "windows-x64" : "linux-x64";
const REAL = `wait,wait,build:untick=windows-x64,build:untick=windows-x86,build:untick=linux-x64,build:untick=macos-universal,build:tick=${HERE_SYSTEM},build:tick=web`;
const zipsOf = (dir) => {
  const out = join(dir, "Projects", "TipCalculator", "build");
  return existsSync(out) ? readdirSync(out).sort() : [];
};
const checkZips = (tag, dir) => {
  const files = zipsOf(dir);
  const wanted = [`Tip-Calculator-1.0.0-${HERE_SYSTEM}.zip`, "Tip-Calculator-1.0.0-web.zip", "SHA256SUMS"];
  for (const w of wanted) ok(files.includes(w), `${tag}: ${w} made (${files.join(", ")})`);
  const sums = existsSync(join(dir, "Projects", "TipCalculator", "build", "SHA256SUMS")) ? readFileSync(join(dir, "Projects", "TipCalculator", "build", "SHA256SUMS"), "utf8") : "";
  ok(sums.includes("Tip-Calculator-1.0.0-web.zip") && sums.includes(`Tip-Calculator-1.0.0-${HERE_SYSTEM}.zip`), `${tag}: SHA256SUMS lists both`);
  for (const f of files.filter((f) => f.endsWith(".zip"))) {
    const list = spawnSync("unzip", ["-l", join(dir, "Projects", "TipCalculator", "build", f)], { encoding: "utf8" }).stdout || "";
    ok(list.includes("README.txt") && (list.includes("tips.csv") || list.includes("assets")), `${tag}: ${f} holds README.txt and the data`);
  }
};
if (want("real-again")) {
  const { dir } = capture({ name: "real-again", steps: `${REAL},build:start`, dump: { "proj.buildfor": new RegExp(HERE_SYSTEM) } }, "light", "en", false, { delay: 120, noDialog: true });
  checkZips("real-again", dir);
}
if (want("real-dialog")) {
  const idle = Array(8).fill("bdfootnote.onclick").join(",");
  // (the web alone: its runner is the checkout's web runtime, no cargo on the way)
  const WEB_ONLY = `${REAL},build:untick=${HERE_SYSTEM}`;
  const { dir, dump } = capture({ name: "real-dialog", steps: `${WEB_ONLY},build:open=4`, dump: { "bdnext.caption": /Show in/, "bdh5.caption": /1 app built in/, "proj.builtpath": /Tip-Calculator-1\.0\.0-web\.zip$/ } }, "light", "en", false, { delay: 70, events: `${idle},bdnext.onclick,bdfootnote.__wait_45000` });
  const files = zipsOf(dir);
  ok(files.includes("Tip-Calculator-1.0.0-web.zip") && files.includes("SHA256SUMS"), `real-dialog: the web zip and SHA256SUMS made from the dialog (${files.join(", ")})`);
}

console.log(`\n${passed} passed, ${failed} failed; captures in ${OUT}`);
process.exit(failed ? 1 : 0);
