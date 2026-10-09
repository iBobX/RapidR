// GUI events on the desktop, built BOTH ways — natively (`rapidr build`, the
// Rust backend) and interpreted (`rapidr build --interp`) — which must give
// the same results. Clicks are fired through the runtime's test hooks
// (RAPIDR_TEST_EVENTS / RAPIDR_TEST_DUMP / RAPIDR_CAPTURE in
// crates/rapidr-ui-app/src/testhooks.rs) on the UI kernel's
// headless host: no desktop session needed (RAPIDR_CAPTURE_WINDOWS=1 shows
// real windows instead).
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
//   * tests/fixtures/message_icons.bas — MESSAGEBOX's MB_ICONQUESTION: the icon left of the text (the web's: its window's pixels and tree, as the desktop's).
//   * tests/fixtures/color_dialog.bas — QCOLORDIALOG's Color, Style, Colors(i), OK / Cancel (RAPIDR_TEST_COLOR_DIALOG).
//   * tests/fixtures/font_dialog.bas — QFONTDIALOG's Name, Size, FontName(i), GetFont / SetFont, OK / Cancel (RAPIDR_TEST_FONT_DIALOG).
//   * tests/fixtures/window_state.bas — QFORM.WindowState: maximized (the work area, OnResize), restored, minimized.
//   * tests/fixtures/header.bas — QHEADER: sections clicked and resized, an owner-drawn section.
//   * tests/fixtures/outline.bas — QOUTLINE (a tree view): AddLines by indent, AddChild(Index, S), Item(i), Row.
//   * tests/fixtures/panel_bevels.bas — QPANEL bevels; a TYPE extending QPANEL created in a form, PROPERTY SET with `.Field`.
//   * tests/fixtures/tree_edit.bas — QTREEVIEW in-place editing: OnEditing's AllowEdit, OnEdited's S, Escape, ReadOnly.
//   * tests/fixtures/dbl_clicks.bas — double clicks in the VCL's order (`__dblclick_x_y`).
//   * tests/fixtures/pause_edit.bas — a click on the selected tree node / list view item edits it after a pause.
//   * tests/fixtures/size_grip.bas — QSTATUSBAR's size grip resizes the window (OnResize, Width / Height).
//   * tests/fixtures/a11y_form.bas — what a screen reader is told (its tree and keys: tests/web_a11y.mjs).
//   * tests/fixtures/menu_hold_timers.bas — timers tick while a native menu holds the window system (`__hold_ms`).
//   * tests/fixtures/dialog_timers.bas — timers tick while message boxes and file / colour / font dialogs wait
//     (RAPIDR_TEST_DIALOG_HOLD), nested ones too; their answers come back.
//   * tests/fixtures/dx_screen.bas — QDXSCREEN (OnInitialize, Flip, Pixel, Fill's colours), QDXIMAGELIST (a .DXG), QDXTIMER; the capture's pixels.
//   * tests/fixtures/dx_more.bas — QDXSCREEN's font, Rotate, View.*, a screen put on a shown form, a hidden form's, FullScreen; QDXTIMER's ActiveOnly.
//   * tests/fixtures/dx_sound.bas — QDXSOUND: a WAV's Size and Frequency, Play / Stop, Playing and Position by the clock, Looped, the end.
//   * tests/fixtures/d3d_scene.bas — Direct3D: frames, faces, lights, the camera, Render (the software rasterizer), Move, CameraLookAt.
//   * tests/fixtures/d3d_xfile.bas — Direct3D: a .X model (frame matrix, materials, a texture on one face), SetRGB, SetTexture.
//   * tests/fixtures/dx_joystick.bas — QDXJOYSTICK: Update, IsLeft …, Button(n); X, Buttons, POV; OnButtonDown / OnButtonUp / OnMove (the case's `joystick` script).
//   * tests/fixtures/bevel_display.bas — QBEVEL and QDIGDISPLAY built in: Shape / Style → bevels and edge lines, Display → size and segments; the capture's pixels.
//   * tests/fixtures/tray_icon.bas — the system tray: QNOTIFYICONDATA, Shell_NotifyIcon, the form's WndProc (`form.__tray_N`).
//   * tests/fixtures/dir_list_view.bas — QDIRLISTVIEW (RapidR's library): a folder listed, into it, up, OnFileSelect (`inWork`).
//   * tests/fixtures/dock_form.bas — QDOCKFORM (RapidR's library): docked, floated, docked at its alternative, closed.
//   * tests/fixtures/themes.bas — the kernel's themes: Application.Theme at run time, and the form captured
//     under each theme (`themes`: RAPIDR_THEME, <case>-<theme>-<kind>-1.bmp in the work directory).
//   * tests/fixtures/rplot_on_form.bas — RPLOT on a form (the kernel's chart): Anchors and Align, a series added,
//     Title set, Render; the capture's pixels.
//
// Usage (repo root, after building ./rapidr):  node tests/native_gui_events.mjs [name…]
// (only the cases whose name contains one of the arguments)
//
// Each run also writes its accessibility trees (RAPIDR_TEST_A11Y) as
// <case>-native.a11y.json / <case>-interpreted.a11y.json in the work
// directory, which must be JSON (tests/web_a11y.mjs compares the browser's
// tree with them).

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync, readFileSync, copyFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cases } from "./gui_parity_cases.mjs";
import { dropBuild } from "./cargo_builds.mjs";
import { startHttpServer } from "./http_test_server.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WORK = join(ROOT, "tests/conformance/.work/native_gui_events");
// (the programs use their own clipboard, never the user's)
process.env.RAPIDR_TEST_CLIPBOARD = "1";
// (QDOWNLOAD's server: the tests' own, local — never the internet; and no
// real serial port: QCOMPORT's scripted ones only, none unless a case's)
process.env.RAPIDR_TEST_HTTP = (await startHttpServer()).address;
process.env.RAPIDR_TEST_COMPORT = "";
// (QMIDI: no MIDI output; QWAVE records the scripted tone, never a
// microphone — the GUI tests have no sound device either)
process.env.RAPIDR_TEST_MIDI = "";
process.env.RAPIDR_TEST_WAVE_IN = "tone:440";
const CARGO_TARGET = join(ROOT, "tests/conformance/.work/cargo-target");
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

