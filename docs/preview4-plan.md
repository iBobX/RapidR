# Preview 4: Studio, solid (plan, 2026-10-20)

**Find anything. Never lose anything. RAI runs and fixes its own work.**

Robert (2026-10-20), after an evening building a real program with RAI and DeepSeek: the next preview puts Studio first. It should be strong, solid, easy to use, responsive, beautiful and secure. That means the IDE details, Versions, stronger AI, and documentation so that any model writes correct RapidR. Then a security audit, and the release.

This plan grows Preview 4 from "Versions" ([versions-plan.md](versions-plan.md)) to the whole of Studio. Versions stays in it unchanged. The core of M1 (RAI runs, sees and asks; [ai-ux-plan.md](ai-ux-plan.md) §7) moves forward into it.

Like the Claude Code desktop app, Studio keeps that power. Unlike it, everything is explained in plain words.

## 1. What the DeepSeek evening showed

The test: "Build me a web server", deepseek-flash, Studio 2.120.0. The project is in `~/Downloads/RapidRWebServer`.

**What went well.** A cheap model wrote a 4,500-line program in 7 files:
- virtual hosts and per-site ports;
- `.htaccess` rules;
- PHP through php-cgi, and Node sites;
- a hosts-file editor;
- certificates;
- a port-80 helper.

The model was capable. Studio made it work blind:

| What happened | What it tells us |
|---|---|
| Robert pasted errors or logs **about 19 times**. RAI said "I can't run programs from here" | RAI must run, read the output and fix in a loop (R7) |
| RAI guessed the cause of the "can't write" bug five times: hidden dot files, then FILEEXISTS, then the shell. RapidR's file functions are fine: OPEN/PRINT #/FILEEXISTS on dot files and MKDIR were tested on 2.120.0 on 2026-10-20 | Guessing without running wastes the user's time and tokens (R7) |
| The program asked for the administrator password at start-up in a loop. It wrote `/etc/hosts`, added a root certificate to the system's trust store, and ran `node` as root | Code that RAI writes or runs must never do that without an explicit, informed OK (R9) |
| `ui.rr` holds three forms (Form, NewForm, CfgForm), but the project tree still listed it as a module. `main.rr`, with no form left, was still "the form" | Forms come from the code, not from a stale project file (S1) |
| 167 "problems", all notes saying "works on the desktop only", one per `RServerSocket` call | The Problems tab must tell the truth (S5) |
| The editor showed errors in module files while the project compiled | A module is never checked alone (S5) |
| Two answers came in German | The reply language is pinned (R10) |
| "You applied… You rejected…" for the same files, one after the other | The apply/reject log is wrong (R11) |
| A 2-line fix arrived as `+382 −384`; a file arrived cut off (`END FUNCTI`); `main.rr` was refused because the modules it includes were still pending | Edits are targeted; truncated writes are refused; pending files are checked together (R11) |
| Sending a message while RAI worked answered "Stopped." | Messages queue (R3) |
| `FOR i AS INTEGER`, duplicate `DIM`s, unknown helper names across files, wrong argument types for `RServerSocket` | The knowledge track (K1–K4) |
| Reopening the project lost the whole chat | Chats are kept (R1) |
| 9,055,355 tokens sent (8.1 M from the cache); "The conversation so far was summarised" after almost every question | Compaction is tuned and visible (R5) |
| Robert didn't find Find in Files. It exists (Ctrl+Shift+F, case/word/regex, Replace All), but it is hidden | Search you can see (S3) |
| No File ▸ Open Recent. The list exists, but only on the Welcome page | S2 |

## 2. The tracks

★ = must-have: it defines the release. Everything else boards the release only if it is walked like a user in time. Otherwise it moves to the next preview instead of delaying this one (the train rule).

### S: Studio essentials

