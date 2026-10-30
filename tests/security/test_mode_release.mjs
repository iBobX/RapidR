// SEC-24 regressions (docs/security/audit-preview4.md, docs/security/test-mode.md):
// a release build's GUI test hooks are off unless a test-mode token signed by
// the key it trusts says otherwise; no environment variable alone turns them
// on.
//
// The source (always): no shipped crate reads a hook's variable around the
// gate (rapidr_testmode); no build turns `dev-hooks` on by default; the
// release scripts and the release app never pass it; test-mode.pub reads.
//
// A binary (RAPIDR_RELEASE, else ./rapidr): a release-profile `rapidr`
// given every hook variable — and then a token signed by a key the test
// makes (a fixture key, trusted by no build), a tampered one and a garbled
// one — runs a program as if none were set: no Studio MCP token, no crash at
// the line RAPIDR_TEST_CRASH names, ENVIRON$ empty for the hooks' names, one
// line on stderr saying the hooks are off. When ./rapidr is a development
// build (dev-hooks) these are skipped, said; RAPIDR_RELEASE must be a release
// build. RAPIDR_DEV (a development build, optional) is the control: the same
// run with the hooks on. --gui adds a windowed program (a window shows for
// half a second): no capture, no scripted click.
//
//   RAPIDR_RELEASE=target/release/rapidr node tests/security/test_mode_release.mjs [--gui]

import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
let failures = 0;
const check = (name, ok, more = "") => {
  if (ok) console.log("ok:", name);
  else {
    console.error("FAIL:", name, more);
    failures++;
  }
};

// ---- the source ----------------------------------------------------------------

const HOOK_READ = /std::env::var(?:_os)?\(\s*"RAPIDR_(?:TEST_|CAPTURE|SCHOOL_POLICY|AI_TEST_|LOGIN_SHELL|DUMP_MENUS)/;
function rustFiles(dir, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (!["target", "tests", "examples", "benches"].includes(e.name)) rustFiles(p, out);
    } else if (e.name.endsWith(".rs") && !/(^|_)tests?\.rs$/.test(e.name)) out.push(p);
  }
  return out;
}
const shipped = [];
for (const top of ["crates", "interpreter"]) {
  for (const c of readdirSync(join(ROOT, top))) {
    // (the prototype host isn't built nor shipped; the gate itself reads them)
    if (["rapidr-ui-proto", "rapidr-testmode", "patches"].includes(c) || !existsSync(join(ROOT, top, c, "src"))) continue;
    rustFiles(join(ROOT, top, c, "src"), shipped);
  }
}
const around = [];
for (const f of shipped) {
  let text = readFileSync(f, "utf8");
  const t = text.search(/#\[cfg\(test\)\]\s*\n\s*mod /);
  if (t >= 0) text = text.slice(0, t);
  text.split("\n").forEach((line, i) => {
    if (HOOK_READ.test(line) && !line.trim().startsWith("//")) around.push(`${relative(ROOT, f)}:${i + 1}`);
  });
}
check(`no shipped crate reads a test hook's variable around the gate (${shipped.length} files)`, around.length === 0, around.join(", "));
check("the GUI hooks' var() falls back to the gate", /rapidr_testmode::var\(name\)/.test(readFileSync(join(ROOT, "crates/rapidr-ui-app/src/testhooks.rs"), "utf8")));
check("ENVIRON$ answers the hooks' names through the gate", /rapidr_testmode::is_hook\(name\) && !rapidr_testmode::allowed\(name\)/.test(readFileSync(join(ROOT, "crates/rapidr-value/src/environ.rs"), "utf8")));
check("Studio passes a program its hooks only while its own act", /if let Some\(prefix\) = rapidr_testmode::var\("RAPIDR_CAPTURE"\)/.test(readFileSync(join(ROOT, "crates/rapidr-runtime-core/src/studio.rs"), "utf8")));

const tomls = [];
for (const top of ["crates", "interpreter"]) for (const c of readdirSync(join(ROOT, top))) if (existsSync(join(ROOT, top, c, "Cargo.toml"))) tomls.push(join(top, c, "Cargo.toml"));
tomls.push("Cargo.toml");
const byDefault = tomls.filter((t) => {
  const text = readFileSync(join(ROOT, t), "utf8");
  return /^default\s*=\s*\[[^\]]*dev-hooks/m.test(text) || /features\s*=\s*\[[^\]]*"[^"]*dev-hooks"/.test(text);
});
check("no crate turns dev-hooks on by default or for a dependency", byDefault.length === 0, byDefault.join(", "));
const scripts = [...readdirSync(join(ROOT, "tools/release")).filter((f) => /\.(sh|py|ps1)$/.test(f)).map((f) => `tools/release/${f}`), "tools/studio_app.sh"];
const passing = scripts.filter((s) => readFileSync(join(ROOT, s), "utf8").includes("dev-hooks"));
check("the release scripts and the release app never pass dev-hooks", passing.length === 0, passing.join(", "));
const pub = readFileSync(join(ROOT, "crates/rapidr-testmode/test-mode.pub"), "utf8");
const pubLine = pub.split("\n").find((l) => /^public:/.test(l.trim()));
check("test-mode.pub has a key line (none, or 64 hex digits)", !!pubLine && /^public:\s*(none|[0-9a-fA-F]{64})\s*$/.test(pubLine.trim()));
check("no secret key in the repository", !/^secret:/m.test(pub) && !existsSync(join(ROOT, "crates/rapidr-testmode/test-mode.key")));

