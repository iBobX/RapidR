# RapidR security audit, Preview 6 — AI / MCP (2026-11-05)

Area **AI-MCP** of the by-area Preview 6 audit (lane SECURITY-AUDIT-AI-MCP,
`dist/lanes/security-audit-brief.txt`). Read end to end: the shared AI tool
set and its one policy (`crates/rapidr-studio/src/{toolset,policy,tools_core,
ai_tools,ai,repository}.rs`, `ide/{tools,mcp,mcpapp}.inc`,
`ide/ai.inc` the RAI side), the MCP server and its HTTP guards
(`crates/rapidr-runtime-core/src/{mcpserver,httpserver,netframe}.rs`),
`crates/rapidr-ai` (keys, providers, the request, tool results as data), the
signed test mode (`crates/rapidr-testmode`), and the Versions engine as the
tools reach it (`crates/rapidr-vcs`). Base: `development` at `1e996749`
(workspace 2.122.1, the SHARED-TOOLS and JIT-TIER1 merges in). Nothing was run
against any network, keychain or the user's settings; the proofs are code
paths and the failing tests in
`crates/rapidr-studio/tests/security_ai_mcp_2026_11_05.rs` (temporary folders,
marker words, `127.0.0.1` only — all `#[ignore]` so the build stays green).

IDs continue the global `SEC-` series. The coordinator reserved SEC-40…49
for this (AI-MCP) area, SEC-31…37 having gone to two other merged audits.
Earlier audits: `docs/security-audit.md` (2026-10-08, SEC-14…20),
`docs/security/audit-preview4.md` (SEC-21…30), `docs/security/test-mode.md`
(SEC-24).

## 1. Findings

