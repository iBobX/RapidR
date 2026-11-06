// RapidR Studio on the web: what a link and a running program may make the
// page do (docs/security/audit-2026-11-05-web.md).
//
//   1. SEC-35: `?do=` runs Studio's commands only on the tests' pages
//      (window.RAPIDR_STUDIO_TEST set before the page loads). From a link a
//      person follows, a list with any command (a pane opened, keys typed)
//      isn't run at all; a page of the manual is.
//   2. SEC-36: the program's frame says when a button is held in it
//      (`held`: Studio stops clipping it to its windows while a window is
//      dragged). A program can say so with no press at all (a pointerdown of
//      its own making, as RJAVASCRIPT can), and never say it's released.
//      Studio takes `held` only for a press it can see — the frame has the
//      focus, and (in a real browser, not automation) the page's user
//      activation is on, which only a real press gives — and for a few
//      seconds at most: a real press un-clips the frame and its release
//      clips it again; `held` that is never released is cleared by the cap;
//      and once the user clicks into Studio (the frame loses the focus) the
//      program's `held` is ignored.
//      (The user-activation half can't be shown here: headless Chromium
//      keeps navigator.userActivation.isActive true with no gesture, so this
//      checks the cap and the focus gate — both host-observable — and
//      tests/security/web_shipping.mjs pins that the activation gate is
//      wired.)
//
// Usage (repo root, after tools/build_studio_web.sh, with the repo served on
// RAPIDR_URL, default http://localhost:8765; or Studio on
// RAPIDR_STUDIO_URL):  node tests/web_studio_safety.mjs

import { chromium } from "playwright";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const STUDIO_URL = (process.env.RAPIDR_STUDIO_URL || `${URL_BASE}/target/studio-web`).replace(/\/+$/, "");
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };
async function until(fn, ms = 20000) {
  for (let waited = 0; waited < ms; waited += 100) {
    const v = await fn();
    if (v) return v;
    await new Promise((r) => setTimeout(r, 100));
  }
  return fn();
}

const browser = await chromium.launch();
const output = (page) => page.evaluate(() => { try { return String(window.rr.rapidr_get_prop("OutputBox", "text") ?? ""); } catch { return ""; } });

/// Studio opened by `q`, the tests' gate set or not (`env`: its object).
async function studio(q, env) {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const page = await ctx.newPage();
  const warnings = [];
  const errors = [];
  page.on("console", (m) => { if (m.type() === "warning") warnings.push(m.text()); });
  page.on("pageerror", (e) => errors.push(e.message));
  if (env) {
    await page.addInitScript((env) => {
      if (window !== window.top) return;
      window.RAPIDR_STUDIO_TEST = env.test;
      window.RAPIDR_STUDIO_TEST_FILES = env.files || [];
    }, env);
  }
  await page.goto(`${STUDIO_URL}/index.html?${new URLSearchParams(q)}`, { waitUntil: "load" });
  await until(() => page.evaluate(() => !!window.RAPIDR_STUDIO_STARTED));
  return { ctx, page, warnings, errors };
}

// ── 1. SEC-35: ?do= from a link ──
{
  const steps = "view.settings,type:PRINT 1{Enter}";
  const { ctx, page, warnings } = await studio({ theme: "rapidr-light", window: "normal", fresh: "", do: steps });
  await page.waitForTimeout(2500);
  const args = await page.evaluate(() => window.RAPIDR_ARGS);
  const out = await output(page);
  ok(!args.includes("--do"), `a link's commands aren't passed to Studio (${JSON.stringify(args)})`);
  ok(!/> view\.settings/.test(out), "and Studio ran none of them (no \"> view.settings\" in Output)");
  ok(warnings.some((w) => /\?do= asks for more than a page of the manual/.test(w)), `the page says why, on its console (${JSON.stringify(warnings)})`);
  await ctx.close();
}
{
  const { ctx, page } = await studio({ theme: "rapidr-light", window: "normal", fresh: "", do: "wait,manual:getting-started.md" });
  const args = await page.evaluate(() => window.RAPIDR_ARGS);
  ok(args.join(" ").includes("--do wait,manual:getting-started.md"), `a link to a page of the manual is passed (${JSON.stringify(args)})`);
  const help = await until(async () => {
    const t = await page.evaluate(() => { try { return String(window.rr.rapidr_get_prop("HelpPage", "text") ?? ""); } catch { return ""; } });
    return t.length > 200 ? t : "";
  }, 8000);
  ok(help.length > 200, `and the Help pane shows it (${help.length} characters)`);
  await ctx.close();
}
{
  const { ctx, page } = await studio({ theme: "rapidr-light", window: "normal", fresh: "", do: "view.settings" }, { test: {} });
  const args = await page.evaluate(() => window.RAPIDR_ARGS);
  const ran = await until(async () => /> view\.settings/.test(await output(page)), 8000);
  ok(args.includes("--do") && ran, "a test's page (RAPIDR_STUDIO_TEST) still drives Studio by ?do=");
  await ctx.close();
}

