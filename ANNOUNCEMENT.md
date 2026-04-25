# RapidR 1.0.0 — Announcement Drafts

Three flavours of the same announcement. Pick whichever matches the venue.

---

## A. RapidQ user-group post / forum (warm, hobbyist tone)

> **RapidR 1.0.0 — a modern reincarnation of RapidQ**
>
> Long-time RapidQ users — you might enjoy this. RapidR is a from-scratch
> Rust reimplementation of the BASIC dialect we all remember, with the
> same `CREATE Form1 AS RForm` / `END CREATE` flavour, the same SUBs and
> FUNCTIONs, the same component-driven feel. It is **inspired by**
> RapidQ but is **not** a drop-in clone — instead it goes where RapidQ
> never quite did: the modern web.
>
> What's in 1.0:
>
> - **Native + Web from one source.** Compile a `.rr` file to a fast
>   native binary (FLTK), or to WebAssembly that runs in a browser tab.
> - **52+ GUI components** plus 9 web-exclusive ones (`RWebView`,
>   `RWebStorage`, `RDOM`, `RJavaScript`, …), 100+ built-ins, MySQL +
>   SQLite, networking, JSON, and a small data-science stack
>   (`RDataFrame`, `RPlot`, `RNum`).
> - **A self-hosted Web IDE.** Open one HTML file in a browser, build
>   forms visually with a 22-component toolbox, edit code in Monaco,
>   click Run to execute it inline, click Build to download a single
>   `.zip` you can drop on any static host. No build server. No
>   network calls.
> - **Bytecode interpreter (`rapidrintr`).** The same wasm module hosts
>   both the compiler *and* the runtime, which is what makes the
>   browser IDE a single-page download.
> - **VS Code extension** with hover docs, IntelliSense for components
>   and built-ins, and one-click Compile / Run / Compile-for-Web.
> - **Open source under the MIT license.**
>
> Try it: `git clone … && ./build.sh --release`, or just open
> `web-ide/index.html` in a browser after running
> `bash tools/build_web_artifacts.sh`. README has the rest.
>
> If you used RapidQ in the early 2000s, give 1.0 a try — and please
> file an issue when something feels wrong, because backwards
> compatibility is a goal, not a guarantee.

---

## B. Hacker News / Reddit (one-paragraph, technical)

> **Show HN: RapidR 1.0 — a BASIC-to-Rust transpiler with a self-hosted
> browser IDE**
>
> RapidR compiles a RapidQ-flavoured BASIC dialect (`.rr` files) to
> standalone Rust projects, then to native binaries (FLTK GUI) or to
> WebAssembly that runs in the browser. 1.0 ships a self-hosted Web
> IDE — plain HTML/JS, no backend — that drives the same wasm module
> exporting both the compiler and a small bytecode interpreter, so
> design-time and runtime use the exact same renderer. Build a form
> visually, hit Run to execute it in an iframe, hit Build to download
> a STORED PKZIP containing the bytecode + runtime + a CSP-locked
> `index.html` ready to drop on any static host. Includes 52+ GUI
> components, MySQL/SQLite, an `RDataFrame`/`RPlot` mini data-science
> stack, a VS Code extension, and end-to-end Playwright tests covering
> the IDE → bundle → standalone-runtime path. MIT.

---

## C. README banner / GitHub release notes

> # RapidR 1.0.0
>
> First stable release of RapidR — a BASIC-to-Rust transpiler, native
> + web runtime, and self-hosted browser IDE in one repo.
>
> **Highlights**
>
> - **Compiler pipeline.** `.rr` → AST → Rust → native binary (FLTK)
>   *or* `.rr` → bytecode (`.rrbc`) → wasm interpreter `rapidrintr`.
> - **Self-hosted Web IDE (`web-ide/`).** Multi-form designer, 22
>   components in 3 groups, type-aware property grid (color/font live
>   pickers, asset dropdowns, enum selects), Monaco-based code editor,
>   1-click Build to a CSP-locked static-host bundle.
> - **52+ GUI components** native, 9 web-exclusive, 100+ built-ins,
>   MySQL + SQLite, sockets, HTTP, JSON, mini data-science stack.
> - **VS Code extension** at `utilities/vscodeext/rapidr/` with
>   IntelliSense + hover docs + Compile / Run / Compile-for-Web.
> - **Test coverage.** Compiler, runtime, web-bundle parity, and Web
>   IDE end-to-end (Playwright drives the IDE, downloads the bundle,
>   spawns a separate HTTP server, opens the bundle, clicks a button,
>   asserts the label updates).
> - **Security.** Bundled `index.html` ships a strict
>   Content-Security-Policy; all user-controlled strings interpolated
>   into IDE DOM are escaped; no `eval()`/`new Function()` outside
>   vendored Monaco + wasm-bindgen glue.
> - Third-party attributions in [`LICENSES.md`](LICENSES.md). MIT.
>
> **Try it**
>
> ```bash
> git clone https://…/RapidR.git
> cd RapidR
> ./build.sh --release
> bash tools/build_web_artifacts.sh
> python3 -m http.server 8765
> open http://localhost:8765/web-ide/index.html
> ```
