#!/usr/bin/env node
// A run's transcript.jsonl as readable lines:
//   node tools/ai_eval/show_transcript.mjs RUN_DIR [--full]
import { readFileSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
const full = process.argv.includes("--full");
if (!dir) { console.error("usage: show_transcript.mjs RUN_DIR [--full]"); process.exit(2); }
const cut = (s, n) => (full || String(s).length <= n ? String(s) : String(s).slice(0, n) + "…");
const one = (s) => String(s ?? "").replace(/\s+/g, " ");
for (const line of readFileSync(join(dir, "transcript.jsonl"), "utf8").split("\n").filter(Boolean)) {
  const e = JSON.parse(line);
  switch (e.type) {
    case "user": console.log(`[${e.t}s] USER: ${cut(one(e.text), 200)}`); break;
    case "model":
      console.log(`[${e.t}s] MODEL #${e.step} (${e.inTok}/${e.outTok} tok, ${e.secs}s, stop ${e.stop}${e.format !== "native" ? `, ${e.format}` : ""}${e.thinkingChars ? `, thinking ${e.thinkingChars} chars` : ""}) ${e.text ? "SAYS: " + cut(one(e.text), 300) : ""}${e.calls.length ? " CALLS: " + e.calls.map((c) => `${c.name}(${c.argBytes}B${c.badArgs ? " BAD ARGS " + c.badArgs : ""})`).join(", ") : ""}`);
      break;
    case "tool":
      console.log(`   -> ${e.name} ${cut(one(e.args), full ? 100000 : 160)}\n      ${e.ok ? "ok" : "ERROR"}${e.protocolError ? " (PROTOCOL)" : ""} ${e.resultBytes}B ${cut(one(e.ok ? e.result : e.error), full ? 100000 : 260)}`);
      break;
    case "model_error": console.log(`[${e.t}s] MODEL ERROR #${e.step}: ${e.error}`); break;
    case "end": console.log(`[${e.t}s] END: ${e.stopped} ${e.detail || ""} · score ${e.score} · ${e.clickTest}`); break;
    default: break;
  }
}