// ── 2. SEC-36: the program frame's `held` ──
{
  const PROGRAM = [
    "CREATE Form AS QFORM",
    '  Caption = "Held"',
    "  Left = 300",
    "  Top = 260",
    "  Width = 420",
    "  Height = 300",
    "END CREATE",
    "Form.ShowModal",
  ].join("\n");
  const { ctx, page, errors } = await studio(
    { theme: "rapidr-light", window: "normal", fresh: "", open: "held.rr", do: "run.start" },
    { test: {}, files: [{ path: "held.rr", text: PROGRAM }] },
  );
  const rects = await until(async () => {
    const r = await page.evaluate(() => window.RAPIDR_STUDIO_RUN_RECTS || []);
    return r.length ? r : null;
  }, 30000);
  ok(!!rects, `the program's window is up (${JSON.stringify(rects)})`);
  const clip = () => page.evaluate(() => document.querySelector("#studio-run iframe")?.style.clipPath ?? "");
  const frameFocused = () => page.evaluate(() => document.activeElement === document.querySelector("#studio-run iframe"));
  const frame = page.frames().find((f) => f.url().endsWith("/run.html"));
  // (what a program can do from inside its frame with no press at all: send
  // `held: true` of its own making, as RJAVASCRIPT's postMessage can, and
  // never send `held: false`)
  const fakeHeld = () => frame.evaluate(() => parent.postMessage({ __rapidr_windows: { held: true } }, "*"));
  if (rects && frame) {
    const [x, y, w, h] = rects[0];
    ok((await clip()).startsWith("path("), `the frame is clipped to the program's windows (${await clip()})`);
    // a) a real press in the program's window, then its release
    await page.mouse.move(x + w / 2, y + h * 0.7);
    await page.mouse.down();
    const unclipped = await until(async () => (await clip()) === "none", 3000);
    ok(unclipped && (await frameFocused()), `a real press: the whole frame shows while it's held (clip ${await clip()}, the frame has the focus: ${await frameFocused()})`);
    await page.mouse.up();
    ok(await until(async () => (await clip()).startsWith("path("), 3000), "its release: clipped to the windows again");
    // b) a real press held still (no window moved) and never released: it
    // un-clips, then the cap clips it again within a few seconds even though
    // the button is still down — a motionless hold can't keep the surface
    await page.mouse.move(x + w / 2, y + h * 0.7);
    await page.mouse.down();
    const heldNow = await until(async () => (await clip()) === "none", 2000);
    const capped = await until(async () => (await clip()).startsWith("path("), 6000);
    await page.mouse.up();
    ok(heldNow && capped, `a motionless hold never released: held, then clipped by the cap (held: ${heldNow}, ${await clip()})`);
    // c) the focus gate: the user clicks into Studio (its left dock, well
    // clear of the program's window at x≥300), so the frame loses the focus;
    // the program's `held` is then ignored
    // (settle: the frame must be clipped there so the click reaches Studio;
    // a real click can race the clip restoring, so try a few times — it is
    // the focus leaving the frame that matters, not the click itself)
    await until(async () => (await clip()).startsWith("path("), 2000);
    let leftFrame = false;
    for (let i = 0; i < 5 && !leftFrame; i++) {
      await page.mouse.click(60, 400);
      leftFrame = await until(async () => !(await frameFocused()), 600);
    }
    ok(leftFrame, `a click on Studio moves the focus off the frame (active is the frame: ${await frameFocused()})`);
    await fakeHeld();
    await page.waitForTimeout(400);
    ok((await clip()).startsWith("path("), `the frame doesn't have the focus: its \`held\` is ignored, the frame stays clipped (${await clip()})`);
  }
  ok(errors.length === 0, `no page errors (${errors.join(" | ")})`);
  await ctx.close();
}

await browser.close();
if (failed) { console.log(`\nStudio web safety: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nStudio web safety: ALL CHECKS PASSED");
