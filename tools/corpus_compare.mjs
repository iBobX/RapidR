// Runs the RapidQ example corpus both ways — native (`rapidr build`) and
// interpreted (`rapidr build --interp`) — and compares what each prints and
// the windows it shows (RAPIDR_CAPTURE, crates/rapidr-runtime-core/src/
// gui.rs), to find where the two backends behave differently.
//
//   node tools/corpus_compare.mjs <corpus.json> [examples dir] [include dir] [filter,…]
//
// Only programs the report lists as compiling to bytecode are tried. Each
// program's folder is copied to a work directory first (programs load their
// BMPs etc. from there and may write files), so the corpus isn't touched.
// Programs using the network (sockets, FTP, HTTP, MySQL) are skipped: they'd
// contact real hosts. Each run gets no input and at most RUN_SECONDS.
// Differences in programs using RND / TIMER / TIME$ … are reported apart,
// as "differs (random / clock)".
// Needs a desktop session (windows appear briefly). Writes
// tests/conformance/.work/corpus-compare/report.json.

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [jsonPath, examplesArg, includeArg, filter] = process.argv.slice(2);
if (!jsonPath) {
  console.error("usage: node tools/corpus_compare.mjs <corpus.json> [examples dir] [include dir] [filter]");
  process.exit(2);
}
const EXAMPLES = examplesArg || join(homedir(), "Downloads/Rapidq/examples");
const INCLUDE = includeArg || join(homedir(), "Downloads/Rapidq/include");
const WORK = join(ROOT, "tests/conformance/.work/corpus-compare");
const CARGO_TARGET = join(ROOT, "tests/conformance/.work/cargo-examples");
const RUN_SECONDS = 20;
const CURSOR_PIXELS = 4;
const NETWORK = /\b(qsocket|qclientsocket|qserversocket|qmysql|qhttp|qftp|qsmtp|qpop3|qwebbrowser|inet)\b/i;
// Programs whose output depends on random numbers or the clock: their
// differences are listed but counted apart.
const VARIES = /\b(rnd|randomize|timer|time\$|date\$|tickcount|gettickcount|now|sleep)\b/i;

// No incremental build caches: every program is its own crate, so they'd
// add up to tens of GB over the corpus.
// Printer.EndDoc saves PDFs here instead of printing on paper.
const PRINTS = join(ROOT, "tests/conformance/.work/corpus-prints");
mkdirSync(PRINTS, { recursive: true });
// …and QREGISTRY writes a scratch store, never the user's.
const env = { ...process.env, RAPIDR_INCLUDE_PATH: INCLUDE, CARGO_TARGET_DIR: CARGO_TARGET, CARGO_INCREMENTAL: "0", RAPIDR_PRINT_TO: PRINTS, RAPIDR_REGISTRY: join(ROOT, "tests/conformance/.work/corpus-registry.reg") };

// The package name `rapidr build` gives a program (rapidr_codegen_rust::crate_name).
function crateName(stem) {
  const name = stem.replace(/[^A-Za-z0-9_-]/g, "_");
  return !name || /^[0-9-]/.test(name) ? `rq_${name}` : name;
}

// Removes a program's own build outputs (the shared runtime builds stay).
function cleanNativeBuild(stem) {
  const crate = crateName(stem);
  const artifact = crate.replace(/-/g, "_");
  for (const sub of ["debug", "debug/deps", "debug/.fingerprint", "debug/build"]) {
    const dir = join(CARGO_TARGET, sub);
    if (!existsSync(dir)) continue;
    for (const f of readdirSync(dir)) {
      if (f === crate || f === artifact || f.startsWith(`${crate}-`) || f.startsWith(`${artifact}-`) || f.startsWith(`${crate}.`)) {
        rmSync(join(dir, f), { recursive: true, force: true });
      }
    }
  }
}
const results = JSON.parse(readFileSync(jsonPath, "utf8")).results;
// `filter`: comma-separated parts of program paths (e.g. "grids/,games/puzzle").
const wanted = filter ? filter.split(",").map((f) => f.trim()).filter(Boolean) : [];
const programs = Object.keys(results).filter((k) => !results[k] || results[k].length === 0).filter((k) => !wanted.length || wanted.some((f) => k.includes(f))).sort();
mkdirSync(WORK, { recursive: true });

function build(src, out, interp) {
  const args = ["build", src, out, "--no-bundle", ...(interp ? ["--interp"] : [])];
  const r = spawnSync(join(ROOT, "rapidr"), args, { cwd: dirname(src), env, encoding: "utf8", timeout: 600_000 });
  return r.status === 0 ? null : (r.stderr || r.stdout || "").split("\n").filter((l) => /error/i.test(l)).slice(0, 3).join(" | ") || `exit ${r.status}`;
}

