# RapidR Studio's AI tools: one tool set for RAI and AI apps

Lane SHARED-TOOLS, 2026-11-05. Robert: "the editor should have its own tools
available for RAI, MCP … AI FIRST component" and "Both MCP and RAI should have
access to the same!" Before this lane, RAI's tools (Rust,
`crates/rapidr-studio/src/ai_tools.rs`) and Studio's MCP server's (RapidR code,
`ide/mcp.inc`, `ide/mcpapp.inc`) were two lists that drifted: RAI got
`save_version` and `run_program`, the MCP had no Versions tool; AI apps could
debug, RAI couldn't. Now every tool is defined once and both callers build
their lists from that definition.

## Where things are

| What | Where |
|---|---|
| Every tool, once: name, what it is for (when to use it, what it returns, an example call), its arguments' JSON Schema, how it acts (read / change / run), its permission class, who answers it, who is offered it | `crates/rapidr-studio/src/toolset.rs` |
| The one permission policy (both callers, Coach, School Lock), the shared audit line | `crates/rapidr-studio/src/policy.rs` |
| The project's and RapidR's knowledge tools (files, compiler, designer, Help, examples, versions read, gaps) | `crates/rapidr-studio/src/ai_tools.rs`, `tools_core.rs` |
| Studio's own tools — what only Studio's state knows (its documents, a run, the debugger, its Versions pane) — one implementation | `ide/tools.inc` |
| The running program's eyes and hands (app_*) | the program's runtime: `crates/rapidr-ui-app/src/link.rs`, `eyes.rs`, `hands.rs`; the request built once: `ai_tools::app_request` |
| RAI's side: offering (`toolset::for_rai`), routing, its cards | `crates/rapidr-studio/src/ai.rs` |
| The MCP side: listing (`McpTools.ToolNames`), aliases, prompts, resources, the pane | `ide/mcp.inc`, `ide/mcpapp.inc` |
| The core rules, the MCP prompts and resources | `crates/rapidr-studio/src/guide.rs` |

### The home, measured

The definitions, the policy and the handlers that need only the project and
RapidR's registry are Rust: one place, unit-tested, the same code both callers
already shared for files and the compiler. The handlers that need Studio's own
state — its open documents, Run and Stop, the debugger's frames and variables,
the Versions pane (save the open files first, reload the editors after a
restore) — are about 600 lines of RapidR code reading Studio's globals; moving
them to Rust would have meant exporting all that state into Rust. They stay in
RapidR, as **one** copy (`ide/tools.inc`, moved out of `mcp.inc`), and both
callers reach it:

- an AI app: `McpRun` calls `ToolsRun "mcp:<call>"`; the answer is the call's
  reply (`McpStudioAnswered`);
- RAI: its `ToolProvider` queues the call (ticket `st:N`); Studio takes it
  (`AITools.TakeStudioRequest`, on RAI's own timer, after "Work on its own"
  applied its waiting changes), runs `ToolsRun "rai:<ticket>"`, and answers
  (`AITools.StudioAnswer`).

## The inventory

Before → after. "RAI" and "MCP" say who had the tool before this lane; every
tool now has one name, one description and one behaviour.

