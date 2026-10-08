# RapidR security audit (2026-10-08)

Audit of `development` at the point this worktree was cut from it (merge `bb23d078`,
workspace version 2.117.0), with extra weight on the web, as the owner asked
("this should be super secure, and even more when it comes to the web").

Method: threat model first, then read the web runtime and canvas host, the
sandboxed program frame (`ide/web/`), the bytecode loader, the preprocessor,
the local services (LSP / DAP / session / build server), the runtime features
(QREGISTRY, files, SHELL/RUN, printer, sockets/HTTP), every `unsafe` block, and
the supply chain (`cargo deny`, `cargo audit`, npm, the release scripts). Each
material finding has a proof-of-concept run locally and a recommended fix; the
one trivial, clearly-safe fix (SEC-14's missing input validation, which the
predecessor code had) was applied in this pass. Everything else is left as a
precise finding for a follow-up fix pass.

Nothing in this audit was tested against any external host. The PoC assets are
under the session scratch dir (`scratchpad/poc/`, `scratchpad/supply/`), not in
the repo.

---

## 1. Threat model

### Assets
- The developer's machine: files, processes, network position, other local users.
- The developer's source, projects and (future) AI API keys.
- A RapidR program's own data at run time (user input, HTTP responses, DB rows).
- The integrity of RapidR Studio (its state, storage, File System Access handles)
  when it runs an untrusted program in its preview frame.
- A deployed web program's origin (its storage, its users' sessions).

### Attackers
1. **A malicious program run by a developer** — a `.bas`/`.rr`/`.rrbc` opened or
   built, or run inside Studio-web's preview frame. It may use every documented
   component, including the deliberately powerful ones (RJavaScript, RDOM,
   RWebView, SHELL, DECLARE…LIB).
2. **A malicious web page that embeds or links to RapidR** (a deployed program,
   or the web IDE) — cross-origin messages, DNS rebinding, clickjacking.
3. **A malicious project / example / corpus file** opened or built — crafted
   `$INCLUDE` / `$RESOURCE`, crafted asset names, crafted `.rrbc`.
4. **A local attacker on the same machine** — another process or user reaching a
   local service (LSP/DAP/session/build server, future MCP socket) or a
   predictable temp path.
5. **The network** — a server a program's RHTTP/RSOCKET talks to, or data flowing
   into a program; and the supply chain of RapidR's own dependencies.

### Trust boundaries
- Studio page ⇄ the program under development (separate process on desktop;
  opaque-origin sandboxed iframe on web). **This is the primary web boundary.**
- RapidR ⇄ the project's content (untrusted data, never instructions).
- The machine ⇄ local services (who may connect, and with what authority).
- RapidR ⇄ AI providers / MCP clients (planned; see §8 of docs/ide-ai.md).
- RapidR's build ⇄ its dependency supply chain.

### By-design, per SECURITY.md (not treated as vulnerabilities here)
`RJavaScript.Eval`, `RDOM.InnerHTML` / attributes, `RWebView.HTML` / `.Navigate`,
`RUSTSTART…RUSTEND`, `DECLARE…LIB`, and "a program doing what its author wrote
(e.g. deleting files)" run author-supplied code/markup. The findings below are
about cases where **untrusted data or another trust domain** gains that power
without the author opting in, or where an isolation boundary leaks.

---

## 2. Findings

Ranked severity × likelihood. IDs continue ROADMAP's `SEC-` series (last used
SEC-13).

