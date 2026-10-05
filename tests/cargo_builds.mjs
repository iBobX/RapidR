// A native build's artifacts in the test runners' shared cargo target,
// removed once its program ran: each is ~350 MB (a debug executable with
// every runtime crate linked in), and a full run builds ~180 of them
// (tests/conformance/run.mjs, tests/native_gui_events.mjs) — 50 to 100 GB
// left behind otherwise. The dependencies stay built.

import { existsSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

// rapidr_codegen_rust::crate_name: the package name `rapidr build` gives a
// program.
export function crateName(stem) {
  const name = stem.replace(/[^A-Za-z0-9_-]/g, "_");
  return name === "" || /^[0-9-]/.test(name) ? `rq_${name}` : name;
}

export function dropBuild(target, stem, profile = "debug") {
  const crate = crateName(stem);
  // (cargo's file names: `-` as `_`, then `-<hash>`)
  const prefixes = [`${crate.replace(/-/g, "_")}-`, `${crate}-`];
  const dir = join(target, profile);
  for (const f of [crate, `${crate}.exe`, `${crate}.pdb`, `${crate}.d`]) rmSync(join(dir, f), { recursive: true, force: true });
  for (const sub of ["deps", "incremental", ".fingerprint"]) {
    const d = join(dir, sub);
    if (!existsSync(d)) continue;
    for (const f of readdirSync(d)) {
      if (prefixes.some((p) => f.startsWith(p))) rmSync(join(d, f), { recursive: true, force: true });
    }
  }
}
