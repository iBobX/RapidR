// The recording made into a video (ffmpeg, a tool on this Mac — never
// bundled with RapidR): demo mode's frames held for their times (the
// concat demuxer), each narration clip placed where its line started, an
// optional music bed (100 Hz high-passed, ducked 12 dB under the voice), the
// final mix loudness-normalised in two passes to EBU R128 -14 LUFS
// integrated, -1.5 dBTP, H.264 (CRF 20) + AAC 160k; the captions as an .srt
// beside it (they are burned into the frames already: demo mode draws them);
// a poster frame as a JPEG.

import { execFile } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import { FFMPEG } from "./config.mjs";

const exec = promisify(execFile);
const ffmpeg = (args, level = "error") => exec(FFMPEG, ["-hide_banner", "-loglevel", level, "-y", ...args], { maxBuffer: 64 * 1024 * 1024 });

/** The final mix's loudness (the web's usual: -14 LUFS integrated, -1.5 dBTP). */
export const TARGET = { i: -14, tp: -1.5, lra: 11 };
/** The music bus is high-passed so a bed never muddies the voice. */
export const BED_HIGHPASS_HZ = 100;

/** The music bed's level: -2 dB at rest, 12 dB lower under the voice. */
const BED_GAIN_DB = -2;
const DUCK_DB = 12;
const DUCK_IN = 0.35;
const DUCK_OUT = 0.7;
/** Pauses shorter than this stay ducked (no pumping between sentences). */
const DUCK_MERGE_GAP = 1.2;

/** The bed's gain over time t (s) as an ffmpeg expression. */
export function duckExpression(spans) {
  const merged = [];
  for (const s of [...spans].sort((a, b) => a.start - b.start)) {
    const last = merged[merged.length - 1];
    if (last && s.start - last.end < DUCK_MERGE_GAP) last.end = Math.max(last.end, s.end);
    else merged.push({ ...s });
  }
  const ramp = ({ start, end }) => `clip((t-${(start - DUCK_IN).toFixed(3)})/${DUCK_IN},0,1)*clip((${(end + DUCK_OUT).toFixed(3)}-t)/${DUCK_OUT},0,1)`;
  const duck = merged.length ? merged.map(ramp).reduce((a, b) => `max(${a},${b})`) : "0";
  return `pow(10,(${BED_GAIN_DB}-${DUCK_DB}*(${duck}))/20)`;
}

const stamp = (s) => {
  const ms = Math.max(0, Math.round(s * 1000));
  const p = (n, w = 2) => String(n).padStart(w, "0");
  return `${p(Math.floor(ms / 3600000))}:${p(Math.floor(ms / 60000) % 60)}:${p(Math.floor(ms / 1000) % 60)},${p(ms % 1000, 3)}`;
};

/** A caption in at most two balanced lines. */
export function wrap(text, width = 46) {
  if (text.length <= width) return text;
  const words = text.split(" ");
  let best = text;
  let score = Infinity;
  for (let i = 1; i < words.length; i++) {
    const a = words.slice(0, i).join(" ");
    const b = words.slice(i).join(" ");
    const s = Math.max(a.length, b.length);
    if (s < score) {
      score = s;
      best = `${a}\n${b}`;
    }
  }
  return best;
}

/** The .srt: one cue a line, from its start to its clip's end (never past the next one). */
export function buildSrt(lines) {
  const cues = lines.filter((l) => l.text && l.text.trim());
  return cues
    .map((l, i) => {
      const start = l.start / 1000;
      const next = cues[i + 1] ? cues[i + 1].start / 1000 : Infinity;
      const end = Math.min(start + l.ms / 1000 + 0.2, next - 0.02);
      return `${i + 1}\n${stamp(start)} --> ${stamp(end)}\n${wrap(l.text)}\n`;
    })
    .join("\n");
}

/** The concat list: each frame held until the next one's time. */
function frameList(frames, dir, endMs, listFile) {
  const lines = [];
  frames.forEach((f, i) => {
    const next = i + 1 < frames.length ? frames[i + 1].t : endMs;
    lines.push(`file '${join(dir, f.file)}'`, `duration ${Math.max(0.001, (next - f.t) / 1000).toFixed(6)}`);
  });
  // (the concat demuxer drops the last entry's duration unless it is listed again)
  lines.push(`file '${join(dir, frames[frames.length - 1].file)}'`);
  writeFileSync(listFile, `${lines.join("\n")}\n`);
}

