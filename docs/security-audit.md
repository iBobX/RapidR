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

| ID | Sev | Area | One line | Status |
|---|---|---|---|---|
| SEC-14 | **High** | Web sandbox | Studio's font bridge let the sandboxed program frame read any same-origin IDE file (fixed in this pass) | **Fixed** (audit pass) |
| SEC-15 | Medium | Web program | Deployed web bundle (`rapidr build --web`) ships **no CSP**; `RWEBVIEW` srcdoc keeps `allow-same-origin allow-scripts` by default (SEC-12 still open in the kernel host) | **Fixed** (SEC-FIX pass, 2026-10-08) |
| SEC-16 | Medium | Build output | `render_index_html` / `render_loader_js` / asset-map interpolate the project name and asset names into HTML+JS with no (or `"`-only) escaping → injection in the built bundle | **Fixed** (SEC-FIX pass) |
| SEC-17 | Medium | Legacy web IDE | The legacy HTML / Monaco IDE was still the shipped web artifact (`tools/release/web.sh`) and still `include_str!`'d into every bundle; it carried the pre-SEC-05/escaping `innerHTML` sinks and a permissive `.htaccess` | **Fixed** (SEC-FIX pass): not shipped, not embedded; then **deleted** (2026-10-08) |
| SEC-18 | Low | Local service | `rapidr lsp` / `rapidr dap` accept `file://` URIs to any path and read/stat them; no project confinement (acceptable for an editor-spawned stdio server, but worth stating) | **Fixed** (SEC-FIX pass); `rapidr mcp`'s authentication designed (§7) |
| SEC-19 | Low | FFI (in progress) | The DLL-call / VARPTR / PEEK-POKE lane is unimplemented-but-present risk; see the review checklist in §6 | Open (its own lane) |
| SEC-20 | Low | Supply chain | 3 informational RustSec advisories reachable in shipped binaries (`ttf-parser`, `memmap2`, and `anyhow` which is not actually linked); one stale `deny.toml` ignore | **Fixed** (SEC-FIX pass); checksum signing proposed (§8), the key is Robert's decision |

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
legacy IDE's `host.js:404` validated this with `/^[\w.-]+$/`; the Studio rewrite
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
IDE's `host.js:405` (`fetch(./runtime/fonts/${file})`) — there it *was* guarded
at `:404` (that IDE is deleted: SEC-17).

**Regression:** `tests/security/web_font_bridge_traversal.mjs` (added) — pins the
guard in the shipped source and runs the predicate against a traversal battery;
dependency-free so it runs in `tools/regress.sh`. Add it to the `web` stage of
`tools/regress.sh`.

---

### SEC-15 — Medium — Deployed web program has no CSP; RWEBVIEW keeps `allow-same-origin`

**Status: fixed** in the SEC-FIX pass (2026-10-08) — §9.1; what a program can
opt into, §9.2.

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
  The legacy IDE's `zip.js:88` already emitted such a CSP for its bundles — port
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

**Status: fixed** in the SEC-FIX pass — §9.3.

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

**Status: fixed** in the SEC-FIX pass — §9.4; the legacy IDE itself was deleted
on 2026-10-08.

**Location** (as found; the paths are gone): `tools/release/web.sh:15` (`SITE` defaulted to
the legacy IDE's folder — the release web artifact was the legacy IDE); `interpreter/rapidr-webbundle/src/lib.rs:69-70`
(`include_str!` of the legacy IDE's `bundle_console.js` and `ansi_screen.js` — pulled into
every bundle); the legacy IDE's `.htaccess` (`Access-Control-Allow-Origin: *` on js/mjs/
wasm/json/css); the legacy IDE's `host.js`, numerous `innerHTML` sinks with an `escapeHtml`
(`host.js:3847`) that does not escape `"` (attribute-context injections at
`host.js:~3304`, `:3381`, `:3385`, and `bodyHtml`-into-`innerHTML` at `:2180`,
`:3893`).