| ID | Sev | Area | One line |
|---|---|---|---|
| SEC-14 | **High** | Web sandbox | Studio's font bridge let the sandboxed program frame read any same-origin IDE file (fixed in this pass) |
| SEC-15 | Medium | Web program | Deployed web bundle (`rapidr build --web`) ships **no CSP**; `RWEBVIEW` srcdoc keeps `allow-same-origin allow-scripts` by default (SEC-12 still open in the kernel host) |
| SEC-16 | Medium | Build output | `render_index_html` / `render_loader_js` / asset-map interpolate the project name and asset names into HTML+JS with no (or `"`-only) escaping → injection in the built bundle |
| SEC-17 | Medium | Legacy web IDE | `web-ide/` (Monaco IDE) is still the shipped web artifact (`tools/release/web.sh`) and still `include_str!`'d into every bundle; it carries the pre-SEC-05/escaping `innerHTML` sinks and a permissive `.htaccess` |
| SEC-18 | Low | Local service | `rapidr lsp` / `rapidr dap` accept `file://` URIs to any path and read/stat them; no project confinement (acceptable for an editor-spawned stdio server, but worth stating) |
| SEC-19 | Low | FFI (in progress) | The DLL-call / VARPTR / PEEK-POKE lane is unimplemented-but-present risk; see the review checklist in §6 |
| SEC-20 | Low | Supply chain | 3 informational RustSec advisories reachable in shipped binaries (`ttf-parser`, `memmap2`, and `anyhow` which is not actually linked); one stale `deny.toml` ignore |

Positives confirmed (no finding): the Studio preview frame runs at an **opaque
origin** (no `allow-same-origin`), with a private `MessageChannel` handshake that
checks `e.source === parent` and ignores window messages afterward; the bytecode
loader bounds every count against the remaining buffer and caps local slots
(`interpreter/rapidr-bytecode/src/io.rs:258`, `:311`); the VM has a call-depth
limit (`MAX_CALL_DEPTH = 100_000`) and a watch-evaluation fuel limit; the build
server is bound to loopback with Host/Origin guards and `..`-safe path checks
(SEC-01/SEC-11, verified still in place); QREGISTRY is scoped to a per-user store
/ `RAPIDR_REGISTRY`; the VM and both VM hosts are `#![forbid(unsafe_code)]`;
downloaded programs get a quarantine/MOTW "run it?" prompt before running
(`crates/rapidr-cli/src/launch.rs:378`).

---

### SEC-14 — High — Studio web font bridge: arbitrary same-origin read from the sandboxed frame

**Location:** `ide/web/studio.js:138-143` (before this pass).

**What.** Studio runs the program under development in `run.html`, an iframe with
`sandbox="allow-scripts allow-modals allow-downloads"` and **no**
`allow-same-origin`, so the program is at an opaque origin and cannot fetch the
IDE origin or reach Studio's OPFS/state — the core web isolation guarantee. But
the program's opaque frame cannot fetch its own fallback fonts, so it asks the
parent over the port: `{ __rapidr_font: { id, file } }`. Studio answered:

```js
const { id, file } = d.__rapidr_font;
fetch(new URL("runtime/fonts/" + file, location.href))   // file not validated
  .then(r => r.ok ? r.arrayBuffer() : null)
  .then(bytes => run.port.postMessage({ __rapidr_font_reply: { id, bytes } }, [bytes]));
```