// ---- the binary ------------------------------------------------------------------

const BIN = process.env.RAPIDR_RELEASE || join(ROOT, "rapidr");
const work = mkdtempSync(join(tmpdir(), "rapidr-sec24-"));
const base = { ...process.env };
for (const k of Object.keys(base)) if (/^RAPIDR_(TEST_|CAPTURE)/.test(k)) delete base[k];
Object.assign(base, {
  RAPIDR_REGISTRY: join(work, "registry.reg"),
  RAPIDR_AI_KEYSTORE: "memory",
  RAPIDR_PRINT_TO: join(work, "prints"),
  RAPIDR_CRASH_DIR: join(work, "crashes"),
});
mkdirSync(join(work, "prints"));
mkdirSync(join(work, "crashes"));
const run = (bin, args, env = {}, timeout = 60000) => spawnSync(bin, args, { cwd: work, env: { ...base, ...env }, encoding: "utf8", timeout });

// Every hook's variable, as a launcher might set them.
const HOOKS = {
  RAPIDR_CAPTURE: join(work, "cap"),
  RAPIDR_CAPTURE_DELAY: "0.1",
  RAPIDR_TEST_EVENTS: "b.onclick",
  RAPIDR_TEST_DUMP: "f.caption",
  RAPIDR_TEST_SYSTEM_LOOK: `file:${join(work, "look.txt")}`,
  RAPIDR_TEST_FILE_DIALOG: join(work, "picked.txt"),
  RAPIDR_TEST_MESSAGE_DIALOG: "Yes;",
  RAPIDR_TEST_CRASH: "panic@4",
  RAPIDR_TEST_MCP_TOKEN: "sec24-known-token",
  RAPIDR_TEST_EXT: work,
  RAPIDR_TEST_CLIPBOARD: "1",
  RAPIDR_SCHOOL_POLICY: join(work, "policy.toml"),
  RAPIDR_AI_TEST_SERVER: "http://127.0.0.1:9/",
};
writeFileSync(join(work, "look.txt"), "dark\n");
const PROGRAM = join(work, "probe.rr");
writeFileSync(
  PROGRAM,
  [
    `PRINT "mcp=[" + ENVIRON$("RAPIDR_TEST_MCP_TOKEN") + "]"`,
    `PRINT "ext=[" + ENVIRON$("RAPIDR_TEST_EXT") + "]"`,
    `PRINT "capture=[" + ENVIRON$("rapidr_capture") + "]"`,
    // (a call into the runtime: where RAPIDR_TEST_CRASH's line is checked)
    `PRINT "line four" + LEFT$(ENVIRON$("HOME"), 0)`,
    `PRINT "done"`,
    "",
  ].join("\n"),
);
// (--gui: a window with a button, and a timer that closes it after half a
// second, as a launcher's captures and clicks would drive it)
const GUI = process.argv.includes("--gui");
const gui = join(work, "window.rr");
writeFileSync(
  gui,
  ["DIM F AS RForm", "DIM B AS RButton", "DIM T AS RTimer", "SUB Clicked", '  PRINT "clicked"', "END SUB", "SUB Tick", "  F.Close", "END SUB", "B.Parent = F", 'B.Caption = "Go"', "B.OnClick = Clicked", "T.Interval = 500", "T.OnTimer = Tick", 'F.Caption = "SEC-24"', "F.ShowModal", 'PRINT "end"', ""].join("\n"),
);
const offLines =(stderr) => (stderr || "").split("\n").filter((l) => /^RapidR: (test hooks are off|the test-mode token was refused)/.test(l));
function expectOff(label, r, why) {
  check(`${label}: the program ran to its end (no crash at RAPIDR_TEST_CRASH's line)`, r.status === 0 && /line four\ndone/.test(r.stdout), `status ${r.status}: ${r.stdout} ${r.stderr}`);
  check(`${label}: ENVIRON$ is empty for the hooks' names (Studio's MCP token, its test folder, the capture)`, /mcp=\[\]\next=\[\]\ncapture=\[\]/.test(r.stdout), r.stdout);
  check(`${label}: no crash report`, readdirSync(join(work, "crashes")).length === 0);
  const lines = offLines(r.stderr);
  check(`${label}: one line on stderr says the hooks are off${why ? ` (${why})` : ""}`, lines.length === 1 && (!why || lines[0].includes(why)), JSON.stringify(r.stderr));
}

