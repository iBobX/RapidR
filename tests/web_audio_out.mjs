// RAudioOut on the web's real device (crates/rapidr-runtime-web's
// audio_web.rs), not the tests' sink: no test hooks on this page, the
// program's own Start, and a real mouse click (the browser's rule: sound
// only after a click or a key on the page).
//
//   1. Start before any click: it plays but the browser holds the sound —
//      Error says so, nothing is taken from the queue;
//   2. a real click on the page's button: the page's audio context runs,
//      Error clears, OnNeedData keeps the queue fed (Web Audio's script
//      processor takes the samples), no gap in two seconds;
//   3. Stop: Playing 0, nothing queued.
//
// (Headless Chromium plays to a silent device; the samples are made at
// Volume 0 anyway.)
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765): node tests/web_audio_out.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = `
DIM S AS RAudioOut
DIM phase AS DOUBLE, buf(8191) AS INTEGER, asked AS LONG
SUB Feed (Count AS INTEGER)
    DIM i AS INTEGER
    FOR i = 0 TO Count - 1
        buf(i) = SIN(phase) * 16000
        phase = phase + 2 * 3.14159265 * 440 / 44100
    NEXT
    S.Write buf, Count
    asked = asked + 1
    Counter.Caption = STR$(asked)
END SUB
SUB Play
    S.Start
END SUB
SUB Halt
    S.Stop
END SUB
CREATE Form AS RForm
    Caption = "stream"
    ClientWidth = 300: ClientHeight = 120
    CREATE PlayBtn AS RButton
        Caption = "Play": Left = 10: Top = 10: Width = 100: Height = 30
        OnClick = Play
    END CREATE
    CREATE StopBtn AS RButton
        Caption = "Stop": Left = 120: Top = 10: Width = 100: Height = 30
        OnClick = Halt
    END CREATE
    CREATE Counter AS RLabel
        Left = 10: Top = 60: Width = 200
    END CREATE
END CREATE
S.Volume = 0
S.OnNeedData = Feed
S.Start
Form.ShowModal
`;

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
// (headless Chromium lets any page play at once — its autoplay flags don't
// reach Web Audio there, and Playwright's own calls count as a person's —,
// so the page gets a browser's rule back: an audio context starts
// suspended, and resume() works only while a real click or key is being
// handled)
await page.addInitScript(() => {
  let gesture = false;
  for (const type of ["pointerdown", "keydown"]) {
    window.addEventListener(type, (e) => {
      if (e.isTrusted) { gesture = true; setTimeout(() => { gesture = false; }, 0); }
    }, true);
  }
  const Real = window.AudioContext;
  window.AudioContext = class extends Real {
    constructor(...a) { super(...a); this.__allowed = false; super.suspend(); }
    get state() { return this.__allowed ? super.state : "suspended"; }
    resume() {
      if (gesture) { this.__allowed = true; return super.resume(); }
      return Promise.resolve();
    }
  };
});
await page.goto(`${URL_BASE}/${process.env.RAPIDR_KERNEL_PAGE || "tests/web_kernel.html"}`, { waitUntil: "load" });
await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "stream", {})), SOURCE);
await k.waitFor(page, "PlayBtn");
const get = async (p) => (await k.prop(page, "S", p)).trim();
await page.waitForTimeout(600);
const before = { error: await get("Error"), playing: await get("Playing"), queued: Number(await get("Queued")), asked: Number(await k.prop(page, "Counter", "Caption")) };
ok(/after a click or a key/.test(before.error) && before.playing === "1", `Start before a click: playing, Error says the browser waits for a click ("${before.error.slice(0, 60)}…")`);
ok(before.queued >= 55 && before.asked <= 1, `and nothing is taken meanwhile: ${before.queued} ms queued, OnNeedData ${before.asked} time(s)`);
// (the program started its stream itself; the person's first click — on
// any button — lets it play)
await k.click(page, "PlayBtn");
await page.waitForTimeout(2000);
const asked0 = Number(await k.prop(page, "Counter", "Caption"));
await page.waitForTimeout(1000);
const after = { error: await get("Error"), queued: Number(await get("Queued")), underruns: Number(await get("Underruns")), asked: Number(await k.prop(page, "Counter", "Caption")) };
ok(after.error === "", `after a real click Error clears ("${after.error}")`);
ok(after.asked > asked0 + 20, `the device takes the samples and OnNeedData keeps it fed (${after.asked - asked0} times in a second)`);
ok(after.queued > 0 && after.queued <= 120, `the queue stays near BufferMs (${after.queued} ms)`);
ok(after.underruns === 0, `no gap in three seconds (Underruns ${after.underruns})`);
await k.click(page, "StopBtn");
await page.waitForTimeout(300);
ok((await get("Playing")) === "0" && Number(await get("Queued")) === 0, "Stop: not playing, nothing queued");
ok(errors.length === 0, `no page errors (${errors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb audio out: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb audio out: ALL CHECKS PASSED");