**What.** The IDE plan (docs/ide-plan.md I1) says the legacy IDE is deleted once the
kernel Studio reaches parity, and `ide/web/` (the kernel Studio) now exists — but
the legacy IDE is still what `tools/release/web.sh` packages by default, and its two
JS helpers are compiled into every program bundle via `include_str!`. So the old
IDE, with its weaker escaping and its wildcard-CORS `.htaccess`, is still on the
shipping path. Its preview isolation (SEC-02/03) is in place
(its `index.html:263` opaque sandbox, `host.js:435` `e.source` check), but the
DOM-injection surface in `host.js` predates the SEC-05 hardening done for the
runtime.

**Recommended fix.** Decide the cutover: either (a) make `tools/release/web.sh`
ship `target/studio-web/` (the kernel Studio) and stop embedding the legacy IDE's scripts
(move `bundle_console.js` / `ansi_screen.js` into `rapidr-webbundle`'s own
sources), or (b) if the legacy IDE must ship for now, fix `escapeHtml` to also escape
`"` and `'`, replace the `bodyHtml`-into-`innerHTML` paths, and drop the
wildcard-CORS `.htaccess` (scope it, or rely on same-origin). Per the project's
"one implementation, no fallbacks" rule, (a) is the direction.

**PoC.** Code-level. Not executed (legacy path).

---

### SEC-18 — Low — LSP/DAP read arbitrary paths from `file://` URIs

**Status: fixed** in the SEC-FIX pass — §9.5; the authentication `rapidr mcp`
will need, §7. (The pass went further than "no change for LSP/DAP now":
confinement is cheap, and a project's own files — `$INCLUDE`s,
`.vscode/launch.json` — are untrusted data.)

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

**Status: fixed** in the SEC-FIX pass — §9.6; signing `SHA256SUMS`, proposed in
§8 (the key is Robert's decision).

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
- The program's windows float over all of Studio (S-DEBUG, 2026-10-08): the
  frame covers the page and Studio clips it (`clip-path`, which also decides
  where the pointer lands) to the window rectangles the frame reports
  (`__rapidr_windows` over the private port). The frame only *reports*
  rectangles; Studio checks them as data (at most 256, finite numbers,
  clamped to the page) and applies them itself. The frame gains no access to
  Studio's DOM, storage or files; what it can do is draw where its windows
  are, as a program's windows can cover the IDE on the desktop. While a
  button is held in the frame (a window dragged) it is shown whole; the
  first click outside its windows after that reaches Studio again. The
  frame stacks just over Studio's main window and under Studio's dialogs
  and menus (z-index 10, after the main window in the page; Studio's
  dialogs are 11 and up; while a Studio pop-up is open the frame goes
  under it), so the program can't cover a Studio prompt or menu the user
  is answering — no clickjacking of Studio's own dialogs.
- Run in Browser (`rapidr serve`, S-DEBUG): the program's web build is
  served from memory on 127.0.0.1 only, under a random 128-bit path
  (`/<token>/`); anything else is 404, a `Host` that isn't its own loopback
  address is refused (421: DNS rebinding), only GET / HEAD are answered,
  `index.html` carries the bundle's Content-Security-Policy, every reply
  `nosniff` and `no-store`. The server ends when its standard input closes
  (Studio went away). tests/run_in_browser.mjs checks each of these.
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

---

## 7. `rapidr mcp`: the authentication it needs (design, SEC-18)

`rapidr lsp` and `rapidr dap` are stdio servers an editor starts; their only
client is that editor. `rapidr mcp` (docs/ide-ai.md §4) is different: it
relays an AI client to a *running* IDE, so the IDE must listen for it, and
anything that can reach that listener can drive the IDE (read the project,
edit, build, run). What it must have before it ships:

**Who can connect: the user, and only the user.**
- The IDE listens on a **per-user local socket, never a network port**: a Unix
  domain socket in the user's runtime directory (`$XDG_RUNTIME_DIR/rapidr/`,
  macOS `$TMPDIR/rapidr-<uid>/`), in a folder made `0700` and checked to be
  the user's own (not a link, owner = the user) before use, the socket itself
  `0600`; on Windows a named pipe with a DACL granting only the current user's
  SID, created with `FILE_FLAG_FIRST_PIPE_INSTANCE` and
  `PIPE_REJECT_REMOTE_CLIENTS`.
