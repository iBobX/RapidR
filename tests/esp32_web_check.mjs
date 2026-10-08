// esp32_web_check.mjs: tools/esp32_check.rr in Chrome against a real ESP32
// on Web Serial (tools/esp32_check.sh runs it; by hand, the board plugged in).
//
// Web Serial opens only a port its user picked in the browser's chooser. A
// test can't click that chooser, so this one starts Chrome on a fresh,
// throwaway profile that already holds the permission a pick would have
// stored (the port's USB vendor / product IDs, serial number, and on macOS
// its driver, as Chrome keys it) for this page's origin only. The page then
// finds the board with getPorts() — as it would on a user's second visit —
// and the program opens it as COM1.
//
// Like the desktop check it NEVER writes to the board: it lists the ports,
// opens the board at 115200, resets it through DTR / RTS (IO0 high) and
// prints the boot log; then it closes the port.
//
// Needs: Google Chrome (Playwright's channel "chrome": Playwright's own
// Chromium shell has no Web Serial), the web runtime (target/web:
// tools/build_web_artifacts.sh) and the repo served on RAPIDR_URL (not 8765).
//   RAPIDR_URL=http://localhost:8847 node tests/esp32_web_check.mjs /dev/cu.usbserial-0001
// ESP32_MONITOR=shot.png runs examples/iot/esp32_monitor.rr instead (Connect,
// Reset) and captures the page (RAPIDR_DPR=2 at 2×).

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8847";
const device = process.argv[2] || "";
const seconds = process.argv[3] || "4";

/// The USB serial port `device` (or the first) as Chrome keys a granted
/// port on macOS: what IOKit says of it (ioreg).
function macPort(device) {
  const lines = execFileSync("ioreg", ["-r", "-c", "IOUSBHostDevice", "-l", "-w0"], { encoding: "utf8", maxBuffer: 1 << 26 }).split("\n");
  const prop = (l, k) => { const m = new RegExp(`"${k}" = "?([^"]*)"?$`).exec(l.trim().replace(/^[| ]*/, "")); return m ? m[1] : null; };
  for (let i = 0; i < lines.length; i++) {
    const callout = prop(lines[i], "IOCalloutDevice");
    if (!callout || !/usb|SLAB|wch/i.test(callout) || (device && callout !== device)) continue;
    // (the nearest values above it: its driver, its interface, its device)
    const near = (k, from = i) => { for (let j = from; j >= 0; j--) { const v = prop(lines[j], k); if (v !== null) return [v, j]; } return [null, -1]; };

    let driver = null;
    for (let j = i; j >= 0; j--) {
      if (prop(lines[j], "IOProviderClass") === "IOUSBHostInterface") {
        // (the same entry's bundle: the lines between its "+-o" and the next)
        let a = j; while (a > 0 && !lines[a].includes("+-o")) a--;
        let b = j; while (b < lines.length && !lines[b + 1]?.includes("+-o")) b++;
        for (let k = a; k <= b; k++) driver = prop(lines[k], "CFBundleIdentifier") || driver;
        break;
      }
    }

    return {
      callout, name: near("USB Product Name")[0], vendor_id: Number(near("idVendor")[0]), product_id: Number(near("idProduct")[0]),
      serial_number: near("USB Serial Number")[0], usb_driver: driver,
    };
  }
  return null;
}

if (process.platform !== "darwin") {
  console.log("esp32_web_check: the profile's permission is written as Chrome keys it on macOS; on Windows and Linux pick the port in the chooser by hand");
  process.exit(2);
}
const port = macPort(device);
if (!port || !port.serial_number || !port.usb_driver) {
  console.log("esp32_web_check: no USB serial port with a serial number" + (device ? ` (${device})` : "") + " — is the board plugged in?", port || "");
  process.exit(1);
}
const { callout, ...grant } = port;
console.log(`board: ${callout} — ${grant.name} ${grant.vendor_id.toString(16)}:${grant.product_id.toString(16)} ${grant.serial_number} (${grant.usb_driver})`);

// (a throwaway profile, the one permission in it: this origin may open this
// port)
const profile = mkdtempSync(join(tmpdir(), "rapidr-esp32-chrome-"));
mkdirSync(join(profile, "Default"));
const origin = new URL(URL_BASE).origin;
writeFileSync(join(profile, "Default", "Preferences"), JSON.stringify({
  profile: { content_settings: { exceptions: { serial_chooser_data: { [`${origin},*`]: { last_modified: "13370000000000000", setting: { "chosen-objects": [grant] } } } } } },
}));

const ctx = await chromium.launchPersistentContext(profile, { channel: "chrome", headless: !process.env.HEADED, viewport: { width: 1000, height: 700 }, deviceScaleFactor: Number(process.env.RAPIDR_DPR || 1) });
let status = 1;
try {
  const page = await ctx.newPage();
  const lines = [];
  page.on("console", (m) => { if (m.type() === "log") lines.push(m.text()); });
  page.on("pageerror", (e) => lines.push("page error: " + e.message));
  await page.goto(`${URL_BASE}/tests/web_kernel.html?COM1`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 30000 });
  const granted = await page.evaluate(async () => JSON.stringify((await navigator.serial.getPorts()).map((p) => p.getInfo())));
  console.log("the page's ports:", granted);
  if (process.env.ESP32_MONITOR) {
    // (examples/iot/esp32_monitor.rr, its Connect then Reset clicked by the
    // test hooks; the page captured to ESP32_MONITOR, a PNG)
    await page.evaluate((source) => {
      window.rr.rapidr_set_test_env({ RAPIDR_TEST_EVENTS: `${["connectbtn.onclick", ...Array(10).fill("state.onclick"), "resetbtn.onclick", ...Array(40).fill("state.onclick")].join(",")}`, RAPIDR_TEST_DUMP: "ports.text,state.caption,log.linecount", RAPIDR_CAPTURE: "web" });
      window.rr.rapidr_run_bc(window.rr.compile(source, "esp32_monitor", {}));
    }, readFileSync(join(ROOT, "examples/iot/esp32_monitor.rr"), "utf8"));
    await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: 60000, polling: 100 }).catch((e) => { console.log(lines.join("\n")); throw e; });
    const dump = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results())).dump;
    console.log(dump.join("\n"));
    await page.screenshot({ path: process.env.ESP32_MONITOR, clip: { x: 0, y: 0, width: 760, height: 480 } });
    status = dump.some((l) => l.startsWith("state.caption=Connected")) ? 0 : 1;
  } else {
    await page.evaluate((source) => window.rr.rapidr_run_bc(window.rr.compile(source, "esp32_check", {})), readFileSync(join(ROOT, "tools/esp32_check.rr"), "utf8"));
    await page.waitForFunction(() => window.rr.rapidr_main_done(), null, { timeout: (Number(seconds) + 30) * 1000, polling: 100 });
    await page.waitForTimeout(300);
    console.log(lines.join("\n"));
    status = lines.some((l) => l.includes("rst:0x")) && lines.some((l) => l.trim() === "closed") ? 0 : 1;
  }
} finally {
  await ctx.close();
  rmSync(profile, { recursive: true, force: true });
}
console.log(status ? "== web: FAILED — no boot log" : "== web: the boot log came (rst:…)");
process.exit(status);
