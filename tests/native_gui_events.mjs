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
//   * tests/fixtures/timer_default.bas — a QTIMER ticks without Enabled set.
//
// Usage (repo root, after building ./rapidr):  node tests/native_gui_events.mjs [name…]
// (only the cases whose name contains one of the arguments)

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WORK = join(ROOT, "tests/conformance/.work/native_gui_events");
const CARGO_TARGET = join(ROOT, "tests/conformance/.work/cargo-target");
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

function build(name, interp) {
  const out = join(WORK, `${name}-${interp ? "interp" : "native"}`);
  mkdirSync(out, { recursive: true });
  const args = ["build", join(ROOT, `tests/fixtures/${name}.bas`), out, ...(interp ? ["--interp"] : [])];
  execFileSync(join(ROOT, "rapidr"), args, { cwd: ROOT, stdio: "ignore", env: { ...process.env, CARGO_TARGET_DIR: CARGO_TARGET } });
  // Native builds also copy the executable next to the source; don't leave it there.
  rmSync(join(ROOT, `tests/fixtures/${name}`), { force: true });
  return interp ? join(out, name) : join(CARGO_TARGET, "debug", name);
}

function run(bin, events, dump, resize = "", split = "") {
  return execFileSync(bin, [], {
    encoding: "utf8",
    env: { ...process.env, RAPIDR_CAPTURE: join(WORK, "window"), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump, RAPIDR_TEST_RESIZE: resize, RAPIDR_TEST_SPLIT: split },
  }).split("\n").filter((l) => l.includes("=")).join("\n");
}

rmSync(WORK, { recursive: true, force: true });
const only = process.argv.slice(2);
for (const c of cases.filter((c) => !only.length || only.some((f) => c.name.includes(f)))) {
  const results = {};
  for (const interp of [false, true]) {
    const kind = interp ? "interpreted" : "native";
    const bin = build(c.name, interp);
    ok(existsSync(bin), `${c.name}: ${kind} executable built`);
    results[kind] = run(bin, c.events, c.dump, c.resize, c.split);
    for (const line of c.expect) ok(results[kind].includes(line), `${c.name} (${kind}): ${line}`);
  }
  ok(results.native === results.interpreted, `${c.name}: native and interpreted builds agree`);
}
if (failed) { console.log(`\nGUI events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGUI events: ALL CHECKS PASSED");