- On accept, the peer's identity is checked too (`getpeereid` /
  `SO_PEERCRED`; on Windows the client process's token SID): another user's
  process is closed at once.

**What it must prove: a token.**
- **256 random bits** from the OS, new each time the server starts, written
  to a `0600` file beside the socket — created with `O_CREAT | O_EXCL` at that
  mode (never chmod after), removed when the server stops. `rapidr mcp` (the
  stdio relay) reads it and sends it as the first message; the IDE compares
  it in constant time and closes a connection that doesn't send it within a
  few seconds or sends it wrong. The token is never in a command line, an
  environment variable, a log or a URL.
- The optional loopback HTTP transport (off by default) takes the same token
  as `Authorization: Bearer`, and keeps the build server's guards: bound to
  `127.0.0.1` only, `Host` must be `127.0.0.1:<port>` / `localhost:<port>`
  (DNS rebinding), any `Origin` other than none or the IDE's own refused, no
  CORS headers granted. One guard module, shared with `rapidr-buildserver`
  (SEC-01 / SEC-11), so the checks can't drift apart.
- The web IDE's bridge (`rapidr mcp --web`, experimental) pairs by a code the
  bridge shows and the user types into the page, then the same token.

**What an authenticated client may do: still not everything.**
- **Files**: every file tool resolves its path against the project's root and
  refuses anything outside it — `..`, absolute paths, and links that leave
  it, checked on the real path (`rapidr_preprocessor::is_within`, the check
  the language server uses since this pass) — and `$INCLUDE`s are analysed
  with the same confinement (`PreprocessOptions::confine_to`).