// Building on one machine and running on another (a VM without Rust:
// tools/vm/): `RAPIDR_BUILD_ONLY=dir` builds every case's executables into
// `dir` (<case>-native / <case>-interp) and runs nothing; `RAPIDR_PREBUILT=dir`
// runs the ones found there instead of building (a kind not there is skipped).
const BUILD_ONLY = process.env.RAPIDR_BUILD_ONLY;
const PREBUILT = process.env.RAPIDR_PREBUILT;
// (`RAPIDR_KINDS=interp` or `native`: only that build kind)
// (Windows: programs are .exe)
const EXE = process.platform === "win32" ? ".exe" : "";
const KINDS = [false, true].filter((i) => !process.env.RAPIDR_KINDS || process.env.RAPIDR_KINDS.includes(i ? "interp" : "native"));

function build(name, interp) {
  if (PREBUILT) return join(PREBUILT, `${name}-${interp ? "interp" : "native"}${EXE}`);
  const out = join(WORK, `${name}-${interp ? "interp" : "native"}`);
  mkdirSync(out, { recursive: true });
  // (native: a debug build, quick to compile; either kind's executable in
  // its output folder, the generated Rust in a build cache beside the target)
  const args = ["build", join(ROOT, `tests/fixtures/${name}.bas`), out, "--no-bundle", ...(interp ? ["--interp"] : ["--debug"])];
  execFileSync(join(ROOT, `rapidr${EXE}`), args, { cwd: ROOT, stdio: "ignore", env: { ...process.env, CARGO_TARGET_DIR: CARGO_TARGET, RAPIDR_BUILD_CACHE: join(dirname(CARGO_TARGET), "build-cache") } });
  return join(out, `${name}${EXE}`);
}

