// A built program started by a script or a terminal in another folder
// finds the pictures it came with (lanes T1b + the pictures' fallback):
// tests/fixtures/carried_files (lang/es.po, art/logo.svg) built as a plain
// executable in a folder of its own, its source folder hidden, started from
// an empty folder. Headless (RAPIDR_CAPTURE), the window's pixels read.
//
//  1. Spanish text, and the logo the build carried (its yellow sun drawn).
//  2. A logo in the current folder wins — RapidQ reads the current folder's
//     file first, and still does (a red sun).
//  3. With no logo anywhere: none (the picture stays empty).
//  4. loads.rr: a list's, a grid's and a tree's LoadFromFile, an RWave's and
//     an RMIDI's Open, a .X model's Load — the files it came with, a list
//     file in the current folder first, nothing when there's none.
//
// Usage (repo root, after building ./rapidr):
//   node tests/built_elsewhere.mjs [--native]
// (RAPIDR_BIN: another rapidr; --native also builds it with Rust, slower.)

import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
// (SEC-24: a release build's test hooks need the test-mode token: tests/test_mode.mjs)
import "./test_mode.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, process.platform === "win32" ? "rapidr.exe" : "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/built_elsewhere");
const EXE = process.platform === "win32" ? ".exe" : "";
const kinds = process.argv.includes("--native") ? ["interp", "native"] : ["interp"];

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };
const env = (extra) => ({
  ...process.env,
  RAPIDR_REGISTRY: join(WORK, "registry.reg"),
  RAPIDR_PRINT_TO: join(WORK, "prints"),
  RAPIDR_AI_KEYSTORE: "memory",
  RAPIDR_BUILD_CACHE: process.env.RAPIDR_BUILD_CACHE || join(ROOT, "tests/conformance/.work/build-cache"),
  ...extra,
});

/// The colour [r, g, b] at (x, y) of a 24- or 32-bit BMP capture.
function pixel(file, x, y) {
  const b = readFileSync(file);
  const off = b.readUInt32LE(10), w = b.readInt32LE(18), h = b.readInt32LE(22), bpp = b.readUInt16LE(28) / 8;
  const row = Math.ceil((w * bpp) / 4) * 4;
  const line = h > 0 ? h - 1 - y : y;
  const i = off + line * row + x * bpp;
  return [b[i + 2], b[i + 1], b[i]];
}

rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
for (const kind of kinds) {
  const src = join(WORK, kind, "src");
  const out = join(WORK, kind, "out");
  const cwd = join(WORK, kind, "cwd");
  cpSync(join(ROOT, "tests/fixtures/carried_files"), src, { recursive: true });
  mkdirSync(cwd, { recursive: true });
  const build = spawnSync(RAPIDR, ["build", join(src, "carried.rr"), out, "--no-bundle", ...(kind === "native" ? ["--debug"] : ["--interp"])], { encoding: "utf8", env: env({}), timeout: 1800000 });
  ok(build.status === 0 && /data files beside the executable: art\/logo\.svg, lang\/es\.po/.test(build.stdout), `${kind}: built with art/logo.svg and lang/es.po beside it${build.status ? `\n${build.stdout}${build.stderr}` : ""}`);
  if (build.status !== 0) continue;
  // (started from the empty folder, the source folder hidden)
  const start = (shot) => {
    renameSync(src, `${src}-hidden`);
    try {
      const r = spawnSync(join(out, `carried${EXE}`), ["es"], { cwd, encoding: "utf8", input: "", timeout: 60000,
        env: env({ RAPIDR_CAPTURE: join(WORK, kind, shot), RAPIDR_TEST_DUMP: "greeting.caption,logo.empty" }) });
      return { out: `${r.stdout}${r.stderr}`, shot: join(WORK, kind, `${shot}-1.bmp`) };
    } finally {
      renameSync(`${src}-hidden`, src);
    }
  };
  // (the logo's sun: at (48, 40) in the 96 × 96 picture placed at (16, 16))
  const sun = (r) => { try { return pixel(r.shot, 64, 56); } catch { return null; } };

  let r = start("own");
  let s = sun(r);
  ok(r.out.includes("greeting.caption=¡Hola, mundo!") && r.out.includes("logo.empty=0"), `${kind}: Spanish, and the logo it came with (${r.out.trim().split("\n").join(" | ")})`);
  ok(s && s[0] > 230 && s[1] > 180 && s[2] < 120, `${kind}: its yellow sun is drawn (${s})`);

  mkdirSync(join(cwd, "art"), { recursive: true });
  writeFileSync(join(cwd, "art/logo.svg"), readFileSync(join(src, "art/logo.svg"), "utf8").replace("#ffd33d", "#e01010"));
  r = start("here");
  s = sun(r);
  ok(r.out.includes("logo.empty=0") && s && s[0] > 200 && s[1] < 60 && s[2] < 60, `${kind}: a logo in the current folder wins (its red sun: ${s})`);

  rmSync(join(cwd, "art"), { recursive: true, force: true });
  rmSync(join(out, "art"), { recursive: true, force: true });
  r = start("none");
  ok(r.out.includes("logo.empty=-1"), `${kind}: no logo anywhere: none (${r.out.trim().split("\n").filter((l) => l.startsWith("logo")).join("")})`);

  // 4. The other files a program loads (loads.rr): a list, a grid and a
  // tree's LoadFromFile, a wave and a MIDI song's Open, a .X model's Load.
  const lout = join(WORK, kind, "loads-out");
  const lbuild = spawnSync(RAPIDR, ["build", join(src, "loads.rr"), lout, "--no-bundle", ...(kind === "native" ? ["--debug"] : ["--interp"])], { encoding: "utf8", env: env({}), timeout: 1800000 });
  const carried = ["data/names.txt", "data/table.csv", "data/tree.txt", "media/chime.wav", "media/tune.mid", "models/model.x"];
  const line = (lbuild.stdout.match(/data files beside the executable: (.*) \(/) || [])[1] || "";
  ok(lbuild.status === 0 && carried.every((f) => line.split(", ").includes(f)), `${kind}: loads.rr built with ${carried.join(", ")} (${line || lbuild.stdout + lbuild.stderr})`);
  if (lbuild.status !== 0) continue;
  const loads = (shot) => {
    renameSync(src, `${src}-hidden`);
    try {
      const x = spawnSync(join(lout, `loads${EXE}`), [], { cwd, encoding: "utf8", input: "", timeout: 60000,
        env: env({ RAPIDR_CAPTURE: join(WORK, kind, shot), RAPIDR_TEST_DUMP: "info.caption,mesh.caption" }) });
      return `${x.stdout}${x.stderr}`;
    } finally {
      renameSync(`${src}-hidden`, src);
    }
  };
  let got = loads("loads-own");
  ok(got.includes("info.caption=list Ben, grid pears, tree4, wave1, midi1") && got.includes("mesh.caption=faces2"),
    `${kind}: started elsewhere, the list, grid, tree, wave, MIDI song and model it came with (${got.trim().split("\n").filter((l) => l.includes("caption")).join(" | ")})`);
  // (the current folder's list wins)
  mkdirSync(join(cwd, "data"), { recursive: true });
  writeFileSync(join(cwd, "data/names.txt"), "Zed\nYan\n");
  got = loads("loads-here");
  ok(got.includes("info.caption=list Yan, grid pears"), `${kind}: a list file in the current folder wins (${got.trim().split("\n").filter((l) => l.startsWith("info")).join("")})`);
  rmSync(join(cwd, "data"), { recursive: true, force: true });
  for (const d of ["data", "media", "models"]) rmSync(join(lout, d), { recursive: true, force: true });
  got = loads("loads-none");
  ok(got.includes("wave0, midi0") && got.includes("mesh.caption=faces0") && got.includes("info.caption=list , grid , tree0"),
    `${kind}: none anywhere: nothing loaded (${got.trim().split("\n").filter((l) => l.includes("caption")).join(" | ")})`);
}
console.log(failed ? `\nBuilt elsewhere: ${failed} FAILED` : "\nBuilt elsewhere: ALL CHECKS PASSED");
process.exit(failed ? 1 : 0);
