// Accessibility in the browser from the same model as the desktop
// (docs/desktop-host-plan.md §6, Stage 12). The UI kernel draws the forms
// in the page (docs/web-host-plan.md; tests/web_kernel.html,
// tests/web_gui_run.mjs) and its accessibility mirror (rapidr-ui-host-web's
// mirror.rs) is what Chrome reads. Each GUI fixture that runs in the
// browser (tests/gui_parity_cases.mjs: `web` not false) runs there with its
// events, as tests/web_gui_parity.mjs runs it. Chrome's accessibility tree
// (CDP Accessibility.getFullAXTree: roles, names, values, states) is then
// compared with the kernel's for the same run (rapidr_test_results' a11y,
// the bytes RAPIDR_TEST_A11Y writes on the desktop) — the tree a screen
// reader gets, not the attributes that make it.
//
// Every kernel node has its own element in the mirror (`data-node`: its
// id): a component's is the first node at or under it (not in another
// node's element) with the role ROLES maps the kernel's to; a part (a
// list's row, a tab, a tree's item, a grid's row and cell, a menu item, a
// status bar's panel) is looked for among its component's nodes of that
// role, in order. It must have the same name, value, states, numbers,
// level, description and shortcut. What's only representation is in ROLES
// and the notes by compare(); anything else is a real difference.
//
// Then the keys, on tests/fixtures/a11y_form.bas running live (no test
// hooks: real keys to the page): Tab's order (TabOrder, TabStop), the
// focus ring, Alt + a caption's letter, Enter for the Default button,
// Escape for the Cancel one, the status bar's live region.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo
// served on http://localhost:8765 or RAPIDR_URL):
//   node tests/web_a11y.mjs [filter …]

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { cases } from "./gui_parity_cases.mjs";
import { runCaseKernel, URL_BASE } from "./web_gui_run.mjs";
import * as k from "./web_kernel_page.mjs";
import { startHttpServer } from "./http_test_server.mjs";

// (QDOWNLOAD's server: the tests' own, local — never the internet)
process.env.RAPIDR_TEST_HTTP = (await startHttpServer()).address;

const HERE = dirname(fileURLToPath(import.meta.url));
const filters = process.argv.slice(2);
let failed = 0, passed = 0, skipped = 0, compared = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); cond ? passed++ : failed++; };

// The kernel's roles (ARIA's names, as AccessNode::to_json writes them) and
// what Chrome calls the node a screen reader gets for each.
const ROLES = {
  // (ARIA has no window: a form is a dialog — aria-modal while ShowModal
  // waits, which the kernel calls a dialog)
  window: ["dialog"], dialog: ["dialog"],
  // (ARIA has no pane: a panel, a scroll box is a group)
  pane: ["group"], group: ["group"],
  // (a polite live region)
  status: ["status"],
  // (a label is the text it shows)
  label: ["StaticText"],
  // (a toggle button: aria-pressed for the kernel's checked)
  button: ["button"],
  checkbox: ["checkbox"], radio: ["radio"], textbox: ["textbox"],
  // (the output console's text: a <textarea>, Chrome's textbox)
  "textbox-multiline": ["textbox"],
  slider: ["slider"], spinbutton: ["spinbutton"], progressbar: ["progressbar"],
  tablist: ["tablist"], tab: ["tab"], listbox: ["listbox"], option: ["option"],
  combobox: ["combobox"],
  tree: ["tree"], treeitem: ["treeitem"], grid: ["grid"], row: ["row"], gridcell: ["gridcell"],
  menubar: ["menubar"], menuitem: ["menuitem"],
  img: ["image", "img"],
  // (a <canvas> is Chrome's Canvas)
  canvas: ["Canvas"],
  separator: ["separator", "splitter"],
  generic: ["generic"],
};

// ------------------------------------------------ the browser's tree --

