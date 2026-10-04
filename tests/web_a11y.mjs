// Accessibility in the browser from the same model as the desktop
// (docs/desktop-host-plan.md §6, Stage 12). Each GUI fixture that runs in
// the browser and on the UI kernel (tests/gui_parity_cases.mjs: `web` not
// false, `kernel: true`) runs in the web IDE's preview with its events, as
// tests/web_gui_parity.mjs runs it (tests/web_gui_run.mjs). Chrome's
// accessibility tree (CDP Accessibility.getFullAXTree: roles, names, values,
// states) is then compared with the UI kernel's for the same fixture after
// the same events (RAPIDR_TEST_A11Y's JSON) — the tree a screen reader
// gets, not the attributes that make it.
//
// The kernel's trees: <dir>/<case>.a11y.json, or the desktop matrix's
// <case>-interpreted-kernel.a11y.json, in RAPIDR_A11Y_DIR (else
// tests/conformance/.work/native_gui_events); a case without one is built
// interpreted (./rapidr) and run on the kernel here, into
// tests/conformance/.work/web_a11y.
//
// Every kernel node is looked for in the page: a component by its element
// (`#rr-<name>`) — the first node at or under it (not in another
// component) with the role ROLES maps the kernel's to; a part (a list's
// row, a tab, a tree's item, a grid's row and cell, a menu item, a status
// bar's panel) among its component's nodes of that role, in order. It
// must have the same name, value, states, numbers, level, description and
// shortcut. What's only representation is in ROLES and the notes by
// compare(); anything else is a real difference.
//
// Then the keys, on tests/fixtures/a11y_form.bas: Tab's order (TabOrder,
// TabStop), the focus ring, Alt + a caption's letter, Enter for the
// Default button, Escape for the Cancel one, the status bar's live region.
//
// Usage (repo root, after tools/build_web_artifacts.sh and ./rapidr, with
// the repo served on http://localhost:8765 or RAPIDR_URL):
//   node tests/web_a11y.mjs [filter …]

import { execFile } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { cases } from "./gui_parity_cases.mjs";
import { openIde, runCase } from "./web_gui_run.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WORK = join(ROOT, "tests/conformance/.work/web_a11y");
const MATRIX = process.env.RAPIDR_A11Y_DIR || join(ROOT, "tests/conformance/.work/native_gui_events");
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
  slider: ["slider"], spinbutton: ["spinbutton"], progressbar: ["progressbar"],
  tablist: ["tablist"], tab: ["tab"], listbox: ["listbox"],
  // (a <select>'s options, dropped down or not, are MenuListOptions)
  option: ["option", "MenuListOption"],
  combobox: ["combobox"],
  tree: ["tree"], treeitem: ["treeitem"], grid: ["grid"], row: ["row"], gridcell: ["gridcell"],
  menubar: ["menubar"], menuitem: ["menuitem"],
  img: ["image", "img"],
  // (a <canvas> is Chrome's Canvas)
  canvas: ["Canvas"],
  separator: ["separator", "splitter"],
  generic: ["generic"],
};

// ------------------------------------------------- the kernel's trees --

const run = promisify(execFile);
const ENV = {
  ...process.env,
  RAPIDR_TEST_CLIPBOARD: "1",
  // (nothing reaches a real printer or the user's registry)
  RAPIDR_PRINT_TO: process.env.RAPIDR_PRINT_TO || join(WORK, "prints"),
  RAPIDR_REGISTRY: process.env.RAPIDR_REGISTRY || join(WORK, "registry.reg"),
};