/**
 * The audio before loudness: each narration clip placed where its line
 * started, the music bed (when there is one) high-passed at 100 Hz and
 * ducked under the voice. Returns the inputs and the filter graph.
 */
export function premixGraph({ clips, bed, total }) {
  const inputs = [];
  clips.forEach((c) => inputs.push("-i", c.clip));
  if (bed) inputs.push("-stream_loop", "-1", "-i", bed);
  const filters = [];
  clips.forEach((c, i) => {
    const ms = Math.max(0, Math.round(c.start));
    filters.push(`[${i}:a]aresample=44100,pan=stereo|c0=c0|c1=c0,adelay=${ms}|${ms}[a${i}]`);
  });
  let label = null;
  if (clips.length) {
    filters.push(`${clips.map((_, i) => `[a${i}]`).join("")}amix=inputs=${clips.length}:duration=longest:normalize=0,apad=whole_dur=${total.toFixed(3)}[voice]`);
    label = "[voice]";
  }
  if (bed) {
    const spans = clips.map((c) => ({ start: c.start / 1000, end: (c.start + c.ms) / 1000 }));
    const b = clips.length;
    filters.push(`[${b}:a]aresample=44100,aformat=channel_layouts=stereo,highpass=f=${BED_HIGHPASS_HZ},atrim=duration=${total.toFixed(3)},afade=t=in:st=0:d=1.5,volume='${duckExpression(spans)}':eval=frame,afade=t=out:st=${Math.max(0, total - 2.5).toFixed(3)}:d=2.5,apad=whole_dur=${total.toFixed(3)}[bed]`);
    if (label) {
      filters.push(`[voice][bed]amix=inputs=2:duration=first:normalize=0[mix]`);
      label = "[mix]";
    } else {
      label = "[bed]";
    }
  }
  return { inputs, filters, label };
}

/** Speech has a high peak-to-loudness ratio: a gentle compressor first, so -14 LUFS fits under -1.5 dBTP. */
const LEVELLER = "acompressor=threshold=-20dB:ratio=3:attack=8:release=120:makeup=1,alimiter=limit=0.84:attack=3:release=40:level=false";

/** A file's loudness as loudnorm measures it (first pass). */
export async function measure(file, pre = "") {
  const { stderr } = await ffmpeg(["-i", file, "-af", `${pre}${pre ? "," : ""}loudnorm=I=${TARGET.i}:TP=${TARGET.tp}:LRA=${TARGET.lra}:print_format=json`, "-f", "null", "-"], "info");
  const m = stderr.match(/\{[^{}]*"input_i"[^{}]*\}/);
  if (!m) throw new Error("couldn't measure the loudness");
  return JSON.parse(m[0]);
}

/** The loudness filter for the second pass: linear, from the first pass's numbers. */
export function loudnessFilter(m) {
  return `loudnorm=I=${TARGET.i}:TP=${TARGET.tp}:LRA=${TARGET.lra}:measured_I=${m.input_i}:measured_TP=${m.input_tp}:measured_LRA=${m.input_lra}:measured_thresh=${m.input_thresh}:offset=${m.target_offset}:linear=true,alimiter=limit=0.82:attack=2:release=30:level=false`;
}

/**
 * The video from demo mode's output folder `take` (frames/, timeline.json):
 * `<out>/<name>.mp4`, `.srt`, `.jpg`. `music`: a bed (optional, Robert's
 * choice of track); `meta`: title and credits for the file's tags.
 */