// Window captures go next to the program's folder, not in it: programs
// that list their directory (QFILELISTBOX) must see the same files in both
// runs.
function run(bin, cwd, prefix) {
  const capDir = dirname(prefix);
  mkdirSync(capDir, { recursive: true });
  for (const f of readdirSync(capDir)) if (f.startsWith(basename(prefix) + "-")) rmSync(join(capDir, f));
  const r = spawnSync(bin, [], {
    cwd, input: "", encoding: "utf8", timeout: RUN_SECONDS * 1000, killSignal: "SIGKILL",
    env: { ...env, RAPIDR_CAPTURE: prefix, RAPIDR_CAPTURE_DELAY: "2" },
  });
  const captures = readdirSync(capDir).filter((f) => f.startsWith(basename(prefix) + "-") && f.endsWith(".bmp")).sort()
    .map((f) => readFileSync(join(capDir, f)));
  // Messages the capture hook itself prints, and paths, don't count.
  const clean = (s) => (s || "").split("\n").filter((l) => !l.startsWith("[rapidr] captured window")).join("\n").replaceAll(cwd, "<dir>");
  return { stdout: clean(r.stdout), stderr: clean(r.stderr), status: r.status, timedOut: r.error?.code === "ETIMEDOUT", captures };
}

// Pixels that differ between two same-sized BMPs (bottom-up 24-bit from
// codec::encode_bmp): a count, so tiny text-antialiasing noise is visible
// as such.
function pixelDiff(a, b) {
  if (a.length !== b.length) return Infinity;
  let n = 0;
  for (let i = 54; i < a.length; i += 3) if (a[i] !== b[i] || a[i + 1] !== b[i + 1] || a[i + 2] !== b[i + 2]) n++;
  return n;
}

const report = [];
for (const rel of programs) {
  const src0 = join(EXAMPLES, rel);
  const text = readFileSync(src0, "latin1");
  const name = rel.replace(/[\/ ]/g, "_").replace(/\.(bas|rqw|rqb|rq)$/i, "");
  const entry = { program: rel };
  report.push(entry);
  if (NETWORK.test(text)) {
    entry.result = "skipped (network)";
    console.log(`- ${rel}: ${entry.result}`);
    continue;
  }
  const dir = join(WORK, name);
  rmSync(dir, { recursive: true, force: true });
  cpSync(dirname(src0), dir, { recursive: true, filter: (s) => !/\.(exe|zip|rar|dll)$/i.test(s) });
  const src = join(dir, basename(rel));
  const stem = basename(rel, extname(rel));
  const nerr = build(src, join(dir, "native-project"), false);
  const ierr = build(src, join(dir, "interp"), true);
  if (nerr || ierr) {
    entry.result = nerr && ierr ? "build fails both ways" : nerr ? "native build fails" : "interpreted build fails";
    entry.detail = nerr || ierr;
    console.log(`✗ ${rel}: ${entry.result}: ${entry.detail}`);
    continue;
  }
  const captures = `${dir}.captures`;
  const native = run(join(dir, stem), dir, join(captures, "cap-native"));
  const interp = run(join(dir, "interp", stem), dir, join(captures, "cap-interp"));
  // Only what's needed to look into a difference stays on disk.
  cleanNativeBuild(stem);
  rmSync(join(dir, "native-project"), { recursive: true, force: true });
  const diffs = [];
  if (native.stdout !== interp.stdout) diffs.push("output");
  if (native.timedOut !== interp.timedOut) diffs.push(`timeout (native ${native.timedOut}, interpreted ${interp.timedOut})`);
  if (!native.timedOut && !interp.timedOut && native.status !== interp.status) diffs.push(`exit (${native.status} vs ${interp.status})`);
  if (native.captures.length !== interp.captures.length) diffs.push(`windows (${native.captures.length} vs ${interp.captures.length})`);
  else native.captures.forEach((c, i) => {
    // A few pixels: a text box's blinking cursor caught in another phase.
    const d = pixelDiff(c, interp.captures[i]);
    if (d > CURSOR_PIXELS) diffs.push(`window ${i + 1}: ${d === Infinity ? "size" : d + " px"}`);
  });
  entry.windows = native.captures.length;
  entry.result = !diffs.length ? "same" : VARIES.test(text) ? "differs (random / clock)" : "differs";
  if (diffs.length) {
    entry.diffs = diffs;
    entry.native = { stdout: native.stdout.slice(0, 400), stderr: native.stderr.slice(0, 400), status: native.status };
    entry.interp = { stdout: interp.stdout.slice(0, 400), stderr: interp.stderr.slice(0, 400), status: interp.status };
  }
  console.log(`${diffs.length ? "✗" : "✓"} ${rel}: ${entry.result}${diffs.length ? " — " + diffs.join("; ") : ""} (${entry.windows} window(s))`);
}

writeFileSync(join(WORK, "report.json"), JSON.stringify(report, null, 1));
const count = (r) => report.filter((e) => e.result === r).length;
console.log(`\nsame=${count("same")} differs=${count("differs")} varies=${count("differs (random / clock)")} skipped=${report.filter((e) => e.result.startsWith("skipped")).length} build-failures=${report.filter((e) => /build/.test(e.result)).length} of ${report.length}`);
console.log(`report: ${join(WORK, "report.json")}`);
