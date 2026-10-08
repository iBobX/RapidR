// Conformance suite: small BASIC programs with the output correct RapidQ/BASIC
// semantics must produce, run on BOTH backends (ROADMAP principle 2: the
// bytecode VM and the Rust codegen must behave identically).
//
// Layout (tests/conformance/cases/):
//   name.bas              the program (console only; no GUI)
//   name.expected         exact expected stdout (trailing whitespace ignored)
//   name.input            optional: what the program reads (INPUT) on stdin
//   name.args             optional: the program's command-line arguments, one
//                         per line (COMMAND$(n), CommandCount)
//   name.expected-error   OR: the program must FAIL to compile, and the
//                         compiler output must contain every non-empty line
//                         of this file (e.g. "3:1" and "Unknown SUB")
//   name.expected-runtime-error  with name.expected: the program prints
//                         that, then stops with a run-time error whose
//                         message has every non-empty line of this file
//                         (e.g. RapidQ's "Division by zero")
//
// Known bugs are tracked with a first-line marker in the .bas file:
//   ' xfail: vm, codegen — reason (ROADMAP item)
// A marked backend that fails is XFAIL (ok); one that passes is XPASS, which
// fails the run so the marker gets removed once the bug is fixed.
//
// Usage (from repo root, after ./build.sh):
//   node tests/conformance/run.mjs                  # both backends
//   node tests/conformance/run.mjs --backend vm     # fast: VM only
//   node tests/conformance/run.mjs gosub case_      # only matching cases
// Env: RAPIDR_BIN (default ./rapidr), CONFORMANCE_WORK (scratch dir).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, copyFileSync, cpSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dropBuild } from "../cargo_builds.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../..");
const CASES = join(HERE, "cases");
// (Windows: programs are .exe)
const EXE = process.platform === "win32" ? ".exe" : "";
const RAPIDR = resolve(ROOT, process.env.RAPIDR_BIN || `rapidr${EXE}`);
// (the programs use their own clipboard, never the user's)
process.env.RAPIDR_TEST_CLIPBOARD = "1";
const WORK = resolve(process.env.CONFORMANCE_WORK || join(HERE, ".work"));
const TIMEOUT_MS = 30_000;
const MAX_OUTPUT = 1 << 20;  // 1 MB: more than any case prints; catches runaway loops

const args = process.argv.slice(2);
let backends = ["vm", "codegen"];
const filters = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--backend") backends = [args[++i]];
  else filters.push(args[i]);
}

if (!existsSync(RAPIDR)) {
  console.error(`rapidr binary not found at ${RAPIDR} (run ./build.sh or set RAPIDR_BIN)`);
  process.exit(2);
}
mkdirSync(WORK, { recursive: true });

