# RapidR threat model (Preview 4, X1, 2026-10-22)

The threat model of `docs/security-audit.md` §1 (2026-10-08) still holds:
its assets, attackers and trust boundaries are repeated here in short, and
the surfaces that arrived since 2.120.0 are added. The audit of those
surfaces is `docs/security/audit-preview4.md`.

## 1. Assets

- The developer's machine: files, processes, keychain, network position.
- The developer's source and projects.
- **The developer's AI keys** (RapidR Studio's, in the OS keychain under
  `dev.rapidr.keys`, names `RapidR Studio/<provider>`) and the chats kept
  with the assistant (Studio's data folder; the browser's storage on the
  web).
- **A program's own secrets** (RSecrets: `rapidr-app:<program>:<service>`
  in the same keychain) and its database passwords.
- A program's data at run time: what it serves (RHttpServer), what it
  reads from a database, what its users type.
- The integrity of RapidR Studio: its state, what it writes to the project,
  what it applies from the assistant.

## 2. Attackers

1. **A malicious program run by the developer** (a `.rr`/`.bas` opened,
   run or built — an example, a download, a project a model wrote). It may
   use every documented component. New since 2.120.0: it may name
   Studio's keychain entries (RAI.APIKeySource), another program's secrets
   (RSecrets), and send what it gets anywhere (RAI.BaseURL, RHTTP).
2. **A malicious project opened in Studio** without being run: crafted
   file names, symlinks inside the folder, text that reads as instructions
   to the assistant (prompt injection), a crafted `.rrproj`.
3. **The model** (or its provider): its answers are rendered in Studio,
   its tool calls name paths and texts, its markdown may carry links.
   The model is never trusted with more than the user gave it.
4. **A web page of another site** reaching a program's local HTTP server
   (DNS rebinding, cross-site requests), or a host on the LAN when a
   program listens on every interface.
5. **The network**: a server a program talks to (an AI provider, a
   MySQL server, a redirect), DNS, an impostor server.
6. **Another process of the same user**: it can set the environment of
   a RapidR process it launches (test hooks), talk to a program's local
   server, or read files written with default permissions.
7. **The supply chain** of RapidR's dependencies and of its builds.

## 3. Trust boundaries

| Boundary | Where it is enforced | Notes |
|---|---|---|
| Studio ⇄ the model | `crates/rapidr-studio/src/ai_tools.rs` (`project_path`, `run`), `crates/rapidr-ai/src/rai.rs` | Tools read only inside the project (lexically); every change is a proposal the user applies; tool results are marked untrusted; file text is key-redacted before it goes to the model |
| Studio ⇄ the project's files | the same, plus `crates/rapidr-studio/src/lib.rs` (`read_text`, `write_text`) | Paths are checked as text; symlinks are followed (finding SEC-23) |
| A program ⇄ the developer's keychain | `crates/rapidr-ai/src/keys.rs` (`resolve`, `get_named`), `crates/rapidr-ai/src/secrets.rs` | One keychain service for Studio and programs' RAI; RSecrets namespaced by the program's file name (findings SEC-21, SEC-22) |
| A program ⇄ the network it serves | `crates/rapidr-runtime-core/src/httpserver.rs`, `netframe.rs` | Host / Origin / Sec-Fetch-Site / token checks, limits, 127.0.0.1 by default; RServerSocket's `Mode = "http"` has none (by design, documented) |
| A program ⇄ its database server | `crates/rapidr-db/src/mysql.rs` | TLS as MySQL's client: the certificate is checked only by VerifyCA / VerifyFull (SEC-25) |
| A RapidR process ⇄ its environment | `crates/rapidr-ui-app/src/testhooks.rs` and the hooks' readers | The test hooks are part of shipped binaries and trust the environment (SEC-24) |
| Studio (web) ⇄ the page's storage | `chats.rs`, `keys.rs`, `audit.rs` (wasm) | Chats, the audit log and a remembered key live in `localStorage` of Studio's origin: the strict CSP is what protects them |
| The debugger ⇄ the program | `crates/rapidr-session/src/program.rs` (`set_value`, `run`), `crates/rapidr-dap/src/adapter.rs` | The debugger runs statements by design (Immediate); the DAP is stdio, spawned by the editor |
| Versions (rapidr-vcs) | not linked by any shipped crate | Nothing of it is reachable from Preview 4's UI |

## 4. New surfaces since 2.120.0, and what each one trusts

- **RAI's kept chats** (`crates/rapidr-ai/src/chats.rs`): files named from
  the project key (letters, digits, `-`, `_` and a digest) and a safe id;
  redacted before writing. Trusts: the user's own settings folder.
- **RAI's tools** (`ai_tools.rs`): `list_files`, `read_file`,
  `rapidr_describe`, `rapidr_list`/`rapidr_get`, `check_code`,
  `find_example`, `project_outline`, `search_manual`, `layout`,
  `propose_edit`, `write_file`, `create_project`; the lite set is a subset
  with the same code behind it. Trusts: the project's folder as given,
  the user's review of proposals, the projects folder for a new program.
- **RSecrets** and **MySQL TLS** (`crates/rapidr-ai/src/secrets.rs`,
  `crates/rapidr-db/src/mysql.rs`).
- **RHttpServer**, **RServerSocket modes and BindAddress**
  (`httpserver.rs`, `netframe.rs`, `network.rs`).
- **Studio**: Search and Replace All (`crates/rapidr-editor/src/search.rs`,
  `ide/search.inc`), Find References / Go to Definition (language service,
  no text evaluation), Problems and the project's `[build] runs_on`
  (`crates/rapidr-project/src/project.rs`), S11 set-value-while-paused
  (`program.rs`, `adapter.rs`), the helpers' value editors (edits to the
  editor's text, saved by the user), Open Recent (`ide/project.inc`),
  forms from code (parsing only), the web Studio's file opening
  (`ide/project.inc` `ProjectFilesElsewhere`, `ide/documents.inc`).
- **Test hooks** (`testhooks.rs`; on the web `kernel_web.rs`
  `set_test_env`, called only by `ide/web/studio.js` from
  `window.RAPIDR_STUDIO_TEST` and by the test page).
- **Builds**: `zip` 9 (writing only: `crates/rapidr-buildserver/src/main.rs`,
  the web bundle), the generated Rust of native builds
  (`crates/rapidr-codegen-rust/src/lib.rs`).

## 5. By design (not findings)

As in the 2026-10-08 audit: a program does what its author wrote
(RJavaScript, RDOM, RWebView, SHELL, DECLARE…LIB, RUSTSTART…RUSTEND, a
native build compiling author code). The debugger's Immediate window and
the set-value editors run the program's own code at the user's request.
`RServerSocket`'s `Mode = "http"` is a raw framing for programs that
check requests themselves; `RHttpServer` is the checked one.
