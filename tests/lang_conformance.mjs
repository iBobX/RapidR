// The language registry against the runtimes (docs/ide-plan.md, stage I0):
// one program per component, generated from its registry entry
// (`rapidr lang conformance`, crates/rapidr-lang/src/conformance.rs), that
// reads every property, calls every method and binds every event the
// registry says the component has. Each runtime must print the registry's
// output (defaults included, except the known gaps in tests/lang/gaps.txt)
// and answer every member: no "not implemented" / "no such method".
//
//   node tests/lang_conformance.mjs                    vm, native and web
//   node tests/lang_conformance.mjs --backend vm       some runtimes (vm, native, web; vm,native)
//   node tests/lang_conformance.mjs rbutton rform      only these components
//   node tests/lang_conformance.mjs --gaps             print what the VM reads where the
//                                                      registry's default differs (gaps.txt lines;
//                                                      with --backend web, the web's `web:` lines)
//
// Native: every component in one program (one cargo build). Web: the web
// IDE's page on RAPIDR_URL (default http://localhost:8765), after
// tools/build_web_artifacts.sh. Env: RAPIDR_BIN (default ./rapidr),
// LANG_CONFORMANCE_WORK (default tests/conformance/.work/lang).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dropBuild } from "./cargo_builds.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXE = process.platform === "win32" ? ".exe" : "";
const RAPIDR = resolve(ROOT, process.env.RAPIDR_BIN || `rapidr${EXE}`);
const WORK = resolve(process.env.LANG_CONFORMANCE_WORK || join(ROOT, "tests/conformance/.work/lang"));
const GAPS = join(ROOT, "tests/lang/gaps.txt");