| Tool | Component | Kind | Answered by | RAI before | MCP before | Now offered to | What changed |
|---|---|---|---|---|---|---|---|
| `studio_status` | project | read | Studio | — | yes | both | RAI gets it; says the permission in the caller's words, whether Versions are kept |
| `list_files` | project | read | core | yes | yes | both | — |
| `read_file` | project | read | core | yes | yes | both | — |
| `list_documents` | project | read | Studio | — | yes | both | — |
| `project_outline` | project | read | core | yes | yes | both | — |
| `open_project` | project | change | Studio | — | yes | AI apps | RAI works in its chat's project; its new programs open with their card's Open |
| `create_project` | project | change | core | yes | yes | both | — |
| `propose_edit` | editor | change | core | yes | yes | both | — |
| `write_file` | editor | change | core | yes | yes | both | — |
| `check_code` | editor | read | core | yes | yes (+ `get_problems`) | both | `get_problems` (MCP) is an alias of it |
| `rapidr_list`, `rapidr_get` | designer | read | core | yes | yes | both | — |
| `set_property` | designer | change | core | — | yes (designer, asked yes/no) | both | now a proposal like any change (the designer's smallest edit, `rapidr-designer`): the user sees a diff (Review) |
| `layout_report` | designer | read | core | — | yes | both | — |
| `button_grid` | designer | read | core | yes (`layout`) | — | both | renamed (next to `layout_report` and `app_layout` the old name confused small models); `layout` is an alias; never in Coach (it writes the code) |
| `run_program` | runner | run | Studio | yes (start/stop/status, 5 s, max 30) | yes (+restart, debug, 3 s, max 60) | both | one behaviour: start, stop, restart, status; debug; 5 s, max 60; **refused while the code has errors** (the first three said), warned loudly with warnings; the ERRORS lane's "started and ended after N s" kept |
| `read_output` | runner | read | Studio | — | yes | both (RAI: while the program runs) | — |
| `debug` | debugger | run | Studio | — | yes | both (RAI: while the program runs) | RAI may step a run it started (asked once a run), set breakpoints and read the state any time |
| `save_version` | versions | change | Studio | yes | — | both | AI apps get it; open files saved first (as Run does); Versions started when the project has none (as Save a Version does — a 4B model stopped at the refusal); the version's author is the AI ("RAI", "<app> (AI app)") |
| `list_versions` | versions | read | core | — | — | both | new |
| `compare_versions` | versions | read | core | — | — | both | new: a diff from a version to another or to now (the editors' text) |
| `restore_version` | versions | change | Studio | — | — | both | new: Go Back to This Version (a new version, Undo takes it back); asks in Ask before changes / on RAI's card |
| `start_versions` | versions | change | Studio | — | — | both | new: View > Versions > Start |
| `rapidr_describe` | help | read | core | yes | yes | both | — |
| `search_manual` | help | read | core | yes | yes | both | — |
| `find_example` | help | read | core | yes | yes | both | also searches RapidR's examples (file name, components and builtins used, header words); a component's name says "rapidr_describe {type}"; `recipe`, `find_recipe` are aliases |
| `how_do_i` | help | read | core | — | — | both | new: the workflow for a task, with the exact tools |
| `report_gap` | help | change (note) | core | — | — | both | new: one short line in the project's `RAPIDR-ISSUES.md`, nothing else written |
| `app_layout` | app | read (drive) | program | yes | yes | both | dialog parts listed by their short names (b0) |
| `app_screenshot` | app | read (drive) | program | — | yes | both | the picture **and** what it shows in words (RAI's models read the words) |
| `app_click`, `app_type`, `app_key`, `app_drag`, `app_scroll` | app | run (drive) | program | yes | yes (capitalised arguments) | both | one request builder; a message box's button by its short name (b0) or caption |
| `app_wait` | app | read | program | yes | yes | both | — |
| `capture` | studio | read | Studio | — | yes | AI apps | a picture of Studio: RAI's models read text (studio_status, layout_report say it in words) |
| `ask_rai`, `get_rai_steps`, `get_rai_transcript`, `get_rai_audit` | studio | rai / read | Studio | — | yes | AI apps | RAI is the one asked: it doesn't ask or read itself |

**Arguments** are lower case everywhere (`path`, `action`, `wait`). An AI app
that sends an older capitalised name (`Path`, `Action`, `ToX`) is still
understood: the server matches argument names in any case, and
`app_request` maps `ToX`/`to_x` alike.

**Aliases** (old names still answered, never listed — `RMcpServer.AddAlias`,
`toolset::ALIASES`): `get_problems` → `check_code`, `layout` → `button_grid`,
`recipe` and `find_recipe` → `find_example`.

## Permissions: one policy

`policy::decide(tool, mode, coach)` — RAI's Handle and the MCP server's
`McpDecision` (through `RAIProjectTools.DecideCall`) ask the same function;
RAI is offered exactly the tools it wouldn't refuse.

| AI apps (the pane's menu) | RAI (Settings > AI) | read | note (`report_gap`) | change | run, drive, rai |
|---|---|---|---|---|---|
| Read only | read-only | allow | refuse | refuse | refuse |
| Ask before changes | propose | allow | allow | ask | ask |
| Allow edits | — | allow | allow | allow | ask |
| Allow everything | auto ("Work on its own") | allow | allow | allow | allow |

"Ask" is each caller's own way of asking: an AI app's call waits in the AI
Apps pane (Allow Once, Allow for This Session, Deny; Review for a change); RAI's
changes are proposals the user reviews, its runs, its restores and its use of
the running program a card in its chat. The person's answers (Allow for This
Session, Allow for this run) are remembered by each caller.

**Coach** (RAI's dial, or School Lock keeping it there) refuses every change
and run and `button_grid` (it writes the student's code); reading and looking
at the running program stay. **School Lock**'s "no AI apps" keeps the MCP
server from starting at all (`ide/schoollock.inc`).

**The audit log** is one file both write (`policy::audit` for AI apps' calls;
RAI's turn loop in `rapidr-ai` for RAI's): when, who, which tool, the
project's file it touched — never a file's text, a picture, typed text or a
key.

## Small models

Layered, so a small local model isn't flooded and still knows the way:

1. **The core rules** (`guide::CORE_RULES`, 12 numbered one-line rules with the
   exact tool names, under 420 tokens, a test keeps it there): the MCP
   server's instructions on connect (with the language card, under 1,200
   tokens), and RAI's instructions when it may change and run. Among them: "To
   change code: save_version, read_file, propose_edit, check_code, run_program,
   app_screenshot or app_layout, save_version", "Problems must be 0 before you
   run, measure or report", "Never use git or a shell for the user's project".
2. **Every tool's description** says when to use it, what it returns and one
   example call (a test checks each has "Example" and no backslash).
3. **`how_do_i`** gives the workflow for a task on demand (change code, fix an
   error, debug a crash, add a form, learn a component, start a new app, test
   the running app, versions, report a gap).
4. **`report_gap`**: a missing tool or a docs gap goes into the project's
   `RAPIDR-ISSUES.md` (created when absent, append-only, at most 256 KB, never
   through a link) instead of an improvised shell.
5. **For MCP clients**: two prompts (`rapidr-way` "Work on this project the
   RapidR way", `new-app` "Start a new RapidR app") and three resources
   (`rapidr://guide/rules`, `rapidr://guide/workflows`, `rapidr://guide/forms`).

The cost: RAI's full set grew from 15 to 25 tools with the program stopped
(35 while it runs). The descriptions were written tight (about 11,500
characters of tool definitions); the debugger and `read_output` are offered
to RAI only while the program runs. The small-window set (`lite`) is
unchanged: 8 tools.

## The guards

- `toolset::tests::every_tool_is_offered_to_rai_and_ai_apps_alike_or_says_why_not` —
  the parity test: every tool is in RAI's list (everything allowed, the
  program running) and in the MCP's, or it is marked one-sided with its
  reason; at most six one-sided.
- `ai::tests::rai_reaches_every_shared_tool` — RAI (the mock-free handle)
  calls every shared tool and each reaches its one implementation.
- `toolset` / `policy` / `guide` / `tools_core` unit tests: names, aliases,
  schemas, the policy table, the core rules' budget, examples, versions,
  gaps.
- `rapidr-runtime-core` `mcpserver::tests::aliases_and_prompts`.
- `rapidr-ui-app` `eyes_tests::a_message_boxs_button_by_its_short_name`.
- `node tests/studio_mcp.mjs shared` — an MCP client: Versions over MCP,
  prompts and resources, the run refused at Problems > 0, `recipe`
  (`rai_models`), `find_example RJson`, `how_do_i`, `report_gap`, the audit log.

## Part B, next lane: RAI as an MCP client (design note, not built)

`RMcpClient` and `RAI.AttachMcp` sit on this plumbing:

- **Tools from a server are specs at run time**: the same shape as
  `toolset::Spec` (name, description, schema, kind, permission class,
  handler), built from the server's `tools/list`; names are prefixed with the
  server's (`design.create_frame`) so they never shadow Studio's; handler
  `external`.
- **One policy**: a new permission class `external` in `policy::decide`,
  asked by default whatever the mode (a server is someone else's program), with
  the per-server choices of Settings > RAI > Connected tools ("Ask / Allow
  reads / Allow all") as the grants; School Lock covers it; Coach refuses
  it.
- **Async**: the same ticket queue RAI's Studio calls use (`st:N` →
  `ext:N`), answered when the server answers; Stop drops them.
- **Data, never instructions**: a server's results go to the model marked
  untrusted with their source, as file contents do now.
- **Audit**: `policy::audit("rai", server, tool, …)` — the same file.
- **A stdio server starts only after the user's OK**, never from a project
  file.
