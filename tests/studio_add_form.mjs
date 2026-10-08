// RapidR Studio makes a program of two forms and runs it — Robert's "keep
// going until I can add new forms" (S-DESIGN-2), dogfooded on both hosts:
//
//   File > New Project (Form app) → Project > Add Form (Form2.rr, named in
//   the project tree: Enter; the main file includes it) → an RLabel, an
//   REdit and an RButton dropped on Form2, their captions set in the
//   inspector, the button's OnClick written (Form2.Close) → Form1 gets a
//   button whose OnClick shows Form2 → Save All → Run.
//
// Desktop: Studio driven through its own commands (`--do`), then the program
// it saved run as Studio's Run runs it (`rapidr run`, its own process) with
// Form1's button clicked (RAPIDR_TEST_EVENTS): both windows shown and
// captured; `--native` also builds it natively and compares the pixels.
// Web: the same steps on Studio's page; Run starts the program in its
// sandboxed frame, where the button is clicked as a user clicks it — Form2
// comes up in the frame — and the page is captured.
//
//   tools/build_studio_web.sh; python3 -m http.server -d target/studio-web <port>
//   STUDIO_WEB_URL=http://127.0.0.1:<port>/ node tests/studio_add_form.mjs [--native]
//
// Captures: tests/results/studio-add-form/.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const URL_BASE = (process.env.STUDIO_WEB_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const OUT = join(ROOT, "tests", "results", "studio-add-form");
const NATIVE = process.argv.includes("--native");

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

// The user's steps, as Studio's commands and the test steps beside them
// (ide/panels.inc PanelStep; ide/editor.inc EditorStep). {dir}: where the
// project is made.
const STEPS = [
  "newproject:gui|{dir}|Multi", "wait", "wait",
  "project.addForm", "wait", "key:Enter", "wait", "wait", "wait",
  "tool:RLABEL", "prop:Caption=Hello from Form2",
  "tool:REDIT", "prop:Text=Type here",
  "tool:RBUTTON", "prop:Caption=Close", "event:OnClick", "wait", "type:Form2.Close",
  "open:main.rr", "wait", "view.designer", "wait",
  "tool:RBUTTON", "prop:Caption=Show Form2", "event:OnClick", "wait", "type:Form2.Show",
  "file.saveAll", "wait", "wait",
];
const DUMP = ["codedoc(0).text", "codedoc(1).text", "proj.filecount", "lang.errorcount"];

const MAIN = /^\$APPTYPE GUI\n\$INCLUDE "Form2\.rr"\n[\s\S]*SUB Button2Click\n    Form2\.Show\nEND SUB[\s\S]*CREATE Form1 AS RForm[\s\S]*    CREATE Button2 AS RButton\n        Caption = "Show Form2"[\s\S]*OnClick = Button2Click/;
const FORM2 = /SUB Button1Click\n    Form2\.Close\nEND SUB[\s\S]*CREATE Form2 AS RForm[\s\S]*    CREATE Label1 AS RLabel\n        Caption = "Hello from Form2"[\s\S]*    CREATE Edit1 AS REdit\n[\s\S]*Text = "Type here"[\s\S]*    CREATE Button1 AS RButton\n        Caption = "Close"[\s\S]*OnClick = Button1Click/;

function parseDump(text) {
  const out = {};
  let cur = null;
  for (const line of text.split("\n")) {
    const key = DUMP.find((n) => line.toLowerCase().startsWith(n.toLowerCase() + "="));
    if (key) {
      cur = key;
      out[key] = line.slice(key.length + 1);
    } else if (cur && !line.startsWith("[rapidr]")) {
      out[cur] += "\n" + line;
    }
  }
  for (const key of Object.keys(out)) out[key] = out[key].replace(/\n+$/, "");
  return out;
}

function checkCode(host, d) {
  ok(MAIN.test(d["codedoc(0).text"] ?? ""), `${host}: main.rr includes Form2.rr, Form1's RButton shows Form2 (its OnClick written)`);
  ok(FORM2.test(d["codedoc(1).text"] ?? ""), `${host}: Form2.rr has the RLabel, REdit and RButton with their captions, the button's OnClick closes Form2`);
  ok(d["proj.filecount"] === "2", `${host}: the project has the two files (${d["proj.filecount"]})`);
  ok(d["lang.errorcount"] === "0", `${host}: no errors (${d["lang.errorcount"]})`);
}

const env = (extra) => ({ ...process.env, RAPIDR_PRINT_TO: join(OUT, "print"), RAPIDR_REGISTRY: join(OUT, "registry"), ...extra });