/// Case `c`'s kernel tree: the matrix's, else made here.
async function kernelTree(c) {
  for (const f of [join(MATRIX, `${c.name}.a11y.json`), join(MATRIX, `${c.name}-interpreted-kernel.a11y.json`), join(MATRIX, `${c.name}-native-kernel.a11y.json`), join(WORK, `${c.name}.a11y.json`)]) {
    if (existsSync(f)) return f;
  }
  const out = join(WORK, `${c.name}-interp`);
  const file = join(WORK, `${c.name}.a11y.json`);
  await run(join(ROOT, "rapidr"), ["build", join(ROOT, `tests/fixtures/${c.name}.bas`), out, "--interp"], { cwd: ROOT, env: ENV });
  const answer = c.fileDialog === undefined ? {} : { RAPIDR_TEST_FILE_DIALOG: c.fileDialog };
  await run(join(out, c.name), [], {
    cwd: ROOT,
    env: { ...ENV, ...answer, RAPIDR_HOST: "kernel", RAPIDR_TEST_A11Y: file, RAPIDR_CAPTURE: join(WORK, `${c.name}-window`), RAPIDR_TEST_EVENTS: c.events, RAPIDR_TEST_DUMP: c.dump, RAPIDR_TEST_RESIZE: c.resize || "", RAPIDR_TEST_SPLIT: c.split || "" },
    timeout: 60_000,
  });
  return file;
}

/// The trees in a file (ids kept whole: they're 64-bit).
function readTrees(file) {
  return JSON.parse(readFileSync(file, "utf8").replace(/"(id|labelledBy)":(\d+)/g, '"$1":"$2"'));
}

