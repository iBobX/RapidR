// A theme never moves or resizes anything (ROADMAP Phase 3B, `$THEME
// rapidr`): every GUI program of examples/ — and, with --corpus, RapidQ's
// own examples (~/Downloads/Rapidq/examples or $RAPIDQ_DIR/examples, read
// only) — run on the UI kernel's headless host in RapidR's look (light,
// dark, high contrast) and in the classic one; every window's components
// (the accessibility tree's nodes: forms, components, their items) must be
// where the classic look has them, the same size (text that sizes a thing —
// an AutoSize label, a tab — is measured as RC.EXE measures it in every
// theme, whatever face the theme draws it in).
//
//   node tests/theme_geometry.mjs [--corpus] [filter …]   (filter `corpus/`: the corpus alone)
//
// Needs ./rapidr (cargo build --release -p rapidr-cli) and the runner
// (built here: cargo build -p rapidr-runner-stub --profile runner). Nothing
// reaches a printer or the user's registry; programs that need a network,
// a device or input to show a window are skipped (no window, no tree).
import { execFileSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const corpus = args.includes("--corpus");
const only = args.filter((a) => !a.startsWith("--"));
const WORK = join(ROOT, "tests/conformance/.work/theme_geometry");
rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
const EXE = process.platform === "win32" ? ".exe" : "";
const THEMES = ["classic", "rapidr light", "rapidr dark", "rapidr high contrast"];

execFileSync("cargo", ["build", "--quiet", "-p", "rapidr-runner-stub", "--profile", "runner"], { cwd: ROOT, stdio: "inherit" });
const stub = readFileSync(join(ROOT, "target/runner", `rapidrintr-runner${EXE}`));

// The programs: .bas / .rr files that make a form.
function programs(dir) {
  const out = [];
  const walk = (d) => {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.(bas|rr)$/i.test(f) && /\b[QR]FORM\b/i.test(readFileSync(p, "latin1"))) out.push(p);
    }
  };
  walk(dir);
  return out.sort();
}
const sources = [...programs(join(ROOT, "examples"))];
if (corpus) {
  // (a copy, run from there: the corpus is read only, and its programs may
  // write beside themselves)
  const dir = join(process.env.RAPIDQ_DIR || join(homedir(), "Downloads/Rapidq"), "examples");
  if (existsSync(dir)) {
    const copy = join(WORK, "corpus");
    cpSync(dir, copy, { recursive: true });
    sources.push(...programs(copy));
  }
}

// Every node's place, by its path in the tree (its role and where it is: a
// name may be a score or a clock).
function places(json) {
  const out = [];
  const walk = (n, path) => {
    const here = `${path}/${n.role}`;
    out.push(`${here} ${n.bounds.join(",")}`);
    (n.children || []).forEach((c, i) => walk(c, `${here}#${i}`));
  };
  json.forEach((t, i) => walk(t, `window${i}`));
  return out;
}

let same = 0, differ = 0, skipped = 0;
const failures = [];
for (const src of sources.filter((s) => !only.length || only.some((f) => s.includes(f)))) {
  const name = relative(ROOT, src).replace(/[^A-Za-z0-9]+/g, "_");
  const bin = join(WORK, name + EXE);
  try {
    const rrbc = join(WORK, name + ".rrbc");
    execFileSync(join(ROOT, "rapidr" + EXE), ["build-bc", src, "-o", rrbc], { cwd: dirname(src), stdio: "ignore", timeout: 60000 });
    const payload = readFileSync(rrbc);
    const len = Buffer.alloc(4);
    len.writeUInt32LE(payload.length);
    writeFileSync(bin, Buffer.concat([stub, payload, Buffer.from("RRBCEXE1"), len]));
    chmodSync(bin, 0o755);
  } catch {
    skipped++;
    continue;
  }
  const trees = {};
  for (const theme of THEMES) {
    const a11y = join(WORK, `${name}-${theme.replace(/ /g, "-")}.json`);
    const env = {
      ...process.env, RAPIDR_THEME: theme, RAPIDR_SCALE: "1", RAPIDR_CAPTURE: join(WORK, `${name}-cap`), RAPIDR_TEST_A11Y: a11y,
      RAPIDR_PRINT_TO: join(WORK, "prints"), RAPIDR_REGISTRY: join(WORK, "registry.reg"), RAPIDR_TEST_HTTP: "", RAPIDR_TEST_COMPORT: "", RAPIDR_TEST_MIDI: "",
      RAPIDR_TEST_WAVE_IN: "tone:440",
    };
    try {
      execFileSync(bin, [], { env, cwd: dirname(src), timeout: 30000, stdio: "ignore" });
    } catch {}
    trees[theme] = existsSync(a11y) ? places(JSON.parse(readFileSync(a11y, "utf8"))) : null;
  }
  if (!trees.classic || !trees.classic.length) {
    skipped++;
    continue;
  }
  const wrong = THEMES.slice(1).filter((t) => JSON.stringify(trees[t]) !== JSON.stringify(trees.classic));
  if (wrong.length) {
    differ++;
    const t = trees[wrong[0]] || [];
    const first = trees.classic.findIndex((l, i) => l !== t[i]);
    failures.push(`${relative(ROOT, src)} (${wrong.join(", ")}): classic ${trees.classic[first]} | ${wrong[0]} ${t[first]}`);
  } else {
    same++;
  }
}
console.log(`theme geometry: ${same} programs the same in every theme, ${differ} not, ${skipped} skipped (no window)`);
for (const f of failures) console.log("  " + f);
process.exit(differ ? 1 : 0);
