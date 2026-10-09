// SEC-20 regressions (docs/security-audit.md): the supply chain's findings
// stay fixed.
//
//  - memmap2 ≥ 0.9.11 (RUSTSEC-2026-0186, unsound `advise` / `flush_range`
//    in 0.9.10, shipped through fontique / parley and winit on Linux), and
//    cargo-deny told to fail on unsound crates (it didn't report it at all).
//  - anyhow ≥ 1.0.103 (RUSTSEC-2026-0190; in the lockfile only).
//  - No stale ignore (RUSTSEC-2026-0173, proc-macro-error2, left the graph).
//  - ttf-parser (RUSTSEC-2026-0192, unmaintained) is ignored on one ground:
//    it only parses fonts built into RapidR — pinned here.
//  - cargo deny (advisories) and cargo audit are clean, offline, when installed.
//
//   node tests/security/supply_chain.mjs

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const read = (p) => readFileSync(join(ROOT, p), "utf8");
let failures = 0;
const check = (name, ok) => { if (!ok) { console.error("FAIL:", name); failures++; } else { console.log("ok:", name); } };

// Cargo.lock's versions of a crate.
const lock = read("Cargo.lock");
const versions = (name) => [...lock.matchAll(new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`, "g"))].map((m) => m[1]);
const atLeast = (v, min) => {
  const a = v.split(/[.+-]/).map(Number), b = min.split(".").map(Number);
  for (let i = 0; i < b.length; i++) { if ((a[i] || 0) !== b[i]) return (a[i] || 0) > b[i]; }
  return true;
};
for (const [name, min] of [["memmap2", "0.9.11"], ["anyhow", "1.0.103"]]) {
  const vs = versions(name);
  check(`${name} ≥ ${min} in Cargo.lock (${vs.join(", ") || "absent"})`, vs.every((v) => atLeast(v, min)));
}

const deny = read("deny.toml");
check("deny.toml fails on unsound crates", /^unsound = "all"$/m.test(deny));
check("deny.toml has no stale proc-macro-error2 ignore (RUSTSEC-2026-0173)", !deny.includes("RUSTSEC-2026-0173"));
const denyIgnores = [...deny.matchAll(/id = "(RUSTSEC-[0-9-]+)"/g)].map((m) => m[1]).sort();
const audit = read(".cargo/audit.toml");
const auditIgnores = [...audit.matchAll(/"(RUSTSEC-[0-9-]+)"/g)].map((m) => m[1]).sort();
check(`cargo-audit ignores what cargo-deny does (${auditIgnores} / ${denyIgnores})`, JSON.stringify(denyIgnores) === JSON.stringify(auditIgnores));

// ttf-parser only ever parses the built-in fonts: the statics
// include_bytes! embeds, as face() picks them for a font (its Regular for a
// character a styled face lacks), or BUILTIN_FACES lists them; nothing a
// program supplies.
const text = read("crates/rapidr-value/src/objects/text.rs");
const statics = new Set([...text.matchAll(/^(?:pub )?static ([A-Z_]+): &\[u8\] = include_bytes!\(/gm)].map((m) => m[1]));
const parses = [...text.matchAll(/ttf_parser::Face::parse\(([^,]+),/g)].map((m) => m[1].trim());
check(`ttf_parser parses only built-in faces (${parses})`, parses.length > 0 && parses.every((p) => statics.has(p) || ["want.data", "regular", "data"].includes(p)));
check("want and regular are face()'s", /let want = face\(/.test(text) && /let regular = face\([^)]*\)\.data;/.test(text));
check("data is BUILTIN_FACES'", (text.match(/ttf_parser::Face::parse\(data,/g) || []).length === (text.match(/for &\(data, _\) in BUILTIN_FACES\.iter\(\)/g) || []).length);
const faceFn = text.match(/fn face\(name: &str, styles: u8\) -> Face \{([\s\S]*?)\n\}/)?.[1] ?? "";
const named = [...new Set([...faceFn.replace(/\/\/.*$/gm, "").matchAll(/\b([A-Z][A-Z_]+)\b/g)].map((m) => m[1]))];
check(`face() picks only built-in fonts (${named})`, named.length > 0 && named.every((n) => statics.has(n)));
check("a Face is made only in face() and liberation()", (text.match(/Face \{ data:/g) || []).length === (faceFn.match(/Face \{ data:/g) || []).length + 1);
const listed = [...(text.match(/pub static BUILTIN_FACES[^=]*= \[([\s\S]*?)\n\];/)?.[1] ?? "").matchAll(/\(([A-Z_]+),/g)].map((m) => m[1]);
check(`BUILTIN_FACES lists only built-in fonts (${listed.length})`, listed.length > 0 && listed.every((n) => statics.has(n)));
const others = spawnSync("git", ["grep", "-l", "ttf_parser", "--", "*.rs"], { cwd: ROOT, encoding: "utf8" }).stdout.trim().split("\n").filter(Boolean);
check(`no other code uses ttf_parser (${others})`, others.length === 1 && others[0] === "crates/rapidr-value/src/objects/text.rs");

// The tools themselves, offline, when installed.
for (const [tool, args] of [["cargo-deny", ["deny", "--offline", "check", "advisories"]], ["cargo-audit", ["audit", "--no-fetch", "--deny", "warnings"]]]) {
  const has = spawnSync(tool, ["--version"], { encoding: "utf8" }).status === 0;
  if (!has) { console.log(`skip: ${tool} isn't installed`); continue; }
  const r = spawnSync("cargo", args, { cwd: ROOT, encoding: "utf8" });
  check(`cargo ${args.join(" ")} is clean`, r.status === 0);
  if (r.status !== 0) console.error((r.stdout + r.stderr).split("\n").filter((l) => /error|warning|RUSTSEC/.test(l)).slice(0, 8).join("\n"));
}

if (failures) { console.error(`\n${failures} check(s) failed`); process.exit(1); }
console.log("\nSEC-20 supply chain: all checks passed");
