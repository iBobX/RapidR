# RapidR security audit, Preview 4 (X3, 2026-10-22)

Audit of `development` at `a01e449b` (the lanes merged since 2.120.0), by
reading the code of every new or changed surface listed in the threat
model (`docs/security/threat-model.md`). Nothing here was run against any
host, network or keychain; the proofs are code paths, each named with its
file and line. No fix was made in this pass (the fix pass follows).
Previous audit: `docs/security-audit.md` (its section 10 holds the
automated checks of the same day: advisories, fuzzing, the secret scan).

IDs continue the `SEC-` series (last used SEC-20).

## 1. Findings, ranked

| ID | Severity | Area | One line |
|---|---|---|---|
| SEC-21 | Medium | Keys | A program can name Studio's keychain entry and send the developer's AI key to a server it picks |
| SEC-22 | Medium | RSecrets | Secrets are keyed by the program's file name, so two programs of the same name share them |
| SEC-23 | Low | RAI tools | `read_file` and proposals follow symlinks inside the project, reaching files outside it |
| SEC-24 | Low | Test hooks | The GUI test hooks ship in the runtimes and Studio and trust the process environment |
| SEC-25 | Low | MySQL | `SSLMode` Preferred (default) and Required do not check the server certificate |
| SEC-26 | Low | Studio chat | A model's markdown can carry Studio's own `ai:`/`rai:` links |
| SEC-27 | Info | RAI transport | `x-api-key` is kept across HTTP redirects |
| SEC-28 | Info | Chats, logs | Chats and the audit log use default file permissions; the web keeps them in `localStorage` |
| SEC-29 | Info | Debugger | Set-value runs the typed text (by design); `setVariable` is gated indirectly, not by an explicit "stopped" check |
| SEC-30 | Info | Sockets, docs | `Mode = "http"` checks nothing and `BindAddress` defaults to every interface |

Nothing is Critical or High. **By the Critical/High rule, nothing blocks
the release.** SEC-21 and SEC-22 are the two worth fixing before shipping:
both are a one-line or small change, and SEC-21 is what a hostile example
would reach for first.

---

### SEC-21 — Medium — A program can read Studio's AI key from the keychain

**Where.** `crates/rapidr-ai/src/keys.rs:26` (one service,
`dev.rapidr.keys`), `:158-196` (`resolve`, `get_named`). Studio stores a
provider's key under the name `RapidR Studio/<provider>`
(`ide/ai.inc:1054`, `AIKeySourceOf`). A program sets
`RAI.APIKeySource = "keychain:RapidR Studio/anthropic"` and the same
`get_named` returns that key; the program may then point
`RAI.BaseURL` at any server and the key is sent there (`rai.rs:endpoint`,
`provider::default_base`).

**Reasoning.** Keys are namespaced only by the keychain *account name*,
not by who is asking. There is no separation between Studio's own service
and a program's RAI. A program that guesses or reads the convention
`RapidR Studio/<provider>` gets the developer's paid key. RSecrets took
the opposite, safe path (a per-program service prefix, SEC-22 aside), so
the asymmetry is clear in the same codebase.

**Impact.** A downloaded or model-written example that the developer runs
once can exfiltrate their Anthropic/OpenAI/DeepSeek key. No prompt, no
network warning. Keys are the asset the project calls out as most
sensitive.

**Recommended fix.** Keep programs' RAI keys under a program-scoped
service (as RSecrets does), separate from `dev.rapidr.keys`, so
`keychain:NAME` from a program cannot resolve Studio's entry. If a shared
name must stay for compatibility, require an explicit opt-in prompt the
first time a program reads a key it did not write.

