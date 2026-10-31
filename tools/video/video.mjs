#!/usr/bin/env node
// RapidR Studio's narrated tutorial videos, made from scripts
// (docs/video-pipeline.md):
//
//   node tools/video/video.mjs tools/video/scripts/first-app.json --lang en,es
//
// For each language: the narration (the local Qwen3-TTS, one clip a
// sentence, cached), a take (Studio headless in demo mode, every frame
// drawn by the kernel), the video (ffmpeg: frames + voice + optional
// music bed, captions .srt, a poster). Output: dist/video/ (git-ignored;
// working files in dist/video/takes, clips, voices).
//
// Options:
//   --lang en,es        the languages (default: every one the title has)
//   --silent            no voice (a dry take: lines timed from their words)
//   --mux-only          the video again from the last take
//   --music FILE        a music bed (Robert chooses; none by default)
//   --webm              a WebM too (VP9 + Opus, royalty-free: rapidr.dev's pages)
//   --out DIR           where the videos go (default dist/video)

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { COPYRIGHT, WORK, creditsFor } from "./lib/config.mjs";
import { chapters, describe } from "./lib/describe.mjs";
import { mux } from "./lib/mux.mjs";
import { contactSheets } from "./lib/sheet.mjs";
import { StopError, connect, narrate, unload } from "./lib/narration.mjs";
import { record } from "./lib/record.mjs";
import { absolutePaths, languagesOf, linesOf, readScript, resolveLang } from "./lib/script.mjs";

function usage(msg) {
  if (msg) console.error(`video: ${msg}`);
  console.error("usage: node tools/video/video.mjs <script.json> [--lang en,es] [--silent] [--mux-only] [--music FILE] [--webm] [--out DIR]");
  process.exit(2);
}

const argv = process.argv.slice(2);
const opts = { langs: null, silent: false, muxOnly: false, music: null, webm: false, out: WORK };
let file = null;
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === "--lang") opts.langs = (argv[++i] || "").split(",").map((s) => s.trim()).filter(Boolean);
  else if (a === "--silent") opts.silent = true;
  else if (a === "--mux-only") opts.muxOnly = true;
  else if (a === "--music") opts.music = resolve(argv[++i] || "");
  else if (a === "--webm") opts.webm = true;
  else if (a === "--out") opts.out = resolve(argv[++i] || "");
  else if (a === "--help" || a === "-h") usage();
  else if (a.startsWith("-")) usage(`unknown option ${a}`);
  else file = a;
}
if (!file) usage("which script?");

const log = (s) => console.log(s);
const { raw, dir } = readScript(file);
const langs = opts.langs || languagesOf(raw);
const id = raw.id || "demo";
let conn = null;
let failed = false;
try {
  if (!opts.silent && !opts.muxOnly) conn = await connect();
  for (const lang of langs) {
    log(`${id} · ${lang}`);
    const script = absolutePaths(resolveLang(structuredClone(raw), lang), dir);
    const lines = linesOf(script);
    const take = join(WORK, "takes", `${id}-${lang}`);
    mkdirSync(take, { recursive: true });
    if (!opts.muxOnly) {
      if (conn) {
        const made = await narrate(lines, lang, conn, log);
        log(`   voice  ${lines.length} line(s), ${made} new clip(s)`);
        for (const l of lines) {
          if (l.clip) Object.assign(l.obj, { ms: l.ms, clip: l.clip });
          const say = script.steps[l.step].say;
          say[l.line] = l.obj;
        }
      }
      await record({ script, lang, take, log });
    }
    const title = typeof script.title === "string" ? script.title : id;
    const name = `${id}-${lang}`;
    const credits = creditsFor(lang);
    const res = await mux({ take, out: opts.out, name, music: opts.music, webm: opts.webm, log, meta: { title, comment: credits.disclosure, copyright: COPYRIGHT } });
    // (title, summary, the AI-voice disclosure right under it — seen before
    // "Show more" —, chapters, credits, links, tags: lib/describe.mjs)
    const timeline = JSON.parse(readFileSync(join(take, "timeline.json"), "utf8"));
    const desc = describe({
      title,
      summary: script.description || "",
      tags: Array.isArray(script.tags) ? script.tags : [],
      credits,
      chapterList: chapters(script.steps, timeline.lines),
      music: opts.music ? script.musicCredit || "(credit the track here)" : null,
      demoModel: !!script.demoModel,
    });
    writeFileSync(join(opts.out, `${name}.description.txt`), desc);
    // (a contact sheet to look at: every tile a moment, its time on it)
    const sheets = await contactSheets({ mp4: res.mp4, out: join(opts.out, "sheets"), name, seconds: res.seconds });
    log(`   video  ${res.mp4} (${res.seconds.toFixed(1)} s)${res.webm ? `\n          ${res.webm}` : ""}\n          ${res.srt}\n          ${res.poster}\n          ${sheets.map((s) => `${s.file} (tiles ${s.step.toFixed(1)} s apart from ${s.from.toFixed(0)} s)`).join("\n          ")}`);
  }
} catch (e) {
  failed = true;
  console.error(e instanceof StopError ? `video: ${e.message}` : `video: ${e.stack || e.message}`);
} finally {
  await unload(conn);
}
process.exit(failed ? 1 : 0);
