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
                        program events, show / close / modal,        ui/testhooks.rs (Stage W2)
                        timers, dialogs' state, test hooks
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
| W1 | `rapidr-ui-render`: `canvas.rs`, `cpu.rs`, `images.rs`, `gpu.rs` out of the desktop host (no behaviour change; the desktop matrix and the spike's byte-identity test green); the spike on it; wasm SIMD build | 1 | Single owner |
| W2 | `rapidr-ui-app`: the host-neutral half of runtime-core's `ui/kernel.rs` (KernelEvent → `rp_fire_event*`, Set / List / Container / MenuPick dispatch, show / hide / close actions / OnShow / modal list, the timer heap, `form_resized`, WindowState's simulation) and `ui/testhooks.rs`, behind a small `Program` trait both runtimes implement; ideally one component registry for both runtimes | 3–4 | Single owner, with the desktop lanes' integrator |
| W3 | The web host proper: `WebHost` (canvases per form, kernel-drawn frames, stacking, moving, sizing, dpr, context loss, rAF and deadlines), input / IME / clipboard / autofill, the mirror in Rust, `WebStore`; `rapidr-runtime-web` feature `kernel` and `?host=kernel` (the DOM host stays the default) | 4–6 | Single owner |
| W4 | The VM and native web builds on it: slices + dispatch, ShowModal / dialogs / DOEVENTS / INKEY$, the kernel's dialogs, timers through `rapidr-ui-app` | 3–4 | Single owner |
| W5 | Parity: `web_gui_parity.mjs` on the kernel host (dumps equal to the desktop's), pixel captures equal to the desktop headless host's at 1× and 2× (and 1.5×), `web_a11y.mjs` on the mirror; each case gets `webKernel: true | "pending: …"` | 2–3 | Tests lane |
| W6 | Web-only components as overlays, the popup layer above them | 2–3 | Parallel after W3 |
| W7 | Fonts as assets, the shared Noto fallback set (web on demand, desktop and captures), the OFL notice in built programs | 2–3 | Parallel after W3 |
| W8 | File dialogs on the web through `file_dialog.rs`; `dialog_web.rs` retired | 1–2 | Parallel after W4 |
| W9 | Performance: damage rectangles, the size levers (§7), WebGPU decided by measurement | 2–3 | Parallel after W3 |
| W10 | The web IDE on the kernel (Phase 3's MDI IDE in the page) | in Phase 3's budget | |
| W11 | Switch: the kernel host the web default, one release with `?host=dom`; then delete the DOM runtime (§5) | 2 | Single owner |

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

After W11's release with `?host=dom` as the escape hatch, delete: `gui_web.rs`, `dialog_web.rs`, `menu_web.rs`, `a11y_web.rs`, `layout_web.rs`, `scroll_web.rs`, `mdi_web.rs`, the DOM half of `object_web.rs` (the registry stays, shared since W2), `rapidr-rrcss` (unless RDOM wants a base stylesheet), the DOM IDE after W10, and the DOM-specific parts of the web tests. The web runtime keeps what isn't GUI (builtins, files, storage, network, database, data science, the VM host) and the web-only components' overlays. Every component then has one implementation (the kernel's), one look, one accessibility tree and one input path, on the desktop, the web and, later, mobile.

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
