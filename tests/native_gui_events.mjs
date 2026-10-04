// GUI events on the desktop, built BOTH ways — natively (`rapidr build`, the
// Rust backend) and interpreted (`rapidr build --interp`) — which must give
// the same results. Clicks are fired through the runtime's test hooks
// (RAPIDR_TEST_EVENTS / RAPIDR_TEST_DUMP / RAPIDR_CAPTURE in
// crates/rapidr-runtime-core/src/gui.rs). Needs a desktop session (FLTK).
//
//   * tests/fixtures/oop_events.bas — EVENT blocks in TYPE … EXTENDS QBUTTON
//     (This per instance) and a component-typed Sender parameter;
//   * tests/fixtures/component_array_events.bas — an array of buttons with
//     one handler bound through `Btn(i).OnClick = Clicked`;
//   * tests/fixtures/statusbar_panels.bas — QSTATUSBAR AddPanels / Panel(i);
//   * tests/fixtures/listview_columns.bas — QLISTVIEW columns, items, sub-items;
//   * tests/fixtures/string_grid.bas — QSTRINGGRID cells, rows/columns, streams;
//   * tests/fixtures/align_layout.bas — Align, and a form resized by the user;
//   * tests/fixtures/list_items.bas — QLISTBOX / QCOMBOBOX items and selection;
//   * tests/fixtures/picture_resource.bas — $RESOURCE, QIMAGE BMPHandle,
//     AutoSize, Transparent, drawing, Pixel, OnClick;
//   * tests/fixtures/grid_draw_cell.bas — QSTRINGGRID OnDrawCell (State,
//     Rect, Sender, Repaint);
//   * tests/fixtures/grid_range_list.bas — a grid with a gcsList column and
//     OnDrawCell (the browser test drags a range and picks from the list);
//   * tests/fixtures/file_browser.bas — QDIRTREE and QFILELISTBOX;
//   * tests/fixtures/canvas_onpaint.bas — QCANVAS OnPaint (form built,
//     Repaint, resize), drawn pixels, TextWidth;
//   * tests/fixtures/form_draw.bas — drawing on a QFORM in its OnPaint
//     (Pixel, TextWidth, a resize paints again);
//   * tests/fixtures/owner_list.bas — an owner-drawn QLISTBOX (OnDrawItem's
//     State and Rect, redrawn when the selection changes);
//   * tests/fixtures/dotted_paint.bas — a canvas's OnPaint handler named with
//     a dot (`bups.OnPaint = bups.paint`);
//   * tests/fixtures/nested_modal.bas — timers during ShowModal, a modal form
//     opened (and closed by a timer) inside an event handler;
//   * tests/fixtures/coolbtn_group.bas — QCOOLBTN GroupIndex / Down /
//     AllowAllUp, Down set by the program;
//   * tests/fixtures/timer_default.bas — a QTIMER ticks without Enabled set;
//   * tests/fixtures/late_parent.bas — components parented after the form is shown;
//   * tests/fixtures/mdi_children.bas — QFORMMDI child windows, OnChildClose's ChildResult.
//   * tests/fixtures/event_answers.bas — event parameters that come back:
//     OnClose's Action, OnSelectCell's CanSelect (also a TYPE's EVENT),
//     OnMeasureItem's Height.
//   * tests/fixtures/input_events.bas — key and mouse events with RapidQ's arguments.
//   * tests/fixtures/list_columns.bas — QLISTBOX Columns, an owner-drawn QCOMBOBOX.
//   * tests/fixtures/startup_modal.bas — ShowModal in the main program waits; Form.Repaint.
//   * tests/fixtures/tree_view.bas — QTREEVIEW nodes, Item(i), OnChanging / OnExpanding answers, OnDeletion.
//   * tests/fixtures/file_dialogs.bas — QOPENDIALOG / QSAVEDIALOG / QFILEDIALOG answers (RAPIDR_TEST_FILE_DIALOG).
//   * tests/fixtures/message_dialogs.bas — under the hooks SHOWMESSAGE prints and goes on, MESSAGEDLG waits (captured open).
//   * tests/fixtures/message_icons.bas — MESSAGEBOX's MB_ICONQUESTION: the icon left of the text (the web's page dialog read).
//   * tests/fixtures/color_dialog.bas — QCOLORDIALOG's Color, Style, Colors(i), OK / Cancel (RAPIDR_TEST_COLOR_DIALOG).
//   * tests/fixtures/font_dialog.bas — QFONTDIALOG's Name, Size, FontName(i), GetFont / SetFont, OK / Cancel (RAPIDR_TEST_FONT_DIALOG).
//   * tests/fixtures/header.bas — QHEADER: sections clicked and resized, an owner-drawn section.
//   * tests/fixtures/outline.bas — QOUTLINE (a tree view): AddLines by indent, AddChild(Index, S), Item(i), Row.
//   * tests/fixtures/panel_bevels.bas — QPANEL bevels; a TYPE extending QPANEL created in a form, PROPERTY SET with `.Field`.
//   * tests/fixtures/tree_edit.bas — QTREEVIEW in-place editing: OnEditing's AllowEdit, OnEdited's S, Escape, ReadOnly.
//   * tests/fixtures/dbl_clicks.bas — double clicks in the VCL's order (`__dblclick_x_y`).
//   * tests/fixtures/pause_edit.bas — a click on the selected tree node / list view item edits it after a pause.
//   * tests/fixtures/size_grip.bas — QSTATUSBAR's size grip resizes the window (OnResize, Width / Height).
//
// Usage (repo root, after building ./rapidr):  node tests/native_gui_events.mjs [name…]
// (only the cases whose name contains one of the arguments)
//
// Both desktop hosts (docs/desktop-host-plan.md §2.4): with
// RAPIDR_HOSTS=fltk,kernel each case is built natively with the kernel host
// beside FLTK (`rapidr build --host kernel`; the interpreter's runner has
// both) and every executable runs once per host (RAPIDR_HOST); the dumps of
// all runs, hosts and build kinds, must be the same. A case runs on the
// kernel only once gui_parity_cases.mjs says `kernel: true`; the others are
// counted as pending, with their reason. The kernel's runs are headless and
// also write their accessibility trees (RAPIDR_TEST_A11Y), which must be
// JSON. Without RAPIDR_HOSTS: FLTK only, as before. RAPIDR_KERNEL_TRY=1 also
// runs the pending cases on the kernel, without counting them, and says
// which pass there now (a lane's quick check before flipping `kernel:`).

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WORK = join(ROOT, "tests/conformance/.work/native_gui_events");
// (the programs use their own clipboard, never the user's)
process.env.RAPIDR_TEST_CLIPBOARD = "1";
const CARGO_TARGET = join(ROOT, "tests/conformance/.work/cargo-target");
const HOSTS = (process.env.RAPIDR_HOSTS || "").split(",").map((h) => h.trim().toLowerCase()).filter(Boolean);
const MATRIX = HOSTS.length > 0;
const TRY = MATRIX && HOSTS.includes("kernel") && !!process.env.RAPIDR_KERNEL_TRY;
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

