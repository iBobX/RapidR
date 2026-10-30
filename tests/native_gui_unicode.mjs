// Text that is not Latin-1 survives end to end, on the desktop host
// (Robert's Solar System 3D, 2.121.3: its Chinese showed as `??`).
// tests/fixtures/unicode_text/main.rr built as a plain executable, both ways
// when asked (`--native` adds the Rust backend), run headless
// (RAPIDR_CAPTURE) with the test hooks' dump:
//
//  1. Chinese: a literal caption, a TR$ caption and a button's TR$ text (with
//     its & accelerator) from a UTF-8 lang/zh-Hans.po are exactly the Chinese
//     text — the dump compares the property, not a picture.
//  2. A file written with WriteStr(s, LEN(s)) (Studio's Apply wrote its
//     translation files so) reads back as it was: accents, Chinese, emoji.
//  3. A canvas's TextOut draws Chinese with the font a label uses: the first
//     ideograph's cell differs from a `?` drawn where it stands (the program
//     counts the canvas's own pixels) — when painted and when drawn first,
//     before any window exists.
//  4. Spanish from lang/es.po as an earlier Studio wrote it (Windows-1252, a
//     byte an accent): `·`, `í`, `¿`, not U+FFFD.
//
// Usage (repo root, after building ./rapidr):
//   node tests/native_gui_unicode.mjs [--native]

import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
// (SEC-24: a release build's test hooks need the test-mode token: tests/test_mode.mjs)
import "./test_mode.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, process.platform === "win32" ? "rapidr.exe" : "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/native_gui_unicode");
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
const DUMP = "Literal.caption,Translated.caption,Pause.caption,Roundtrip.caption,Report.caption,Early.caption";

rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });
for (const kind of kinds) {
  const src = join(WORK, kind, "src");
  const out = join(WORK, kind, "out");
  const cwd = join(WORK, kind, "cwd");
  cpSync(join(ROOT, "tests/fixtures/unicode_text"), src, { recursive: true });
  mkdirSync(cwd, { recursive: true });
  const build = spawnSync(RAPIDR, ["build", join(src, "main.rr"), out, "--no-bundle", ...(kind === "native" ? ["--debug"] : ["--interp"])], { encoding: "utf8", env: env({}), timeout: 1800000 });
  ok(build.status === 0 && /lang\/es\.po/.test(build.stdout) && /lang\/zh-Hans\.po/.test(build.stdout), `${kind}: built with lang/es.po and lang/zh-Hans.po${build.status ? `\n${build.stdout}${build.stderr}` : ""}`);
  if (build.status !== 0) continue;
  const start = (language) => {
    const r = spawnSync(join(out, `main${EXE}`), [language], { cwd, encoding: "utf8", input: "", timeout: 120000,
      env: env({ RAPIDR_CAPTURE: join(WORK, kind, `shot-${language}`), RAPIDR_TEST_DUMP: DUMP }) });
    const got = {};
    for (const line of `${r.stdout}${r.stderr}`.split("\n")) {
      const m = /^([A-Za-z]+\.caption)=(.*)$/.exec(line);
      if (m) got[m[1]] = m[2];
    }
    return got;
  };

  const zh = start("zh-Hans");
  ok(zh["Literal.caption"] === "中文(简体) 日本語 한국어", `${kind}: a literal Chinese caption is the Chinese text (${zh["Literal.caption"]})`);
  ok(zh["Translated.caption"] === "日期: 2027-02-24", `${kind}: a TR$ caption from lang/zh-Hans.po is the Chinese text (${zh["Translated.caption"]})`);
  ok(zh["Pause.caption"] === "暂停(&P)", `${kind}: a button's TR$ text keeps its accelerator (${zh["Pause.caption"]})`);
  ok(zh["Roundtrip.caption"] === "same 26", `${kind}: WriteStr(s, LEN(s)) then ReadAllText gives back all of the text (${zh["Roundtrip.caption"]})`);
  ok(/ ink yes differs yes$/.test(zh["Report.caption"] || ""), `${kind}: a canvas draws Chinese with a real font, not as \`?\` (${zh["Report.caption"]})`);
  ok(/ ink yes differs yes$/.test(zh["Early.caption"] || ""), `${kind}: and so does a first draw, made before any window exists (${zh["Early.caption"]})`);
  ok(!Object.values(zh).some((v) => /\?\?|�/.test(v)), `${kind}: no \`??\` and no U+FFFD anywhere`);

  const es = start("es");
  ok(es["Translated.caption"] === "Fecha · día: 2027-02-24", `${kind}: Spanish from a Windows-1252 lang/es.po (${es["Translated.caption"]})`);
  ok(es["Pause.caption"] === "&Pausa ¿si?", `${kind}: ¿ from the same file (${es["Pause.caption"]})`);
  ok(!Object.values(es).some((v) => /�/.test(v)), `${kind}: no U+FFFD in the Spanish`);
}
console.log(failed ? `\n${failed} FAILED` : "\nall passed");
process.exit(failed ? 1 : 0);