// (`colorDialog` / `fontDialog` / `messageDialog`: what the colour / font
// dialogs and message boxes answer in turn, `;`-separated; `dialogHold`:
// how many ms each answered dialog stays open first, waited for as the
// user's; `delay`: seconds before the test's script starts)
const dialogAnswers = (c) => ({
  ...(c.colorDialog === undefined ? {} : { RAPIDR_TEST_COLOR_DIALOG: c.colorDialog }),
  ...(c.fontDialog === undefined ? {} : { RAPIDR_TEST_FONT_DIALOG: c.fontDialog }),
  ...(c.messageDialog === undefined ? {} : { RAPIDR_TEST_MESSAGE_DIALOG: c.messageDialog }),
  ...(c.dialogHold === undefined ? {} : { RAPIDR_TEST_DIALOG_HOLD: String(c.dialogHold) }),
  ...(c.delay === undefined ? {} : { RAPIDR_CAPTURE_DELAY: String(c.delay) }),
  // (`joystick`: QDXJOYSTICK's gamepad, the tests' script)
  ...(c.joystick === undefined ? {} : { RAPIDR_TEST_JOYSTICK: c.joystick }),
  // (`comport`: QCOMPORT's scripted ports)
  ...(c.comport === undefined ? {} : { RAPIDR_TEST_COMPORT: c.comport }),
  // (`drop`: the files `form.__drop` drops on a form, OnDropFiles)
  ...(c.drop === undefined ? {} : { RAPIDR_TEST_DROP: c.drop }),
});

// A captured window's pixel (x, y) as "rrggbb" (an uncompressed 24- or
// 32-bit BMP).
function capturePixel(file, x, y) {
  try {
    const b = readFileSync(file);
    const off = b.readUInt32LE(10), w = b.readInt32LE(18), h = b.readInt32LE(22), bpp = b.readUInt16LE(28) / 8;
    const stride = (w * bpp + 3) & ~3;
    const row = h > 0 ? h - 1 - y : y;
    const i = off + row * stride + x * bpp;
    return [b[i + 2], b[i + 1], b[i]].map((v) => v.toString(16).padStart(2, "0")).join("");
  } catch (e) {
    return "(no capture)";
  }
}

function run(bin, events, dump, resize = "", split = "", fileDialog = undefined, extra = {}, cwd = undefined) {
  // (`fileDialog`: what the file dialogs answer, `a;b`)
  const answer = fileDialog === undefined ? {} : { RAPIDR_TEST_FILE_DIALOG: fileDialog };
  return execFileSync(bin, [], {
    // (`inWork`: the program's files — a folder it makes — in the work
    // directory, not the checkout)
    ...(cwd ? { cwd } : {}),
    encoding: "utf8",
    // (RapidQ's look, named: the cases check pixels against RC.EXE's —
    // the classic theme, as `$THEME classic` asks for it)
    env: { RAPIDR_THEME: "classic", ...process.env, ...answer, RAPIDR_CAPTURE: join(WORK, "window"), RAPIDR_TEST_EVENTS: events, RAPIDR_TEST_DUMP: dump, RAPIDR_TEST_RESIZE: resize, RAPIDR_TEST_SPLIT: split, ...extra },
  }).split("\n").filter((l) => l.includes("=")).join("\n");
}