`file` is attacker-controlled: a program can call `window.RAPIDR_FONT_FETCH(...)`
from `RJavaScript.Eval` (which runs in the opaque frame; `run.html` sets no CSP),
or any in-frame script can. `file = "../../secret.txt"` resolves against
`location.href` to `http://<ide-origin>/secret.txt`; Studio (the real origin)
fetches it and returns the bytes to the frame. The frame thus gains a read
primitive on the whole IDE origin — exactly what the opaque origin is meant to
deny — and can exfiltrate what it reads (it has unrestricted network). On a plain
static deployment that is `studio.rrbc`, the examples, and any other file served
from that origin; the impact scales with whatever else shares the origin (other
users' shared projects, an app behind the same host, etc.). The predecessor
`web-ide/host.js:404` validated this with `/^[\w.-]+$/`; the Studio rewrite
dropped the check.

**PoC (run locally, headless Chromium).** `scratchpad/poc/`:
`run_poc.mjs` serves the built `target/studio-web/` on 127.0.0.1:8790 with a
planted `secret.txt` at the origin root, and `poc_harness.html` loads the **real
`run.html`** in the sandboxed iframe exactly as `studio.js` does, booting it with
a tiny "runtime" module that calls `window.RAPIDR_FONT_FETCH("../../secret.txt")`.
Result:

```
VULNERABLE: sandboxed program frame read same-origin IDE file ../../secret.txt => "TOP-SECRET-STUDIO-ORIGIN-FILE"
```

**Fix (applied in this pass).** `ide/web/studio.js` now rejects any `file` that is
not a plain font file name before fetching:

```js
if (typeof file !== "string" || !/^[\w.-]+$/.test(file) || file.includes("..")) {
  if (run.port) run.port.postMessage({ __rapidr_font_reply: { id, bytes: null } });
  return;
}
```

This matches every real chunk name in `tools/fonts.py`'s index (e.g.
`NotoSansSC-Regular.000.otf`, `index.json`) and rejects `..`, path separators and
absolute paths. The same unvalidated pattern exists in the legacy
`web-ide/host.js:405` (`fetch(./runtime/fonts/${file})`) — there it *is* guarded
at `:404`; keep that guard if `web-ide/` is kept (see SEC-17).

**Regression:** `tests/security/web_font_bridge_traversal.mjs` (added) — pins the
guard in the shipped source and runs the predicate against a traversal battery;
dependency-free so it runs in `tools/regress.sh`. Add it to the `web` stage of
`tools/regress.sh`.

---

### SEC-15 — Medium — Deployed web program has no CSP; RWEBVIEW keeps `allow-same-origin`

**Location:** `interpreter/rapidr-webbundle/src/lib.rs:125` (`render_index_html` —
no CSP meta); `crates/rapidr-runtime-web/src/overlay_web.rs:89` and `:163-164`
(RWEBVIEW iframe `sandbox`).

**What.**
1. The page `rapidr build --web` generates has **no `Content-Security-Policy`**
   at all (the Studio page `ide/web/index.html` does have a strict one; the
   deployed-program page does not). A deployed program that shows any untrusted
   data through `RDOM.InnerHTML`, `RDOM.SetAttribute` (arbitrary attribute name
   and value, so `onerror=` etc.), or `RWebView.HTML` therefore has no
   defence-in-depth second layer. These sinks are "by design" for author markup,
   but with no CSP the blast radius of a mistake (markup built from HTTP/DB/AI
   data) is the full origin.
2. Every `RWEBVIEW` iframe is created with
   `sandbox="allow-scripts allow-same-origin …"` (`overlay_web.rs:89`), and the
   `html` property goes to `set_srcdoc` (`:155`). `allow-scripts` +
   `allow-same-origin` on srcdoc content means that content runs **with the
   program's own origin** — it is not sandboxed from the app. This is the exact
   SEC-12 condition from ROADMAP, still present in the kernel host. For author
   HTML it is intended; the risk is a program that puts untrusted HTML into
   `RWebView.HTML`. A program can also set/clear the sandbox itself
   (`overlay_web.rs:163`, `RWebView.Sandbox = …`) — fine, but means the default
   should be the safe one.

**PoC.** Code-level; not separately executed (requires a full `rapidr build
--web` of a program that injects into RDOM/RWebView). The sinks and the missing
CSP are confirmed by reading `render_index_html` (no `<meta http-equiv>`) and
`overlay_web.rs:89`.

