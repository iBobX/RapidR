// SEC-17 and SEC-12 regressions (docs/security-audit.md), without a browser.
//
// SEC-17: the legacy HTML / Monaco IDE (web-ide/) was the shipped web
// artifact (tools/release/web.sh packaged it) and its scripts were compiled
// into every program's bundle (include_str! of web-ide/bundle_console.js and
// ansi_screen.js), with its wildcard-CORS .htaccess and weak escaping on the
// shipping path. Now the release ships RapidR Studio's web build, and a
// bundle carries only rapidr-webbundle's own files.
//
// SEC-12: an RWEBVIEW's frame was sandboxed with `allow-scripts
// allow-same-origin`, so its Html ran with the program's own origin. Now the
// default leaves allow-same-origin out (a program may opt in through its
// Sandbox property) and the Html runs in the RWEBVIEW frame file, which
// takes it only from its parent window. (The browser proof:
// tests/web_bundle_csp.mjs, tests/web_overlays.mjs.)
//
//   node tests/security/web_shipping.mjs

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const read = (p) => readFileSync(join(ROOT, p), "utf8");
let failures = 0;
const check = (name, ok) => { if (!ok) { console.error("FAIL:", name); failures++; } else { console.log("ok:", name); } };

// --- SEC-17: what ships.
const web = read("tools/release/web.sh");
check("tools/release/web.sh doesn't default to web-ide/", !/SITE="\$\{1:-web-ide\}"/.test(web) && !/git ls-files[^\n]*web-ide/.test(web));
check("tools/release/web.sh ships RapidR Studio's web build", /tools\/build_studio_web\.sh/.test(web) && /target\/studio-web/.test(web));
check("tools/release/web.sh refuses web-ide/", /web-ide\|web-ide\/\*/.test(web));
const bundle = read("interpreter/rapidr-webbundle/src/lib.rs");
check("rapidr-webbundle embeds nothing from web-ide/", !/include_str!\([^)]*web-ide/.test(bundle) && !/include_bytes!\([^)]*web-ide/.test(bundle));
for (const f of ["bundle_console.js", "ansi_screen.js", "loader.js", "start.js", "rapidr-webview.html"]) {
  check(`rapidr-webbundle has its own ${f}`, existsSync(join(ROOT, "interpreter/rapidr-webbundle/web", f)));
}
const sbom = read("tools/release/sbom.py");
check("the SBOM doesn't list the unshipped Monaco", !/monaco-editor/.test(sbom));
for (const f of ["ide/web/.htaccess", "ide/web/_headers"]) {
  const t = read(f);
  check(`${f}: no wildcard CORS`, !/Access-Control-Allow-Origin/i.test(t));
  check(`${f}: no framing by other sites`, /frame-ancestors 'self'/.test(t));
}
const studioBuild = read("tools/build_studio_web.sh");
check("Studio's web build ships the RWEBVIEW frame and its hosts' headers", /rapidr-webview\.html/.test(studioBuild) && /_headers/.test(studioBuild));

// --- SEC-12: RWEBVIEW's frame.
const overlay = read("crates/rapidr-runtime-web/src/overlay_web.rs");
const def = overlay.match(/pub const WEBVIEW_SANDBOX: &str = "([^"]*)"/)?.[1];
check(`RWEBVIEW's default sandbox has no allow-same-origin (${def})`, def !== undefined && /allow-scripts/.test(def) && !/allow-same-origin/.test(def));
check("no RWEBVIEW frame is made with allow-same-origin", !/set_attribute\("sandbox",\s*"[^"]*allow-same-origin/.test(overlay));
check("RWEBVIEW's Html isn't an srcdoc when scripts may run (it would inherit the page's policy)", /fn show_html[\s\S]*?if !scripts \{[\s\S]*?set_srcdoc[\s\S]*?return;[\s\S]*?set_src\(&webview_frame_url\(\)\)/.test(overlay));
check("the frame is given its Html only while it shows the frame file", /if frame\.src\(\) != webview_frame_url\(\)/.test(overlay));
const frame = read("interpreter/rapidr-webbundle/web/rapidr-webview.html");
check("the frame file hears only its parent window", /e\.source !== parent/.test(frame) && !/innerHTML/.test(frame));
check("Studio's program frame names the RWEBVIEW frame file", /RAPIDR_WEBVIEW_FRAME/.test(read("ide/web/run.html")));

if (failures) { console.error(`\n${failures} check(s) failed`); process.exit(1); }
console.log("\nSEC-17 / SEC-12 web shipping: all checks passed");
