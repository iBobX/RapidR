// Narration: one clip per sentence from the local Qwen3-TTS model on
// MLX-Serve (http://127.0.0.1:11234, POST /v1/audio/speech), the
// language's narrator clip sent as `ref_audio` so every line keeps one
// voice. Clips are cached by (model, narrator, tempo, words): a script
// re-recorded after a change only speaks its new lines.
//
// The server's key is read from the macOS Keychain at run time
// (`security find-generic-password -a $USER -s mlx-serve-key -w`): never
// printed, never written to a file. MLX-Serve is Robert's to start and
// stop — this never starts or stops it; if it isn't running, or memory is
// short (HTTP 503), the pipeline says so and stops. The model is unloaded
// at the end (POST /v1/unload-model).

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { userInfo } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

import { FFMPEG, FFPROBE, TTS, VOICES, WORK } from "./config.mjs";

const run = promisify(execFile);

export class StopError extends Error {}

/** The server's key, from the Keychain (kept in memory only). */
async function keyOf() {
  try {
    const { stdout } = await run("security", ["find-generic-password", "-a", userInfo().username, "-s", TTS.keychainService, "-w"]);
    return stdout.trim();
  } catch {
    throw new StopError(`The narration server's key isn't in the Keychain (service "${TTS.keychainService}"). Add it there; it is never read from a file.`);
  }
}

export async function durationOf(file) {
  const { stdout } = await run(FFPROBE, ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", file]);
  return Math.round(Number(stdout.trim()) * 1000);
}

/** The language's narrator clip, copied into dist/video/voices (git-ignored). */
export function narrator(lang) {
  const v = VOICES[lang.split("-")[0]];
  if (!v) throw new StopError(`No narrator for "${lang}" (tools/video/lib/config.mjs VOICES: ${Object.keys(VOICES).join(", ")}).`);
  if (!existsSync(v.file)) throw new StopError(`The ${lang} narrator's reference clip isn't there: ${v.file} (VIDEO_NARRATOR_${lang.toUpperCase()} or VIDEO_NARRATORS to point at it).`);
  const dir = join(WORK, "voices");
  mkdirSync(dir, { recursive: true });
  const copy = join(dir, `${lang}-${v.name}.wav`);
  if (!existsSync(copy) || statSync(copy).size !== statSync(v.file).size) copyFileSync(v.file, copy);
  return { ...v, copy };
}

/** A connection to the server, checked once: running, the key taken. */
export async function connect() {
  if (!TTS.url) throw new StopError(`MLX_SERVE_URL is ${TTS.asked}: the narration server must be on this computer (127.0.0.1, localhost or ::1) — the key and the narrator's clip never go to another host.`);
  const key = await keyOf();
  let r;
  try {
    r = await fetch(`${TTS.url}/v1/models`, { headers: { Authorization: `Bearer ${key}` }, signal: AbortSignal.timeout(8000) });
  } catch {
    throw new StopError(`MLX-Serve isn't running at ${TTS.url} — start it (the narration needs the local Qwen3-TTS), then run this again. Nothing was recorded.`);
  }
  if (r.status === 401 || r.status === 403) throw new StopError("MLX-Serve refused the key from the Keychain (mlx-serve-key).");
  if (r.status === 503) throw new StopError("MLX-Serve says it is short of memory (HTTP 503): close something and run this again.");
  if (!r.ok) throw new StopError(`MLX-Serve answered ${r.status} to /v1/models.`);
  return { key };
}

async function speak(conn, text, voice, file) {
  const body = JSON.stringify({ model: TTS.model, input: text, ref_audio: readFileSync(voice.copy).toString("base64") });
  let r;
  try {
    r = await fetch(`${TTS.url}/v1/audio/speech`, { method: "POST", headers: { Authorization: `Bearer ${conn.key}`, "Content-Type": "application/json" }, body, signal: AbortSignal.timeout(240000) });
  } catch (e) {
    throw new StopError(`MLX-Serve stopped answering (${e.message}).`);
  }
  if (r.status === 503) throw new StopError("MLX-Serve is short of memory (HTTP 503): close something and run this again (the clips made so far are kept).");
  if (!r.ok) throw new StopError(`Qwen3-TTS failed (${r.status}): ${(await r.text()).slice(0, 200)}`);
  const tmp = `${file}.part.wav`;
  writeFileSync(tmp, Buffer.from(await r.arrayBuffer()));
  return tmp;
}

/** Played `tempo` times as fast, the pitch kept (ffmpeg atempo); mono 44.1 kHz. */
async function settle(src, dest, tempo) {
  const filters = ["aresample=44100"];
  if (Math.abs(tempo - 1) > 1e-6) filters.push(`atempo=${tempo}`);
  await run(FFMPEG, ["-hide_banner", "-loglevel", "error", "-y", "-i", src, "-af", filters.join(","), "-ac", "1", dest]);
}

/**
 * Clips for every line (`lines`: [{ spoken }]), each line given `clip` and
 * `ms`. `conn` null: no voice (a silent take, lengths estimated by demo
 * mode). Returns the number of clips spoken now.
 */
export async function narrate(lines, lang, conn, log = () => {}) {
  const voice = narrator(lang);
  const dir = join(WORK, "clips");
  mkdirSync(dir, { recursive: true });
  const voiceId = createHash("sha1").update(readFileSync(voice.copy)).digest("hex").slice(0, 12);
  let made = 0;
  for (const l of lines) {
    if (!l.spoken.trim()) continue;
    const hash = createHash("sha1").update(`${TTS.model}|${voiceId}|${voice.tempo}|${l.spoken}`).digest("hex").slice(0, 16);
    const file = join(dir, `${lang}-${hash}.wav`);
    if (!existsSync(file)) {
      if (!conn) continue;
      log(`   voice  ${l.spoken.slice(0, 70)}${l.spoken.length > 70 ? "…" : ""}`);
      const raw = await speak(conn, l.spoken, voice, file);
      await settle(raw, `${file}.tmp.wav`, voice.tempo);
      renameSync(`${file}.tmp.wav`, file);
      rmSync(raw, { force: true });
      made++;
    }
    l.clip = file;
    l.ms = await durationOf(file);
  }
  return made;
}

/** The model let go of (its memory back), when the server is there. */
export async function unload(conn) {
  if (!conn) return;
  try {
    await fetch(`${TTS.url}/v1/unload-model`, { method: "POST", headers: { Authorization: `Bearer ${conn.key}`, "Content-Type": "application/json" }, body: JSON.stringify({ model: TTS.model }), signal: AbortSignal.timeout(20000) });
  } catch {
    // (the server may have gone meanwhile: nothing to unload)
  }
}