**Recommended fix.**
- Generate a CSP for the bundle derived from the components the program uses
  (ROADMAP's open item "CSP generated per bundle"): `script-src 'self'
  'wasm-unsafe-eval'`; `'unsafe-eval'` only if `RJavaScript` is used;
  `connect-src` listing the hosts the program's RHTTP/RSOCKET/RAI target;
  `frame-src` only if `RWebView` is used; `object-src 'none'`; `base-uri 'none'`.
  The legacy `web-ide/zip.js:88` already emits such a CSP for its bundles — port
  that logic into `rapidr-webbundle`.
- Default `RWEBVIEW` srcdoc content to **drop `allow-same-origin`**; keep an
  explicit opt-in property (`RWebView.SameOrigin = True`) for authors who need it
  (SEC-12's recommended fix).

**Regression to add:** `tests/security/web_bundle_csp.mjs` — build a bundle whose
program uses only RDOM, assert `index.html` carries a CSP with no `'unsafe-eval'`
and no `frame-src`; a second program using RWebView asserts `frame-src` is present
and srcdoc iframes have no `allow-same-origin` unless opted in.

---

### SEC-16 — Medium — Project name and asset names injected into the built bundle

**Location:** `interpreter/rapidr-webbundle/src/lib.rs:125-160` (`render_index_html`
`<title>{title}`), `:161-180` (`render_loader_js`, `project_name` into JS string
and template literals), `:131-134` (asset map: `escaped_name = name.replace('"',
"\\\"")` — only `"` escaped).

**What.** `title` and `project_name` are the source file's stem
(`crates/rapidr-cli/src/main.rs:453`, `:948`), i.e. a file name, interpolated
unescaped into `<title>…</title>` and into JavaScript string/template literals.
Asset names (project file names) are interpolated into an inline `<script>` with
only `"` escaped — a name containing `</script>` or a backslash breaks out. A
file or asset named, e.g., `</title><img src=x onerror=…>` or
`x</script><script>…` yields HTML/JS injection **in the generated bundle**.

Likelihood: the developer usually controls these names, so this is mainly a
robustness/defence issue — but it becomes real when a build pipeline or IDE
builds an **untrusted project** (crafted `.rrproj` asset names, a corpus file with
an odd name). The build server avoids it today only because it writes a fixed
`program.rr` (`crates/rapidr-buildserver/src/main.rs:239`).

**PoC.** Code-level (reading the three interpolation sites). Not executed as a
full build.

**Recommended fix.** HTML-escape `title` in `<title>`; JSON-encode `project_name`
and every asset name when emitting them into JS (`serde_json::to_string`), and
HTML-escape when emitting into HTML. Reject or sanitize project/asset names
containing path separators or control characters at project-load time.

**Regression to add:** `tests/security/web_bundle_injection.rs` (unit test in
`rapidr-webbundle`) — build with `project_name` / asset name containing
`</script>`, `</title>`, `"`, `\`, and assert the rendered HTML/JS contains the
escaped forms only.

---

### SEC-17 — Medium — The legacy Monaco web IDE is still shipped and still embedded

**Location:** `tools/release/web.sh:15` (`SITE="${1:-web-ide}"` — the release web
artifact is `web-ide/`); `interpreter/rapidr-webbundle/src/lib.rs:69-70`
(`include_str!("../../../web-ide/{bundle_console,ansi_screen}.js")` — pulled into
every bundle); `web-ide/.htaccess` (`Access-Control-Allow-Origin: *` on js/mjs/
wasm/json/css); `web-ide/host.js` numerous `innerHTML` sinks with an `escapeHtml`
(`host.js:3847`) that does not escape `"` (attribute-context injections at
`host.js:~3304`, `:3381`, `:3385`, and `bodyHtml`-into-`innerHTML` at `:2180`,
`:3893`).

**What.** The IDE plan (docs/ide-plan.md I1) says `web-ide/` is deleted once the
kernel Studio reaches parity, and `ide/web/` (the kernel Studio) now exists — but
`web-ide/` is still what `tools/release/web.sh` packages by default, and its two
JS helpers are compiled into every program bundle via `include_str!`. So the old
IDE, with its weaker escaping and its wildcard-CORS `.htaccess`, is still on the
shipping path. Its preview isolation (SEC-02/03) is in place
(`web-ide/index.html:263` opaque sandbox, `host.js:435` `e.source` check), but the
DOM-injection surface in `host.js` predates the SEC-05 hardening done for the
runtime.

