// Console input, line by line (Robert's lane list, 2026-10-22): `INPUT` on the
// console and `LINE INPUT #f` on `/dev/stdin` return each line as soon
// as it arrives — not when all the input has. Line-by-line protocols (an MCP
// stdio server: one JSON-RPC line in, one out) depend on it: the program must
// have answered the first line before the second is written.
//
// Each program is started with a pipe for stdin; the first line is written,
// the answer must come on stdout (within a few seconds) while the pipe is
// still open and the second line has not been written; then the second line
// is written and its answer must come, and the pipe is closed.
//
//   run     `rapidr run prog.bas`  (the RapidR Runtime, the bytecode VM)
//   interp  `rapidr build prog.bas dir --interp`: a standalone executable
//   native  `rapidr build prog.bas dir`: generated Rust, built by cargo
//           (slow: only with --runtimes native)
//
// The web has no stdin (a page has no pipe to read): nothing to check there.
//
// Usage (repo root, after building ./rapidr):
//   node tests/stdin_lines.mjs [--runtimes run,interp,native]

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RAPIDR = join(ROOT, process.platform === "win32" ? "rapidr.exe" : "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/stdin_lines");
const EXE = process.platform === "win32" ? ".exe" : "";
const argv = process.argv.slice(2);
const runtimes = argv.includes("--runtimes") ? argv[argv.indexOf("--runtimes") + 1].split(",") : ["run", "interp"];

const programs = {
  // (the console's own input: INPUT reads a line; `LINE INPUT` is the file form, #n)
  console: ["$APPTYPE CONSOLE", "DIM a AS STRING, b AS STRING", "INPUT a", 'PRINT "got "; a', "INPUT b", 'PRINT "got "; b'],
  // (a file that is the console's own input: not read whole when it is opened)
  devstdin: ["$APPTYPE CONSOLE", "DIM a AS STRING, b AS STRING", 'OPEN "/dev/stdin" FOR INPUT AS #1', "LINE INPUT #1, a", 'PRINT "got "; a', "LINE INPUT #1, b", 'PRINT "got "; b', "CLOSE #1"],
  // (EOF asks whether another line is coming: it waits for one, or the end)
  devstdin_eof: ["$APPTYPE CONSOLE", "DIM a AS STRING", 'OPEN "/dev/stdin" FOR INPUT AS #1', "DO WHILE NOT EOF(1)", "LINE INPUT #1, a", 'PRINT "got "; a', "LOOP", 'PRINT "end"', "CLOSE #1"],
};

let failed = 0, passed = 0;
const ok = (cond, msg, detail = "") => {
  console.log(`${cond ? "✓" : "✗"} ${msg}${cond || !detail ? "" : `\n    ${detail}`}`);
  cond ? passed++ : failed++;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/// Starts `cmd`, feeds it line by line as described above; resolves with what
/// it printed at each step.
async function drive(cmd, args, lines, name) {
  const child = spawn(cmd, args, { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, RAPIDR_PRINT_TO: join(WORK, "prints"), RAPIDR_REGISTRY: join(WORK, "registry.reg") } });
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  let err = "";
  child.stderr.on("data", (d) => (err += d));
  const exited = new Promise((r) => child.on("exit", (code) => r(code)));
  const seen = [];
  // (a line's answer: within 4 s, the pipe still open — the process waits for the next line)
  const waitFor = async (text, ms) => {
    for (let t = 0; t < ms; t += 50) {
      if (out.includes(text)) return true;
      await sleep(50);
    }
    return out.includes(text);
  };
  for (const [i, line] of lines.entries()) {
    child.stdin.write(line + "\n");
    const got = await waitFor(`got ${line}`, 4000);
    seen.push({ line, got, alive: child.exitCode === null });
    // (the next line is only written after this one was answered: a program that reads everything first never gets here)
    if (!got) break;
  }
  child.stdin.end();
  const code = await Promise.race([exited, sleep(6000).then(() => "timeout")]);
  if (code === "timeout") child.kill();
  return { seen, out, err, code };
}

rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
for (const [name, lines] of Object.entries(programs)) writeFileSync(join(WORK, `${name}.bas`), lines.join("\n") + "\n");

for (const runtime of runtimes) {
  for (const name of Object.keys(programs)) {
    const src = join(WORK, `${name}.bas`);
    let cmd = RAPIDR, args = ["run", src];
    if (runtime !== "run") {
      const dir = join(WORK, `${name}-${runtime}`);
      mkdirSync(dir, { recursive: true });
      const build = spawnSync(RAPIDR, ["build", src, dir, "--no-bundle", ...(runtime === "interp" ? ["--interp"] : ["--debug"])], { encoding: "utf8", cwd: ROOT, env: { ...process.env, CARGO_TARGET_DIR: join(WORK, "cargo-target"), RAPIDR_BUILD_CACHE: join(WORK, "build-cache") } });
      cmd = join(dir, `${name}${EXE}`);
      args = [];
      if (!existsSync(cmd)) { ok(false, `${name} (${runtime}): built`, (build.stderr || build.stdout || "").slice(-400)); continue; }
    }
    const lines = name === "devstdin_eof" ? ["first", "second"] : ["first", "second"];
    const r = await drive(cmd, args, lines, name);
    ok(r.seen[0]?.got, `${name} (${runtime}): the first line is answered before the second is written`, `got ${JSON.stringify(r.out)} ${r.err.slice(-200)}`);
    ok(r.seen[1]?.got, `${name} (${runtime}): the second line is answered`, `got ${JSON.stringify(r.out)}`);
    if (name === "devstdin_eof") ok(r.out.includes("end"), `${name} (${runtime}): EOF is true once the input ends`, `got ${JSON.stringify(r.out)}`);
    ok(r.code === 0 || r.code === null, `${name} (${runtime}): ends with the input (exit ${r.code})`);
  }
}
console.log(`\nStdin lines: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
