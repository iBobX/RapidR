// SEC-19 regression: PEEK / POKE / MEMCPY / VARPTR$ never reach memory
// RapidR doesn't own, and DLL calls stay off the web.
//
// RapidR's memory functions work on a model of the program's own memory
// (rapidr_value::memory: blocks for variables, arrays, TYPEs and streams;
// rapidr_value::console: the console's pages). An address outside them, or
// past a block's end, must stop the program with a run-time error that says
// so — never read or write the process, never crash. Each probe below is a
// program run by the interpreter (`rapidr build-bc` + `run-bc`); the same
// model serves native builds and the web (the conformance cases
// peek_bad_address and peek_poke_memory run there).
//
// The second half pins the web's side: its hosts answer a DLL call with
// the error and the web runtime links no library loader.
//
//   node tests/security/peek_poke_unowned_memory.mjs     (after ./build.sh)

import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const EXE = process.platform === "win32" ? ".exe" : "";
const RAPIDR = resolve(ROOT, process.env.RAPIDR_BIN || `rapidr${EXE}`);
if (!existsSync(RAPIDR)) {
  console.error(`rapidr binary not found at ${RAPIDR} (run ./build.sh or set RAPIDR_BIN)`);
  process.exit(2);
}

let failures = 0;
const check = (name, ok, detail = "") => {
  if (ok) console.log("ok:", name);
  else { console.error("FAIL:", name, detail); failures++; }
};

const dir = mkdtempSync(join(tmpdir(), "rapidr-sec19-"));
const env = { ...process.env, RAPIDR_PRINT_TO: join(dir, "prints"), RAPIDR_REGISTRY: join(dir, "reg.reg") };

// Runs `src`; it must print "before", then stop with a run-time error
// containing `want` (exit code 1, not a signal or an access violation).
function refused(name, src, want) {
  const bas = join(dir, `${name}.bas`), rrbc = join(dir, `${name}.rrbc`);
  writeFileSync(bas, `PRINT "before"\n${src}\nPRINT "not reached"\n`);
  const c = spawnSync(RAPIDR, ["build-bc", bas, "-o", rrbc], { encoding: "utf8", env });
  if (c.status !== 0) return check(name, false, `(didn't compile: ${c.stderr.trim()})`);
  const r = spawnSync(RAPIDR, ["run-bc", rrbc], { encoding: "utf8", env, timeout: 30_000 });
  const err = r.stderr || "";
  check(
    `${name}: refused with a run-time error`,
    r.status === 1 && r.signal === null && r.stdout.startsWith("before") && !r.stdout.includes("not reached") && err.includes("run-time error") && err.includes(want),
    `(status ${r.status}, signal ${r.signal}, stdout ${JSON.stringify(r.stdout)}, stderr ${JSON.stringify(err.trim())})`,
  );
}

// Addresses that are no memory of the program: real process memory on
// Windows (KUSER_SHARED_DATA, the low 2 GB, a 64-bit pointer), just past
// the console's pages, negative.
refused("peek_kuser_shared", "x = PEEK(&H7FFE0000)", "isn't memory of this program");
refused("peek_past_pages", "x = PEEK(4000)", "isn't memory of this program");
refused("peek_negative", "x = PEEK(-1)", "isn't memory of this program");
refused("poke_top_of_2gb", "POKE &H7FFFFFF0, 1", "isn't memory of this program");
refused("poke_huge", "POKE 1E18, 1", "isn't memory of this program");
// Inside the address range of a block but past what it holds.
refused("peek_past_long", "DIM n AS LONG\nx = PEEK(VARPTR(n) + 4)", "past the end");
refused("poke_past_string", "s$ = \"Hi\"\nPOKE VARPTR(s$) + 10, 65", "past the end");
refused("poke_between_blocks", "DIM n AS LONG\nPOKE VARPTR(n) + 6000, 1", "isn't memory of this program");
refused("peek_past_array", "DIM a(3) AS INTEGER\nx = PEEK(VARPTR(a(0)) + 16)", "past the end");
// A console page that doesn't exist.
refused("peek_page_8", "x = PEEK(#8, 0)", "page");
// The other memory functions follow the same rules.
refused("memcpy_to_unowned", "DIM n AS LONG\nMEMCPY 12345678, VARPTR(n), 4", "isn't memory of this program");
refused("memcpy_overrun", "DIM n AS LONG\nDIM m AS LONG\nMEMCPY VARPTR(m), VARPTR(n), 64", "past the end");
refused("memset_unowned", "MEMSET 99999999, 0, 16", "isn't memory of this program");
refused("varptr_string_unowned", "x$ = VARPTR$(305419896)", "isn't memory of this program");
// Hardware ports: always a run-time error.
refused("inp_port", "x = INP(&H378)", "hardware port");
// A DLL call where there are no Windows DLLs: the error names the function.
if (process.platform !== "win32") {
  refused("dll_off_windows", "DECLARE FUNCTION GetTickCount LIB \"kernel32\" () AS LONG\nt = GetTickCount", "'GetTickCount' is a Windows function (kernel32)");
  refused("third_party_dll_off_windows", "DECLARE FUNCTION Ping LIB \"evil.dll\" (BYVAL n AS LONG) AS LONG\nt = Ping(1)", "'Ping' is a function of evil.dll, a Windows DLL");
}

// The web: no library loader, and both web hosts refuse a DLL call.
const read = (p) => readFileSync(join(ROOT, p), "utf8");
check("the web VM host answers __dll_call with the error", /key == "__dll_call"[\s\S]{0,300}?Err\(rapidr_value::dll::needs_windows_error\([^;]{0,40}?, true\)\)/.test(read("interpreter/rapidr-vm-host-web/src/lib.rs")));
check("the web runtime answers __dll_call with the error", /needs_windows_error\([^;]{0,40}?, true\)/.test(read("crates/rapidr-runtime-web/src/builtins.rs")));
for (const toml of ["crates/rapidr-runtime-web/Cargo.toml", "interpreter/rapidr-vm-host-web/Cargo.toml"]) {
  check(`${toml} links no library loader`, !/libloading/.test(read(toml)));
}
// The VM itself has no unsafe code at all.
check("the VM forbids unsafe code", /#!\[forbid\(unsafe_code\)\]/.test(read("interpreter/rapidr-vm/src/lib.rs")));

rmSync(dir, { recursive: true, force: true });
if (failures) { console.error(`\n${failures} check(s) failed`); process.exit(1); }
console.log("\nSEC-19 PEEK / POKE / DLL boundaries: all checks passed");
