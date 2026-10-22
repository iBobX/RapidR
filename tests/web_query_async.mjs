// Background queries on the web (REQUEST-001 items 2 and 3): the page has
// no threads, so RSQLITE's QueryAsync runs on its thread in slices of a few
// milliseconds, the page drawn between them. examples/data/million_rows_async.rr
// runs on the UI kernel's page with 100,000 rows (the desktop's million is
// the desktop's), driven by the desktop's own test hooks:
//   - Make data fills the table in the background; a Ping clicked meanwhile
//     is answered; the query that follows streams its rows into the RDBGrid,
//     the first ones in under 200 ms;
//   - a heading clicked runs the query again sorted (ORDER BY: the program
//     sorts a database's rows), a Ping clicked while it runs is answered;
//   - End goes to the last row;
//   - the page never stops answering for long: the longest gap between two
//     animation frames while the queries run is measured and reported;
//   - Cancel stops a query that never ends: OnError "cancelled".
// RAPIDR_SHOT=dir keeps the page's captures (BMP) to look at.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_query_async.mjs

import { chromium } from "playwright";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { URL_BASE } from "./web_gui_run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

// (the web's demo: 100,000 rows)
const source = readFileSync(join(HERE, "../examples/data/million_rows_async.rr"), "utf8").replace('Text = "1000000"', 'Text = "100000"');
ok(source.includes('Text = "100000"'), "the example, with 100,000 rows for the page");

/// The program on the kernel's page with the test hooks; the dump's lines,
/// the captures and the longest gap between two frames while it ran.
async function run(browser, events, dump, extra = "") {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
  await page.evaluate(({ source, env }) => {
    // (the frames' gaps, while the program runs)
    window.__gaps = [];
    let last = performance.now();
    const tick = (t) => { window.__gaps.push(t - last); last = t; if (!window.__stop) requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
    const bc = window.rr.compile(source, "million_rows_async.rr", {});
    window.rr.rapidr_set_test_env(env);
    window.rr.rapidr_run_bc(bc);
  }, { source: source + extra, env: { RAPIDR_THEME: "rapidr", RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: "1", RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump } });
  await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: 120000, polling: 100 });
  const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
  const gaps = await page.evaluate(() => { window.__stop = true; return window.__gaps; });
  await page.close();
  const lines = Object.fromEntries((Array.isArray(results.dump) ? results.dump : String(results.dump || "").split("\n")).filter(Boolean).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1)]));
  return { lines, results, errors, longest: Math.max(...gaps.slice(2)) };
}

const browser = await chromium.launch();

// 1. Make data, Ping meanwhile; the query; a heading's sort with a Ping; End.
{
  const { lines, results, errors, longest } = await run(
    browser,
    "makebtn.onclick,pingbtn.onclick,db.__until_busy_0,grid.__cell_1_-1,pingbtn.onclick,db.__until_busy_0,grid.__do_end",
    "status.caption,pinglabel.caption,db.rowcount,grid.row,grid.sortcolumn,grid.rowcount",
  );
  console.log("  " + Object.entries(lines).map(([k, v]) => `${k}=${v}`).join("\n  "));
  ok(errors.length === 0, `no page errors${errors.length ? ` (${errors.join("; ")})` : ""}`);
  ok(lines["db.rowcount"] === "100000" && lines["grid.rowcount"] === "100000", "the grid has the query's 100,000 rows");
  const m = /^100000 rows in (\d+) ms; the first showed after (\d+) ms\.$/.exec(lines["status.caption"] || "");
  ok(m, `OnDone said how long: ${lines["status.caption"]}`);
  if (m) {
    console.log(`  (the sorted query: ${m[1]} ms in all, its first rows after ${m[2]} ms — the sort comes first)`);
  }
  ok(/^Pings: 2/.test(lines["pinglabel.caption"] || ""), `both Pings answered: ${lines["pinglabel.caption"]}`);
  ok(/with \d+ rows in/.test(lines["pinglabel.caption"] || ""), "the second Ping was answered while the sorted query ran");
  ok(lines["grid.sortcolumn"] === "1", "the heading asked for column 1");
  ok(lines["grid.row"] === "99999", "End went to the last row");
  console.log(`  (the longest gap between two frames while it ran: ${longest.toFixed(0)} ms)`);
  ok(longest < 1500, `the page kept drawing (longest gap ${longest.toFixed(0)} ms)`);
  if (process.env.RAPIDR_SHOT) results.captures.forEach((s, i) => writeFileSync(join(process.env.RAPIDR_SHOT, `web_query_async-${i + 1}.bmp`), Buffer.from(s.bmp, "base64")));
}