function build(name, interp) {
  const out = join(WORK, `${name}-${interp ? "interp" : "native"}`);
  mkdirSync(out, { recursive: true });
  const host = !interp && HOSTS.includes("kernel") ? ["--host", "kernel"] : [];
  const args = ["build", join(ROOT, `tests/fixtures/${name}.bas`), out, ...(interp ? ["--interp"] : []), ...host];
  execFileSync(join(ROOT, "rapidr"), args, { cwd: ROOT, stdio: "ignore", env: { ...process.env, CARGO_TARGET_DIR: CARGO_TARGET } });
  // Native builds also copy the executable next to the source; don't leave it there.
  rmSync(join(ROOT, `tests/fixtures/${name}`), { force: true });
  return interp ? join(out, name) : join(CARGO_TARGET, "debug", name);
}

// (`colorDialog` / `fontDialog`: what the colour / font dialogs answer in
// turn, `;`-separated)
const dialogAnswers = (c) => ({
  ...(c.colorDialog === undefined ? {} : { RAPIDR_TEST_COLOR_DIALOG: c.colorDialog }),
  ...(c.fontDialog === undefined ? {} : { RAPIDR_TEST_FONT_DIALOG: c.fontDialog }),
});

function run(bin, events, dump, resize = "", split = "", fileDialog = undefined, extra = {}) {
  // (`fileDialog`: what the file dialogs answer, `a;b`)
  const answer = fileDialog === undefined ? {} : { RAPIDR_TEST_FILE_DIALOG: fileDialog };
  return execFileSync(bin, [], {
    encoding: "utf8",
    env: { ...process.env, ...answer, RAPIDR_CAPTURE: join(WORK, "window"), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump, RAPIDR_TEST_RESIZE: resize, RAPIDR_TEST_SPLIT: split, ...extra },
  }).split("\n").filter((l) => l.includes("=")).join("\n");
}

