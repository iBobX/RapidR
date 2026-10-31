// The video pipeline's pure parts (tools/video, docs/video-pipeline.md):
// chapter times, the description's order, the audio graph (the music bed
// high-passed at 100 Hz, the voice placed where its line starts), the
// loudness target (-14 LUFS, -1.5 dBTP), the launch scripts' shape.
//
//   node tests/video_pipeline.mjs

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { CREDITS } from "../tools/video/lib/config.mjs";
import { chapters, clock, describe } from "../tools/video/lib/describe.mjs";
import { BED_HIGHPASS_HZ, TARGET, buildSrt, duckExpression, loudnessFilter, premixGraph, wrap } from "../tools/video/lib/mux.mjs";
import { isLang, languagesOf, linesOf, resolveLang, stripComments } from "../tools/video/lib/script.mjs";

const ROOT = new URL("..", import.meta.url).pathname;
let n = 0;
const test = (name, f) => {
  f();
  n++;
  console.log(`ok  ${name}`);
};

test("chapter times read as a video site wants them", () => {
  assert.equal(clock(0), "0:00");
  assert.equal(clock(75.4), "1:15");
  assert.equal(clock(3725), "1:02:05");
});

test("chapters: the first at 0:00, three at least, none shorter than 10 s", () => {
  const steps = [{ chapter: "Intro" }, {}, { chapter: "Build" }, { chapter: "Too soon" }, { chapter: "Run" }];
  const lines = [
    { step: 1, start: 800 },
    { step: 3, start: 14000 },
    { step: 4, start: 18000 },
    { step: 5, start: 31000 },
  ];
  assert.deepEqual(chapters(steps, lines), [{ name: "Intro", at: 0 }, { name: "Build", at: 14 }, { name: "Run", at: 31 }]);
  // (two chapters are none: a video site shows three or more)
  assert.deepEqual(chapters([{ chapter: "A" }, { chapter: "B" }], [{ step: 1, start: 0 }, { step: 2, start: 20000 }]), []);
});

test("the description: summary, then the AI-voice disclosure, then chapters, credits, links, tags", () => {
  for (const lang of ["en", "es"]) {
    const text = describe({
      title: "T",
      summary: "Summary.",
      tags: ["a", "b"],
      credits: CREDITS[lang],
      chapterList: [{ name: "One", at: 0 }, { name: "Two", at: 12 }, { name: "Three", at: 30 }],
      demoModel: true,
    });
    const at = (s) => {
      const i = text.indexOf(s);
      assert.ok(i >= 0, `${lang}: "${s}" is in the description`);
      return i;
    };
    assert.ok(at("Summary.") < at(CREDITS[lang].disclosure), "the disclosure is right under the summary");
    assert.ok(at(CREDITS[lang].disclosure) < at("0:00 One"), "chapters after the disclosure");
    assert.ok(at("0:30 Three") < at(CREDITS[lang].voice), "credits after chapters");
    assert.ok(at(CREDITS[lang].voice) < at("https://github.com/iBobX/RapidR"), "links after credits");
    for (const link of ["https://github.com/iBobX/RapidR", "https://github.com/sponsors/iBobX", "https://www.buymeacoffee.com/roanbema", "https://rapidr.dev"]) at(link);
    assert.ok(text.trimEnd().endsWith("a, b"), "tags last");
    assert.ok(text.includes(CREDITS[lang].demoNote), "a demo model is said to be one");
    assert.doesNotMatch(text, /RapidQ/i);
  }
});

test("the audio: voice clips placed by their start, the music bed high-passed at 100 Hz and ducked", () => {
  const clips = [{ clip: "a.wav", start: 500, ms: 2000 }, { clip: "b.wav", start: 4000, ms: 1000 }];
  const withBed = premixGraph({ clips, bed: "bed.wav", total: 10 });
  const graph = withBed.filters.join(";");
  assert.match(graph, /\[0:a\][^;]*adelay=500\|500\[a0\]/);
  assert.match(graph, /\[1:a\][^;]*adelay=4000\|4000\[a1\]/);
  assert.match(graph, new RegExp(`\\[2:a\\][^;]*highpass=f=${BED_HIGHPASS_HZ}`));
  assert.equal(BED_HIGHPASS_HZ, 100);
  assert.equal(withBed.label, "[mix]");
  assert.deepEqual(withBed.inputs.slice(-4), ["-stream_loop", "-1", "-i", "bed.wav"], "the bed loops, after the clips");
  const voiceOnly = premixGraph({ clips, bed: null, total: 10 });
  assert.equal(voiceOnly.label, "[voice]");
  assert.doesNotMatch(voiceOnly.filters.join(";"), /highpass/);
  assert.match(duckExpression([{ start: 0.5, end: 2.5 }]), /pow\(10,/);
});

test("the loudness: -14 LUFS integrated, -1.5 dBTP, the second pass linear from the first's numbers", () => {
  assert.deepEqual(TARGET, { i: -14, tp: -1.5, lra: 11 });
  const f = loudnessFilter({ input_i: "-22.0", input_tp: "-4.1", input_lra: "3.0", input_thresh: "-32.5", target_offset: "0.2" });
  assert.match(f, /^loudnorm=I=-14:TP=-1.5:LRA=11:measured_I=-22\.0:measured_TP=-4\.1:measured_LRA=3\.0:measured_thresh=-32\.5:offset=0\.2:linear=true/);
});

test("captions: two balanced lines at most", () => {
  assert.equal(wrap("Short."), "Short.");
  const w = wrap("Studio opens its code, right where the click goes, and waits.");
  assert.equal(w.split("\n").length, 2);
  const srt = buildSrt([{ text: "One.", start: 0, ms: 1000 }, { text: "Two.", start: 2000, ms: 1000 }]);
  assert.match(srt, /1\n00:00:00,000 --> 00:00:01,200\nOne\./);
});

test("the launch scripts: valid, English and Spanish for every line, nothing real named", () => {
  const dir = join(ROOT, "tools/video/scripts/launch");
  const ids = new Set();
  const names = readdirSync(dir).filter((f) => f.endsWith(".json"));
  assert.ok(names.includes("0-trailer.json"));
  for (const name of names) {
    const raw = JSON.parse(stripComments(readFileSync(join(dir, name), "utf8")));
    assert.ok(!ids.has(raw.id), `${name}: its id is its own`);
    ids.add(raw.id);
    assert.deepEqual(languagesOf(raw).sort(), ["en", "es"], `${name}: English and Spanish titles`);
    for (const lang of ["en", "es"]) {
      const script = resolveLang(structuredClone(raw), lang);
      for (const l of linesOf(script)) assert.ok(l.spoken.trim(), `${name} ${lang}: a line to speak`);
      const text = JSON.stringify(script);
      assert.doesNotMatch(text, /RapidQ|Ollama|LM Studio|ChatGPT|GPT-|Claude|Gemini|Delphi|Xojo|Visual Basic/i, `${name} ${lang}: no other product's name`);
    }
    for (const k of Object.keys(raw.title)) assert.ok(isLang(k));
  }
});

console.log(`${n} passed`);