- **Environment**: a client never sets the environment of anything the IDE
  runs (contrast DAP's `env`, now allow-listed: §9.5); a run gets the
  project's own settings only.
- **Tiers**: the per-client permission tiers of docs/ide-ai.md §5 (read auto
  / edit with checkpoint / run sandboxed with no network / anything else
  asks), shown live in the MCP panel, with an audit log of every call.
- **No native power**: an MCP-initiated run never gets `DECLARE … LIB`,
  `RUSTSTART` or (later) VARPTR / PEEK / POKE (SEC-10, SEC-19).
- **Limits**: request size, results truncated with a cursor, a rate limit per
  client.

Tests to land with it: a connection without the token, with a wrong one, and
from another user is closed; the token file's mode and `O_EXCL`; `Host` /
`Origin` probes on the HTTP transport; a file tool asked for `../x`, an
absolute path and a link out of the project; a client-supplied environment
ignored.

---

## 8. Signing `SHA256SUMS` (proposal, SEC-20 — the key is Robert's decision)

Today `tools/release/finish.sh` writes `SHA256SUMS` beside the packages: it
proves a download matches the list, not that the list is RapidR's. Proposal:
sign the list with **minisign**.

- **Why minisign**: one small Ed25519 signature file (`SHA256SUMS.minisig`),
  a public key that fits on one line, a trusted comment (the version) inside
  the signature, and one tool to run. Licences, all permissive: the
  reference tool `minisign` (Frank Denis) is **ISC**; the Rust ports
  (`rsign2`, the `minisign` crate) are **MIT**; `minisign-verify` (verifying
  only, no dependencies) is **MIT**. OpenBSD's `signify` is the same idea
  (ISC). Nothing is copyleft, and nothing enters a user's program.
- **Not chosen**: GnuPG (a GPL tool — only run, never linked, but heavy, and
  keyring handling is easy to get wrong); Sigstore `cosign` (Apache-2.0)
  keyless signing ties each release to an online identity provider and a
  public transparency log — at odds with releases made locally, by hand.
- **How it would work** (nothing done yet, no key made):
  1. Robert makes the key once, offline: `minisign -G -p rapidr.pub -s
     <private key file>`, with a passphrase. The private key never enters the
     repository, a CI system or a VM; two offline backups.
  2. The public key is published where a download isn't: the repository
     (`SECURITY.md`, `rapidr.pub`), rapidr.dev, the release notes.
  3. `finish.sh`, after writing `SHA256SUMS`: `minisign -S -m SHA256SUMS -t
     "RapidR <version>"` (it asks for the passphrase) → `SHA256SUMS.minisig`,
     uploaded with the release.
  4. Users: `minisign -V -m SHA256SUMS -P <public key>`, then
     `shasum -a 256 -c SHA256SUMS`.
  5. Later, if `rapidr setup` or an updater downloads anything of RapidR's,
     it verifies with `minisign-verify` and the public key built in.
- **Robert decides**: whether to sign at all; who holds the key and its
  backups; the passphrase; what happens if the key is lost or leaks (a new
  key announced on rapidr.dev and in the repository, the old one listed as
  revoked).

---

## 9. The fix pass (SEC-FIX, 2026-10-08)

Every fix has a regression test under `tests/security/`, run by
`tools/regress.sh security` (a stage of its own, in the default run); each was
seen to fail without the fix and pass with it. The browser's half runs in the
web stage.

| Test | Covers | Without the fix |
|---|---|---|
| `tests/security/web_bundle_injection.rs` (built into rapidr-webbundle's tests) | SEC-15: a policy on every page; SEC-16: hostile project and asset names; SEC-17: nothing of the IDE in a bundle | there was neither a policy nor escaping (`render_index_html` interpolated names as they were) |
| `tests/web_bundle_csp.mjs` (browser, web stage) | a real bundle under its policy: RDOM markup's `<img onerror>` stopped; RWEBVIEW's script runs in an opaque frame and can't reach the page; RJAVASCRIPT's Eval works; a file name made of markup is text; a plain program runs with no violation | 11 checks fail with the pre-fix `rapidr`: `window.__pwned = 1`, and the hostile name breaks `loader.js` ("missing ) after argument list") |
| `tests/web_overlays.mjs` (updated) | RWEBVIEW sandboxed without `allow-same-origin`, its page at origin `null` | the old frame was same-origin (its `contentDocument` was readable) |
| `tests/security/web_shipping.mjs` | SEC-17: what ships; SEC-12: RWEBVIEW's defaults | `web.sh` defaulted to the legacy IDE; `include_str!` of its files; `allow-same-origin` in the default sandbox |
| `tests/security/lsp_workspace_confinement.rs` (rapidr-lsp's tests) | SEC-18: a never-opened file outside the workspace, a `..` URI and an `$INCLUDE` outside read nothing; the workspace's own files still work | the outside file's outline (`LeakedSecretName`) and the outside include's SUB came back |
| `tests/security/dap_launch_confinement.rs` (rapidr-dap's tests) | SEC-18: loader variables never reach the debuggee; a launch names only a RapidR program | `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` passed; a key file accepted as `program` |
| `tests/security/supply_chain.mjs` | SEC-20: versions, deny.toml, the ttf-parser ground, `cargo deny` / `cargo audit` clean | memmap2 0.9.10 (and `cargo deny` silent about it), the stale ignore |

### 9.1 SEC-15 — a policy on every page (and SEC-12)

- `rapidr-webbundle` makes each page's **Content-Security-Policy** from what
  the program uses (`csp.rs`: `WebNeeds::scan` of the preprocessed source —
  the component types it names and the URLs it spells out). A plain program
  gets `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src
  'self' 'unsafe-inline'; img-src / media-src / font-src 'self' data: blob:;
  connect-src 'self' data: blob:; frame-src 'none'; worker-src 'self' blob:;
  manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action
  'self'`. Inline styles stay allowed (the kernel's host and `RDOM.CssStyle`
  set them; a style runs no code). The bundle pages (`rapidr bundle-bc`,
  `build --web --interp`) and the native `rapidr build --web` page both have
  it, and both builds print the policy they made.
- No page has an inline script any more: `loader.js` (bundles) and `start.js`
  (native builds) are fixed files; the program's name is a `<meta>` they
  read; the project's files are `rapidr-assets.js`.
- Each build also writes `_headers` (Netlify, Cloudflare Pages) and
  `.htaccess` (Apache): the same policy plus `frame-ancestors 'self'`, on
  `index.html` only, and on every file `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: strict-origin-when-cross-origin`,
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Resource-Policy: same-origin`. No COEP: the runtime uses no
  SharedArrayBuffer, and `require-corp` would stop a program showing
  pictures and media from other sites. No CORS. On nginx, the same headers as
  `add_header` lines (the policy in `location = /index.html`).
- **RWEBVIEW (SEC-12)**: its frame's default sandbox is now `allow-scripts
  allow-forms allow-popups allow-modals allow-downloads` — no
  `allow-same-origin`, so the page it shows runs at an opaque origin, away
  from the program's page, storage and cookies. Its **Html** no longer goes
  into an `srcdoc` (which would inherit the page's policy: the Html's own
  scripts would stop, or the page's policy would have to be weakened): it
  runs in `rapidr-webview.html`, a small file every web build ships beside
  the page, which takes the HTML once, only from its parent window, and
  writes it as its document — its scripts run under that frame's rules, not
  the page's. With scripts not allowed (a `Sandbox` without
  `allow-scripts`) the Html is an `srcdoc` (markup only). `Url` / `Navigate`
  are as before; `Html` and `Url` read back what the program set.
- RapidQ programs see no change: RWEBVIEW, RDOM and RJAVASCRIPT are RapidR's
  own web components; no RapidQ component is affected.

### 9.2 What a web program can opt into, and how

| To… | Do this |
|---|---|
| let RWEBVIEW's page use the program's origin (its storage, its DOM) | `Web.Sandbox = "allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-downloads"` before setting `Html` / `Url` — that page then *is* the program, as far as the browser is concerned |
| show an RWEBVIEW page with no scripts at all | `Web.Sandbox = ""` (or any list without `allow-scripts`) |
| call a server whose address the program builds at run time, or an `http://` / `ws://` one | `rapidr build --web --csp "connect-src http://192.168.1.10:8080"` (also `rapidr bundle-bc … --csp …`, `build --web --interp`) |
| load a script from a CDN through RJAVASCRIPT | `--csp "script-src https://cdn.example.com"` |
| show an `http://` site in RWEBVIEW | `--csp "frame-src http://intranet.local"` |
| anything else | `--csp` takes any directive the policy has (`default-src script-src style-src img-src font-src media-src connect-src frame-src worker-src manifest-src object-src base-uri form-action`) and source expressions; an unknown directive, or a source with `;`, `,`, quotes or markup in it, is an error, never dropped silently. The built `index.html` (and `_headers` / `.htaccess`) are the author's to edit, too |

Allowed without asking, by component: RJAVASCRIPT → `'unsafe-eval'`;
RWEBVIEW → `frame-src 'self' https:`; RHTTP / RSOCKET / RDOWNLOAD (Q or R) →
`connect-src https: wss:`; RDOM / RWEBVIEW → `img-src https:`; RWEBAUDIO /
RWEBVIDEO / RVIDEO → `media-src https:`; an `http(s)://` or `ws(s)://`
origin written in the program → that origin. A page never allows inline
scripts unless its author asks with `--csp`.

### 9.3 SEC-16 — names, escaped for where they go

`rapidr-webbundle::escape`: `html` (text and quoted attributes: `& < > " '`),
`js_string` (a JSON string that stays one inside `<script>`: `<`, `>`, `&`,
`'`, U+2028 / U+2029 and control characters escaped), `url_component`,
`file_name` (the `.rrbc`'s name in the bundle: no separator, no leading dot,
never `..`). The title is HTML text; the program's file name an attribute;
asset names are only in `rapidr-assets.js`, as string literals; no name is
written into a script. Font file names in a bundle go through `file_name`
too.

### 9.4 SEC-17 — what ships

- `tools/release/web.sh` ships **RapidR Studio's web build**
  (`tools/build_studio_web.sh` → `target/studio-web`, with the RWEBVIEW frame,
  the page's icons and Studio's own `_headers` / `.htaccess`: no CORS,
  `frame-ancestors 'self'` on `index.html`). The SBOM doesn't list Monaco.
- Program bundles carry only `rapidr-webbundle`'s own files (`web/`:
  `bundle_console.js`, `ansi_screen.js`, `loader.js`, `start.js`,
  `rapidr-webview.html`); the crate embeds nothing from outside its `web/`
  folder (`tests/security/web_shipping.mjs`, `web_bundle_injection.rs`).
- **The legacy HTML / Monaco IDE is deleted** (2026-10-08, Robert's decision:
  RapidR Studio is the web IDE), with its wildcard-CORS `.htaccess`, its
  `escapeHtml`, its vendored Monaco, `rapidr lang export`'s generator for its
  `lang-data.js`, the brand export's copy of the icons into it, and
  `tools/lang_seed.py`'s reading of its data. What the program's frame needed
  from the IDE's page is now Studio's (`ide/web/studio.js`): the browser's file
  pickers shown for the opaque-origin frame (`frameFiles`: only the options a
  program's Filter / FileName make; writes only through a token the user's
  pick gave) and the program's RWEBSTORAGE kept per program under its own key,
  capped at 1 MB (`applyAppStorageOp`). Its browser suites were sorted
  (docs/studio-wow.md §7): the web runtime's re-pointed at the runtime's page
  (`tests/web_run.mjs`) or Studio's run frame (`tests/studio_run_frame.mjs`:
  SEC-02 / 03's isolation, storage, 1:1 pixels), the IDE features' listed for
  Studio, the old IDE's internals' deleted.

### 9.5 SEC-18 — LSP and DAP confined

- `rapidr lsp` reads from the disk only inside the **workspace folders the
  client opened** (`workspaceFolders`, else `rootUri` / `rootPath`, kept
  current by `workspace/didChangeWorkspaceFolders`) and the **folders of the
  documents it opened**, plus the include folders it is given
  (`initializationOptions.includeDirs`, `RAPIDR_INCLUDE_PATH`). A request
  about any other file answers nothing; an `$INCLUDE` resolving elsewhere is
  the diagnostic "Include file outside the workspace". The check is on the
  real path (links and `..` resolved: `rapidr_preprocessor::is_within`).
  Builds and `rapidr run` are unchanged (RapidQ's include rules: any file the
  program names); RapidR Studio's analysis isn't confined (it opens the
  project's own files).
- `rapidr dap`: a launch's `program` must be a `.bas`, `.rr` or `.rrbc` file;
  its `env` reaches the program only for plain variable names that aren't the
  loaders' (`LD_*`, `DYLD_*`, `_RLD*`, `LIBPATH`, `SHLIB_PATH`, `GCONV_PATH`,
  `LOCPATH`, `PATH`, `PATHEXT`, `COMSPEC`, `__COMPAT_LAYER`); refused names are
  said in the debug console.

### 9.6 SEC-20 — supply chain

- `memmap2` 0.9.10 → **0.9.11** (RUSTSEC-2026-0186); `anyhow` → 1.0.104
  (RUSTSEC-2026-0190).
- `deny.toml`: `unsound = "all"` (without it cargo-deny didn't report
  memmap2's unsoundness — checked: with it, the old lockfile fails); the
  stale RUSTSEC-2026-0173 ignore is gone. `.cargo/audit.toml` ignores what
  deny.toml ignores, for the same reason.
- `ttf-parser` (RUSTSEC-2026-0192: unmaintained, no vulnerability) **does not
  affect us** as a security matter: it only ever parses fonts built into
  RapidR (`rapidr-value`'s `face_data`, `include_bytes!`) and, on Linux
  Wayland, the system's own fonts for winit's title bars; no program's or
  project's font reaches it. Kept and ignored with that reason, pinned by
  `tests/security/supply_chain.mjs` (any other use of `ttf_parser` fails it).
  The way off is skrifa (already in the tree); it can't leave completely
  until winit's `sctk-adwaita` does.
- `cargo deny check`: advisories, bans, licences and sources ok.
  `cargo audit`: no warnings. `THIRD_PARTY_NOTICES.md` regenerated.
