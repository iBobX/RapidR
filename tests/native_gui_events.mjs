// EVENT blocks in TYPE … EXTENDS QBUTTON and Sender parameters in a NATIVE
// build (Rust backend, objects.rs): builds tests/fixtures/oop_events.bas with
// `rapidr build`, fires clicks through the runtime's test hooks
// (RAPIDR_TEST_EVENTS / RAPIDR_TEST_DUMP, crates/rapidr-runtime-core/src/gui.rs)
// and checks each instance's own state. Needs a desktop session (FLTK).
//
// Usage (repo root, after building ./rapidr):  node tests/native_gui_events.mjs

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "tests/conformance/.work/native_gui_events");
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
execFileSync(join(ROOT, "rapidr"), ["build", join(ROOT, "tests/fixtures/oop_events.bas"), OUT], {
  cwd: ROOT, stdio: "inherit", env: { ...process.env, CARGO_TARGET_DIR: join(ROOT, "tests/conformance/.work/cargo-target") },
});
const bin = join(ROOT, "tests/conformance/.work/cargo-target/debug/oop_events");
ok(existsSync(bin), "native executable built");
const out = execFileSync(bin, [], {
  encoding: "utf8",
  env: { ...process.env, RAPIDR_CAPTURE: join(OUT, "window"), RAPIDR_TEST_EVENTS: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", RAPIDR_TEST_DUMP: "b1.caption,b2.caption,b3.caption" },
});
ok(/b1\.caption=Clicked 2/.test(out), "first instance counted its own clicks (This)");
ok(/b2\.caption=Clicked 1/.test(out), "second instance has its own state");
ok(/b3\.caption=Sender works/.test(out), "a component-typed Sender parameter works natively");
if (failed) { console.log(`\nNative GUI events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nNative GUI events: ALL CHECKS PASSED");
