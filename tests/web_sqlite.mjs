// RSQLITE on the web is SQLite itself (rapidr-db, compiled to wasm):
//   1. A project's .db file — made here by Python's sqlite3, with what the
//      old hand-written reader couldn't read (a value longer than a page,
//      a view, an index, a table WITHOUT ROWID) — opens with SQLite's own
//      reader, and a database stays for the page's session as a file does
//      on the desktop (Connect again: 1, the rows still there; a new name:
//      0 the first time).
//   2. Widgets bound to the component (DataSource / DataField) show its
//      current row, and an edit in one goes into the database (the table
//      the column came from, the row with that id).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on http://localhost:8765):  node tests/web_sqlite.mjs

import { chromium } from "playwright";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

// The project's database file.
const dir = mkdtempSync(join(tmpdir(), "rapidr-websqlite-"));
const dbPath = join(dir, "people.db");
const made = spawnSync("python3", ["-c", `
import sqlite3, sys
c = sqlite3.connect(sys.argv[1])
c.executescript("""
CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT, city TEXT);
INSERT INTO people (name, city) VALUES ('Ann', 'Lima'), ('Bob', 'Oslo, Norway'), ('Cy', 'a=b AND c');
CREATE INDEX people_city ON people (city);
CREATE VIEW v_people AS SELECT name FROM people WHERE id > 1;
CREATE TABLE notes (k TEXT PRIMARY KEY, body TEXT) WITHOUT ROWID;
""")
c.execute("INSERT INTO notes VALUES ('long', ?)", ("x" * 20000,))
c.commit()
`, dbPath], { encoding: "utf8" });
if (made.status !== 0) {
  console.error("can't make the database:", made.stderr);
  process.exit(1);
}
const asset = { name: "people.db", mime: "application/octet-stream", dataUrl: "data:application/octet-stream;base64," + readFileSync(dbPath).toString("base64") };
rmSync(dir, { recursive: true, force: true });

const browser = await chromium.launch();
const page = await browser.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
await page.goto(`${URL_BASE}/web-ide/index.html`, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status")?.textContent?.includes("ready"), { timeout: 15000 });
await page.evaluate((a) => { window.RapidR.state.project.assets = [a]; }, asset);

const output = () => page.evaluate(() => (document.querySelector('.obody[data-tab="output"]')?.innerText || "").split("\n").map((l) => l.trim()));
async function run(lines) {
  await page.evaluate((src) => {
    window.RapidR.runCommand("run.stop");
    document.querySelector('.obody[data-tab="output"]').textContent = "";
    window.RapidR.state.project.rawSource = src;
    window.RapidR.runCommand("run.start");
  }, lines.join("\n") + "\n");
}
async function waitFor(cond, ms = 15000) {
  for (let waited = 0; waited < ms; waited += 100) {
    if (await cond()) return true;
    await page.waitForTimeout(100);
  }
  return false;
}
const frameOf = () => page.frames().find((f) => f.url().includes("preview.html"));

// 1. The project's file, read by SQLite.
await run([
  "DIM DB AS RSQLITE",
  "DIM M AS RSQLITE",
  'PRINT "connect "; DB.Connect("people.db")',
  'DB.Query("SELECT id, name, city FROM people ORDER BY id")',
  "WHILE DB.FetchRow",
  '  PRINT "row "; DB.Row(0); "|"; DB.Row(1); "|"; DB.Row(2)',
  "WEND",
  'PRINT "city "; DB.QueryScalar("SELECT name FROM people WHERE city = ?", "a=b AND c")',
  'PRINT "long "; LEN(DB.QueryScalar("SELECT body FROM notes WHERE k = ?", "long"))',
  'PRINT "view "; DB.QueryScalar("SELECT group_concat(name) FROM v_people")',
  'DB.Query("INSERT INTO people (name, city) VALUES (?, ?)", "Dan", "Rome")',
  "DB.Close",
  'PRINT "again "; DB.Connect("people.db"); " "; DB.QueryScalar("SELECT COUNT(*) FROM people")',
  'PRINT "new "; M.Connect("fresh.db")',
  'M.Query("CREATE TABLE t (a); INSERT INTO t VALUES (7)")',
  "M.Close",
  'PRINT "new again "; M.Connect("fresh.db"); " "; M.QueryScalar("SELECT a FROM t")',
  'PRINT "memory "; M.Connect(":memory:"); " "; M.QueryScalar("SELECT COUNT(*) FROM sqlite_master")',
  'PRINT "done"',
]);
ok(await waitFor(async () => (await output()).includes("done")), "the program ran");
const out = await output();
const has = (line) => out.includes(line);
ok(has("connect 1"), "Connect to the project's file: 1 (it was there)");
ok(has("row 1|Ann|Lima") && has("row 2|Bob|Oslo, Norway") && has("row 3|Cy|a=b AND c"), "its rows, as SQLite reads them");
ok(has("city Cy"), "a bound value with = and AND finds its row");
ok(has("long 20000"), "a value longer than a page (overflow pages)");
ok(has("view Bob,Cy"), "a view");
ok(has("again 1 4"), "Connect again: the database is still there, with the row added");
ok(has("new 0") && has("new again 1 7"), "a new name: 0, then 1 with its rows");
ok(has("memory 0 0"), ":memory: is a new, empty database");

// 2. Bound widgets.
await run([
  "DIM DB AS RSQLITE",
  "SUB Show",
  '  PRINT "row1 "; DB.Row(1); " db "; DB.QueryScalar("SELECT name FROM people WHERE id = 2"); " Ann "; DB.QueryScalar("SELECT name FROM people WHERE id = 1")',
  "END SUB",
  "CREATE Form AS QFORM",
  "  CREATE NameEdit AS QEDIT",
  '    DataSource = "DB"',
  '    DataField = "name"',
  "  END CREATE",
  "  CREATE Btn AS QBUTTON",
  "    Top = 40",
  "    OnClick = Show",
  "  END CREATE",
  "END CREATE",
  'DB.Connect("people.db")',
  'DB.Query("SELECT id, name FROM people ORDER BY id")',
  'PRINT "first "; NameEdit.Text',
  "DB.FetchRow",
  "DB.FetchRow",
  'PRINT "second "; NameEdit.Text',
  'PRINT "ready"',
  "Form.ShowModal",
]);
ok(await waitFor(async () => (await output()).includes("ready")), "the form program ran");
const out2 = await output();
ok(out2.includes("first Ann"), "before FetchRow the bound edit shows the first row");
ok(out2.includes("second Bob"), "after FetchRow, the current row");
const frame = frameOf();
// (the user types over the edit's text, then clicks the button: real
// input on the UI kernel's window, where the mirror's elements are)
await frame.locator("#rr-nameedit").click({ force: true });
await page.keyboard.press("ControlOrMeta+KeyA");
await page.keyboard.type("Bobby");
await frame.locator("#rr-btn").click({ force: true });
ok(await waitFor(async () => (await output()).some((l) => l.startsWith("row1 "))), "the button's handler ran");
ok((await output()).includes("row1 Bobby db Bobby Ann Ann"), "the edit went into the current row and the database (that row only)");

ok(pageErrors.length === 0, `no page errors${pageErrors.length ? ": " + pageErrors.join("; ") : ""}`);
await browser.close();
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
