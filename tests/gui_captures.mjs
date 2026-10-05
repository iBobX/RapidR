// The desktop's GUI captures, kept: every case of tests/gui_parity_cases.mjs
// built interpreted and run on the UI kernel's headless host at
// RAPIDR_SCALE 1 and 2 with its events, dump, resize / splitter drag and
// dialog answers; each run's windows (RAPIDR_CAPTURE's BMPs), accessibility
// trees (RAPIDR_TEST_A11Y) and dump lines kept as
//
//   <outdir>/<case>@<scale>x/window-<n>.bmp, a11y.json, dump.txt
//
// Two uses:
//   * a change that must not change the desktop (shared UI code): capture
//     before and after, then `diff -r before after` — every file must be
//     byte-identical (dialog_timers' are timing-dependent: its tick counts);
//   * the web's kernel host (Stage W3): RAPIDR_WEB_HOST=kernel
//     RAPIDR_DESKTOP_CAPTURES=<outdir> node tests/web_gui_parity.mjs compares
//     the browser's windows and trees with these.
//
// Usage (repo root, after building ./rapidr):
//   node tests/gui_captures.mjs <outdir> [case filter …]
// RUNNER=<rapidrintr-runner> uses that runner (a copy made before a change:
// the program's bytecode is appended as `rapidr build --interp` does) instead
// of the workspace's (built with `cargo build -p rapidr-runner-stub
// --profile runner`). Nothing reaches a real printer or the user's registry.
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync, readFileSync, chmodSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = resolve(process.argv[2] || join(ROOT, "tests/conformance/.work/gui_captures"));
const only = process.argv.slice(3);
const BIN = join(OUT, "_bin");
const WORK = join(OUT, "_work");
mkdirSync(BIN, { recursive: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
const EXE = process.platform === "win32" ? ".exe" : "";

let runner = process.env.RUNNER;
if (!runner) {
  execFileSync("cargo", ["build", "--quiet", "-p", "rapidr-runner-stub", "--profile", "runner"], { cwd: ROOT, stdio: "inherit" });
  runner = join(ROOT, "target/runner", `rapidrintr-runner${EXE}`);
}
const stub = readFileSync(runner);

const answers = (c) => ({
  ...(c.fileDialog === undefined ? {} : { RAPIDR_TEST_FILE_DIALOG: c.fileDialog }),
  ...(c.colorDialog === undefined ? {} : { RAPIDR_TEST_COLOR_DIALOG: c.colorDialog }),
  ...(c.fontDialog === undefined ? {} : { RAPIDR_TEST_FONT_DIALOG: c.fontDialog }),
  ...(c.messageDialog === undefined ? {} : { RAPIDR_TEST_MESSAGE_DIALOG: c.messageDialog }),
  ...(c.dialogHold === undefined ? {} : { RAPIDR_TEST_DIALOG_HOLD: String(c.dialogHold) }),
  ...(c.delay === undefined ? {} : { RAPIDR_CAPTURE_DELAY: String(c.delay) }),
  ...(c.joystick === undefined ? {} : { RAPIDR_TEST_JOYSTICK: c.joystick }),
});

let n = 0;
for (const c of cases.filter((c) => !only.length || only.some((f) => c.name.includes(f)))) {
  const bin = join(BIN, c.name + EXE);
  try {
    const rrbc = join(BIN, c.name + ".rrbc");
    execFileSync(join(ROOT, "rapidr" + EXE), ["build-bc", join(ROOT, `tests/fixtures/${c.name}.bas`), "-o", rrbc], { cwd: ROOT, stdio: "ignore" });
    const payload = readFileSync(rrbc);
    const len = Buffer.alloc(4);
    len.writeUInt32LE(payload.length);
    writeFileSync(bin, Buffer.concat([stub, payload, Buffer.from("RRBCEXE1"), len]));
    chmodSync(bin, 0o755);
  } catch (e) {
    console.log(`${c.name}: build failed (${e.message.split("\n")[0]})`);
    continue;
  }
  for (const scale of [1, 2]) {
    const dir = join(OUT, `${c.name}@${scale}x`);
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    const env = {
      ...process.env, RAPIDR_TEST_CLIPBOARD: "1", RAPIDR_PRINT_TO: join(WORK, "prints"), RAPIDR_REGISTRY: join(WORK, "registry.reg"), RAPIDR_SCALE: String(scale),
      RAPIDR_CAPTURE: join(dir, "window"), RAPIDR_TEST_A11Y: join(dir, "a11y.json"), RAPIDR_TEST_EVENTS: c.events, RAPIDR_TEST_DUMP: c.dump,
      RAPIDR_TEST_RESIZE: c.resize || "", RAPIDR_TEST_SPLIT: c.split || "", ...answers(c),
    };
    let stdout;
    try {
      stdout = execFileSync(bin, [], { encoding: "utf8", env, cwd: ROOT, timeout: 60000, stdio: ["ignore", "pipe", "ignore"] });
    } catch (e) {
      stdout = "(failed) " + (e.stdout || "");
    }
    writeFileSync(join(dir, "dump.txt"), stdout.split("\n").filter((l) => l.includes("=")).join("\n") + "\n");
    for (const theme of c.themes ?? []) {
      try {
        execFileSync(bin, [], { env: { ...env, RAPIDR_THEME: theme, RAPIDR_CAPTURE: join(dir, `theme-${theme}`), RAPIDR_TEST_A11Y: "", RAPIDR_TEST_EVENTS: "" }, cwd: ROOT, timeout: 60000, stdio: "ignore" });
      } catch {}
    }
    n++;
  }
  rmSync(bin, { force: true });
  console.log(`${c.name}: captured`);
}
rmSync(WORK, { recursive: true, force: true });
rmSync(BIN, { recursive: true, force: true });
console.log(`GUI captures: ${n} runs into ${OUT}`);