const norm = (s) => s.replace(/\r\n/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").trimEnd();

// A case that prints (Printer.EndDoc) writes a PDF here, never on paper.
const PRINTS = join(WORK, "prints");
mkdirSync(PRINTS, { recursive: true });

// Each run's QREGISTRY keys in a fresh store of their own (never the
// user's).
let runs = 0;
function run(cmd, cmdArgs, opts = {}) {
  const registry = join(WORK, `registry-${process.pid}-${runs++}.reg`);
  rmSync(registry, { force: true });
  // (and no real gamepad: QDXJOYSTICK reads an empty test script; no
  // real serial port: QCOMPORT has the empty test script's — none; no
  // sound, MIDI output or recording input)
  opts = { ...opts, env: { ...(opts.env || process.env), RAPIDR_PRINT_TO: PRINTS, RAPIDR_REGISTRY: registry, RAPIDR_TEST_JOYSTICK: "", RAPIDR_TEST_COMPORT: "", RAPIDR_TEST_SOUND: "", RAPIDR_TEST_MIDI: "", RAPIDR_TEST_WAVE_IN: "" } };
  const r = spawnSync(cmd, cmdArgs, { encoding: "utf8", timeout: TIMEOUT_MS, maxBuffer: MAX_OUTPUT, ...opts });
  rmSync(registry, { force: true });
  let err = r.stderr || "";
  if (r.error?.code === "ENOBUFS") err += `\noutput exceeded ${MAX_OUTPUT} bytes (runaway loop?)`;
  else if (r.error?.code === "ETIMEDOUT") err += `\ntimed out after ${opts.timeout || TIMEOUT_MS} ms`;
  else if (r.error) err += `\n${r.error}`;
  return { ok: r.status === 0 && !r.error, out: r.stdout || "", err };
}

// Returns { compiled: bool, output: string, diagnostics: string }.
function runVm(name, src, input, progArgs) {
  const rrbc = join(WORK, `${name}.rrbc`);
  const c = run(RAPIDR, ["build-bc", src, "-o", rrbc]);
  if (!c.ok) return { compiled: false, output: "", diagnostics: c.out + c.err };
  const r = run(RAPIDR, ["run-bc", rrbc, ...progArgs], { input });
  return { compiled: true, output: r.out, diagnostics: c.err + r.err, crashed: !r.ok };
}

function runCodegen(name, src, input, progArgs) {
  const dir = join(WORK, "codegen");
  mkdirSync(dir, { recursive: true });
  const rr = join(dir, `${name}.rr`);
  copyFileSync(src, rr);
  // Files cases build in with $RESOURCE, next to the copy as next to the case.
  // (resource_files: files a case lists; picture_files: pictures it loads;
  // video_files: QVIDEO's clips)
  for (const folder of ["resource_files", "picture_files", "video_files"]) {
    const resources = join(CASES, folder);
    if (existsSync(resources)) cpSync(resources, join(dir, folder), { recursive: true });
  }
  const env = { ...process.env, CARGO_TARGET_DIR: join(WORK, "cargo-target") };
  const c = run(RAPIDR, ["build", rr, join(dir, `${name}_rust`), "--no-bundle"], { env, timeout: 600_000 });
  const bin = join(dir, `${name}${EXE}`);
  if (!c.ok || !existsSync(bin)) {
    dropBuild(env.CARGO_TARGET_DIR, name);
    return { compiled: false, output: "", diagnostics: cargoErrors(c.out + c.err) };
  }
  const r = run(bin, progArgs, { input });
  // (its build, ~350 MB, gone once it ran: tests/cargo_builds.mjs)
  rmSync(bin, { force: true });
  dropBuild(env.CARGO_TARGET_DIR, name);
  return { compiled: true, output: r.out, diagnostics: r.err, crashed: !r.ok };
}

// Cargo output is mostly warnings from the runtime crates; keep only the
// error blocks (the "error..." line plus its location/snippet lines).
function cargoErrors(text) {
  const out = [];
  let inError = false;
  for (const line of text.split("\n")) {
    if (/^error/.test(line)) inError = true;
    else if (/^(warning|\s*Compiling|\s*Building|\s*Finished)/.test(line) || line.trim() === "") inError = false;
    if (inError && !/could not compile|aborting due to/.test(line)) out.push(line);
  }
  return out.length ? out.join("\n") : text;
}

const RUNNERS = { vm: runVm, codegen: runCodegen };

function check(result, expected, expectedError, runtimeError = null) {
  if (expectedError !== null) {
    if (result.compiled) return "compiled, but a compile error was expected";
    const missing = expectedError.split("\n").map((l) => l.trim()).filter(Boolean)
      .filter((needle) => !result.diagnostics.includes(needle));
    return missing.length ? `error output lacks: ${missing.join(" | ")}` : null;
  }
  if (!result.compiled) return `failed to compile:\n${result.diagnostics.trim()}`;
  if (runtimeError !== null) {
    if (!result.crashed) return `ran to the end, but a run-time error was expected\n--- output ---\n${result.output}`;
    const missing = runtimeError.split("\n").map((l) => l.trim()).filter(Boolean)
      .filter((needle) => !result.diagnostics.includes(needle));
    if (missing.length) return `error output lacks: ${missing.join(" | ")}\n--- error output ---\n${result.diagnostics.trim()}`;
  } else if (result.crashed) return `crashed:\n${result.diagnostics.trim()}\n--- output ---\n${result.output}`;
  if (norm(result.output) !== norm(expected)) {
    return `output differs\n--- expected ---\n${norm(expected)}\n--- actual ---\n${norm(result.output)}`;
  }
  return null;
}

const cases = readdirSync(CASES).filter((f) => f.endsWith(".bas")).map((f) => f.slice(0, -4)).sort()
  .filter((n) => !filters.length || filters.some((f) => n.includes(f)));

const tally = { PASS: 0, FAIL: 0, XFAIL: 0, XPASS: 0 };
for (const name of cases) {
  const src = join(CASES, `${name}.bas`);
  const text = readFileSync(src, "utf8");
  const xfailLine = text.split("\n")[0].match(/^'\s*xfail:\s*([^—-]+)/i);
  const xfail = new Set(xfailLine ? xfailLine[1].split(",").map((s) => s.trim().toLowerCase()) : []);
  // `' skip-on: win32` (a process.platform name): a case whose outcome is
  // another on that system (a Windows DLL call succeeds there) isn't run.
  const skipLine = text.split("\n")[0].match(/^'\s*skip-on:\s*([^—-]+)/i);
  if (skipLine && skipLine[1].split(",").map((s) => s.trim().toLowerCase()).includes(process.platform)) {
    console.log(`SKIP     ${name} (not on ${process.platform})`);
    continue;
  }
  const expPath = join(CASES, `${name}.expected`);
  const errPath = join(CASES, `${name}.expected-error`);
  const expected = existsSync(expPath) ? readFileSync(expPath, "utf8") : "";
  const expectedError = existsSync(errPath) ? readFileSync(errPath, "utf8") : null;
  const runtimePath = join(CASES, `${name}.expected-runtime-error`);
  const runtimeError = existsSync(runtimePath) ? readFileSync(runtimePath, "utf8") : null;
  const inputPath = join(CASES, `${name}.input`);
  const input = existsSync(inputPath) ? readFileSync(inputPath, "utf8") : "";
  const argsPath = join(CASES, `${name}.args`);
  const progArgs = existsSync(argsPath) ? readFileSync(argsPath, "utf8").replace(/\r\n/g, "\n").split("\n").filter((l) => l !== "") : [];

  for (const backend of backends) {
    const problem = check(RUNNERS[backend](name, src, input, progArgs), expected, expectedError, runtimeError);
    let status;
    if (problem && xfail.has(backend)) status = "XFAIL";
    else if (problem) status = "FAIL";
    else if (xfail.has(backend)) status = "XPASS";
    else status = "PASS";
    tally[status]++;
    console.log(`${status.padEnd(5)}  ${backend.padEnd(7)}  ${name}`);
    if (status === "FAIL" || (process.env.VERBOSE && problem)) {
      const lines = problem.split("\n");
      const shown = lines.length > 40 ? [...lines.slice(0, 40), `... (${lines.length - 40} more lines)`] : lines;
      console.log(shown.map((l) => "         | " + l).join("\n"));
    }
    if (status === "XPASS") console.log(`         | passes now — remove '${backend}' from the xfail marker`);
  }
}

console.log(`\n${tally.PASS} passed, ${tally.XFAIL} known failures (xfail), ${tally.FAIL} failed, ${tally.XPASS} unexpectedly passed`);
process.exit(tally.FAIL || tally.XPASS ? 1 : 0);