// ---- the desktop ----------------------------------------------------------
rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
{
  const dir = join(OUT, "desktop");
  mkdirSync(dir, { recursive: true });
  const r = spawnSync(RAPIDR, ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", STEPS.join(",").replaceAll("{dir}", dir)], {
    cwd: ROOT, encoding: "utf8", timeout: 180000,
    env: env({ RAPIDR_CAPTURE: join(dir, "studio"), RAPIDR_CAPTURE_DELAY: "14", RAPIDR_MENU: "window", RAPIDR_TEST_DUMP: DUMP.join(",") }),
  });
  checkCode("desktop", parseDump(r.stdout || ""));
  const project = join(dir, "Multi");
  ok(existsSync(join(project, "Form2.rr")) && existsSync(join(project, "Multi.rrproj")), "desktop: Form2.rr and Multi.rrproj saved beside main.rr");
  // Run, as Studio's Run runs it: Form1's button clicked
  const run = (exe, args, tag) => spawnSync(exe, args, {
    cwd: project, encoding: "utf8", timeout: 120000,
    env: env({ RAPIDR_CAPTURE: join(OUT, tag), RAPIDR_CAPTURE_DELAY: "2", RAPIDR_TEST_EVENTS: "button2.onclick", RAPIDR_TEST_DUMP: "form1.visible,form2.visible,label1.caption,edit1.text" }),
  });
  const shown = (r) => /form1\.visible=-1/.test(r.stdout) && /form2\.visible=-1/.test(r.stdout);
  const interp = run(RAPIDR, ["run", "main.rr"], "run");
  ok(shown(interp) && /label1\.caption=Hello from Form2/.test(interp.stdout), `desktop run: Form1's button shows Form2 (${(interp.stdout || "").trim().split("\n").filter((l) => /visible/.test(l)).join(", ")})`);
  ok(existsSync(join(OUT, "run-1.bmp")) && existsSync(join(OUT, "run-2.bmp")), "desktop run: both windows captured (run-1, run-2)");
  if (NATIVE) {
    const b = spawnSync(RAPIDR, ["build", "main.rr", join(OUT, "native-build"), "--no-bundle"], { cwd: project, encoding: "utf8", timeout: 900000, env: env({}) });
    const exe = join(project, process.platform === "win32" ? "main.exe" : "main");
    ok(b.status === 0 && existsSync(exe), "native: built");
    if (existsSync(exe)) {
      const n = run(exe, [], "native");
      ok(shown(n), "native: Form1's button shows Form2");
      for (const i of [1, 2]) {
        const same = existsSync(join(OUT, `native-${i}.bmp`)) && readFileSync(join(OUT, `native-${i}.bmp`)).equals(readFileSync(join(OUT, `run-${i}.bmp`)));
        ok(same, `native: window ${i} pixel for pixel as interpreted`);
      }
      rmSync(exe, { force: true });
    }
    rmSync(join(OUT, "native-build"), { recursive: true, force: true });
  }
}

// ---- the web --------------------------------------------------------------
const browser = await chromium.launch();
try {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript((env) => {
    if (window !== window.top) return;
    window.RAPIDR_STUDIO_TEST = env;
  }, { RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: "16", RAPIDR_TEST_DUMP: DUMP.join(",") });
  const steps = [...STEPS, "run.start", "wait", "wait"].join(",").replaceAll("{dir}", "/flows/add-form");
  const q = new URLSearchParams({ theme: "rapidr-light", window: "normal", fresh: "", do: steps });
  await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: 120000, polling: 250 });
  const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
  checkCode("web", parseDump(results.dump.join("\n")));
  // the program in Studio's run frame: Form1, its button clicked as a user does
  const frame = await (async () => {
    for (let i = 0; i < 300; i++) {
      const f = page.frames().find((f) => f.url().includes("run.html"));
      if (f) return f;
      await page.waitForTimeout(100);
    }
    return null;
  })();
  ok(!!frame, "web: Run started the program in its frame");
  if (frame) {
    await k.waitFor(frame, "Button2", 30000);
    ok(await k.shown(frame, "Form1"), "web run: Form1 shown");
    await page.screenshot({ path: join(OUT, "web-run-1.png") });
    await k.click(frame, "Button2");
    let up = false;
    for (let i = 0; i < 50 && !up; i++) {
      up = await k.shown(frame, "Form2");
      if (!up) await page.waitForTimeout(100);
    }
    ok(up, "web run: Form1's button shows Form2");
    ok((await k.text(frame, "Label1")) === "Hello from Form2", `web run: Form2's label (${await k.text(frame, "Label1")})`);
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(OUT, "web-run-2.png") });
  }
  ok(errors.length === 0, `web: no page errors${errors.length ? ` (${errors.join("; ")})` : ""}`);
} finally {
  await browser.close();
}

console.log(`\nStudio add form: ${failed ? `${failed} failed` : "all passed"} (captures: ${OUT})`);
process.exit(failed ? 1 : 0);
