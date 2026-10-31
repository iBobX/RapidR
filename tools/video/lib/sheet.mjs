// A contact sheet of a finished video (ffmpeg tile): evenly spaced moments,
// to look at after every take — a stuck highlight, English text in a
// Spanish take, a clipped caption, the pointer off its target, anything
// private. One sheet per ~75 s of video. (This ffmpeg has no drawtext, so
// the tiles aren't stamped: the log says where the first tile is and how far
// apart they are; tiles run left to right, top to bottom.)

import { execFile } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { promisify } from "node:util";

import { FFMPEG } from "./config.mjs";

const exec = promisify(execFile);

/**
 * Sheets of `cols` x `rows` tiles (each `tile` px wide) for the video at
 * `mp4`. Returns [{ file, from, step }]: tile i (0-based) is the frame at
 * `from + step * (i + 0.5)` seconds.
 */
export async function contactSheets({ mp4, out, name, seconds, cols = 4, rows = 5, tile = 480, perSheet = 75 }) {
  mkdirSync(out, { recursive: true });
  const n = cols * rows;
  const count = Math.max(1, Math.ceil(seconds / perSheet));
  const span = seconds / count;
  const made = [];
  for (let k = 0; k < count; k++) {
    const file = join(out, count === 1 ? `${name}.jpg` : `${name}-${k + 1}.jpg`);
    const step = span / n;
    // (a frame at the middle of each interval: not the black first frame)
    const vf = `fps=1/${step.toFixed(4)}:start_time=${(step / 2).toFixed(3)},scale=${tile}:-1,tile=${cols}x${rows}:padding=4:margin=4:color=0x202020`;
    await exec(FFMPEG, ["-hide_banner", "-loglevel", "error", "-y", "-ss", (k * span).toFixed(3), "-t", span.toFixed(3), "-i", mp4, "-vf", vf, "-frames:v", "1", "-q:v", "3", file], { maxBuffer: 32 * 1024 * 1024 });
    made.push({ file, from: k * span, step });
  }
  return made;
}