**Recommended fix.** Decide the cutover: either (a) make `tools/release/web.sh`
ship `target/studio-web/` (the kernel Studio) and stop embedding `web-ide/*.js`
(move `bundle_console.js` / `ansi_screen.js` into `rapidr-webbundle`'s own
sources), or (b) if `web-ide/` must ship for now, fix `escapeHtml` to also escape
`"` and `'`, replace the `bodyHtml`-into-`innerHTML` paths, and drop the
wildcard-CORS `.htaccess` (scope it, or rely on same-origin). Per the project's
"one implementation, no fallbacks" rule, (a) is the direction.

**PoC.** Code-level. Not executed (legacy path).

---

### SEC-18 — Low — LSP/DAP read arbitrary paths from `file://` URIs

**Location:** `crates/rapidr-lsp/src/lib.rs:576` (`uri_to_path`) and the many
`uri_to_path` call sites; `crates/rapidr-dap/src/adapter.rs:~405` (`start_program`
joins `cwd`+`program`, no confinement); `crates/rapidr-session/src/process.rs`
(spawns `rapidr run --session <program>` for any path).

**What.** `rapidr lsp` and `rapidr dap` are stdio servers an editor launches; they
open, read and analyse whatever file URI the client sends, and DAP spawns the
runtime on any program path the client names, with client-supplied `env`
(`adapter.rs:431`). There is no project-root confinement. For the intended model
(a local editor the user already trusts, talking over stdio) this is standard and
acceptable — an editor can already read those files. It is listed so the planned
`rapidr mcp` socket (docs/ide-ai.md §4) does **not** inherit the same "any path,
any env" latitude for a *remote* MCP client: the MCP server must confine reads to
the project and must not take arbitrary `env` from the client.

