// Runs the RapidQ example corpus in the browser (the web IDE's wasm compiler
// and VM) and on the desktop interpreter (`rapidr build --interp`), and
// compares what each prints and the forms and components each shows — the
// web counterpart of tools/corpus_compare.mjs (ROADMAP: the same comparison
// against the web runtime).
//
//   node tests/corpus_web_compare.mjs <corpus.json> [examples dir] [include dir] [filter,…]
//
// Only programs the report lists as compiling are tried; programs using the
// network are skipped. Each program's folder is copied to a work directory
// (programs write files); the browser gets the program preprocessed by
// `rapidr preprocess` (its $INCLUDEs resolved the desktop's way) and the
// folder's files as the project's assets. What's compared:
//   output      PRINT output (ANSI escapes left out)
//   forms       which forms show, their Caption and ClientWidth/Height
//   components  each CREATEd component's Left/Top/Width/Height and whether
//               it shows, as the program reads them (desktop:
//               RAPIDR_TEST_DUMP at the window capture; web: the runtime's
//               rapidr_get_prop once the forms have settled)
// Needs a desktop session, ./rapidr, the web artifacts and the repo served on
// http://localhost:8765. Writes tests/conformance/.work/corpus-web/report.json.

import { chromium } from "playwright";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [jsonPath, examplesArg, includeArg, filter] = process.argv.slice(2);
if (!jsonPath) {
  console.error("usage: node tests/corpus_web_compare.mjs <corpus.json> [examples dir] [include dir] [filter]");
  process.exit(2);
}
const EXAMPLES = examplesArg || join(homedir(), "Downloads/Rapidq/examples");
const INCLUDE = includeArg || join(homedir(), "Downloads/Rapidq/include");
const WORK = join(ROOT, "tests/conformance/.work/corpus-web");
const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
const RUN_SECONDS = 20;
// How long a shown form is left before it's looked at (the desktop
// capture's RAPIDR_CAPTURE_DELAY).
const SETTLE_SECONDS = 2;
const NETWORK = /\b(qsocket|qclientsocket|qserversocket|qmysql|qhttp|qftp|qsmtp|qpop3|qwebbrowser|inet)\b/i;
const VARIES = /\b(rnd|randomize|timer|time\$|date\$|tickcount|gettickcount|now|sleep)\b/i;
// Programs showing where they run (the program's file name, the current
// folder): the page and the desktop differ there by nature.
const MACHINE = /application\.exename|\bcurdir\$/i;
// Assets larger than this stay out of the browser project.
const ASSET_LIMIT = 8 << 20;

const PRINTS = join(ROOT, "tests/conformance/.work/corpus-prints");
mkdirSync(PRINTS, { recursive: true });
// (RAPIDR_MENU=window: a QMAINMENU inside its form on macOS too, as on
// Windows and in the browser, so ClientHeight compares)
const env = { ...process.env, RAPIDR_INCLUDE_PATH: INCLUDE, RAPIDR_PRINT_TO: PRINTS, RAPIDR_REGISTRY: join(ROOT, "tests/conformance/.work/corpus-registry.reg"), RAPIDR_MENU: "window" };

const results = JSON.parse(readFileSync(jsonPath, "utf8")).results;
const wanted = filter ? filter.split(",").map((f) => f.trim()).filter(Boolean) : [];
const programs = Object.keys(results).filter((k) => !results[k] || results[k].length === 0).filter((k) => !wanted.length || wanted.some((f) => k.includes(f))).sort();
mkdirSync(WORK, { recursive: true });