| # | Item | Model, size |
|---|---|---|
| S1 ★ | **Forms in any file.** Any file with `CREATE … AS RForm` shows its forms in the tree, and each one opens in the designer (several per file). The project file follows the code. RAI's tools keep it in step | Opus M |
| S2 ★ | **File ▸ Open Recent** (projects and files, Clear list). Reopen the last project and its tabs | Sonnet S |
| S3 ★ | **Search you can see.** A magnifier tab next to Project; a scope (this file, open files, modules, forms, resources, the whole project, a folder); `file:line:col` with the match highlighted. Also **Find References** and **Go to Definition**, because the real question was "where is this variable used?" | Sonnet M (UI) + Opus M (references in the language service) |
| S4 | **RRegex** for programs: the engine Studio already uses (Rust `regex`, MIT/Apache). It runs in linear time, so no pattern can hang a program (no ReDoS). Match, Replace, Split, Groups; desktop and web; in the registry and MCP. An extension (origin rapidr) | Opus M |
| S5 ★ | **Problems that tell the truth.** Module files are checked as part of their project. Errors come first, grouped by file. Portability notes appear only when the project targets the web (a project **Targets** setting), one per component | Opus M |
| S6 ★ | **Run gets your Terminal's PATH.** On macOS and Linux, GUI apps start with a short PATH, so `SHELL "php …"` fails from Studio but works in Terminal; Studio reads the login shell's PATH once. Runtime errors show `file:line` as a link | Sonnet S |
| S7 | **Responsive and beautiful pass**, measured on the 4,500-line project: typing latency, open, search, scroll. The docked diff and streamlined buttons (ai-ux §6, shared with Versions G2) | Opus M (visual, one lane at a time) |

### R: RAI, the assistant that runs and fixes its own work

| # | Item | Model, size |
|---|---|---|
| R1 ★ | **Chats are kept**, per project, across restarts. A chat list: switch, rename, delete, search; New Chat keeps the old one. Chats are stored in Studio's data folder, not the project folder, so they are never shared or committed by accident. Keys are redacted | Opus M |
| R2 | **Edit a sent message**, and **Retry**. The answers after it move to a saved copy, so nothing is lost | Sonnet M |
| R3 ★ | **Typing while RAI works queues the message.** Stop is its own button | Sonnet S |
| R4 ★ | **A live status line:** Thinking · Reading sites.rr · Searching the manual · Writing server.rr · Checking · Running your program · Waiting for you. It shows the seconds, and "No answer from the model for 30 s: wait, or Stop" | Sonnet S |
| R5 | **Context, in the open:** a meter; auto-compact (tuned); a **Compact** button; `/` commands shown as you type (`/compact /new /model /effort /run /undo /help`). Reasoning effort Low/Medium/High where the provider supports it; "Thought for 12 s", collapsed and expandable | Opus M |
| R6 ★ | **One model picker with ↻ refresh.** The provider row moves to Settings. **Set up RAI**, a wizard: free on this computer (it finds MLX, Ollama, LM Studio), or a cloud key (stored in the Keychain, tested, a model picked), with cost and privacy in plain words | Opus M (visual) |
| R7 ★ | **RAI runs and debugs.** It can: run the program with a time limit; read the output and runtime errors with `file:line`; read Problems; do scripted clicks and typing, and read properties back (our `RAPIDR_TEST_EVENTS` / `RAPIDR_TEST_DUMP`); take screenshots of the program's window when the model can see images; use breakpoints and variables through the debugger. It loops (build → run → check → fix) until it works, or asks. The same in the web Studio, where programs already run in the browser's sandbox | Opus L |
| R8 ★ | **Permissions, like Claude Code, in plain words.** "RAI wants to run your program": Allow once · Allow for this chat · Not now. The defaults: reading the project is always allowed; edits follow Review first or Auto with Undo; running asks; screenshots ask; nothing outside the project | Opus M |
| R9 ★ | **Safe test runs.** When RAI runs a program, the runtime blocks `SHELL`/`RUN`, administrator prompts, files outside the project folder and ports below 1024. Each one can be allowed separately. The policy lives in the shared runtime core. A **red card** on proposed code that asks for administrator rights, changes system files, trusts a certificate, or kills processes: "Only accept if you understand this" | Opus L; design review by Fable, with Robert's OK |
| R10 ★ | **RAI answers in your language**, every time | Sonnet S |
| R11 ★ | **Fixes from the transcript:** the apply/reject log; targeted edits instead of whole-file rewrites; a truncated file is refused with a clear message; pending files are checked together; network errors retry once | Opus M |

### K: Knowledge, so every model writes correct RapidR

