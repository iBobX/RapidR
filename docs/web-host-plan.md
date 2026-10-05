# RapidR web host: the UI kernel in the browser (a canvas, vello, an ARIA mirror)

The desktop now draws every form with RapidR's own UI kernel (`docs/desktop-host-plan.md`). The web still builds each component a second time out of DOM elements (`crates/rapidr-runtime-web`: `gui_web.rs`, `object_web.rs`, `dialog_web.rs`, `menu_web.rs`, `a11y_web.rs`, `layout_web.rs`, `scroll_web.rs`, `mdi_web.rs` — about 11,800 lines, styled by `rapidr-rrcss`). Parity is held by tests, but text metrics, looks and behaviour drift, and every feature is built twice. This plan puts the kernel in the browser too, so a form is the same retained tree, the same input routing, the same display list and the same pixels on the web, native and interpreted, and both IDEs and (later) mobile sit on one UI.

## 0. Key findings that shape the plan

The spike (`crates/rapidr-ui-host-web`, `tests/web_host_spike.{html,js,mjs}`; numbers in "Spike results" at the end) answers the questions that decide the approach:

- **The browser draws byte-identical pixels.** Three fixture forms (track bars, tabs, edits, a memo, a list box, a check box, a canvas drawing) and an IDE-sized window, painted by the kernel and rasterized by vello_cpu in wasm, equal the desktop headless host's CPU capture of the same `MemStore` forms **to the byte, at 1× and 2×**, with scalar and SIMD wasm. Both sides run the very same drawing code (the desktop host's `canvas.rs` / `cpu.rs` / `images.rs`, compiled into the web crate unchanged). The only differences left are glyphs the built-in Liberation fonts lack (✓, CJK): the desktop takes them from the system's fonts, the browser has none to take.
- **It is fast enough on the CPU.** A typical form's whole frame (kernel paint + vello_cpu + `putImageData`) takes 0.2–0.5 ms with wasm SIMD (0.3–1.2 ms without), and a 1200 × 760 window with 187 components 4.8 ms at 1× and 8.3 ms at 2× (desktop native: 3.7 / 5.5 ms). The first frame is drawn about 60 ms after navigation starts. Input to the frame on screen takes 5–7 ms (median), most of it waiting for the next animation frame.
- **WebGPU works but isn't needed, and isn't everywhere.** vello on WebGPU drew the same forms within the desktop matrix's tolerance of the CPU capture (≤ 0.07 % of pixels off by more than 8). Its first frame costs 0.1 s with warm shader caches and 1.5 s cold; on a machine without a GPU, Chrome's software adapter needed 59 s. Firefox lacks WebGPU on Linux and Android in 2026. The CPU renderer is the one renderer on the web; the GPU is a later accelerator, if ever.
- **Accessibility can be exact.** The kernel's `AccessNode` tree (what AccessKit gets on the desktop) as invisible elements over the canvas gives Chrome's accessibility tree **the same roles, names, values, numbers and states** for every node (33 per round, 0 differences, after input too), with the browser's focus on the kernel's focused node, and a screen reader's click reaching the component's OnClick.
- **Text input works through the platform's own fields.** The mirror's element for an edit is a real `<input>` / `<textarea>` holding the edit's text and selection: it receives keys, input-method compositions (CDP `Input.imeSetComposition` / `insertText`), a phone keyboard's text without key events, and the clipboard events. A tap on an edit focuses it inside the gesture (what makes a phone show its keyboard).
- **The kernel needed one additive change**: its clock. std's `Instant::now` panics on `wasm32-unknown-unknown`; `rapidr_ui_kernel::tick::Instant` is now `web_time::Instant` there (performance.now()) and `std::time::Instant` everywhere else (so the desktop host and runtime-core are unchanged). Everything else — `FormUi::build / paint / mouse_* / key_* / ime_* / mouse_wheel / tick / next_wake / access_tree / access_action / focus_info`'s pieces — was already the API a web host needs.
- **Size is the cost.** The spike's wasm is 6.84 MB raw / 1.71 MB brotli, without the VM or the compiler; today's web runtime (VM, compiler, DOM GUI) is 5.13 / 1.52 MB. What the kernel brings that the runtime doesn't have yet is about 2.1 MB of code (font outlines and hinting 0.96 MB, shaping 0.33, the kernel 0.30, vello_cpu 0.38, parley 0.12) and about 1.45 MB of tables (mostly ICU4X's segmentation dictionaries); it removes about 0.52 MB of DOM GUI code. §7 has the levers.
- **A 2D canvas can lose its contents.** Headless Chrome's first page dropped every canvas's backing once (`contextlost` / `contextrestored`), leaving them blank; the page now draws the frame again on `contextrestored`. A real host must too.

---

## 1. Options compared

| | (A) Keep the DOM runtime (± a CSS framework) | (B) The kernel on a canvas, an ARIA mirror, DOM only for web things | (C) Hybrid: some components DOM, some canvas |
|---|---|---|---|
| Implementations of each component | Two (kernel, DOM) | **One** | Two for the DOM ones, one for the rest |
| Pixels, text metrics vs the desktop | Drift (browser text layout, kerning, rounding, anti-aliasing) | **Identical** (measured: 0 differing pixels) | Identical for the canvas ones only |
| Behaviour (focus, keys, selection, events) | Re-implemented per component, held by tests | **The kernel's** | Mixed |
| Accessibility | Native elements + `a11y_web.rs` mapping (Stage 12) | The same `AccessNode` tree as the desktop, as ARIA (measured exact in Chrome) | Mixed |
| Text input, IME, mobile keyboards, autofill | Native | Native through the mirror's text fields (§3.3) | Native for DOM edits |
| Find in page, page text selection, browser translation, spellcheck squiggles | Yes | No (as in a desktop app; §1.3) | Partly |
| Wasm size | Today's | + kernel, parley, vello_cpu (§7) | Both |
| Speed | Browser layout / style per change | 0.2–0.5 ms a frame for a form; damage rectangles later | Both costs |
| Web-only components (RWEBVIEW, RDOM, media) | Native | DOM overlays at the kernel's rectangles (§3.6) | Native |
| The IDE and mobile on one UI | No | **Yes** | No |

### 1.1 Recommendation: (B)

The kernel draws every RapidQ component on the web, exactly as on the desktop; the DOM is used only for (1) the invisible accessibility mirror, (2) the text fields inside it that the keyboard, input methods and phone keyboards type into, and (3) components that *are* browser things (RWEBVIEW's iframe, RDOM's elements, RWEBVIDEO / RWEBAUDIO), placed as overlays at the rectangles the kernel gives them. The CPU renderer (vello_cpu, wasm SIMD) is the renderer; WebGPU stays out until a measured need.

Reasons, in order: it is the only option with one implementation per thing (the user's rule, ROADMAP principle 7), the only one where "identical behaviour on web, native and interpreted" is true by construction rather than by tests, the pixels and accessibility tree are already proven identical, the desktop's lanes (lists, text, dialogs, menus, MDI) arrive on the web for free, and the IDE (Phase 3) and mobile (Phase 7) get one UI. The costs are wasm size (§7), browser features a canvas doesn't have (§1.3), and the risks in §6.

**(C) is rejected** because it keeps the drift where it hurts most (text editing, lists) and splits each form's pixels between two renderers. (B) already uses the browser's own text fields for input — invisibly — which is what (C) would have bought.

### 1.2 A CSS framework (Tailwind CSS)?

Tailwind is MIT-licensed (its v4 engine uses Lightning CSS, MPL-2.0, only at build time; its output CSS isn't covered), so it is allowed. It is not useful here:

- **What it would solve:** styling consistency and less handwritten CSS for a DOM UI with a fixed set of looks — e.g. an app's or an IDE's HTML chrome.
- **What it wouldn't:** behaviour (focus order, keys, selection, events are code), text metrics (browser layout vs `TextWidth`), pixel identity with the desktop, or a look driven by run-time properties (each component's Color, Font, BorderStyle are set by the program; Tailwind generates CSS at build time from class names it finds in source files, so program values would need inline styles or a safelist of every colour anyway). The runtime's base stylesheet (`rapidr-rrcss`) is 42 lines.
- **In the recommended end state** there is no hand-styled DOM left to apply it to: forms are kernel-drawn, the mirror is invisible, overlays carry the program's own HTML (an RDOM program can use any CSS it likes, Tailwind included, on its own terms). Adding a Node build step to the runtime for it has negative value. **Don't add it.**

### 1.3 What a canvas UI gives up in a browser, and what the plan does about it

- **Find in page (Ctrl+F)** finds the mirror's text (labels' captions, edits' values, list rows) but highlights nothing visible. A desktop app has no find-in-page either; programs that want search provide it. Kept as is.
- **Selecting text on the page** (not inside an edit) isn't possible, as on the desktop. Copying from edits, memos and grids works (the kernel's selection, the clipboard events).
- **Browser zoom and devicePixelRatio**: handled — a scale change re-rasterizes at the new device resolution (`SpikeForm::set_scale`, `matchMedia(resolution)`); the kernel's OnScaleChanged fires as on the desktop.
- **Spellcheck**: the browser could check the mirror's fields but its squiggles would be invisible; RapidQ's edits have none on the desktop either. Off (`spellcheck=false`).
- **Autofill and password managers**: they fill the mirror's fields, which are real `<input>`s. The host gives them `autocomplete` / `name` hints from new optional properties and reads a filled value back as the user's edit (an `input` event without `beforeinput`). Stage W3.
- **Browser translation, reader mode, extensions that rewrite text**: they see the mirror only; nothing visible changes. Acceptable for applications.
- **SEO**: irrelevant (RapidR programs are applications).
- **Printing the page** prints the canvases' bitmaps; RapidQ's QPRINTER keeps making a PDF (`object_web.rs::web_print`).

---

## 2. Licences

Everything the web host ships, and everything a program built with it ships, is under a permissive licence allowing commercial use by RapidR and by its users; `cargo deny check licenses` passes with and without the spike's `gpu` feature. Versions are the workspace lock's.