**Recommended fix.** No change for LSP/DAP now. When MCP lands, enforce
project-root confinement on all file tools and ignore client-supplied environment
(the plan already says tools can't read outside the project — make `uri_to_path`
/ path joins reject `..` and absolute escapes in the MCP layer, and reuse the
build server's loopback+token+Origin/Host guard).

---

### SEC-19 — Low (design, in progress) — DLL-call / VARPTR / PEEK-POKE lane

The Windows FFI lane (`DECLARE … LIB`, `VARPTR`, `PEEK`/`POKE`) is partly present:
`crates/rapidr-runtime-core/src/ffi.rs` (68 `unsafe` blocks) loads an arbitrary
library by program-supplied path and calls an arbitrary symbol with up to 4
integer/float args, with **no signature check** and a raw `CStr::from_ptr` on the
integer a DLL returns for a STRING result (`ffi.rs:265`). This is inherent to
`DECLARE…LIB` (an author-opt-in, by-design power), but the VARPTR/PEEK/POKE
pointer model is the part still being designed. Its review checklist is in §6;
run that pass when the lane lands. Until then: `ffi` is a default-on feature of
`rapidr-runtime-core` and the `full` feature of `rapidr-vm-host-native`, so any
native/interpreted program can already load libraries — keep it **out of any
sandboxed-run path** (AI-initiated runs, extension VMs, the web).

---

### SEC-20 — Low — Supply chain

`cargo deny check` (advisory DB 2026-10-07, 633 crates) and `cargo audit` both
exit clean. Three informational RustSec advisories remain, none with a CVSS score:

| Advisory | Crate | Shipped? | Note |
|---|---|---|---|
| RUSTSEC-2026-0192 | `ttf-parser` 0.25.1 | yes | unmaintained; already ignored in `deny.toml:20`; skrifa is the way off |
| RUSTSEC-2026-0186 | `memmap2` 0.9.10 | yes (via `fontique`/`parley`, and `winit` on Linux) | unsound `advise/flush_range`; fixed in 0.9.11. **Not** caught by `cargo deny` (no `unsound` setting). Bump the lock. |
| RUSTSEC-2026-0190 | `anyhow` 1.0.102 | no | in `Cargo.lock` only (via inactive `wit-*` optionals); fixed in 1.0.103 |

Also: `deny.toml:18` ignores RUSTSEC-2026-0173 (`proc-macro-error2`) but that crate
is no longer in the graph (the mysql upgrade dropped it) — the ignore is **stale**,
remove it. `deny.toml:52` allows `CC0-1.0` which nothing uses (harmless warning).
42 duplicate-version warnings (`multiple-versions = "warn"`), none security-relevant.

npm: both `package.json` lockfiles (`tests/`, `utilities/vscodeext/`) audit clean
(0 vulns). Vendored Monaco is 0.52.2 (MIT), read by `tools/release/sbom.py`.

**Release packaging** (`tools/release/`) — notable, not vulnerabilities:
- Signing is opt-in; identities come from the command line / user keychain,
  nothing hard-coded. macOS entitlement `com.apple.security.cs.disable-library-
  validation` is on `rapidr` only (for `DECLARE…LIB`), a deliberate trade-off.
- `SHA256SUMS` is produced but **not signed** (no GPG/minisign/cosign) — consider
  signing release checksums so downloads are verifiable.
- Windows Authenticode timestamp URL is plain `http://timestamp.digicert.com`
  (`windows.ps1:155`, `:178`) — standard for RFC-3161 timestamping (the response is
  signed), but worth noting.
- `rapidr setup` does `curl … https://sh.rustup.rs | sh` (unix) /
  downloads `rustup-init.exe` to a predictable `%TEMP%` path, unverified
  (`crates/rapidr-cli/src/setup.rs:271`, `:280`) — only on explicit `rapidr setup`,
  user-confirmed, but the Windows temp path is predictable and the download is not
  hash-pinned. Consider a pinned hash or writing to a fresh mkdtemp dir.
- `tools/release/linux/setup-tools.sh` uses `http://` Ubuntu mirrors but verifies
  via `gpgv` + SHA-256 chained from the signed `InRelease`; LLVM-MinGW defaults to
  the GitHub "latest" release (unpinned unless `-LlvmMingw` is passed).

---

## 3. `unsafe` inventory

135 real `unsafe` occurrences, all in native (desktop) code; **none** in the web
runtime, the web host, the interpreters, the compiler, or any wasm-reachable
crate. The VM and both VM hosts are `#![forbid(unsafe_code)]`. No `static mut`,
no `#[no_mangle]`, no `union` anywhere. Full per-line inventory is long; summary
by crate:

| Crate / file | Count | What | Assessment |
|---|---|---|---|
| `rapidr-runtime-core/src/ffi.rs` | 68 | `libloading` load + symbol call as `extern "C" fn` for every 0–4 arg int/float combo; `CStr::from_ptr` on DLL-returned pointer (`:265`) | The main exposure. No signature check, no pointer validation; inherent to `DECLARE…LIB`. Behind default-on `ffi` feature. **Review with SEC-19 / §6; keep out of sandboxed runs.** |
| `rapidr-runtime-core/src/terminal.rs` | 7 | libc termios/read/poll (Unix), CRT `_kbhit`/`_getch` (Windows) | Sound: local structs, checked returns, SAFETY comments, Mutex-guarded saved state. |
| `rapidr-runtime-core/src/object.rs` | 4 (1 non-test) | lifetime-erasing `transmute` of `*mut dyn FnMut` for re-entrant wait serving (`:1557`, `:1588`) | High-quality: drop guards + re-entry flag + SAFETY comments. 2 are test-only. |
| `rapidr-runtime-core/src/joystick/evdev.rs` | 3 | `ioctl`, `read_unaligned` of `#[repr(C)]` structs from exact-size buffers | Sound: sizes from `size_of`, `chunks_exact`. |
| `rapidr-runtime-core/src/serial.rs` | 3 | `openpty`/`from_raw_fd` | **Test-only** (`cfg(all(test, linux))`). |
| `rapidr-ui-host-winit/src/tray.rs` | 13 | objc2 class, Win32 DIB/icon/tray, `slice::from_raw_parts_mut` (`:304`) | Mostly sound; `icon()` (`:288`) does not assert `rgba.len() == w*h*4` and does unchecked `w*h*4` — `chunks_exact(4)` prevents over-read, `CreateDIBSection` failure returns early. **Add a length assert.** |
| `rapidr-ui-host-winit/src/tracking.rs` | 10 | lifetime transmute for popup tick + CF/Win32 timer callbacks | Sound: drop guards, re-entry flag, SAFETY comments. |
| `rapidr-ui-host-winit/src/platform.rs` | 10 | Win32 SPI/registry/cursor; X11 via x11_dl; `unsafe impl Send/Sync for X` (`:217`) | The X11 `Send`/`Sync` depends on a **documented-but-unenforced** main-thread-only use; `global_mouse`/`beep` are callable from any thread and Xlib isn't thread-safe without `XInitThreads`. **Flag: enforce main-thread or add XInitThreads.** |
| `rapidr-ui-host-winit/src/menu.rs` | 2 | muda context-menu on live window handle | Sound: main thread, live handle, SAFETY comments. |
| `rapidr-launcher/src/main.rs` | 6 | objc2 NSApplication delegate (macOS) | Standard objc2 pattern, SAFETY comments. |
| `rapidr-cli/src/launch.rs` | 2 | `libc::getxattr` for quarantine/MOTW (`:360`, `:362`) | Sound: NUL-terminated CStrings, buffer length passed, `len < 0` handled. |
| `rapidr-editor/examples/editor_bench.rs` | 7 | counting `GlobalAlloc` wrapper | Bench example, not shipped; thin forwarding to `System`. |

Recommendations: (1) add `#![forbid(unsafe_code)]` to the many crates that have
no `unsafe` today (lexer, parser, ast, value, preprocessor, project, session,
langsvc, lsp, dap, db, designer, studio, ui-kernel, ui-render, ui-app,
runtime-web, codegen-rust, diagnostics, icons, lang, bytecode, bcgen,
compiler-wasm, webbundle) so it can't creep in; (2) run Miri over `ffi.rs`'s
non-FFI logic and the `object.rs`/`tracking.rs` transmutes (ROADMAP SEC-08);
(3) the two flags above (`tray.rs::icon` length assert; `platform.rs` X11 thread
safety).

---

## 4. Supply chain — see SEC-20.

Captured output: `scratchpad/supply/{deny.txt,audit.txt,audit.json,cargo-tree-advisories.txt,npm-audit-*.json}`.

---

## 5. Secure-by-default checklist for future lanes

**Web sandbox**
- The program frame stays opaque-origin (no `allow-same-origin`), private
  `MessageChannel`, `e.source === parent` check, window messages ignored after
  boot. (Holds today.)
- Every bridge the frame can call must **validate its input as data**, never as a
  path or a capability: font names match a font-name pattern (SEC-14); file
  pickers and storage are off unless the lane grants them (`filePickers:false`,
  `storage:{}` — holds); no bridge proxies a same-origin fetch on a
  frame-supplied path.
- Every page RapidR emits carries a CSP derived from the components used (SEC-15);
  `object-src 'none'`, `base-uri 'none'`, `'unsafe-eval'` only for `RJavaScript`.
- `RWEBVIEW` / `RDOM` default to the isolating option (no `allow-same-origin` on
  srcdoc); author opt-in to widen.

**Paths**
- Never join an untrusted-supplied path without rejecting non-`Normal` components
  (the build server's `is_safe_relative_path` is the model). Applies to
  `$INCLUDE`/`$RESOURCE` in IDE/MCP contexts, MCP file tools, bundle asset names,
  the font bridge.
- HTML-escape / JSON-encode every program-derived string put into generated
  HTML/JS (SEC-16).

**IPC / local services**
- Loopback bind only; Host + Origin guards (block DNS rebinding); a per-start
  256-bit token in a 0600 file for any socket; no CORS headers granted (the build
  server and the planned MCP server; reuse one guard module).
- No client-supplied environment or out-of-project path for anything a *remote*
  (MCP) client drives (contrast SEC-18, where a local editor is already trusted).

**FFI / native power**
- `DECLARE…LIB`, `RUSTSTART`, VARPTR/PEEK/POKE never run in a sandboxed context
  (AI-initiated runs, extension VMs, the web); the "native-privileged" project
  flag (SEC-10) gates them with a confirmation.

---

## 6. Review checklist for the DLL-call / VARPTR / PEEK-POKE design (do this pass when the lane lands)

For `crates/rapidr-runtime-core/src/ffi.rs` and the new VARPTR/PEEK/POKE opcodes:

**Address validation & pointer provenance**
- [ ] Does `PEEK`/`POKE` accept an arbitrary integer address? If so it is, by
      construction, an arbitrary read/write of the process — an author-opt-in
      power. Confirm it is **gated by the native-privileged flag** and never
      reachable from a sandboxed run or the web (where it must be a no-op/error,
      as RapidQ's own web story has no memory).
- [ ] `VARPTR`/`STRPTR`/`CODEPTR`: what do they return and does anything downstream
      assume the pointer is still valid after the `Value` moves/reallocs? RapidR's
      `Value::String` is a Rust `String` that can reallocate — a pointer handed to
      a DLL must pin the buffer for the call's duration (document the lifetime;
      consider only allowing VARPTR into a fixed/pinned buffer type).
- [ ] Distinguish "RapidR-issued" handles from raw integers; never deref a plain
      program integer as a pointer without the native-privileged gate.

**Buffer sizes**
- [ ] For string/array out-params passed to a DLL, is the buffer length known and
      enforced, or does it trust the DLL to not overrun? `ffi.rs:265`'s
      `CStr::from_ptr` trusts a NUL to exist — a DLL returning a non-terminated
      buffer reads out of bounds. Prefer length-bounded reads where the ABI gives
      a length.
- [ ] `POKE` of a string/array: bounds-check the destination against a known
      allocation size.

**Call ABI / signatures**
- [ ] `ffi.rs` dispatches on arg count (0–4) and int/float class with **no check**
      that the declared signature matches the real C function — a mismatch is UB.
      Can the DECLARE's type info be used to pick the right trampoline and reject
      obviously-wrong arities? Document that a wrong DECLARE is the author's UB.
- [ ] More than 4 args silently returns null today — make it a clear error.

**Callbacks**
- [ ] If the lane adds C→RapidR callbacks (function pointers passed to a DLL),
      review re-entrancy into the VM (the VM is single-threaded, cooperative) and
      the lifetime of the trampoline; a callback firing on another thread must not
      touch VM state.

**Non-Windows behaviour**
- [ ] `DECLARE…LIB` / `VARPTR` / `PEEK`/`POKE` on macOS/Linux and on the web:
      define each explicitly (load `.so`/`.dylib`? error? no-op?). The web must
      never expose any of them. Keep `ffi` out of the web build (it already is).

**General**
- [ ] Run Miri on the non-FFI parts; keep the FFI module behind the default-on
      `ffi` feature but ensure every sandboxed entry point builds/links without it
      or refuses to call it.
- [ ] Add a SEC-id and a CHANGELOG line when it lands; add a test that the
      native-privileged flag is required.
