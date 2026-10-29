// Where RMIDI's song goes (Robert's report, 2026-10-29: a program played a
// MIDI song on a Mac and nothing was heard). RMIDI plays on a system MIDI
// output only when it makes sound; a Mac's outputs are buses that carry
// messages to other programs, which are silence, so the song is played by
// RapidR's built-in synthesizer instead (rapidr_value::objects::midiout).
//
// Nobody can listen to a test, so RMIDI's sink is a file: with
// RAPIDR_TEST_MIDI=capture:<prefix> nothing is played; the song goes to
// <prefix>.txt (where it went; for the synthesizer its loudness) and
// <prefix>.wav (the sound), with the outputs the system "has" in
// RAPIDR_TEST_MIDI_PORTS. The program is examples/media/midi_compose.rr,
// which writes a song in code the way Robert's does, with channel 10 drums;
// its Play button is clicked on the headless host.
//
// Usage (repo root, after building ./rapidr, or RAPIDR_BIN=path):
//   node tests/midi_output.mjs

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RAPIDR = process.env.RAPIDR_BIN || join(ROOT, process.platform === "win32" ? "rapidr.exe" : "rapidr");
const WORK = join(ROOT, "tests/conformance/.work/midi_output");

let failed = 0, passed = 0;
const ok = (cond, msg, detail = "") => {
  console.log(`${cond ? "✓" : "✗"} ${msg}${cond || !detail ? "" : `\n    ${detail}`}`);
  cond ? passed++ : failed++;
};

rmSync(WORK, { recursive: true, force: true });
mkdirSync(join(WORK, "prints"), { recursive: true });

/// Runs the example with Play clicked: where the song went.
function play(name, ports, extraEnv = {}) {
  const dir = join(WORK, name);
  mkdirSync(dir, { recursive: true });
  cpSync(join(ROOT, "examples/media/midi_compose.rr"), join(dir, "midi_compose.rr"));
  const prefix = join(dir, "song");
  const r = spawnSync(RAPIDR, ["run", join(dir, "midi_compose.rr")], {
    cwd: dir,
    encoding: "utf8",
    timeout: 120000,
    env: {
      ...process.env,
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, "registry.reg"),
      RAPIDR_AI_KEYSTORE: "memory",
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_TEST_EVENTS: "playbtn.onclick,info.onclick,info.onclick",
      RAPIDR_TEST_DUMP: "info.caption",
      RAPIDR_TEST_MIDI: `capture:${prefix}`,
      RAPIDR_TEST_MIDI_PORTS: ports,
      RAPIDR_TEST_MIDI_SECS: "9",
      RAPIDR_MIDI_OUT: "",
      ...extraEnv,
    },
  });
  const log = existsSync(`${prefix}.txt`) ? readFileSync(`${prefix}.txt`, "utf8") : "";
  const wav = existsSync(`${prefix}.wav`) ? readFileSync(`${prefix}.wav`) : null;
  return { log, wav, out: (r.stdout || "") + (r.stderr || ""), lines: log.split("\n") };
}

const field = (log, name) => Number((log.match(new RegExp(`^${name}: (.*)$`, "m")) || [])[1]);

/// The synthesizer played it, and it is sound: a 16-bit stereo WAV of at
/// least 8 seconds whose level is a song's, not silence.
function heardOnSynth(name, r) {
  ok(r.log.startsWith("out: synth"), `${name}: RapidR's synthesizer plays it`, r.log.slice(0, 120) || r.out.slice(-300));
  ok(r.wav && r.wav.subarray(0, 4).toString() === "RIFF" && r.wav.readUInt16LE(22) === 2 && r.wav.readUInt16LE(34) === 16, `${name}: it is a stereo 16-bit WAV`);
  ok(field(r.log, "frames") >= 8 * 44100, `${name}: at least 8 seconds of it (${field(r.log, "frames")} frames)`);
  ok(field(r.log, "rms") > 0.01 && field(r.log, "peak") > 0.05 && field(r.log, "peak") <= 1, `${name}: audible, not clipped (rms ${field(r.log, "rms")}, peak ${field(r.log, "peak")})`);
}

// 1. The outputs a Mac has with nothing plugged in: buses to other
//    programs. The song used to be sent to the first of them: silence.
heardOnSynth("buses only", play("buses", "IAC Driver Bus 1;Studio Link-Internal;Session 1"));
heardOnSynth("no outputs", play("none", ""));
heardOnSynth("Linux Midi Through", play("through", "Midi Through:Midi Through Port-0 14:0"));

// 2. A synthesizer among them gets the song: its messages, in time order,
//    with Volume (80) applied to the velocities (70 → 56, 110 → 88), the
//    program changes first, and channel 10's drums (status &H99).
{
  const r = play("synth_port", "IAC Driver Bus 1;Microsoft GS Wavetable Synth");
  ok(r.log.startsWith("out: port Microsoft GS Wavetable Synth"), "a synthesizer's output gets the song", r.log.slice(0, 120));
  const events = r.lines.slice(1).filter(Boolean);
  ok(events[0] === "0 FF 51 03 07 A1 20" || events[0] === "0 C0 04", "the first message plays at 0", events[0]);
  ok(events.includes("0 C0 04") && events.includes("0 C1 21"), "the program changes (piano, bass) are sent");
  ok(events.some((e) => e.endsWith(" 99 24 58")), "the kick is on channel 10 at velocity 110 x 0.8 = 88");
  ok(events.some((e) => e.endsWith(" 90 3C 38")), "the chord's notes are at velocity 70 x 0.8 = 56");
  const times = events.map((e) => Number(e.split(" ")[0]));
  ok(times.every((t, i) => i === 0 || t >= times[i - 1]), "the messages are in time order");
}

// 3. RAPIDR_MIDI_OUT: the program's author chooses.
{
  const r = play("named", "IAC Driver Bus 1;UM-ONE", { RAPIDR_MIDI_OUT: "um-one" });
  ok(r.log.startsWith("out: port UM-ONE"), "RAPIDR_MIDI_OUT names an output (a part of its name, any case)", r.log.slice(0, 80));
  heardOnSynth("RAPIDR_MIDI_OUT=builtin", play("builtin", "Microsoft GS Wavetable Synth", { RAPIDR_MIDI_OUT: "builtin" }));
  const missing = play("missing", "IAC Driver Bus 1", { RAPIDR_MIDI_OUT: "nothing" });
  heardOnSynth("RAPIDR_MIDI_OUT names nothing there", missing);
  ok(/RAPIDR_MIDI_OUT=nothing: no MIDI output has that name \(IAC Driver Bus 1\)/.test(missing.out), "...and says so", missing.out.slice(-300));
}

// 4. The example itself works: composed, opened, played.
{
  const r = play("example", "");
  ok(/info\.caption=playing, \d+ of 8 s/.test(r.out), "midi_compose.rr shows its song playing", r.out.slice(-300));
}

console.log(`\nMIDI output: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
