// SEC-14 regression: RapidR Studio's web font bridge must not let the
// sandboxed program frame read arbitrary same-origin IDE files.
//
// ide/web/studio.js answers a frame's `{ __rapidr_font: { id, file } }` by
// fetching `runtime/fonts/<file>` on the IDE origin and handing the bytes
// back to the opaque-origin program frame. The program controls `file`
// (e.g. via RJavaScript.Eval, which runs in the frame). Without validation,
// `file = "../../studio.rrbc"` (or any same-origin path) is read and
// exfiltrated to the frame, defeating the opaque-origin isolation.
//
// The full browser proof-of-concept is in the security audit
// (docs/security-audit.md, SEC-14). This test pins the guard in the shipped
// source and checks the predicate against a traversal battery, with no
// browser dependency so it runs in tools/regress.sh.
//
//   node tests/security/web_font_bridge_traversal.mjs

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const studio = readFileSync(join(here, "..", "..", "ide", "web", "studio.js"), "utf8");

let failures = 0;
const check = (name, ok) => { if (!ok) { console.error("FAIL:", name); failures++; } else { console.log("ok:", name); } };

// 1. The guard is present in the shipped handler.
check("studio.js validates the font `file` before fetching", /__rapidr_font\b[\s\S]{0,900}?includes\("\.\."\)[\s\S]{0,200}?fetch\(/.test(studio));

// 2. The predicate the guard uses: a plain font file name only.
const allowed = (file) => typeof file === "string" && /^[\w.-]+$/.test(file) && !file.includes("..");

// Real font chunk names from tools/fonts.py must still pass.
for (const good of ["index.json", "NotoSans-Regular.otf", "NotoSansSC-Regular.000.otf", "Noto-COLRv1.ttf"]) {
  check(`allows ${good}`, allowed(good));
}
// Traversal / absolute / cross-origin attempts must be rejected.
for (const bad of ["../../studio.rrbc", "../studio.js", "..%2f..%2fsecret", "/etc/passwd", "a/b.otf", "", "..", "x/../../y"]) {
  check(`rejects ${JSON.stringify(bad)}`, !allowed(bad));
}

if (failures) { console.error(`\n${failures} check(s) failed`); process.exit(1); }
console.log("\nSEC-14 font-bridge traversal guard: all checks passed");