rmSync(WORK, { recursive: true, force: true });
const only = process.argv.slice(2);
for (const c of cases.filter((c) => !only.length || only.some((f) => c.name.includes(f)))) {
  // (`headlessOnly`: what the case checks is the headless host's own
  // simulation, which real windows — RAPIDR_CAPTURE_WINDOWS — can't repeat)
  if (c.headlessOnly && process.env.RAPIDR_CAPTURE_WINDOWS && !BUILD_ONLY) {
    console.log(`- ${c.name}: skipped with real windows (${c.headlessOnly})`);
    continue;
  }
  const results = {};
  for (const interp of KINDS) {
    const kind = interp ? "interpreted" : "native";
    const bin = build(c.name, interp);
    if (BUILD_ONLY) {
      mkdirSync(BUILD_ONLY, { recursive: true });
      copyFileSync(bin, join(BUILD_ONLY, `${c.name}-${interp ? "interp" : "native"}${EXE}`));
      continue;
    }
    if (PREBUILT && !existsSync(bin)) continue;
    ok(existsSync(bin), `${c.name}: ${kind} executable built`);
    const a11y = join(WORK, `${c.name}-${kind}.a11y.json`);
    let out;
    try {
      out = run(bin, c.events, c.dump, c.resize, c.split, c.fileDialog, { ...dialogAnswers(c), RAPIDR_TEST_A11Y: a11y }, c.inWork ? WORK : undefined);
    } catch (e) {
      out = `(failed: ${String(e.message).split("\n")[0]})`;
    }
    results[kind] = out;
    for (const line of c.expect) ok(out.includes(line), `${c.name} (${kind}): ${line}` + (out.includes(line) ? "" : `\n    got: ${out.trim().split("\n").join(" / ")}`));
    // (`pixels`: the window as captured)
    if (c.pixels) {
      // (the capture's device pixels a logical one: RAPIDR_SCALE headless,
      // the screen's with real windows — from the window's client width)
      const capWidth = (() => { try { return readFileSync(join(WORK, "window-1.bmp")).readInt32LE(18); } catch { return 0; } })();
      const scale = c.clientWidth && capWidth ? capWidth / c.clientWidth : Number(process.env.RAPIDR_SCALE || 1);
      const got = c.pixels.map(([x, y]) => capturePixel(join(WORK, "window-1.bmp"), Math.floor((x + 0.5) * scale), Math.floor((y + 0.5) * scale)));
      const want = c.pixels.map((p) => p[2]);
      ok(got.join(",") === want.join(","), `${c.name} (${kind}): captured pixels ${want.join(",")}` + (got.join(",") === want.join(",") ? "" : `   [got: ${got.join(",")}]`));
    }
    let trees = null;
    try { trees = JSON.parse(readFileSync(a11y, "utf8")); } catch {}
    ok(Array.isArray(trees) && trees.length > 0 && trees.every((t) => typeof t.role === "string"), `${c.name} (${kind}): accessibility trees written`);
    // (kernel themes: the form under each theme a program can name, its
    // captures kept as <case>-<theme>-<kind>-<n>.bmp)
    for (const theme of c.themes ?? []) {
      let shown;
      try {
        shown = execFileSync(bin, [], {
          encoding: "utf8",
          env: { ...process.env, RAPIDR_THEME: theme, RAPIDR_CAPTURE: join(WORK, `${c.name}-${theme}-${kind}`), RAPIDR_TEST_EVENTS: "", RAPIDR_TEST_DUMP: c.dump },
        });
      } catch (e) {
        shown = `(failed: ${String(e.message).split("\n")[0]})`;
      }
      const want = `lbl.caption=theme ${theme}`;
      ok(shown.includes(want) && existsSync(join(WORK, `${c.name}-${theme}-${kind}-1.bmp`)), `${c.name} (${kind}, ${theme}): ${want}, captured` + (shown.includes(want) ? "" : `\n    got: ${shown.trim().split("\n").join(" / ")}`));
    }
  }
  // (a native build, ~350 MB, gone once it ran: tests/cargo_builds.mjs)
  if (!PREBUILT && KINDS.includes(false)) dropBuild(CARGO_TARGET, c.name);
  if (BUILD_ONLY || !("native" in results && "interpreted" in results)) continue;
  const same = results.native === results.interpreted;
  ok(same, `${c.name}: native and interpreted builds agree` + (same ? "" : `\n    native: ${results.native.trim().split("\n").join(" / ")}\n    interpreted: ${results.interpreted.trim().split("\n").join(" / ")}`));
}
if (BUILD_ONLY) { console.log(`\nGUI events: executables built into ${BUILD_ONLY}`); process.exit(0); }
if (failed) { console.log(`\nGUI events: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nGUI events: ALL CHECKS PASSED");