| # | Item | Model, size |
|---|---|---|
| K1 ★ | **The language card** (about 2K tokens, sent with every chat), generated from the compiler and the registry so it can never drift. It holds the rules models actually get wrong | Opus M |
| K2 | **Every method with a tested 3-line example**, generated and compiled in the regression suite. Exact signatures (the "expecting DOUBLE but got STRING" errors) | Sonnet M |
| K3 ★ | **`search_manual`** (BM25 over docs/manual) and **`project_outline`** (files, include order, forms, and each file's SUBs, FUNCTIONs and globals), so a model never re-declares a name or calls one that doesn't exist | Sonnet M |
| K4 ★ | **The mistake ledger.** Each recurring model mistake becomes one of three things: (a) an additive extension when it is safe and unambiguous, such as `FOR i AS INTEGER = 1 TO n` (written constantly by models trained on FreeBASIC and VB), in both backends and the web, never changing RapidQ behaviour; (b) a compiler message that says the fix; or (c) a line in the card | Opus M |
| K5 | **Project notes for RAI** (`.rapidr/notes.md`): decisions that survive compaction and new chats, editable by you | Sonnet S |
| K6 | **Evals and a "Which model?" report.** New cases: a multi-file app, a server, a game, and this evening's failures. Per model: Haiku, DeepSeek, Gemma (local) and others. Published with the release | Sonnet M (+ small API cost) |

### E: Helpers, the code that shows and edits its own values ([code-helpers-plan.md](code-helpers-plan.md), Robert 2026-10-21)
- What they are: inline marks (colour swatches, thumbnails, font samples, ▶, units, missing-file marks), argument-name chips, a one-line note for the caret's line, and values while running.
- Value editors: one click or Ctrl+. opens one, and it writes back as a single undo step in your own format. The same editors serve the Properties panel, RAI's review cards and programs (`RValueEditor`).
- ★ E0 (the framework: a `kind` facet in the registry, sites from the language service, no layout shift, keyboard and screen reader), E1 (colour, files, fonts, choices, links), E2 (chips, line note, values while running), E6 (filling `kind`).
- If ready: more helpers, live tweak while running (the WOW), and your own helpers in RapidR.

### V: Versions (unchanged; [versions-plan.md](versions-plan.md))
- ★ G0–G4, G8 checkpoints, G9 review, G10.
- G5 blame, G6 remotes (needs Robert's GitHub OAuth App) and G7 Pro board the release if ready.
- The docked Diff tab (G2) is shared with RAI's review.

### X: Security audit and legal
| # | Item | Model, size |
|---|---|---|
| X1 ★ | Threat model update for the new surfaces: RAI runs code, stored chats, Git credentials and remotes, regex, project-wide search, MCP | Opus S |
| X2 ★ | Automated: `cargo deny` / `cargo audit`; fuzzing of the lexer and parser, `.rrproj`, regex patterns and Git objects; a secret scan of the tree | Sonnet M |
| X3 ★ | **The audit**: on Fable with Robert's OK (or Opus); Opus fixes; a public report in `docs/security/` | Fable or Opus L |
| X4 ★ | **The six legal traps** (Robert's rule, one Sonnet each): Versions adds a GitHub sign-in and chats keep personal data locally. Most are expected to be N/A; the check proves it | 6 × Sonnet S |

## 3. Order

1. **Thursday (after the reset):**
   - M0 (small; master-plan §3);
   - the quick wins: S2, S6, R3, R10, R11;
   - the "RapidR's own words" sweep: no other products named in code, comments, docs or UI (about 340 mentions; RapidQ stays only as the compatibility footnote; Sonnet S);
   - S1;
   - G0 (survey and spike);
   - K1, the card.
2. **Then:**
   - R7–R9: runs, permissions, safe runs. The Fable design review comes first, if Robert agrees;
   - S3, S5;
   - E0–E2, E6 (helpers);
   - R1;
   - G1–G2.
3. **Then:**
   - G3–G4, G8;
   - R4–R6;
   - K3, K4;
   - S4, S7;
   - whatever else boards.
4. **Last:**
   - X1–X4;
   - the USE-AUDIT walk;
   - the manual with screenshots;
   - the regression run and web parity;
   - the video (EN/ES/PT): "Find anything. Never lose anything. RAI runs and fixes its own work.";
   - the release.

**Budget, roughly:**

| Part | Share of a week's budget |
|---|---|
| Versions | ~20–25% |
| R | ~30% |
| S | ~15% |
| K | ~10% |
| E (helpers) | ~10–15% |
| X | ~10% (+ Fable) |
| **Total** | **a little over one full week, spread over about two** |

The train rule keeps the date honest.

## 4. Decided
- Preview 4 is "Studio, solid". Versions is part of it.
- Chats are stored in Studio's data folder, never in the project.
- RAI's test runs are safe by default (R9). Studio never runs AI-written code with administrator rights, shell access or system changes unless you allow each one.
- Model mistakes are fixed in the language (additive), in the messages, or in the card, never by changing RapidQ behaviour.