export async function mux({ take, out, name, music, webm = false, meta = {}, log = () => {} }) {
  mkdirSync(out, { recursive: true });
  const timeline = JSON.parse(readFileSync(join(take, "timeline.json"), "utf8"));
  const index = JSON.parse(readFileSync(join(take, "frames", "frames.json"), "utf8"));
  const frames = index.frames;
  if (!frames.length) throw new Error("the recording has no frames");
  const fps = timeline.fps || 30;
  const endMs = Math.max(timeline.end || 0, frames[frames.length - 1].t + 1000 / fps);
  const total = endMs / 1000;
  const listFile = join(take, "frames.txt");
  frameList(frames, join(take, "frames"), endMs, listFile);
  const [w, h] = index.size;
  const clips = timeline.lines.filter((l) => l.clip && existsSync(l.clip));
  const bed = music && existsSync(music) ? music : null;
  if (music && !bed) log(`   music  ${music} isn't there: no music bed`);
  // (the audio first, on its own: placed clips, the bed, then the loudness
  // measured on the leveled mix and applied in a second pass)
  let audioFile = null;
  let loud = null;
  if (clips.length || bed) {
    const g = premixGraph({ clips, bed, total });
    const premixScript = join(take, "premix-filter.txt");
    writeFileSync(premixScript, [...g.filters, `${g.label}${LEVELLER},atrim=duration=${total.toFixed(3)}[pre]`].join(";\n"));
    audioFile = join(take, "premix.wav");
    await ffmpeg([...g.inputs, "-filter_complex_script", premixScript, "-map", "[pre]", "-ar", "44100", "-c:a", "pcm_f32le", audioFile]);
    loud = await measure(audioFile);
  }
  const inputs = ["-f", "concat", "-safe", "0", "-i", listFile];
  if (audioFile) inputs.push("-i", audioFile);
  const filters = [
    // (1920 × 1080 whatever the frames' size; Lanczos when they're bigger: sharp)
    `[0:v]fps=${fps},scale=1920:1080:flags=lanczos,setsar=1,format=yuv420p,trim=duration=${total.toFixed(3)},fade=t=in:st=0:d=0.4,fade=t=out:st=${Math.max(0, total - 0.6).toFixed(3)}:d=0.6[v]`,
  ];
  let audio = null;
  if (audioFile) {
    filters.push(`[1:a]${loudnessFilter(loud)},afade=t=in:st=0:d=0.2,afade=t=out:st=${Math.max(0, total - 0.7).toFixed(3)}:d=0.7,apad=whole_dur=${total.toFixed(3)}[a]`);
    audio = "[a]";
  }
  const script = join(take, "filter.txt");
  writeFileSync(script, filters.join(";\n"));
  const mp4 = join(out, `${name}.mp4`);
  const tags = Object.entries({ title: meta.title, comment: meta.comment, copyright: meta.copyright }).filter(([, v]) => v).flatMap(([k, v]) => ["-metadata", `${k}=${v}`]);
  log(`   mux    ${frames.length} frames (${index.written} distinct), ${clips.length} narration clip(s)${bed ? ", music bed" : ""}, ${total.toFixed(1)} s${loud ? `, mix ${Number(loud.input_i).toFixed(1)} LUFS -> ${TARGET.i}` : ""}`);
  await ffmpeg([
    ...inputs,
    "-filter_complex_script", script,
    "-map", "[v]",
    ...(audio ? ["-map", audio, "-c:a", "aac", "-b:a", "160k", "-ar", "44100"] : []),
    "-c:v", "libx264", "-crf", "20", "-preset", "slow", "-pix_fmt", "yuv420p", "-r", String(fps),
    "-movflags", "+faststart",
    ...tags,
    "-t", total.toFixed(3),
    mp4,
  ]);
  // (--webm: the same video in royalty-free codecs, VP9 + Opus, for
  // rapidr.dev's own pages; the MP4 is the upload master for video sites)
  let webmFile = null;
  if (webm) {
    webmFile = join(out, `${name}.webm`);
    log("   webm   VP9 + Opus");
    await ffmpeg([
      ...inputs,
      "-filter_complex_script", script,
      "-map", "[v]",
      ...(audio ? ["-map", audio, "-c:a", "libopus", "-b:a", "128k", "-ar", "48000"] : []),
      "-c:v", "libvpx-vp9", "-crf", "31", "-b:v", "0", "-row-mt", "1", "-deadline", "good", "-cpu-used", "2", "-pix_fmt", "yuv420p", "-r", String(fps),
      ...tags,
      "-t", total.toFixed(3),
      webmFile,
    ]);
  }
  const srt = join(out, `${name}.srt`);
  writeFileSync(srt, buildSrt(timeline.lines));
  // (the poster: the frame at the "poster" mark, else the last one — straight
  // from the recorded PNG, sharper than a frame of the MP4)
  const at = timeline.marks?.poster ?? frames[frames.length - 1].t;
  const pf = frames.reduce((best, f) => (f.t <= at + 1 ? f : best), frames[0]);
  const poster = join(out, `${name}.jpg`);
  await ffmpeg(["-i", join(take, "frames", pf.file), "-vf", `scale=1920:1080:flags=lanczos`, "-q:v", "2", poster]);
  // (beside the video: the timeline with each clip's file name only — no
  // path of this computer next to the files that get posted)
  const shared = { ...timeline, lines: timeline.lines.map((l) => (l.clip ? { ...l, clip: basename(l.clip) } : l)) };
  writeFileSync(join(out, `${name}.timeline.json`), `${JSON.stringify(shared, null, 2)}\n`);
  return { mp4, webm: webmFile, srt, poster, seconds: total, size: [w, h] };
}
