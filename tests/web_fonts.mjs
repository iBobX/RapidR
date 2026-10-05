// The fallback fonts on the UI kernel's page (docs/web-host-plan.md §3.7,
// Stage W7): what the built-in Liberation fonts lack — Chinese, Korean, ✓ —
// is drawn with the Noto chunks beside the runtime (target/web/fonts,
// tools/fonts.py), each fetched the first time text needs it: in a label,
// an edit and a window's title. A character no font has stays a box, and
// text the built-in fonts have fetches nothing. The same in a `rapidr
// bundle-bc` bundle, which carries the fonts beside its page.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo
// served on http://localhost:8765):  node tests/web_fonts.mjs

import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as k from "./web_kernel_page.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WORK = join(ROOT, "tests/conformance/.work/web_fonts");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = `
CREATE Form AS QFORM
  Caption = "字体 test"
  Width = 360
  Height = 220
  CREATE Han AS QLABEL
    Left = 10: Top = 10: Width = 60: Height = 30
    FontSize = 16
    Caption = "中"
  END CREATE
  CREATE NoFont AS QLABEL
    Left = 80: Top = 10: Width = 60: Height = 30
    FontSize = 16
    Caption = "${String.fromCodePoint(0xE000)}"
  END CREATE
  CREATE Mixed AS QLABEL
    Left = 10: Top = 50: Width = 300: Height = 30
    Caption = "한국어 ✓ abc"
  END CREATE
  CREATE Kor AS QLABEL
    Left = 160: Top = 10: Width = 60: Height = 30
    FontSize = 16
    Caption = "국어"
  END CREATE
  CREATE NoFont2 AS QLABEL
    Left = 230: Top = 10: Width = 60: Height = 30
    FontSize = 16
    Caption = "${String.fromCodePoint(0xE000)}${String.fromCodePoint(0xE000)}"
  END CREATE
  CREATE Ed AS QEDIT
    Left = 10: Top = 90: Width = 200
    Text = "汉字"
  END CREATE
  CREATE Emo AS QLABEL
    Left = 230: Top = 90: Width = 60: Height = 40
    FontSize = 20
    Caption = "${String.fromCodePoint(0x1F600)}"
  END CREATE
  CREATE Ed2 AS QEDIT
    Left = 10: Top = 120: Width = 200
    Text = "${String.fromCodePoint(0xE000)}${String.fromCodePoint(0xE000)}"
  END CREATE