// A11y node ids (rapidr_value::objects::a11y::node_id): FNV-1a of the
// lowercase id, off the top byte, never 0.
function nodeId(name) {
  let h = 0xcbf29ce484222325n;
  for (const b of Buffer.from(name.toLowerCase(), "utf8")) {
    h ^= BigInt(b);
    h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  h &= 0x00ffffffffffffffn;
  return (h === 0n ? 1n : h).toString();
}

// ------------------------------------------------ the browser's tree --

/// The preview's accessibility tree, each node with its component (the
/// nearest element at or above it that is one) — and the components'
/// names by node id.
async function browserTree(page) {
  // (a timer changing the page between the two snapshots: taken again)
  for (let tries = 1; ; tries++) {
    const tree = await snapshot(page);
    if (tree.whole || tries === 5) return tree;
    await page.waitForTimeout(50);
  }
}

async function snapshot(page) {
  const cdp = await page.context().newCDPSession(page);
  const { frameTree } = await cdp.send("Page.getFrameTree");
  const find = (t) => (t.frame.url.includes("preview.html") ? t.frame.id : (t.childFrames || []).map(find).find(Boolean));
  const frameId = find(frameTree);
  const { root } = await cdp.send("DOM.getDocument", { depth: -1, pierce: true });
  const owner = new Map();
  const names = new Map();
  const walk = (n, comp) => {
    const attrs = {};
    for (let i = 0; i + 1 < (n.attributes || []).length; i += 2) attrs[n.attributes[i]] = n.attributes[i + 1];
    if (attrs["data-rr-name"] && attrs.id === "rr-" + attrs["data-rr-name"].toLowerCase()) {
      comp = attrs["data-rr-name"].toLowerCase();
      names.set(nodeId(comp), comp);
    }
    owner.set(n.backendNodeId, comp);
    for (const c of [...(n.children || []), ...(n.contentDocument ? [n.contentDocument] : []), ...(n.shadowRoots || [])]) walk(c, comp);
  };
  walk(root, null);
  const { nodes } = await cdp.send("Accessibility.getFullAXTree", { frameId });
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
  const whole = order.every((n) => n.ignored || n.backendDOMNodeId === undefined || owner.has(n.backendDOMNodeId));
  return { order, byId, names, whole };
}

const prop = (n, name) => (n.properties || []).find((p) => p.name === name)?.value?.value;
const squash = (s) => String(s ?? "").replace(/\s+/g, " ").trim();
const roleOf = (n) => n.role?.value;

/// The nodes of component `comp` with one of `roles`, in order (`within`:
/// only under that node).
function nodesOf(tree, comp, roles, within = null) {
  let list = tree.order;
  if (within) {
    const under = [];
    const visit = (n) => {
      if (!n) return;
      under.push(n);
      for (const c of n.childIds || []) visit(tree.byId.get(c));
    };
    for (const c of within.childIds || []) visit(tree.byId.get(c));
    list = under;
  }
  return list.filter((n) => !n.ignored && n.comp === comp && roles.includes(roleOf(n)));
}

// --------------------------------------------------------- comparing --

/// What differs between kernel node `k` and the page's node `w` (empty: the same).
function compare(k, w, opts = {}) {
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
  // character), a combo box's, a gauge's percentage. (Not after a case's
  // `__key_` events: the desktop's hook types in the focused edit, a
  // browser types nothing for the synthetic keys the test fires — the
  // program reads its keys, not the edit's text.)
  if (k.value !== undefined && ["textbox", "combobox"].includes(role) && !opts.typedIn) {
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
  // (an edit combo box's list is the browser's <datalist>: whether it's
  // open is the browser's to say, and it doesn't)
  const datalist = role === "combobox" && prop(w, "autocomplete") === "list";
  if (ke !== undefined && we !== ke && !datalist) out.push(`expanded ${ke} ≠ ${we}`);
  if (ke === undefined && we !== undefined) out.push(`expanded ${we} (kernel: not expandable)`);
  for (const [s, p] of [["readonly", "readonly"], ["multiline", "multiline"], ["modal", "modal"]]) {
    if (!!prop(w, p) !== has(s)) out.push(has(s) ? `not ${s}` : s);
  }
  if (k.level !== undefined && Number(prop(w, "level")) !== k.level) out.push(`level ${k.level} ≠ ${prop(w, "level")}`);
  if (squash(k.description) !== squash(w.description?.value)) out.push(`description "${squash(k.description)}" ≠ "${squash(w.description?.value)}"`);
  const kk = (k.shortcut || "").toLowerCase(), wk = String(prop(w, "keyshortcuts") || "").toLowerCase();
  if (kk !== wk) out.push(`shortcut "${k.shortcut || ""}" ≠ "${prop(w, "keyshortcuts") || ""}"`);
  return out;
}

/// Kernel tree `k` (a form's) against the page (`typed`: the case typed
/// keys, so the edits' texts differ); the differences.
function compareTree(tree, k, typed = false) {
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
    say(where, k.role === "label" ? (squash(k.name) === name ? [] : [`name "${squash(k.name)}" ≠ "${name}"`]) : compare(k, w, { typedIn: typed }));
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
      // (an edit combo box's items are its <datalist>'s: the browser shows
      // them only while it's open)
      if (k.role === "combobox" && role === "option" && wparts.length === 0) continue;
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
  if (!component(k)) diffs.push(`${k.role} "${k.name}": no form of that name in the page`);
  return diffs;
}

// ------------------------------------------------------------ the run --

rmSync(WORK, { recursive: true, force: true });
mkdirSync(WORK, { recursive: true });
const chosen = cases.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)));
const runnable = chosen.filter((c) => c.web !== false && c.kernel === true);
for (const c of chosen.filter((c) => !runnable.includes(c))) {
  skipped++;
  console.log(`- ${c.name}: skipped (${c.web === false ? c.why || "no browser counterpart" : "not on the kernel yet"})`);
}
// The kernel's trees, four at a time.
const files = {};
const queue = [...runnable];
await Promise.all([0, 1, 2, 3].map(async () => {
  for (let c = queue.shift(); c; c = queue.shift()) {
    try {
      files[c.name] = await kernelTree(c);
    } catch (e) {
      files[c.name] = null;
      console.log(`  (${c.name}: the kernel's tree: ${String(e.message).split("\n")[0]})`);
    }
  }
}));