// The hosts × build kinds matrix (RAPIDR_HOSTS).
function runMatrix(c) {
  const kernelReady = c.kernel === true;
  const hosts = HOSTS.filter((h) => h !== "kernel" || kernelReady);
  if (HOSTS.includes("kernel") && !kernelReady) {
    pending.push(`${c.name}: ${typeof c.kernel === "string" ? c.kernel.replace(/^pending:\s*/, "") : "not ported to the kernel host yet"}`);
  }
  const dumps = {};
  for (const interp of [false, true]) {
    const kind = interp ? "interpreted" : "native";
    const bin = build(c.name, interp);
    ok(existsSync(bin), `${c.name}: ${kind} executable built`);
    if (TRY && !kernelReady) {
      let out;
      try {
        out = run(bin, c.events, c.dump, c.resize, c.split, c.fileDialog, { RAPIDR_HOST: "kernel", ...dialogAnswers(c) });
      } catch (e) {
        out = `(failed: ${String(e.message).split("\n")[0]})`;
      }
      tried[c.name] = (tried[c.name] || []).concat([{ kind, out, pass: c.expect.every((l) => out.includes(l)) }]);
    }
    for (const host of hosts) {
      const a11y = join(WORK, `${c.name}-${kind}-${host}.a11y.json`);
      const extra = { RAPIDR_HOST: host, ...dialogAnswers(c), ...(host === "kernel" ? { RAPIDR_TEST_A11Y: a11y } : {}) };
      let out;
      try {
        out = run(bin, c.events, c.dump, c.resize, c.split, c.fileDialog, extra);
      } catch (e) {
        out = `(failed: ${String(e.message).split("\n")[0]})`;
      }
      dumps[`${kind}/${host}`] = out;
      for (const line of c.expect) ok(out.includes(line), `${c.name} (${kind}, ${host}): ${line}` + (out.includes(line) ? "" : `\n    got: ${out.trim().split("\n").join(" / ")}`));
      if (host === "kernel") {
        let trees = null;
        try { trees = JSON.parse(readFileSync(a11y, "utf8")); } catch {}
        ok(Array.isArray(trees) && trees.length > 0 && trees.every((t) => typeof t.role === "string"), `${c.name} (${kind}, kernel): accessibility trees written`);
      }
    }
  }
  const runs = Object.keys(dumps);
  const first = dumps[runs[0]];
  ok(runs.every((r) => dumps[r] === first), `${c.name}: ${runs.join(", ")} agree` + (runs.every((r) => dumps[r] === first) ? "" : "\n    " + runs.map((r) => `${r}: ${dumps[r].trim().split("\n").join(" / ")}`).join("\n    ")));
}
const pending = [];
const tried = {};

rmSync(WORK, { recursive: true, force: true });
const only = process.argv.slice(2);
for (const c of cases.filter((c) => !only.length || only.some((f) => c.name.includes(f)))) {
  if (MATRIX) {
    runMatrix(c);
    continue;
  }
  const results = {};
  for (const interp of [false, true]) {
    const kind = interp ? "interpreted" : "native";
    const bin = build(c.name, interp);
    ok(existsSync(bin), `${c.name}: ${kind} executable built`);
    results[kind] = run(bin, c.events, c.dump, c.resize, c.split, c.fileDialog, dialogAnswers(c));
    for (const line of c.expect) ok(results[kind].includes(line), `${c.name} (${kind}): ${line}` + (results[kind].includes(line) ? "" : `\n    got: ${results[kind].trim().split("\n").join(" / ")}`));
  }
  ok(results.native === results.interpreted, `${c.name}: native and interpreted builds agree`);
}
if (MATRIX && HOSTS.includes("kernel")) {
  console.log(`\nkernel host: ${pending.length} case(s) pending`);
  for (const p of pending) {
    const name = p.split(":")[0];
    const t = tried[name];
    const note = !t ? "" : t.every((r) => r.pass) ? "  [passes on the kernel now]" : `  [kernel: ${t.filter((r) => !r.pass).map((r) => `${r.kind} got ${r.out.trim().split("\n").join(" / ")}`).join("; ")}]`;
    console.log(`  … ${p}${note}`);
  }
}
if (failed) { console.log(`\nGUI events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGUI events: ALL CHECKS PASSED");