const missing = !existsSync(BIN);
const status = missing ? null : run(BIN, ["dev", "test-mode", "status"]);
const dev = status && /^build: development/m.test(status.stdout);
if (missing || dev) {
  const why = missing ? `${BIN} isn't built` : `${BIN} is a development build (dev-hooks or debug)`;
  if (process.env.RAPIDR_RELEASE) check("RAPIDR_RELEASE is a release build", false, why);
  else console.log(`SKIP: the release binary's checks: ${why} (give RAPIDR_RELEASE=<a release build>)`);
} else {
  check("the release build says so", /^build: release/m.test(status.stdout) && /^hooks: off/m.test(status.stdout), status.stdout);

  // No token: every hook variable ignored.
  expectOff("no token", run(BIN, ["run", PROGRAM], HOOKS), "no RAPIDR_TEST_MODE token");

  // A token signed by a fixture key (made here, trusted by no build).
  const keys = join(work, "keys");
  const made = run(BIN, ["dev", "test-mode", "new-key", "--out", keys]);
  check("new-key makes a key pair outside the repository", made.status === 0 && existsSync(join(keys, "test-mode.key")) && existsSync(join(keys, "test-mode.pub")), made.stderr);
  if (process.platform !== "win32" && existsSync(join(keys, "test-mode.key"))) check("the secret key file is the owner's only (0600)", (statSync(join(keys, "test-mode.key")).mode & 0o077) === 0);
  const token = join(work, "fixture.token");
  const issued = run(BIN, ["dev", "test-mode", "issue", "--key", join(keys, "test-mode.key"), "--days", "30", "--out", token]);
  check("issue writes a token", issued.status === 0 && existsSync(token), issued.stderr);
  check("issue refuses more than a year", run(BIN, ["dev", "test-mode", "issue", "--key", join(keys, "test-mode.key"), "--days", "400", "--out", token]).status !== 0);
  const inRepo = run(BIN, ["dev", "test-mode", "new-key", "--out", join(ROOT, "target", "sec24-key-must-not-exist")]);
  check("new-key refuses a folder inside the repository", inRepo.status !== 0 && !existsSync(join(ROOT, "target", "sec24-key-must-not-exist", "test-mode.key")), inRepo.stdout);
  if (existsSync(token)) {
    const trusted = /^trusted key: none/m.test(status.stdout) ? "this build trusts no test-mode key" : "signed with another key";
    expectOff("a fixture-signed token", run(BIN, ["run", PROGRAM], { ...HOOKS, RAPIDR_TEST_MODE: token }), trusted);
    const text = readFileSync(token, "utf8");
    const tampered = join(work, "tampered.token");
    writeFileSync(tampered, text.replace(/expires: .*/, "expires: 2099-12-31"));
    expectOff("a tampered token", run(BIN, ["run", PROGRAM], { ...HOOKS, RAPIDR_TEST_MODE: tampered }), "the test-mode token was refused");
  }
  const garbled = join(work, "garbled.token");
  writeFileSync(garbled, "hooks: on, please\n");
  // (a build that trusts no key yet — the placeholder — refuses every token
  // before reading it: the tokens' own checks are rapidr-testmode's tests)
  const noKey = /^trusted key: none/m.test(status.stdout);
  expectOff("a garbled token", run(BIN, ["run", PROGRAM], { ...HOOKS, RAPIDR_TEST_MODE: garbled }), noKey ? "this build trusts no test-mode key" : "not a test-mode token");
  expectOff("a token that isn't there", run(BIN, ["run", PROGRAM], { ...HOOKS, RAPIDR_TEST_MODE: join(work, "nothing.token") }), noKey ? "this build trusts no test-mode key" : "can't be read");

  if (GUI) {
    const r = run(BIN, ["run", gui], HOOKS, 30000);
    check("a window: the program ends by itself (its timer)", r.status === 0 && /end/.test(r.stdout), `${r.status} ${r.stdout} ${r.stderr}`);
    check("a window: no scripted click (RAPIDR_TEST_EVENTS)", !/clicked/.test(r.stdout), r.stdout);
    check("a window: no capture written (RAPIDR_CAPTURE)", !readdirSync(work).some((f) => f.startsWith("cap")), readdirSync(work).join(" "));
    check("a window: no dump (RAPIDR_TEST_DUMP)", !/f\.caption/i.test(r.stdout), r.stdout);
  }
}