const stripAnsi = (s) => s.replace(/\x1b\[[0-9;?]*[A-Za-z]/g, "").replace(/\x1b[()][A-Z0-9]/g, "");
const norm = (s) => stripAnsi(s).replace(/\r\n?/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").replace(/^\n+|\n+$/g, "");

// The program's forms and components (`CREATE x AS QFORM`, `DIM x AS QBUTTON`)
// by name, lower-case: what both sides are asked about.
function components(source) {
  const found = new Map();
  const re = /^\s*(?:CREATE|DIM)\s+([A-Za-z_][A-Za-z0-9_]*)\s+AS\s+(Q[A-Za-z]+)\b(?!\s*\()/gim;
  for (const m of source.matchAll(re)) {
    const type = m[2].toUpperCase();
    // (only visual components; arrays and objects without a window aren't asked)
    if (/^Q(FORM|BUTTON|LABEL|EDIT|RICHEDIT|LISTBOX|COMBOBOX|CHECKBOX|RADIOBUTTON|GROUPBOX|PANEL|IMAGE|CANVAS|STRINGGRID|LISTVIEW|TREEVIEW|TABCONTROL|TRACKBAR|SCROLLBAR|SCROLLBOX|STATUSBAR|COOLBTN|OVALBTN|GAUGE|FILELISTBOX|DIRTREE|SPLITTER|HEADER|PROGRESSBAR|UPDOWN)$/.test(type)) {
      found.set(m[1].toLowerCase(), type);
    }
  }
  return found;
}

function interp(src, dir, out, comps) {
  const b = spawnSync(join(ROOT, "rapidr"), ["build", src, out, "--interp", "--no-bundle"], { cwd: dir, env, encoding: "utf8", timeout: 300_000 });
  if (b.status !== 0) return { error: (b.stderr || b.stdout || "").split("\n").filter((l) => /error/i.test(l)).slice(0, 3).join(" | ") || `exit ${b.status}` };
  const props = [];
  for (const [name, type] of comps) {
    props.push(`${name}.__shown`);
    if (type === "QFORM") props.push(`${name}.caption`, `${name}.clientwidth`, `${name}.clientheight`);
    else props.push(`${name}.left`, `${name}.top`, `${name}.width`, `${name}.height`);
  }
  const capDir = `${dir}.captures`;
  rmSync(capDir, { recursive: true, force: true });
  mkdirSync(capDir, { recursive: true });
  const r = spawnSync(join(out, basename(src, extname(src))), [], {
    cwd: dir, input: "", encoding: "utf8", timeout: RUN_SECONDS * 1000, killSignal: "SIGKILL",
    env: { ...env, RAPIDR_CAPTURE: join(capDir, "cap"), RAPIDR_CAPTURE_DELAY: String(SETTLE_SECONDS), RAPIDR_TEST_DUMP: props.join(",") },
  });
  const dump = {};
  const lines = [];
  for (const l of (r.stdout || "").split("\n")) {
    if (l.startsWith("[rapidr] captured window")) continue;
    const m = /^([a-z_][a-z0-9_]*)\.(__shown|caption|clientwidth|clientheight|left|top|width|height)=(.*)$/.exec(l);
    if (m && comps.has(m[1])) (dump[m[1]] ??= {})[m[2]] = m[3];
    else lines.push(l);
  }
  return { stdout: norm(lines.join("\n").replaceAll(dir, "<dir>")), stderr: (r.stderr || "").slice(0, 400), timedOut: r.error?.code === "ETIMEDOUT", dump, windows: readdirSync(capDir).length };
}

// The browser gets the desktop's screen (Screen.Width / Height): programs
// size their forms from it.
const screenProbe = join(WORK, "screen.bas");
writeFileSync(screenProbe, "PRINT Screen.Width; \",\"; Screen.Height\n");
spawnSync(join(ROOT, "rapidr"), ["build", screenProbe, join(WORK, "screen"), "--interp", "--no-bundle"], { env, encoding: "utf8" });
const [screenW, screenH] = (spawnSync(join(WORK, "screen", "screen"), [], { env, encoding: "utf8" }).stdout || "1280,720").split(",").map((n) => parseInt(n, 10) || 0);
const browser = await chromium.launch();
let page, logs = [], pageErrors = [];
// A program that never yields (a busy loop in the VM) blocks the page: every
// call into it is bounded, and a blocked page is replaced by a fresh one.
// (Other failures — a frame gone while the program restarts — fall back.)
const CALL_MS = 5000;
const bounded = (p, ms = CALL_MS) => Promise.race([p, new Promise((_, reject) => setTimeout(() => reject(new Error("page unresponsive")), ms))]);
async function openPage() {
  if (page) await page.close({ runBeforeUnload: false }).catch(() => {});
  page = await browser.newPage({ screen: { width: screenW || 1280, height: screenH || 720 } });
  page.on("pageerror", (e) => pageErrors.push(e.message));
  // The IDE's Output panel is written through console.log in host.js
  // (logOutput); its Errors panel through console.error/warn.
  page.on("console", (m) => {
    if (!/host\.js/.test(m.location()?.url || "")) return;
    logs.push({ type: m.type(), text: m.text() });
  });
  page.on("dialog", (d) => d.dismiss().catch(() => {}));
  await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
  await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
}
await openPage();

async function web(source, dir, comps) {
  const assets = [];
  const walk = (d) => {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      const st = statSync(p);
      if (st.isDirectory()) walk(p);
      else if (st.size <= ASSET_LIMIT && !/\.(bas|exe|dll|zip|rar)$/i.test(f)) {
        assets.push({ name: relative(dir, p).replaceAll("\\", "/"), mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(p).toString("base64") });
      }
    }
  };
  walk(dir);
  await bounded(page.evaluate(() => window.RapidR.runCommand("run.stop")));
  await page.waitForTimeout(150);
  logs = [];
  pageErrors = [];
  await bounded(page.evaluate(({ src, assets }) => {
    window.RapidR.state.project.assets = assets;
    window.RapidR.state.project.rawSource = src;
    window.RapidR.runCommand("run.start");
  }, { src: source, assets }));
  let firstForm = null, ended = false, waited = 0;
  for (; waited < RUN_SECONDS * 1000; waited += 250) {
    await page.waitForTimeout(250);
    const status = await bounded(page.evaluate(() => document.getElementById("status")?.textContent || ""));
    if (/compile failed|compile error/.test(status)) break;
    const frame = page.frames().find((f) => f.url().includes("preview.html"));
    const shown = frame ? await bounded(frame.evaluate(() => [...document.querySelectorAll(".rr-form")].some((f) => f.offsetWidth > 0)).catch(() => false)) : false;
    // An INPUT: the desktop's runs read an empty stdin, so an empty line.
    // A SHOWMESSAGE: under the desktop's test hooks it's printed and the
    // program goes on — so here, OK.
    if (frame) {
      const message = await bounded(frame.evaluate(() => {
        // (not a file dialog: the desktop's capture shows that as a window)
        const dialog = document.querySelector(".rr-dialog:not(.rr-file-dialog)");
        if (!dialog) return null;
        const text = dialog.querySelector(".rr-dialog-input") ? null : dialog.querySelector(".rr-dialog-text")?.textContent ?? "";
        dialog.querySelector(".rr-dialog-button")?.click();
        return text;
      }).catch(() => null));
      if (message !== null) logs.push({ type: "log", text: `[SHOWMESSAGE] ${message}` });
    }
    if (shown && firstForm === null) firstForm = waited;
    // (its main code done, no form showing: a console program's exit)
    if (!shown && frame && await bounded(frame.evaluate(() => window.__rapidr_rt?.rapidr_main_done?.() ?? false).catch(() => false))) { ended = true; break; }
    if (firstForm !== null && waited - firstForm >= SETTLE_SECONDS * 1000) break;
  }
  const status = await bounded(page.evaluate(() => document.getElementById("status")?.textContent || ""));
  const frame = page.frames().find((f) => f.url().includes("preview.html"));
  // (read as the program reads them: the runtime's rapidr_get_prop)
  const dump = frame ? await bounded(frame.evaluate((names) => {
    const rt = window.__rapidr_rt;
    const out = {};
    for (const [name, type] of names) {
      const props = type === "QFORM" ? ["__shown", "caption", "clientwidth", "clientheight"] : ["__shown", "left", "top", "width", "height"];
      out[name] = Object.fromEntries(props.map((p) => [p, rt.rapidr_get_prop(name, p)]));
    }
    return out;
  }, [...comps]).catch(() => ({}))) : {};
  const text = logs.filter((l) => l.type === "log" && !l.text.startsWith("------ source ------") && l.text !== "[RapidR] Program ended.").map((l) => l.text).join("\n");
  const errors = logs.filter((l) => l.type === "error" || l.type === "warning").map((l) => l.text).concat(pageErrors);
  return { stdout: norm(text), errors, status, ended, timedOut: !ended && firstForm === null, dump };
}

const report = [];
for (const rel of programs) {
  const src0 = join(EXAMPLES, rel);
  const text = readFileSync(src0, "latin1");
  const name = rel.replace(/[\/ ]/g, "_").replace(/\.bas$/i, "");
  const entry = { program: rel };
  report.push(entry);
  if (NETWORK.test(text)) {
    entry.result = "skipped (network)";
    console.log(`- ${rel}: ${entry.result}`);
    continue;
  }
  const dir = join(WORK, name);
  rmSync(dir, { recursive: true, force: true });
  cpSync(dirname(src0), dir, { recursive: true, filter: (s) => !/\.(exe|zip|rar|dll)$/i.test(s) });
  const src = join(dir, basename(rel));
  const pre = spawnSync(join(ROOT, "rapidr"), ["preprocess", src], { cwd: dir, env, encoding: "utf8", timeout: 60_000 });
  if (pre.status !== 0) {
    entry.result = "preprocess fails";
    entry.detail = (pre.stderr || "").slice(0, 300);
    console.log(`✗ ${rel}: ${entry.result}`);
    continue;
  }
  // `rapidr preprocess` turns each `$RESOURCE NAME AS "file"` into a CONST
  // (the desktop build embeds the file): the browser's compiler gets the
  // directive back, and the file from the assets.
  const resources = new Map();
  for (const f of readdirSync(dir).filter((f) => /\.(bas|inc)$/i.test(f))) {
    for (const m of readFileSync(join(dir, f), "latin1").matchAll(/^\s*\$RESOURCE\s+([A-Za-z_][A-Za-z0-9_]*)\s+AS\s+("[^"]*")/gim)) resources.set(m[1].toLowerCase(), `$RESOURCE ${m[1]} AS ${m[2]}`);
  }
  pre.stdout = pre.stdout.replace(/^CONST ([A-Za-z_][A-Za-z0-9_]*) = 6\d{4}$/gm, (line, name) => resources.get(name.toLowerCase()) ?? line);
  const comps = components(pre.stdout);
  const desk = interp(src, dir, join(dir, "interp"), comps);
  if (desk.error) {
    entry.result = "interpreted build fails";
    entry.detail = desk.error;
    console.log(`✗ ${rel}: ${entry.result}: ${desk.error}`);
    continue;
  }
  let w;
  try {
    w = await web(pre.stdout, dir, comps);
  } catch (e) {
    // (a program that never ends on the desktop either — `cpuhog: GOTO
    // cpuhog` — is the same, though it blocks the page)
    entry.result = desk.timedOut ? "same (never ends)" : "web hangs";
    entry.detail = `${e.message} (the VM never yields to the page)`;
    console.log(`${desk.timedOut ? "✓" : "✗"} ${rel}: ${entry.result}`);
    await openPage();
    continue;
  }
  const diffs = [];
  if (/compile/.test(w.status) && /fail|error/.test(w.status)) diffs.push(`web compile: ${w.errors.slice(0, 2).join(" | ")}`);
  if (desk.stdout !== w.stdout) diffs.push("output");
  if (desk.timedOut !== w.timedOut) diffs.push(`timeout (desktop ${desk.timedOut}, web ${w.timedOut})`);
  let checked = 0;
  for (const [n] of comps) {
    const a = desk.dump[n] || {}, b = w.dump[n] || {};
    if ((a.__shown ?? "0") === "0" && (b.__shown ?? "0") === "0") continue;
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) {
      checked++;
      if ((a[k] ?? "") !== (b[k] ?? "")) diffs.push(`${n}.${k}: ${a[k] ?? "-"} vs ${b[k] ?? "-"}`);
    }
  }
  const runtimeErrors = w.errors.filter((e) => !/^\s*$/.test(e));
  entry.checked = checked;
  entry.forms = [...comps].filter(([n, t]) => t === "QFORM" && desk.dump[n]?.__shown === "1").length;
  entry.result = !diffs.length ? (runtimeErrors.length ? "same (web errors)" : "same") : VARIES.test(text) ? "differs (random / clock)" : MACHINE.test(text) ? "differs (machine)" : "differs";
  if (diffs.length || runtimeErrors.length) {
    entry.diffs = diffs;
    entry.webErrors = runtimeErrors.slice(0, 5);
    entry.desktop = { stdout: desk.stdout.slice(0, 600), stderr: desk.stderr };
    entry.web = { stdout: w.stdout.slice(0, 600), status: w.status };
  }
  console.log(`${diffs.length ? "✗" : "✓"} ${rel}: ${entry.result}${diffs.length ? " — " + diffs.slice(0, 6).join("; ") + (diffs.length > 6 ? ` (+${diffs.length - 6})` : "") : ""}${checked ? ` (${checked} properties)` : ""}${runtimeErrors.length ? ` [web errors: ${runtimeErrors[0].slice(0, 120)}]` : ""}`);
  rmSync(join(dir, "interp"), { recursive: true, force: true });
}
await browser.close();

writeFileSync(join(WORK, "report.json"), JSON.stringify(report, null, 1));
const count = (r) => report.filter((e) => e.result === r).length;
console.log(`\nsame=${count("same") + count("same (web errors)") + count("same (never ends)")} (with web errors ${count("same (web errors)")}, never ending ${count("same (never ends)")}) differs=${count("differs")} varies=${count("differs (random / clock)")} machine=${count("differs (machine)")} skipped=${report.filter((e) => e.result.startsWith("skipped")).length} failures=${report.filter((e) => /fails|hangs/.test(e.result)).length} of ${report.length}`);
console.log(`report: ${join(WORK, "report.json")}`);