/// The page's accessibility tree, each node with its kernel node (the
/// nearest mirror element at or above it, `data-node`) — and the kernel
/// nodes the mirror has.
async function snapshot(page) {
  const cdp = await page.context().newCDPSession(page);
  const { root } = await cdp.send("DOM.getDocument", { depth: -1, pierce: true });
  const owner = new Map();
  const names = new Map();
  const walk = (n, comp) => {
    const attrs = {};
    for (let i = 0; i + 1 < (n.attributes || []).length; i += 2) attrs[n.attributes[i]] = n.attributes[i + 1];
    if (attrs["data-node"]) {
      comp = attrs["data-node"];
      names.set(comp, comp);
    }
    owner.set(n.backendNodeId, comp);
    for (const c of n.children || []) walk(c, comp);
  };
  walk(root, null);
  const { nodes } = await cdp.send("Accessibility.getFullAXTree");
  await cdp.detach();
  const byId = new Map(nodes.map((n) => [n.nodeId, n]));
  // (in tree order)
  const order = [];
  const visit = (n) => {
    if (!n) return;
    order.push(n);
    for (const c of n.childIds || []) visit(byId.get(c));
  };
  visit(nodes.find((n) => !n.parentId));
  for (const n of order) n.comp = owner.get(n.backendDOMNodeId) ?? null;
  return { order, byId, names };
}

const prop = (n, name) => (n.properties || []).find((p) => p.name === name)?.value?.value;
const squash = (s) => String(s ?? "").replace(/\s+/g, " ").trim();
const roleOf = (n) => n.role?.value;

/// The nodes under `node` in the page's tree, in order.
function under(tree, node) {
  const out = [];
  const visit = (n) => {
    if (!n) return;
    out.push(n);
    for (const c of n.childIds || []) visit(tree.byId.get(c));
  };
  for (const c of node.childIds || []) visit(tree.byId.get(c));
  return out;
}

/// The nodes of component `comp` with one of `roles`, in order (`within`:
/// only under that node).
function nodesOf(tree, comp, roles, within = null) {
  const list = within ? under(tree, within) : tree.order;
  return list.filter((n) => !n.ignored && n.comp === comp && roles.includes(roleOf(n)));
}

// --------------------------------------------------------- comparing --

/// What differs between kernel node `k` and the page's node `w` (empty: the same).
function compare(k, w) {
  const out = [];
  const states = new Set(k.states || []);
  const has = (s) => states.has(s);
  const tri = (s) => (has(s) ? true : has("not-" + s) ? false : undefined);
  const role = k.role;
  // (ARIA names a row by its cells' text; the kernel names the cells)
  const name = squash(w.name?.value);
  if (squash(k.name) !== name && role !== "row") out.push(`name "${squash(k.name)}" ≠ "${name}"`);
  // Values: a text box's text (its line breaks a textarea's "\n"; a
  // password's by its length: the browser masks it with its own
  // character), a combo box's, a gauge's percentage.
  if (k.value !== undefined && ["textbox", "textbox-multiline", "combobox"].includes(role)) {
    const kv = k.value.replace(/\r\n?/g, "\n");
    const wv = String(w.value?.value ?? "");
    const same = kv === wv || /^(\*+|•+)$/.test(kv) && [...kv].length === [...wv].length;
    if (!same) out.push(`value ${JSON.stringify(kv)} ≠ ${JSON.stringify(wv)}`);
  }
  // (a gauge's percentage is its aria-valuetext)
  if (k.value !== undefined && role === "progressbar" && String(prop(w, "valuetext") ?? "") !== k.value) out.push(`value "${k.value}" ≠ "${prop(w, "valuetext")}"`);
  if (k.numeric) {
    const num = (v) => (v === undefined || v === "" ? undefined : Number(v));
    const wmin = num(prop(w, "valuemin")), wmax = num(prop(w, "valuemax"));
    const wnow = num(w.value?.value);
    if (wmin !== k.numeric.min || wmax !== k.numeric.max) out.push(`range ${k.numeric.min}..${k.numeric.max} ≠ ${wmin}..${wmax}`);
    if (wnow !== undefined && wnow !== k.numeric.value) out.push(`value ${k.numeric.value} ≠ ${wnow}`);
  }
  if (!!prop(w, "disabled") !== has("disabled")) out.push(has("disabled") ? "not disabled" : "disabled");
  // (a toggle button's checked: aria-pressed)
  const checked = role === "button" ? prop(w, "pressed") : prop(w, "checked");
  const kc = tri("checked");
  if (kc !== undefined && String(checked) !== String(kc)) out.push(`checked ${kc} ≠ ${checked}`);
  if (kc === undefined && checked !== undefined && checked !== "false" && role !== "button") out.push(`checked ${checked} (kernel: not checkable)`);
  const ks = tri("selected");
  if (ks !== undefined && !!prop(w, "selected") !== ks) out.push(`selected ${ks} ≠ ${!!prop(w, "selected")}`);
  const ke = tri("expanded");
  const we = prop(w, "expanded");
  if (ke !== undefined && we !== ke) out.push(`expanded ${ke} ≠ ${we}`);
  if (ke === undefined && we !== undefined) out.push(`expanded ${we} (kernel: not expandable)`);
  // (ARIA has no read-only row — aria-readonly is for text boxes, grids and
  // combo boxes —: the inspector's read-only rows are the kernel's alone)
  for (const [s, p] of [["readonly", "readonly"], ["multiline", "multiline"], ["modal", "modal"]]) {
    if (s === "readonly" && role === "row") continue;
    if (!!prop(w, p) !== has(s)) out.push(has(s) ? `not ${s}` : s);
  }
  if (k.level !== undefined && Number(prop(w, "level")) !== k.level) out.push(`level ${k.level} ≠ ${prop(w, "level")}`);
  if (squash(k.description) !== squash(w.description?.value)) out.push(`description "${squash(k.description)}" ≠ "${squash(w.description?.value)}"`);
  const kk = (k.shortcut || "").toLowerCase(), wk = String(prop(w, "keyshortcuts") || "").toLowerCase();
  if (kk !== wk) out.push(`shortcut "${k.shortcut || ""}" ≠ "${prop(w, "keyshortcuts") || ""}"`);
  return out;
}

