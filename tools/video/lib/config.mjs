// The video pipeline's settings (docs/video-pipeline.md). Everything can be
// changed with an environment variable; nothing secret lives here — the
// narration server's key is read from the macOS Keychain at run time.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** tools/video */
export const HERE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
/** The RapidR checkout. */
export const ROOT = resolve(HERE, "..", "..");

/**
 * The main checkout (a worktree's dist/ is empty: the narrators' clips and
 * the videos live in the main checkout's git-ignored dist/).
 */
function mainCheckout() {
  try {
    const common = execFileSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    return common.endsWith("/.git") ? dirname(common) : ROOT;
  } catch {
    return ROOT;
  }
}
export const MAIN = mainCheckout();

/** Working files and the videos: git-ignored (dist/). Audio never goes in Git. */
export const WORK = resolve(process.env.VIDEO_WORK || join(MAIN, "dist", "video"));

/** The frame rate the recordings are made at. */
export const FPS = 30;

/**
 * The narrators (Robert's picks, 2026-10-30; docs/video-pipeline.md):
 * voices made up by the model (generated with no reference recording),
 * kept as reference clips in the git-ignored dist/voices/narrators and sent
 * as `ref_audio` on every line so each language keeps one narrator; no
 * real person's recording as reference. `tempo` plays the clips faster
 * with the pitch kept (ffmpeg atempo).
 */
const NARRATORS = process.env.VIDEO_NARRATORS || join(MAIN, "dist", "voices", "narrators");
export const VOICES = {
  en: { file: process.env.VIDEO_NARRATOR_EN || join(NARRATORS, "en-79.wav"), tempo: 1.0, name: "voice-79" },
  es: { file: process.env.VIDEO_NARRATOR_ES || join(NARRATORS, "es-13.wav"), tempo: 1.2, name: "es-13" },
  pt: { file: process.env.VIDEO_NARRATOR_PT || join(NARRATORS, "pt-25.wav"), tempo: 1.0, name: "pt-25" },
  zh: { file: process.env.VIDEO_NARRATOR_ZH || join(NARRATORS, "zh-21.wav"), tempo: 1.0, name: "zh-21" },
};

/**
 * The local Qwen3-TTS (Apache-2.0) on MLX-Serve: a tool on this Mac, never
 * bundled with RapidR. Only this computer (loopback): the key and the
 * narrator's clip never go to another host.
 */
function loopback(url) {
  try {
    const h = new URL(url).hostname.replace(/^\[|\]$/g, "");
    return h === "127.0.0.1" || h === "localhost" || h === "::1" ? url : null;
  } catch {
    return null;
  }
}
const ttsUrl = (process.env.MLX_SERVE_URL || "http://127.0.0.1:11234").replace(/\/+$/, "");
export const TTS = {
  /** null: MLX_SERVE_URL names another computer (refused). */
  url: loopback(ttsUrl),
  asked: ttsUrl,
  model: process.env.VIDEO_TTS_MODEL || "mlx-community/Qwen3-TTS-12Hz-1.7B-Base-bf16",
  /** The Keychain item holding the key (account: the user). */
  keychainService: "mlx-serve-key",
};

/**
 * The credits every video carries, per language (the cards say them in
 * the script; the description and the file's tags from here):
 * docs/legal/review-2026-10-30-demo-mode.md M1. The links are the same in
 * every language (lib/describe.mjs puts them in the description).
 */
export const LINKS = {
  site: "https://rapidr.dev",
  source: "https://github.com/iBobX/RapidR",
  sponsors: "https://github.com/sponsors/iBobX",
  coffee: "https://www.buymeacoffee.com/roanbema",
};
export const CREDITS = {
  en: {
    disclosure: "Narrated by an AI-generated voice (Qwen3-TTS, Apache License 2.0, run locally). The voice was made up by the model; no recording of a real person was used.",
    voice: "Voice: AI-generated (Qwen3-TTS, Apache License 2.0)",
    madeWith: "Made with RapidR Studio",
    demoNote: "Where an AI answers in this video, a scripted demo model answers, so the recording is the same every time; a real model's answers differ.",
    music: "Music",
    chapters: "Chapters",
    credits: "Credits",
    links: "Links",
    tags: "Tags",
    site: "RapidR",
    source: "Source code",
  },
  es: {
    disclosure: "Narrado por una voz generada con IA (Qwen3-TTS, licencia Apache 2.0, ejecutado en local). La voz la inventó el modelo; no se usó ninguna grabación de una persona real.",
    voice: "Voz: generada con IA (Qwen3-TTS, licencia Apache 2.0)",
    madeWith: "Hecho con RapidR Studio",
    demoNote: "Donde una IA responde en este video, responde un modelo de demostración con guion, para que la grabación sea igual cada vez; las respuestas de un modelo real son distintas.",
    music: "Música",
    chapters: "Capítulos",
    credits: "Créditos",
    links: "Enlaces",
    tags: "Etiquetas",
    site: "RapidR",
    source: "Código fuente",
  },
};
/** A language's credits (English when there are none for it yet). */
export const creditsFor = (lang) => CREDITS[lang.split("-")[0]] || CREDITS.en;
export const COPYRIGHT = "Copyright 2026 Ruta Internet SRL";

/** The RapidR binary that runs Studio: $RAPIDR, ./rapidr, else the release build. */
export function rapidrBinary() {
  const candidates = [process.env.RAPIDR, join(ROOT, "rapidr"), process.env.CARGO_TARGET_DIR && join(process.env.CARGO_TARGET_DIR, "release", "rapidr"), join(ROOT, "target", "release", "rapidr")].filter(Boolean);
  return candidates.find((p) => existsSync(p)) || null;
}

/** The signed test-mode token a release build needs for demo mode (docs/security/test-mode.md). */
export function testModeToken() {
  const t = process.env.RAPIDR_TEST_MODE || join(homedir(), ".rapidr", "test-mode.token");
  return existsSync(t) ? t : null;
}

export const FFMPEG = process.env.FFMPEG || "/opt/homebrew/bin/ffmpeg";
export const FFPROBE = process.env.FFPROBE || "/opt/homebrew/bin/ffprobe";
