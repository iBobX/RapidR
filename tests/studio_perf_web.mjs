// RapidR Studio's start on the web (docs/ide-plan.md §6.2: ≤ 2.5 s on a
// first visit at 50 Mbit/s, ≤ 1.2 s cached): from navigation to Studio's
// first frame drawn (the shell running, then two animation frames), and
// with a 50-file project. Serve target/studio-web as for studio_shell.mjs.
//
//   node tests/studio_perf_web.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473";
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });

async function load(page, query) {
  await page.goto(`${URL_BASE}/index.html?${query}`, { waitUntil: "load" });
  await page.waitForFunction(() => window.RAPIDR_STUDIO_STARTED !== undefined, null, { timeout: 60000, polling: 10 });
  return page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(performance.now())))));
}

const page = await context.newPage();
const cdp = await context.newCDPSession(page);
await cdp.send("Network.enable");
// (a first visit at 50 Mbit/s, 20 ms latency)
await cdp.send("Network.emulateNetworkConditions", { offline: false, latency: 20, downloadThroughput: (50 * 1024 * 1024) / 8, uploadThroughput: (10 * 1024 * 1024) / 8 });
await cdp.send("Network.setCacheDisabled", { cacheDisabled: true });
const first = await load(page, "fresh&theme=rapidr-light");
await cdp.send("Network.setCacheDisabled", { cacheDisabled: false });
// (cached: the files from the browser's cache, no throttle)
await cdp.send("Network.emulateNetworkConditions", { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
await load(page, "fresh&theme=rapidr-light");
const cached = [];
for (let i = 0; i < 5; i++) cached.push(await load(page, "fresh&theme=rapidr-light"));
const opened = [];
for (let i = 0; i < 3; i++) opened.push(await load(page, "fresh&theme=rapidr-light&open=examples/gui/hello_form.rr"));
const med = (a) => [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)];
console.log(`first visit at 50 Mbit/s: ${Math.round(first)} ms (target ≤ 2500)`);
console.log(`cached: ${cached.map(Math.round).join(", ")} ms, median ${Math.round(med(cached))} (target ≤ 1200)`);
console.log(`cached, a project opened: median ${Math.round(med(opened))} ms`);
await browser.close();