**Fixed in** `6e91d999` (lane SEC-FIX, 2026-10-23). `keys.rs`: a `Scope` — Studio's keys
under `dev.rapidr.keys` (unchanged: nothing moved or rewritten), a
program's under its own service `rapidr-app:<program>:RAI`
(`Scope::service`); only the process Studio runs in is Studio
(`keys::set_studio`, Rust only: `rapidr ide`, `rapidr ai`, and `rapidr run`
of the executable's own `ide/studio.rr` — never by `RAPIDR_HOME` or the
current folder, `rapidr-cli/src/home.rs` `is_own_ide`). A program naming
`keychain:RapidR Studio/…` finds nothing and its `Error` says to give the
program its own key (`studio_key_note`). The designer keeps a key typed for
a program in that program's service (`RAI.KeyOwner`, honoured in Studio
only; `ide/ai.inc` `DesignAITake`). Test: `tests/security/keychain_scopes.rs`
(`sec21_*`, memory keystore). Residual: a program that runs commands
(`SHELL`) can still start `rapidr ai … --base-url` (Studio's CLI) — SHELL is
by design (threat model §5).

### SEC-22 — Medium — RSecrets are keyed by the program's file name

**Where.** `crates/rapidr-ai/src/secrets.rs:52-65` (`program_name`,
`keychain_service`): the service is `rapidr-app:<program>:<service>`,
`<program>` the file's stem in lower case. `dbstudio.rr`,
`DBStudio.exe` and `DBStudio.app` all map to `dbstudio` (tests at
`:296-304`).

**Reasoning.** Any program whose file is named `dbstudio` reads and
overwrites `dbstudio`'s secrets. There is no path, signature or publisher
in the key — only the base name. A user who saves a second program as
`dbstudio.rr` in another folder shares the first's stored passwords; a
hostile program named to match a known app harvests its secrets.

**Impact.** Cross-program secret disclosure, silent, within one user
account. Lower than SEC-21 because it needs the attacker to know and match
a victim program's name, and the keychain itself still gates by OS user.

**Recommended fix.** Fold a stable project/path identity into the service
name (as the chats' `project_key` does with a path digest,
`chats.rs:195`), or document plainly that RSecrets isolation is by file
name only and is not a trust boundary between programs.

**Fixed in** `6e91d999` (lane SEC-FIX, 2026-10-23). `secrets.rs` `program_id`: a built
macOS app's bundle identifier, else the file's name and a 64-bit digest of
its canonical path (`dbstudio~1a2b3c4d5e6f7081`), used by RSecrets and by
programs' RAI keys. Items kept under the name alone are **not** read: they
carry nothing that says which program saved them, so a one-time move could
hand them to whichever program of that name ran first; RSecrets was never
in a release, so only development machines have such items. The manual
(`docs/manual/secrets.md`) says that this keeps programs apart within one
account and is not a wall against a program that rewrites another's file.
Test: `tests/security/keychain_scopes.rs` (`sec22_*`).

### SEC-23 — Low — RAI tools follow symlinks inside the project

**Where.** `crates/rapidr-studio/src/ai_tools.rs:243-273`
(`project_path`) resolves `..` and leading slashes **as text**, then
`:276-289` (`Project::full`, `text`) joins the result to the folder and
reads it through the normal file hook. `crate::join`
(`crates/rapidr-studio/src/lib.rs:177`) does no canonicalisation.

**Reasoning.** `project_path` correctly refuses `../x`, absolute paths and
`C:/…`. It does not resolve symlinks: a project that contains
`notes -> /Users/me/.ssh` lets `read_file notes/id_rsa` read outside the
folder, and an applied `write_file notes/x` writes outside it. The write
needs the user to accept the diff, so it is a review-bypass, not silent.

**Impact.** A crafted project (opened, not even run) widens the model's
read reach to any file the user can read, through a link the user may not
notice. Reads go to the model; redaction (`keys::redact`,
`ai_tools.rs:482`) removes key-shaped text but not other secrets.

**Recommended fix.** After joining, canonicalise and confirm the real path
is still inside the project folder (reject when it is not), on both
`read_file` and the proposal path. The symlink check belongs next to the
textual `..` check, not instead of it.

**Fixed in** `2ce6d8a8` (lane SEC-FIX, 2026-10-23). `ai_tools.rs` `project_path` (the
gate of `read_file`, the file list, `propose_edit`, `write_file` and
`setfile`) now also resolves the path on disk (`inside_for_real`): the
deepest existing part, canonicalised, must be inside the canonical project
folder, and a link to nowhere is refused. Test:
`tests/security/ai_tools_symlinks.rs`.

### SEC-24 — Low — Test hooks ship in release binaries and trust the environment

**Where.** `crates/rapidr-ui-app/src/testhooks.rs` (no `#[cfg(test)]` or
feature gate: `under_test()` at `:88` reads `RAPIDR_CAPTURE` /
`RAPIDR_TEST_EVENTS`; `print_dump` at `:328` prints any
`component.property`; `system_look` at `:445` reads a file named by
`RAPIDR_TEST_SYSTEM_LOOK=file:PATH`). On the web the hooks are off unless
`window.RAPIDR_STUDIO_TEST` is set (`ide/web/studio.js:35`,
`kernel_web.rs:834`), so a plain deployed program is not affected.

**Reasoning.** On the desktop these are compiled into shipped runtimes and
Studio. A process that launches a RapidR program controls its environment,
so it can drive the program (`RAPIDR_TEST_EVENTS`), dump field values to
stdout (`RAPIDR_TEST_DUMP`), set file dialog and message-box answers, and
make it screenshot itself (`RAPIDR_CAPTURE`). It cannot read anything the
launcher could not already read by other means (a child process it
started), so this is a hardening note, not an escalation: there is no
cross-user or network reach, and the hooks never write a key.

**Impact.** A malicious launcher can automate a RapidR GUI program
headlessly and read what it shows. Within the same user account this adds
little over launching the program yourself, but the capture-to-disk and
field-dump behaviours are more than a shipped app should expose.

**Recommended fix.** Gate the capture/dump/event hooks behind a build
feature (on for test and CI binaries, off for release runtimes and the
released Studio), or behind a single `RAPIDR_TEST_OK` guard the release
build leaves unset. Confirm `file:PATH` in `RAPIDR_TEST_SYSTEM_LOOK` reads
only under test.

**Accepted for Preview 4** (Robert's lead, 2026-10-23). We test the binary
we ship, so the hooks stay in it for this release. Whoever can set a
process's environment already controls that process (it can run the program
itself, read what it shows, and read the user's files), so the hooks add no
reach across users or the network. Revisit in Preview 5 with a signed
test-mode design (ROADMAP, Preview 5).

### SEC-25 — Low — MySQL Preferred/Required do not verify the certificate

**Where.** `crates/rapidr-db/src/mysql.rs:444-453` (`SslMode::opts`):
`Preferred` and `Required` use
`with_danger_accept_invalid_certs(true).with_danger_skip_domain_validation(true)`.
Default is Preferred (`:259`, `:469`); RapidQ's `CLIENT_SSL` flag raises
Preferred to Required (`:266`), still unverified.

**Reasoning.** This matches MySQL's own client (documented at
`mysql.rs:27-33`): only `VerifyCA` and `VerifyFull` check the chain. So a
connection the user believes is "encrypted" is open to an active
man-in-the-middle, who gets the password and the data. It is a Low because
the behaviour is intended, documented, and equal to the reference client;
the risk is that a user reads `Required` as "safe".

**Impact.** On a hostile or spoofed network, an attacker between the
program and the MySQL server reads the credentials and the query traffic
even with TLS "on".

**Recommended fix.** No code change required for compatibility; make the
manual and the registry `SSLMode` doc state in one line that only VerifyCA
/ VerifyFull authenticate the server, and consider surfacing `Encrypted`
alongside a "verified" flag so a program can tell the difference.

**Fixed in** `77948a74` (lane SEC-FIX, 2026-10-23), documentation: the registry's
`SSLMode` (`databases.toml`) and the manual (`docs/manual/databases.md`)
say that only VerifyCA / VerifyFull check the server, and the manual's
example for a remote host uses `VerifyFull`. No example or Studio code sets a
weaker mode for a remote host. Behaviour unchanged (MySQL's client's).

### SEC-26 — Low — Model markdown can carry Studio's own action links

**Where.** The assistant's answer is rendered as markdown in `AIChat`
(an RMarkdownView) with `OnLinkClick = AIChatLink` (`ide/ai.inc:600`).
`AIChatLink` (`ide/ai.inc:2309-2341`) acts on `ai:open`, `ai:undo`,
`ai:review`, `rai:edit`, `rai:retry`, `rai:thought`. The model's text is
not scrubbed of these schemes before rendering.

**Reasoning.** A model (or a prompt-injected project telling the model
what to write) can put `[click here](ai:undo/MyApp)` or
`[ok](rai:retry)` in its reply. `AIChatLink` guards the destructive ones:
`ai:open`/`ai:undo` only act on the assistant's *last* created folder and
check the name matches (`:2331-2339`), and `ai:review` only opens the
review panel. So the blast radius is small — at worst an unwanted Undo of
the program just created, or a spurious Retry — and nothing is deleted
outside that folder. Hence Low, not Medium.

**Impact.** A single click on a model-planted link performs a Studio UI
action the user did not intend (most harmfully, undoing the new program).
It cannot reach arbitrary files or settings.

**Recommended fix.** In the chat renderer, treat `ai:` and `rai:` as
Studio-internal: ignore them when they appear in model-authored markdown
(only Studio's own cards use them), or confirm before an `ai:undo` that
came from the transcript body rather than a card.

**Fixed in** `97ef5e23` (lane SEC-FIX, 2026-10-23). Studio's and RAI's own action links
carry a mark only the process knows while the transcript is put together
(`rai.rs` `own_link`; the cards in `rapidr-studio/src/ai.rs`); the whole
transcript is then read as the view reads it (pulldown-cmark,
`rapidr_value::panels::markdown::unlink`) and every other `ai:`/`rai:` link
or reference definition becomes plain words — written plainly, as an
autolink, a reference, with entities, or across answers (a code fence left
open in an earlier one). The marks are taken off before anyone sees the
transcript and never kept in a saved chat. Test:
`tests/security/keychain_scopes.rs` (`sec26_*`).

### SEC-27 — Info — `x-api-key` kept across redirects

**Where.** `crates/rapidr-ai/src/http.rs` uses `ureq` 2.12.1. ureq strips
`Authorization` and `Cookie` on cross-host redirects
(`ureq .../unit.rs:220-224`, default `RedirectAuthHeaders::Never`) but not
`x-api-key`, which is how Anthropic-style keys are sent (`wire.rs`).

**Reasoning.** If an AI endpoint (or a misconfigured BaseURL) 30x-redirects
to another host, the API key header rides along. For the default providers
this does not happen; it matters when a user sets a custom BaseURL that
redirects. ureq follows up to its redirect limit by default.

**Recommended fix.** Set ureq to not follow redirects for provider calls
(the APIs do not need them), or strip `x-api-key` before following one.

**Fixed in** `b0546c98` (lane SEC-FIX, 2026-10-23). `http.rs`: provider calls follow no
redirect (`redirects(0)`); a 3xx is an error in plain words
(`wire::explain_error`). Test: `tests/security/ai_transport.rs` (`sec27_*`,
two servers on 127.0.0.1).

### SEC-28 — Info — Chats and the audit log: permissions and web storage

**Where.** `crates/rapidr-ai/src/chats.rs` and `audit.rs` write under the
user's settings folder with default permissions (no explicit mode).
Redaction before writing is thorough (`chats.rs:76-164`, tested
`:466-482`): provider key shapes, `Bearer`/`x-api-key`/`api_key=` labels,
and each known key value are replaced. On the web, chats
(`chats.rs:420-451`), the audit log (`audit.rs:113-126`) and a remembered
key (`keys.rs:531-562`) are in `localStorage` of Studio's origin.

**Reasoning.** No secret is written by design, so this is informational.
The residual exposure is: another process of the same user can read the
chat files and the audit log (they hold project text and digests, not
keys); and on the web the strict CSP is the only thing standing between a
remembered key and a script on the page. The audit log never carries file
contents or keys (`audit.rs:5-7`, confirmed: tool entries log a digest and
the target, `rai.rs:1961`).

**Recommended fix.** None required. Optionally restrict the settings
files to the user (0600) and keep confirming the web CSP in
`tests/security/web_shipping.mjs`.

**Fixed in** `b0546c98` (lane SEC-FIX, 2026-10-23). `private_files.rs`: the chats'
files 0600 and folders 0700 (the chats' own folders made so even when they
were there), the audit log 0600 (an older one made so when written), on
Unix. Test: `tests/security/ai_transport.rs` (`sec28_*`).

### SEC-29 — Info — Set-value runs the typed text (by design); `setVariable`'s stopped-guard is indirect

**Where.** `crates/rapidr-session/src/program.rs:524-569` (`set_value`):
after its checks it runs `"{target} = ({text})"` through the VM
(`run`, `:591`). `crates/rapidr-dap/src/adapter.rs:364-373`: `setVariable`
does not test `self.stopped`, unlike `evaluate` (`:349`) and
`setExpression` (`:381`).

**Reasoning.** Running the user's typed expression is the intended
behaviour of a set-value / Immediate box — the same power the debugger
already gives — so splicing it into a statement is not an injection in the
threat sense (the author is the user). `check_value_name` (`:575`) blocks
a bare unknown word; for scalars the final statement is built from
`fit_value`/`literal` (`:564-566`), so the splice is bounded. On the
"stopped" question, `setVariable` is in fact gated *indirectly*: it needs
a `variables_reference` from `self.refs`, and `self.refs` is cleared on
every resume (`adapter.rs:418-419`, `:646`; and on each new stop,
`:515-516`), so after the program runs on, the reference is gone and the
command fails with "that variable is gone" (`:368-370`). The residual is
only that the guard is a side effect of ref lifetime rather than an
explicit check, so it is easy to break in a refactor.

**Recommended fix.** Add an explicit `self.stopped` check to
`setVariable`, matching `setExpression`, for defence in depth. No change
to the evaluate-by-design behaviour.

**Fixed in** `130cbbb3` (lane SEC-FIX, 2026-10-23). `adapter.rs` `setVariable`: "the
program isn't stopped" before the reference is looked up. Test:
`tests/security/dap_set_variable_stopped.rs`.

### SEC-30 — Info — `Mode = "http"` and `BindAddress` defaults

**Where.** `crates/rapidr-runtime-core/src/netframe.rs:27-33`
(`Framing::by_name`): `Mode = "http"` on RServerSocket does framing only —
no Host, Origin, token or size-equivalent checks (those live in
`httpserver.rs`, the checked server). `BindAddress` defaults to `0.0.0.0`
(`network.rs:244-252`, `network.toml:22`).

**Reasoning.** This is by design and documented for the raw socket
(`docs/manual/network.md:63`: "`Mode = "http"` checks nothing for you").
`RHttpServer` is the safe one and defaults to 127.0.0.1 with the full set
of checks (audited below, fine). The gap is only in the registry: the
`Mode` property doc (`network.toml:74`) describes "http" framing without
the one-line warning the manual gives, so an AI or a reader using the
registry alone does not see it.

**Recommended fix.** Add the manual's "checks nothing" sentence to the
registry `Mode` doc, and keep `BindAddress`'s own doc (which already warns,
`network.toml:22`).

**Fixed in** `77948a74` (lane SEC-FIX, 2026-10-23): RServerSocket's `Mode` doc
(`network.toml`) says that "http" checks nothing and that the socket listens
on every interface unless `Host = "127.0.0.1"`; the reference regenerated.

---

## 2. Checked and found fine

Each scope item read; where no finding is listed, it held up.

**RAI — saved chats** (`chats.rs`). Storage is Studio's data folder, never
the project (`:5-13`); writes are atomic (temp + rename, `:388-397`). File
names come from `safe_id` (letters/digits/`-`/`_`, length ≤ 64, `:227`)
and `project_key` (sanitised name + FNV digest, `:195-207`): a chat id or
project path cannot escape the folder — the tests assert
`../../etc/passwd` and `../evil` are refused (`:528-530`). Redaction before
write is strong and tested (see SEC-28). Delete removes the file and the
empty folder (`:399-407`); Undo restores from the kept raw and re-checks
the id (`restore`, `:304-309`). Fine.

**RAI — tools and the lite set** (`ai_tools.rs`). `project_path`
(`:243-273`) rejects absolute paths, drive-letters, `//` and `..` that
climbs out; results go back as `{"untrusted":true,…}` (`:316-319`) and the
instructions tell the model tool output is data, not instructions
(`ai.rs:198`, `:226`, `:259`). No tool writes to disk during a call:
`propose_edit`/`write_file`/`create_project` only build `Proposal`s
(`:1470`, `:1515`, `:1622`); files are written only when the user applies
(`ai.rs:decide`) or creates (`ai.rs:634` `create_project`, which picks a
fresh folder and records what it wrote for Undo, confining removals to
inside the folder, `:724-736`). Read-only permission blocks the proposing
tools (`:352`). The lite set (`:194-218`) is a strict subset using the same
`run`. Prompt injection from project files is contained by the untrusted
marking plus the human-in-the-loop on every change. Aside from SEC-23
(symlinks), fine.

**Secrets — the keystore** (`keys.rs`). `memory_only` (`:81-103`) makes a
redirected registry (`RAPIDR_REGISTRY`) mean the memory keystore unless
`…KEYSTORE=system`, with a unit test (`:112-119`): a test or capture can
never reach the real keychain by forgetting a variable. A key is shown only
as its last four characters (`hint`, `:242-249`); `clean` refuses spaces
and control characters (`:199-208`). `build_warnings` warns when a key is
written into a program and never prints it (`:304-336`, tested `:591-608`).
`redact` for files to the model removes key-shaped words (`:393-422`).
Aside from SEC-21 (shared service), fine.

**RSecrets** (`secrets.rs`). Values are never returned in errors
(`mask`, component `:273`); the debugger shows `••••` and the registry
marks Read/Save `secret` (`:13-15`); the web refuses outright, with no
fallback to page storage (`:20-22`, `:146-159`). Length and control-char
checks on service, name and value (`:68-84`, `:122-129`). Aside from SEC-22
(name keying), fine.

**Network — RHttpServer** (`httpserver.rs`, `netframe.rs`). The refusal set
is correct and tested: foreign Host → 403, cross-site / foreign Origin →
403, non-GET/HEAD without a token → 401, missing/wrong Bearer → 401, with a
constant-time token compare (`same_secret`, `:77-83`, tested `:676-683`);
preflight from an allowed origin passes without the token (`:123-131`).
Limits: head 64 KB / body 4 MB default (431/413), slow request 408, too
many connections 503, non-HTTP/1.x 400/505 (`netframe.rs:250-463`, tests
`:627-638`). Request smuggling is closed: both `Content-Length` and
`Transfer-Encoding` → 400, two differing lengths → 400, only `chunked`
accepted, folded headers → 400 (`:435-462`, `:422-426`). Response headers
drop CR/LF values so a program cannot split a response (`header_lines`,
`:498-507`), and SSE event/data strip CR/LF (`:533-546`). `NewToken` is 32
bytes from `getrandom` as 64 hex (`:223-227`, tested `:668-674`). Fine.

**Databases** (`mysql.rs`). `InTransaction`/`QueryAsync` read the server's
own session tracker, not a query of their own (`:41-49`). The SQL colouring
in RMarkdownView / the SQL editor is lexical only — no execution (it is a
code-scheme tokenizer, `markdown.rs:220-231`). The virtual grid streams
rows. Aside from SEC-25 (cert checking), fine.

**Studio — Search / Replace All** (`rapidr-editor/src/search.rs`,
`ide/search.inc`). The engine is Rust `regex` 1.12 / `regex-automata`
(`Cargo.toml:17,48` and `:18,21`) — linear, no backtracking; no other
engine. Replace All writes only into open editors as one undo step per file
(`search.inc:617-647`), through the project's files — no path escape, no
write outside the project. Fine.

**Studio — Find References / Go to Definition** use the language service's
symbols (include order), never a text search or evaluation
(`search.inc:1-21`). Fine.

**Studio — Problems / Targets / `runs_on`** (`project.rs:106-141`).
`runs_on` is read as one of three words with a safe default
(`runs_on_or_default`, `:133-141`); it drives a portability note, not
execution. Fine.

**Studio — S11 set-value-while-paused** — see SEC-29 (gate the DAP path);
the evaluate-by-design behaviour is intended and bounded by the kind
checks and `fit_value`.

**Studio — the helpers' value editors** write back into the editor's text
(the user saves), not to disk directly; same path as any edit. Fine.

**Studio — Open Recent** (`ide/project.inc:422-446`): a path that no longer
exists is dropped from the list with a message, never opened; paths are not
interpolated into any shell. Fine.

**Studio — forms from code** parse the source; no execution at design time.
Fine.

**Studio — the web's file opening** (`ide/project.inc`
`ProjectFilesElsewhere`, `ide/documents.inc` `OpenDocument`). Today's
WEB-OPEN fix is sound: a `.rrproj` whose main file is not in the page store
is not opened with empty files — the open project stays, the folder is
offered, and the picker starts at the project file (`file_picker_web.rs`
`LAST_PICKED` / `startIn`); `OpenDocument` clears the editor before
`LoadFromFile` so a file that cannot be read opens empty, never with
another file's code (commit `64174247`). Fine.