| Component | Version | Licence | Notes |
|---|---|---|---|
| rapidr-ui-kernel, rapidr-value, rapidr-ui-host-web | 2.113.0 | MIT (RapidR's) | |
| vello_cpu, vello_common | 0.3.0 | Apache-2.0 OR MIT | the CPU renderer |
| vello, vello_encoding, vello_shaders | 0.11.0 | Apache-2.0 OR MIT | only with WebGPU |
| peniko, kurbo, color, linebender_resource_handle | 0.6.1, 0.13.1, 0.3.3, 0.1.1 | Apache-2.0 OR MIT | |
| fearless_simd, glifo | 0.7.0, 0.4.0 | Apache-2.0 OR MIT | |
| parley, fontique, parley_data, parlance | 0.11.1 / 0.1.0 | Apache-2.0 OR MIT | text layout and editing |
| harfrust | 0.12.0 | MIT | shaping (HarfBuzz's port) |
| skrifa, read-fonts, font-types | 0.44.0, 0.41.0, 0.12.6 | MIT OR Apache-2.0 | outlines, hinting (swash isn't used) |
| ICU4X (icu_segmenter, icu_properties, icu_normalizer, their data, zerovec …) | 2.1 | Unicode-3.0 | permissive; notice kept |
| wgpu, wgpu-core, wgpu-hal, wgpu-types, naga | 30.0.1 | MIT OR Apache-2.0 | only with WebGPU |
| wasm-bindgen, js-sys, web-sys, wasm-bindgen-futures | 0.2.129, 0.3.106, 0.4.79 | MIT OR Apache-2.0 | |
| web-time | 1.1.0 | MIT OR Apache-2.0 | the kernel's clock on wasm |
| bytemuck, foldhash, hashbrown, smallvec … | | Zlib / MIT / Apache-2.0 | the rest of the 111 (CPU) / 149 (with WebGPU) crates |
| tiny-skia, resvg / usvg, png, jpeg-decoder, ttf-parser | | BSD-3-Clause, Apache-2.0 OR MIT, MIT OR Apache-2.0 | already shipped by today's web runtime (pictures, SVG) |
| Liberation Sans / Serif / Mono 2.1.5 | | SIL OFL 1.1 | see below |
| (later) Noto fallback fonts | | SIL OFL 1.1 | §3.7 |
| Tailwind CSS (not adopted) | v4 | MIT | §1.2 |

**The fonts.** The OFL 1.1 lets the fonts "be bundled, embedded, redistributed and/or sold with any software"; programs and documents made with them aren't covered by it. So embedding them in RapidR and in users' commercial programs is fine, on two conditions: the copyright notice and licence travel with the font software (RapidR ships `crates/rapidr-value/fonts/OFL-1.1.txt` and `AUTHORS`; programs built by `rapidr build` / `bundle-bc` should carry the same notice in their notices file — to check in Stage W7), and a **modified** font (a subset made to save bytes is a modification) must not use the Reserved Font Names ("Liberation"; "Arimo", "Tinos", "Cousine"). The plan therefore ships the fonts unmodified (brotli on the wire roughly halves them) and doesn't subset them.

**The notices.** The spike crate isn't in any regular build, so THIRD_PARTY_NOTICES.md wasn't regenerated (`--check` still passes: every crate it links already ships). Adopting the host (Stage W3) makes it a shipped crate; the notices are regenerated then.

---

## 3. Architecture

### 3.1 Crates

```
rapidr-value            models, ops, a11y rules (wasm-safe)          unchanged
rapidr-ui-kernel        GUI-free, wasm-safe                          + tick::Instant (done)
rapidr-ui-render   NEW  the display list → vello_cpu / vello         from host-winit's canvas.rs, cpu.rs,
                        (Canvas trait, Painter, images cache)         images.rs, gpu.rs (Stage W1)
rapidr-ui-app      NEW  host-neutral program glue: KernelEvent →     from runtime-core's ui/kernel.rs,
                        program events, show / close / modal,        ui/testhooks.rs (Stage W2, done:
                        timers, dialogs' state, test hooks           "W2 results")
rapidr-ui-host-winit    desktop windows                               uses render + app
rapidr-ui-host-web      browser: canvases, DOM input, the mirror     the spike, grown (Stage W3)
rapidr-runtime-web      the web runtime: object store, builtins      loses gui_web & co. (Stage W11)
```

- The spike compiles the desktop's drawing files into the web crate with `#[path]` (so the comparison compares the same code). Stage W1 moves them into `rapidr-ui-render`, which both hosts depend on; feature `gpu` adds vello / wgpu. Nothing in it depends on winit or the DOM.
- `rapidr-ui-host-web` never depends on `rapidr-runtime-web` (the desktop rule, §1.1 of the desktop plan): its callbacks route input into the kernel and return events; the runtime dispatches them.

### 3.2 The store, the VM's time slices, and the event loop

- **The store.** The web runtime's registry (`object_web.rs`: `COMPONENTS` of `RpComponent { type_name, properties, creation_order }`, with `rp_comp_get` / `rp_comp_set` routing shared models' properties to `rapidr_value::objects`) is the same shape as runtime-core's. A `WebStore` implements the kernel's `Store` over `rp_comp_get`, `rp_comp_type` and the children by Parent in creation order, with `RtStore`'s rules (a QLABEL's, QFORM's and QPANEL's creation-default white Color read as unset). Better, in Stage W2: one registry for both runtimes in shared code, so there's one `Store` and one set of property defaults. Kernel-drawn dialogs keep their `MemStore`s (`dialogs::PREFIX`), as on the desktop.
- **The rule from the desktop holds: callbacks never run program code.** A DOM event handler passes the input to the kernel (`FormUi::mouse_down` …) and collects its `KernelEvent`s; nothing else. This is already the web runtime's model: DOM events queue bytecode handlers, which the VM runs at its next safe point, in time slices (`interpreter/rapidr-vm-host-web`: `Host::yield_now`, `continue_slice` over a `MessageChannel`).
- **One turn of the page's loop:**

  ```
  DOM event   → kernel input (synchronous; the mirror's focus moved within the
                gesture) → KernelEvents → app::dispatch → rp_fire_event (queues the
                VM's handler, or runs a native web build's at once) → schedule()
  VM slice    → handlers run, the store changes → forms marked dirty → schedule()
  schedule()  → one requestAnimationFrame: for each dirty form, kernel paint →
                vello_cpu → putImageData (the damage rectangles later); the mirror
                synced; then a setTimeout for the earliest deadline (the kernel's
                next_wake: caret, held scroll bars; the program's QTIMERs)
  ```

- **Timers** move from one `setTimeout` per QTIMER (`TIMER_HANDLES`) to the shared timer heap of `rapidr-ui-app` (the desktop's: Interval / Enabled re-read each tick, re-armed after the handler), driven by one `setTimeout` for the earliest due. Same order of events on both hosts.
- **Waits.** ShowModal, MESSAGEBOX / MESSAGEDLG and the colour / font dialogs keep the VM's protocol (`Wait::Form`, the VM suspended, resumed when the form closes); the dialogs become the kernel's `MemStore` forms (`Dialog::message / color / font`), so `dialog_web.rs`'s page dialogs go. DOEVENTS ends the slice. A native web build (generated Rust compiled to wasm) can't block either: it keeps today's web behaviour (ShowModal returns at once where nothing can wait), until generated code gets resumable waits (outside this plan).

### 3.3 Input

- **Pointer events** on each form's canvas (CSS pixels = the kernel's logical pixels): `pointerdown` (with `setPointerCapture`, and `preventDefault` so the page doesn't take the focus), `pointermove`, `pointerup` (+ `edit_commands` for an edit's context-menu pick, as `Desktop::mouse_up`), `pointerleave`, `wheel` (notches positive down: a line mode's 3 lines, a pixel mode's 48 logical pixels — the desktop host's touchpad rule), `contextmenu` suppressed (the kernel's own menus). Touch is pointer events: a tap is a click; long press = right click and gestures are Phase 7's.
- **Keys** reach the mirror's focused element and bubble to its form: `KeyboardEvent.key` / `.code` → Windows' virtual keys (`rapidr_value::input::vk_of_key`, already shared), the typed text only for a printable key without Ctrl / Cmd, modifiers as the platform's (Cmd is the shortcut key and Option moves by words on a Mac). The kernel handles them (Tab order, mnemonics, Default / Cancel buttons, the menus, the edits) and the host prevents the browser's default — except for the browser's own (reload, zoom, tabs, devtools) and Cmd/Ctrl + C / X / V (below). Tab stays inside the window, as on the desktop; when a program is embedded in a larger page, F6 / Ctrl+Tab leaves it (WCAG 2.1.2, no keyboard trap) — Stage W3.
- **Input methods, phone keyboards, dictation, the emoji picker**: the edit's mirror element is a real text field holding the kernel's text and selection, so the browser's machinery works unchanged: `compositionupdate` → `FormUi::ime_preedit`, `compositionend` → `ime_commit` (OnChange, then OnKeyPress per character, as the desktop); `beforeinput` without a key (`insertText`, `insertReplacementText`, `deleteContentBackward`, `insertLineBreak` …) → the same commits and keys. The field lays its text out with the same Liberation fonts (`@font-face`), so the browser's caret — where input methods put their window — falls where the kernel draws its caret for unscrolled text; Stage W3 aligns it from `FormUi::ime_area` for scrolled edits and memos, and uses `beforeinput.getTargetRanges()` for autocorrect's replacements.
- **The clipboard** through the clipboard events, synchronously and without a permission prompt: `copy` / `cut` run the kernel's Cmd/Ctrl + C / X with a `MemClipboard` and put its text in `clipboardData`; `paste` hands `clipboardData`'s text to Cmd/Ctrl + V. A kernel context menu's Copy / Cut uses `navigator.clipboard.writeText` (allowed in the gesture). Its **Paste** needs `navigator.clipboard.readText()`, which is asynchronous and may prompt: the host reads first, then runs the pick (a small async step in the host; the kernel's `Clipboard` stays synchronous).

### 3.4 Drawing