// The control: a development build, the same run with the hooks on.
if (process.env.RAPIDR_DEV && existsSync(process.env.RAPIDR_DEV)) {
  const r = run(process.env.RAPIDR_DEV, ["run", PROGRAM], { ...HOOKS, RAPIDR_TEST_CRASH: "" });
  check("the control: a development build reads the hooks (ENVIRON$ answers)", /mcp=\[sec24-known-token\]/.test(r.stdout) && offLines(r.stderr).length === 0, r.stdout + r.stderr);
  const crashed = run(process.env.RAPIDR_DEV, ["run", PROGRAM], HOOKS);
  check("the control: a development build crashes where RAPIDR_TEST_CRASH says", crashed.status !== 0 && !/line four/.test(crashed.stdout), `${crashed.status} ${crashed.stdout}`);
  if (GUI) {
    const w = run(process.env.RAPIDR_DEV, ["run", gui], { ...HOOKS, RAPIDR_TEST_CRASH: "" }, 30000);
    check("the control: a development build clicks and captures the window (headless)", /clicked/.test(w.stdout) && readdirSync(work).some((f) => f.startsWith("cap")), `${w.status} ${w.stdout} ${w.stderr} ${readdirSync(work).join(" ")}`);
  }
}

rmSync(work, { recursive: true, force: true });
console.log(failures ? `${failures} failed` : "all passed");
process.exit(failures ? 1 : 0);