**Test hooks** — see SEC-24. The web path is correctly off by default
(needs `window.RAPIDR_STUDIO_TEST`).

**Supply chain / build** (`zip` 9, codegen). `zip` 9.0.5 is used for
writing only — `ZipWriter` with `SimpleFileOptions`
(`rapidr-buildserver/src/main.rs:419-421`) and the web bundle; the only
`ZipArchive`/extract use is in the bundle-injection test
(`tests/security/web_bundle_injection.rs:30`), not in shipped code. The
web bundle injection tests cover the hostile-name cases (SEC-16
regressions). Native codegen escapes user strings for Rust: string
literals via `replace('\\',…).replace('"',…)` into `v_str("…")`
(`codegen-rust/src/lib.rs:2541-2544`), floats via `{n:?}`, `OPEN`'s mode is
a fixed keyword (`:2418`); `toml_escape` escapes `\` and `"` for the
generated `Cargo.toml` (`:3340`), and `crate_name` sanitises the package
name (`:3329-3336`). A user string reaches generated Rust only inside a
`v_str("…")` literal, escaped. Fine (a line-continuation `\` at a string's
end would be doubled, so no code injection).

**rapidr-vcs** (Versions engine). Merged, but nothing shipped links it:
the only dependants are its own tests (`crates/rapidr-vcs/tests/*`), and
no `[dependencies]` of any other crate names `rapidr-vcs` (only the
workspace members list does, `Cargo.toml:37`). Its UI ships in Preview 5.
So its foreign-repo read-only stance and secret scan are not reachable from
Preview 4's UI. Fine for this release; audit its surface when the UI lands.

## 3. Note for the fix pass

- Fix **SEC-21** and **SEC-22** before shipping (keychain scoping).
- SEC-23, SEC-24, SEC-25, SEC-26 are good follow-ups; SEC-27, SEC-29 are
  one-liners. SEC-28, SEC-30 are documentation/permission touch-ups.
- No secret value appears in this document.
- **Fix pass done** (lane SEC-FIX, 2026-10-23): SEC-21, SEC-22, SEC-23,
  SEC-25, SEC-26, SEC-27, SEC-28, SEC-29 and SEC-30 fixed, each with a
  "Fixed in" line above and a test in `tests/security/`; SEC-24 accepted for
  Preview 4 and moved to Preview 5.
