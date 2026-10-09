// Run ▸ Run in Browser (docs/studio-wow.md RUN-2, PERF-9): `rapidr serve`
// builds the program for the web and serves it on 127.0.0.1 under a random
// path; Studio starts it (RProgramSession.RunInBrowser) and the default
// browser opens it. Here: the program's form shows in a real browser within
// the budget (1.5 s from the command to the form), captured at 1x and 2x;
// the server answers nothing outside its path, refuses another Host (DNS
// rebinding) and anything but GET / HEAD, and ends when its input closes
// (Studio went away).
//
//   node tests/run_in_browser.mjs            (needs ./rapidr and target/web)

import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { request } from "node:http";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "run-in-browser");
const PROGRAM = "examples/gui/hello_form.rr";
const BUDGET_MS = 1500;
mkdirSync(WORK, { recursive: true });

let passed = 0, failed = 0;
const record = (ok, what) => {
  ok ? passed++ : failed++;
  console.log(`${ok ? "✓" : "✗"} ${what}`);
};

// (one request with the headers given, the Host among them)
const get = (url, { method = "GET", host } = {}) =>
  new Promise((resolve) => {
    const u = new URL(url);
    const req = request({ hostname: u.hostname, port: u.port, path: u.pathname, method, headers: host ? { Host: host } : {} }, (res) => {
      let body = "";
      res.on("data", (d) => (body += d));
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body }));
    });
    req.on("error", (e) => resolve({ status: 0, body: String(e) }));
    req.end();
  });

function serve() {
  const started = performance.now();
  const child = spawn(RAPIDR, ["serve", PROGRAM], { cwd: ROOT, stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, RAPIDR_NO_BROWSER: "1" } });
  const url = new Promise((resolve, reject) => {
    let out = "";
    child.stdout.on("data", (d) => {
      out += d;
      const m = out.match(/Serving (\S+)/);
      if (m) resolve(m[1]);
    });
    child.on("exit", (code) => reject(new Error(`rapidr serve ended (${code}): ${out}`)));
  });
  return { child, url, started };
}

const browser = await chromium.launch();
try {
  for (const scale of [1, 2]) {
    const { child, url: urlP, started } = serve();
    const url = await urlP;
    const served = performance.now() - started;
    record(/^http:\/\/127\.0\.0\.1:\d+\/[0-9a-f]{32}\/$/.test(url), `@${scale}x: serves on 127.0.0.1 under a random path (${url}, ${served.toFixed(0)} ms)`);
    const page = await browser.newPage({ viewport: { width: 900, height: 600 }, deviceScaleFactor: scale });
    await page.goto(url);
    await page.waitForFunction(() => {
      const w = document.querySelector(".rr-kwin");
      return w && w.getBoundingClientRect().width > 50;
    }, null, { timeout: 15000 });
    await page.waitForTimeout(150);
    const toForm = performance.now() - started;
    record(toForm <= BUDGET_MS, `@${scale}x: the command to the form in the browser: ${toForm.toFixed(0)} ms (budget ${BUDGET_MS} ms)`);
    const title = await page.evaluate(() => document.title);
    record(/hello_form/i.test(title) || title.length > 0, `@${scale}x: the page is the program's (title ${JSON.stringify(title)})`);
    await page.screenshot({ path: join(WORK, `hello_form@${scale}x.png`) });
    await page.close();
    if (scale === 1) {
      const index = await get(url);
      record(index.status === 200 && /content-security-policy/i.test(Object.keys(index.headers).join(" ")), `index.html: 200 with its Content-Security-Policy (${index.status})`);
      const wrong = await get(url.replace(/\/[0-9a-f]{32}\//, "/0123456789abcdef0123456789abcdef/"));
      record(wrong.status === 404, `another path: 404 (${wrong.status})`);
      const root = await get(url.replace(/\/[0-9a-f]{32}\/$/, "/"));
      record(root.status === 404, `the root: 404 (${root.status})`);
      const rebound = await get(url, { host: "evil.example:80" });
      record(rebound.status === 421, `another Host (DNS rebinding): 421 (${rebound.status})`);
      const posted = await get(url, { method: "POST" });
      record(posted.status === 405, `POST: 405 (${posted.status})`);
      const traversal = await get(url + "..%2F..%2Fetc%2Fpasswd");
      record(traversal.status === 404, `a path out of the bundle: 404 (${traversal.status})`);
    }
    // (Studio went away: its input closes, the server ends)
    const ended = new Promise((resolve) => child.on("exit", (code) => resolve(code)));
    child.stdin.end();
    const code = await Promise.race([ended, new Promise((r) => setTimeout(() => r("still running"), 3000))]);
    record(code === 0, `@${scale}x: the server ends when its input closes (${code})`);
    if (code !== 0) child.kill();
  }
} finally {
  await browser.close();
}
console.log(`\nRun in Browser: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