const args = process.argv.slice(2);
let backends = ["vm", "native", "web"];
const filters = [];
let gapsMode = false;
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--backend") backends = args[++i].split(",");
  else if (args[i] === "--gaps") gapsMode = true;
  else filters.push(args[i].toLowerCase());
}
if (!existsSync(RAPIDR)) {
  console.error(`rapidr binary not found at ${RAPIDR} (cargo build --release -p rapidr-cli, then copy it)`);
  process.exit(2);
}
for (const d of ["desktop", "web", "native"]) rmSync(join(WORK, d), { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
const PRINTS = join(WORK, "prints");
mkdirSync(PRINTS, { recursive: true });
const ENV = {
  ...process.env,
  RAPIDR_PRINT_TO: PRINTS, RAPIDR_REGISTRY: join(WORK, "registry.reg"), RAPIDR_TEST_CLIPBOARD: "1",
  RAPIDR_TEST_JOYSTICK: "", RAPIDR_TEST_COMPORT: "", RAPIDR_TEST_SOUND: "", RAPIDR_TEST_MIDI: "", RAPIDR_TEST_WAVE_IN: "",
  RAPIDR_TEST_FILE_DIALOG: "", RAPIDR_TEST_COLOR_DIALOG: "", RAPIDR_TEST_FONT_DIALOG: "", RAPIDR_TEST_MESSAGE_DIALOG: "",
};
const norm = (s) => s.replace(/\r\n/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").trimEnd();
// What a runtime says when it doesn't answer a member.
const UNANSWERED = /not implemented|no such method|NotImplemented/i;

function run(cmd, cmdArgs, opts = {}) {
  const r = spawnSync(cmd, cmdArgs, { encoding: "utf8", timeout: opts.timeout || 60_000, maxBuffer: 1 << 22, env: ENV, cwd: WORK, stdin: "ignore", ...opts });
  return { ok: r.status === 0 && !r.error, out: r.stdout || "", err: (r.stderr || "") + (r.error ? `\n${r.error}` : "") };
}

function generate(target) {
  const dir = join(WORK, target);
  const g = run(RAPIDR, ["lang", "conformance", dir, "--target", target, ...(gapsMode ? [] : ["--gaps", GAPS])]);
  if (!g.ok) {
    console.error(g.out + g.err);
    process.exit(2);
  }
  return readdirSync(dir).filter((f) => f.endsWith(".bas")).map((f) => f.slice(0, -4)).sort()
    .filter((n) => !filters.length || filters.includes(n))
    .map((n) => ({ name: n, dir, source: readFileSync(join(dir, n + ".bas"), "utf8"), expected: readFileSync(join(dir, n + ".expected"), "utf8") }));
}

/// The first line where `got` differs, as a short report. A method may
/// print (RDATAFRAME.Head): lines before its `… called` line are its own.
function difference(expected, got) {
  const a = norm(expected).split("\n"), b = norm(got).split("\n");
  let j = 0;
  for (let i = 0; i < a.length; i++, j++) {
    if (a[i] !== b[j] && / called$/.test(a[i])) {
      const k = b.indexOf(a[i], j);
      if (k >= 0) j = k;
    }
    if (a[i] !== b[j]) return `line ${i + 1}: expected ${JSON.stringify(a[i])}, got ${JSON.stringify(b[j] ?? "<end>")}`;
  }
  return j < b.length ? `extra output: ${JSON.stringify(b[j])}` : null;
}

const tally = { pass: 0, fail: 0 };
function report(backend, name, problems) {
  if (problems.length) {
    tally.fail++;
    console.log(`FAIL  ${backend.padEnd(6)}  ${name}`);
    for (const p of problems) console.log(`        | ${p}`);
  } else {
    tally.pass++;
    console.log(`PASS  ${backend.padEnd(6)}  ${name}`);
  }
}

function unanswered(text) {
  return text.split("\n").filter((l) => UNANSWERED.test(l)).slice(0, 60).map((l) => `unanswered: ${l.trim()}`);
}

// --- the interpreter ----------------------------------------------------------------
function vm(p) {
  const src = join(p.dir, p.name + ".bas");
  const bc = join(p.dir, p.name + ".rrbc");
  const c = run(RAPIDR, ["build-bc", src, "-o", bc]);
  if (!c.ok) return { problems: [`compile: ${(c.out + c.err).trim().split("\n").slice(-3).join(" / ")}`], out: "" };
  const r = run(RAPIDR, ["run-bc", bc]);
  const problems = [...unanswered(r.err)];
  if (!r.ok) problems.push(`crashed: ${r.err.trim().split("\n").slice(-2).join(" / ")}`);
  const d = difference(p.expected, r.out);
  if (d) problems.push(d);
  return { problems, out: r.out };
}

// --- native: every program in one build ----------------------------------------------
function native(programs) {
  const dir = join(WORK, "native");
  mkdirSync(dir, { recursive: true });
  const src = join(dir, "lang_all.rr");
  writeFileSync(src, programs.map((p) => p.source).join("\n"));
  // (the conformance suite's build cache: the runtime crates built once)
  const target = resolve(process.env.CONFORMANCE_WORK || join(ROOT, "tests/conformance/.work"), "cargo-target");
  const b = run(RAPIDR, ["build", src, join(dir, "lang_all_rust"), "--no-bundle"], { timeout: 1_800_000, env: { ...ENV, CARGO_TARGET_DIR: target } });
  const bin = join(dir, "lang_all" + EXE);
  if (!b.ok || !existsSync(bin)) {
    dropBuild(target, "lang_all");
    report("native", "(all)", [`build failed: ${(b.out + b.err).split("\n").filter((l) => /^error/.test(l)).slice(0, 5).join(" / ")}`]);
    return;
  }
  const r = run(bin, [], { timeout: 300_000, env: { ...ENV, RUST_BACKTRACE: "1" } });
  if (!r.ok) console.log(r.err.split("\n").filter((l) => /panicked|error/i.test(l)).slice(0, 3).join("\n"));
  rmSync(bin, { force: true });
  dropBuild(target, "lang_all");
  // (split the output at each program's `== NAME` line)
  const parts = {};
  let cur = null;
  for (const line of r.out.replace(/\r\n/g, "\n").split("\n")) {
    const m = /^== (\S+)$/.exec(line);
    if (m) cur = m[1].toLowerCase();
    if (cur) (parts[cur] ||= []).push(line);
  }
  const errs = unanswered(r.err);
  for (const p of programs) {
    const got = (parts[p.name] || []).join("\n");
    const problems = [...errs.filter((e) => e.toLowerCase().includes(`${p.name}_o`))];
    if (!r.ok && !got) problems.push(`never ran (crashed earlier: ${r.err.trim().split("\n").slice(-1)[0] || "?"})`);
    const d = difference(p.expected, got);
    if (d) problems.push(d);
    report("native", p.name, problems);
  }
}

// --- the web ---------------------------------------------------------------------------
async function web(programs, onOutput = null) {
  const { chromium } = await import("playwright");
  const { openRunner } = await import("./web_run.mjs");
  const browser = await chromium.launch();
  const r = await openRunner(browser);
  for (const p of programs) {
    const tag = `@@lang_${p.name}@@`;
    // (the closing marker on a line of its own; a program that ENDs itself
    // never prints it)
    await r.run(`${p.source}\nPRINT\nPRINT "${tag}E"\n`, { name: p.name });
    const ended = () => r.lines.includes(`${tag}E`) || r.ended() || r.errs.some((e) => e.startsWith("compile:"));
    for (let waited = 0; waited < 20000 && !ended(); waited += 100) await r.page.waitForTimeout(100);
    const m = new RegExp(`^([\\s\\S]*?)\\n?^(?:${tag}E|\\[RapidR\\] Program ended\\.)$`, "m").exec(r.output());
    const got = m ? m[1] : `<no output: ${JSON.stringify((r.output() + " " + r.errors()).slice(-200))}>`;
    if (onOutput) {
      onOutput(p, got);
      continue;
    }
    const problems = [...unanswered([...r.lines, ...r.errs].join("\n"))];
    const d = difference(p.expected, got);
    if (d) problems.push(d);
    report("web", p.name, problems);
  }
  await browser.close();
}

// --- gaps: what a runtime reads where the expected value differs --------------------------
// (the VM against the registry's defaults: gaps.txt's lines for every
// runtime; `--backend web`: the web against those, its `web:` lines)
function gapLines(p, out, prefix) {
  const want = norm(p.expected).split("\n"), got = norm(out).split("\n");
  for (let i = 0; i < want.length; i++) {
    const w = /^([A-Za-z0-9_]+)=(.*)$/.exec(want[i]);
    const g = got[i] !== undefined ? /^([A-Za-z0-9_]+)=(.*)$/.exec(got[i]) : null;
    if (w && g && w[1] === g[1] && w[2] !== g[2]) console.log(`${prefix}${p.name.toUpperCase()}.${w[1]} =${g[2] === "" ? "" : " " + g[2]}`);
  }
}
if (gapsMode) {
  if (backends.length === 1 && backends[0] === "web") {
    gapsMode = false; // (the web's against what gaps.txt already says)
    await web(generate("web"), (p, out) => gapLines(p, out, "web: "));
  } else {
    for (const p of generate("desktop")) gapLines(p, vm(p).out, "");
  }
  process.exit(0);
}

if (backends.includes("vm") || backends.includes("native")) {
  const desktop = generate("desktop");
  if (backends.includes("vm")) for (const p of desktop) report("vm", p.name, vm(p).problems);
  if (backends.includes("native")) native(desktop);
}
if (backends.includes("web")) await web(generate("web"));
console.log(`\n${tally.pass} passed, ${tally.fail} failed`);
process.exit(tally.fail ? 1 : 0);
