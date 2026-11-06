// SEC-34 regression (docs/security/audit-2026-11-05-web.md): rapidr-webview.html,
// the frame an RWEBVIEW's Html runs in, ships in every web bundle and in
// RapidR Studio's web build (runtime/rapidr-webview.html). It document.write()s
// the HTML its parent window sends. Before the fix it heard any parent, and
// no header kept other sites from framing it, so a page of another site
// could frame it and have its own markup run in the program's (or Studio's)
// origin, with that origin's storage.
//
// The fix, checked here in a real browser against the REAL shipped files:
//   - the frame file takes the HTML only from a parent of its own site, or
//     — when it is itself at an opaque origin — from an opaque parent (Studio's
//     program frame), where what runs can't reach the site;
//   - a bundle's hosts' headers give the file frame-ancestors 'self'
//     (rapidr_webbundle::WEBVIEW_FRAME_POLICY), Studio's _headers sandbox it
//     (ide/web/_headers: its frame-ancestors can't name Studio's opaque
//     run.html);
//   - the RWEBVIEW's own Html still runs: in a bundle (sandboxed, and with
//     the program's allow-same-origin) and in Studio's program frame.
//
// Each case is run as a static host serves it (no headers) and as a host
// that reads the headers. Two loopback servers: the "site" and another
// origin (a different port). Exit 0: not vulnerable and the RWEBVIEW works.
//
//   node tests/security/webview_frame_xss.mjs
// Needs the repo's tests/node_modules (Playwright + chromium). Loopback only.

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const read = (p) => readFileSync(join(repo, p), "utf8");
const frameHtml = read("interpreter/rapidr-webbundle/web/rapidr-webview.html");
const bundlePolicy = read("interpreter/rapidr-webbundle/src/lib.rs").match(/pub const WEBVIEW_FRAME_POLICY: &str = "([^"]*)"/)?.[1];
const studioHeaders = read("ide/web/_headers");

let failures = 0;
const check = (name, ok) => { if (!ok) { console.error("FAIL:", name); failures++; } else { console.log("ok:", name); } };
check(`the bundle's frame file has a policy of its own (${bundlePolicy})`, /frame-ancestors 'self'/.test(bundlePolicy || ""));

// Netlify / Cloudflare Pages `_headers`: a path line, then its indented
// headers; `/*` matches everything under it.
function headersFor(text, path) {
  const out = {};
  let match = false;
  for (const line of text.split("\n")) {
    if (!line.trim() || line.trim().startsWith("#")) continue;
    if (!/^\s/.test(line)) {
      const p = line.trim();
      match = p === path || (p.endsWith("/*") && path.startsWith(p.slice(0, -1)));
    } else if (match) {
      const k = line.indexOf(":");
      out[line.slice(0, k).trim()] = line.slice(k + 1).trim();
    }
  }
  return out;
}
const studioFrameHeaders = headersFor(studioHeaders, "/runtime/rapidr-webview.html");
check(`Studio's _headers sandbox its RWEBVIEW frame (${JSON.stringify(studioFrameHeaders)})`, /\bsandbox\b/.test(studioFrameHeaders["Content-Security-Policy"] || "") && !/allow-same-origin/.test(studioFrameHeaders["Content-Security-Policy"] || ""));
check("Studio's _headers keep other sites from framing run.html", /frame-ancestors 'self'/.test(headersFor(studioHeaders, "/run.html")["Content-Security-Policy"] || ""));

// What runs in the frame reports where it really runs (self.origin: the
// document's origin, "null" when opaque) and whether it could use that
// origin's storage, to the site's server, and to its parent window.
const REPORT = "(function(){var s='no';try{localStorage.setItem('rr_probe','1');s='yes'}catch(e){}" +
  "var q='origin='+encodeURIComponent(String(self.origin))+'&storage='+s;" +
  "try{fetch(location.protocol+'//'+location.host+'/REPORT?'+q)}catch(e){};parent.postMessage({rr_report:q},'*')})()";
const PAYLOAD = `<img src=x onerror="${REPORT}">`;