END CREATE
Form.ShowModal
`;

const browser = await chromium.launch();
/// The checks on a page `open` runs the program in (`kind`: which).
async function check(kind, open) {
  const say = (cond, msg) => ok(cond, `${kind}: ${msg}`);
  const page = await browser.newPage({ deviceScaleFactor: Number(process.env.RAPIDR_DPR || 1) });
  const errors = [];
  const fetched = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("response", (r) => { if (r.url().includes("/fonts/")) fetched.push(`${r.status()} ${r.url().split("/fonts/")[1]}`); });
  await open(page);
  await k.waitFor(page, "Han");
  // (the chunks come, then the windows are drawn again: until no more come)
  for (let i = 0, seen = -1; i < 50 && seen !== fetched.length; i++) {
    seen = fetched.length;
    await page.waitForTimeout(400);
  }
  await page.waitForTimeout(300);

  const files = fetched.map((f) => f.split(" ")[1]);
  say(fetched.every((f) => f.startsWith("200 ")), `every font file came (${fetched.join(", ")})`);
  say(files.includes("index.json"), "the index was read");
  say(files.some((f) => f.startsWith("NotoSansSC-Regular.")), "Chinese: a Noto Sans SC chunk was fetched");
  say(files.some((f) => f.startsWith("NotoSansKR-Regular.")), "Korean: a Noto Sans KR chunk was fetched");
  say(files.includes("NotoSansSymbols2-Regular.otf"), "✓: Noto Sans Symbols 2 was fetched");
  say(files.includes("Noto-COLRv1.ttf"), "😀: Noto Color Emoji was fetched");
  say(!files.includes("NotoSans-Regular.otf"), "nothing for what Liberation has (abc)");
  say(new Set(files).size === files.length, "each file once");

  /// The ink (non-background pixels) of a label: a glyph's strokes.
  const ink = async (name) => {
    const p = await k.pixels(page, name);
    if (!p) return null;
    // (the background: its most common colour — an edit's white, not its frame)
    const seen = new Map();
    for (let i = 0; i < p.data.length; i += 4) {
      const c = p.data.slice(i, i + 3).join(",");
      seen.set(c, (seen.get(c) || 0) + 1);
    }
    const bg = [...seen].sort((a, b) => b[1] - a[1])[0][0];
    const out = [];
    for (let i = 0; i < p.data.length; i += 4) out.push(p.data.slice(i, i + 3).join(",") === bg ? 0 : 1);
    return out;
  };
  const han = await ink("Han"), none = await ink("NoFont");
  const count = (a) => a?.reduce((s, v) => s + v, 0) ?? 0;
  say(han && none && han.join("") !== none.join(""), `中 is drawn as a glyph, not as the missing glyph's box (${count(han)} / ${count(none)} ink pixels)`);
  say(count(han) > 30, `中 has its strokes (${count(han)} ink pixels)`);
  say(count(await ink("Mixed")) > 100, "the Korean, the check mark and the Latin are drawn");
  const kor = await ink("Kor"), none2 = await ink("NoFont2");
  say(kor && none2 && kor.join("") !== none2.join(""), `국어 (two chunks of Noto Sans KR) is drawn, not boxes (${count(kor)} / ${count(none2)} ink pixels)`);
  const ed = await ink("Ed"), ed2 = await ink("Ed2");
  say(ed && ed2 && ed.join("") !== ed2.join(""), `an edit's Chinese text is drawn, not boxes (${count(ed)} / ${count(ed2)} ink pixels)`);
  say(await k.text(page, "Ed") === "汉字", "the edit holds its text");
  // (a colour glyph: the face's yellow, not the text's black)
  const emo = await k.pixels(page, "Emo");
  let yellow = 0;
  if (emo) for (let i = 0; i < emo.data.length; i += 4) if (emo.data[i] > 200 && emo.data[i + 1] > 150 && emo.data[i + 2] < 110) yellow++;
  say(yellow > 30, `😀 is drawn in colour (${yellow} yellow pixels)`);

  say(errors.length === 0, `no page errors (${errors.join(" / ")})`);
  await page.close();
}

// (the test page: the runtime of target/web, its fonts beside it)
await check("page", async (page) => {
  await page.goto(`${URL_BASE}/${process.env.RAPIDR_KERNEL_PAGE || "tests/web_kernel.html"}`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
  await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "fonts", {})), SOURCE);
});

// (a bundle-bc bundle: the fonts in its zip, beside its index.html)
if (!process.env.RAPIDR_KERNEL_PAGE) {
  rmSync(WORK, { recursive: true, force: true });
  mkdirSync(WORK, { recursive: true });
  writeFileSync(join(WORK, "fonts_test.bas"), SOURCE);
  execFileSync(join(ROOT, "rapidr"), ["bundle-bc", join(WORK, "fonts_test.bas"), "-o", join(WORK, "app.zip"), "--wasm", join(ROOT, "target/web/rapidrintr_bg.wasm"), "--js", join(ROOT, "target/web/rapidrintr.js")], { stdio: "ignore" });
  execFileSync("unzip", ["-q", "-o", join(WORK, "app.zip"), "-d", join(WORK, "site")]);
  ok(existsSync(join(WORK, "site/fonts/index.json")) && existsSync(join(WORK, "site/fonts/OFL.txt")), "the bundle has the fonts and their licence");
  await check("bundle", async (page) => {
    await page.goto(`${URL_BASE}/tests/conformance/.work/web_fonts/site/index.html`, { waitUntil: "load" });
  });
}

await browser.close();
if (failed) { console.log(`\nWeb fonts: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb fonts: ALL CHECKS PASSED");
