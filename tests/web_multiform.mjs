// Two forms on the web, each with its button and handler, shown and
// focused across each other (SetFocus, RapidR's; the user's report: after adding Show / Focus
// handlers across two forms, Principal showed "with no buttons at all").
// Principal's button shows Usuarios; Usuarios' button focuses Principal,
// whose button is still there. (Whether a form's SetFocus also brings it to
// the front, as the old DOM host did, is the runtimes' to decide: not
// checked here.) Real clicks on the
// windows the UI kernel draws (tests/web_kernel_page.mjs), the program on
// the runtime's own page (tests/web_run.mjs).
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_multiform.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = [
  "DECLARE SUB BtnShowUsuarios_Click",
  "DECLARE SUB BtnFocusPrincipal_Click",
  "CREATE Principal AS QFORM",
  '  Caption = "Principal"',
  "  CREATE BtnShowUsuarios AS QBUTTON",
  '    Caption = "Show Usuarios"',
  "    Left = 16: Top = 16: Width = 160: Height = 32",
  "    OnClick = BtnShowUsuarios_Click",
  "  END CREATE",
  "END CREATE",
  "CREATE Usuarios AS QFORM",
  '  Caption = "Usuarios"',
  "  Left = 300: Top = 200",
  "  CREATE BtnFocusPrincipal AS QBUTTON",
  '    Caption = "Focus Principal"',
  "    Left = 16: Top = 16: Width = 160: Height = 32",
  "    OnClick = BtnFocusPrincipal_Click",
  "  END CREATE",
  "END CREATE",
  "SUB BtnShowUsuarios_Click",
  "  Usuarios.Show()",
  "END SUB",
  "SUB BtnFocusPrincipal_Click",
  "  Principal.SetFocus",
  "END SUB",
  "Principal.ShowModal",
].join("\n");

const browser = await chromium.launch();
const r = await openRunner(browser);
const page = r.page;
await r.run(SOURCE);
await k.waitFor(page, "BtnShowUsuarios");
const shownForms = async () => (await k.windows(page)).map((w) => w.form);
ok((await shownForms()).includes("principal") && (await k.text(page, "BtnShowUsuarios")) === "Show Usuarios",
  `Principal shows its button 'Show Usuarios' (windows=${await shownForms()}, button=${await k.text(page, "BtnShowUsuarios")})`);
await k.click(page, "BtnShowUsuarios");
await page.waitForTimeout(300);
ok((await shownForms()).at(-1) === "usuarios" && (await k.text(page, "BtnFocusPrincipal")) === "Focus Principal",
  `Usuarios.Show() shows Usuarios with 'Focus Principal' (windows=${await shownForms()}, button=${await k.text(page, "BtnFocusPrincipal")})`);
await k.click(page, "BtnFocusPrincipal");
await page.waitForTimeout(300);
ok((await shownForms()).includes("principal") && (await k.text(page, "BtnShowUsuarios")) === "Show Usuarios",
  `after Principal.SetFocus both forms are shown, Principal's button still there (windows=${await shownForms()})`);
ok(r.pageErrors.length === 0, `no page errors (${r.pageErrors.join("; ")})`);
await page.screenshot({ path: "scratch/web_multiform.png" });
await browser.close();
if (failed) { console.log(`\nMulti-form: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nMulti-form: ALL CHECKS PASSED");
