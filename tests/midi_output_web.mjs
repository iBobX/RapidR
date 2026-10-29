// Where RMIDI's song goes in a browser (tests/midi_output.mjs is the
// desktop's): the page's Web MIDI outputs are chosen by the same rule
// (rapidr_value::objects::midiout) — a bus that only carries messages to
// other programs gets nothing and RapidR's synthesizer plays; a synthesizer
// among them gets the song; the page variable RAPIDR_MIDI_OUT chooses.
//
// The page's navigator.requestMIDIAccess is a stub whose outputs record what
// is sent to them; the program is examples/media/midi_compose.rr with its
// Play clicked on tests/web_kernel.html.
//
// Usage (repo root; the web runtime built: tools/build_web_artifacts.sh; the
// repo served on RAPIDR_URL, default http://localhost:8765 — use your own port):
//   RAPIDR_URL=http://127.0.0.1:<port> node tests/midi_output_web.mjs

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const SOURCE = readFileSync(join(ROOT, "examples/media/midi_compose.rr"), "utf8");

let failed = 0, passed = 0;
const ok = (cond, msg, detail = "") => {
  console.log(`${cond ? "✓" : "✗"} ${msg}${cond || !detail ? "" : `\n    ${detail}`}`);
  cond ? passed++ : failed++;
};

const browser = await (await import("playwright")).chromium.launch();

/// Plays the example on a page whose Web MIDI has `outputs` (names); what each
/// output was sent, as { name: [[bytes…], …] }, and the page's errors.
async function play(outputs, midiOut = "") {
  const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(({ outputs, midiOut }) => {
    window.__sent = Object.fromEntries(outputs.map((n) => [n, []]));
    const outs = outputs.map((name) => ({ name, send: (bytes) => window.__sent[name].push(Array.from(bytes)), clear() {} }));
    const access = { outputs: { values: () => outs[Symbol.iterator]() } };
    Object.defineProperty(navigator, "requestMIDIAccess", { value: () => Promise.resolve(access), configurable: true });
    if (midiOut) window.RAPIDR_MIDI_OUT = midiOut;
  }, { outputs, midiOut });
  await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 20000 });
  await page.evaluate(({ source }) => {
    const bc = window.rr.compile(source, "midi_compose.rr", {});
    window.rr.rapidr_set_test_env({ RAPIDR_CAPTURE: "web", RAPIDR_TEST_EVENTS: "playbtn.onclick,info.onclick,info.onclick,info.onclick", RAPIDR_TEST_DUMP: "info.caption" });
    window.rr.rapidr_run_bc(bc);
  }, { source: SOURCE });
  await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: 60000, polling: 100 });
  await page.waitForTimeout(300);
  const sent = await page.evaluate(() => window.__sent);
  await page.close();
  return { sent, errors };
}

const total = (sent) => Object.values(sent).reduce((n, l) => n + l.length, 0);

// 1. Only buses (a Mac's outputs): nothing is sent to them; no page error
//    from RapidR's synthesizer starting instead.
{
  const r = await play(["IAC Driver Bus 1", "Studio Link-Internal"]);
  ok(total(r.sent) === 0, "buses only: nothing is sent to them", JSON.stringify(r.sent).slice(0, 200));
  ok(r.errors.length === 0, "buses only: no page errors", r.errors.join("; "));
}

// 2. A synthesizer among the buses gets the whole song, Volume applied
//    (the kick, 110 x 0.8 = 88; a chord note, 70 x 0.8 = 56).
{
  const r = await play(["IAC Driver Bus 1", "Microsoft GS Wavetable Synth"]);
  const gs = r.sent["Microsoft GS Wavetable Synth"];
  ok(r.sent["IAC Driver Bus 1"].length === 0 && gs.length > 50, `a synthesizer's output gets the song (${gs.length} messages), the bus none`);
  ok(gs.some((m) => m.join() === "153,36,88"), "the kick is on channel 10 at velocity 88");
  ok(gs.some((m) => m.join() === "144,60,56"), "the chord's notes are at velocity 56");
  ok(gs.some((m) => m.join() === "192,4"), "the program changes are sent");
}

// 3. RAPIDR_MIDI_OUT (a page variable here) names an output, or builtin.
{
  const r = await play(["IAC Driver Bus 1", "UM-ONE"], "um-one");
  ok(r.sent["UM-ONE"].length > 50 && r.sent["IAC Driver Bus 1"].length === 0, "RAPIDR_MIDI_OUT names an output");
  const b = await play(["Microsoft GS Wavetable Synth"], "builtin");
  ok(total(b.sent) === 0 && b.errors.length === 0, "RAPIDR_MIDI_OUT=builtin: RapidR's synthesizer plays, nothing is sent");
}

await browser.close();
console.log(`\nMIDI output (web): ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
