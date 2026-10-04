// node tests/_webprobe.mjs file.bas comp.prop,…  — run a program in the web IDE, print props after 2s
import { chromium } from "playwright";
import { readFileSync } from "node:fs";
const [file, props = ""] = process.argv.slice(2);
const browser = await chromium.launch();
const page = await browser.newPage();
page.on("console", (m) => { if (/host\.js/.test(m.location()?.url || "") && !m.text().startsWith("------")) console.log("OUT", m.type(), m.text().slice(0, 200)); });
await page.goto(`http://localhost:8765/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate((src) => { window.RapidR.state.project.rawSource = src; window.RapidR.runCommand("run.start"); }, readFileSync(file, "latin1"));
await page.waitForTimeout(2500);
const frame = page.frames().find((f) => f.url().includes("preview.html"));
console.log(await frame.evaluate((props) => props.split(",").filter(Boolean).map((p) => { const [c, k] = p.split("."); const el = document.getElementById("rr-" + c); const cs = el && getComputedStyle(el); return `${p}=${window.__rapidr_rt.rapidr_get_prop(c, k)} (el ${el?.offsetWidth}x${el?.offsetHeight} style=${el?.getAttribute("style")} min=${cs?.minWidth} pad=${cs?.padding})`; }).join("\n"), props));
await browser.close();