/// Kernel tree `k` (a window's) against the page; the differences.
function compareTree(tree, k) {
  const diffs = [];
  // (each node compared counts one)
  const say = (where, list) => { compared++; list.forEach((d) => diffs.push(`${where}: ${d}`)); };
  const component = (k) => {
    const comp = tree.names.get(k.id);
    if (!comp) return false;
    const roles = ROLES[k.role] || [k.role];
    const where = `${k.role} ${comp}`;
    let w;
    let name;
    if (k.role === "label") {
      // (a label: its text, all of it)
      const texts = nodesOf(tree, comp, ["StaticText"]);
      name = squash(texts.map((t) => t.name?.value ?? "").join(""));
      w = texts[0] || { name: { value: "" } };
    } else {
      w = nodesOf(tree, comp, roles)[0];
    }
    if (!w) {
      const seen = [...new Set(tree.order.filter((n) => n.comp === comp && !n.ignored).map(roleOf))];
      diffs.push(`${where}: not in the page as ${roles.join(" / ")} (it has: ${seen.join(", ") || "nothing"})`);
      return true;
    }
    say(where, k.role === "label" ? (squash(k.name) === name ? [] : [`name "${squash(k.name)}" ≠ "${name}"`]) : compare(k, w));
    // Its parts, by role, in order.
    const parts = (k.children || []).filter((c) => !tree.names.has(c.id));
    const byRole = new Map();
    for (const p of parts) byRole.set(p.role, [...(byRole.get(p.role) || []), p]);
    for (const [role, kparts] of byRole) {
      let wparts = nodesOf(tree, comp, ROLES[role] || [role]).filter((n) => n !== w);
      let kp = kparts;
      // (a status bar's panels: the texts they show; an empty one shows none)
      if (role === "label") {
        kp = kparts.filter((p) => squash(p.name));
        wparts = wparts.filter((n) => squash(n.name?.value));
      }
      if (kp.length !== wparts.length) {
        diffs.push(`${where}: ${kp.length} ${role}(s) ≠ ${wparts.length} in the page`);
        continue;
      }
      kp.forEach((p, i) => {
        say(`${where} ${role} ${i}`, compare(p, wparts[i]));
        // (a grid's row's cells)
        if (role === "row") {
          const cells = nodesOf(tree, comp, ROLES.gridcell, wparts[i]);
          if ((p.children || []).length !== cells.length) diffs.push(`${where} row ${i}: ${(p.children || []).length} cells ≠ ${cells.length}`);
          else (p.children || []).forEach((cell, j) => say(`${where} cell ${j},${i}`, compare(cell, cells[j])));
        }
      });
    }
    for (const c of k.children || []) if (tree.names.has(c.id)) component(c);
    return true;
  };
  if (!component(k)) diffs.push(`${k.role} "${k.name}": not in the page's mirror`);
  return diffs;
}