- **One canvas per shown form** (its client area plus the kernel-drawn window frame: title bar, borders, buttons — the MDI child frame's drawing, `components/mdi.rs`, generalized into a `WindowFrame` both QFORMMDI children and web windows use). The page is the screen (Screen.Width / Height = the viewport); windows are stacked, moved and sized by the host (the frame's drag as `Container::Mdi`-like actions), so dragging, maximize (WindowState) and the size grip work as on the desktop.
- **The CPU renderer**: vello_cpu into the form's pixmap, `putImageData` into a 2D canvas of the form's size × devicePixelRatio, CSS-sized to the form's logical size. wasm SIMD (`-C target-feature=+simd128`) on: 2.4–2.8× faster, the same bytes; every 2026 browser has it, so the build is SIMD-only.
- **Damage rectangles** (Stage W9): the kernel already knows what changed (`dirty`, per node); a frame rasterizes only the changed rectangles (vello_cpu with a clip) and `putImageData(…, dirtyX, dirtyY, w, h)` them. The big window's 8 ms full frame then becomes a component's fraction of a millisecond for a caret blink or a typed key.
- **Context loss**: on `contextrestored`, the form's last frame is drawn again (the spike does). Hidden forms drop their canvases.
- **WebGPU, later and optional**: the same display list through `gpu.rs` (the spike's `gpu_web.rs`), never on a software adapter (`adapter.info` / `isFallbackAdapter`), and only after the CPU frame (first frames, small forms) — the desktop plan's Stage 0 advice. Decided by measurement (big plots, animation), not by default.

### 3.5 The accessibility mirror

- After each frame of a changed form, the kernel's `access_tree` (the Stage 12 rules in `rapidr_value::objects::a11y`) becomes elements: one per node, nested as the nodes are, at the node's bounds, transparent, `pointer-events: none`. The element is chosen so the browser's own semantics say what the node says: a text box is `<input>` / `<textarea>` (its value the text), a label its text (StaticText), a canvas a `<canvas>`, everything else a `<div>` with the role (a form, which ARIA has no window for, is a dialog; a pane a group) and `aria-checked / pressed / selected / expanded / disabled / readonly / level / valuenow / valuemin / valuemax / orientation / keyshortcuts / description`, a status bar `role=status` (polite, atomic). Element ids are the node ids (stable hashes), so patches are small (0.18 ms a frame in the spike).
- **Focus**: the DOM focus is on the kernel's focused node (Chrome's focused node is the kernel's, checked); a screen reader moving the focus is the kernel's focus (`Action::Focus`), its click (`click` on the element: VoiceOver's VO-Space, NVDA's Enter) is `Action::Click`, so OnClick fires as for a user.
- The mirror is built in Rust (Stage W3: `web-sys`, today JS in the spike page) from `rapidr-ui-host-web::aria`. `a11y_web.rs`'s mapping onto DOM elements is no longer needed; the shared rules stay where they are.
- **Found in the spike, for both hosts**: a focused tab control reports the tab list as focused (the kernel's node is the component); ARIA's convention is the selected tab. A kernel `describe` change (the focused state on the selected tab), checked on AccessKit and in Chrome.

### 3.6 Web-only components as overlays

RWEBVIEW (an `<iframe>`, sandboxed as today), RDOM (the program's elements), RWEBVIDEO / RWEBAUDIO, and later RPLOT if it keeps an HTML renderer, are DOM elements placed over the canvas at the kernel node's absolute rectangle and clipped to its parents' client areas; the kernel draws nothing there and routes no pointer events there (a new `ComponentKind::overlay` → the host's overlay list after each frame). Non-visual web components (RJAVASCRIPT, RWEBSTORAGE, RWEBGEOLOCATION, RWEBNOTIFICATION, RROUTER) are untouched. **Stacking**: a DOM overlay is above its form's canvas, so a kernel drop-down or menu opening over it would be hidden. The kernel already paints those last (`combo::paint_popup`, `paint_menus`); the host asks for them as a second display list and draws it into a second canvas above the overlays.

### 3.7 Fonts

- The built-in Liberation fonts become cached assets beside the wasm (fetched once, registered in fontique), not bytes inside it: −1.12 MB from every wasm (today's runtime embeds them too), cached across releases.
- **Fallback fonts, the same everywhere.** The only pixel difference measured is ✓ and CJK, which the desktop takes from the system. For "identical on every runtime", RapidR ships one fallback set — Noto Sans (symbols), Noto Sans CJK and Noto Color Emoji (OFL 1.1) split by Unicode range — loaded on demand on the web when a layout meets a missing glyph (then laid out again), and used on the desktop before the system's fonts. Captures and tests use exactly that set on both hosts.

### 3.8 Dialogs, menus, printing

- **Open / Save**: `ui/file_dialog.rs`'s `Request` and answer (FileName, Files(…), FilterIndex …) are host-neutral; on the web the `pick` is `<input type=file>` (or the File System Access API's `showOpenFilePicker` / `showSaveFilePicker` where present), into the page's file store (`object_web.rs`'s `SAVED_FILES` / uploads). `RAPIDR_TEST_FILE_DIALOG`'s answer works the same.
- **MessageBox, MessageDlg, colour and font dialogs**: the kernel's (`dialogs.rs`), identical to the desktop's; `dialog_web.rs` goes.
- **Menus**: the kernel's in-window bar and pop-ups (`menubar.rs`, `popupmenu.rs`), as on Windows and Linux; F10 / Alt are the kernel's when the browser lets them through (Alt alone opens the browser's menu on some platforms: tested per browser).
- **Printing**: unchanged (QPRINTER → PDF → the browser's viewer).

### 3.9 The IDE

The Phase 3 MDI IDE is written once and runs on the kernel on both hosts: `examples/ide.rr` already runs on `RAPIDR_HOST=kernel` (designer, code editor, property grid, the event editor). On the web the same program runs in the page instead of the DOM IDE (`web-ide/`, ~9,900 lines of JS / HTML / CSS) and the program under test runs in the workspace as MDI windows rather than a preview iframe. The code editor is the kernel's RCODEEDITOR; Monaco (MIT) retires when the kernel editor has the language service's UI (completion list, signature help, hover, diagnostics' squiggles) — the Phase 3 language-service crate built native and wasm feeds both. The DOM IDE is deleted with the DOM runtime (§5, no long-term fallback).

### 3.10 Mobile (Phase 7)

The same web host, inside Tauri 2's iOS / Android shells (ROADMAP step 1): the canvas, the mirror (VoiceOver / TalkBack read it), the text fields (the phone's keyboard, verified emulated: a tap focuses the field inside the gesture, the keyboard's text arrives). Additions there: `inputmode` / `enterkeyhint` on the mirror's fields from the component (a numeric edit → `inputmode=numeric`), `visualViewport` (the keyboard covering the window), touch gestures in the kernel's input (long press = right click, pan = the wheel), safe areas. Step 2 (the kernel's renderer natively) stays optional.

---

## 4. Stages

A session is one focused agent session ending in a green commit.

| Stage | Content | Sessions | Who |
|---|---|---|---|
| W0 | Spike (this document's results) | done | |
| W1 | `rapidr-ui-render`: `canvas.rs`, `cpu.rs`, `images.rs`, `gpu.rs` out of the desktop host (no behaviour change; the desktop matrix and the spike's byte-identity test green); the spike on it; wasm SIMD build | done (W1 results) | Single owner |
| W2 | `rapidr-ui-app`: the host-neutral half of runtime-core's `ui/kernel.rs` (KernelEvent → `rp_fire_event*`, Set / List / Container / MenuPick dispatch, show / hide / close actions / OnShow / modal list, the timer heap, `form_resized`, WindowState's simulation) and `ui/testhooks.rs`, behind a small `Program` trait both runtimes implement; ideally one component registry for both runtimes | done (W2 results: `Program` + `Windows`; the registry's path) | Single owner, with the desktop lanes' integrator |
| W3 | The web host proper: `WebHost` (canvases per form, kernel-drawn frames, stacking, moving, sizing, dpr, context loss, rAF and deadlines), input / IME / clipboard / autofill, the mirror in Rust, `WebStore`; `rapidr-runtime-web` feature `kernel` and `?host=kernel` (the DOM host stays the default) | done (W3 results: 64 of 68 browser cases on the kernel host) | Single owner |
| W4 | The VM and native web builds on it: slices + dispatch, ShowModal / dialogs / DOEVENTS / INKEY$, the kernel's dialogs, timers through `rapidr-ui-app` | 3–4 | Single owner |
| W5 | Parity: `web_gui_parity.mjs` on the kernel host (dumps equal to the desktop's), pixel captures equal to the desktop headless host's at 1× and 2× (and 1.5×), `web_a11y.mjs` on the mirror; each case gets `webKernel: true | "pending: …"` | 2–3 | Tests lane |
| W6 | Web-only components as overlays, the popup layer above them | 2–3 | Parallel after W3 |
| W7 | Fonts as assets, the shared Noto fallback set (web on demand, desktop and captures), the OFL notice in built programs | 2–3 | Parallel after W3 |
| W8 | File dialogs on the web through `file_dialog.rs`; `dialog_web.rs` retired | 1–2 | Parallel after W4 |
| W9 | Performance: damage rectangles, the size levers (§7), WebGPU decided by measurement | 2–3 | Parallel after W3 |
| W10 | The web IDE on the kernel (Phase 3's MDI IDE in the page) | in Phase 3's budget | |
| W11 | Switch: once W5's parity holds, the kernel host becomes the only web host and the DOM runtime is deleted in the same step — no release with a `?host=dom` fallback (the user's rule, 2026-10-04; §5) | 2 | Single owner |

Total about 22–31 sessions besides W10. W1–W4 are on the critical path, in order; W5–W9 run in parallel lanes after W3 / W4.

**Lanes after W3**: rendering and performance (W9, `rapidr-ui-render`), fonts (W7), overlays and dialogs (W6, W8), tests (W5), each owning its files; `rapidr-ui-host-web`'s `app` loop and `rapidr-ui-app` have one integrator, as `ui/kernel.rs` has on the desktop.

### 4.1 Tests

- **Pixels as the parity test.** The web host captures each fixture's forms (the wasm's own pixels) after its events; the desktop's headless host captures the same fixture; the two must be **byte-identical** at 1× and 2× (the spike shows this holds when both use the same fonts — hence the shared fallback set in captures). A failure is a real difference, not a tolerance question. WebGPU, if adopted, is held to the desktop matrix's tolerance (±8 on ≤ 0.5 % of pixels) against the CPU.
- **Dumps**: `tests/web_gui_parity.mjs` / `web_gui_run.mjs` run the fixtures on the kernel host and compare `RAPIDR_TEST_DUMP`'s lines with the desktop's, as today; the four-way desktop matrix becomes six-way (native / interpreted × FLTK-free desktop × web).
- **Accessibility**: `tests/web_a11y.mjs` compares Chrome's tree with the kernel's JSON per fixture (the spike's comparison is the same check over the mirror: it should become exact for every node, no representation notes).
- **Input**: Playwright's real mouse, keyboard, touch and CDP input-method events (the spike's), plus the existing web suites (`web_conformance`, `web_vm_yield`, `web_end_timer`, `web_bundle_*`, `web_ide_*` until W10) re-pointed at the kernel host.
- **By hand on devices** before the switch: VoiceOver (macOS, iOS), NVDA and JAWS (Windows), TalkBack (Android), Narrator; Japanese / Chinese / Korean input methods on each OS; iOS and Android keyboards' autocorrect and dictation; Firefox and Safari as well as Chrome.

---

## 5. Migration end state (the user's rule: one implementation, no long-term fallback)

At W11, once parity holds (no release keeps `?host=dom` as an escape hatch), delete: `gui_web.rs`, `dialog_web.rs`, `menu_web.rs`, `a11y_web.rs`, `layout_web.rs`, `scroll_web.rs`, `mdi_web.rs`, the DOM half of `object_web.rs` (the registry stays, shared since W2), `rapidr-rrcss` (unless RDOM wants a base stylesheet), the DOM IDE after W10, and the DOM-specific parts of the web tests. The web runtime keeps what isn't GUI (builtins, files, storage, network, database, data science, the VM host) and the web-only components' overlays. Every component then has one implementation (the kernel's), one look, one accessibility tree and one input path, on the desktop, the web and, later, mobile.

---

## 6. Risks and mitigations

1. **Real screen readers over the mirror** (the most serious): Chrome's tree is exact, but VoiceOver, NVDA, JAWS and TalkBack each have quirks (virtual cursor navigation, live regions, focus moves they didn't cause). Mitigation: native elements wherever a node has one (inputs, textareas, later `<select>`-like structures where they fit), real-AT testing from W3 on, not at the end; Flutter's web semantics tree is a production precedent for the architecture.
2. **Input methods and phone keyboards**: candidate-window placement, autocorrect's replacement ranges, composition quirks per OS / keyboard. Mitigation: the field holds the real text and selection (so the platform's own logic works), `ime_area` placement, device testing (§4.1).
3. **Wasm size** (+~2.6 MB raw / ~+0.6 MB brotli before work): the levers in §7; measured per release.
4. **Canvas memory and context loss**: a 1200 × 760 form at devicePixelRatio 3 is 33 MB of pixels. Mitigation: only shown forms keep canvases, redraw on `contextrestored`, damage rectangles.
5. **WebGPU fragmentation**: not depended on (CPU renderer), never on a software adapter.
6. **The glue extraction (W2) touches runtime-core's shared files** while desktop lanes work there. Mitigation: one owner with the desktop integrator, a no-behaviour-change commit checked by the full desktop matrix first.
7. **Keyboard conflicts with the browser** (F10, Alt, Ctrl+W, Ctrl+T, Ctrl+N can't be taken): document them per browser; in an installed PWA / Tauri shell most become available.
8. **Fractional device scales** (1.25, 1.5): snapping identical on both hosts is tested at 1.5× in W5.

---

## 7. Size

Measured (bytes / 10⁶; brotli at quality 11):

| Build | Raw | gzip -9 | brotli |
|---|---|---|---|
| Today's web runtime (VM + compiler + DOM GUI; `target/web/rapidrintr_bg.wasm`, built 2026-10-04) | 5.13 | 2.11 | 1.52 |
| Spike, vello_cpu (no VM, no compiler) | 6.84 | 2.97 | 1.71 |
| Spike, wasm SIMD | 6.64 | 2.90 | 1.68 |
| Spike, opt-level s + LTO | 7.40 | 2.92 | 1.65 |
| Spike + vello on WebGPU | 7.87 | 3.22 | 1.86 |

Where the spike's bytes go (twiggy, before wasm-opt; the part today's runtime doesn't have): skrifa + read-fonts 0.96 MB, vello_cpu + vello_common + fearless_simd + glifo + kurbo 0.38, harfrust 0.33, the kernel 0.30, parley + fontique 0.12, and about 1.45 MB more data than the runtime's (whose 1.34 MB is mostly the same fonts) — by the data crates' sizes, mostly ICU4X's segmentation dictionaries, which parley's `LineSegmenter::new_dictionary` / `WordSegmenter::new_dictionary` pull in for CJK and South-East Asian word breaking. The DOM GUI code the switch removes is about 0.52 MB (`gui_web` 281 KB, `object_web` 104, `dialog_web` 57, `a11y_web` 43, `layout_web` 14, `scroll_web` 13, `menu_web` 12); what the spike lacks of today's runtime (the VM, the compiler, the non-GUI runtime) is about 0.9 MB of code. A kernel-based runtime would so be about 7.6–8 MB raw / 2.0–2.2 MB brotli before size work (today: 5.13 / 1.52).

Levers, in order of effect: the fonts as cached assets (−1.12 MB raw / −0.42 MB brotli from every wasm, today's runtime's too; downloaded once, cached across releases); rule-based segmenters without the dictionaries (GDI breaks lines by rules too; needs a parley option — upstream, Apache-2.0 / MIT); wasm SIMD (smaller and 2.5× faster); size-tuned profiles for the web build (LTO, opt-level s: −4 % brotli); the GPU path left out.

---

## Spike results (2026-10-04)

Code: `crates/rapidr-ui-host-web` (`lib.rs`: `SpikeForm`, the wasm API; `forms.rs`: three fixtures' forms and an IDE-sized one built in `MemStore`s, a few lines of "program" answering their events; `aria.rs`: the mirror's elements from `AccessNode`s; `gpu_web.rs`: vello on WebGPU, feature `gpu`; `examples/desktop_capture.rs`: the desktop host's CPU captures of the same forms), `tests/web_host_spike.html` + `.js` (the page), `tests/web_host_spike.mjs` (the measurements and checks: 57 passing). Kernel: `tick::Instant` (web-time on wasm32-unknown-unknown; std elsewhere), `NodeUi::wake` and `FormUi::last_click` typed with it. A workspace member, out of every regular build (no runtime, bundle or IDE uses it); `cargo check --workspace`, `cargo check -p rapidr-ui-kernel --target wasm32-unknown-unknown`, the kernel's 86 and the desktop host's 14 tests, `cargo deny check licenses` and `third_party_notices.py --check` pass.

Run it: `cargo run -p rapidr-ui-host-web --example desktop_capture --release`; `wasm-pack build crates/rapidr-ui-host-web --target web --release --out-dir ../../target/web-host-spike` (SIMD: with `RUSTFLAGS="-C target-feature=+simd128"`, `CARGO_TARGET_DIR="$PWD/target/simd"` (its own target directory: the flags rebuild everything; a relative one lands under the crate, where wasm-pack runs cargo) and `--out-dir ../../target/web-host-spike-simd`; WebGPU: `-- --features gpu` into `web-host-spike-gpu`); serve the repository; `node tests/web_host_spike.mjs` (`RAPIDR_URL`, default `http://127.0.0.1:8782`; `RAPIDR_COMPARE_WASM` for today's runtime). Measured in Playwright's Chromium (headless) on this Mac (Apple silicon, macOS 27).

**Pixels** (the wasm's frame against the desktop's `cpu::capture` of the same form; also checked: the canvas read back and an element screenshot equal the wasm's pixels):

| Form | Size at 1× | With the same (built-in) fonts, 1× and 2×, scalar and SIMD | Against the desktop with system fallback fonts |
|---|---|---|---|
| trackbar (`trackbar.bas`) | 318 × 209 | 0 pixels differ | 0 |
| texts (`tab_control.bas` + edits, memo, check box) | 352 × 330 | 0 | 1,451 of 116,160 (1×), 4,125 of 464,640 (2×), all inside the two text runs with ✓ and CJK, which the desktop draws with macOS's fonts and the browser as missing-glyph boxes |
| lists (`canvas_onpaint.bas` + a list box) | 400 × 170 | 0 | 0 |
| big (1200 × 760, 187 components, 1,027 display items) | 1200 × 760 | 0 | — |
| WebGPU vs the CPU capture, 2× | | trackbar 637 differ (121 by > 8, max 43), texts 2,516 (329, max 52), lists 1,331 (195, max 56): 0.05–0.07 % by > 8, within the desktop matrix's tolerance | |

**Times** (means of 60 forced full frames; `performance.now()`'s resolution is 0.1 ms):

| | 1× | 2× | 1×, SIMD | 2×, SIMD | Desktop native (vello_cpu, NEON) 1× / 2× |
|---|---|---|---|---|---|
| trackbar | 0.31 ms | 0.51 | 0.17 | 0.25 | 0.08 / 0.12 |
| texts | 0.67 | 1.18 | 0.29 | 0.47 | 0.18 / 0.29 |
| lists | 0.42 | 0.75 | 0.19 | 0.30 | 0.12 / 0.20 |
| big | 11.1 | 19.4 | 4.8 | 8.3 | 3.7 / 5.5 |

Of a frame, the kernel's paint is 0.01–0.05 ms (0.6 ms for the big window), `putImageData` 0.01–0.3 ms (plus the compositor's upload, not measured), the rest vello_cpu. Startup: wasm fetched, compiled and instantiated in 14–19 ms (6.8 MB from a local server), the fonts registered in 1 ms, the first form built in 8 ms (later ones 1 ms), its first frame 15 ms (13 ms of it the first rasterization: glyph caches), the page's three forms all drawn at 57–60 ms after navigation start. Input (47 clicks, keys, wheel turns, compositions) to the frame drawn: median 5–7 ms, p95 21–27 ms, dominated by the wait for the next animation frame; the handlers themselves 0.4–0.5 ms; the mirror's sync 0.18 ms a frame. WebGPU: first frame 0.1 s with Chrome's shader cache warm, 1.5 s cold (the device and vello's pipelines), 59 s on Chrome's software adapter (`--enable-unsafe-webgpu` without a GPU backend); then 0.12–0.17 ms of CPU a frame (the GPU's work not waited for). Headless Chrome needs `--enable-unsafe-webgpu --use-angle=metal --enable-features=WebGPU` for the Apple GPU.

**Input and accessibility** (all checked by the test): a page click, → and a button through the kernel's routing reach the track bar's model and the program's handlers (`102111|10|5,6,|4`, the fixture's answer); a click on an edit gives its mirror `<input>` the DOM focus; typing, a composition (not text until committed, then committed), text without a key (a phone's keyboard), copy of a selection and paste all reach the edit's model; a tab, a list row, the wheel on a list and a canvas button act; an emulated phone's tap focuses the edit's field and its keyboard's text types. Chrome's accessibility tree equals the kernel's for all 33 nodes of the three forms at load and after input (roles, names, values, numbers, checked / selected / expanded / disabled, multiline, shortcuts, descriptions); Chrome's focused node is the kernel's; a screen reader's click on the mirror's button runs its OnClick.

Amendments and findings:

1. **Kernel**: `tick::Instant` (above). No other kernel change was needed.
2. **Context loss**: Chrome dropped the first page's canvas backings once (3 `contextlost` events): the page redraws on `contextrestored` (§3.4).
3. **Fallback fonts** decide the remaining pixel differences (§3.7).
4. **Focus on a tab control** is the tab list, not the selected tab, on both hosts (§3.5).
5. **SwiftShader WebGPU is unusable** for vello (59 s first frame): never on a fallback adapter.
6. The spike's page glue (the mirror's DOM patching, the listeners) is JavaScript for speed of iteration; Stage W3 moves it into the Rust host (`web-sys`), leaving the page a loader.

Open (for W3 onward): real screen readers and devices (§4.1); IME window placement for scrolled edits; autofill; the kernel menu's Paste through `navigator.clipboard.readText`; window frames, stacking and resizing on the page; the runtime's store, the VM, timers and test hooks (W2–W4); Firefox and Safari (the spike ran in Chromium only).

Sources for WebGPU's availability: [web.dev — WebGPU is now supported in major browsers](https://web.dev/blog/webgpu-supported-major-browsers) (Chrome / Edge 113+ on Windows, macOS, ChromeOS; Android 121+; Safari 26 on macOS, iOS, iPadOS, visionOS; Firefox 141 on Windows, 145 on Apple-silicon macOS; Firefox on Linux and Android in progress, Android targeted late 2026), and [Chromium's 2026 additions](https://www.youngju.dev/transcribe/culture/2026-05-16-webgpu-wgsl-ecosystem-2026-chrome-safari-firefox-wgpu-naga-tint-three-js-babylon-webllm-transformers-deep-dive) (Linux Intel Gen12+, compatibility mode on older Android).

---

## W1 results (2026-10-04)

**`crates/rapidr-ui-render`** holds the drawing both hosts use, moved out of `rapidr-ui-host-winit` with `git mv` and changed only in imports and doc comments: `canvas.rs` (the display list as the `Canvas` primitives: `Painter`, `draw_list`, `draw_layout`, `device_size`), `cpu.rs` (vello_cpu: `CpuRenderer`, `capture`), `images.rs` (the picture cache, with its test) and, behind feature `gpu`, `gpu.rs` (the vello scene, renderer and parameters). kurbo and peniko come through vello_cpu's re-exports (the crates vello uses), so the CPU path needs no vello; feature `png` turns on vello_cpu's PNG decoder (colour bitmap glyphs among the system's fallback fonts). Nothing in it knows winit or the DOM.

- **The desktop host** depends on it with `gpu` and `png` — the very features its vello and vello_cpu had (vello_cpu's defaults: std, png, text, u8_pipeline) — and keeps vello itself only for its windows' surfaces and devices (`vello::util`, wgpu). `lib.rs` loses the four modules and `capture` calls `rapidr_ui_render::cpu::capture`; `winit_host.rs` changes two imports. parley, vello_cpu and glifo are no longer its direct dependencies.
- **The spike** depends on it without features (no PNG decoder: a browser has no system fonts with bitmap glyphs); its own `gpu` feature turns on `rapidr-ui-render/gpu`. It no longer reaches into the desktop host: no `#[path]`, no `extern crate vello_cpu as vello`, no dev-dependency on `rapidr-ui-host-winit`; `examples/desktop_capture.rs` uses `rapidr-ui-render` with `png`, as the desktop host builds it.
- **wasm SIMD is the spike's default build.** `tools/build_web_host_spike.sh [--gpu] [--no-scalar]` builds it with `-C target-feature=+simd128` into `target/web-host-spike` (what the page loads), without SIMD into `target/web-host-spike-scalar` (`?pkg=web-host-spike-scalar`), with `--gpu` vello on WebGPU (SIMD too) into `target/web-host-spike-gpu` — the SIMD builds in their own target directory, `target/wasm-simd`, since the flags rebuild every crate — and then the desktop captures. wasm-pack's wasm-opt (117) takes the SIMD module without extra flags (`wasm-opt --metrics` counts its SIMD instructions). `tests/web_host_spike.mjs` checks the default (SIMD) build first and the scalar one where the SIMD build used to be the optional extra.
- **Dependencies**: the lock only moves edges (no versions change) and the shipped crates are the same: `cargo deny check licenses` ok, `tools/third_party_notices.py --check` up to date.

Checks, on development's c01a98b (after FLTK's removal) plus these commits:

- `cargo check --workspace`; `cargo test -p rapidr-ui-render -p rapidr-ui-host-winit -p rapidr-ui-kernel`: 1 + 13 + 86 passed (the desktop host's 14 of before, one of which moved); `cargo test -p rapidr-ui-render` alone (the CPU path only) and `-p rapidr-ui-host-web`; `cargo check --target wasm32-unknown-unknown` of the kernel, of `rapidr-ui-render` with and without `gpu` and with wasm SIMD, and of the spike with and without `gpu`; clippy on `rapidr-ui-render` (all targets, no features and all features, and for wasm32 with SIMD): no warnings in the crate.
- **Byte comparisons before (c01a98b) and after.** Kernel captures (`RAPIDR_CAPTURE`, headless host, interpreted builds of each version) of 22 fixtures — canvas_onpaint, form_draw, picture_resource, panel_bevels, message_icons, tab_control, trackbar, grid_draw_cell, font_size, svg_picture, code_editor, mdi_children, menus, icons, border_icons, listview_columns, string_grid, statusbar_panels, tree_view, header, a11y_form, owner_list — at `RAPIDR_SCALE` 1 and 2: 48 windows, **all byte-identical** (the baseline was captured twice first: deterministic). The spike's desktop captures (`desktop_capture`: four forms at 1× and 2×, with the system's and with the built-in fonts): **16 of 16 byte-identical**.
- The desktop GUI matrix, `node tests/native_gui_events.mjs` (the one kernel host after FLTK's removal: every case native and interpreted, headless, accessibility trees written), with the new `./rapidr`: **549 of 549 checks** at scale 1 and **549 of 549** with `RAPIDR_SCALE=2`.
- `tests/web_host_spike.mjs` against the worktree served on 127.0.0.1: **57 of 57** — byte-identical to the desktop's captures at 1× and 2× with SIMD and without, the big window with both builds, Chrome's accessibility tree equal to the kernel's, input, and WebGPU within the tolerance (637 / 2,516 / 1,331 pixels differing, exactly the spike's figures: the GPU path draws as before too).

Times of the default (SIMD) build, Playwright's Chromium on this Mac: a form's whole frame 0.18–0.48 ms (scalar 0.32–1.19), the big window 4.9 ms at 1× and 7.6 ms at 2× (scalar 11.3 / 19.7). Sizes: 6.64 MB raw / 1.68 MB brotli with SIMD, 6.84 / 1.71 without, 7.69 / 1.83 with WebGPU (SIMD).

Open: nothing for W1. For later stages: the hosts still make their GPU surfaces and devices with vello's `util` themselves (the desktop's windows, `gpu_web.rs`'s canvas) — a shared helper in `rapidr-ui-render::gpu` if W9 adopts WebGPU; `RendererKind` / `RAPIDR_RENDERER` stays the desktop host's choice; the spike stays out of every regular build and of THIRD_PARTY_NOTICES.md until W3; the web runtime's own wasm (`tools/build_web_artifacts.sh`) is still built without SIMD — making it SIMD-only is W3's build decision (§3.4).

---

## W2 results (2026-10-04)

**`crates/rapidr-ui-app`** holds the host-neutral half of runtime-core's UI glue: what turns the kernel's events into the program's, what a form's window goes through, the timers, the bookkeeping of the waits a VM serves, menus, the lists' events, the dialogs' requests and answers, and the GUI test hooks with the script that plays them. It depends on rapidr-value and the kernel only (no winit, no DOM, no runtime) and builds for `wasm32-unknown-unknown`; tools/regress.sh's unit stage now checks it there with the kernel. runtime-core uses it with feature `gui`; generated native programs get it through runtime-core. Nothing changed on the desktop (the byte comparisons below).

### What moved

| From runtime-core's `ui/` | To `rapidr-ui-app` |
|---|---|
| `testhooks.rs` (git mv) | `testhooks.rs` — unchanged |
| `file_dialog.rs`, `choose_dialogs.rs` (git mv) | the same names — generic over `Program` |
| `kernel_lists.rs` (git mv) | `lists.rs` — the lists lane's `ListAction`s (OnChanging / OnExpanding / OnEditing / OnEdited / OnSelectCell / OnListDropDown and their answers), the owner-draw events before a paint, OnDeletion |
| `kernel/menus.rs` (git mv), all but the menu shown | `menus.rs` — a pick's OnClick, Popup's and AutoPopup's OnPopup and where the menu goes, `RAPIDR_DUMP_MENUS` |
| `kernel.rs`: `dispatch`, `mouse_event`, `container_event` | `dispatch.rs` — every `KernelEvent` (Click, DblClick, Change, KeyDown / KeyPress / KeyUp with KeyPreview and INKEY$, Mouse, Close, Resized, Moved, ScaleChanged, MenuPick, Set, List, Container) |
| `kernel.rs`: the forms' state (built, shown, the first OnPaint, the pending shows, the modal list, the scales, the normal bounds), `build_form`, `show_window`, `after_show`, `fire_first_paint`, `show_pending`, `gui_show` / `_visible`, `gui_hide`, `gui_close` (OnClose's Action), `gui_set_visible`, `gui_apply_geometry`, `gui_center`, `gui_move_form` (and its APPLYING guard), `gui_set_form_border`, `gui_apply_icon(s)`, `gui_set_caption`, `form_resized`, a user's move, `scale_changed`, `gui_set_window_state` and `simulate_state`, the toggle buttons, ShowModal's setup, `modal_ended` | `forms.rs` |
| `kernel.rs`: the timer heap | `timers.rs` — register, start, schedule, fire what's due (Interval and Enabled read again each tick, armed again after the handler), the next deadline |
| `kernel.rs`: `Wait`, the cooperative flag, `wait_started`, whether the innermost wait is over | `waits.rs` |
| `kernel.rs`: `WinOp`'s queue, `NOTIFY` (invalidate / restructure) | `windows.rs` — `WindowOp`, `take_ops`, `take_notify` |
| `kernel.rs`: `Script`, `script_step`, `run_test_event`, `shown_up`, the dump | `script.rs` |

**What stays in runtime-core** (`ui/kernel.rs`: 1,174 lines, from 1,790; `ui/kernel/*`): everything that pumps or waits for the desktop host — the host itself, `sync_desk` (the window commands into `Desktop` and `HostCmd`s), `pump`, `step`, `dispatch_pending`, the tracking tick and `held`, the native waits (ShowModal's loop, DOEVENTS's step, INPUT$, the main loop, `gui_pump_wait`'s turns), the kernel-drawn dialogs and rfd's sheets (`kernel/dialogs.rs`), the menu shown (`kernel/menus.rs`: `open_at`), the platform (frames, the cursor, `$THEME`), the screen / work area / monitors / mouse, the test script's input into the kernel and the captures at its end; `RtStore` (`kernel_store.rs`); and the facade's names, now one-line calls into the app where their logic moved. `ui/program.rs` is runtime-core's `Rt`.

### The traits

Two, both implemented by a runtime's unit struct passed by value (`Copy + 'static`, as `rapidr_value::mdi::Runtime`: a continuation the glue hands over keeps it). runtime-core's is `ui::program::Rt`.

- **`Program`** — the program: its components (`get`, `set` with what follows a set, `store` without, `type_of`, `children`, `form_of`, `flag` as the kernel reads it), its events (`fire`, `fire_args`, `fire_then` — the continuation runs once the handler has, with its by-reference arguments as the handler left them — `has_handler`, `in_host_callback`), its layout (`quietly`, `constraints`, `client_changed` = the aligned and anchored children and the scroll bars, `container` = a scroll / splitter / MDI action) and the clock (`now`: the kernel's `Instant`, `performance.now()` in a browser). Over `object.rs`, `layout.rs`, `scroll.rs` and `mdi.rs` on the desktop.
- **`Windows`** — what the glue asks of the host: `start` / `started`; `flush` (the queued window commands carried out now: the desktop pumps once without waiting, so a window exists before its OnShow); `dispatch_pending`; what only the host knows (`headless`, `forced_scale`, `window_scale`, `screen`, `work_area`, `menu_in_window`, `stacking`, `place_of`); `system_resized` (the headless host's maximize); `open_popup`; a test's `script_input` and `capture_and_end`.

Made for the web's constraints (§3.2): **no method of either waits for the user, and nothing in the app blocks** — every wait that does is the runtime's, around the app's functions. A handler is only ever fired (`fire*`): queued for a VM, run at once natively; what must follow it is a `fire_then` continuation (`rapidr_value::events::fire_then` queues it behind the handler in the interpreter). What the host must do waits in queues its next turn takes (`take_ops`, `take_notify`), so the app may run while the host is busy (inside a DOM handler, inside a pump). The waits a VM serves are data (`waits::Wait`), not loops.

### How W3 and W4 plug in

- **W3 — the web's `Program` and `Windows`.** rapidr-runtime-web gets a unit struct implementing both: `Program` over `object_web.rs` (`rp_comp_get` / `rp_comp_set` / `rp_comp_set_prop_only`, `rp_comp_type`, `get_children_of`, `form_of`, `rp_fire_event*`, `rp_has_handler`; `flag` with `WebStore`'s rules) and `layout_web` / `scroll_web` / `mdi_web`, `in_host_callback` true while a DOM handler routes input; `Windows` over the web host: `flush` applies the queued `WindowOp`s to the page's frames at once (nothing to pump), `dispatch_pending` dispatches the `KernelEvent`s the DOM handlers queued (`dispatch::dispatch`), `headless` false (the page maximizes for real), no `forced_scale`, `window_scale` the devicePixelRatio, `screen` / `work_area` the viewport, `menu_in_window` true, `stacking` the page's z-order, `place_of` from the form's kernel tree, `open_popup` the kernel's pop-up (which never blocks there: the pick comes back as an event), `script_input` / `capture_and_end` the web test harness's (the capture: the wasm's own pixels).
- **The page's turn** (§3.2's loop) is then: a DOM event → kernel input → `KernelEvent`s queued → `dispatch_pending` (handlers queued for the VM) → the VM's slice → `schedule()`: `forms::show_pending`, `lists::pre_paint` over `forms::shown_forms()`, `take_ops` / `take_notify` into the host, the dirty forms painted, then one `setTimeout` for the earliest of `timers::next_due()`, the kernel's `next_wake` and `script::next_step()`, whose callback runs `timers::fire_due` and `script::step`. The per-QTIMER `setTimeout`s (`TIMER_HANDLES`) go: one heap, the desktop's firing rules.
- **W4 — the waits.** The web VM's waits are always cooperative (`waits::set_cooperative(true)`). ShowModal is `forms::begin_modal` and `waits::start(Wait::Form)`: the VM suspends; each page turn asks `waits::over()` (its form closed; no window left for the main loop), and when it is, `waits::pop()` and `forms::modal_ended` give the ModalResult the VM resumes with. DOEVENTS is `Wait::Once(since)` (`waits::turn`, then `waits::pop` once `timers::due_since(since)` is false): the slice ends. A native web build (generated Rust in wasm) calls `begin_modal` and returns at once, as today.
- **The dialogs** (MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE / MSGBOX, colour, font, Open / Save) are waits the VM serves, in the app since 2026-10-05 (docs/desktop-host-plan.md, "Dialogs the interpreter waits for"): `dialogs::message` / `execute` show the dialog — the kernel's `Dialog` given a window through `Windows::open_dialog`, or the host's own Open / Save through `Windows::ask_files` / `files_answer` — and return `Pending`; the web's VM, always cooperative, starts `Wait::Dialog(id)`, and each page turn asks `waits::answered(rt)`, whose `Some(result)` (the builtin's: the button's IDYES / mrNo, Execute's 1 / 0 / -1 with the component's properties set) the VM resumes with. The desktop's interpreter serves the same wait (and so runs its timers' handlers while a box is open). For W4: implement `Windows`' dialog methods over the page (`open_dialog` a kernel form on the page — `dialog_web.rs`' page dialogs go; `ask_files` the page's file input, W8), and INPUT$ is `Wait::Key`. Timers: `timers::fire_due` fires one handler queued for the VM per round (`hold_back`): fire again once the VM has run it.
- **`Desktop`** (`rapidr_ui_host_winit::desktop`: the forms' kernel sides, stacking, modal routing, the `HostCmd` / `HostEvent` queues, the input entry points) is GUI-free too, and the web host needs the same. W3 should move it into `rapidr-ui-app` (or the kernel) rather than write a second one; `Windows`' `stacking`, `place_of`, `system_resized` and `script_input` then become the app's own over it, and the script's input (keys, the mouse, a double click, a resize, a component's step) moves with it. Not done here: it is the host's code, not runtime-core's, and W2 is the move of runtime-core's glue.

### The component registry: not unified (the path)

One table for both runtimes isn't clean or safe in this stage: the two registries don't agree, and making them one changes what programs read on one runtime or the other.

- **Different defaults per type.** RFORM: the desktop's Color &HFFFFFF and BorderStyle 2, the web neither. RLABEL: the desktop's Visible, Alignment, Color, FontColor, FontSize 12; the web only Caption / Left / Top. RCANVAS (pen, brush, font), REDIT (Enabled, ReadOnly, MaxLength), RCHECKBOX / RRADIOBUTTON (Checked), RIMAGE (Stretch), RFILESTREAM (the web's Text, EOF, MimeType) … differ; RCOOLBTN, ROVALBTN, RTRACKBAR, RWEB*, RPLOT and RDATAFRAME are only the web's, RMYSQL, RSOCKET, RHTTP, RMENUITEM, RSTATUSBAR's SimpleText … only the desktop's; an unknown type is empty on the desktop and 100 × 25 at (0, 0) on the web.
- **Different keys and reads**: lowercase names on the desktop, uppercase on the web; each runtime's `rp_comp_get` answers differently for a property never set; RtStore reads a label's / form's / panel's creation-default white Color as unset.
- **Different creation**: the web makes its DOM widget, its data binding and its hooks in the same call.

**The path.** (1) W3, with `WebStore`: a shared table in rapidr-value (both runtimes depend on it) holding the types and defaults both already agree on, used by both constructors (no behaviour change), and RtStore's "unset" rules in one place for RtStore and WebStore. (2) Each remaining difference becomes a conformance case reading the default on the three runtimes; RapidQ's value is decided (its manual, the RapidQ install, `.reference/`) and both runtimes take it — RapidQ's behaviour, exactly, everywhere. (3) At W11 the DOM half of `object_web.rs` goes and the shared table is the only one, its keys lowercase.

### Tests (this worktree: development's 097d9d7 plus these commits)

- **Byte comparisons before (097d9d7) and after.** All 66 cases of `tests/gui_parity_cases.mjs`, built interpreted, run headless with each case's events, dump, resize / splitter drag and dialog answers, at `RAPIDR_SCALE` 1 and 2: 144 windows captured (`RAPIDR_CAPTURE`'s BMPs), 132 accessibility trees (`RAPIDR_TEST_A11Y`), 132 dumps (228 lines). The baseline was captured twice first (408 of 408 files identical: deterministic); after the change **all 408 files are byte-identical** to it.
- The desktop GUI suite with the new `./rapidr` (`cargo build --release -p rapidr-cli`): `node tests/native_gui_events.mjs` — every case native and interpreted, headless, accessibility trees written — **558 of 558 checks** at scale 1 and **558 of 558** with `RAPIDR_SCALE=2` (66 cases). `node tests/conformance/run.mjs`: 228 passed. `cargo test --workspace`: 433 passed, 1 ignored. `tools/third_party_notices.py --check`: up to date (the crate adds no dependency; `Cargo.lock` only gains it); `cargo deny check licenses`: ok. No file of the web runtime, the interpreter, the kernel or the hosts changed.
- `cargo check -p rapidr-ui-kernel -p rapidr-ui-app --target wasm32-unknown-unknown`: ok. `cargo clippy -p rapidr-ui-app --all-targets`, and for wasm32: no warnings in the crate; `cargo clippy -p rapidr-runtime-core`: none in `ui/`.
- `rapidr-ui-app`'s 14 unit tests: the test hooks' 6 (moved) and 8 new ones driving the glue through an in-memory `Program` + `Windows` (a store, the events fired, the window commands, a clock the test moves — what a runtime implements, at its smallest): a form shown (OnLoad, OnShow, the first OnPaint, the window there before OnShow), OnClose's Action (stay, minimize, hide), KeyPreview's order, a user's resize within the Constraints and a move, Set and Container, the timers (armed once, Interval read again, disabled and enabled again), a modal form as the VM's wait, the headless maximize and restore.

Open: ~~the dialogs as waits the VM serves (W4, above)~~ done 2026-10-05 (above); ~~`Desktop` into the app (W3, above); the registry (above)~~ done in W3 (step 1 of the registry; below). Nothing else from W2.

---

## W3 results (2026-10-05)

The UI kernel hosts a web page's forms with `?host=kernel` (the page's address, or `RAPIDR_HOST = "kernel"` set before the runtime starts; the web IDE passes its own `?host=kernel` on to its preview). The DOM host stays the default. On the kernel host, **64 of the 68 browser GUI cases** give the desktop's dumps, played by the desktop's own test hooks. Their windows are **byte-identical to the desktop headless host's captures** at 1× and 2×, apart from the in-window menu bar macOS doesn't have. Their accessibility trees are the kernel's to the byte, and Chrome's tree over the mirror equals the kernel's.

### Step 1: `Desktop` into the app, and the registry's shared table (ca09ead)

- **`rapidr_ui_app::desktop`** now holds `Desktop`: the forms' kernel sides, stacking, the modal list, the `HostCmd` / `HostEvent` queues, and the input entry points the user and test scripts share. It moved with `git mv` from `rapidr-ui-host-winit`, together with `Frame` / `frame_of` / `BI_DEFAULT` and `FileRequest`. winit's buttons stay the host's (`platform::buttons`), and the host re-exports everything under the old names. `Icon` is now the app's.
- Runtime-core glue the web host needs moved into the app too:
  - the program's window commands into the forms: `Desktop::apply`, `sync_forms`, `desktop::window_spec`, `frame`;
  - the test script's input: keys, the mouse, a double click, a component's step, the resize, `place_of` (`desktop::script_input`);
  - the accessibility JSON: `Desktop::access_json`.
- **The component registry, step 1.** `rapidr_value::component_defaults::shared` holds what both registries already gave a new component, by type. `desktop` holds what only the desktop's gives (QFORM's Color and BorderStyle, QLABEL's Visible / Alignment / FontSize …). `kernel_reads_unset` is RtStore's creation-white Color rule, now used by both kernel stores. runtime-core's `RpComponent::new` = `shared` + `desktop`; the web's `rp_create_component` = `shared` + its own arms. Under `?host=kernel` the web adds `desktop` too, so the kernel draws, and the program reads, what it does on the desktop: a label's 12-point default font is the difference between equal and unequal captures. Step 2 (deciding each remaining difference by RapidQ's value) is unchanged.
- **Byte-identical desktop.** `tests/gui_captures.mjs` (new) captures every GUI case's windows, accessibility trees and dumps at 1× and 2×, interpreted, headless. It was run before and after each step: 74 cases, 148 runs, 610 files, all identical except `dialog_timers`' captures. Those are timing-dependent: two runs of development itself differ, and their dumps are equal.

### The web host (`rapidr-ui-host-web`)

- **`host.rs`, `WebHost`.** Each form is a window on the page: a `<div>` with a canvas for the frame, a canvas for the client area, the mirror over it, and invisible drag edges.
  - **The client area** is the kernel's display list rasterized by the shared CPU renderer at `devicePixelRatio` and put with `putImageData`: the very code the desktop's captures use.
  - **The frame** is drawn by the kernel's `Painter` in the current theme (`frame.rs`): 1-pixel border and a 29-pixel title bar, as `layout::form_frame` accounts every runtime's forms, with the title bar buttons BorderStyle / BorderIcons leave, active or inactive. A theme change redraws it.
  - **Window behaviour.** Windows stack (z-order from `Desktop`; a click raises one, or the modal window that keeps it from input). They move by the title bar (Left / Top follow: `form_moved`) and size by the right / bottom edges and the corner (OnResize: `form_resized`). They maximize (the viewport), minimize (to the title bar) and restore, through WindowState or the buttons; a double click on the title bar also maximizes or restores. The close box fires OnClose.
  - **Scale and context loss.** A `devicePixelRatio` change (another monitor, the browser's zoom) redraws everything and fires OnScaleChanged. A canvas whose backing the browser dropped is redrawn on `contextrestored`.
  - **The host loop.** The host never runs program code. A listener routes input through `Desktop` (`Source::User`) and wakes the runtime, which dispatches with the host not borrowed. Events the browser fires while the host is busy (a focus the mirror moved) are dropped.
- **Input.**
  - Pointer events, the wheel (notches as the desktop's touchpad rule), and keys on the mirror. The browser keeps its own keys, and F6 / Ctrl+Tab leave a program embedded in a page (no keyboard trap).
  - Clipboard events: Cmd/Ctrl + C / X / V through a page clipboard; a context menu's Copy / Cut goes to `navigator.clipboard`.
  - Input methods: `compositionupdate` / `compositionend` become `ime_preedit` / `ime_commit`. While composing, the field moves to `FormUi::ime_area`, so the candidate window opens at the kernel's caret even in a scrolled edit.
  - `beforeinput` without a key: a phone's keyboard, dictation, autocorrect's replacement, a drop.
  - **Autofill**: an `input` event the mirror didn't cause is the user's edit (select all, then the filled text).
- **`mirror.rs`, the mirror in Rust.** `aria::specs` describes the elements, and the mirror patches them after each frame by stable node id. The DOM focus is kept on the kernel's focused node in the active window. A text field's value and selection are the kernel's (UTF-16). A screen reader moving the focus becomes `Action::Focus`, and its click becomes `Action::Click`. A combo box is a text field (its value is what Chrome reads), with its list beside it.

### The web runtime (`rapidr-runtime-web` feature `kernel`, `kernel_web.rs`)

- **`WebStore`** is the kernel's `Store` over the registry, with children's ids lowercase as the desktop keeps them, so node ids (and so the accessibility trees) are the same. **`Web`** implements `Program` (over `object_web`, `layout_web`, `scroll_web`, `mdi_web` and `directx_web`) and `Windows` (over the host), as "How W3 and W4 plug in" planned. `rapidr-vm-host-web` turns the feature on (`default = ["kernel"]`), so the IDE's and `rapidr bundle-bc`'s wasm carry both hosts.
- **The page's turn:** a DOM event leads to the kernel's input, then `turn` (dispatch, the test script's step), then one `requestAnimationFrame`. The frame does `show_pending`, the owner-drawn lists' events, the program's window commands, the kernel's deadlines and the dirty windows drawn; then one timer for the earliest deadline (a caret, a held scroll bar, the script's next step). The program's changes (`object_web` / `gui_web`, whose GUI entry points call into `kernel_web` when the kernel hosts) ask for the frame. A form's Visible is whether its window shows, QTREEVIEW's OnDeletion and GetItemAt are the app's, and `Application.Theme` / `$THEME` set the kernel's theme (`auto`: the page's `prefers-color-scheme` / `forced-colors`).
- **What is still the web's own (W4):**
  - ShowModal: `forms::begin_modal`, then the VM suspends in `dialog_web` until the form closes.
  - Message boxes and the colour / font / file dialogs are the page's.
  - QTIMERs are `setInterval`s.
- **Test hooks in the browser.** `testhooks::set_vars` gives the hooks an environment where a process has none (`rapidr_set_test_env`). `Windows::capture_and_end` now returns, and the desktop's still exits. `rapidr_test_results` gives the dump lines, the trees (the bytes `RAPIDR_TEST_A11Y` writes) and each window's BMP (the bytes `RAPIDR_CAPTURE` writes). The script's next event waits while the VM is between two time slices. New page `tests/web_kernel.html`.
- **Real input, checked by hand** (Playwright's real mouse and keyboard, CDP's input method, no hooks): typing in an edit, Tab by TabOrder (the DOM focus followed), a check box's click and OnClick, a window dragged by its title bar (Left / Top followed), a composition committed into an edit.

### Tests

- `RAPIDR_WEB_HOST=kernel node tests/web_gui_parity.mjs` (with `RAPIDR_DESKTOP_CAPTURES=<tests/gui_captures.mjs' dir>`): **68 cases run, 64 with every expected line**. The 4 pending are marked `webKernel: "pending: …"` in the case table:
  - `color_dialog`, `font_dialog` and `input_chars` are W4 (waits the VM serves);
  - `file_dialogs` is W8.
- **Windows byte-identical to the desktop's: 68 of 70 at 1×, 67 of 70 at 2×.**
  - `menus` and `themes` have an in-window menu bar, which macOS' desktop doesn't draw. Against desktop captures made with `RAPIDR_MENU=window` (as on Windows and Linux) both are byte-identical, and so are their trees.
  - `modal_result` at 2× differs in one pixel by one level (216 against 217, the anti-aliased edge of a glyph): wasm SIMD's rounding against NEON's, open.
- **Accessibility trees equal to the desktop's (the bytes): 60 of 64.** `menus` and `themes` are the menu bar again. `message_icons` is the page's message box (W4). `event_answers` is a grid that scrolled one row on the web after a refused OnSelectCell: open.
- `RAPIDR_WEB_HOST=kernel node tests/web_a11y.mjs` (Chrome's tree over the mirror against the kernel's, every node matched by its `data-node`): **68 of 68** (65 at first: three combo boxes whose value Chrome read from their options' text, fixed by making a combo box's element a text field).
- **The DOM host (the default) is unchanged**, run on the wasm SIMD build that carries both hosts:
  - web conformance: 111 passed, 2 known failures;
  - web GUI parity: 126 / 126 at 1× and 2×;
  - `tests/web_a11y.mjs`: 79 / 79;
  - every `web_ide_*`, `web_bundle_*`, `web_end_timer`, `web_vm_yield`.

  Two test fixes: `web_a11y.mjs`' own desktop runs now get a case's joystick script, and `web_ide_picture.mjs` expects HEX$'s 8 digits (development's RC.EXE change).
- **The desktop on the final code:** `node tests/native_gui_events.mjs` at 1× and `RAPIDR_SCALE=2`, and the interpreted byte comparison against development (above).

### Sizes

| `target/web/rapidrintr_bg.wasm` | Raw | gzip -9 | brotli 11 |
|---|---|---|---|
| Before (development, DOM host only, scalar) | 6.27 MB | 2.66 | 1.95 |
| With the kernel host (scalar) | 10.40 | 4.40 | 2.73 |
| With the kernel host, wasm SIMD (the build now) | 10.18 | 4.34 | 2.70 |

So the kernel host costs +3.9 MB raw / +0.75 MB brotli, as §7 estimated (the fonts' outlines and shaping, vello_cpu, parley, ICU4X's dictionaries, the kernel). `tools/build_web_artifacts.sh` now builds with wasm SIMD (§3.4) into its own target directory (`target/wasm-simd`).

### Licences

No new external dependency: every crate the host and the `kernel` feature link was already shipped and listed. `cargo deny check licenses` is ok, and `tools/third_party_notices.py --check` is up to date. No JavaScript or CSS was vendored.

### Open (W4 onward)

- **W4:**
  - the VM's waits through `rapidr_ui_app::waits` (ShowModal, DOEVENTS, INPUT$'s `Wait::Key`);
  - the kernel's message boxes and colour / font dialogs on the page: `Windows::open_dialog` is implemented, and the builtins still use `dialog_web`;
  - timers through the app's heap;
  - native web builds (`rapidr build --web`'s generated Rust) on the kernel host: they build the runtime without the feature.
- **W5:**
  - pixels at 1.5×;
  - `event_answers`' grid row, and `modal_result`'s one-level pixel at 2× (SIMD rounding);
  - real screen readers and devices (§4.1);
  - Firefox and Safari.
- **The host:**
  - resizing from the left and top edges;
  - a minimized window stays where it was (no task bar);
  - autofill's `autocomplete` / `name` hints from properties (the fields say `autocomplete=off` today);
  - touch gestures (Phase 7);
  - the IDE preview with `?host=kernel` is wired but not covered by the IDE suites.
- **W6:** web-only components (RWEBVIEW, RDOM, media, RPLOT) aren't drawn on the kernel host yet.
- **W7:** fonts as assets, and the fallback fonts (CJK shows as boxes).
- **W9:** damage rectangles, and the size levers.

---

## W4 results (2026-10-05)

The VM, the kernel's dialogs and the timers on the kernel host, and native web builds on it. Every browser GUI case now runs there: no case is pending.

### The VM's waits (`kernel_web.rs`, `dialog_web.rs`)

A browser's VM can't block, so a wait is the VM suspended and resumed. ShowModal, DOEVENTS, INPUT$ / WAITKEY and the kernel-drawn dialogs start a `rapidr_ui_app::waits` wait (the desktop's own bookkeeping) and suspend the VM (`dialog_web::suspend_for_wait`). After each turn the kernel host serves the innermost wait the way the desktop's interpreter does:

- an answered dialog resumes the VM with its answer;
- a modal form that closed resumes it with its ModalResult, and the timers stop as `rp_stop_all_timers` stops them on the desktop;
- DOEVENTS (`Wait::Once`) fires what's due, then resumes once per turn, so a script's next step gets its turn too.

Nested waits resume in stack order. The interpreter's session says it serves waits (`set_interpreter`); a native web build, whose generated Rust can't be suspended, keeps the cooperative path (handlers queued, the modal list), as before.

### The kernel's dialogs and the timers

- MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE and MSGBOX are the kernel's message box (`Windows::open_dialog`), with its icons and its beep; the colour and font dialogs (QCOLORDIALOG / QFONTDIALOG's Execute) are the kernel's dialogs. All are pixel-identical to the desktop's.
- **Open / Save** (W8 pulled forward): QOPENDIALOG / QSAVEDIALOG's Execute go through `Windows::ask_files`: the page's own file picker over the program's files (`object_web::page_file_dialog`), the answer coming back as the kernel's `files_answer`. `RAPIDR_TEST_FILE_DIALOG` answers it as on the desktop.
- **Timers** (QTIMER, QDXTIMER, QDXJOYSTICK's polling) run on `rapidr_ui_app`'s timer heap, as on the desktop: they start at ShowModal / a dialog / DOEVENTS / INPUT$, re-arm after their handler (`fire_then`), are held back while a handler waits, and wake the page through one deadline (`arm_deadline`). A joystick is only looked at once the program has a handler for it.

### The host's gaps

- Windows size from all eight edges and corners; the left and top edges move the window (`EDGES`, a minimum width of 160).
- A minimized window is a title bar along the bottom of the page in its own slot; restoring or maximizing frees the slot, and a double click on its title restores it.
- Autofill hints: an edit's, memo's, rich edit's or combo box's `AutoComplete` property becomes its mirror field's `autocomplete` (default `off`), and the field's `name` is the component's.
- `event_answers`' grid row: a grid scrolls its newly selected cell into view before it is described as well as before it's drawn (`show_selection`). The desktop's accessibility tree said the old TopRow until the next frame; now it says what is drawn, on both hosts. That is the only desktop change (that case's `a11y.json`).

### Native web builds

`rapidr build --web` builds the runtime with feature `kernel` (codegen's Cargo template), copies the workspace's Cargo.lock for web projects too, and builds with wasm SIMD unless RUSTFLAGS says otherwise. `tests/web_end_timer.mjs` builds and runs one each way.

### Tests

- `RAPIDR_WEB_HOST=kernel node tests/web_gui_parity.mjs` with `RAPIDR_DESKTOP_CAPTURES`: **70 of 70 cases with every expected line** at 1× and 2×.
  - Windows byte-identical to the desktop's: **72 of 75 at 1×, 71 of 75 at 2×**. The differences are `menus` / `themes` (the desktop's macOS menu bar, as in W3), `message_icons`' second box (two pixels: wasm SIMD's rounding against NEON's fused multiply-add) and `modal_result` at 2× (one pixel, the same).
  - Accessibility trees: 65 of 68 before the grid fix; `event_answers`' is now equal (checked separately), leaving `menus` / `themes`.
- `RAPIDR_WEB_HOST=kernel node tests/web_a11y.mjs`: **70 of 70**.
- The DOM host, unchanged: web conformance 128 passed (2 known failures); GUI parity 128 / 128 at 1× and 2×; `web_a11y.mjs` 81 / 81 (download and media needed `./rapidr` rebuilt with the media objects); every `web_ide_*`, `web_bundle_*`, `web_end_timer`, `web_vm_yield`.
- The desktop: `tests/gui_captures.mjs` against W3's captures: byte-identical apart from `event_answers`' tree (above) and `dialog_timers` (its tick counts are timing-dependent).
- `tools/regress.sh` now makes the desktop's captures itself and runs the kernel host's parity against them at 1× and 2×, printing each `≠`.
- A page error is the page's uncaught exception, as in the DOM host's runner: the 404s in `download` and `media` are the fixtures asking for missing files on purpose (`no_such_file.txt`, `no_such_song.mid`).

### Sizes

`target/web/rapidrintr_bg.wasm`: 10.41 MB raw, 4.42 MB gzip -9, 2.75 MB brotli 11 (W3: 10.18 / 4.34 / 2.70).

### Licences

No new dependency.

### Next: the kernel host becomes the only web host

The user's direction (2026-10-05): no opt-in. The kernel host becomes the default for the IDE preview, `bundle-bc` bundles and `rapidr build --web`. Once every web suite passes there, the DOM host is deleted (§5), together with the `RAPIDR_WEB_HOST` switches. Before that:

- the web-only components (RWEBVIEW, RDOM, media, RPLOT) as DOM overlays over the canvas (W6);
- the fallback fonts, so CJK and symbols don't regress (W7);
- every suite ported to the kernel host: `web_ide_*`, `web_bundle_*` and `web_end_timer` look for the DOM host's elements.

The HTML / Monaco web IDE (`web-ide/`) stays for now, as the test harness and the only web IDE, running its preview on the kernel host. It retires when the kernel-drawn MDI IDE (ROADMAP's IDE phase, §3.9) runs in the page.

---

## The kernel host by default, and W6 (2026-10-05)

### The default

`kernel_web::on()` is true unless a page asks for the old host (`?host=dom`, or `RAPIDR_HOST = "dom"` on the page). That covers the IDE's preview (`index.html?host=dom` passes it on), `bundle-bc` bundles and `rapidr build --web` output. The switch exists only until the DOM host is deleted. In the tests, `RAPIDR_WEB_HOST` defaults to `kernel`, and `dom` runs the old host.

- **Mirror ids.** A component's mirror element has the id `rr-<name>` (lowercase) and `data-rr-name`, the ids the DOM runtime gave its elements. Scripts and tests find a component by them; parts keep `rrn-<form>-<node>`.
- **DOM listeners.** On the kernel host, `bind_dom_event` binds DOM listeners only to the web-only components' own elements. The kernel fires every other component's events.

### W6: the web-only components (`overlay_web.rs`)

**The elements.** RWEBVIEW (a sandboxed `<iframe>`), RDOM (the program's element), RWEBAUDIO / RWEBVIDEO and RPLOT (its chart's canvas) are real elements with the id `rr-<name>`. Each is made once the component is registered; `overlay_web::create` applies what the program set.

**Placement.** The kernel places them as nodes of their form, drawn by nothing. The host (`host::set_overlay_types`, `place_overlays`) puts each element in its window's `.rr-koverlays` layer over the client canvas, at the node's place. It's clipped to its parents' rectangles (`clip-path`) and hidden with them. The element takes its own pointer events.

**The page's own RDOMs.** An RDOM whose parent isn't a kernel component (none, or another RDOM by ParentId / AppendTo) stays where the program puts it in the page. `<style>` / `<script>` RDOMs go into the head.

**Properties, methods and defaults.**
- `overlay_web` handles their properties (Url, Html, Src, InnerHTML, CssStyle, TagName …), live reads (CurrentTime, Duration …) and methods (Navigate, SetHtml, AppendTo, Play …).
- They get default sizes (`layout::default_size`: 100 × 25, the DOM runtime's) and read Visible = True until hidden.

**The popup layer.** While a form has such elements, the kernel draws its open drop-down list and menus apart (`FormUi::popups_apart`, `paint_popups`). The host renders them on a transparent canvas, `.rr-kpopups`, above the elements. That canvas takes the client area's pointer while a list or menu is open, so a drop-down opens over an iframe and still picks. The desktop never sets `popups_apart` and draws exactly as before.

**Test.** `tests/web_overlays.mjs` checks placement, clipping, clicks, moving and hiding, the plot's pixels, and a drop-down over the web view; it passes at 1× and 2×.

### The suites on the kernel host

Every `web_ide_*`, `web_bundle_*`, `web_end_timer` and `web_vm_yield` test now runs on the kernel host (31 of 31). They use `tests/web_kernel_page.mjs`:
- components found by their mirror elements;
- real clicks at their place (`click({ force: true })` lands on the canvas);
- `rapidr_get_prop` for the program's state;
- the window's canvas for pixels.

Runtime fixes the ports found:
- INPUT with windows shown opens the kernel's input box (`Dialog::input`, `dialogs::input`; the typed line echoed as before).
- END in a native web build hides the windows directly (`kernel_web::ended`). Its unwinding leaves the host borrowed, so nothing after it would draw.
- The bundle console docks while a kernel window shows and watches for windows appearing.
- Typing in a combo box sets ItemIndex as Windows does (CBUpdateLBox): -1 while the list is closed, the first item the text begins while it's dropped down.
- A grid's in-place editor and its gcsList drop-down are in the accessibility tree. The editor takes the focus, so a screen reader and an input method reach it.
- The desktop shares the kernel changes (combo typing, the grid's tree): `native_gui_events` passes at 1× and 2×, and `gui_captures` is byte-identical apart from `dialog_timers` (timing) and `file_browser`'s directory list (the checkout's `target` is now a link).

Results: kernel GUI parity has 75 of 75 cases with every expected line; windows are byte-identical in 79 of 82 at 1× and 78 of 82 at 2× (the known menus / themes / SIMD pixels), and trees in 73 of 75. Kernel a11y passes 75 of 75, web conformance 133 (3 known failures), web overlays all checks, and the DOM host (still there) 141 / 141 parity and 86 / 86 a11y.

### Still before the DOM host goes

- W7: the fallback fonts (CJK, symbols; then emoji).
- A web IDE question: the preview shows every form at (100, 100) in a 480 × 320 frame (the kernel's default place), so bigger forms are cut off. That's a layout matter for the IDE's preview, not the host.

---

## W7: the fallback fonts on the web (2026-10-05)

A browser has no system fonts the wasm can draw with. So what the Liberation fonts lack — ✓ and other symbols, Chinese, Japanese, Korean — comes from Noto fonts (SIL OFL 1.1) shipped beside the runtime and loaded as text needs them. The desktop keeps the system's fonts for now; loading the same set from an install's resources is the next step there.

**The set.** It's listed in `fonts/fallback/fonts.toml`:
- Noto Sans, Noto Sans Symbols and Noto Sans Symbols 2: in the repository (`fonts/fallback/`, unhinted OTF, 0.8 MB), so an offline source build still covers symbols;
- Noto Sans SC (Han, kana) and Noto Sans KR (Hangul), from Noto CJK Sans 2.004: fetched by `python3 tools/fonts.py fetch` from the official release assets, pinned by SHA-256 and cached in `target/fonts-src`. Offline without a cache, the build leaves them out and names that command.

None of these fonts declares a Reserved Font Name (LICENSES.md §4, docs/licensing.md §3.1).

**The chunks.** `python3 tools/fonts.py build target/web/fonts` (run by `tools/build_web_artifacts.sh`, fontTools, MIT) writes:
- the three small fonts whole;
- the CJK fonts split by codepoint into chunks of 900 characters (about 200–300 KB each), each renamed `<family> NNN` so it loads as a family of its own (fonts of one family name would be one family to fontique, which draws from the first);
- `index.json`, saying which file has which characters, and the first font in the list keeps a character;
- `OFL.txt`.

Altogether that's 51 files, 10.1 MB.

**The runtime.**
- `rapidr_ui_kernel::text` has:
  - a fallback family list, tried after a QFONT's face and before the system's;
  - a missing-glyph hook: `note_missing` reports a layout's glyph-0 characters, from labels' layouts and editors' alike;
  - `TextSystem::add_font`, whose `generation` makes editors lay out again.

  On the desktop, neither the list nor the hook is set, so its drawing is unchanged.
- The web runtime's `fonts_web.rs` handles a missing character:
  - it reads the index on the first one;
  - it fetches the chunk with that character, once (`RAPIDR_FONTS` names the folder, else `fonts/` beside the page);
  - at the next frame it adds the chunk and draws every window again, frames included (`WebHost::fonts_changed`).
- **The IDE's preview** has an opaque origin, so it can't fetch: it asks the IDE for each file (`RAPIDR_FONT_FETCH`), and the IDE reads `runtime/fonts/`.

**Shipping.**
- `bundle-bc` puts `fonts/` (beside its `--wasm`) into the zip; the IDE's Build does the same from `runtime/fonts/`.
- `rapidr build --web` copies them next to its page, from an install's `lib/rapidr/web/fonts` or a checkout's `target/web/fonts`.
- An installed RapidR never downloads. The release scripts copy `target/web/fonts/` with the runtime (the packaging lane).
- The web notices (`rapidr notices web`) carry the Noto fonts' OFL.

**Test.** `tests/web_fonts.mjs` checks, on the test page and in a `bundle-bc` bundle:
- 中, 국어 (two KR chunks), ✓ and an edit's 汉字 are drawn as glyphs, not the missing glyph's box (pixels against a private-use character's box);
- each file comes once;
- text Liberation has fetches nothing.

It passes at 1× and 2×.

**Privacy.** The web IDE no longer loads Google Fonts: its interface uses the system's fonts, and the font picker's names (Inter, Roboto …) resolve through the fallback like any unknown family. Neither the IDE, the runtime nor a bundle makes a third-party request a program doesn't make itself.

**Emoji (since).** Noto Color Emoji in its COLRv1 form, drawn in colour by the renderer (glifo paints COLR through skrifa: no PNG decoder needed). It's fetched from the noto-emoji repository's file at release tag v2.051 (that project publishes no release asset), pinned by SHA-256, and kept whole (5 MB): emoji sequences are ligatures over several characters. `web_fonts.mjs` checks 😀's yellow.

**Open.**
- Chunks by frequency rather than by codepoint, so a sentence needs fewer files.
- The desktop on the same set from the install's resources.

---

## W11: the DOM host deleted (2026-10-05)

The UI kernel is the web runtime's only GUI host, as §5 planned. There is no `?host=dom`, no `RAPIDR_HOST`, and no `kernel` feature any more: `rapidr-runtime-web` depends on the kernel, `rapidr-ui-app` and `rapidr-ui-host-web` unconditionally.

**Deleted:**
- `gui_web.rs`, the DOM widgets: 6,200 lines;
- `a11y_web.rs`, the JavaScript-era ARIA mirror;
- `menu_web.rs`;
- the DOM dialogs in `dialog_web.rs`: message boxes, input, colour, font, and the DOM ShowModal wait;
- `scroll_web.rs`'s SVG bars;
- the DOM half of `object_web.rs`: widget creation, the DOM event binding for drawn components, the VCL click order, the OnShow bookkeeping;
- the data science DOM placeholders;
- the `rapidr-rrcss` crate. `rapidr_webbundle::PAGE_CSS` is the page's few rules now.

The wasm shrank from 10.63 to 10.21 MB.

**Kept, now host-free:**
- **`dialog_web.rs`:** the VM's suspend / resume / yield protocol, and the page's Open / Save picker behind `Windows::ask_files`.
- **`webapi_web.rs` (new):** RJAVASCRIPT, RWEBSTORAGE, RWEBNOTIFICATION, RWEBGEOLOCATION and RROUTER. Their methods come before the windows' methods, so a notification's `Show` is no longer taken for a form's. The router's `Route` / `Hash` read the address again.
- **`page_web.rs` (new):** the page's document and elements.
- **`layout_web.rs`, `scroll_web.rs`'s model, `mdi_web.rs`:** the models the kernel host's `Program` works through.

**Settled on the way:**
- **QIMAGE.LoadFromPlot:** the chart's pixels become the picture, as on the desktop.
- **The page's icon:** it follows Application.Icon, so a bundle's tab shows the program's icon.
- **DataSource / DataField:** a user's edit or click on a bound component writes the field (`kernel_web::bound_input`). The other direction is unchanged.
- **A QIMAGE given a PNG / JPEG:** it isn't loaded and the console warns, as the desktop does. The DOM host had shown it in an `<img>`.
- **A native web build** can't wait for the kernel's boxes, so its MESSAGEBOX / SHOWMESSAGE / file / colour / font dialogs keep the browser's `alert` / `confirm` / `prompt` / colour input.
- **The title bar** shows the form's icon (IcoHandle / Icon, else Application.Icon), as the DOM host's did.
- **The pointer** follows Screen.Cursor and the components' Cursor, by the desktop's rule: `cursor_at` moved from the winit host into `rapidr_ui_app::desktop`, shared by both hosts.

**The tests** are kernel-host only:
- `web_gui_run.mjs` has no DOM runner.
- `web_gui_parity.mjs` and `web_a11y.mjs` have no DOM mode. Their checks that pixels, trees and dumps don't cover are ported: the tray strip, the title-bar glyphs, the page's icon, and a11y_form's keys (Tab order, mnemonics, Enter / Escape, the focus ring by its pixels, the status bar's live region).
- `message_dialogs`, `design_surface` and `size_grip` now run on the web too.
- New: `tests/web_webapi.mjs` (the five API components and LoadFromPlot). `tests/web_sqlite.mjs` (the binding both ways) joins `tools/regress.sh`.

**Open after W11.**
- `dialog_timers` on the kernel host gives the desktop's dump in most runs but stays off the web cases: its ticks are counted in tens of milliseconds, and the open file dialog's step misses now and then. Its real bug is fixed: a SLEEP now holds the program whole, as the desktop's does (`dialog_web::sleep`). The timers that fell due meanwhile fire once the program waits. They no longer run inside the program on its way out of the SLEEP, where a handler's MESSAGEBOX couldn't wait (it answered 0).
- An open menu's items aren't in the accessibility tree (desktop and web alike).
- Ad-hoc scripts outside `regress.sh` still look for DOM-host elements: `corpus_web_compare.mjs`, `verify_sqlite_rendering.mjs`, `test_dropdown_sqlite_hover.mjs`, `test_all_dropdown_examples.mjs`, `debug_e2e_event_handling.mjs`, `_q.mjs`.

**The web IDE** (`web-ide/`) runs its preview and builds on the kernel host. Retiring the HTML / Monaco IDE in favour of the kernel-drawn MDI IDE (§3.9, ROADMAP's IDE phase) needs:
- the MDI IDE's program runs in the page;
- the program under test runs in the workspace as MDI windows rather than in the preview frame;
- the code editor has the language service's UI;
- the IDE suites (`web_ide_*`) are moved onto it.
