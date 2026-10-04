# RapidR desktop host: from FLTK to the UI kernel (winit + vello + parley + AccessKit)

## 0. Key findings that shape the plan

- **The seam already exists.** Outside `gui.rs`, the runtime calls FLTK through 64 `crate::gui::*` functions. They are used from `object.rs`, `layout.rs`, `scroll.rs`, `mdi.rs`, `builtins.rs` and `globals.rs`. Beyond those there are 7 direct `fltk::` uses (in `builtins.rs` and `globals.rs`) and 2 prelude re-exports (`set_theme`, `gui_register_timer`) that generated code calls. Nothing outside `rapidr-runtime-core` touches `gui`.
- **FLTK can be re-entered; winit cannot.** Today program handlers run *inside* FLTK callbacks, which run inside `app::wait()`. These handlers include `form_resized` → `rp_fire_event(form,"onresize")`, timer callbacks and button callbacks. Nested `app::wait()` loops then run inside them: `gui_showmodal`, `gui_choice`, the font dialog and `gui_wait_key`. winit's `pump_app_events` cannot be called from inside its own callbacks. This is the main architectural change.
- **The VM's side is already the right shape.** `Vm::after_host` → `Host::wait_started` / `Host::pump` → `rp_pump_wait` → `gui_pump_wait` is already "one step of the innermost wait, outside any callback". `serve_app` in `interpreter/rapidr-vm-host-native/src/lib.rs` does the same for the app loop. Native builds are the hard case: their handlers are plain Rust functions that block in `gui_showmodal`'s `while app::wait()`.
- **Models are mostly ready.** TabControl and scroll bars produce `Op` lists, TrackBar produces `Shape`s, Grid produces `CellDraw`s, ListView paints a `Bitmap`, Header produces `OwnerDrawn`, Menu produces `entries()`, TextEdit tracks a `revision`, and bitmaps have the HiRes layer. `rapidr_value::mdi::Runtime` and `rapidr_value::globals::Platform` already show the trait pattern the kernel needs to reach the runtime.
- **The wasm-bindgen pin is not deep.** `=0.2.118` in `generate_cargo_toml_web` (commit 530762f) only matches the installed CLI. The root lock has 0.2.115, and wasm-pack fetches whichever CLI the lock asks for. Generated programs have their own `[workspace]` and lockfile, so the root lock never reaches them.
- **The prototype and the runtime disagree on text metrics.** The prototype's `text::font_pixels` rounds like Windows' MulDiv (10 pt → 13 px). `rapidr_value::objects::text::text_size`, which backs `TextWidth`, does not round (10 pt → 13.33 px) and adds no kerning, while parley kerns by default. This has to be settled before the kernel draws captions.

---

## 1. Architecture

### 1.1 Crates and dependency graph

```
rapidr-value  (models, ops, layout, input, events; wasm-safe)
   ▲
rapidr-ui-kernel        NEW, GUI-free, must keep building for wasm32
   ▲                    (form tree, focus, input routing, hit/capture/hover,
   │                     display list, parley text and editors, AccessNode tree,
   │                     kernel-drawn dialogs, themes, headless tests)
rapidr-ui-host-winit    NEW, desktop only
   ▲                    (EventLoop + pump, a window per shown form, vello GPU and
   │                     vello_cpu renderers, softbuffer present, accesskit_winit,
   │                     muda menus, rfd file dialogs, platform shims)
rapidr-runtime-core  [feature "kernel"]   (src/ui/: facade, selection,
                                           kernel glue, waits, timers, test hooks)
```

- `rapidr-ui-kernel` depends only on `rapidr-value` and `parley` (pure Rust). It has no winit, wgpu or accesskit dependency. It defines its own `AccessNode` so the web (ARIA) and RAI (JSON) can use it later. CI checks it with `cargo check -p rapidr-ui-kernel --target wasm32-unknown-unknown`.
- `rapidr-ui-host-winit` does **not** depend on `rapidr-runtime-core`. Callbacks therefore cannot call program code: the types enforce the rule. The host returns `HostEvent`s, and runtime-core turns them into `rp_fire_event*` calls.
- **Where models live.** Per-component models (state, ops, hit-testing, keys, `describe()`) stay in or go to `rapidr_value::objects`, so the web runtime can draw the same new models without depending on the kernel. This includes the new button, check, radio, label, panel, group box, progress, status bar and combo models. The kernel handles composition. If the ROADMAP's "kernel crate (models…)" is meant literally, moving `objects/*` later is a mechanical move.
- **One op vocabulary.** Add `rapidr_value::objects::ops::Op` as a superset of `tabcontrol::Op`: `Fill`, `Edge` (bevels), `Line`, `Shape` (= `trackbar::Shape`), `Text{rect, place, angle}`, `Focus`, `Arrow`, `Image{source, revision, rect}`, `ClipPush/Pop`. Keep `tabcontrol::Op` as a re-export. The kernel's `DisplayList` is `Vec<Item>`, where `Item` is either a positioned `Op` or a `TextLayout{node, origin, selection colours}` for parley editors.
- **The prototype is absorbed, then deleted:**

  | Prototype file | Goes to |
  |---|---|
  | `form.rs` | kernel `tree.rs` / `focus.rs` / `input.rs` / `components/{label,button,edit,trackbar,tabcontrol}.rs` |
  | `paint.rs` | kernel `paint.rs` (emits a `DisplayList`, no vello) and host `render/vello.rs` (consumes it) |
  | `text.rs` | kernel `text.rs` |
  | `a11y.rs` | kernel `a11y.rs` (`AccessNode`) and host `a11y.rs` (→ `TreeUpdate`) |
  | `render.rs`, `menu.rs`, `main.rs` | host |
  | `tests.rs`, `demo.rs` | kernel tests |

### 1.2 Choosing FLTK or the kernel

- In `crates/rapidr-runtime-core/Cargo.toml`:
  - `desktop-ui = []`
  - `gui = ["desktop-ui", "dep:fltk", "dep:fltk-theme", "dep:arboard"]`
  - `kernel = ["desktop-ui", "dep:rapidr-ui-kernel", "dep:rapidr-ui-host-winit", "dep:arboard"]`
- Every `#[cfg(feature = "gui")] crate::gui::X` call site outside `gui.rs` becomes `#[cfg(feature = "desktop-ui")] crate::ui::X`. Only FLTK-internal code keeps `cfg(feature = "gui")`.
- `src/ui/select.rs` holds `enum Backend { Fltk, Kernel }`, decided **once, before any GUI starts**. FLTK's `app::App::default()` and winit's `EventLoop` both claim NSApplication, so they must never both start in one process.
  - If only one host is compiled in, use it.
  - If both are, read `RAPIDR_HOST=kernel|fltk`. The default is `fltk` until the switch (Stage 11), then `kernel`.
