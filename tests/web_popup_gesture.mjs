// A pop-up menu RapidR draws (SystemMenu = 0) that the program opens with
// PopupAt from the handler of the gesture the person is making, on the web,
// with real input (page.mouse, page.keyboard) — tests/fixtures/
// popup_from_click.bas:
//
//   * a button's OnClick: the menu drops under it and is still open when
//     the click is over;
//   * a label's OnMouseDown: the menu opens while the button is down; the
//     release — a few pixels off, over the menu's first item — keeps it
//     open and picks nothing; the next click on an item picks it;
//   * pressed, dragged to an item and let go: a pick, as a system menu does;
//   * a label's OnMouseUp, a right click's OnContextMenu, Shift+F10 and the
//     Menu key (the same OnContextMenu, Key 1): open after;
//   * a click anywhere else closes it, and nothing under it hears the click.
//
// The same cases run on the desktop through the test hooks (the
// "popup_from_click" case of tests/gui_parity_cases.mjs); the kernel's own
// tests (components/popupmenu.rs) pin the rule.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765):  node tests/web_popup_gesture.mjs

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";
import { openRunner } from "./web_run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const source = readFileSync(join(HERE, "fixtures", "popup_from_click.bas"), "utf8");
const browser = await chromium.launch();
const r = await openRunner(browser, { viewport: { width: 1000, height: 700 } });
// (RapidQ's look, named: the menu's border is the classic one's 3 px, so a
// release 4 px off is over the first item)
await r.run(source, { env: { RAPIDR_THEME: "classic" } });
await k.waitFor(r.page, "dots");
await r.page.waitForTimeout(300);

const items = () => r.page.locator('[role="menuitem"]').count();
const said = async () => String(await k.prop(r.page, "said", "caption")).trim();
const settle = () => r.page.waitForTimeout(250);
const at = async (name, dx, dy) => {
  const b = await k.rect(r.page, name);
  return { x: b.x + dx, y: b.y + dy };
};
const centerOfItem = async (n) => {
  const b = await r.page.locator('[role="menuitem"]').nth(n).boundingBox();
  return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
};

// The button's OnClick: still open when the click is over; its letter picks.
let p = await at("dots", 20, 10);
await r.page.mouse.click(p.x, p.y, { delay: 80 });
await settle();
ok((await items()) === 2, "OnClick: the menu is open once the click is over");
await r.page.keyboard.press("a");
await settle();
ok((await items()) === 0 && (await said()) === "A", `OnClick: a letter picks Alpha (${await said()})`);

// A label's OnMouseDown: pressed, let go 4 px away over the first item.
p = await at("downlbl", 10, 10);
await r.page.mouse.move(p.x, p.y);
await r.page.mouse.down();
await settle();
ok((await items()) === 2, "OnMouseDown: the menu opens while the button is down");
await r.page.mouse.move(p.x + 4, p.y + 4);
await r.page.mouse.up();
await settle();
ok((await items()) === 2 && (await said()) === "A", `OnMouseDown: the release keeps it open and picks nothing (${await said()})`);
await r.page.keyboard.press("b");
await settle();
ok((await items()) === 0 && (await said()) === "A B", `OnMouseDown: it was open to be picked from (${await said()})`);

// Pressed, dragged to the second item, let go: a pick.
p = await at("downlbl", 10, 10);
await r.page.mouse.move(p.x, p.y);
await r.page.mouse.down();
await settle();
const beta = await centerOfItem(1);
await r.page.mouse.move(beta.x, beta.y, { steps: 6 });
await r.page.mouse.up();
await settle();
ok((await items()) === 0 && (await said()) === "A B B", `a press dragged to an item and let go picks it (${await said()})`);

// A label's OnMouseUp: it opens as the button is let go.
p = await at("uplbl", 10, 10);
await r.page.mouse.click(p.x, p.y, { delay: 80 });
await settle();
ok((await items()) === 2, "OnMouseUp: the menu is open");
await r.page.keyboard.press("a");
await settle();
ok((await items()) === 0 && (await said()) === "A B B A", `OnMouseUp: a letter picks (${await said()})`);

// A right click's OnContextMenu.
p = await at("ctxlbl", 10, 10);
await r.page.mouse.click(p.x, p.y, { button: "right", delay: 60 });
await settle();
ok((await items()) === 2, "OnContextMenu: the menu is open");
await r.page.keyboard.press("a");
await settle();
ok((await items()) === 0 && (await said()) === "A B B A A", `OnContextMenu: a letter picks (${await said()})`);

// Shift+F10 and the Menu key on the focused label: its OnContextMenu, the
// menu open with its first item chosen; Enter picks it.
p = await at("ctxlbl", 10, 10);
await r.page.mouse.click(p.x, p.y, { delay: 60 });
await r.page.keyboard.press("Shift+F10");
await settle();
ok((await items()) === 2, "Shift+F10: the menu is open");
await r.page.keyboard.press("Enter");
await settle();
ok((await items()) === 0 && (await said()) === "A B B A A A", `Shift+F10: Enter picks the first item (${await said()})`);
await r.page.keyboard.press("ContextMenu");
await settle();
ok((await items()) === 2, "the Menu key: the menu is open");
await r.page.keyboard.press("Enter");
await settle();
ok((await items()) === 0 && (await said()) === "A B B A A A A", `the Menu key: Enter picks the first item (${await said()})`);

// The button again: a click on the menu's second item picks it.
p = await at("dots", 20, 10);
await r.page.mouse.click(p.x, p.y, { delay: 80 });
await settle();
const item = await centerOfItem(1);
await r.page.mouse.click(item.x, item.y, { delay: 60 });
await settle();
ok((await items()) === 0 && (await said()) === "A B B A A A A B", `a click on an item picks it (${await said()})`);

// And a click anywhere else closes it, picking nothing.
await r.page.mouse.click(p.x, p.y, { delay: 80 });
await settle();
ok((await items()) === 2, "the button opens it again");
const away = await at("said", 200, 8);
await r.page.mouse.click(away.x, away.y, { delay: 60 });
await settle();
ok((await items()) === 0 && (await said()) === "A B B A A A A B", `a click elsewhere closes it (${await said()})`);

const errs = [...r.errs, ...r.pageErrors].filter(Boolean);
ok(errs.length === 0, `no errors${errs.length ? ": " + errs.join("; ") : ""}`);
await browser.close();
console.log(failed ? `\n${failed} failed` : "\nWeb pop-up gesture: ALL CHECKS PASSED");
process.exit(failed ? 1 : 0);