// ------------------------------------------------------------ the run --

const browser = await chromium.launch();
const dpr = Number(process.env.RAPIDR_DPR || 1);
let same = 0, differ = 0;
const chosen = cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)));
for (const c of chosen) {
  if (c.web === false) { skipped++; console.log(`- ${c.name}: skipped (${c.why || "no browser counterpart"})`); continue; }
  const { results, errors, page } = await runCaseKernel(browser, c, dpr);
  if (!results) { ok(false, `${c.name} (kernel): ran (${errors.join("; ")})`); await page.close(); continue; }
  // (the mirror follows the last frame)
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  // (ids kept whole: they're 64-bit)
  const trees = JSON.parse(results.a11y.replace(/"(id|labelledBy)":(\d+)/g, '"$1":"$2"'));
  const tree = await snapshot(page);
  compared = 0;
  const diffs = trees.flatMap((t) => compareTree(tree, t));
  diffs.length ? differ++ : same++;
  ok(diffs.length === 0, `${c.name} (kernel): the browser's accessibility tree is the kernel's (${compared} nodes)` + (diffs.length ? "\n    " + diffs.slice(0, 12).join("\n    ") : ""));
  await page.close();
}
if (chosen.some((c) => c.name === "a11y_form")) await keys();
await browser.close();
console.log(`\nKernel host: Chrome's tree equal to the kernel's for ${same} of ${same + differ} cases`);
console.log(`Web accessibility (kernel host): ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
process.exit(failed ? 1 : 0);

/// The keys on a11y_form.bas: the program running live in its own page
/// (no test hooks), the keys real ones; the focused component is the
/// mirror element that has the page's focus (`#rr-<name>`).
async function keys() {
  const page = await browser.newPage({ deviceScaleFactor: dpr });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${URL_BASE}/tests/web_kernel.html`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
  const source = readFileSync(join(HERE, "fixtures/a11y_form.bas"), "utf8");
  await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "a11y_form", {})), source);
  await k.waitFor(page, "Lbl");
  await page.waitForFunction(() => window.rr.rapidr_get_prop("LBL", "caption") === "ready", null, { timeout: 15000 });
  const active = () => page.evaluate(() => {
    const a = document.activeElement;
    return a?.closest(".rr-a11y") && a.dataset.rrName ? a.dataset.rrName.toLowerCase() : `(${a?.tagName.toLowerCase()})`;
  });
  const caption = () => k.prop(page, "LBL", "caption");
  const checked = (name) => page.evaluate((n) => document.getElementById("rr-" + n)?.getAttribute("aria-checked"), name);
  // (the device pixels at a component's place, to compare)
  const drawn = async (name) => (await k.pixels(page, name)).data.join(",");
  /// Whether pixels `b` are `a` with a ring drawn on them: every pixel that
  /// differs on the edge of the box the differences make (up to four logical
  /// pixels thick, `s` device pixels each: RapidR's look draws the theme's
  /// focus width, 2, and a thin ring inside it on the Default button's
  /// accent), the box most of the component's size.
  const ring = (a, b) => {
    const pa = a.split(","), pb = b.split(",");
    const { width, height } = okBox, s = Math.ceil(okBox.scale) * 4;
    const diff = [];
    for (let i = 0; i < pa.length; i += 4) if (pa[i] !== pb[i] || pa[i + 1] !== pb[i + 1] || pa[i + 2] !== pb[i + 2]) diff.push([(i / 4) % width, Math.floor(i / 4 / width)]);
    if (!diff.length) return false;
    const [l, t] = [Math.min(...diff.map((d) => d[0])), Math.min(...diff.map((d) => d[1]))];
    const [r, btm] = [Math.max(...diff.map((d) => d[0])), Math.max(...diff.map((d) => d[1]))];
    return diff.every(([x, y]) => x - l < s || r - x < s || y - t < s || btm - y < s) && r - l > width / 2 && btm - t > height / 2;
  };
  const okBox = await k.pixels(page, "BtnOK");
  await k.click(page, "EdName");
  await page.waitForTimeout(100);
  ok((await active()) === "edname", `a11y_form: a click focuses the edit [got: ${await active()}]`);
  // Tab: by TabOrder (Cancel's is 1), past the label, the gauge and the
  // disabled panel, not the memo (TabStop = 0), round again.
  const order = [];
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("Tab");
    order.push(await active());
  }
  ok(order.join(",") === "btncancel,edpass,chkremember,radday,radnight,copies,btnok,edname", `a11y_form: Tab's order [got: ${order.join(",")}]`);
  // (the OK button, the Default one, unfocused: the edit has the keys)
  await page.waitForTimeout(100);
  const okPlain = await drawn("BtnOK");
  await page.keyboard.press("Shift+Tab");
  await page.waitForTimeout(100);
  ok((await active()) === "btnok", "a11y_form: Shift+Tab goes back");
  // The focus ring the kernel draws: a ring on the button when the keys
  // focus it (nothing else of it changes), gone once the focus has left.
  const okRing = await drawn("BtnOK");
  await page.keyboard.press("Tab");
  await page.waitForTimeout(100);
  const okLeft = await drawn("BtnOK");
  await page.keyboard.press("Shift+Tab");
  await page.waitForTimeout(100);
  ok(ring(okPlain, okRing) && okLeft === okPlain && (await drawn("BtnOK")) === okRing && (await active()) === "btnok",
    `a11y_form: a focus ring on the component the keys focused [ring drawn: ${ring(okPlain, okRing)}, gone after: ${okLeft === okPlain}]`);
  // Alt + a letter: the check box clicked (focused, checked, OnClick) …
  await page.keyboard.press("Alt+KeyR");
  await page.waitForTimeout(100);
  ok((await checked("chkremember")) === "true" && (await active()) === "chkremember" && (await caption()).endsWith(" on"),
    `a11y_form: Alt+R clicks "&Remember me" [got: ${await checked("chkremember")} ${await active()} ${await caption()}]`);
  // … a radio button; a label's: what follows it; a disabled button's: nothing
  await page.keyboard.press("Alt+KeyI");
  await page.waitForTimeout(100);
  ok((await checked("radnight")) === "true" && (await checked("radday")) === "false", `a11y_form: Alt+I picks "N&ight" [got: ${await checked("radnight")}]`);
  await page.keyboard.press("Alt+KeyW");
  await page.waitForTimeout(100);
  ok((await active()) === "edpass", `a11y_form: Alt+W (the label "Pass&word:") focuses its edit [got: ${await active()}]`);
  await page.keyboard.press("Alt+KeyM");
  await page.waitForTimeout(100);
  ok((await active()) === "edpass", `a11y_form: Alt+M (a disabled button's) does nothing [got: ${await active()}]`);
  // Enter: the Default button; the status bar says so (a polite live region)
  await page.keyboard.press("Alt+KeyN");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(200);
  ok((await caption()).endsWith(" on ok"), `a11y_form: Enter clicks the Default button [got: ${await caption()}]`);
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const tree = await snapshot(page);
  const status = tree.order.find((n) => roleOf(n) === "status");
  const said = status ? under(tree, status).filter((n) => roleOf(n) === "StaticText").map((n) => n.name?.value).join("") : "";
  ok(said === "Signed in as Ann" && prop(status, "live") === "polite", `a11y_form: the status bar, a polite live region, says what changed [got: "${said}" ${status && prop(status, "live")}]`);
  // Escape: the Cancel button
  await page.keyboard.press("Escape");
  await page.waitForTimeout(200);
  ok((await caption()).endsWith(" ok cancel"), `a11y_form: Escape clicks the Cancel button [got: ${await caption()}]`);
  ok(errors.length === 0, `a11y_form: no page errors (${errors.join("; ")})`);
  await page.close();
}