- `interpreter/rapidr-vm-host-native`: add `kernel = ["rapidr-runtime-core/kernel"]` and include it in `full`, so the `rapidr` CLI and `rapidrintr-runner` carry both hosts.
- Generated native projects (`generate_cargo_toml` in `crates/rapidr-codegen-rust/src/lib.rs`) get `features = ["kernel"]` when built with `rapidr build --host kernel`, or always once Stage 4's matrix needs it. Also copy the root `Cargo.lock` into generated projects so wgpu, vello and winit versions don't drift per program.

### 1.3 The facade in runtime-core (Stage 1)

`src/ui/mod.rs` holds one forwarding function per name in the 64-function list, plus `set_theme`, `gui_register_timer`, and new `screen_size`, `work_area`, `mouse`, `monitors`, `minimize` and `message_box` replacing the direct `fltk::` calls in `globals.rs` and `builtins.rs`. A small `forward!` macro generates them. The prelude in `lib.rs` re-exports `crate::ui::{set_theme, gui_register_timer}`. `gui.rs` stays as it is apart from `pub(crate)` visibility and is frozen except for bug fixes during the migration.

Kernel versions of the facade functions (`src/ui/kernel.rs`) fall into a few kinds:

| Kind | Facade functions | Kernel action |
|---|---|---|
| Invalidate | `redraw_widget`, `gui_redraw`, `canvas_redraw`, `picture_refresh`, `tab_control_changed`, `list_refresh`, `listview_refresh`, `grid_refresh`, `tree_refresh`, `header_refresh`, `dirtree_refresh`, `gui_set_caption`, `gui_apply_font`, `toggle_down_set` | `notify(Invalidate(name))` |
| Structure | `attach_late`, `gui_set_visible`, `gui_set_parent`, `gui_widget_add_items`, `gui_widget_clear`, `stack_widgets` | `notify(Structure(form_of(name)))` |
| Geometry | `gui_apply_geometry` | `notify(Geometry(name))`; for a form, also `HostCmd::SetSize` |
| Window | `gui_show`, `gui_show_visible`, `gui_hide`, `gui_close`, `gui_center`, `gui_move_form`, `gui_set_form_border`, `gui_apply_icon(s)` | `HostCmd`s, plus OnShow / OnClose logic copied from `gui_show` and `gui_close` (`rp_fire_event_then` with `CloseAction`) |
| Text | `text_push`, `gui_set_text`, `gui_set_input_value` | No-ops: the edit node re-reads `TextEdit::revision`, as the prototype's `Edit::refresh` does |
| Text | `text_pull`, `gui_get_text`, `gui_get_input_value` | No-ops: the kernel writes user edits into the model on every keystroke (`TextEdit::user_edit`, as the prototype's `Edit::sync`) |
| Waits | `gui_showmodal`, `gui_doevents`, `gui_wait_key`, `gui_pump_wait`, `gui_begin_app_wait`, `gui_take_wait_started`, `gui_set_cooperative_waits`, `run_gui_event_loop`, `gui_choice`, `gui_dialog_execute` | See §1.5 |
| Queries | `window_shown`, `form_window_exists`, `form_scale`, `menu_offset`, `is_modal`, `mouse_in_form` | Answered from kernel and host state |
| Deferred | `design_surface_*`, `code_editor_method` | Unsupported in the kernel until Stage 10 (one-time warning) |

`notify()` only pushes to a thread-local `NOTIFY` queue. The kernel drains it at the start of every host callback and every runtime step. Store hooks can therefore fire while the kernel is borrowed inside a pump.

### 1.4 How the kernel maps to the property store and the shared models

- **The store is the only source of truth.** The kernel never copies properties into a widget the way FLTK does with `GUI_WIDGETS`. It reads them through a trait shaped like `mdi::Runtime`:

  ```rust
  pub trait Store {
      fn get(&self, id: &str, prop: &str) -> Value;
      fn type_of(&self, id: &str) -> String;
      fn children(&self, id: &str) -> Vec<(String, String)>;
      fn font(&self, id: &str) -> Font; // rapidr_value::objects::font_from_props
  }
  ```

  runtime-core implements it over `rp_comp_get`, `rp_comp_type` and `get_children_of` (`src/ui/kernel_store.rs`). Kernel-drawn dialogs use an in-memory `MemStore`.
- **Per-form retained tree.** Each form has `FormUi { root, nodes: Vec<Node>, focus, hover, pressed, capture, caret_on, structure_rev }`. A node is `Node { id: String, kind: NodeKind, abs_rect, ui: NodeUi }`. `NodeUi` holds UI-only state: a parley `PlainEditor`, a scroll offset, a cached image, an open popup.
  - The tree is rebuilt from `Store::children` when a `Structure` notification arrives, which replaces `build_form_widgets` and `build_children_recursive`. It needs none of their temporary absolute-offset hacks: the kernel computes absolute rectangles from Left/Top while walking the tree, adding `menu_offset` for in-window menus.
  - Geometry and captions are read from the store when painting.
- **Components are dispatched through a table.** `components/mod.rs` maps a type name (`"RTRACKBAR"`) to a `&'static dyn ComponentKind` with `paint`, `hit`, `mouse`, `key`, `describe`, `focusable` and `test_action`. Implementations call the existing model accessors, for example `rapidr_value::objects::with_trackbar_mut(id, |t| t.mouse_down(..))`. Adding a component never touches the runtime glue.
- **Kernel output is events.** `KernelEvent::{Click(id), Change(id), Key{chain, down, vk, shift, text}, Mouse{id, kind, button, x, y, shift}, Close(form), Resized(form, w, h), Moved, ScaleChanged, MenuPick(id), …}`. runtime-core maps these one-to-one onto what `gui.rs` does today: `key_events` (with `rapidr_value::input::key_targets` for KeyPreview), `mouse_event`, `rp_fire_event`, `button_modal_result`, `form_resized`. The key order stays FLTK's: the OnKeyDown chain, then the model, then OnKeyPress.

### 1.5 Event-loop ownership

**The choice: winit's `pump_app_events` (`EventLoopExtPumpEvents`), not `run_app`.**

- `pump_app_events` exists on Windows, macOS, X11, Wayland and Android. It does not exist on iOS or the web.
- Generated native code is ordinary Rust: `fn main()` runs the program, and `Form.ShowModal` must block and return a value. Native builds cannot hand the main thread to `run_app`.
- Running the program on a worker thread is ruled out. The store, events and models are all `thread_local`, handlers can be `Rc` closures, and FFI or `RUSTSTART` code may expect the UI thread.
- iOS, later, gets `run_app` with the interpreter only, driven by the VM's existing `Yielded` / `Suspended` / `resume` (as the web host is). This matches ROADMAP Phase 7.

**Rules:**

1. **One `EventLoop` per process.** It is created lazily on the main thread by `ensure_host()` and lives in `thread_local HOST: RefCell<Option<Host>>`. Never call `event_loop.exit()`: `PumpStatus::Exit` cannot be undone. The process ends through `rp_end` / `exit`, and closing a form is not an exit.
2. **Callbacks never run program code.** Inside a pump, the `ApplicationHandler` shim only does four things: drain `NOTIFY` into the kernel, turn winit input into kernel input (which queues `KernelEvent`s), render on `RedrawRequested`, and record resize, move and scale changes. A safety net catches anything that slips through: `object.rs` gets `thread_local IN_HOST_CALLBACK`, and while it is set, `fire()` and `rp_fire_event_then` queue native handlers and their continuations into a `DEFERRED` queue instead of running them (with a `debug_assert` and a log, so offending paths get found).
   - Because of the safety net, the real `form_resized` logic can run inside the `Resized` callback (`quietly` store, `layout::client_changed`, `scroll::update`), and its OnResize/OnPaint are deferred.
   - On macOS, live resize keeps the pump inside AppKit's tracking loop, so the layout follows the drag live and the handlers run at mouse-up.
3. **Program code runs only between pumps,** in `step()` in `src/ui/kernel.rs`:

   ```
   step(max_wait):
     show_pending()                         (kept from gui.rs)
     run pre-paint: owner-draw events for dirty grids/lists (OnDrawCell/OnDrawItem
       recorded as ops, as grid_replay does today), first OnPaint (after_show)
     t = min(max_wait, next timer, caret blink, test-script step, rfd future poll)
     host::pump(t, &store)                  → appends HostEvents to a queue
     drain DEFERRED; drain HostEvents → dispatch (rp_fire_event*, close, resize…)
     fire due timers (re-read Interval/Enabled each tick, as schedule_timer does)
     test-hook step
   ```

   - In a native build, `dispatch` runs handlers directly. A handler may call `ShowModal`, which calls `step()` again: a nested pump that is legal because no callback is on the stack. Queues are always moved out with `mem::take` before dispatching.
   - In the VM, `rp_fire_event` only queues bytecode events, and `Vm::after_host` runs them.
4. **Window commands** (create, show, hide, title, size, position, border, icon) go into a `HostCmd` queue. The shim runs them in `new_events` / `about_to_wait`, where an `ActiveEventLoop` is available. `gui_show` then calls `host::pump(Some(ZERO))` so the window exists before OnShow and `after_show` (scale and first paint) run, as they do today.
   - The AccessKit adapter is created before the window becomes visible, as the prototype does.
   - The deprecated `EventLoop::create_window` is an acceptable stopgap.

**How each wait maps:**

| RapidQ call | FLTK today | Kernel host: native build | Kernel host: VM |
|---|---|---|---|
| `Form.ShowModal` | `gui_showmodal`: `while app::wait()` | Push to `MODAL_FORMS`; `while form_shown { step(None) }`; `modal_ended` | Unchanged protocol: push `Wait::Form`, set `WAIT_STARTED`; `Host::pump` → `ui::gui_pump_wait` → one `step(None)`, returning `Some(modal_result)` when the form closes |
| Main loop after MAIN | `run_gui_event_loop` / `serve_app` | `while any window shown { step(None) }` | `serve_app` → `rp_pump_wait`, as today |
| `DOEVENTS` | `app::wait_for(0.0)` | `step(Some(ZERO))` | Same; handlers run in `after_host` |
| INPUT$ / `rp_waitkey` | `gui_wait_key` | `step` until `console::key_waiting()` | Same |
| `SLEEP` | `thread::sleep` | Unchanged (RapidQ doesn't pump during SLEEP) | Unchanged |
| MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE | `gui_choice`'s nested `app::wait` | A kernel-drawn modal dialog (`MemStore` form: label and buttons, first button default, Escape → `dialogs::dismissed`); `step` until a choice | Same; timer events are queued and run after the builtin returns, as today |
| Open / Save / File dialog | `NativeFileChooser` (blocking) | `rfd::AsyncFileDialog` future; `step` until it is ready (the windows keep painting); `RAPIDR_TEST_FILE_DIALOG` short-circuits | Same |
| Colour / Font dialog | FLTK windows | Kernel-drawn modal forms (rfd has neither). Until lists are ported, a minimal kernel dialog | Same |

- **Modality.** Input to any window other than the top of `MODAL_FORMS` is dropped and the modal window is focused. On Windows, use `with_owner_window` so the modal stays above its owner.
- **Timers.** The FLTK `add_timeout3` chain is replaced by a `BinaryHeap<(Instant, gen, name)>` in `src/ui/kernel.rs`. It ticks only while `step` runs (ShowModal, the main loop, DoEvents, INPUT$, dialogs), as FLTK timeouts do. `rp_stop_all_timers` clears it.
- **Menus.**
  - macOS: muda's NSMenu bar, built from `rapidr_value::objects::menu::entries`. muda events arrive through an `EventLoopProxy` user event and become `HostEvent::Menu`.
  - Windows and Linux: a kernel-drawn in-window bar using the shared menu model and `layout::MAIN_MENU_HEIGHT`, the same `ClientHeight` as the web and as `RAPIDR_MENU=window`. muda on Linux needs GTK.
  - Shortcuts: `menu::item_for_shortcut` is checked before a key is routed.
  - QPOPUPMENU uses muda context menus on macOS and Windows, and a kernel-drawn popup elsewhere.
- **Popups** (combo drop-downs, grid lists, kernel popup menus) are borderless top-level windows on Windows, macOS and X11. Wayland cannot position them in winit 0.30, so they fall back to an in-window overlay.
- **Headless host.** When `RAPIDR_CAPTURE` is set and `RAPIDR_CAPTURE_WINDOWS` is not, use a second host implementation, `HeadlessHost`. It creates no OS windows, `pump(t)` just sleeps until `t`, the screen is a fixed 1920×1080 at `RAPIDR_SCALE`, and captures use vello_cpu. The GUI matrix then runs without a desktop session or GPU, including on Linux CI.

---

## 2. Porting order, the first slice, and tests

### 2.1 First vertical slice

Components: QFORM (window, Caption, Color, Width/Height and client size, Show, ShowModal, Hide, Close/OnClose, Center, ModalResult), QLABEL, QBUTTON (Kind and ModalResult through `button_modal_result`), QTRACKBAR (the existing shared model), and QTIMER (host timers).

Fixtures, all from `tests/fixtures`, with their expectations already in `tests/gui_parity_cases.mjs`:

1. **`timer_default.bas`**: a form, a label and a QTIMER at 50 ms under ShowModal. It proves the window, the pump, timers, `RAPIDR_CAPTURE` / `RAPIDR_TEST_DUMP`, and the native-versus-VM wait. Expected: `lbl.caption=ticking`, `t.enabled=-1`.
2. **`trackbar.bas`**: `tb.__key_39`, `__key_34`, `__key_36`, `__mousedown_190_10`, `__key_35` and `btn.onclick`. It proves synthetic keys and mouse input going through the **real** kernel input path into the TrackBar model, plus OnChange and OnClick. Expected: `lbl.caption=102111|10|4,6,0,2,10,|4`.
3. **`nested_modal.bas`**: a ShowModal inside a button handler, a modal form closed by a timer, and a timer ticking under the outer modal. This is the architectural proof: a nested `step()` in native builds and the `WAITS` stack in the VM. Expected: `lbl.caption=open;timer-close;closed;`, `lbl2.caption=ticking`.

Next, at the end of the slice: `oop_events.bas` (Center, TYPE EXTENDS QBUTTON, Sender) and `component_array_events.bas`. `modal_result.bas` waits for QEDIT (Stage 7a).

### 2.2 Order after the slice

| # | Group | Fixtures that confirm it |
|---|---|---|
| 1 | Forms and events | `startup_modal`, `form_visible`, `late_parent`, `inherit_event`, `doevents_loop`, `screen_scale`, `border_icons`, `event_answers` (`__close`; its grid cases after the grid port) |
| 2 | Containers and layout | QPANEL and QGROUPBOX (bevel model), Align/Anchors with `RAPIDR_TEST_RESIZE`, QSPLITTER (`RAPIDR_TEST_SPLIT`), QSTATUSBAR, scroll bars and QSCROLLBOX. Fixtures: `panel_bevels`, `align_layout`, `anchors`, `autoscroll`, `onshow_scroll`, `statusbar_panels` |
| 3 | Shared-model components | QTABCONTROL (`tab_control`; needs QEDIT for its aligned edit, so as a read-only edit first), menus (`menus`), QLISTBOX / QCOMBOBOX and owner-draw (`list_items`, `list_columns`, `owner_list`; popup windows), QLISTVIEW (`listview_columns`, `listview_views`; `listview_paint` bitmap → image op), QSTRINGGRID (`string_grid`, `grid_moving`, `grid_draw_cell`, `grid_range_list`, `grid_draw_hidden`), QHEADER (`header`), QTREEVIEW / QOUTLINE (`tree_view`, `tree_edit`, `tree_images`, `outline`; needs a tree paint model in `rapidr_value::objects::tree`), check / radio / QCOOLBTN (`coolbtn_group`, `toggle_group`), QFORMMDI (`mdi_children`, via `mdi::Runtime`), QDIRTREE / QFILELISTBOX (`file_browser`) |
| 4 | Drawing surfaces | `canvas_onpaint`, `form_draw`, `dotted_paint`, `picture_resource`, `svg_picture`, `icons`, `option_icon` |
| 5 | Text | `text_edits`, `modal_result`, `input_events`, `input_chars`, `inkey_wait`, `inkey_trapall`; QRICHEDIT last |
| 6 | Dialogs | `file_dialogs` (no UI under test); MessageBox, ColorDialog and FontDialog |
| 7 | IDE-only components | RDESIGNSURFACE, RCODEEDITOR (port or retire) |

### 2.3 Test hooks for the kernel host

Parsing moves to `crates/rapidr-runtime-core/src/ui/testhooks.rs`: the env grammar, the `__key_N` / `__mousedown_x_y` / `__item_i` / `__node_i` / `__toggle_i` / `__cell_c_r` / `__edit` / `__enter` / `__escape` / `__close` forms, and the dump formatting. FLTK's `install_capture_hook`, `fire_test_events` and `capture_windows` stay as they are, to avoid touching FLTK. The kernel driver:

- **Start.** The script starts `RAPIDR_CAPTURE_DELAY` (default 1.5 s) after the first `step()` with a form shown. One event per step, and the next event waits until the handler queue (native `DEFERRED`, VM events) is empty, which replaces FLTK's 0.05 s timeouts. User input is dropped (`IGNORE_USER`).
- **`x.onclick`, `x.onchange`, …** call `rp_fire_event` directly, with the toggle-group press first, exactly as FLTK does.
- **`__key_N`** focuses `x`, then injects KeyDown and KeyUp through the kernel's input entry point, with VK→text from `rapidr_value::input::text_of_vk`.
- **`__mousedown/up/move_x_y`** inject a pointer event at `x`'s absolute logical position plus (x, y). This uses the real routing (hit test, capture, scroll bars first) rather than calling the model directly.
- **`__item_i`, `__node_i`, `__toggle_i`, `__cell_c_r`, `__edit`, `__enter`, `__escape`** go to `ComponentKind::test_action`. It should synthesize real input at the row or cell rectangle where it can. `__enter` types "Renamed" into the editor, as the FLTK hook does.
- **`__close`** goes to the window's close path (`gui_close`).
- **`RAPIDR_TEST_DUMP`** uses the shared formatter; `__shown` asks the kernel whether the node is visible all the way up its parents and its window is shown.
- **`RAPIDR_TEST_RESIZE=w,h`** sends the frontmost form a `Resized` through the same code as a user drag, so `form_resized` logic runs. **`RAPIDR_TEST_SPLIT`** uses `layout::splitter_*`, which is already host-neutral. **`RAPIDR_TEST_FILE_DIALOG`** goes into the host-neutral file-dialog logic, moved out of `gui.rs::file_dialog` into `ui/file_dialog.rs` with a `pick` closure.
- **`RAPIDR_CAPTURE=<prefix>`** renders each shown window in stacking order, dialogs included, with vello_cpu at the window's scale, and writes `<prefix>-<n>.bmp` via `codec::encode_bmp`. It then prints the dump and exits.
- **New: `RAPIDR_TEST_A11Y=<file.json>`** writes each form's `AccessNode` tree.

### 2.4 The "both hosts must agree" matrix

Extend `tests/native_gui_events.mjs`. Build each fixture once natively and once interpreted, with both hosts compiled in, then run each binary twice with `RAPIDR_HOST=fltk` and `RAPIDR_HOST=kernel`. regress.sh already adds `RAPIDR_SCALE=2`. Checks:

- (a) Every one of the four runs contains `case.expect`.
- (b) All four dumps are byte-identical. This replaces today's native-equals-interpreted check.
- (c) Kernel only: capture BMPs match `tests/golden/kernel/<case>@<scale>x-<n>.bmp` within a tolerance (±8 per channel on ≤0.5 % of pixels). This covers ROADMAP's "screenshots compared per scale".
- (d) Kernel only: the `RAPIDR_TEST_A11Y` JSON matches a golden (roles, names, values, states).

Each case in `gui_parity_cases.mjs` gets `kernel: true | "pending: <reason>"`. The harness prints the pending count, and the switch criterion is zero pending plus a green `tools/regress.sh`. regress.sh gets a `gui-kernel` stage that runs headless.

---

## 3. Dependencies and the lockfile

Cargo resolves **one lockfile per workspace for all targets**. Target-gated dependencies (`[target.'cfg(not(target_arch="wasm32"))'.dependencies]`) still enter `Cargo.lock`. Cargo also cannot hold two semver-compatible `wasm-bindgen 0.2.x` versions, and wasm-bindgen's internal `=` pins enforce that as well. So option C (target-specific dependencies) alone does not help.

**Option A (recommended): bring the host into the workspace and align wasm-bindgen in the same commit (Stage 0).**

*Status:* the alignment (steps 1–3 and the one-version check of step 5) is done: the workspace is on wasm-bindgen 0.2.129, js-sys / web-sys 0.3.106 and wasm-bindgen-futures 0.4.79 with no API changes needed, generated web projects pin `=0.2.129`, and the web suites pass. Steps 4 and 5's kernel checks wait for the crates.

1. Bump `wasm-bindgen` in `crates/rapidr-runtime-web/Cargo.toml`, `interpreter/rapidr-vm-host-web/Cargo.toml` and `interpreter/rapidr-compiler-wasm/Cargo.toml` to the prototype's resolved 0.2.129 (≥ 0.2.127 for wgpu 30), together with js-sys / web-sys 0.3.106 and the matching wasm-bindgen-futures.
2. In `generate_cargo_toml_web`, change `=0.2.118` to `=0.2.129`. Update the README's `cargo install wasm-bindgen-cli --version 0.2.129` and the buildserver's environment.
3. Run `tools/build_web_artifacts.sh` (wasm-pack fetches the matching CLI itself), then regress.sh's web stages: web_conformance, web_gui_parity at 1× and 2×, web_ide_*, web_bundle_*, web_vm_yield, web_end_timer.
4. Add `crates/rapidr-ui-kernel` and `crates/rapidr-ui-host-winit` as members. Remove the prototype from `exclude` once its code is absorbed, and delete its `Cargo.lock`.
5. Check that `cargo tree -i wasm-bindgen --target wasm32-unknown-unknown` shows one version, and that `cargo tree -p rapidr-runtime-web --target wasm32-unknown-unknown` contains no wgpu, winit or vello. Add `cargo check -p rapidr-ui-kernel --target wasm32-unknown-unknown` to the unit stage.

This is low risk. The pin exists only to match the installed CLI, and generated web programs resolve their own lock, so they are unaffected even before step 2.

**Option B (fallback only): keep the host in its own workspace.** runtime-core would expose `install_backend(Box<dyn DesktopUi>)`. Every final binary (the `rapidr` CLI, `rapidrintr-runner`, generated programs) would then have to be built from the host's workspace, which means two lockfiles for the same crates and two CI paths. Use it only if the wasm-bindgen bump hits a real blocker.

**Rejected:** pinning older wgpu or vello to fit 0.2.115. vello 0.11 needs wgpu 30, and parley and accesskit are tied to the same Linebender versions.

---

## 4. Text

- **One size rule. Decided (v2.108.0): Windows' rounded MulDiv** — `Font::pixel_size()` in `rapidr_value::objects::font` is used by `text::text_size`, the FLTK host, the web runtime (which also stopped treating FontSize as pixels) and must be used by the kernel; no fixture expectation changed. Original note: Add `Font::pixel_size()` in `crates/rapidr-value/src/objects/font.rs` and use it in `text::text_size`, the kernel, and `gui.rs::font_pixels`. Decide between Windows' rounded MulDiv (RapidQ-faithful; 10 pt = 13 px) and the current unrounded value. Changing `text_size` changes `TextWidth`, which fixtures read (`canvas_onpaint` `|36`, `form_draw`), so the decision, the web runtime and the fixture expectations change in one commit.
- **No kerning.** GDI's TextOut doesn't kern, and `text_size` sums advances. The kernel sets parley's font features to `kern=0`, so a caption is drawn exactly as wide as `TextWidth`, and as tab widths in `tabcontrol.rs` measure. Glyphs are hinted when upright, as in the prototype's `draw_layout`.
- **Fonts.** Keep the built-in Liberation Sans, Serif and Mono (`crates/rapidr-value/fonts`), registered in fontique (prototype `TextSystem::new`), with `family()` matching `text.rs::face_data`. System fallback covers CJK and emoji. Known gap: `text_size` measures missing glyphs with `?`'s advance. A later fix is a measuring hook in `rapidr_value::objects::text` so the desktop measures with parley.
- **QEDIT** uses the prototype's `Edit` (a `PlainEditor` over the `TextEdit` model, with `revision` refresh and `user_edit` sync). Still to add:
  - PasswordChar: draw a masked layout, keep the model's text.
  - Alignment, HideSelection, ReadOnly caret, MaxLength and CharCase (already in the prototype).
  - Double-click selects a word; triple-click selects all.
  - A right-click menu with Cut / Copy / Paste through the kernel popup.
  - Clipboard through `globals::Platform::clipboard_text`, which keeps `RAPIDR_TEST_CLIPBOARD` working.
- **IME.** Handle `Ime::Enabled`, `Ime::Preedit(text, cursor)` → `PlainEditor::set_compose`, `clear_compose`, and `Ime::Commit`; the prototype handles only Commit. Every frame, call `window.set_ime_cursor_area` from `cursor_geometry`. Call `set_ime_allowed(true)` only while an edit has focus, so shortcuts and keys in other components aren't swallowed.
- **QMEMO, multi-line.** A `TextEdit { multi: true }` laid out as one parley layout **per paragraph**, painting only the visible paragraphs. Vertical and horizontal scrolling use the shared `scrollbars::Scroller`. WordWrap means `break_all_lines(Some(width))`. Caret and selection use `parley::{Cursor, Selection}` per paragraph. `Lines`, `WhereX`, `WhereY` and `LineCount` come from the model. Start with one `PlainEditor` if that is faster to land, but measure: `AddStrings` in a loop relayouts everything.
- **QRICHEDIT, last.**
  - Step 1: plain multi-line, identical to QMEMO. This is today's behaviour; FLTK's RRICHEDIT is a plain TextEditor, and `text_edits.bas` only uses text and selection.
  - Step 2, if wanted: a `richtext` model in `rapidr_value::objects` (style runs over `TextEdit`'s chars: colour, bold, italic, underline, size, face, paragraph alignment) for `SelAttributes` and `Paragraph`. Drawn with parley's `RangedBuilder` per paragraph, edited by the kernel's paragraph editor. RTF in and out as a minimal subset (`\b \i \ul \fs \cf \colortbl \fonttbl \par \pard \qc`), shared with the web. Budget it separately.

---

## 5. Drawing surfaces

- **QCANVAS, the form surface (`create_form_surface`), QIMAGE and QBITMAP stay CPU models in `rapidr_value::objects::bitmap`.** Low-res pixels are what programs read (`Pixel`, `.BMP`, flood fill). The HiRes layer is what the screen shows.
  - The kernel emits `Op::Image { source: id, revision, rect }`.
  - The host keeps a `HashMap<(id, revision), peniko::ImageData>` built from `Bitmap::display_rgba()`, re-uploading only when the revision changes. Add a `revision` counter to `Bitmap`, bumped by every drawing method and by `invalidate_display`.
  - The image is drawn at its logical size times the scale with `ImageQuality::High`; `Medium` is the downscale case. `$OPTION SCALING LEGACY` later uses nearest-neighbour.
- **OnPaint keeps today's flow.** The first OnPaint comes after the window shows (`after_show` → `fire_first_paint`). Repaint and resize fire it again. The surfaces keep what was drawn, so painting a frame never calls the program. `set_display_scale` is called per window from `scale_factor()` or `RAPIDR_SCALE`, as `after_show` does now. `ScaleChanged` → OnScaleChanged, then a first paint, with the events after the pump (not the `add_timeout3(0.0)` hack).
- **Vector drawing at full GPU resolution, later.** Optionally, a canvas could record its vector calls (Line, Circle, Rectangle, TextOut) as `Op`s next to the low-res raster, and the host would draw them with vello. That is a ROADMAP high-DPI refinement and is not needed for parity, since the HiRes raster is already sharp.
- **SVG.** Already rasterized per scale by `resvg` inside `bitmap.rs`. Icons use `objects::icon_pixels` → window icon via winit `set_window_icon` (Windows, X11), and a macOS platform shim for the application icon.
- **CPU fallback.** The host defines `trait Renderer { fn render(&mut self, list, size, scale) }` with `VelloGpu` and `VelloCpu` implementations, presenting CPU frames through `softbuffer`.
  - It falls back automatically when wgpu finds no adapter or vello's compute pipelines fail (VMs, GL-only, RDP).
  - `RAPIDR_RENDERER=cpu|gpu` forces one.
  - Captures always use the CPU renderer, for determinism.
  - The vello_cpu version that matches peniko 0.6 / vello 0.11 must be checked in the Stage 3 spike. tiny-skia plus skrifa outlines is the fallback-of-the-fallback.

---

## 6. Accessibility

- **`AccessNode` lives with the models.** In `crates/rapidr-value/src/objects/a11y.rs`: `AccessNode { role, name, description, value, numeric: Option<{value, min, max, step, jump}>, states (focused, disabled, checked, selected, expanded, read_only, multiline, modal), actions (Click, Focus, SetValue, Increment, Decrement, Expand, Collapse, ScrollIntoView), bounds (logical), children }`. Each model or `ComponentKind` implements `describe`:
  - TrackBar → Slider; TabControl → TabList with Tab children (needs `TabControl::tab_rect(i)` made public, which fixes the prototype's placeholder bounds); ListBox → ListBox with ListBoxOption rows; TreeView → Tree / TreeItem with levels; Grid → Grid / Row / Cell; Edit → TextInput (plus text selection later); Menu → MenuBar / MenuItem; Form → Window or Dialog.
- **Stable node ids.** `hash(lowercase component id)`, with sub-items as `hash(id) ^ (kind << 56 | index)`. The prototype's `100 + index` changes whenever components are added or removed.
- **Names.** `AccessibleName` (new), otherwise Caption, Text, Hint, then the nearest QLABEL to the left or above on the same parent (a geometric rule rather than the prototype's "last label"). `AccessibleDescription` is also new.
- **Updates.** The kernel assembles the tree per form after each `step()` when that form is dirty. The host converts it to `accesskit::TreeUpdate`, sending only nodes that changed since the last tree, and calls `adapter.update_if_active`. Action requests come back as `HostEvent::A11y(form, request)` and become the same kernel input a user action would, so OnClick and OnChange fire identically, as the prototype's `a11y::apply` does.
- **Keyboard.** TabOrder / TabStop instead of creation order, a visible focus rectangle, mnemonics (`&File`, Alt+letter), Enter → default button, Escape → Cancel or ModalResult.
- **Web, later.** `rapidr-runtime-web` maps the same `AccessNode` to `role`, `aria-label`, `aria-valuenow/min/max`, `aria-selected`, `aria-expanded` on its DOM elements, plus a live region for status changes. Phase 5's RAI serializes the same tree to JSON.

---

## 7. Stages, effort, parallel work, risks

A session is one focused agent session ending in a green commit.

| Stage | Content | Sessions | Who |
|---|---|---|---|
| 0 | Spikes on the prototype: (a) `pump_app_events` on macOS with a modal-in-handler loop, live resize, rfd **async** and muda during a pump; (b) a vello_cpu capture equal to the GPU capture within tolerance; (c) a headless host. Decide the `pixel_size` / kerning question. wasm-bindgen alignment (§3) | 2–3 | Single owner |
| 1 | Facade `src/ui/` (forward all 64 functions, prelude, `globals.rs` and `builtins.rs` FLTK calls), `select.rs`, the `desktop-ui` / `kernel` features, the deferred-handler safety net in `object.rs`, `testhooks.rs` parse module. FLTK behaviour unchanged (full regress) | 2–3 | Single owner |
| 2 | `rapidr-ui-kernel` from the prototype: `ops::Op` superset, `DisplayList`, paint, text (with the §4 decisions), `Store` trait, tree, focus, input, `ComponentKind` table with label, button, trackbar, tabcontrol and single-line edit, `AccessNode`, headless tests (prototype `tests.rs` against a `MemStore`), wasm check | 3–4 | Single owner |
| 3 | `rapidr-ui-host-winit` (pump host, `HostCmd`, windows, vello GPU and CPU, softbuffer, accesskit, headless host) plus `src/ui/kernel.rs` (`step`, waits, timers, ShowModal native and VM, DoEvents, `wait_key`, `form_resized`, dispatch, test-hook driver). Slice fixtures green in both build kinds | 4–6 | Single owner |
| 4 | Matrix in `native_gui_events.mjs` (hosts × kinds × scales, `kernel:` field, goldens, A11Y JSON), regress.sh `gui-kernel` stage, `rapidr build --host kernel`, lock copied into generated projects | 1–2 | Single owner |
| 5 | Component lanes (§2.2 groups 1–3) | 18–25 in total, 4–6 calendar-wise | 3–4 agents in parallel |
| 6 | Drawing surfaces (§5) | 3–4 | Parallel with 5 |
| 7a | QEDIT complete plus IME, QMEMO | 4–6 | Parallel with 5 |
| 7b | QRICHEDIT step 1 (step 2: +5–8, optional) | 2–3 | After 7a |
| 8 | Dialogs: MessageBox family, rfd async, colour and font dialogs | 2–3 | Parallel |
| 9 | Platform: screen, work area, global mouse (CGEvent, GetCursorPos, XQueryPointer; none on Wayland), monitors, cursors (`Cursor` → winit `CursorIcon`), BorderStyle / BorderIcons, minimize, icons, `$THEME` → kernel themes (classic, modern, high-contrast) | 3–4 | Parallel |
| 10 | RDESIGNSURFACE / RCODEEDITOR: port, or retire in favour of Phase 3's IDE | 2–4 | Late |
| 11 | Switch the default to the kernel; one release with `RAPIDR_HOST=fltk` as fallback; then remove FLTK (`gui.rs`, the fltk deps, `fltk` in `file_dialog.rs`) | 2 | Single owner |
| 12 | Web ARIA from `AccessNode` | 2–3 | Later |

Total: about 55–75 sessions. Stages 0–4 (12–18 sessions) are on the critical path and must be done in order by one owner.

### Lane ownership in Stage 5 and after

| Lane | Files owned | Done when |
|---|---|---|
| Containers and layout | `crates/rapidr-ui-kernel/src/components/{panel,groupbox,scrollbox,splitter,statusbar,mdi,form}.rs`; `crates/rapidr-value/src/objects/bevel.rs`, `crates/rapidr-value/src/scrollbars.rs` | align_layout, anchors, autoscroll, mdi_children, statusbar_panels, panel_bevels pass |
| Lists | `components/{list,combo,listview,grid,header,tree,dirtree,filelist}.rs`, `crates/rapidr-ui-host-winit/src/popup.rs`, tree paint in `crates/rapidr-value/src/objects/tree.rs` | list, grid, tree, header fixtures pass |
| Buttons and menus | `components/{check,radio,coolbtn,ovalbtn,progress,menubar,popupmenu}.rs`, `crates/rapidr-ui-host-winit/src/menu.rs` | menus, coolbtn_group pass |
| Surfaces | `components/{canvas,image}.rs`, `crates/rapidr-ui-host-winit/src/render/images.rs`, `Bitmap::revision` | canvas and picture fixtures pass |
| Text | `crates/rapidr-ui-kernel/src/text/*`, `components/{edit,memo,richedit}.rs` | text fixtures pass |
| Dialogs and platform | `crates/rapidr-ui-kernel/src/dialogs.rs`, `crates/rapidr-ui-host-winit/src/{dialogs,platform}.rs` | file_dialogs, screen_scale, border_icons, icons pass |

- **Shared files with an integrator.** `src/ui/mod.rs`, `src/ui/kernel.rs` and the host's `app.rs` (input translation; the text lane needs its IME arm, the platform lane its cursors) have one owner, and other lanes request changes. `components/mod.rs` registrations are append-only one-liners. In `gui_parity_cases.mjs`, each lane flips only the `kernel:` field of its own cases.
- **Changes to a shared model's API** must update the model's web callers (`crates/rapidr-runtime-web/src/gui_web.rs`) in the same commit and rerun web parity.

### Risks

1. **pump_events on macOS** (the most serious). During live resize or menu tracking the pump doesn't return, and events from nested Cocoa modals outside a pump (blocking rfd) may be dropped. Mitigations: the deferred-handler safety net, async rfd polled from `step`, and the Stage 0 spike before anything else depends on it.
2. **Event ordering changes.** Handlers now run after a batch of input instead of inside each callback, which affects OnResize→OnPaint and key ordering. The four-way dump equality catches this.
3. **Text metrics.** Changing `pixel_size` rounding or kerning changes `TextWidth`, which programs read. This is decided once, in one commit across both runtimes.
4. **Popups on Wayland** (fall back to in-window overlays), **global mouse position** (no winit API; none on Wayland), and **work area** (platform shims).
5. **GPU availability** and wgpu startup (100–300 ms for the device and vello shaders) are covered by the CPU fallback and area-only anti-aliasing. **Binary size** grows by about 10–15 MB while both hosts are compiled in.
6. **Build time for native programs.** Each generated project builds wgpu and vello in its own target directory, as it builds fltk-sys today. A shared default `CARGO_TARGET_DIR` (e.g. `~/.cache/rapidr/target`) and the copied lockfile help.
7. **AccessKit platforms.** Linux needs the AT-SPI bus. Also, rfd on Linux should use the xdg-portal backend (no GTK), with a kernel-drawn fallback if no portal is present.
8. **Scope:** RDESIGNSURFACE and RCODEEDITOR, and QRICHEDIT step 2, are kept as separate, optional budgets.
9. **The headless host diverging from real windows** (screen size, focus, scale). Run the matrix with `RAPIDR_CAPTURE_WINDOWS=1` on a desktop machine before the switch.

### Critical Files for Implementation
- /Users/roanbema/Programming/rust/RapidR/crates/rapidr-runtime-core/src/gui.rs (`gui_showmodal`, `gui_pump_wait`, `gui_choice`, `form_resized`, `install_capture_hook`, `fire_test_events`, `capture_windows`, `schedule_timer`: the behaviour to reproduce)
- /Users/roanbema/Programming/rust/RapidR/crates/rapidr-runtime-core/src/object.rs (`rp_comp_set` / `rp_comp_get` hooks, `fire`, `rp_fire_event_then`, `rp_pump_wait`, `rp_set_cooperative_waits`: the facade call sites and the deferred-handler safety net)
- /Users/roanbema/Programming/rust/RapidR/crates/rapidr-ui-proto/src/form.rs (and `paint.rs`, `text.rs`, `a11y.rs`, `main.rs`: the code that seeds `rapidr-ui-kernel` and `rapidr-ui-host-winit`)
- /Users/roanbema/Programming/rust/RapidR/interpreter/rapidr-vm-host-native/src/lib.rs (`serve_app`, `install_event_queue`, `Host::pump`; with `/Users/roanbema/Programming/rust/RapidR/interpreter/rapidr-vm/src/lib.rs` `Vm::after_host`)
- /Users/roanbema/Programming/rust/RapidR/tests/native_gui_events.mjs (with `/Users/roanbema/Programming/rust/RapidR/tests/gui_parity_cases.mjs`: the four-way matrix)

---

## Stage 0 results (2026-10-04) — all three spikes GO

Code: `crates/rapidr-ui-proto` (`host.rs` pump / CPU / headless hosts, `kernel.rs`, `ui.rs` step layer with timers, nested modals and the script driver, `cpu.rs`, `compare.rs`, `macos_probe.rs`, `scripts/macos-probes.script`). These amend the plan above:

1. **§1.5 rule 2, live resize.** On macOS 27, live resize does *not* hold the pump: each drag step returns with `Resized` (inLiveResize), so OnResize and timers run during the drag (92 resizes, 48 ticks, worst 8 ms late, in a 1.5 s drag). Keep layout-in-callback for loops that do hold the pump: menu tracking, and Windows' modal size/move loop (unverified).
2. **§1.5 rule 1, program end.** Pump once with a zero timeout so pending Hides run, then `mem::forget` the host before `process::exit`. Dropping winit windows during main-thread TLS teardown aborts (a panic in `WindowDelegate::window_will_close`).
3. **§1.5 rule 4, file dialogs.** rfd async dialogs are created by a `HostCmd` *inside* a pump. rfd makes a sheet only while `NSApp.isRunning`, which is false between pumps; otherwise it silently falls back to a blocking `runModal`. The dialog is polled after each step, and its waker sends an EventLoopProxy user event.
4. **Waits table, Open/Save.** As in 3. The sheet is window-modal, so the kernel pushes the parent onto the modal list for RapidQ's app-modal behaviour. The sheet animation blocks about 0.25–0.7 s.
5. **§5 CPU fallback.** vello_cpu 0.3.0 + glifo 0.4 work on peniko 0.6.1 / kurbo 0.13.
   - Accuracy: within §2.4(c)'s tolerance, ≤0.33 % of pixels differing by >8, only at glyph edges.
   - Speed: 0.16–0.24 ms per frame on the CPU vs 0.8–1.2 ms on the GPU; the GPU's cold start takes 521 ms (18 ms warm).
   - On `Occluded` or `Timeout`, skip the frame without re-requesting one, and redraw on `Occluded(false)`.
   - Consider the CPU renderer for first frames and small forms.
6. **Risk 1, measured.**
   - Menu tracking holds the pump with nothing running, not even GCD main-queue blocks.
   - A blocking rfd dialog between pumps drops winit events and is cancelled by any redraw.
   - A blocking rfd dialog inside a callback stalls timers.
   - **Rule: never block on a native dialog.**
   - Open divergence: on Windows, RapidQ's timers keep firing while a menu is open. Here they stall.
7. **§1.5 headless host, verified.** No EventLoop, `NSApp` nil, no LaunchServices entry, no windows, and captures byte-identical to the windowed hosts' (GPU and CPU).

Still to verify once on an unlocked screen:
- a real hand drag of a window edge;
- a real menu-bar click, held open;
- `scripts/macos-probes.script` re-run;
- GPU windows painting.

---

## Stage 2 results (2026-10-04) — `crates/rapidr-ui-kernel`

A workspace member, GUI-free (deps: `rapidr-value`, `parley` 0.11 with its `system` fallback fonts behind the default feature `system-fonts`); `cargo check -p rapidr-ui-kernel --target wasm32-unknown-unknown` is in regress.sh's unit stage. Layout: `store.rs` (`Store`, `MemStore`), `tree.rs` (`FormUi`, `Node`, `NodeUi`: build / rebuild / sync / hit), `focus.rs`, `input.rs` (`Mods`, `Clipboard`, `KernelEvent`, the routing), `paint.rs` (`Painter`, `FormUi::paint`), `display.rs` (`DisplayList`, `Item`, `TextItem`), `text.rs` (`TextSystem`), `a11y.rs` (`FormUi::access_tree` / `access_action`), `components/{label,button,edit,trackbar,tabcontrol}.rs` behind the `KINDS` table, `tests.rs` (20 headless tests, the prototype's five included). In `rapidr-value`: `objects::ops` (`Op`, `ModelOp`, `Place`, `lift`, `edge_fills`, `focus_dots`), `objects::a11y` (`AccessNode`, `Role`, `States`, `Action`, `node_id` / `part_id`, the label rules, `mnemonic`, `to_json`; `TrackBar::describe`, `TabControl::describe`), `TabControl::tab_rect`, `objects::remove`, `text::{BUILTIN_FONTS, family_name}`.

Amendments to the plan above:

1. **§1.1 one op vocabulary.** `tabcontrol::Op` is `ops::ModelOp` (the four ops the models emit) re-exported, not an alias of the superset `ops::Op`: `gui.rs` and `gui_web.rs` match it exhaustively (and `Text` gained `place`), and runtime-core is frozen during Stage 1. `impl From<ModelOp> for Op` (`ops::lift`) converts unchanged. The models can keep emitting `ModelOp`.
2. **§1.4 kernel output.** Keys are three events, `KeyDown{chain, vk, shift}`, `KeyPress{chain, key}`, `KeyUp{…}`, so the queue holds the FLTK order: OnKeyDown, the model's `Change`, OnKeyPress. `chain` is raw (focused component … form); runtime-core applies `key_targets` / KeyPreview and INKEY$. An IME commit is `Change` then one `KeyPress` per character. Alt + a letter (mnemonics) and Cmd shortcuts type nothing.
3. **Mouse.** The model first, then OnMouseDown; on release OnClick before OnMouseUp (as the real FLTK dispatch). A **disabled** component gets no mouse events (Windows); FLTK fires OnMouseDown for one — the Stage 4 matrix will tell. Input in the in-window menu bar's strip is ignored until the menu lane draws it.
4. **§6 names.** "Nearest label" is the label *starting* to the left on the same line (a QLABEL's box is 75 wide by default and overlaps its control), else one starting above that doesn't already name a control to its right.
5. **§4 text, measured.** With `kern`, `liga` and `clig` off and `Font::pixel_size()`, parley's width is within 0.5 px of `text_size` for regular and italic Arial / Times / Courier / MS Sans Serif at several sizes (test `text_measurement_matches_text_width`). Bold differs by exactly 1 px: parley's synthetic bold keeps the advances, `text_size` adds GDI's overhang.

Open for Stage 3:
- Bold captions: give the host's text drawing a 1-px advance for synthetic bold, or accept the difference.
- runtime-core's `Store`: what `rp_comp_get` returns for unset props (a label's `color` must stay Null for "no background"; `visible`, `enabled`, `taborder`, `tabstop`, `default`, `cancel` read with defaults when Null).
- `TextItem`: the host reads the layout with `FormUi::editor_layout` while rendering (callback-safe: no program code).
- `ComponentKind::test_action` is declared, not implemented; the test-hook driver synthesizes input through the routing for the slice's fixtures.
- A multi-form `Kernel` (modal list, stacking, `HostEvent`s) and the shared `TextSystem` live in Stage 3's `src/ui/kernel.rs` / host.

## Stage 1 results (2026-10-04) — the `ui` facade

- **The facade.** `crates/rapidr-runtime-core/src/ui/` has 63 forwarded functions (the plan counted 64; a recount gives 63), plus `set_theme` / `gui_register_timer` in the prelude and `screen_size`, `work_area`, `mouse`, `monitors`, `minimize`, `message_box` (`ui/fltk_platform.rs`).
- **Features.** `desktop-ui`, `gui = desktop-ui + FLTK`, and `kernel = desktop-ui` (no dependencies yet). `kernel` isn't in rapidr-vm-host-native's `full` yet; it goes there in Stage 3.
- **Host choice.** `ui/select.rs` is a constant when only one host is built, else reads `RAPIDR_HOST` once (default `fltk`).
- **Safety net.** `object.rs` has `IN_HOST_CALLBACK` and `DEFERRED`, drained after `gui_doevents` and `gui_pump_wait`. It logs in debug builds rather than asserting. 5 unit tests.
- **Test hooks.** `ui/testhooks.rs` parses the hooks with every old quirk kept; 6 unit tests. FLTK's hooks use it.
- **Clipboard.** arboard is still tied to `gui`; the `kernel` feature gets it in Stage 3.