// 2. The first rows of a plain query come in under 200 ms; a query without
// end, cancelled, says "cancelled". (A program of its own: it runs the
// queries when the form is shown, and keeps what it saw.)
{
  const program = [
    "$THEME rapidr",
    "DIM T AS DOUBLE, FirstMs AS INTEGER",
    "CREATE Form AS RForm",
    "  CREATE DB AS RSQLite",
    "    OnRows = GotRows",
    "    OnDone = Done",
    "    OnError = Failed",
    "  END CREATE",
    "  CREATE Seen AS RLabel",
    "  END CREATE",
    "  CREATE Last AS RLabel",
    "    Top = 30: Width = 300",
    "  END CREATE",
    "  CREATE Go AS RButton",
    "    Top = 60: OnClick = Start",
    "  END CREATE",
    "  CREATE Forever AS RButton",
    "    Top = 90: OnClick = RunForever",
    "  END CREATE",
    "  CREATE Stop AS RButton",
    "    Top = 120: OnClick = StopIt",
    "  END CREATE",
    "END CREATE",
    "SUB GotRows(Count AS INTEGER)",
    "  IF FirstMs < 0 THEN FirstMs = INT((TIMER - T) * 1000): Seen.Caption = STR$(FirstMs)",
    "END SUB",
    "SUB Done(Rows AS INTEGER, Ms AS INTEGER)",
    "  Last.Caption = \"done \" + STR$(Rows)",
    "END SUB",
    "SUB Failed(Message AS STRING)",
    "  Last.Caption = \"error \" + Message",
    "END SUB",
    "SUB Start",
    "  FirstMs = -1: T = TIMER",
    "  DB.QueryAsync \"SELECT * FROM big\"",
    "END SUB",
    "SUB RunForever",
    "  DB.QueryAsync \"WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c\"",
    "END SUB",
    "SUB StopIt",
    "  DB.Cancel",
    "END SUB",
    "DB.Connect \":memory:\"",
    "DB.Query \"CREATE TABLE big (id INTEGER PRIMARY KEY, name TEXT, amount REAL)\"",
    "DB.Query \"WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < 100000) INSERT INTO big SELECT i, 'n' || i, i / 7.0 FROM c\"",
    "Form.ShowModal",
  ].join("\n");
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
  await page.evaluate(({ source, env }) => {
    const bc = window.rr.compile(source, "first_rows.rr", {});
    window.rr.rapidr_set_test_env(env);
    window.rr.rapidr_run_bc(bc);
  }, { source: program, env: { RAPIDR_THEME: "rapidr", RAPIDR_CAPTURE: "web", RAPIDR_CAPTURE_DELAY: "1", RAPIDR_TEST_EVENTS: "go.onclick,db.__until_busy_0,forever.onclick,stop.onclick,db.__until_busy_0", RAPIDR_TEST_DUMP: "seen.caption,last.caption,db.busy" } });
  await page.waitForFunction(() => window.rr.rapidr_test_results(), null, { timeout: 120000, polling: 100 });
  const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
  await page.close();
  const lines = Object.fromEntries((Array.isArray(results.dump) ? results.dump : String(results.dump || "").split("\n")).filter(Boolean).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1)]));
  console.log("  " + Object.entries(lines).map(([k, v]) => `${k}=${v}`).join("\n  "));
  ok(errors.length === 0, `no page errors${errors.length ? ` (${errors.join("; ")})` : ""}`);
  const first = Number(lines["seen.caption"]);
  ok(first >= 0 && first < 200, `the first rows of 100,000 came after ${lines["seen.caption"]} ms (under 200)`);
  ok(lines["last.caption"] === "error cancelled" && lines["db.busy"] === "0", `Cancel stopped the query without end: ${lines["last.caption"]}`);
}

await browser.close();
console.log(failed ? `\n${failed} failed` : "\nall passed");
process.exit(failed ? 1 : 0);