const { browser, page, pageErrors } = await openIde(Number(process.env.RAPIDR_DPR || 1));
for (const c of runnable) {
  if (!files[c.name] || !existsSync(files[c.name])) { ok(false, `${c.name}: the kernel's accessibility tree`); continue; }
  const trees = readTrees(files[c.name]);
  const { frame, missing } = await runCase(page, c);
  if (!frame) { ok(false, `${c.name}: preview frame`); continue; }
  for (const target of missing) ok(false, `${c.name}: ${target} exists`);
  // (the ARIA follows a change once the program's code returns)
  await page.waitForTimeout(100);
  const tree = await browserTree(page);
  // (its `__key_` events typed in the desktop's focused edit: not the page's)
  const typed = /\.__key_/i.test(c.events);
  compared = 0;
  const diffs = trees.flatMap((t) => compareTree(tree, t, typed));
  ok(diffs.length === 0, `${c.name}: the browser's accessibility tree is the kernel's (${compared} nodes)` + (diffs.length ? "\n    " + diffs.join("\n    ") : ""));
  if (c.name === "a11y_form") await keys(page, frame);
}

/// The keys on a11y_form.bas.
async function keys(page, frame) {
  const active = () => frame.evaluate(() => (document.activeElement?.closest("[data-rr-name]")?.dataset.rrName || "").toLowerCase());
  const caption = (n) => frame.evaluate((n) => window.__rapidr_rt.rapidr_get_prop(n, "caption"), n);
  await frame.focus("#rr-edname");
  // Tab: by TabOrder (Cancel's is 2), past the label, the gauge and the
  // disabled panel, not the memo (TabStop = 0), round again.
  const order = [];
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("Tab");
    order.push(await active());
  }
  ok(order.join(",") === "btncancel,edpass,chkremember,radday,radnight,copies,btnok,edname", `a11y_form: Tab's order [got: ${order.join(",")}]`);
  await page.keyboard.press("Shift+Tab");
  ok((await active()) === "btnok", "a11y_form: Shift+Tab goes back");
  // (the focus ring shows on a component the keys focused)
  const ring = await frame.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
  ok(ring === "dotted", `a11y_form: a focus ring [got: ${ring}]`);
  // Alt + a letter: the check box clicked (focused, checked, OnClick) …
  await page.keyboard.press("Alt+KeyR");
  await page.waitForTimeout(100);
  const checked = await frame.evaluate(() => document.getElementById("rr-chkremember-cb").checked);
  ok(checked && (await active()) === "chkremember" && (await caption("LBL")).endsWith(" on"), `a11y_form: Alt+R clicks "&Remember me" [got: ${checked} ${await active()} ${await caption("LBL")}]`);
  // … a radio button; a label's: what follows it; a disabled button's: nothing
  await page.keyboard.press("Alt+KeyI");
  ok(await frame.evaluate(() => document.getElementById("rr-radnight-rb").checked), "a11y_form: Alt+I picks \"N&ight\"");
  await page.keyboard.press("Alt+KeyW");
  ok((await active()) === "edpass", "a11y_form: Alt+W (the label \"Pass&word:\") focuses its edit");
  await page.keyboard.press("Alt+KeyM");
  ok((await active()) === "edpass", "a11y_form: Alt+M (a disabled button's) does nothing");
  // Enter: the Default button; the status bar says so (a polite live region)
  await page.keyboard.press("Alt+KeyN");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(200);
  ok((await caption("LBL")).endsWith(" on ok"), `a11y_form: Enter clicks the Default button [got: ${await caption("LBL")}]`);
  const tree = await browserTree(page);
  const status = tree.order.find((n) => roleOf(n) === "status");
  const said = status ? nodesOf(tree, status.comp, ["StaticText"]).map((n) => n.name?.value).join("") : "";
  ok(said === "Signed in as Ann" && prop(status, "live") === "polite", `a11y_form: the status bar, a polite live region, says what changed [got: "${said}" ${prop(status, "live")}]`);
  // Escape: the Cancel button
  await page.keyboard.press("Escape");
  await page.waitForTimeout(200);
  ok((await caption("LBL")).endsWith(" ok cancel"), `a11y_form: Escape clicks the Cancel button [got: ${await caption("LBL")}]`);
}

ok(pageErrors.length === 0, `no page errors (${pageErrors.join("; ")})`);
await browser.close();
rmSync(WORK, { recursive: true, force: true });
console.log(`\nWeb accessibility: ${passed} checks passed, ${failed} failed, ${skipped} skipped`);
if (failed) process.exit(1);