| ID | Severity | Area | One line |
|---|---|---|---|
| SEC-40 | **Medium** | AI tools, confinement | The source-reading tools and `compare_versions` read project files through the project's own file list and git history, without the path confinement `read_file` enforces (SEC-23) — out-of-project and hidden files reach the model — **fixed** (`fe4d1716`) |
| SEC-41 | Low | AI run, native power | An AI- or RAI-initiated `run_program` does not set `RAPIDR_SANDBOX`: under *Allow everything* / *auto* the run has full `DECLARE…LIB` / `SHELL` / `VARPTR` power (SEC-10's gate still not wired to AI-initiated runs) — **fixed** (`fe4d1716`) |
| SEC-42 | Low | AI tools, integrity | `set_property`'s `value` is written into the source as given, so a "set one property" change can carry extra source lines — the designer's "smallest edit" invariant and the Review diff's meaning break — **fixed** (`fe4d1716`) |

**Nothing is Critical or High. By the Critical/High rule, nothing in this area
blocks Preview 6.** SEC-40 is the one worth fixing before shipping: it
breaks a guarantee the AI-apps manual states in words ("nothing outside the
project or hidden in it"). SEC-41 is documented and gated behind the user's
explicit consent; SEC-42 is defence-in-depth (the AI already has
`write_file`).

---

### SEC-40 — Medium — Source-reading tools and `compare_versions` aren't confined to the project

**Status: fixed** (lane SEC-FIX-AI). Fixed in `fe4d1716`: one rule,
`ai_tools::readable` (`project_path`'s — no `..` above the folder, nothing
absolute, no link out of it — and never a hidden part), used by
`Project::text` (so every whole-project tool and every proposal's base),
`resolve` (every `path` argument, RAI's too), `list_files` / `files_note`,
`ai::outside_path_ok`, and `compare_versions` / `list_versions` for every path
from the file list, the editors and both versions' trees (the refused ones
counted, never named). The reading tools' answers are key-redacted
(`ai_tools::redacted`). The three proof tests run (no `#[ignore]`) and pass;
`ai_tools::tests::every_read_follows_the_one_rule` adds the outline, the
listing, RAI's `read_file` / `write_file` of a hidden file and a key in a
form's caption.

**Where.**
- `crates/rapidr-studio/src/ai_tools.rs` — `Project::text` (`:403`),
  `Project::all` / `sources` (`:419`, `:424`): read `join(folder, rel)` for
  every `rel` in `p.files` / `p.buffers`, with **no** `project_path`
  confinement (no `..` rejection, no SEC-23 symlink check). Used by
  `project_outline` (`:980`), `check(code)` (`:1606` → `p.all()`),
  `layout_report` / `rapidr_list` / `rapidr_get` (through `sources()`).
- `crates/rapidr-studio/src/tools_core.rs` — `compare_versions` (`:508`):
  for the whole-project diff it iterates `p.files` + `p.buffers` +
  `repository::read::files_in(folder, version)` and reads each with
  `p.text(rel)` (`:542`, `:561`). `list_versions`'s "changed" list comes the
  same way.
- The guarantee being bypassed: `read_file` **is** confined —
  `ai_tools::resolve` → `project_path` → `inside_for_real` (`:290`, `:372`,
  the SEC-23 fix) — and for AI apps `ai::outside_call` also runs
  `outside_path_ok` (`crates/rapidr-studio/src/ai.rs:1584`), which refuses a
  hidden file and, in a home / drive-root folder, anything not in the file
  list. The source-reading tools and `compare_versions` run **none** of this
  on the files they read.

**The attacker and the scenario.** A crafted project the developer opens
(a download, a clone, a model-written project, a corpus file) — opened, not
even run — and then lets RAI, or an AI app over MCP, work on. Three vectors,
all confirmed by the proof tests:

1. **A `.rrproj` file-list entry that climbs out** (no symlink needed).
   `rapidr_project::normalize_path` keeps a leading `..` for a relative path
   (`crates/rapidr-project/src/project.rs:665-677`), so a `.rrproj` with
   `[[files]] path = "../outside/lib.rr"` loads into `Proj.Files` verbatim;
   `ide/mcp.inc` `McpSyncProject` (`:990`) copies `Proj.File(i)` into
   `McpTools.ProjectFiles` as is. `read_file "../outside/lib.rr"` is refused
   ("outside the project"), but `project_outline` reads it and shows its
   SUBs / FUNCTIONs. (Proof: `sec40_a_listed_file_outside_the_project_is_
   never_read`.)
2. **A folder that is a symbolic link out of the project.** A project folder
   whose `notes/` is a link to elsewhere: `compare_versions` reads
   `notes/todo.txt` from the link target even though `read_file` refuses that
   path (SEC-23). (Proof:
   `sec40_compare_versions_never_reads_through_a_link_out_of_the_project`.)
3. **A hidden file in history** (AI apps' extra guarantee). `.env` (or
   `.git/config`, an SSH key copied in) is refused to an AI app by name
   (`outside_path_ok`), but `compare_versions {}` shows its old and new
   contents because the whole-project diff never calls `outside_path_ok`.
   (Proof: `sec40_ai_apps_never_see_a_hidden_file_through_compare_versions`.)

**Impact.** The contents of files outside the project folder, or hidden
inside it, are returned to the model — for a cloud provider, sent off the
machine. `rapidr_ai::keys::redact` (run on `read_file`'s result) is **not**
applied on these paths either, so a key-shaped line in an out-of-project file
is not masked. This is the same class as SEC-23 (there rated Low, for RAI
following a symlink); it is Medium here because it also defeats the AI-apps
manual's explicit promise — "list and read the files in your open project …
and nothing outside the project or hidden in it (a file or folder whose name
starts with a dot)" (`docs/manual/ai-apps.md`) — and because vector 1 needs
no symlink, only a crafted project file.

**Recommended fix.** Confine the source-reading paths the way `read_file`
is. Options: (a) make `Project::all` / `sources` drop any `rel` that
`project_path(folder, rel)` rejects (both the `..`/absolute check and the
`inside_for_real` symlink check), so `project_outline`, `check_code`,
`layout_report` and the designer tools never read out of the folder or
through a link; (b) in `tools_core::compare_versions` and `list_versions`,
resolve every `rel` (from the file list, the buffers and the git trees)
through `project_path` before `p.text(rel)`, dropping the ones that don't
resolve; (c) for AI apps, also run `ai::outside_path_ok` over every path the
whole-project tools would read, so the hidden-file rule holds there too.
Reads that pass should still be key-redacted (`keys::redact`) as `read_file`
does. A `.rrproj` loaded into Studio could also reject a `path` with `..` at
load time (`rapidr_project`), as the build server does for its inputs.

**Evidence.** `crates/rapidr-studio/tests/security_ai_mcp_2026_11_05.rs`:
`sec40_a_listed_file_outside_the_project_is_never_read`,
`sec40_compare_versions_never_reads_through_a_link_out_of_the_project`
(Unix), `sec40_ai_apps_never_see_a_hidden_file_through_compare_versions`.
Each fails today with the marker from outside / the hidden file in the tool's
answer.

---

### SEC-41 — Low — An AI-initiated run is not sandboxed (SEC-10 still open for AI runs)

**Status: fixed** (lane SEC-FIX-AI). Fixed in `fe4d1716`: a run started
while one of Studio's tools is answered (`run_program`, `debug`'s steps —
`ToolsCalling`, `ide/tools.inc`) sets `RProgramSession.Sandboxed`, and the
desktop host gives the child `RAPIDR_SANDBOX=1`, unless the user turned on
Settings > AI's new *Programs the AI runs may use DLLs and SHELL*
(`AIRunFullPower`, off by default, a consent question before it turns on).
The gate now also covers `SHELL` / `SHELLWAIT` / `RUN`
(`rapidr_value::dll::sandboxed`, one test for both), and `ENVIRON` can't
remove `RAPIDR_SANDBOX`; a program the run starts inherits it. The run's
answer and `studio_status` say what is blocked and what to tell the user
(F5, the setting), so a model doesn't loop. The user's own Run is
unchanged. Tests: `tests/studio_mcp.mjs sandbox` (an AI app's run refused at
SHELL, a debug-started run too, full power when allowed, the user's F5
not sandboxed), `tests/security/peek_poke_unowned_memory.mjs` (SHELL,
SHELLWAIT, RUN, ENVIRON), `rapidr_value::environ::tests::the_sandbox_stays`.
What the sandbox does **not** block (said in the tool's answer): files, the
network, the program's own windows — it is the native-power gate of SEC-10,
not an operating-system sandbox.

**Where.** `ide/debug.inc:778` `StartProgram` (what `run_program` calls,
`ide/tools.inc:288`) runs the program exactly as the user's own Run does: no
`RAPIDR_SANDBOX` is set for a run that RAI or an AI app started. The runtime
gate exists and works — `crates/rapidr-runtime-core/src/ffi.rs:367`
`sandboxed()` reads `RAPIDR_SANDBOX`, and a `DECLARE…LIB` call,
`SENDMESSAGE` / `POSTMESSAGE` / `KILLMESSAGE` are refused when it is set
(`crates/rapidr-value/src/dll.rs:112`) — but no `ide/*.inc` ever sets it for
Studio's Run. `grep -rn RAPIDR_SANDBOX ide` finds nothing.

**The attacker and the scenario.** A prompt-injected or hostile model, or a
hostile AI app, under **Allow everything** (AI apps) or **Work on its own**
(RAI, `auto`) — where `run_program` is allowed with no per-run question —
writes a program that `DECLARE`s a system library (Windows) or uses `SHELL`
and runs it. The program then does anything the user can on this computer.

**Impact.** Full local code execution from an AI-initiated run. It is **gated
behind the user's explicit consent**: *Allow everything* shows a confirmation
and a standing amber "No questions asked" badge, and the manual states "a
program they run can do anything you can on this computer … Choose it only
while you watch an AI app you trust" (`docs/manual/ai-apps.md`); RAI's `auto`
had its own consent dialog. So this is the documented, chosen trade-off, not
a silent bypass — hence Low. It is recorded here because it is squarely in
this area (the shared `run_program`) and the first audit's design (§7 of
`docs/security-audit.md`: "an MCP-initiated run never gets `DECLARE … LIB`,
`RUSTSTART` or … VARPTR / PEEK / POKE") is **not yet implemented for
AI-initiated runs** — it was "left with SEC-10".

**Recommended fix (SEC-10, for the fix lane).** When a run is started by RAI
or an AI app (not the user's own Run), set `RAPIDR_SANDBOX` on the child
unless the project is explicitly marked native-privileged with a one-time
confirmation (the SEC-10 "native-privileged project flag"). The cleanest seam
is `StartProgram`: thread "who started this run" from `ToolsRunProgram`
(`ide/tools.inc`) and set the environment for AI-started runs. This does not
change the user's own Run.

**Evidence.** Code path above; no separate test (it would require a built
runner and is the documented behaviour — a test belongs with the SEC-10 fix,
asserting an AI-started run refuses `DECLARE…LIB`).

**Does not block Preview 6** (documented, consent-gated).

---

### SEC-42 — Low — `set_property` writes its value into the source verbatim

**Status: fixed** (lane SEC-FIX-AI). Fixed in `fe4d1716`:
`ai_tools::one_value` checks the value before the designer writes it — no
control character, and read by RapidR's own lexer and parser as
`v = <value>` with a line after it: one assignment ending where the value
ends, the next line still its own (no `:` and a second statement, no `_`
continuation, no comment). Refused with "can't be set to …: that is more
than one value … Give the value alone, on one line". The proof test runs and
passes; `ai_tools::tests::a_property_value_is_one_value` covers what the
designer writes (numbers, constants, `&H`, sums of flags, quoted text with
`""`, `CHR$` joins) and the refusals.

**Where.** `crates/rapidr-studio/src/ai_tools.rs:1837` `set_property` →
`:1864` builds `source` from `value` (for a typed or quoted property, the raw
text) and `:1865` applies `rapidr_designer::Command::SetProp`, which replaces
the property's value span with that text
(`crates/rapidr-designer/src/lib.rs:684-701`). The value is never required to
be a single line / single token.

**The attacker and the scenario.** A model (hostile or prompt-injected) calls
`set_property {"component":"Button1","property":"Width","value":"120\n
Caption = \"…\""}`. The newline goes straight into the CREATE block, so one
`set_property` call adds a second property assignment (and, with the right
text, could add further lines inside the block).

**Impact.** The change presented to the user — in the Review diff for *Ask
before changes* / *propose*, or applied silently for *Allow edits* /
*auto* — is described as setting one property, but carries more. It is **not
a privilege escalation**: the same caller already has `write_file` and
`propose_edit`, which write arbitrary code by design, so `set_property`
grants nothing new. The harm is integrity / honesty: the designer's
"smallest change, as the Properties pane sets it" invariant
(`docs/ai-tools.md`, `docs/manual/ai-apps.md`) is broken, and the diff a user
approves under *Ask before changes* understates what it does. Coach cannot
reach it (it is `change`, refused in Coach). Hence Low.

**Recommended fix.** Reject (or escape) a `value` that contains a newline or
would parse as more than one property value, in `set_property` before
calling the designer — or have `rapidr_designer` refuse a `SetProp` whose
value spans statements. A property's value should stay one value.

**Evidence.** `sec42_set_property_takes_one_value_never_more_lines` — the
injected `Caption = "SEC42-MARKER"` appears in the proposed text of
`main.rr`.

---

## 2. Checked and found SOUND

Each read end to end; where no finding is listed, it held up — so the next
audit need not redo it.

**MCP HTTP guards** (`httpserver.rs` `Config::check`, `netframe.rs`). The
refusal set is correct and applied **before the program sees anything**, one
module for RHttpServer and RMcpServer: a foreign `Host` → 403 (DNS
rebinding; `own_hosts` is `127.0.0.1` / `localhost` / `[::1]` at the bound
port); `Sec-Fetch-Site: cross-site` → 403; a foreign `Origin` → 403; without
a Token, only GET / HEAD; a missing / wrong `Authorization: Bearer` → 401,
compared in constant time (`same_secret`, `:77` — length-xor plus a cyclic
compare over the *given* value, so no secret byte leaks). The OPTIONS
preflight token-bypass (`token_ok`, `:125`) is reached only after `host_ok`
and `origin_ok` pass, and RMcpServer starts with an **empty** allowed-origins
list (`mcpserver.rs:465`), so a cross-origin preflight is 403 at `origin_ok`;
OPTIONS to the MCP endpoint is 405 with no body (`arrived`, `:896`).
`random_token` is 32 bytes from `getrandom` as 64 hex (`:223`).

**MCP protocol** (`mcpserver.rs`). Both eras on one POST endpoint; the current
revision cross-checks `MCP-Protocol-Version` / `Mcp-Method` / `Mcp-Name`
against the body (`modern_request`, `:976`) and the `_meta` version; a path
other than the server's → 404 (`:869`); GET → 405 (no server-initiated
stream); `ascii()` emits every non-ASCII byte as `\u` inside strings, so a
tool result can't break framing; `header_value` base64 decode is bounded and
failure-safe. `check_args` / `coerce_args` validate against the schema and
tolerate argument case and JSON-as-string without widening types. Aliases are
resolved to the canonical tool **before** the program sees the call
(`:1160`), and a client the user disconnected (`shut_out`) is refused on every
path — `initialize`, `server/discover`, modern and legacy (`:937`, `:1010`,
`:1098`, `:1122`). Body limit 4 MB (RHttpServer default), 32 connections.

**RMcpServer for users' apps** (`mcpserver.rs`). `Start` refuses an empty
token and one shorter than 16 chars (`:458-463`) and empties the origins
(never a web page). `AddTool` / `AddAlias` / `AddPrompt` / `AddResource`
validate the name charset and length, parse the schema / annotations as JSON,
and a `params_schema` SUB-signature parser that only emits known types. A
program never gets a RapidR address or the raw socket; it answers calls on its
own thread, one at a time.

**One permission policy** (`policy.rs`, `ide/mcp.inc` `McpDecision`,
`ai.rs::call`). RAI and the MCP server both call `policy::decide`, which
resolves aliases to the canonical name (`toolset::find` → `canonical`) **before**
the `COACH_NEVER` check, so `layout` (alias of `button_grid`) is refused in
Coach like its tool. The four levels map identically for both callers
(parity test `toolset::tests::every_tool_is_offered_to_rai_and_ai_apps_alike`,
`policy::tests::the_four_levels_decide_alike_for_both_callers`). Read-only
refuses every change / run / drive / rai / note (including `report_gap`);
Coach refuses change and run and `button_grid`, keeps read and looking at the
running program. RAI is only *offered* tools it wouldn't be refused
(`rai_offers`), and `ai.rs::call` refuses again before anything — a tool RAI
isn't offered, and anything the permission or Coach forbids — so the offering
is not the enforcement. The MCP server lists every tool in every mode and
tells a refused call why.

**School Lock** (`school_lock.rs`, `ide/schoollock.inc`). "No AI apps"
(`ai_apps`) keeps `McpStart` from starting the server and `McpStopServer`
stops a running one on lock; `ai` turns RAI off; `coach` keeps RAI in Coach,
and `ai.rs::coach()` reads `locked_now("coach")` on every call, so the lock
holds in code, not only in the UI. The password is a PBKDF2-HMAC-SHA256 hash
(600k rounds, 16-byte salt, constant-time compare), never the password; wrong
tries are rate-limited and kept across restarts; the computer policy file
fails closed (an unreadable policy locks everything). `RAPIDR_SCHOOL_POLICY`
can only *add* a lock (read only when the computer has no policy).

**RAI's own enforcement** (`ai.rs`). `app_call` / `studio_call` gate the
running program's eyes-and-hands and Studio's own tools: reading-only refuses
them, Coach refuses clicking / typing (keeps looking), and a run / stop /
restore / program-use waits for the user's card once a run unless `auto`.
`ask_rai` from an AI app is answered at **propose** level even when the user
set RAI to `auto` (`AIPermissionNow`, `ide/ai.inc:1894`; `AIAskedByApp`):
"Work on its own" is the user's yes to RAI, not to every app allowed to ask
RAI once, and it stays propose until the user has decided on the changes that
question left. `drop_requests` / `begin_turn` clear held and queued calls, so
nothing is done after Stop or a new question.

**Keys** (`crates/rapidr-ai`). A provider key travels only in `x-api-key`
(Anthropic) or `Authorization: Bearer` (OpenAI) — `wire.rs:48-68` — never in
the URL or another header; `HttpRequest::redacted` ("What Was Sent") hides
both (`wire.rs:83-100`). `read_file`'s result is run through
`keys::redact` before it reaches the model (`ai_tools.rs:629`). The two
`eprintln!` in the crate (`secrets.rs:371`, `provider.rs:255`) carry a masked
error and a config message, never a key. SEC-21 / SEC-22 keychain scoping is
present (`keys.rs` `Scope`, `secrets.rs` `program_id`): a program's RAI key
and RSecrets are under its own service, so it can't read Studio's.

**Prompt injection** (`ai.rs`, `rai.rs`, `wire.rs`). Tool results go back
marked `{"untrusted": true, "source": …, "content": …}` (`ai_tools.rs:445`),
and the instructions tell the model tool output and the program's printed
output are data, never instructions (`ai.rs:629`, `:657`, `:698`). The last
run's output is in the system prompt's *tail* (`facts`, `ai.rs:695`;
`rai.rs:1783` → `wire.rs:162-202`), also marked untrusted with its source —
acceptable (it is the program's own output, and the model is told so).

**`report_gap`** (`tools_core.rs:400`). Writes only `RAPIDR-ISSUES.md` in the
project folder (fixed name via `crate::join`), refuses when it is not a plain
file (symlink check, `:422`), caps the file at 256 KB, and cleans the model's
text of control chars / headings / `<` `>` before writing. Refused in
read-only. (Test `tools_core::tests::a_gap_is_written_…`.)

**`restore_version` / `save_version` / `start_versions`** (`ide/tools.inc`,
`repository.rs`, `rapidr-vcs`). `restore_version` with a `path` checks
`InProject(ProjectPath(rel))`; the whole-project `go_back` switches the
working tree to a git tree, and `go_back_file` / `file_at` read from a git
tree — git tree paths are repo-relative and cannot contain `..`, so restore
is bounded by what is tracked. `save_version` runs the secrets scan (refuses
lines that look like keys unless the user allows) and saves as the AI's
author, not the user's. Versioning refuses a drive root, the home folder and
its Desktop/Documents/Downloads, and a folder inside another repo
(`repository.rs:422`).

**Signed test mode (SEC-24)** (`crates/rapidr-testmode`). A release build's
hooks act only with an Ed25519-signed token for the key it trusts; the
repository ships a placeholder (`public: none`) that trusts nothing, so it
**fails closed**. `verify` checks the key id, the signature, the purpose, the
expiry (`expires >= today`, and `issued <= expires`, at most 366 days ahead)
and the optional machine; the token file is capped at 4 KB. Every hook —
including Studio's `RAPIDR_TEST_MCP_TOKEN` stand-in — reads its variable
through the gate; `test_mode_release.mjs` keeps shipped crates from reading a
hook variable directly.

**Studio's MCP server config** (`ide/mcp.inc`). `127.0.0.1` only, the token
from the system keychain via RSecrets (`McpTokenNow`), no extra hosts or
origins set. `capture` refuses while any Studio dialog that could show a key
or the token is open (`McpDialogOpen`). `open_project` opens only
`.rrproj` / `.rr` / `.bas` / `.rqw` / `.rqb`, refuses `..`, and asks in every
mode for a path outside the Projects folder / examples / the recent list;
it never discards unsaved changes. App-tool pictures are written 0600 and
`KILL`ed after they are handed over, with a 60-second sweep of any left by a
crash (`session.rs:133`, `app_picture`, `mcpapp.inc:110`).

## 3. Note for the fix lane

- Fix **SEC-40** before Preview 6 ships (it breaks a documented AI-apps
  guarantee). The fix is to run the same `project_path` / `outside_path_ok`
  confinement the `path`-argument tools use over every file the
  source-reading tools and `compare_versions` read, and to key-redact those
  reads.
- **SEC-41** belongs to the SEC-10 lane (native-privileged gate): set
  `RAPIDR_SANDBOX` on AI-started runs. It is documented and consent-gated, so
  it does not block.
- **SEC-42** is a one-place guard in `set_property` (or `rapidr_designer`):
  a property's value stays one value.
- No key, token, password or file content appears in this document or in the
  proof tests (marker words only).
- The proof tests are `#[ignore]`d; remove the attribute with each fix so the
  test turns green and guards the fix.
