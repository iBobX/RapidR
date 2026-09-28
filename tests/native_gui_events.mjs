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
//   * tests/fixtures/nested_modal.bas — timers during ShowModal, a modal form
//     opened (and closed by a timer) inside an event handler.
//
// Usage (repo root, after building ./rapidr):  node tests/native_gui_events.mjs

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

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
const cases = [
  { name: "oop_events", events: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", dump: "b1.caption,b2.caption,b3.caption",
    expect: ["b1.caption=Clicked 2", "b2.caption=Clicked 1", "b3.caption=Sender works"] },
  { name: "component_array_events", events: "btn(2).onclick,btn(3).onclick,btn(3).onclick", dump: "btn(1).caption,btn(2).caption,btn(3).caption",
    expect: ["btn(1).caption=Button1", "btn(2).caption=Hit Button2", "btn(3).caption=Hit Hit Button3"] },
  { name: "statusbar_panels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=Ready|Line 42|INS|3|150"] },
  { name: "listview_columns", events: "lv.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=2|photo.jpg|Deflated|5|3|200|Method"] },
  { name: "string_grid", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=P2|P1|Lima|6|3|64|P1|Lima|4|41|-1"] },
  { name: "align_layout", events: "btn.onclick", dump: "loose.caption,side.caption,bar.caption,status.simpletext", resize: "600,350", split: "split:60",
    expect: ["loose.caption=105,40,233,205|100|245", "side.caption=moved160|160|165", "bar.caption=600x350|373x255|538|150", "status.simpletext=433|255|5|598"] },
  { name: "list_items", events: "items.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=5|zero|four|a/b & c|3|Applepear|2|2", "lbl.caption=picked 3 four"] },
  { name: "picture_resource", events: "img.onclick,img.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=1|20x10|FF00|FF0000|80FFFF|40|FF|FFFFFF|-1", "lbl.caption=click;click;"] },
  { name: "grid_draw_cell", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=round2:25|0,130,25,194,49,two|fixed4 selected3"] },
  { name: "grid_range_list", events: "", dump: "lbl.caption",
    expect: ["lbl.caption=selected1"] },
  { name: "file_browser", events: "", dump: "lbl.caption",
    expect: ["lbl.caption=resource_files|2|hello.txt|1"] },
  { name: "canvas_onpaint", events: "btn.onclick,big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints3|form1|255|65280|220x80|36"] },
  { name: "nested_modal", events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
];
for (const c of cases) {
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
