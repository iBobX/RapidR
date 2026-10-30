// A demo script read as demo mode reads it (crates/rapidr-ui-app/src/demo/
// script.rs): JSON with `//` comment lines, `{"en": …, "es": …}` anywhere a
// text goes, `{shown|spoken}` in narration lines.

import { readFileSync } from "node:fs";
import { dirname, isAbsolute, resolve } from "node:path";

// (the same languages as script.rs's LANGS: never a key an action uses)
const LANGS = ["en", "es", "pt", "zh", "fr", "de", "it", "ja", "ko", "nl", "ru", "ar", "hi", "ca", "gl", "eu", "pl", "tr", "uk", "sv", "da", "fi", "nb", "xx"];

export function isLang(k) {
  const [a, b = ""] = k.split("-");
  return LANGS.includes(a) && (b === "" || /^[A-Z]{2}$/.test(b) || b.length === 4);
}

export function stripComments(text) {
  return text.split("\n").map((l) => (l.trimStart().startsWith("//") ? "" : l)).join("\n");
}

/** Every language map replaced by its `lang` text (else English, else the first). */
export function resolveLang(v, lang) {
  if (Array.isArray(v)) return v.map((x) => resolveLang(x, lang));
  if (v && typeof v === "object") {
    const keys = Object.keys(v);
    if (keys.length && keys.every(isLang)) {
      const base = lang.split("-")[0];
      const k = keys.find((x) => x === lang) || keys.find((x) => x === base) || keys.find((x) => x.split("-")[0] === base) || (keys.includes("en") ? "en" : keys[0]);
      return resolveLang(v[k], lang);
    }
    return Object.fromEntries(keys.map((k) => [k, resolveLang(v[k], lang)]));
  }
  return v;
}

/** `{shown|spoken}`: what the caption shows, what the voice says. */
export function markup(text) {
  const re = /\{([^|{}]*)\|([^{}]*)\}/g;
  return { shown: text.replace(re, "$1"), spoken: text.replace(re, "$2") };
}

/** The languages a script has texts in (from its title, else English). */
export function languagesOf(raw) {
  const t = raw.title;
  return t && typeof t === "object" && !Array.isArray(t) ? Object.keys(t).filter(isLang) : ["en"];
}

/** A script file read: the raw object (all languages) and its folder. */
export function readScript(file) {
  const raw = JSON.parse(stripComments(readFileSync(file, "utf8")));
  if (!Array.isArray(raw.steps)) throw new Error(`${file}: a demo script needs "steps": [ … ]`);
  return { raw, dir: dirname(resolve(file)) };
}

/** Each narration line of a resolved script, with its step and place. */
export function linesOf(script) {
  const out = [];
  (script.steps || []).forEach((step, si) => {
    const say = step.say === undefined ? [] : Array.isArray(step.say) ? step.say : [step.say];
    say.forEach((l, li) => {
      const obj = typeof l === "string" ? { text: l } : l;
      out.push({ step: si, line: li, obj, ...markup(obj.text || "") });
    });
    if (!Array.isArray(step.say) && step.say !== undefined) step.say = say;
  });
  return out;
}

/** Card pictures' paths made absolute (the resolved script lives elsewhere). */
export function absolutePaths(script, dir) {
  for (const step of script.steps || []) {
    const c = step.card;
    if (!c) continue;
    for (const k of ["background", "logo"]) {
      if (typeof c[k] === "string" && !isAbsolute(c[k])) c[k] = resolve(dir, c[k]);
    }
  }
  return script;
}