let headers = "none";  // "none" (a static host) or "real" (a host that reads them)
const reports = [];
const site = createServer((req, res) => {
  const url = req.url.split("?")[0];
  const h = { "Content-Type": "text/html; charset=utf-8" };
  if (url === "/REPORT") {
    reports.push(req.url);
    res.writeHead(200, { "Content-Type": "text/plain" });
    return res.end("ok");
  }
  if (url === "/rapidr-webview.html" || url === "/runtime/rapidr-webview.html") {
    if (headers === "real") {
      if (url === "/rapidr-webview.html") h["Content-Security-Policy"] = bundlePolicy;
      else Object.assign(h, studioFrameHeaders);
    }
    res.writeHead(200, h);
    return res.end(frameHtml);
  }
  // The site's own pages, as overlay_web.rs drives the frame: an RWEBVIEW
  // (sandboxed: ?sandbox=) loads the file and is sent its Html on load.
  // (a report is passed up to the top page, which keeps it)
  const up = `addEventListener("message", (e) => { if (e.data && e.data.rr_report && parent !== window) parent.postMessage(e.data, "*"); });`;
  const rwebview = (src, sandbox) => `<iframe id="f" sandbox="${sandbox}" src="${src}"></iframe><script>
    ${up}
    const f = document.getElementById("f");
    f.addEventListener("load", () => f.contentWindow.postMessage({ __rapidr_webview_html: ${JSON.stringify(PAYLOAD)} }, "*"));
  </script>`;
  const q = new URL(req.url, "http://x").searchParams;
  const pages = {
    // a bundle's page: the RWEBVIEW beside it
    "/bundle.html": () => rwebview("/rapidr-webview.html", q.get("sandbox") ?? "allow-scripts allow-forms allow-popups allow-modals allow-downloads"),
    // Studio's run.html: in Studio's page, sandboxed (opaque); its RWEBVIEW runtime/rapidr-webview.html
    "/run.html": () => rwebview("/runtime/rapidr-webview.html", "allow-scripts allow-forms allow-popups allow-modals allow-downloads"),
    "/studio.html": () => `<iframe sandbox="allow-scripts allow-modals allow-downloads" src="/run.html"></iframe><script>${up}</script>`,
  };
  if (!pages[url]) { res.writeHead(404); return res.end("not found"); }
  res.writeHead(200, h);
  res.end(`<!doctype html><html><body>${pages[url]()}</body></html>`);
});
await new Promise((r) => site.listen(0, "127.0.0.1", r));
const siteOrigin = `http://127.0.0.1:${site.address().port}`;

// Another site: frames the site's frame file and sends it markup (as the
// audit's proof did), directly or from a page of no origin (a data: URL).
const other = createServer((req, res) => {
  const target = `${siteOrigin}${new URL(req.url, "http://x").searchParams.get("file")}`;
  const framer = `<iframe id="f" src="${target}"></iframe><script>
    const f = document.getElementById("f");
    f.addEventListener("load", () => f.contentWindow.postMessage({ __rapidr_webview_html: ${JSON.stringify(PAYLOAD)} }, "*"));
  </script>`;
  const body = req.url.startsWith("/data")
    ? `<iframe src="data:text/html;base64,${Buffer.from(framer).toString("base64")}"></iframe>`
    : framer;
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(`<!doctype html><html><body>${body}</body></html>`);
});
await new Promise((r) => other.listen(0, "127.0.0.1", r));
const otherOrigin = `http://127.0.0.1:${other.address().port}`;

const browser = await chromium.launch();
// What a page's frames reported: to the site's server (with the origin they
// ran in) and back to the top page.
async function visit(url) {
  reports.length = 0;
  const page = await browser.newPage();
  const back = [];
  await page.exposeFunction("__rr_back", (q) => back.push(q));
  await page.addInitScript(() => addEventListener("message", (e) => { if (e.data && e.data.rr_report && window.top === window) window.__rr_back(e.data.rr_report); }));
  await page.goto(url);
  await page.waitForTimeout(1200);
  await page.close();
  return { server: reports.slice(), back };
}
const ranAsSite = (r) => r.server.some((u) => u.includes(`origin=${encodeURIComponent(siteOrigin)}`) || u.includes("storage=yes"));

for (const h of ["none", "real"]) {
  headers = h;
  const how = h === "none" ? "a static host" : "a host that reads the headers";
  // The attacks: nothing of the other site's runs as the site, on any host.
  for (const file of ["/rapidr-webview.html", "/runtime/rapidr-webview.html"]) {
    const direct = await visit(`${otherOrigin}/framed?file=${encodeURIComponent(file)}`);
    check(`${how}: another site framing ${file} runs nothing as the site (${JSON.stringify(direct.server)})`, !ranAsSite(direct));
    const opaque = await visit(`${otherOrigin}/data?file=${encodeURIComponent(file)}`);
    check(`${how}: a page of no origin framing ${file} runs nothing as the site (${JSON.stringify(opaque.server)})`, !ranAsSite(opaque));
  }
  // The RWEBVIEW's own Html still runs.
  const sandboxed = await visit(`${siteOrigin}/bundle.html`);
  check(`${how}: a bundle's RWEBVIEW runs its Html, at an opaque origin (${JSON.stringify(sandboxed.back)})`, sandboxed.back.some((q) => q === "origin=null&storage=no"));
  const sameOrigin = await visit(`${siteOrigin}/bundle.html?sandbox=${encodeURIComponent("allow-scripts allow-same-origin")}`);
  check(`${how}: an RWEBVIEW whose Sandbox allows same-origin runs its Html as the program (${JSON.stringify(sameOrigin.back)})`, sameOrigin.back.some((q) => q === `origin=${encodeURIComponent(siteOrigin)}&storage=yes`));
  const studio = await visit(`${siteOrigin}/studio.html`);
  check(`${how}: an RWEBVIEW in Studio's program frame runs its Html, at an opaque origin (${JSON.stringify(studio.back)})`, studio.back.some((q) => q === "origin=null&storage=no"));
}

await browser.close();
site.close();
other.close();
if (failures) {
  console.error(`\nSEC-34: ${failures} check(s) failed: VULNERABLE, or the RWEBVIEW's Html no longer runs`);
  process.exit(1);
}
console.log("\nSEC-34 (rapidr-webview.html framing): not vulnerable; the RWEBVIEW's Html runs in a bundle and in Studio");
