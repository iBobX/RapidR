// A video's description for the video site (<id>-<lang>.description.txt):
// the title, a 2-3 sentence summary, the AI-voice disclosure right under it
// (seen before "Show more"), the chapters with their times, the credits, the
// links, the tags. Pure functions, tested by tests/video_pipeline.mjs.

import { LINKS } from "./config.mjs";

/** 75 -> "1:15"; 3725 -> "1:02:05" (a video site's chapter times). */
export function clock(seconds) {
  const s = Math.max(0, Math.round(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor(s / 60) % 60;
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/**
 * The chapters: each step with a `chapter` name, at the time its first
 * narration line starts (the first chapter at 0:00: a video site wants
 * that). `steps`: the resolved script's steps; `lines`: the timeline's lines
 * ({ step (1-based), start (ms) }). A video site shows chapters only with
 * three or more, each at least 10 s long: a chapter shorter than that is
 * folded into the one before it, and fewer than three leave none.
 */
export function chapters(steps, lines) {
  const found = [];
  steps.forEach((step, i) => {
    const name = typeof step.chapter === "string" ? step.chapter.trim() : "";
    if (!name) return;
    const starts = lines.filter((l) => l.step === i + 1 && typeof l.start === "number").map((l) => l.start);
    if (!starts.length) return;
    found.push({ name, at: Math.min(...starts) / 1000 });
  });
  if (!found.length) return [];
  found[0].at = 0;
  const kept = [];
  for (const c of found) {
    if (!kept.length || c.at - kept[kept.length - 1].at >= 10) kept.push(c);
  }
  return kept.length >= 3 ? kept : [];
}

/** The description's text. */
export function describe({ title, summary, tags = [], credits, chapterList = [], music = null, demoModel = false }) {
  const out = [title, "", summary || "", "", credits.disclosure];
  if (demoModel) out.push("", credits.demoNote);
  if (chapterList.length) out.push("", `${credits.chapters}:`, ...chapterList.map((c) => `${clock(c.at)} ${c.name}`));
  out.push("", `${credits.credits}:`, credits.voice);
  if (music) out.push(`${credits.music}: ${music}`);
  out.push(`${credits.madeWith}: ${LINKS.site}`, "Copyright 2026 Ruta Internet SRL");
  out.push("", `${credits.links}:`, `${credits.site}: ${LINKS.site}`, `${credits.source}: ${LINKS.source}`, `GitHub Sponsors: ${LINKS.sponsors}`, `Buy Me a Coffee: ${LINKS.coffee}`);
  if (tags.length) out.push("", `${credits.tags}: ${tags.join(", ")}`);
  return `${out.join("\n").replace(/\n{3,}/g, "\n\n")}\n`;
}
