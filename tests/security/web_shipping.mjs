// SEC-17 and SEC-12 regressions (docs/security-audit.md), without a browser.
//
// SEC-17: the legacy HTML / Monaco IDE was the shipped web artifact
// (tools/release/web.sh packaged it) and its scripts were compiled into
// every program's bundle (include_str! of its bundle_console.js and
// ansi_screen.js), with its wildcard-CORS .htaccess and weak escaping on the
// shipping path. Now the release ships RapidR Studio's web build, a bundle
// carries only rapidr-webbundle's own files, and the old IDE is deleted.
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
check("tools/release/web.sh ships RapidR Studio's web build (by default; nothing taken from git's file list)", /SITE=target\/studio-web/.test(web) && /tools\/build_studio_web\.sh/.test(web) && !/ls-files/.test(web));
const bundle = read("interpreter/rapidr-webbundle/src/lib.rs");
const embeds = [...bundle.matchAll(/include_(?:str|bytes)!\("([^"]*)"\)/g)].map((m) => m[1]);
check(`rapidr-webbundle embeds only its own web/ files (${embeds.join(", ")})`, embeds.length > 0 && embeds.every((p) => p.startsWith("../web/") && !p.slice(7).includes("..")));
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

// --- SEC-34 (docs/security/audit-2026-11-05-web.md): who may frame the
// frames and send them what they run. (The browser proof:
// tests/security/webview_frame_xss.mjs.)
check("the frame file takes its Html only from its own site, or opaque from opaque", /e\.origin !== location\.origin && !\(String\(self\.origin\) === "null" && e\.origin === "null"\)\) return;/.test(frame));
check("the frame file has no <meta> policy (document.write would bind the RWEBVIEW's Html to it)", !/http-equiv/i.test(frame));
check("a bundle's hosts' files give the frame file its own policy", /pub const WEBVIEW_FRAME_POLICY: &str = "frame-ancestors 'self'";/.test(bundle) && /headers\.push_str\(&format!\("\/\{WEBVIEW_FRAME_FILE\}\\n  Content-Security-Policy: \{WEBVIEW_FRAME_POLICY\}/.test(bundle) && /<Files \\"\{WEBVIEW_FRAME_FILE\}\\">/.test(bundle));
check("rapidr serve gives the frame file its own policy, not the page's", /fn policy_for[\s\S]*?WEBVIEW_FRAME_FILE[\s\S]*?WEBVIEW_FRAME_POLICY/.test(read("crates/rapidr-cli/src/serve.rs")));
const studioHeaders = read("ide/web/_headers");
check("Studio's _headers: run.html framed by Studio only", /^\/run\.html\n  Content-Security-Policy: frame-ancestors 'self'$/m.test(studioHeaders));
check("Studio's _headers: its RWEBVIEW frame always sandboxed, never same-origin", /^\/runtime\/rapidr-webview\.html\n  Content-Security-Policy: sandbox allow-scripts[^\n]*$/m.test(studioHeaders) && !/allow-same-origin/.test(studioHeaders));
const studioHt = read("ide/web/.htaccess");
check("Studio's .htaccess says the same", /<Files "run\.html">\s*Header always set Content-Security-Policy "frame-ancestors 'self'"/.test(studioHt) && /<Files "rapidr-webview\.html">\s*Header always set Content-Security-Policy "sandbox allow-scripts/.test(studioHt));
check("Studio's web build refuses headers without the frames' rules", /runtime\/rapidr-webview\.html\$/.test(studioBuild) && /SEC-34/.test(studioBuild));
check("run.html boots only from Studio's own page", /e\.source !== parent \|\| e\.origin !== location\.origin \|\| !e\.data\?\.__rapidr_boot/.test(read("ide/web/run.html")));

// --- SEC-35: a link's ?do= (the browser proof: tests/web_studio_safety.mjs).
const studioJs = read("ide/web/studio.js");
check("Studio-web passes ?do= on only through linkSteps", /const steps = params\.get\("do"\) \? linkSteps\(params\.get\("do"\)\) : "";\s*if \(steps\) args\.push\("--do", steps\);/.test(studioJs) && (studioJs.match(/"--do"/g) || []).length === 1);
check("linkSteps: everything only under the tests' gate", /function linkSteps\(list\) \{\s*if \(window\.RAPIDR_STUDIO_TEST \|\| window\.RAPIDR_STUDIO_ALLOW_DO\) return list;/.test(studioJs));
const linkStep = studioJs.match(/const LINK_STEP = (\/.*\/);/)?.[1];
const allowedStep = linkStep ? new Function(`return ${linkStep}`)() : null;
const linkOk = (list) => list.split(",").every((s) => allowedStep.test(s.trim()));
check(`a link's steps: only wait and a page of the manual (${linkStep})`, !!allowedStep
  && ["wait", "manual:getting-started.md", "wait,manual:debugging.md", "manual:language#loops"].every(linkOk)
  && ["run.start", "type:PRINT 1", "key:Ctrl+S", 'code:"a"=>"b"', "view.settings", "manual:../x", "manual:a,run.start", "wait,file.saveAll", "ai:on", "manual:", "help:x"].every((l) => !linkOk(l)));

// --- SEC-36: the program frame's `held`.
check("held is taken only through setHeld, never straight from the frame", /if \(typeof w\.held === "boolean"\) setHeld\(w\.held\);/.test(studioJs) && !/run\.held = w\.held/.test(studioJs));
check("setHeld honours held only for a press Studio sees (focus + user activation)", /function pressSeen\(\) \{[\s\S]*?document\.activeElement !== run\.frame[\s\S]*?navigator\.userActivation[\s\S]*?\}/.test(studioJs) && /run\.held = held && pressSeen\(\);/.test(studioJs));
check("held is capped (HELD_MS), re-armed only while the windows move, dropped when the frame loses focus", /run\.heldTimer = setTimeout\(\(\) => \{ run\.held = false;/.test(studioJs)
  && Number(studioJs.match(/const HELD_MS = (\d+);/)?.[1]) <= 6000
  && /if \(run\.held && pressSeen\(\) && JSON\.stringify\(run\.rects\) !== before\) armHeld\(\);/.test(studioJs)
  && /if \(run\.held && document\.activeElement !== run\.frame\) setHeld\(false\);/.test(studioJs));

if (failures) { console.error(`\n${failures} check(s) failed`); process.exit(1); }
console.log("\nSEC-17 / SEC-12 / SEC-34 / SEC-35 / SEC-36 web shipping: all checks passed");
