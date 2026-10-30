# Changelog

All notable changes to RapidR are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/). Planned work lives in
[ROADMAP.md](ROADMAP.md); security finding IDs (`SEC-xx`) refer to it.

## [Unreleased]

### Added — Versions in RapidR Studio (lane VERSIONS-G3, Robert 2026-10-29: "our own git", nothing ever lost)
- **The Versions pane** (View > Versions; a tab beside Project, shown by itself for a project that has versions): **Start Versioning This Project** for a project without versions (refused, said in words, for a whole drive, the home folder or its Desktop, Documents or Downloads, and a folder inside another one with versions); then the files **not saved as a version** (M changed, N new, D removed — a letter and words, never colour alone), **Save a Version...**, the **Timeline** (each version's words, who, when; an AI's checkpoints smaller and lighter), **What Changed?**, **Go Back...** (the whole project, or only the file open) and **Undo** with what it would undo. Every plain word carries its Git name in a tooltip ("Git calls this a commit"). Its own menu: File > Versions; the command palette has every command.
- **Save a Version...** (File, **Ctrl+Shift+V** / Cmd+Shift+V — Ctrl+S stays Save): a few words, the files (every changed one ticked), your name (remembered); files open with unsaved changes are saved first. **The check for keys and passwords** (rapidr-vcs' secret scan) is said in the dialog — the file, the line, the secret masked — and the version is saved only once **These aren't real keys or passwords: save them anyway** is ticked.
- **What Changed?** opens in a tab beside the code: a version's files against the one before it (or what changed since the last version), each file before and after with the changed words marked (RDiffView, read only). **Go Back** asks first (Cancel the default), then saves a *new* version with that version's files ("Went back to “first” (abc1234)"; the newer versions stay); open documents follow. **Undo** takes back the last action, refusing — changing nothing — when it would overwrite newer work.
- **The editor's gutter marks against the last version** (RCodeEditor `ChangeBase`, `ChangesSince`, `ChangeCount`): added and changed lines a bar, removed ones a triangle; a click on a mark — or File > Versions > Show This Line's Change — shows what the lines were under them, with **Undo This Change** (one Ctrl+Z step) and Close (Escape); new `ChangeAt`, `ChangeText`, `ShowChange`, `HideChange`, `RevertChange`, `NextChange`, `PreviousChange`, `ChangePeek`. **The Project pane's badges** (RProjectTree `FileBadge`, `ClearBadges`: a letter in a soft status square, said in the row's description to a screen reader). **The status bar:** the project, the experiment ("main"; Git calls it a branch) and "1 change not saved as a version".
- **The local history: every save kept, versioned or not, desktop and web.** Before Studio writes a file it keeps what the disk had (once), and after, what it saved, in the project's `.rapidr/history` (each content once, named by its SHA-256 and checked when read back; the newest 100 copies of each file; nothing over 8 MB). **Local History of This File...** lists the copies, compares one with the editor and **Restore This Copy** puts it back as one undo step. `RRepository.Snapshot(Path, "before-reload")` is the API the coming external-changes work calls before it reloads a file.
- **RRepository** (IDE components, RapidR's): a project's versions and local history for any program `rapidr run` runs — `Folder`, `Available`, `Versioned`, `ReadOnly`, `Experiment`, `ChangeCount`, `Changes`, `Author`, `Email`, `Error`, `ErrorKind`, `Secrets`, `CanUndo`, `HistoryFolder`, `Now`; `Start`, `Refresh`, `SaveVersion`, `CheckSecrets`, `Timeline`, `VersionFiles`, `FileAt`, `HasFile`, `GoBack`, `GoBackFile`, `Undo`, `History`, `Checkpoint`, `Snapshot`, `LocalHistory`, `LocalFiles`, `SnapshotText`. Desktop on rapidr-vcs (gitoxide); in a browser `Available` is False and the versions' methods say why (gitoxide doesn't build for wasm), the local history works. Registry entry with AI facets and an icon (`components/repository`); reference pages regenerated.
- **rapidr-vcs:** `go_back_file` (a new version with one file as it was, the others' unsaved changes untouched, one Undo), `changes_in` (what a version changed against its parent; every file for the first), `write_ignore_file` (RapidR's `.gitignore`: `.rapidr/`, what Build writes; a project's own .gitignore gets `.rapidr/` added, nothing else changed); `.rapidr/` is never part of a version, a safety point or a checkpoint; the go-back version's message says "Went back to “…”", as its history does. The engine's safety semantics are unchanged.
- **RDiffView `ReadOnly`:** what changed, to look at — no Accept or Reject, nothing decided. A diff line's + and − are drawn in the text's colour (the bar beside them carries the hue): they read at 4.5:1 on the tinted line; the high-contrast scheme's added-word mark darkened (white text on it 7:1).
- The manual's [Versions](docs/manual/versions.md) chapter: Studio's Versions step by step with pictures from reproducible scenes (`tools/manual/scenes/versions.mjs`), the keys, the Git names, RRepository. Tests: rapidr-vcs (18, every repository `git fsck --strict`), rapidr-studio's local history and RRepository walk (`git fsck`), the code editor's change base, `tests/studio_flows.mjs` versions-* (the walk in light, dark and high contrast, contrast-judged, screen-reader names; the web's pane and local history).

### Added
- **RAudioOut: sound made sample by sample**, played as the program makes it — an emulator's beeper or sound chip, a synthesizer, a tone for a lesson ([the manual](docs/manual/directx-and-media.md#sound-made-sample-by-sample-raudioout)). `Write` queues 16-bit samples from an INTEGER array, a binary string or an RMemoryStream (with Count and Offset), `WriteFloat` -1 to 1, `WriteSample` one at a time; `SampleRate` (8000–96000), `Channels`, `Bits` (16 or 8), `BufferMs`, `Volume` (by the ear, gliding), `Enabled`, `Playing`, `Queued` (ms), `Underruns`, `Error`; `Start`, `Stop`, `Clear`; `OnNeedData(Count)` on the program's thread when the queue drops under BufferMs. The queue is lock-free (the sound device never waits for the program); a gap plays silence after a 2 ms fade and is counted; Start gives the device 40 ms of silence first, so a Bluetooth headset's burst as it begins never cuts the sound; the device converts the rate. Desktop (interpreted and native) through rodio, the web through Web Audio (after a click, as browsers require: Error says so until then); without a sound device it plays silently, in time. The tests' clocked sink checks exact samples on every runtime. Examples: `media/tone_generator.rr` (with an oscilloscope), `media/beeper.rr` (a one-bit speaker as an emulator plays it: 882 samples a frame from an RTimer), `media/chiptune.rr` (three square-wave voices, RapidR's own tune), and the recipe `recipes/frame_audio.rr` (emulator sound from a level timeline).
### Added — small models succeed with RapidR (RAI2-SMALL-A; from the evaluation of 2026-10-29, `dist/ai-eval-2026-10-29/report.md`)
- **Hints for syntax from other languages, under the compiler's own message.** `SET x = 1`, `LET`, `var`, `a % b`, `x ^= 2`, `a ** b`, `a // b`, `c ? a : b`, `!=`, `!a`, `||`, `for i in range()` / `for (;;)` / `FOR EACH`, `END FOR` / `END WHILE`, `ELSE IF`, `elif`, `ENDIF` / `ENDSUB`, `IF` without `THEN`, a one-line `IF` with its `ELSE` on the next line, `:` after `IF` / `def`, braces, `def` / `function f() {}`, `return`, `;`, `switch`, `break` / `continue`, `#include`, `new`, `s = 'text'`, `str()` / `parseInt()` / `toUpperCase()` … each get one line of plain words with the RapidR form (*RapidR has no SET: write x = 1*). The message stays RC.EXE's; the hint is an additional line, in `rapidr check`, in `rapidr run` / `build`, in Studio's Problems pane and the editor's tooltip, in `rapidr lsp` and in the AI tools' `check_code`. One place for the texts, each with a stable id (`hint.set` …): `crates/rapidr-langsvc/src/foreign.rs`. Manual: [Coming from another language?](docs/manual/language.md#coming-from-another-language). `+=`, `-=`, `*=`, `/=`, `++`, `==` and `print(x)` compile and work (the classic compiler's own): no error, no hint.
- **`rapidr check <file>…`**: what Studio's Problems pane lists, from the command line (exit status 1 on an error).
- **RC.EXE's "Missing NEXT statement" (and WEND, LOOP, END IF, END SELECT).** A FOR, WHILE, DO, IF or SELECT never closed compiled without a word; the classic compiler warns, so now does RapidR (a warning: the program still compiles), with a hint — and for `ELSE IF`, the usual cause, *write ELSEIF*.
- **`&&` and `console.log(…)` compile, so they are warnings with a hint.** The classic compiler takes `&&` as two `&` (text joining: `IF x = 3 && x = 3` is never true); RapidR keeps that and now warns — *RapidR uses AND and OR* — and *use PRINT* for `console.log` / `System.out.println`. For an AI app and RAI, `check_code` reports them (and the Missing … warnings) as errors.
- **Studio's MCP server tells an AI app how to work.** Its instructions are now the recipe-first workflow (`studio_status`, `find_example`, start from the recipe, `check_code` after every write, read the error's hint, the same error twice → start again from the recipe) and a 25-line language card, about 1,000 tokens, tested to stay under 1,200; an empty project's `studio_status` ends *This project is empty: call find_example first*. The open project IS the user's program, said in `create_project`, `open_project`, `write_file` and `propose_edit`.
- **The same refusal twice changes the advice.** After a file's second identical refusal (a syntax error, a cut-off text, an edit that doesn't fit, errors in a proposal) the answer is *Same error again. Stop rewriting the whole file. Call find_example('…') and start from that recipe*, with the nearest recipe's first lines (found with the file's own text).
- **Paths as models write them.** `project/main.rr`, `MyApp/main.rr`, `./main.rr` and the absolute path inside the project are the project's `main.rr` (never outside it; `~/…` is outside); a path that isn't a file says *The project's files: …*. Argument names are lower case (`path` for `open_project` too); the capitalised ones are still taken, by any RMcpServer tool.
- **A change is one clear sentence**: *Waiting for the user to apply the change to main.rr* or *Changed main.rr in Studio's editor (not on disk yet: Run saves it)* — not "Applied" and "Proposed … they decide" together.
- Manual: [Let AI apps work in Studio](docs/manual/ai-apps.md#what-studio-tells-your-ai-app) — what Studio tells an AI app, and a note for building one: retry a reply with neither text nor a tool call.
- **Studio no longer stops answering after a refused new program.** A `create_project` refused because files were unsaved stayed among the waiting changes; after Run saved them, the next `write_file` made that program and opened it over its own unsaved text, and Studio waited on a Save dialog, answering nothing (found by the 2026-10-29 runs, `quiz-game`, on the development build too). The refused program is dropped, and one written while files are unsaved is not opened over them.
- **Measured** (qwen3.5 4B and bonsai-2 2B on this Mac through Ollama, 13 apps, one run each, before → after): an outside AI app with the 4B model passes 4/10 → 7/10 of the apps (requests per app 19.9 → 11.2, seconds 180 → 59), Studio's own RAI 7/10 → 7/10 (and 2/3 of three unseen apps); the 2B model 0/10 → 1/10 either way. Every run now looks at an example first (before: none of the 2B's, 12 of 20 of the 4B's).
### Added — RapidR in your language, the plumbing (I18N-STUDIO; Spanish first, Preview 5)
- **Tools > Settings > General > Language**: System default, English, and each language Studio has (by its own name: *Español*), switched at once — menus, panes, open dialogs, the status bar — and kept; Cancel goes back. `rapidr ide --lang es`, and `?lang=es` on the web page. A Spanish **sample** (`ide/lang/es.po`: the menu bar, part of the File menu, Settings' Language, marked for review, from the approved glossary `docs/i18n/glossary-es.md`) proves it; the rest of Studio shows in English until the translation work fills it.
- **Every text Studio shows goes through `TR$`** (about 1,200 of them in `ide/*.inc`, a sentence with values one `TR$` with `{placeholders}`, counts with `TRN$`), and RapidR's own words — the toolbox, inspector, console, palette, project tree, tool bar, code editor's bars, built-in dialogs — through the runtime's `tr_ui` (about 320, RapidR's own catalog compiled in: `crates/rapidr-value/lang/`). Templates: `ide/lang/studio.pot`, `crates/rapidr-value/lang/runtime.pot`.
- **The compiler's messages in your language**: a catalog with stable IDs (`RQ…` for RC.EXE's own words, `RR…` RapidR's) and English templates (`crates/rapidr-diagnostics`, `lang/compiler.pot`); Problems speaks Studio's language, the CLI `RAPIDR_LANG`'s or the system's; RC.EXE's original stays beside a translation in brackets. **`rapidr check file.rr`** lists a program's errors without running it.
- **Help in your language**: `docs/manual/<language>/<page>.md` shown in the Help pane over the English page (a page not translated yet shows in English, with a note), the registry's docs translated beside it (`crates/rapidr-lang/data/i18n/<language>/*.toml`, keyed `RBUTTON.CAPTION`) in Help, hovers and completion, RAI's manual search in the user's language first. Samples: *Primeros pasos* (the start of Getting started) and four entries.
- **Programs**: captions, hints and accessible names that are a catalog's text whole translate themselves when `Application.Language` changes (before `OnLanguageChange`); `Application.LanguageFolder` (catalogs from another folder), `Application.Languages` (the catalogs' languages) and `Application.LanguageName(tag)` (*Español*, *中文（简体）*); `examples/translations/language_picker.rr`.
- **`rapidr i18n`**: `extract` (a template from `TR$`/`TRN$` and the routines a source names as taking text: `' i18n-keywords: AddCmd:2 AddTopMenu:2@menubar`), `update`, `check` (placeholders, `` `code` ``, one `&`, a menu's `...`, Spanish's `¿`/`¡`, a mnemonic two items of a menu share), `pseudo`, `studio`, `runtime`, `compiler`, `manual-stamp`, `coverage` (Studio's text, compiler messages, manual pages, registry docs, per language). New crate `rapidr-i18n` (RapidR's own PO reader and writer, no dependencies).
- **The pseudo-language `xx`** (`[Šàvé ÿöûŕ ŵöŕķ·····]`, a third longer) and **`tests/studio_lang.mjs`**: Studio's main scenes in it on the desktop and the web — the main window, a project in the designer, Settings, Set Up RAI, the AI apps consent, Find, Problems, the palette — fail on any English left or text cut off; Spanish from the start and switched live and back; a compiler error in Spanish in Problems and on the command line; the Help page in Spanish. Layouts that now fit a longer language: the Welcome page (Start's column, two-line example cards), Find (its buttons, In:, its note), Set Up RAI's texts, the consent dialog's heads.
- [The manual: Studio in your language](docs/manual/studio-languages.md), and [translations.md](docs/manual/translations.md) for programs.

### Added — a program's eyes and hands (Robert, 2026-10-29: an AI connected to Studio "wanted to see how it looked, and had to use the Mac screen")
- **AI apps and RAI see and use the program Studio runs, never the screen.** `app_layout` (its windows and every control's name, type, caption or text, place and size, and what looks wrong: a caption cut off, a word-wrapped label too low for its lines, controls on top of each other, a control past the window's edge), `app_screenshot` (a PNG of one window or of all of them, framed with their title bars, at the screen's scale or 1×), `app_click` / `app_type` / `app_key` / `app_drag` / `app_scroll` (input played through the program's own windows as a person's — by a control's caption, its name, the label beside it, or a point —, shown by a virtual pointer that glides there and a ring at each click; reduced motion: it jumps) and `app_wait`. Each answer says what was done, or what is there instead ("No button shows "Go" in form1. Buttons now: "7", "8" …"); a window waiting for an answer in front, a disabled control and a paused program ("The program is paused in the debugger at line 28 of calc.rr: continue it first") are said, and a paused program can still be looked at. Only the program's own windows, through its own runtime: no Screen Recording or Accessibility permission, nothing else on the computer reachable (a test asks for "Finder" and "RapidR Studio" and is refused). Looking at the program and using it count as running it (a picture or the layout shows whatever the program shows, other people's data too): Read only refuses both, Ask before changes and Allow edits ask once per run (Allow Once covers pictures, clicks and typing until the run ends; a new run asks again; a Deny grants nothing), Allow everything doesn't ask; `app_wait` only lets time pass. The question says what Allow Once covers: "<AI app> wants to look at or use your running program: <what>. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data." Studio's MCP tools are in `ide/mcpapp.inc`, included by `ide/mcp.inc`; the app tools are offered only once the user confirmed the question that names them. **Everyone who turned AI apps on before is asked once more** as Studio starts (the first-time question's version 2: Turn On, or Cancel — AI apps off, nothing started), and a project left in Allow everything is set back to Ask before changes once (its confirmation never said "click and type in your program"). The question's rows, its footer, Allow everything's confirmation and the four modes' notes say the running program in the words of the legal review ("It also gets pictures of Studio's window and of your running program's windows …", "… a real Send, Pay or Delete button does it for real, so use test data", "Studio keeps a list of what they did on this computer only; Clear Activity removes it"); Read only's refusal says the running program can't be looked at or used. **A project an AI app creates or opens keeps the permission of the project open before** (Robert's choice: an AI app working alone isn't stopped by a question the other project didn't ask), kept as that project's own and said in the AI Apps pane and the status bar — "New project: same permission as before (Allow edits)"; a project with a permission of its own keeps it, and with nothing open before it starts with Ask before changes. **Allow everything is never passed on** (it was confirmed for one project, while the user watched): the new project starts with Allow edits, and the pane, the status bar and a screen reader say "New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it." (`tests/studio_mcp.mjs` carry; the legal review's Addendum 2). RAI gets the same tools while your program runs (none when its permission is reading only) and **asks once per run** with a card in its chat — "RAI wants to click and type in your running program": **Allow for this run** / **Don't allow** (for that answer) —, the pane shown and the question said aloud; its turn waits for the user, then for the program's answer, without blocking Studio (`ToolOutcome::later`; `ToolProvider::waits_for_user`: a person's answer has no time limit), the status line saying what it waits for.
- **What is kept, what is shown.** The activity log (`ai-audit.jsonl`) gets the action and "ok" or "error" only — never a picture, what was typed or what the program showed (an error's sentence quotes captions: it goes to the AI app alone). The AI Apps pane, the status line, the screen reader and the question say "Typing 17 characters", never the text. RAI's audit keeps no digest of input played into the program and says "ran in your program" or "not done in your program". A picture's temporary file is removed once handed over, and one a crashed Studio left is removed at the next start once it is a minute old. A program run outside Studio — `rapidr run` without `--session`, a built program — never reads these requests (a test sends them to both). RAI's setup, its Send note and the manual say that, while your program runs, what its windows show goes to the provider. Legal review: `docs/legal/review-2026-10-29-app-eyes.md` (the names, the consent, the log, RAI).
- **The session protocol's `app`** (rapidr-session): a JSON request answered by the program's runtime on its own thread, running or paused, under the debugger or not — on the desktop from the session's reader (`ui/drive.rs`), on the web beside the VM in the run frame (`kernel_web`). RProgramSession's `App(Request)` and `OnApp(Id, Ok, Text, Picture)` (a picture a 0600 file in the temporary folder, a `data:` URL on the web; a program busy in its own code or ended said so). One engine for both hosts: `rapidr_ui_app::{eyes, hands, link, app_components}`; the pointer is the kernel's (`rapidr_ui_kernel::virtual_cursor`, theme tokens).
- **`RWindowCapture`** (RapidR's): `SaveToFile` a picture of one of the program's windows or all of them (`Scale`, `Frame`, `Width`, `Height`), `LayoutReport`, `LayoutJson`, `Problems`, `Windows`, `Error` — desktop, native and web. **`RAppHands`** (RapidR's): `Click`, `DoubleClick`, `RightClick`, `ClickAt`, `Hover`, `TypeText`, `PressKeys`, `Drag`, `Scroll`, `Wait`, `Cancel`, `ShowCursor`, `Speed`, `Busy`, `Pending`, `LastResult`, `OnDone(Id, Ok, Text)` — for tours, demos, lessons and self-tests. Both docs and the manual's "Use it responsibly" box: tell users what is captured, never record or automate without their knowledge, never click "I agree" or "Pay" for someone, test with test data. Examples `examples/gui/snapshot.rr` (saves a picture of itself, finds its own cut-off caption) and `examples/gui/guided_tour.rr` ("Show me" fills a to-do list); manual [A program's eyes and hands](docs/manual/app-eyes-and-hands.md) with pictures from scenes (`tools/manual/scenes/app-eyes.mjs`); icons; registry entries with AI facets.
- Tests: the engine against a real kernel desktop (`rapidr-ui-app` eyes_tests: the report and its problems, pictures at 2× and 1× and framed, no other window, clicks by caption, name, label and point, typing, keys, disabled and modal refusals, reduced motion), the kernel's pointer, `crates/rapidr-cli/tests/app_eyes.rs` (Studio's end of the session against `rapidr run --session`), `tests/web_session.mjs` (the same on the web), `tests/app_eyes.mjs` (an AI app over Studio's MCP in all four modes, each with a canary — text typed, a caption asked for, a caption shown — that must never reach the log or the pane; and RAI with the mock provider: its card denied, then allowed), `crates/rapidr-ai/tests/later_tools.rs` (and its audit line), `crates/rapidr-cli/tests/app_eyes.rs` (requests ignored outside Studio), `tests/app_eyes.mjs` session (Allow for This Session kept across runs, cleared when another project opens) and upgrade (the question once more: Cancel never starts the server, Turn On brings the app tools, Allow everything set back, not asked again), the examples suite.
### One design (DESIGN-GUIDE)
- **RapidR's design guide** ([docs/design-guidelines.md](docs/design-guidelines.md)): how Studio's screens are put together, measured from Studio — principles, colour (theme tokens only), type, spacing (4 · 8 · 12 · 16 · 20 · 24), controls by context (compact 24 px in panes and strips, dialog buttons 32), buttons, chips, pane anatomy, confirmations and consent, icons, motion, keyboard, accessibility, words, and a checklist before merging UI. CONTRIBUTING.md and docs/ide-plan.md point every UI change at it; `tools/check_design.py` (in `tools/regress.sh`) fails on a new colour written as a number, an unknown token or a control height off the scale.
- **Status colours for every program:** `Application.ThemeColor("status.info")`, `status.success`, `status.warning`, `status.danger` — a soft fill, each with `.text` and `.border` — from the brand palette, in light, dark and high contrast (framed in white there), tested at 4.5:1 for text (7:1 in high contrast) and 3:1 for borders. New example [status_colors.rr](examples/gui/status_colors.rr).
- **The AI Apps pane, calm:** its header one row — the title and its line at the left; a soft amber **No questions asked** chip with a warning sign (was a solid brown block), the permission menu and **Stop** all 24 px tall on one centre line; the waiting question's strip with an amber mark and compact buttons, as tall as its words; the pane lays itself out again when resized.
- **Let AI apps use RapidR Studio?** is a question you can read: four rows with icons (what an AI app can see, what it can do, what it never sees, what to keep in mind), how to disconnect, **Learn more**, and **Turn On** / **Cancel** (the default) — instead of a 12-line paragraph in a warning box. Settings > AI Apps shows the same rows, and its Allow everything note is no longer cut off (it reads in the warning colour).
- **Copy from the AI Apps pane:** right-click the activity for **Copy** (Ctrl+C, ⌘C) and **Copy All**, plain text, and **Clear Activity…**; the list shows whole rows (no half row at its top).
- **What AI apps are told they can do, said in full** (the legal review's addendum): the rows name the picture of Studio's window, changing forms and code, opening other projects, running and debugging, and that asking RAI may cost money, each only as the permission mode allows; the footer adds "Share only what you may share"; Allow everything says Studio's own tools never hand over keys or files outside the project. **`capture` refuses while a dialog is open** (Settings, Set Up RAI, a question, a review window — one may show the token or a key), and `studio_status` says which is open. `set_property`'s description reaches AI apps whole (it was cut at `(\` — BASIC strings take no backslash escapes), and a test checks every tool's description.

### Fixed
- **RMIDI is heard on a Mac (and on Linux, and in a browser).** RMIDI sent its song to the first MIDI output the system listed. On Windows that is the GS Wavetable Synth, but a Mac's outputs are buses that carry messages to other programs (an IAC bus, a network session, a virtual port another app made) and Linux always has "Midi Through": the song "played" — the position moved, the volume was set — and nothing sounded. RMIDI now uses an output only when it is known to sound (a synthesizer by its name: Windows' GS Wavetable Synth, FluidSynth, TiMidity …; on Windows, else the first output that isn't a loop or a bus, as before) and otherwise plays the song on RapidR's own synthesizer. An output that can't be opened falls back to the synthesizer too. `RAPIDR_MIDI_OUT` names the output to use (a part of its name — a keyboard, a sound module) or `builtin`; a name nothing has is said on stderr. The desktop and the web share the rule (`rapidr_value::objects::midiout`). Checked without ears: `RAPIDR_TEST_MIDI=capture:<prefix>` records where a song goes and renders the synthesizer to `<prefix>.wav` (`tests/midi_output.mjs`), and a real run against a virtual CoreMIDI destination named like a synthesizer received the song (and one named like a bus received nothing). Example: `examples/media/midi_compose.rr` writes a song in code (chords, bass, channel-10 drums) and plays it.
- **RUpDown's arrows now change what you see.** An RUpDown fired OnClick but never OnChange — the event the designer gives it and the registry names as its default — and had no way to show its number, so an edit beside it stayed as it was. OnChange now fires when an arrow (or the Up and Down keys) changes Position. New, RapidR's: `Associate` (an edit or a label that shows the number and follows it, and the arrows count on from a number typed there — RapidQ's QUpDown.Associate), `Increment` (the size of a step) and `Wrap` (go round from Max to Min), on the desktop and in the browser. Example: `examples/gui/updown.rr`.
- **Two registry reads answer what the registry says:** a component's `AccessibleHidden` reads 0 (False) until it is set (it read an empty string, on the desktop, the interpreter and the web), and `RAI.SummaryAt` reads 0 while it is automatic (it read the effective limit; `ContextLimit` is the point the chat is compacted at). The language conformance suite is green again.
### Added
- **RAI and audio icons** (`actions/`, monochrome, 16/24/32 px): plan, ask a question, vision (an eye in a frame), record the window, search the web, agent working, tool (a wrench), local model (a chip), on this device, microphone, speaker on and speaker off. The four-pointed sparkle marks what the AI does; the brand's gradient stays on `actions/ai` alone.
- **Versions icons** (RapidR's own set, `actions/`, monochrome, 16/24/32 px): save version, version history (timeline), branch, merge, compare versions, line history (who changed it), push, pull, sync, merge conflict, restore version and pull request — rings on lines, in the set's one weight, no product's mark. For Studio's Versions (docs/versions-plan.md) and for RAssets.
- **A translation icon, and nine more file-type icons, in RapidR's own icon set** (`files/translation`, `xml`, `log`, `pdf`, `svg`, `midi`, `model`, `certificate`, `key`, `env`; each hinted at 16, 24 and 32 px, in every theme). `.po` and `.pot` files show a speech bubble with swap arrows in text-amber in the Project pane, tabs, Search results and file lists (`lang/es.po` was the plain file icon); `.svg` has its own vector icon (it was the image's), `.mid`/`.midi` a keyboard (they were the audio's), and `.xml`, `.log`, `.pdf`, 3D models, certificates, keys and `.env` files each have theirs. Studio's new golden `project-translations` guards the Project pane of a program with a `lang` folder. Part of the icon set RAssets will offer to users' apps.
- **Chinese (and every character above Latin-1) no longer turns into `?` in a program's files and on its canvases** (Robert's Solar System 3D: with Chinese chosen, every caption and the 3D view's labels showed `??`). Three faults in a chain, each fixed: (1) `RFileStream.WriteStr` mixed Latin-1 and UTF-8 in one string and its length argument cut the UTF-8 tail (Studio's Apply wrote RAI's new files with `WriteStr(Text, LEN(Text))`): a string with any character above 255 is now written whole as UTF-8, a string of characters up to 255 stays bytes (binary data is exact), and the count is of characters; (2) a Studio editor opens a file that isn't UTF-8 as Windows-1252, and its save wrote `?` for every character that code page lacks — `RCodeEdit`, the text boxes (`RMemo`, `REdit`, `RRichEdit` plain text) and the designer's saves now write UTF-8 instead, never a `?`; (3) `RCanvas.TextOut` drew a `?` for each character RapidR's built-in fonts lack — it now draws it with the font a label uses (the system's CJK font on the desktop, the Noto fonts on the web), from the program's first line. TR$ catalogs, lists, grids, `LINE INPUT #` and text boxes also read a file that isn't UTF-8 as Windows-1252, so an `é` is never turned into `�`. New: `WriteText` / `ReadAllText` on both streams (UTF-8 text on purpose). A `.po` file the old Studio already wrote with `?` can't be recovered: write it again. Tests: stream, editor, text box, catalog and encoder unit tests; conformance `stream_unicode_text`, `i18n_encodings`; `tests/native_gui_unicode.mjs`. Manual: [Text and bytes](docs/manual/language.md#text-and-bytes-accents-chinese-emoji); example `examples/basics/unicode.rr`.
### Studio keeps your work (Robert's evenings with RAI on 2.121.2 / 2.121.3)
- **A mistake in Studio's own code never closes Studio.** A component's error in a running RapidR program ("Cannot create file icons/folder.svg.") ended the whole process under the interpreter; now it is the program's run-time error at its line (`rapidr_value::host_call`, as on the web), and an event loop goes on after it, as for any handler error. Studio hears it (`Application.OnError`) and shows *Something went wrong in Studio* — what happened, where in Studio's code, *your work is safe* — with **Copy Details**; it is written to Studio's log (`studio-errors.log`, RPROJECT.LogFile) and the Output, and Studio goes on with your documents and the chat as they were. Flow `studio-error-safe`; picture in the manual.
- **Unsaved work survives a crash:** a copy of each document with unsaved changes is kept in Studio's own folder (every 20 seconds, and the moment something goes wrong); a Studio that ended without closing asks at the next start **Restore your unsaved work?** (Restore / Discard / Not Now); a clean exit removes the copies (RPROJECT.Recovery…). Flow `studio-restore-unsaved`.
- **RAI's Apply is all or nothing:** every change checked first (a new file's place, a changed file still as RAI read it), then new files written (folders made, never over a file, written beside and renamed), then the editors' changes; anything failing takes back what that Apply did and says why, the changes still waiting (RPROJECT.CanWriteNewFile, WriteNewFile, ApplyBegin, ApplyUndo). New files are UTF-8 exactly as proposed: RFileStream.WriteStr had written a byte a character (`lang/zh-Hans.po`: "??? 3D"); Project > Add Form / Add Module writes the same way now. Flows `ai-apply-all-or-nothing`, `ai-apply-utf8`.
- **RAI's waiting changes are kept with the chat:** after a restart the project's chat comes back with Review the Changes… and Apply still works ("can't apply again"); another project's are never left to apply here. Flow `ai-proposals-kept`.
- **A project as RAI left ShopInvoices:** `create_project` refuses a main file it names and doesn't send (a project cut short by the length limit) instead of making the first module the main file (main = lang.rr while main.rr includes it); a module named as the main file gives way to the file that includes it; on the web it never writes into a folder with any of its files. Opening a project whose main file another file includes says so (RPROJECT.MainIncludedBy). Run always says why it didn't start (nothing open, already running, no main file, the errors in Problems) and notes a main file that ran and ended at once with no window. Flows `project-main-included`, `project-main-runs-module`, `project-main-fixed`, `run-nothing-open`.
- **The designer never shows another file's form:** a design surface used again for another file whose code had errors kept the last file's form on show ("read-only until fixed") — Beat Maker's form over ShopInvoices' code. Unit test.
- **The project tree's selection shows its file:** its tab to the front, or a preview tab the next selected file takes over while unchanged; the keys stay in the tree (es.po selected showed pt-BR.po's text). Flow `project-tree-preview`.

### Added
- **`Application.OnError(Message, File, Line)`** (RapidR's): a program hears each run-time error that ended one of its event handlers (the window keeps working, as before), to tell its user, log it or save their work; never for an error in that handler itself. Interpreter, native builds (no file or line there) and the web. [examples/gui/on_error.rr](examples/gui/on_error.rr); manual: the language page's *When a handler fails*.

### Fixed
- **The code editor's scroll bars no longer flicker** after a file opens (both bars on and off at every repaint, faster as the mouse moved, until the first scroll): the widest line is measured over the same rows whether the horizontal bar shows or not, and the text's area follows a bar's change at once. Unit test over 24 places of a long line, 8 repaints with mouse moves.
- **macOS: a context menu opens where it was asked for** (the code editor's opened far from the click: muda measures a view's y from its bottom, winit's view is flipped). Flow `code-menu-at-click` (the kernel's menu, desktop and web) and a unit test of the flip.
- **RAI's context meter knows Claude's windows before the models list is fetched:** claude-opus-5-5 said "121,309 of 40,000 (100%)" (no window known, so the 40,000 fallback) while Sonnet said "of 700,000". RAI now knows each Claude model's window (`rapidr_ai::provider::ANTHROPIC_WINDOWS`: 1,000,000 for the current models, 200,000 for Haiku 4.5 and any Claude it doesn't know yet), so the chat is summarised at 70% of it; the provider's own list still wins once fetched. Test: every model in the table has a window.
### Added
- **The VS Code extension finds a `rapidr` new enough** (the 2.121.3 walk: an ancient `rapidr` 0.1.0 first on `PATH` was used and nothing worked, silently): every candidate is asked its version, one too old for the language server is skipped and said so, an older one that has it is used with a warning, and the notification offers *Use the installed RapidR* / *Choose…* / *Download*. RapidR is the default formatter for RapidR files. New in the extension: RapidR Help beside the code (Shift+F1), parameter names, colour swatches and picker, the list of types after `AS `, project commands for `.rrproj`, `$INCLUDE` links; the grammar's word lists are generated from the registry and a test checks them, and so are the snippets (docs/manual/other-editors.md, `utilities/vscodeext/rapidr/CHANGELOG.md`).
- **`rapidr lsp`:** `textDocument/inlayHint` (the parameter's name before a literal argument, Studio's chips), `documentColor` / `colorPresentation`, `documentHighlight`, and RapidR's own `rapidr/helpPage` and `rapidr/helpSuggest` (the pages Studio's Help pane shows: `rapidr_langsvc::help`, moved out of the Studio crate and shared). **`rapidr dap`** advertises conditional breakpoints, hit counts and logpoints, which the session already had.

### Fixed
- Hover on a type after `AS` says the type (`AS STRING` said `STRING$`); the Help for a property set in a CREATE body (`Caption = "Hi"`) is its component's.
### Added
- **Let AI apps work in RapidR Studio** (desktop; [manual](docs/manual/ai-apps.md)): Studio can be an MCP server, so an AI app on this computer can read the open project, check code, propose and make changes, run and debug the program, look at the forms' layout and a picture of Studio, and ask RAI — while you watch every call in the new **AI Apps** pane. Off by default (Tools > AI Apps Settings…: a first-time confirmation of what AI apps can see), on `127.0.0.1` only, a token from the keychain on every request (Copy Address, Copy Header, Reset Token), every call in the activity log (never a key, the token or a file's text; Clear Activity…). **One permission menu per project** — Read only, Ask before changes (the default: Allow Once, Allow for This Session, Deny, Review… in the pane), Allow edits, Allow everything (an amber "No questions asked" badge and "AI apps: allow everything" in the status bar, confirmed when chosen) — and Stop. The tools are RAI's own plus `studio_status`, `list_documents`, `get_problems`, `read_output`, `open_project`, `run_program`, `debug`, `set_property`, `layout_report`, `capture`, `ask_rai`, `get_rai_steps`, `get_rai_transcript`, `get_rai_audit`. Studio in the browser can't listen for AI apps (said in the manual; WebMCP later).
- **RMcpServer**: your program's own tools for AI apps ([manual](docs/manual/network.md#rmcpserver-your-programs-tools-for-ai-apps), example `network/mcp_tools.rr`). The component speaks the protocol itself — the current revision (2026-07-28: no session, its headers checked against the body, `server/discover`) and the initialize-based ones (2025-11-25, 2025-06-18, 2025-03-26) — on 127.0.0.1 with a token it won't start without; tools added with their arguments written as a SUB's parameters (`Text AS STRING, [Count AS INTEGER]`) or a JSON Schema, checked before the program sees a call (a wrong call is told what the tool needs); `OnToolCall` answered now or later (`Reply`, `ReplyImage`, `ReplyJson`, `Progress`); resources; `Clients`, `OnConnect`, `OnDisconnect`, `Disconnect`. RHttpServer's listener and checks are shared (one implementation). Desktop only.
- **`Form.SaveImage(FileName)`** (RapidR's): what a form shows now, saved as PNG (BMP by the name) — the program's own windows only. Desktop for now.
- **RAIProjectTools** for an outside AI app: `ToolNames`, `ToolDescription`, `ToolSchema`, `ToolKind`, `ToolActivity`, `CallTool` (no hidden files, nothing past a home folder's listed files), `CallFailed` / `CallNote` / `CallTarget` / `CallRisk`, `LayoutReport`, `AuditCall`, `AuditCalls`, `ClearAudit`, `AuditLocation`.
### Added
- **Copy anything in Output, Build and Problems** (Robert: "There is no way to copy an error, or a message"): drag to select text (across lines, scrolling past the edges), double-click a word, triple-click a line, Shift + a click to extend, then Ctrl+C / ⌘C (Ctrl+Insert) — plain text, no colours, a wrapped line as one line; Ctrl+A selects the page. The pane's right-click menu is now **Copy**, **Copy Line** (the line right-clicked), **Copy All** (**Copy All Problems**), **Select All**, then Wrap Long Lines and Clear; the status bar says what was copied. Problems' rows (several with Shift) copy as `file:line:col: severity: message`. Edit ▸ Copy / Select All work on the pane while it has the keyboard. A link still opens on a click; a drag that starts on it selects. Screen readers read the pane by character, word and line and hear the selection. `ROutputConsole` gains `SelText`, `HasSelection`, `ClickedLine`, `SelectAll`, `CopySelection`, `CopyAll`, `CopyLine` (desktop and web; example `examples/studio/log_copy.rr`; manual: debugging.md, "Copying from Output, Build and Problems").
- **Call Stack and Breakpoints copy:** Ctrl+C (⌘C) and a right-click menu (Copy, Copy All).
- **Ctrl+C (⌘C) on a message box** (SHOWMESSAGE, MESSAGEDLG, MESSAGEBOX) copies its title, text and buttons, as Windows' message boxes do.
- Test hook `comp.__rightclick_x_y`: a right click through the kernel (the context menu opens).
### Added
- **Archives in your programs: `RArchive`.** Open zip, 7z, tar, tar.gz, .gz, .xz and .zst files — the format is found from the first bytes, never the name — list what they hold (`Count`, `Name`, `Size`, `CompressedSize`, `IsFolder`, `IsEncrypted`, `DateTime`, `Find`) and unpack it: `Extract` (the bytes as a string), `ExtractText`, `ExtractToStream`, `ExtractToFile`, `ExtractAll`. Make zip, tar, tar.gz and .gz files with `Create` / `CreateStream`, `AddFile`, `AddText`, `AddData`, `AddStream`, `AddFolder` and `Close` (`Level` 0-9). An archive opens from a file, a stream, a string of bytes (`OpenData`: a download) or a `$RESOURCE`. `FormatOf(FileName)` says what a file is without opening it, so an emulator takes `game.z80` or `game.zip` alike. Every call says why it failed in `Error`. The same on the desktop, native builds and the web. Example `examples/data/archives.rr`, manual page "Archives: zip, 7z, tar and gzip files".
- **Safe with archives from anywhere:** `MaxSize` (1 GB; 256 MB in a browser) and `MaxRatio` (1000 to 1 past 1 MB) stop zip bombs by the bytes really unpacked; `ExtractAll` checks every name and unpacks everything before writing, so a name with `..`, an absolute path, a drive or a device writes nothing; links and devices are never extracted, nothing is written through a link already in the folder; encrypted entries and RAR files are refused with a clear message. The readers were fed 300,000 damaged archives without a crash.
- **`COMPRESS$` and `DECOMPRESS$`:** pack and unpack strings of bytes — gzip, zlib and raw Deflate both ways, xz and zstd unpacked — with a size limit (256 MB by default) on unpacking.
- A native build carries the archive code only when the program uses `RArchive`, `COMPRESS$` or `DECOMPRESS$` (a first step of lean builds).
### Added
- **Ten "systems" recipes for the assistant and for you** (`examples/recipes/`, [Recipes for systems programs](docs/manual/recipes-systems.md)): memory as a `BYTE` array (words, wrapping addresses), bits and flags (an 8-bit ALU with Carry, Zero, Sign, Overflow, Half-carry), a tiny CPU (fetch, decode, execute; Step / Run), the stack (`PUSH`, `POP`, `CALL`, `RET`), a hex dump, a disassembler (an opcode table from `DATA`), a binary file into memory with its header parsed, video memory drawn on an `RCanvas` (bitmap and colour attributes, only the changed cells redrawn), a keyboard matrix with an `IN` port, and a CPU run in cycles per timer frame. One tiny 8-bit computer of our own ties them together. Each is click-tested on every runtime, and Studio's recipe search finds them in English, Spanish and Portuguese ("emulador cpu registros" finds the CPU). They are in Studio's Open an Example list and the assistant's manual search; `tools/bench/systems_loops.rr` times their main loops (a CPU step loop runs about 4 million emulated cycles a second in the RapidR Runtime, about 40 million native).
### Changed
- **Faster strings and file streams, on every runtime (`rapidr run`, native, web).** Reading a 64 KB binary file's string a byte at a time with `ASC(MID$(b, i, 1))` took 8.2 s with `rapidr run` (18 s native, 13 s in a browser); now 13 ms (7 ms native, 23 ms in a browser). Strings are shared instead of copied each time a variable is read, and a long string gets an index from character positions to bytes the first time one is asked for, so `MID$`, `LEFT$`, `RIGHT$`, `LEN`, `INSTR`, `RINSTR`, `ASC`, `INSERT$`, `DELETE$` and `s[i]` cost what they return, not the string's length. `s = s + x` and `s = s & x` grow the string in place (262,144 one-character appends: 1,029 → 79 ms). An `RFileStream` keeps its writes and writes them together — at Close, every 64 KB, when the program waits or ends, and before anything else uses the file by its name — so writing a 64 KB file a byte at a time twice (`WriteNum(x, 1)`, then `WriteByte`) takes 0.15 s, not 2.0 s, and whatever reads the file still sees every byte; `SaveArray` / `LoadArray` of a numeric array are one call (65,536 elements each: 1,617 → 8 ms). The same answers as before: the conformance suite, three new cases (`string_long_positions`, `filestream_buffered_writes`, `stream_arrays_bulk`) and a randomized test against the character-by-character walk. The benchmarks are in `tools/bench/vm/` (`vm_bench`, `tools/bench/web_bench.mjs`; numbers in tools/bench/README.md), a test fails if reading or building a long string turns quadratic again, and the new example `examples/basics/bytes.rr` reads a binary file's bytes.

## [2.121.3] — 2026-10-25 (hotfix)

### Fixed
- **Applying RAI's new files in a new folder no longer closes Studio** (`icons/folder.svg` with no `icons` folder yet: a run-time error "Cannot create file" ended Studio). The folder is made first; flow `ai-apply-new-folder`.
- **Long chats are summarised at 70% of the model's window** (RAI.SummaryAt now defaults to 0, automatic): the old default of 40,000 was set on every RAI, so the chat was still summarised at 40,000 tokens even on a 1,000,000-token model.

### Changed
- Help, Studio, the examples and the manual describe RapidR on its own; classic compatibility is on one page.
### Added
- **Project properties in Studio:** select the project's name at the top of the Project pane and the Properties pane shows the project (Theme, Name, MainFile, Targets, Arguments, the designer's Grid, Snap and Guides). Theme is the `$THEME` line of the main file, changed as one undo step in its editor, and the form designer draws the program in that look at once, whatever look Studio itself has (RPROJECT: RunArgs, DesignerGrid, DesignerSnap, DesignerGuides, ThemeOf, ThemeEdit; RPROJECTTREE.SelectedKind; RPROPERTYINSPECTOR.SetHeading; RDESIGNSURFACE.ShownTheme).

## [2.121.2] — 2026-10-25 (RAI finishes the job, and shows its work)

### RAI finishes the job
- **Longer answers:** for cloud models RAI asks for up to 32,000 tokens per answer (a quarter of the model's window, up to the provider's limit), not a fixed 8,192; Studio's default MaxTokens is 32,000. A model that allows less says so, and RAI asks again once with that model's own limit (`reply_limit_from_error`).
- **It goes on by itself:** an answer cut at its length limit no longer ends the turn. RAI continues the same task up to 4 times, asked to write in smaller parts, and the status says "Continuing (part 2)…".
- **Summaries fit the model and keep your request:** the conversation is compacted at 70% of the model's window (SummaryAt still sets a lower point), and the user's request is kept word for word next to the summary (a 250-word summary had cut a long request short); the summary may be 400 words and says what's done and what's left.

### RAI shows its work
Robert built an app with RAI and saw only a seconds counter for minutes: no idea what it was doing, then it stopped. Now the chat shows the work, as it happens (docs/manual/ai-assistant.md, docs/rai2-plan.md §4).
- **A live strip under the conversation while an answer works**, where the answer will appear, calm and each thing said once: one row — three small dots (the theme's accent at a gentle strength, a slow 1.2 s wave; still when the system asks for reduced motion), what RAI does now in words (*Thinking…*, *Reading the help of RListBox…*, *Looking at the form…*, *Writing main.rr (+318 lines)…* with the lines counted as they stream in, *Checking (compiling)…*, *Fixing 2 problems…*, *Continuing (part 2)…*, *Summarising the chat to keep it short…*), the seconds at the right, muted — with the steps done above it (✓, one a line, wrapped, the older ones counted), and the tokens of this answer so far in the token meter's line (*This answer: 12,340 in (11,900 cached) · 1,204 out*; the session's totals again when it is done). The status line hides while the strip shows, and the dots are skipped by screen readers, which hear each new step once, politely (never the seconds). **Never a silent wait:** after 10 s with nothing new it says *Waiting for Claude's answer… 24 s* (seen on a real Haiku run). The chat no longer says "Thinking…" itself, and a chat at its end now stays at its end when the strip appears (it didn't: the end of a long answer was out of sight).
- **The chat's cards read as one design** (Robert's look at a live run): the step icon of the status list (✓) first, the headline, then groups of their own apart by wide spaces — *✓ Proposed a change to hello_form.rr  +11 −0  Review*, *✓ Created Time List  main.rr · compiles  Open · Undo* —; the counts are in the diff's green and red (made readable on the chat's ground in every look, 7:1 in high contrast), in the code font so the digits line up, the signs kept so colour is never alone; a screen reader hears "Proposed a change to hello_form.rr, 11 lines added, 0 lines removed, Review". RMarkdownView draws `<ins>` / `<del>` text this way (a `title` is what is spoken).
- **After the answer, its steps are one line** under its name — *✓ 6 steps · show* — that opens the list and hides it again (kept with the chat).
- **The length limit, visible and calm:** when an answer reaches its limit and RAI goes on by itself, the chat shows one muted line in order — *Answer was long: continuing (part 2 of up to 5)* — (a code block the cut left open is closed, so the lines after it are never code), and a tool call cut off is never carried out. If it finally stops, it says why in plain words and offers **Continue in smaller steps**, which sends that request.
- **A smart chat.** RapidR code in an answer is coloured as the editor colours it — fenced blocks (and a block with no language that looks like RapidR's) and `inline code` too. **Hover** a keyword, a component, a builtin or a member and its one-line help shows (*PRINT — Statement: Writes text…*); the pointer is a hand; **click** opens it in the Help pane. Every code block has **Copy** and, for RapidR code, **Insert** (typed into the open editor at the caret, one undo step).
- **Settings say what happens:** *Longest answer* (32,000 tokens by default, "a part, up to the model's limit"), and in words that an answer past it goes on by itself in up to 5 parts and that the chat is summarised, your request kept word for word, at 70% of the model's window.
- **Found by running it for real (Haiku):** a text with a … or an accent in the middle of a tool call's arguments panicked Studio (the lines-so-far counter cut a letter in half) — fixed, with a test; and a long answer cut at the limit five parts in a row now ends with the plain stop and the way on, the chat scrolled to it.
- **For programs:** `AccessibleHidden` (decoration a screen reader skips), RMarkdownView has `CodeLanguage` (inline code coloured; words answer the mouse) and `OnCodeClick(Word, OfType)`; RAI has `TurnInputTokens`, `TurnCachedTokens`, `TurnOutputTokens`, `Part`, `MaxParts`, `ToggleSteps`, `CodeBlock`; `Application.ReducedMotion` says what the user asked the system for (macOS Reduce Motion, Windows' animations off, the browser's `prefers-reduced-motion`) — and RapidR's own animations (the designer's drop, the dock's flyout) now follow it too.

### Fixed
- **Web Studio: running a program no longer stops with "RapidR panicked" messages.** When a program ended (or you pressed Stop) Studio stopped its window and the page answered by calling straight back into Studio while Studio was still busy with that very run; that is a panic ("RefCell already (mutably) borrowed"), and after one the page stayed broken (a frozen Studio, more panics). The page now answers after Studio is done, and anything the program's window tells Studio while Studio is busy waits its turn (in order, nothing dropped). Regression test: `tests/studio_run_panic.mjs` (the program run and ended 20 times, then the page calling back from every call Studio makes to it).

### Windows 7 / 8 (development: not in a release yet)
- **DLL calls in the 32-bit build (Windows 7 / 8) pass each argument at its own size**: a LONG or a pointer 4 bytes, a SINGLE 4, a DOUBLE 8, on the stack as a 32-bit stdcall function expects (`crates/rapidr-runtime-core/src/ffi.rs`, `call_x86`). The 64-bit tables passed 8 bytes for every argument, so on 32-bit Windows the function read them shifted and removed the wrong amount from the stack: the first DLL call with arguments crashed (`midiOutGetDevCapsA` in `midi_sysex_out.rr`: an access violation, heap corruption, "capacity overflow"). Checked in the Windows 7 VM: MulDiv, lstrlenA, GetComputerNameA (BYREF), VarR8FromI4 / VarI4FromR8 / VarI4FromR4 (DOUBLE and SINGLE), 2000 calls in a row, and the whole SysEx sequence (prepare, send, MHDR_DONE, unprepare, close: all 0). The alpha's tests now include `dll_calls.rr` (step 3b). `tools/win7/vm.sh exec` waits for the command and prints its output (the guest agent now runs in the test VM).
- `examples/windows/midi_sysex_out.rr` works in a 32-bit program too (the Windows 7 / 8 build): it asks Windows which it is (`PROCESSOR_ARCHITECTURE` is `x86` in a 32-bit program) and uses a 32-bit `MIDIHDR32` of 16 LONGs (64 bytes) there, the 64-bit header of 120 bytes elsewhere. Before, the 64-bit header in a 32-bit program put the pointer's high half in `dwBufferLength`, and winmm wrote past the buffer (Robert's run of Alpha 2 on Windows 7).
- **Windows 7 / 8 / 8.1 (32-bit) alpha download** (`tools/win7/alpha.sh`, not part of 2.121.0's release): one zip a person unzips and runs, `RapidR-2.121.0-win7-x86-alpha1.zip` (+ `.sha256`): `rapidr.exe`, `RapidR-Studio.bat`, Studio's files, the examples, the smoke tests, `README-ALPHA.txt` (English and Spanish: what it is, what works, the known limits, how to report) and every licence text. `rapidr.exe` is linked with version information (Ruta Internet S.R.L., RapidR, 2.121.0, "RapidR (Windows 7 alpha)") and a manifest (`crates/rapidr-cli/build.rs`, `llvm-windres`), as native Windows x64 programs are, so an antivirus no longer takes the resource-less program for a threat. The cube root (`cbrt`, `cbrtf`) is the program's own (`crates/rapidr-cli/src/cbrt.rs`): the msvcrt build would otherwise link mingw-w64's Cephes-derived one. `tools/win7/link_audit.sh` traces every toolchain object in the program to its source and licence (0 flagged; nothing GPL or LGPL, no winpthreads, no libgcc); the review is `docs/legal/review-2026-10-23-win7-alpha.md`, the mingw-w64 and LLVM licence texts are in `tools/win7/licenses/`. `imports.py --against older.exe` lists the imports new since an .exe known to start on Windows 7.
- `tools/win7/`: `rapidr.exe` builds from the Mac for the Windows 7 targets (`i686-win7-windows-gnu`, `x86_64-win7-windows-gnu`; nightly Rust + LLVM-MinGW, msvcrt flavour): `build.sh`, `imports.py` (what the .exe needs at start-up; flags what Windows 7 lacks), `stage.sh` (the test drop and its disc image), `serve.sh` (host-only, read-only), `tests/` (console and window smoke tests). The 32-bit system-tray code compiles (`NOTIFYICONDATAW` is packed there). The six functions only Windows 8 and later has (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`, `CoIncrementMTAUsage`, `CoCreateFreeThreadedMarshaler`, `CoTaskMemFree`) are RapidR's own small functions on those builds (`crates/rapidr-cli/src/win7.rs`: the real one is found on first use, or a stand-in answers "not available"), so Windows 7 starts the program; nothing is delay-loaded and no delay-load helper from mingw-w64 is linked (`tools/win7/clean-room.md`). The alpha zip now also carries the full `THIRD-PARTY-NOTICES.txt` (made by `rapidr notices`), mingw-w64's `AUTHORS`, and a README-ALPHA.txt with the as-is, out-of-support, trademark and support sentences.

## [2.121.1] — 2026-10-24 (hotfix)

What Robert found using 2.121.0, fixed.

### Fixed
- **The Help pane (F1) is a page you can use.** It used to be a box that looked like an input
  (focused, empty, nothing typed) over a text that its own box covered. Now a real **search box**
  ("Search the help", focus goes there when you press F1) looks up the word as you type, after a short
  pause, with the names it may mean under it (Up and Down choose, Enter opens, Escape goes back to the
  code); below it **one page that scrolls**: the title, what it is (wrapping, never cut off), the syntax as
  code with **Insert into the code** under it (only when there is a code editor to take it), what it does,
  and for a component its **Properties, Methods and Events**, each name a link to its own entry. F1 on
  **your own names** says what they are: `PickedFont` — a variable in this program, its declaration line,
  where it is declared (a link that jumps there), how often it is used, **Find References**, and the help
  of its type (RFont) below; a SUB or FUNCTION shows its signature. F1 on a property after a dot
  (`Hello.Font.Color`) is the entry of the thing before the dot (RFont's Color). With no word, or a word
  nothing knows, the page is a **start page** (how to use Help and a few places to start) or "No entry
  for X" with the closest names. Light, dark and high contrast; keyboard and screen reader.
- **The Search tab's results have room**: rows 24 px high in RapidR's look (16 in the classic one), in the
  chrome's font, as the other trees.
- **A colour of a sub-object gets its swatch and its colour editor** (Ctrl+.): `Font.Color = &H800000` in a
  CREATE body, as `Color = …` and `Hello.Font.Color = …` already had; any path of dots there works.
- **Every pane tab has its icon**: the Assistant (a spark) and Help (a question mark) had none.
- **Help > About says the version of the build**, not a number written in Studio's source
  (`RLanguageService.Version`, new, read-only); a test keeps it equal to the workspace's.

### Added
- `REdit.Placeholder` (RapidR's own, additive): the grey line in an empty text box ("Search the help").
- `RLanguageService.HelpPage` and `HelpFind` (the Help pane's page and suggestions, Markdown) and `Version`.
- **`examples/windows/midi_sysex_out.rr`: a MIDI SysEx message sent with
  winmm** (Windows; on other systems and the web it says so and Send stays
  off). Pick the MIDI output, write the bytes in hex (the GS Reset is
  there), Send: `midiOutOpen`, `midiOutPrepareHeader`, `midiOutLongMsg`, a
  wait for `MHDR_DONE` with a timeout, `midiOutUnprepareHeader`,
  `midiOutClose`, each result code listed in Windows' words. The `MIDIHDR`
  is laid out for 64-bit Windows by hand — 120 bytes: each pointer two
  LONGs, the padding as fields, `dwReserved` an inline array field — and
  the calls say `SIZEOF(hdr)`: 64 for a 32-bit header's 16 LONGs, 120 for
  this one, on the interpreter and native, in a DLL call's argument list
  too (conformance case `sizeof_type_argument`, expected output RC.EXE's).
  Run on Windows 11 ARM64 and x64: every code 0 and `MHDR_DONE` set.
  RC.EXE counts a TYPE's array field as one 4-byte slot (its `SIZEOF` of a
  32-bit MIDIHDR with `dwReserved(7) AS LONG` is 36; RapidR's is 64):
  docs/rapidq-ground-truth.md, ROADMAP.

## [2.121.0] — 2026-10-23 (Preview 4: Studio, solid)

### Legal
- **2.118.0–2.120.0 contained a diff routine derived from GNU diff (GPL-3.0-or-later)**, contrary to LEGAL.md's promise that programs contain no copyleft code. It is removed: RDiffView now uses RapidR's own diff, written from Myers' 1986 paper. **If you shipped a program built with 2.118.0, 2.119.0 or 2.120.0, rebuild it with this version at your next update.** Details: docs/legal/review-2026-10-22-gpl-diff.md.

### Security
- **`cargo deny check` (advisories, licenses, bans, sources) and `cargo audit`
  pass again.** `zip` 0.6.6 → 9.0.5 (MIT; the web bundle's and the build
  server's zip writers, which only write zips: RUSTSEC-2026-0367/0368/0369 are
  in `extract()`, which RapidR never calls, but 0.6 is gone), with deflate
  from zlib-rs (already in the tree); the one new crate is `typed-path`
  0.12.4 (MIT OR Apache-2.0, zip's path type). A bundle is still
  byte-for-byte reproducible (two builds, one hash) and opens with `unzip -t`. `rustls` 0.23.45 → 0.23.46 and `rustls-webpki`
  0.103.15 → 0.103.16 in `Cargo.lock` (RUSTSEC-2026-0346 to 0349 and 0382:
  they are listed there only through an optional feature of `reqwest` that no
  build turns on, and `deny.toml` bans them, so no program ever carried
  them). `zbus_names`, `zvariant` and `zvariant_derive` (RUSTSEC-2026-0370 to
  0372: unmaintained since zbus 6 absorbed them, no vulnerability, Linux
  D-Bus only: the screen-reader bridge, the tray and the keyring) are
  justified exceptions in `deny.toml` and `.cargo/audit.toml`, with when to
  revisit. `tests/security/supply_chain.mjs` pins the new versions;
  `tools/regress.sh legal` runs all four deny checks, `cargo audit` and the
  secret scan (`RAPIDR_OFFLINE=1` reads the advisory database already on
  disk).
- **Fuzz tests for what reads files and patterns from other people.** A
  seeded generator (no new dependency) feeds the lexer, parser and
  preprocessor with edited programs, random soup of the language's words and
  random bytes; the `.rrproj` reader, the web IDE v1 JSON import and the
  workspace file with edited and random text (and every imported file name is
  checked to stay inside the project's folder); and Find's regular
  expressions with random and expensive patterns (a time budget each).
  They run in the normal test run (each under 10 s);
  `RAPIDR_FUZZ_ITERS=200000` (best with `--release`) is the long run, and
  `RAPIDR_FUZZ_SEED` replays a failure. 200,000 inputs of each ran clean.
- **A secret scan of the tree:** `tools/check_secrets.py` looks in every
  tracked file for private keys and certificates (PEM blocks with a body),
  minisign secret keys, key and keystore files by name, and tokens (JSON Web
  Tokens, Stripe, npm, Hugging Face, SendGrid, Twilio, Slack webhooks,
  passwords in URLs), and prints the file, line and kind only, never the
  value. Nothing found.
- **Security audit of Preview 4's new surfaces** (RAI chats and tools, the
  keystore and RSecrets, RHttpServer and the socket modes, MySQL TLS, Studio's
  Search/Replace/Problems and the set-value editors, the test hooks, the web
  bundle and native codegen). Threat model updated for the new surfaces
  (docs/security/threat-model.md); findings written up with severity, place
  and fix (docs/security/audit-preview4.md). No Critical or High finding; the
  keychain-scoping items (SEC-21, SEC-22) are flagged to fix before release.
  The fix pass follows.
- **A program can no longer read RapidR Studio's AI keys (SEC-21).** Each
  program's RAI keys are its own, under its own keychain service
  (`rapidr-app:<program>:RAI`); Studio keeps its own where they always were
  (`dev.rapidr.keys`, nothing moved or rewritten), and only Studio's own
  process reads them. A program that names Studio's key
  (`keychain:RapidR Studio/…`) finds none, and `Error` says, in plain words,
  to give the program its own key. Studio's designer keeps a key typed for
  a program in that program's service (RAI.KeyOwner, Studio's only). Keys a
  program kept before are asked for once again.
- **RSecrets are kept by the program's identity, not its file name alone
  (SEC-22):** its name and a digest of its file's full path (a built macOS
  app: its bundle identifier), so two programs called `main.rr` in two
  folders never share a secret. Items kept under the name alone (only on
  development builds: RSecrets wasn't in a release) are never read: nothing
  in them says which program saved them.
- **The assistant's tools never follow a link out of the project (SEC-23):**
  `read_file`, the file list and every proposed change resolve links and
  refuse a path that leads outside the project's folder (or through a link
  to nowhere), in plain words.
- **A model's answer can't carry Studio's own action links (SEC-26):** an
  `ai:` or `rai:` link the model writes — plainly, as an autolink, a
  reference, with entities, or across answers — shows as words, not a link;
  Studio's own (Review, Open, Undo, Edit, Retry, the thinking) still work.
- **RAI never follows a redirect (SEC-27):** a provider's address that
  answers 3xx gets an error in plain words, and the key's header never
  reaches the address it points to.
- **The assistant's kept chats and the audit log are only the user's
  (SEC-28):** files 0600, folders 0700 on Unix (older ones are made so when
  written again).
- **The debugger's setVariable checks that the program is stopped (SEC-29)**,
  outright, as setExpression does.
- **The registry says what the manual says (SEC-25, SEC-30):** only
  `VerifyCA`/`VerifyFull` check a MySQL server's certificate (the manual's
  example now uses `VerifyFull`); RServerSocket's `Mode = "http"` checks
  nothing and listens on every interface unless `Host = "127.0.0.1"`.

### Fixed
- **Native builds on Windows: an antivirus no longer takes a new small
  program for a threat, and when one still does, you are told how to get out
  of it.** A native build stopped with `…\release\<program>.exe: Access is
  denied. (os error 5)` although cargo had finished: some antivirus programs
  judge a program they have never seen by guesswork, and refused a bare,
  stripped native `.exe` with no resources at all (the 2.121.0 release test on
  Windows x64: all six small native test programs, refused for good within a
  second of being linked; the same programs built for arm64 were left alone).
  - A native program is now **linked with version information and a manifest**
    (the manifest only says it runs as the person who starts it: nothing
    else about how it runs changes). The SDK's `llvm-windres` makes them and
    cargo hands them to the linker for the program's own executable only —
    the runtime stays built. Afterwards the icon and your app's details are
    written over them as before. In the release test's VM the same six
    programs, three times over, were all accepted (18 of 18).
  - An antivirus that is only *scanning* the new `.exe` is **waited for**:
    `rapidr build` tries again (100 ms, doubling, up to 10 seconds in all)
    when Windows says the file is in use (error 5, 32 or 33).
  - One that still **blocks** it never lets go, and waiting can't help, so
    the build now says so in plain words: "Your antivirus most likely blocked
    the new program … often a false alarm for a brand-new program", names the
    two folders to add to its exceptions (RapidR's build folder and the folder
    the app goes to), and points to the manual's new **Troubleshooting >
    Antivirus** (how to restore the program, how to report a false alarm to the
    antivirus maker, signing). A program that vanished into the quarantine is
    explained the same way.
  - The Windows release test names the antivirus installed and says when it
    was the cause.
  Only on Windows; macOS and Linux are unchanged (`crates/rapidr-cli/src/
  locked.rs`, `winres.rs`, `rapidr-package` `windows.rs`).
- **Studio, small things made solid (POLISH-2).**
  - **Go to Line** (Ctrl+G, ⌘G on a Mac) keeps its `:` — the number you
    type goes after it, where it once replaced it — and the status bar's
    `Ln` follows every jump the program makes (Go to Line, the Outline, a
    symbol, Find in Files, a problem, F12 into another file).
  - **Undo back to the text you saved** clears the document's changed
    mark (its tab's dot, the project tree's, the window's); redo away from
    it marks it again. The text is compared with what was saved, so typing a
    character and deleting it isn't a change either, and typing on right
    after a save is.
  - **The form designer** shows a colour written as `RGB(…)` (it showed
    clBtnFace) and an RImage's picture named by its file — `BMP = "…"`,
    `Load "…"`, or a `Logo.Load "…"` line after the form — read from the
    program's folder only. Properties shows the `RGB(…)` colour's swatch
    too.
  - **While the program runs on after a pause**, Call Stack and Variables
    say *Running…* (Watch: *(running)*) a moment later instead of showing
    the last pause's values as current; they fill again at the next pause,
    Variables where it was.
  - **Variables and Watch: ⌘C copies the selected value on a Mac** (and
    Edit ▸ Copy does, with them focused). RTreeView has a new RapidR event,
    `OnCopy` (Ctrl+C / ⌘C, which OnKeyDown's Shift state can't say), as
    `OnUndo` is for Ctrl+Z; RapidQ's OnKeyDown values are unchanged.
  - **A form's Enabled** (and Visible, ShowHint on the few components that
    had them as numbers: RToolBar, RGlassFrame, RMemo's, RUpDown's,
    RDateTimePicker's, RCodeEditor's, the media players') is on / off in the
    registry, so the debugger shows `Enabled = 1 · on` for a form too and an
    AI writes True / False; what programs read is unchanged.
  - **The status bar's state** takes the room its words need (at least the
    230 pixels it had), from the project's name, never from the file's
    facts; only a message longer than the whole bar is shortened, the whole
    of it in the tooltip and for screen readers.
  - **Set Up RAI's Finish** asks the provider for its models at once, as ↻
    does: the model's context window and kind of thinking are known without
    pressing it.
  - **Wrapped labels** (WordWrap) are broken where the text, as drawn, fits
    — measured with the drawing's own metrics at the screen's scale — so a
    modern theme at 2× no longer cuts a word's last letter.
  - **Chinese, Japanese and Korean text is measured as wide as it is
    drawn**: a character RapidR's faces don't have is drawn by the system's
    CJK font, an em wide, and is now measured so (it counted as a `?`, half
    that) — tab names such as "查询 1" and "查询 2" no longer run into each
    other, and every component that sizes to its text (AutoSize, tabs,
    lists) gets it right.
  - **A tree view scrolls to its last row whole** (Database Studio's: a
    root of 47 tables, then a CSV, the CSV half hidden at the end of the
    wheel or the bar, the thumb with room below it): its bar's range now
    ends where the last row shows whole, and a row the keys select (End,
    PageDown, Down) — or the program's ItemIndex — scrolls into view, as
    Windows' TreeView.
  - **A status bar's text is 10 pixels in** in RapidR's light, dark and high
    contrast looks (a SimpleText sat 4 from the edge), as Studio's own; the
    classic look keeps RapidQ's.
  - Tests: the language service's and the LSP's compiler-diagnostics checks
    take the `.rr` error cases too; Studio goldens' AI mock takes a free
    port when another run has 18695 (its two scenes then run but aren't
    compared). Studio flows: `editor-go-to-line`, `editor-undo-to-saved`,
    `editor-redo-from-saved`, `debug-running-stale`, `debug-copy-value`,
    `debug-form-on-off`, `debug-status-long-name-room`,
    `ai-r6-setup-local-window` (and checks added to `debug-status-long-name`,
    `ai-r6-setup-local`, `ai-r6-setup-cloud`); `RAPIDR_TEST_DUMP` reads a
    sub-object's member (`status.panel(2).caption`).
- **Studio: Search's and Problems' file icons, and Search's Forms and
  Modules scopes, go by the code**, as the Project pane does: a file that
  makes a form has a form's icon and is searched under Forms even when the
  project file still lists it as a module (and the other way round). Seen in
  the real walk on a project whose `.rrproj` was stale. Flows
  `search-stale-kinds`, `search-stale-forms`, `search-stale-modules`
  (tests/fixtures/studio_stale).
- **Studio: Problems opens at its top**, errors first in view: it no longer
  follows new rows to its end as Output does (27 problems showed the last
  file's). Filled again while you read, it stays where you scrolled.
  ROUTPUTCONSOLE's new `TopLine` says the first line (row) in view. Flow
  `problems-top` (27 problems, tests/fixtures/studio_problems).
- **RapidR Studio on the web opens a project picked as its `.rrproj` file.**
  File > Open (and the Welcome page's Open Project or File...) gives a page
  the one file picked, so Studio opened the project with empty files: main.rr
  blank, no forms in the tree, or a closed document's code under main.rr's
  name. Now Studio asks to open the project's folder (Open Folder...), the
  folder picker starts where the `.rrproj` is, and the project opens with all
  its files and forms; Cancel leaves Studio as it was and the Output says how
  to open it. A document whose file can't be read now opens empty, never with
  another file's code. The desktop is unchanged.
- **A `$MACRO` line with a `)` before its `(` no longer crashes the compiler
  and the editor.** `$MACRO a) b(c = 1` made the parameter list's range run
  backwards ("byte range starts at 30 but ends at 27"); it now defines
  nothing, like a missing `)` (found by the new fuzz tests).
- **Test hooks reach a dialog a handler opened, in native builds.** A form
  shown with ShowModal (or a message box, INPUT$, a file dialog) from inside
  an event handler — a menu item's OnClick, a form's OnClose — waits inside
  the handler in a native build, and the test's next events never came:
  the run waited forever, and RAPIDR_CAPTURE never captured the dialog. The
  script now goes on while a handler waits for the user, as the
  interpreter's always did: its events reach the dialog, and a test that
  ends with it open dumps and captures it. GUI case `handler_modal_events`
  (native and interpreted agree).
- **`__until_PROP_VALUE` compares VALUE as written.** It was lowercased with
  the rest of the event, so a caption with capitals never matched and the
  run waited its two minutes; its value may also hold dots now
  (`lbl.__until_Caption_v1.2 Ready.`).
- **A grid's heading cycle goes on across a query's new rows.** Setting an
  RDBGrid's `DataSource` again to the component it already shows (a program
  binding before each query) cleared its sort, so the next click on the
  same heading started over at ascending. It now keeps the sort: ascending,
  descending, none, whatever rows arrive; another `DataSource` starts
  unsorted, and rows added to a grid's own data are sorted in. Test
  `a_grids_sort_stays_as_rows_come_and_its_source_is_set_again`.
- **Studio: each pane keeps its thin edge all the way down, and in dark the
  gaps between panes show without hovering.** A docked pane's component (and
  each group of documents) now sits inside its group's 1-pixel frame instead
  of over it, so the frame runs down both sides and along the bottom; before,
  it showed only beside the header or the tab strip — the "extra pixel or two"
  at the top of each pane. A pane's component in a dock draws no border or
  focus ring of its own there (one edge, never two; the group's accent line
  says which pane has the focus), so the code editor, the Outline, Variables
  and Call Stack trees, the Watch, Breakpoints and Search lists (a box out to
  the edges of a plain panel that is the pane counts too) and the Immediate
  box no longer stack a second line.
  The dark look's edge is a little stronger (1.7:1 against the gutter and the
  panes, was 1.37), since its gutter is the panes' own colour; the light look
  is unchanged, high contrast's edges are its crisp white (the accent around
  the active one), and the classic look keeps its sunken edges. A splitter's
  accessible value is now a whole percentage ("33"), the same on the desktop
  and in the browser (Chrome read 32.7 back as 32.70000076293945). Unit tests:
  `every_pane_keeps_its_edge_all_the_way_down` (every look, docked groups and
  one or two groups of documents) and `a_panes_edge_shows_at_rest_in_every_look`;
  the dock_manager parity case's pane sizes (each 2 narrower, 1 shorter). Studio
  goldens approved in light, dark and high contrast: `welcome` (1x, 2x),
  `designer-pins` (1x, 2x), `designer-handles`, `editor-completion` (1x, 2x),
  `editor-hover`, `editor-problems`, `debug-variables`, `debug-callstack`,
  `debug-breakpoints`, `debug-watch`, `debug-datatip` (1x, 2x), `debug-error`,
  `markdown`, `richtext`, `import-report`, `find-in-files`, `palette`,
  `ai-panel`; the manual's `panes/splitters-*` shots made again.
- **A parameter or variable named like a keyword no longer changes your
  strings** (found by Database Studio, whose Open button went blank). After
  `SUB Show(Open AS STRING)`, every string literal `"Open"` (any case) became
  that parameter — empty outside the SUB — until the next SUB or FUNCTION;
  after `Step = 2` or `DEFSTR return`, every `"Step"` or `"return"` printed
  the variable's value for the rest of the program. Any keyword that can
  name a parameter or a variable did it (Open, Close, Input, Kill, Seek,
  Write, Print, Step, Type, Case, Bind…). The parser read a string literal's
  text as a word; a literal is now always data, on every runtime (VM, native
  build, web). Also `5 "SHL" 2` is no longer a shift: it's 5, as RC.EXE
  reads it. Conformance cases `keyword_param_literals` (checked against
  RC.EXE) and `keyword_param_literals_rr`.
- **Tab names are no longer cut short while there is room for them** (found
  in high contrast, where the debugging layout's "Breakpoints (2)" showed as
  "Breakpoints…"; light and dark did the same). When a dock group or the
  documents' strip is too narrow for all its tabs, the tabs now give up their
  padding first — every tab alike, the names whole — and only then are the
  widest trimmed to one width, so a short name is never cut for a long one's
  sake (before, every tab shrank in proportion and the widest lost its end).
  The geometry says where each title is drawn (`Tab.text`) and the painters
  use it. A name that still can't fit keeps its full tooltip and its full
  accessible name. Flow `tabs-fit-names` (the three looks, contrast judged),
  unit tests for the squeeze and for a crowded group; the Studio goldens
  `debug-variables`, `debug-callstack`, `debug-breakpoints`, `debug-watch`,
  `debug-datatip` (tab strip), `debug-error` (the Output's long line wraps)
  and `palette` (one more command, a shorter scroll thumb) approved in all
  three looks.

### Added
- **A message box's safe answer on Enter** (for RapidR Database Studio's
  delete and consent questions). `MESSAGEDLG(Msg, Type, Buttons,
  HelpContext, DefaultButton)`: the fifth argument (RapidR's; RapidQ's
  MESSAGEDLG has four) names the button Enter presses by its answer —
  `mrNo` — or its `mb` flag (`mbNo`) when no button answers that number.
  That button has the focus as the box opens, and Tab still walks left to
  right from it; Escape and the close box answer as they always did (Cancel,
  else No). Without it, the first button, as before. `MESSAGEBOX` now
  honours Windows' `MB_DEFBUTTON2` / `MB_DEFBUTTON3` flags (`&H100`,
  `&H200`), as RapidQ's MESSAGEBOX (Windows' MessageBox) did. The same on the
  desktop (interpreted and native) and the web, every theme; a native web
  build's browser confirm makes OK the default. Test hook:
  `RAPIDR_TEST_MESSAGE_DIALOG=__enter` answers with the default button. The
  registry's constant set `modal-result` (the `mr` answers) offers them.
  Tests: `default_buttons` (rapidr-value), `a_default_button_takes_enter_and_the_focus_tab_goes_round`
  (the kernel: Enter, Tab round, Escape), the GUI case `message_default`
  (SendKeys Enter and Escape through the kernel's keyboard, native and
  interpreted); manual components.md "Message boxes: the safe answer on Enter".
- **InTransaction on RSQLite and RMySQL, Autocommit on RMySQL** (REQUEST-003
  backlog). Whether a transaction is open, read without running a query (the
  grid's rows stay): SQLite's own word (autocommit off) after every
  statement — Query, QueryScalar, a background query's before OnDone, a
  failed one too —; MySQL's and MariaDB's session tracker
  (`session_track_transaction_info`, MySQL 5.7+, MariaDB 10.3+), which puts
  the transaction's state in the server's answer to each statement that
  changes it: BEGIN / START TRANSACTION, COMMIT, ROLLBACK, the first change
  with autocommit off, the implicit commit after DDL, `SET autocommit = 1`.
  With MySQL it is -1 (can't tell, true in an IF) after a query of several
  statements whose last gives rows, after a read with autocommit off, and
  on older servers. Close sets it to 0 (the server rolls back). Tests:
  `in_transaction_follows_sqlite` (BEGIN, COMMIT, END, ROLLBACK, SAVEPOINT /
  RELEASE, errors, several statements, QueryScalar, QueryAsync, Close),
  `what_an_answer_leaves_unsaid`, and against a local MySQL 8.0.33
  `in_transaction_on_a_local_server` (savepoints, SET autocommit = 0, the
  implicit commit after CREATE TABLE, background queries). Example
  `examples/data/transactions.rr` (run, interp, native, web): its close
  question asks with No as Enter's answer. Manual databases.md "Is a
  transaction open?".
- **Test hooks** (Database Studio's findings): `any.__wait_N` (or
  `__wait_ms_N`) lets the program run on its own for N ms — its timers, its
  servers — before the next event; `__hold_ms` now also waits its ms on a
  host that can't hold the pump (real windows, the web).
- **Studio: Search you can see** (Preview 4 S3). Find in Files is now the
  **Search** tab beside Project, a magnifier on it, always there (Ctrl+Shift+F
  brings it to the front, the selection in its box). **In** chooses where to
  look: this file, the open files, the project's modules, its forms, its
  resources, the whole project (the default) or a folder (picked; its text
  files, at most 2,000). The results come by file: the file's icon (the
  Project pane's, one lookup), its name in bold, its folder dimmed, how many
  in a pill; under it each result's line:col in a muted column and the line,
  leading spaces trimmed, the match marked on the theme's match colour and
  always in view (a long line shown around it, "…" where it's cut). Chevrons
  and indents, no dotted lines (the classic look keeps them); the theme's
  hover and selection. RTreeView's new item properties, RapidR's: `Icon`
  (RapidR's icon set by name, or `file:<path>` for a file's type icon),
  `Column`, `Detail`, `Badge`, `Bold`, `MarkStart` and `MarkLength`. A click
  on a result shows the place and keeps the keys in the results; Enter or a
  double click opens it with the match selected; a click or Enter on a file
  shows or hides its results, Left and Right too.
  Replace All works as before, one undo step a file. "1 result in 1 file",
  said to screen readers.
- **Studio: Find References and Go to Definition across the project's files**
  (S3). Shift+F12 lists every use of the name at the caret — a variable, a
  SUB or FUNCTION, a component — in the Search tab, by file: "References to
  osName: 19 in 4 files" (the language service's symbols, the includes in
  their order: a comment or a string with the same word isn't a use). F12
  jumps to the declaration in any file; **Navigate Back** (Alt+Left) and
  **Forward** (Alt+Right; on a Mac Cmd+Option+Left and Right, since
  Option+arrows move by words there — RPROJECT's new `MacKeys` tells), in the
  Edit menu, go back and forth over the last 40 places a definition, a result
  or a reference took you from. The code's
  right-click menu has Go to Definition, Find References and Rename Symbol at
  the top, then Cut, Copy, Paste, Select All and Find in Files; a right click
  on a word puts the caret there first.
- **Studio: Problems that tell the truth** (S5). A module — any file the main
  file includes, at any depth — is checked as part of its project, open or
  not, never alone: no false errors in a module of a program that compiles
  (RLanguageService's new `Project`; the code editors follow it too). The
  Problems page lists the whole project's problems, errors first, then
  warnings, then notes, grouped by file: a heading with the file's icon (the
  Project pane's), its folder and its count in a pill, and under it each
  problem's line:col in a column and its message (ROUTPUTCONSOLE's
  AddProblem: severity `file` or `file:<icon>` makes a heading, Line its
  count). On a web server of 7 files and 4,500 lines that compiles:
  26 problems (the `NOT FILEEXISTS` warnings) instead of 167, no errors in its
  modules.
- **Project Options: Targets** (S5): Desktop (the default), Web, or Desktop
  and web, kept in the project file (`[build] runs_on`; a project that builds
  a web app too starts as Desktop and web). What works on one runtime only is
  noted only when the project runs on the other, once per component or
  builtin, at its first use, with how many uses: "RServerSocket works on the
  desktop only: the web runtime doesn't have it (45 uses)" — was a note per
  call. RPROJECT's `Targets` and `FilesIn(Folder)`, RLanguageService's
  `Targets` and `ProjectDiagnostics`, in the registry. Manual: The code
  editor (Search, Find References, Back and Forward, Problems) and Projects
  (Targets), with new pictures. Tests: the language service's
  `tests/project.rs` (a module with its project, references and definitions
  across files, notes per component, the order); Studio flows `search-tab`
  (light, dark, high contrast, text judged), `search-scope-modules`,
  `search-scope-forms`, `find-references`, `go-to-definition`,
  `navigate-back`, `problems-project`, `problems-targets-web`,
  `problems-web-note` (three looks, judged) and `problems-errors-first`, on
  the desktop and the web.
- **Servers that only this computer reaches** (REQUEST-003, for RapidR
  Database Studio). RSocket's `Open(Port, Address)` and its new
  `BindAddress` property (RapidR's) choose where a QSOCKET server listens:
  `"127.0.0.1"` this computer only — nothing on the network can reach it,
  and the system doesn't ask about incoming connections. Without them it
  is `0.0.0.0` (every interface), as RapidQ's. RServerSocket's `Host` was
  already its listening address (now documented: `0.0.0.0` by default).
  The web still answers -1 (a page can't listen).
- **RServerSocket: data arrives whole** (REQUEST-003: a POST's body was
  lost). Its connections are now read as bytes into a buffer and cut by
  the new `Mode` (RapidR's): `"lines"` (the default: each line trimmed,
  blank ones left out, as before — but a line that comes slowly is never
  dropped any more, and a last line without its end comes when the client
  closes), `"bytes"` (what arrived, nothing trimmed or dropped) and
  `"http"` (each whole HTTP request: head and body, by its Content-Length
  or its chunks — joined, given a Content-Length —, "100 Continue" sent to
  a client that waits for it, 4 MB at most, malformed or too large ones
  answered 400 / 413 / 431 / 505 and closed). `Send(Address, Data)` sends
  without a line end, `Disconnect(Address)` closes one client; `Port 0`
  takes a free port and `Port` says which; `Stop` closes the clients'
  connections. One framing (`rapidr_runtime_core::netframe`) for it and
  RHttpServer. The example `examples/network/echo_server.rr` (curl gets
  1 MB back whole, by length and chunked); the examples suite starts a
  client for server examples (`client`, tests/http_client_probe.mjs).
- **RHttpServer: a local web server for programs** (REQUEST-003 item 1;
  desktop only). `Host` (127.0.0.1 unless set) and `Port`, `Start` /
  `Stop`; each request whole in `OnRequest (Method, Path, Headers, Body)`
  on the program's own thread, one at a time, woken by its arrival (the
  window never waits for the network; DOEVENTS serves it in a console
  program), with `Query`, `Header(Name)`, `Peer` and `Request` (its number,
  to answer later). `Respond(Status, Headers, Body [, Request])` answers
  (Content-Length and a text Content-Type added); `BeginStream`,
  `SendEvent(Data [, EventName])`, `EndStream` stream Server-Sent Events
  (chunked on HTTP/1.1) — enough for MCP's Streamable HTTP; keep-alive,
  `Close`. Refused by the server itself: a Host that isn't its own (DNS
  rebinding, 403; `AllowedHosts`), a request a browser marks
  `Sec-Fetch-Site: cross-site` or whose Origin isn't allowed (403;
  `AllowedOrigins`), anything but GET and HEAD while no `Token` is set
  (401, "this server has no Token"), a missing or wrong
  `Authorization: Bearer` when `Token` is set (401; an allowed page's CORS
  preflight passes), bodies over `MaxRequestSize` (413), requests slower
  than `Timeout` (408), more than `MaxConnections` (503). A link or an
  image on another site can still send a GET without an Origin: the manual
  says GET handlers must only read, and anything more needs a Token.
  `NewToken` sets Token to 64 hex digits from the operating system's
  secure random source (getrandom, already in the build through
  native-tls: no new crate) and returns it. Plain HTTP: on 127.0.0.1
  nothing goes over the network, but other programs on the computer can
  connect, so the Token is the protection (the manual says so). On the web
  `Start` fails and `Error` says why. The example
  `examples/network/local_api.rr`, an examples-suite case driving it with
  a client (401, 403 twice, 413, /hello, /echo, three events), its own
  component icon (a server and a globe), the manual's
  [Networking](docs/manual/network.md) page.
- **`Application.Announce(Text [, Assertive])`** (REQUEST-003 item 3): a
  screen reader says Text without the focus moving — politely, or at once
  when Assertive is True; the same text twice is said twice. On the desktop
  through the focused window's AccessKit live regions (two status nodes
  taking turns, `rapidr_value::objects::a11y::announcement_nodes`), on the
  web through the page's `aria-live` regions. Desktop status nodes are now
  live regions too (polite, as the web's `aria-live` always had them): the
  code editor's and the designer's announcements are live there as well.
  The local API example announces each request it answers; the
  manual's [Accessibility](docs/manual/components.md#accessibility).
- **RMarkdownView colours SQL code blocks** (REQUEST-003 item 2): a
  ```` ```sql ```` block (```` ```sqlite ````, ```` ```mysql ```` with that
  dialect's words) is coloured as the code editor colours SQL, and
  ```` ```json ````, ```` ```html ````, ```` ```css ````,
  ```` ```javascript ````, ```` ```toml ````, ```` ```csv ````,
  ```` ```rust ````, ```` ```markdown ```` blocks as theirs, in the
  theme's colours; any other stays plain. The example
  `examples/gui/markdown_code.rr`, the manual's
  [Markdown](docs/manual/markdown.md#code-blocks-in-colour).
- **The Output, Build and Problems pages wrap long lines** (ROutputConsole's
  new `WordWrap`, RapidR's own, True unless a program sets it). A line longer
  than the pane breaks onto more rows at a space near the edge (else at the
  edge), so a long message reads whole instead of running off. It stays one
  line to the mouse (a click on any of its rows selects it, `file:line` links
  work on every row), the selection, Copy and a screen reader, and the wheel
  and the bar move by rows. On the Problems page a row grows to hold its whole
  message, the place after its last row (or under it). Off, a line is cut at
  the edge as before — a problem's message ends in "…" — and resting the mouse
  on it shows the whole text (an Output line's too: new). In RapidR Studio:
  **View > Wrap Long Output Lines** (a check item, kept in the settings) and
  **Wrap Long Lines** in the pane's right-click menu; flows `problems-wrap`,
  `output-wrap-long-line`, `output-wrap-menu`, `output-wrap-off`,
  `output-wrap-remembered` (the text contrast judged in every look), kernel
  tests for the flow, the click, the copy, the wheel and the tooltips, the
  manual's "Long lines in Output and Problems".
- **Labels as links: `TabStop` on an RLabel** (RapidR's own; RapidQ's QLABEL
  never takes the focus, and a label without it doesn't). A label given
  `TabStop = 1` is a link: Tab reaches it (in `TabOrder`), the theme's focus
  ring shows round its text, Enter or Space clicks it (`OnClick`), `OnEnter`,
  `OnExit` and `SetFocus` work, and a screen reader announces a link named by
  its `Caption` (or `AccessibleName`), with the `Hint` as its description.
  RapidR Studio's Welcome page Recent links were the mouse-only ones left —
  Tab skipped them; now Shift+Tab from the offer to reopen reaches the link
  and Enter opens the project (flows `welcome-recent-link-keyboard` and
  `welcome-recent-link-enter`). The registry's RLABEL has the property, the
  method and the events; the example `examples/gui/links.rr`; the manual's
  Accessibility section.
- **Menus drawn in the window are in the accessibility tree** (Windows,
  Linux and the web; macOS uses the system's own menu bar). An open menu —
  a bar's drop-down, a submenu, a pop-up menu or an edit's context menu —
  is a `menu` node with its items inside it: `menuitem`, `menuitemcheckbox`
  and `menuitemradio` with their checked state, disabled items disabled, a
  submenu's item collapsed or expanded (with a "has a menu" mark) and holding
  its submenu, the item's ShortCut as its keyboard shortcut (AccessKit's
  `keyboard_shortcut`, ARIA's `aria-keyshortcuts`) and its Hint as its
  description. The item the arrow keys (or the mouse) chose has the screen
  reader's focus — and no component has it meanwhile — so VoiceOver,
  Narrator, NVDA, Orca and a browser's screen reader read each item as the
  keys move; the bar selected with F10 or Alt focuses its item the same way.
  A screen reader can press an item, expand or collapse a submenu and move to
  an item. `RAPIDR_TEST_A11Y`'s JSON carries `accelerator`. Kernel tests walk
  the keys and read the tree, Studio's `open-recent` flow reads the File menu
  and its Open Recent submenu on both hosts, and `tests/web_a11y.mjs`
  compares the kernel's menu nodes with Chrome's on the new `menus_open`
  fixture.
- **RAI keeps your chats, and shows what it is doing** (Preview 4 R1, R2,
  R4, R5, R6, K8; Robert's notes on RAI). In RapidR Studio:
  - **Chats are kept**, per project, across restarts: opening a project
    shows its newest chat again. **Chats** lists them — open, rename,
    delete (with **Undo**), search titles and text; **New Chat** keeps the
    one it replaces. They are kept in Studio's data folder, never in the
    project's (`chats/` beside the settings: `~/Library/Application
    Support/RapidR/chats` on macOS, `%APPDATA%\RapidR\chats` on Windows,
    `~/.config/rapidr/chats` on Linux; the page's storage on the web), one
    file a chat; API keys and tokens are replaced by *[key removed]* before
    anything is written (`rapidr_ai::chats`, unit-tested); Delete removes
    the file. A chat the assistant used to make a new program goes with
    that program's project (and back when it is undone).
  - **Edit a sent question** (Edit beside *You*): a bar says what Send will
    do; what followed the question moves to a saved copy, "… (before the
    edit)", nothing lost. **Retry** beside the last answer.
  - **A live status line**: what it does now — *Thinking*, *Reading
    sites.rr*, *Searching the manual*, *Writing server.rr*, *Checking the
    code* — from the stream itself (a tool call's file named as soon as the
    model writes it), the steps done after it, the seconds beside Stop, and
    *No answer from the model for 30 s: wait, or Stop*; *Waiting for you*
    when a change waits for review.
  - **The context meter** (how much the next question sends, against where
    the chat is compacted by itself) and **Compact**; **/ commands** listed
    as you type: `/compact /new /chats /model /effort /undo /help`.
  - **Reasoning effort** Low / Medium / High where the provider has it —
    Anthropic's adaptive thinking at that effort (an older model's thinking
    budget, stepping down by itself when a model refuses), an
    OpenAI-compatible `reasoning_effort`; DeepSeek's models choose for
    themselves. The reasoning shows as **Thought for N s**, a link that opens
    and closes it (the link goes to the top of the chat, the reasoning under
    it: RMarkdownView's `ScrollTo` also takes a link's target now).
  - **One model list with ↻**; the provider row is gone from the pane (it is
    Settings'). **Set Up RAI**, a two-step wizard for a first-time user: free
    on this computer (finds Ollama, LM Studio and MLX Serve running — it
    never starts one) or a cloud key (a key already found can be used; Test;
    a model, the fast ones first), with what each costs and sends in plain
    words. Tools > Set Up RAI, Ctrl+I before it is set up, and Settings > AI.
  - **A lite set for small windows** (K8): a model of a context window under
    16,384 tokens gets short instructions, the language card and eight
    tools — about 2,500 tokens instead of 7,800 (estimated at three
    characters a token, `rapidr-studio`'s `a_small_window_gets_the_lite_set`;
    a live Claude Haiku question with the full set counted 10,966 input
    tokens, the project's facts and the question included) —, so an
    8,192-token local model starts and answers again.
  - RAI, for every program: `ChatState` (the conversation as JSON, keys
    removed), `Rewind(N)`, `QuestionCount`, `QuestionText(N)`, `Editable`,
    `Activity`, `Elapsed`, `Quiet`, `ContextTokens`, `ContextLimit`,
    `ContextWindow`, `Compact`, `Effort`, `EffortSupported`,
    `ToggleThought(N)`, `Lite`; RAIProjectTools: `ChatList`, `SearchChats`,
    `SaveChat`, `LoadChat`, `DeleteChat`, `RestoreChat`, `RenameChat`,
    `ChatTitle`, `ChatsFolder`, `Lite`. Anthropic's models list's
    capabilities are read (how each model thinks). Flows `ai-r1-*` …
    `ai-r6-*`, `ai-k8-lite`; the mock provider streams thinking and writes
    slowly on request; the manual's [AI assistant](docs/manual/ai-assistant.md)
    page and seven new pictures.
- **Versions, the engine (lane G1, docs/versions-plan.md).** `rapidr-vcs`
  speaks plain words over gitoxide: save a version (every changed file or
  some), the timeline, what changed, go back (a new version with the old
  files, never a destructive reset), restore one file, experiments whose
  switch rewrites only the files that differ, shared folders (Get latest /
  Share, in-process). **Nothing is ever lost**: a safety point
  (`refs/rapidr/safety/<n>`, the whole working folder, unsaved changes
  included) before every risky action; a history of actions
  (`.git/rapidr/history.jsonl`) whose Undo takes the last one back and
  refuses, changing nothing, when it would overwrite newer work; RAI's
  checkpoints (`refs/rapidr/checkpoints/<n>`) and going back to them.
  **The secret check** stops a save whose added lines look like a private
  key, an API key, an access token or a password in the code, with the file,
  the line and the secret masked (RapidR's own patterns), until the person
  saves anyway. A project owned by another user account is refused for
  changes with the reason, and opens read-only on request. Errors in plain
  words, the technical cause kept apart. Measured (release): 1,000 versions'
  timeline 49 ms, status 0.6 ms. Not in Studio yet (the panels are G3).
- **RapidR's own diff engine, `rapidr-diff`** (lane G2): Myers' O(ND)
  algorithm in linear space, written from the 1986 paper
  (docs/licensing.md §8 item 6), line hunks with unified numbering, word
  marks, CSV table diffs, form diffs in words; no dependencies, builds for
  the web. A 4,500-line file diffs in 0.25–16 ms (release; the worst case is
  the file reversed). RDiffView, Versions and RAI share it.
- **RDiffView's form and table views** (lane G2). `Kind = "form"` compares
  two versions of a program's CREATE blocks: the form before and after,
  drawn as the designer draws it, the changed components outlined in the
  theme's added / removed / changed colours, then the changes listed
  ("GreetButton: Caption "&Greet" → "&Say hello"; Width 90 → 110", "Added
  Hint (RLabel)"). `Kind = "table"` compares two CSV files as one table:
  rows matched by a unique first column (else by content), a changed row as
  its old values over its new ones with the changed cells marked, added
  and removed columns tinted, a sticky header and gutter, Left / Right and
  Shift+wheel across wide tables. New `ChangeCount`, `Summary` (the changes
  in words, one per line; also the screen reader's list) and `HasHeader`.
  The example `examples/studio/diff_views.rr`, the manual's
  [Versions](docs/manual/versions.md).
- **A debugger that looks as good as it works** (Preview 4 S10). In
  RapidR Studio, Variables, Watch, the Call Stack, the Breakpoints pane and
  the data tips draw each value with its kind's icon, the value in its
  kind's colour (the code editor's), an empty text as a quiet `""`, a colour
  as a swatch and its name (`FontColor = ■ clWindowText`), an array with its
  count (`ColName = Array(0 TO 64) · 65 items`), a property's constant by
  name (`WindowState = wsMaximized (2)`); a value that changed since the
  last stop is highlighted and marked. Screen readers hear each row as a
  sentence ("FontColor, colour clWindowText"). The margin and the
  Breakpoints pane mark each breakpoint by its kind: a condition `=`, a hit
  count `#` (no longer the same as a condition), a log point's diamond,
  off a ring, not bound to code a dashed ring — the shapes tell them apart
  in high contrast too. A pane header's tooltip (Close, Auto Hide) shows
  above the header instead of over the pane's first row. For your own
  programs: `TreeView.SetItemValue i, Name, Kind, Value [, Count [, Hint]]`
  shows a tree node as a value, and `Item(i).Changed` marks it; the
  debugger session gives each property's hint (`color`, `handle`,
  `const:…`, `readonly`). See [the manual](docs/manual/debugging.md#how-values-look).
- **Change values while paused** (Preview 4 S11). In RapidR Studio's
  Variables and Watch, F2, a double click or the menu's *Set Value…* opens
  an editor over the value only (the name stays), the value selected. What
  you type is checked for the value's kind before it reaches the program:
  text is typed without quotes, numbers take a number or a constant's name
  (`clRed`), on/off properties take `on`/`off`, True/False values `yes`/`no`
  too, a colour `#RRGGBB`, and `=` starts an expression (`=total * 2`). A
  property's constants (`Cursor`, `WindowState` …) come as a menu with the
  current one ticked. A text is always edited **whole**, never the
  shortened display; a long one, or one with line breaks, opens in a
  window of its own (its line breaks kept as the program had them). Refused
  in plain words, in Output beside Variables, the status bar pointing there:
  *total holds a whole number, and abc isn't one. Type a number such as
  42.*, a read-only property, a whole array or record, a component, a
  credential (`••••`, never edited from its mask). A value set is marked
  changed, the status bar says what it is now, and **Ctrl+Z** puts it back
  while the program stays paused. The Immediate pane's `name = value` sets
  through the same checks (and Ctrl+Z). An on/off property shows its number
  with a quiet *on*/*off* after it (`KeyPreview = 0 · off`) — never `True`,
  which is `-1` in a `.rr`. The debugger session checks every set
  (`setVariable`) the same way, and the debug adapter (`rapidr dap`) adds
  `setExpression` (a watch's place set). For your own trees:
  `TreeView.EditItem Index [, Text]` opens a node's editor with that text,
  `TreeView.SetFocus` gives a tree the keyboard, its `OnUndo` hears Ctrl+Z
  (⌘Z) on it, `Item(i).Top` says where a node's row is, and
  `PopupMenu.PopupAt Control, X, Y` shows a menu at a place inside a
  control wherever it sits (a menu under a tree's node). Ctrl+C in
  Variables and Watch copies the value again (it looked for the wrong
  key bit).
  See [the manual](docs/manual/debugging.md#changing-a-value).
- **Helpers: the code shows and edits its own values** (Preview 4 E0, E1,
  E2, E6; docs/code-helpers-plan.md). In RapidR Studio's code editor, desktop
  and web alike: a colour written as `RGB(…)`, `&H…` or a constant shows a
  swatch after it; a picture's name its thumbnail, or a red mark when the
  file isn't in the project; a font's name a small sample; a sound a
  ▶. A click on a mark, or Ctrl+. (Cmd+.) on a value, opens a value editor:
  a colour wheel around a saturation-and-brightness square with the Red,
  Green, Blue and Hex numbers, the name in words, the old colour beside the
  new and the project's, standard and system colours; the project's
  pictures with a preview; the fonts with a preview line; a constant set's
  choices (ticks for flags, a toggle for True/False); the program's SUBs and
  **Create handler**, which writes the new SUB and the link as one step.
  What is picked is written back as one undo step in the code's own form
  (`RGB()` stays `RGB()`, `&H` stays `&H`, a constant stays a constant). The
  caret line's one-line help shows at its end, argument names before a
  call's literal arguments (all of them while Ctrl is held), and while the
  debugger is paused the paused routine's lines show their values at their
  ends, drawn as Variables draws them. Marks and names take fixed columns of
  the row (the caret stops beside the text, word wrap counts them, nothing
  before them moves; the line being typed on keeps them until the caret
  leaves). Tools > Settings > General > Code helpers: Helpful (the default),
  Quiet, Everything or Off; RCodeEditor's `Helpers`, `HelpersOff`,
  `ProjectFolder`, `ProjectFiles`, `HelperText`, `HelperEditorText`,
  `LineValuesText`, `OpenHelper`, `ReadLineWithHelpers` (Ctrl+K H),
  `ShowLineValues`, `HideLineValues`. Pictures are read only from inside the
  project's folder; nothing is fetched from the network. The Properties
  panel's colours and fonts open the same editors (its old swatch grid is
  gone). The registry's new `kind` facet (color, font, file:image|sound|any,
  shortcut, cursor, range, handler, component:<TYPE>, choice:<set>) on
  properties, parameters, builtins and constant groups, filled across the
  registry and checked by a test, is where the helpers' sites come from (the
  language service's `helpers`), and the AI export carries it. The example
  `examples/studio/helpers.rr`; the manual's
  [Helpers](docs/manual/studio-editor.md#helpers).
- **RSecrets: the OS keychain for programs** (REQUEST-001 item 4, for
  RapidR Database Studio and every program). `Save(Service, Name, Value)`,
  `Read`, `Delete` and `Exists` keep passwords, tokens and keys in the macOS
  Keychain, Windows Credential Manager or the Secret Service, through the
  same code RapidR Studio keeps its AI keys with (`rapidr_ai::keys`' store,
  now shared, and `rapidr_ai::secrets`); `Error`, `Available`, `Store` and
  `Program` say how it went. Each program has its own items
  (`rapidr-app:<program>:<service>`, the program's file name in lower
  case), so no program reaches another's or Studio's. What is read and saved
  is a credential (`rapidr_value::secrets`): the debugger shows `••••` for a
  variable, watch, data tip or logpoint that holds one, and for a secret
  property (RMySQL's Password, RAI's APIKey); the registry's new method
  facet `secret` marks Read and Save for AI tools; errors never carry the
  value. On the web every call fails with "RSecrets isn't available in the
  browser" (no fallback to the page's storage). `RAPIDR_AI_KEYSTORE=memory`
  (or the new `RAPIDR_KEYSTORE=memory`) is the test keystore for programs
  too, and the conformance, examples and manual-shot runners and
  `tools/regress.sh` now always set it. Its own component icon (a padlock),
  the example `examples/data/secrets.rr`, the manual's
  [Keeping secrets](docs/manual/secrets.md).
- **Encrypted MySQL and MariaDB connections** (REQUEST-001 item 7). RMySQL's
  `SSLMode` (`Preferred`, the default; `Disabled`, `Required`, `VerifyCA`,
  `VerifyFull`: MySQL's own client's modes), `SSLCA` (a CA's .pem or .der
  file) and `Encrypted` (what the server says this connection is), on the
  operating system's TLS through the `mysql` crate's `native-tls` feature
  (native-tls is already HTTPS's: no new crate, `cargo deny check licenses
  bans` passes). `Preferred` falls back to a plain connection where the server
  has no TLS, so every program that connected before still does.
  `RealConnect`'s `CLIENT_SSL` flag now asks for `Required`. Errors in plain
  words: a server without TLS, a refused certificate, macOS's 825-day limit
  for server certificates. Checked live against a local MySQL 8.0.33
  (`Required`: encrypted, ECDHE-RSA-AES256-GCM-SHA384) and against a mock
  server without TLS.
- **Programs' own SVG icons** (REQUEST-001 item 10).
  `RImageList.AddSVGFile(FileName [, Theme])` draws an SVG at the list's size
  and again at the screen's scale (sharp at 2×) and returns its index.
  Every SVG a program loads (RImage, RBitmap, a button's BMP, image lists)
  now takes the theme's colours when it is drawn with `currentColor`
  (before, black: invisible on dark) or with RapidR's icon colour tokens
  written as their light values (`rapidr_icons::theme_svg`: only when every
  colour is a token, so an illustration keeps its own), and is drawn again
  when the theme changes; `Theme` fixes the colours. A `$RESOURCE`'s name
  now goes wherever a picture file goes (`RImage.Load RES` said "could not
  load '65536'"), so a built app started from another folder still has its
  pictures. Desktop, native and web alike. The example
  `examples/gui/svg_icons.rr`, the manual's
  [Your own SVG icons](docs/manual/svg-icons.md).
- **Every model writes correct RapidR: the language card, the outline, the
  manual, the mistake ledger** (ROADMAP K1, K3, K4, K4b; docs/rai-mistake-ledger.md).
  - **The language card** (K1): about 1,300 tokens of the rules models get
    wrong, sent with every RAI chat — DIM before use (an undeclared name is a
    DOUBLE), a global declared once in the whole program, only SUBs that
    exist, `FOR i AS INTEGER`, `True` is -1 while `Checked` and `FILEEXISTS`
    give 1, arrays, `CREATE` windows, each event SUB's parameters, the types,
    RapidR's names. Generated from the registry (new `[[rule]]` entries in
    data/language.toml, each with a program `rapidr lang truth` compiles and
    runs as a `.rr`), never hand-written; a test fails when it names
    something the registry doesn't have or grows past 2,000 tokens.
    `rapidr lang export --card` prints it. It replaces the hand-written
    language lines of Studio's assistant prompt.
  - **`project_outline` and `search_manual`** (K3), two new RAI tools: the
    program as the compiler reads it (files in include order, who includes
    whom, each file's forms and components, SUBs and FUNCTIONs with their
    parameters, globals with their types, CONSTs and TYPEs, with lines; a
    file the main file doesn't include, said), and BM25 over the manual's
    guide pages (the best three sections, trimmed). The outline's
    description tells models to call it before multi-file changes.
  - **`FOR i AS INTEGER = 1 TO n [STEP s]`** (K4), a RapidR extension: the
    counter is declared where it isn't yet (a local in a SUB, a global in
    the main program); the same type is the same variable, another type or
    a non-number type is an error that says so. One front-end pass
    (`rapidr_ast::for_as`) both compilers run: the VM, native builds and the
    web behave alike; plain `FOR i = …` is unchanged.
  - **Messages that say the fix** (K4): a name declared again says where
    the first one is, in which file (`Identifier siteCount already used, try
    another name — it is declared in globals.rr:12 already …`; a file
    `$INCLUDE`d twice is said as such); an unknown SUB or FUNCTION names the
    nearest of the program's and the builtins (`— did you mean
    ShowNumber?`), or says no file has it. RapidQ's words come first, as
    before. check_code adds a hint under "expecting type DOUBLE, but got
    STRING": the variable was never declared.
  - **`NOT FILEEXISTS(x)` is always true** (K4b): FILEEXISTS gives 1 and NOT 1
    is -2, so the editor warns — `FILEEXISTS gives 1, and NOT 1 is -2 (true):
    write FILEEXISTS(x) = 0` — and its quick fix (Ctrl+.) writes it; also
    DIREXISTS (every builtin whose reference says "True (1)").
- **`ctText`, `ctImage`, `ctState` (0, 1, 2) in `.rr` programs**: the `Change`
  kinds an RListView's OnChange hears are RapidR's own, in none of RapidQ's
  includes, so they are built in the way RAPIDQ.INC's constants are (the same
  pass, `rapidr_ast::builtin_constants`, both backends; a name the program
  declares wins) for a RapidR program only — a RapidQ program reads them as
  undeclared variables, 0, as RapidQ does. In the language registry as RapidR's
  (origin `rapidr`), completion and hover; `docs/manual/reference/constants.md`
  lists them. Conformance: `rr_builtin_constants` (the numbers) and
  `rr_only_constants_not_in_bas` (nothing in a `.bas`), on both backends.
- **Background queries, a grid of a database's rows, and an SQL editor**
  (RapidR Database Studio's REQUEST-001 items 1-3; every program has them).
  - `QueryAsync(sql [, values…])` on `RSQLite` and `RMySQL` runs a query in
    the background: the rows stream in chunks (`OnRows(Count)`, `RowCount`
    following), then `OnDone(Rows, Ms)` or `OnError(Message)`; `Cancel` stops
    it (`OnError("cancelled")`: SQLite's interrupt, MySQL's `KILL QUERY` on a
    second connection); `Busy`. One query at a time: a query while one runs
    is refused at once with `OnError("busy: …")`, never waited for. On the
    desktop each connection runs its queries on a worker thread; in a
    browser on the page's thread in 8 ms slices, the page drawn between them.
    Measured with the new example: a million rows on the desktop, first rows
    after 15-25 ms, all in about 200 ms; 100,000 in Chromium, first rows after
    1-25 ms; a button clicked meanwhile answered at once on both.
  - `RDBGrid.DataSource` can be an `RSQLite` or `RMySQL`: their last query's
    rows (a background query's as they arrive), drawn only on screen. A
    heading's click on a database's rows is the program's to sort with
    `ORDER BY` (`SortColumn`, `SortDescending`, which a program can now set:
    -1 takes the arrow away). A new query's rows show from the top. Ctrl+C
    copies the current row; End shows the last row whole; columns are sized
    from the first and the last rows.
  - `ColName(i)` and `ColType(i)` on both: a result's column names and types
    from its own metadata (SQLite's declared types; MySQL's column types, on
    a read-only connection too). `Schema`: the database's tables, views and
    columns as JSON.
  - `RCodeEditor` with `Language = "sql"`: `SqlDialect` (`sqlite`, `mysql`),
    completion of the dialect's words and the program's tables and columns
    (`SetSchema(json)`, `Editor.SetSchema DB.Schema`; aliases resolved), a
    statement over several lines folds (`[folding] statements` in language
    definitions), Ctrl+Enter fires `OnRunStatement(SQL)` with the selection
    or the statement at the caret, up to its `;` or a blank line
    (`CurrentStatement`).
  - Example `examples/data/million_rows_async.rr`; manual: Databases and the
    new "The code editor in your programs"; tests: rapidr-db unit tests and a
    live MySQL test (`RAPIDR_TEST_MYSQL`), the conformance case
    `sqlite_query_async` (VM and native), `tests/web_query_async.mjs`, the
    examples suite on every runtime. GUI test hooks: `comp.__until_PROP_VALUE`
    waits for work in the background, `comp.__do_NAME` runs a component's own
    test step.
- **Forms come from your code, several in one file** (Preview 4 S1). The
  Project pane takes each file's kind from its code, not from what the
  project file said: a file that creates a form at its top level (`CREATE …
  AS RForm`, `RFormMDI`, `QFORM`, `QFORMMDI`; an `.inc` too) is under
  Forms, one that creates none under Modules, and it follows what you type.
  A file of several forms opens to show each of them; opening one (double
  click, Enter) designs that form in the file's designer, and selecting a
  form or one of its components switches the designer to it, announced
  ("Designing NewForm, form 2 of 3 in this file"). On Save the `.rrproj`
  is written with the code's kinds and the Output says what changed; a file
  the AI assistant writes is added by its code too. New members:
  `RProject.FollowCode`, `RDesignSurface.FormNames`; `SelectName` designs
  the form that holds the name; `RProjectTree` describes a file of several
  forms ("form, 3 forms") and a form node ("form") to screen readers.
- **File ▸ Open Recent** (Preview 4 S2): the last ten projects and files,
  newest first, each with its folder (`~` for your home), a number key
  each, and Clear Recent; the same in the command palette ("Open Recent").
  One that isn't there any more is taken off the list, with a note in the
  status bar and the Output. When Studio starts without a file, the
  Welcome page offers the last project with the files you had open in it,
  a card with the keyboard's focus (Enter reopens it all, tabs and layout
  as they were); Tools ▸ Settings ▸ General turns the offer off. Markdown
  files opened on their own are on the list too. `RPanel.SetFocus` (a
  card that takes the focus).
- **Programs run from Studio get your Terminal's PATH** (Preview 4 S6). On
  macOS and Linux an app started from the Dock or Finder has the system's
  short PATH, so a program's `SHELL "php …"`, `command -v curl` or a
  Homebrew tool failed from Studio. Studio now reads your login shell's
  PATH once at start-up (`$SHELL -l -i -c` printing it between markers, in
  a session of its own with no terminal, three seconds at most; it only
  reads, never writes your shell's files), else uses its own PATH plus the
  usual folders that exist (`/opt/homebrew/bin`, `/usr/local/bin`,
  `~/.cargo/bin` …). Run, Build and Run in Browser get it, and so do the
  program's `SHELL` and `RUN`. Tools ▸ Settings ▸ General shows the folders
  and where they came from, with Refresh. Windows and the web are
  unchanged. New `RProgramSession` members: `ShellPath`,
  `ShellPathSource`, `ReadShellPath`, `RefreshShellPath`. Manual: [Projects
  in RapidR Studio](docs/manual/studio-projects.md).
- **The reference tells the truth** (ROADMAP K7): `rapidr lang truth` checks the
  language registry, which is where RAI's language card and every model's
  knowledge come from, so a wrong signature there taught every model a wrong
  call. It makes every `syntax` line of a statement, directive and builtin into
  programs (a metavariable such as `name` or `expression` gets a valid stand-in,
  each `[optional]` part and each `A|B` choice is tried alone) and compiles them;
  it compiles every `example`; and it **runs** every example that has the new
  **`expect`** facet (the lines it prints) in a scratch folder and compares. The
  runtimes accept any number of arguments and ignore the extra ones, so nothing
  but a program whose output depends on every argument can prove a signature:
  an example with an `expect` calls the method with variables named like its
  parameters, in order (`row = 1`, `col = 0`, `PRINT d.Cell(row, col)`), which
  ties the documented order to what it prints, so swapping `Cell`'s documented
  parameters back fails the check. In the families where two neighbouring
  parameters of one kind make a swapped call still compile (RDataFrame, RNum,
  RCanvas, RBitmap, the string builtins) every such method has an `expect`: 87
  examples run today. In `tools/regress.sh`'s unit stage, and a cargo test
  (`crates/rapidr-cli/tests/lang_truth.rs`); `--verbose` lists what is not run
  and why, `--emit DIR` writes the `expect` programs for the other runtimes.
- **The debugger's own icons and a value row that draws a value as what it
  is** (Preview 4 S10, phase 1: the icons and the UI kernel's `value_view`). A
  new two-tone icon category, `debug/`, our own drawings hinted at 16, 24
  and 32 px in every look: whole and decimal numbers, text, True/False,
  arrays, handles, empty, a FUNCTION's result, and the sections Locals,
  Globals, Watch and a call on the stack; new actions for a breakpoint with
  a hit count, Add Watch, Copy Value and Edit Value; a "changed since the
  last stop" mark. The UI kernel's `value_view` draws one value as a row —
  its kind's icon (a component's own), its name, ` = `, its value in the
  code editor's colours, an empty text as a quiet `""`, a colour as a
  swatch and its name (`FontColor = ■ clWindowText`, drawn in the theme's
  own colour), an array with its count (`Array(0 TO 64) · 65 items`), a
  changed value highlighted and marked — and gives screen readers a
  sentence for it ("FontColor, colour clWindowText"). It is built for
  thousands of rows (each icon's picture made once and shared by every row
  that shows it), and the Properties panel and the code's helpers will use
  it too. New theme colours, readable by programs through
  `Application.ThemeColor`: `debug.name`, `debug.number`, `debug.string`,
  `debug.keyword`, `debug.constant`, `debug.type`, `debug.dim`,
  `debug.punctuation`, `debug.changedback`, each 4.5:1 on the window (7:1
  in high contrast). Every icon is in the [icon catalog](docs/manual/icons/index.html).
- **RapidR Studio: an editing layout and a debugging layout** (Preview 4
  S9). While you edit, the bottom panel spans the documents' whole width,
  with Output (its Output, Build and Problems pages), Immediate and
  Breakpoints as tabs; a long Problems message shows whole in a tooltip.
  When a run with the debugger starts, Variables, Watch, Call Stack and
  Breakpoints appear beside Output (Variables in front at a pause); when
  it ends, they go. Each layout keeps its own sizes across restarts.
  View ▸ Debug Panes (Ctrl+Shift+D) shows or hides them by hand (shown so,
  they stay); Tools ▸ Settings ▸ General ▸ Debug panes: Automatic, Always or
  Never. The switch is said in the status bar and never takes the keyboard.
  The Call Stack is now one of the debug panes (no longer in the Toolbox's
  place). See [Debugging: two layouts](docs/manual/debugging.md#two-layouts-editing-and-debugging).
- **Splitters you can see, grab and move from the keyboard** (RapidR
  Studio's panes, the document groups, RSplitter in RapidR's look): quiet
  at rest, the panes' own gap with a short grip in its middle (two new theme
  tokens: `splitter_grip`, 3:1 against the gap in light, dark and high
  contrast, and `splitter`, a soft 1-pixel line along the middle where no
  pane edge shows it, crisp in high contrast); the whole gap in the accent
  once the mouse rests on it (150 ms, no flicker passing over) and while it
  is dragged, in the light look too; an 8-pixel grab area over a 4-pixel
  gutter; where two splitters meet, the corner moves both; a double click
  puts a splitter back where it was. Window ▸ Resize Panes (Ctrl+F6) gives
  the keyboard to the splitter beside the active pane: arrows move it 10
  pixels, Shift+arrows 50, Alt+an arrow across it the splitter it meets,
  Home / End as far as it goes, Tab the next one, Enter or Escape back. A
  screen reader hears a separator whose value is a percentage. RDockManager gains `FocusSplitter` and `PaneExtent(Name,
  "width" | "height", [Extent])`. Programs' RSplitters keep their geometry;
  only the drawing changes. See [Panes and splitters](docs/manual/getting-started.md#panes-and-splitters).
- **`END n` ends the program with exit code `n`** (RapidR's; RapidQ's `END`
  takes no number), so a console tool can fail a build:
  `IF failures > 0 THEN END 1`, then `echo $?` (or `%ERRORLEVEL%`) says 1.
  `n` is any numeric expression, stored as a LONG is; a bare name must be
  one you declared (`DIM code AS INTEGER = 5`, then `END code`), so a typo
  in a block's ending — `END SUBB`, `END FUNCTON` — stays RapidQ's syntax
  error ("did you mean END SUB?") instead of quietly ending the program; a plain `END`, running
  off the end of the program, `END IF` and `END SUB` are as they were. It
  works under `rapidr run`, in interpreted and native builds and in programs
  with windows; in a web page the program ends and the number is ignored.
  A new conformance check, `.expected-exit`, asserts a program's exit code.
  See [the manual](docs/manual/language.md#end-and-the-exit-code).
- **`OnMessage`, another name for a form's `WndProc`**: `OnMessage = MySub` in
  the form's CREATE (or `Form.OnMessage = MySub`) is the same event as
  `WndProc` — the same four parameters (`Hwnd, Msg, wParam, lParam`), the same
  delivery on every runtime: Windows' messages on Windows, the system tray
  icon's on macOS, Linux and the web — so code written for either name runs
  the same. In the language reference (`OnMessage` on RForm, marked RapidR's)
  and the manual's [WndProc section](docs/manual/components.md#windows-messages-a-forms-wndproc).
- **Programs in every language** (docs/program-languages-plan.md, lane T1):
  `TR$("Save file")` gives the text in the program's language from PO files
  in a `lang` folder beside the program (`lang/es.po`, `lang/pt-BR.po`,
  `lang/zh-Hans.po`), read by RapidR's own small reader; `TRN$("{n} file",
  "{n} files", count)` picks the language's plural form (its `Plural-Forms`);
  `{name}` placeholders and contexts (`TR$("Open", "menu")`).
  `Application.Language` follows the system (or the browser) until the
  program sets it, and falls back `es-AR` → `es` → the program's own text;
  `Application.OnLanguageChange` fires when it changes. The same on the
  interpreter, native builds and the web. A new example,
  examples/translations/languages.rr, and a manual page,
  [Your program in every language](docs/manual/translations.md).
- **A built program takes every file it needs** (lane T1b): the files in
  folders below the program's that its source names (`"art/logo.svg"`,
  `"data\sample.csv"`) go with it, in their folders, and so does its
  `lang/` folder (the `.po` files) when it translates — in every kind of
  build (native, interpreted, the web, `bundle-bc`), RapidR Studio's
  builds, and `rapidr examples copy`. A name that leads out of the
  program's folder (`"../secret.txt"`, an absolute path) is never copied:
  the build says so in a warning. A web page gets `lang/index.txt`, the
  list of its translations, and asks only for those: a language without a
  file costs no request (before, up to five failed requests).
  [Building apps](docs/manual/building-apps.md#the-programs-data-files-go-with-it).
- **Pictures a built app came with, wherever it's started**: started by a
  terminal or a script in another folder, a program whose picture file
  (`RImage.Load "art/logo.svg"`, a `BMP`, an `Icon`, an `RImageList`'s
  files, a toolbar's or DirectX picture) isn't in that folder reads the one
  it came with — the rule `RCSV` and `RJson` already follow, now one helper
  for both (`command_line::read_or_programs_own`). A file in the current
  folder still comes first, as in RapidQ; `FILEEXISTS` and `OPEN` are
  unchanged. Native and interpreted builds; the web reads the page's files
  as before. tests/built_elsewhere.mjs.
  The same for the other files a program loads: sounds (`PLAYWAV`,
  `PLAYSOUND`, `RWave` / `RMIDI` / `RVideo`'s `Open`, a DirectX sound's
  `FileName`), `.X` models, and a list's, grid's or tree's `LoadFromFile`.
- **Versions, the engine proved** (lane G0, not in Studio yet;
  [docs/versions-g0.md](docs/versions-g0.md)): a new crate, `rapidr-vcs`,
  built on gitoxide 0.89 (pure Rust, MIT OR Apache-2.0). It can start
  versioning a folder, save a version, list what changed (with hunks),
  show the timeline, find who wrote each line, start experiments, bring
  one into another (fast-forward or a three-way merge; conflicts are
  reported and nothing is touched) and share with a folder. It never runs
  hooks, filter, diff or merge drivers, credential helpers or the `git`
  command. Every repository its tests write passes the system's
  `git fsck`, and agrees with `git status`, `git diff` and `git blame`.

### Changed
- **RDiffView marks changed words, not characters**, and its diff is the
  shared engine's (`rapidr-diff`): a run of only added or only removed lines
  now sits with its block (a new SUB with its trailing blank line). The old
  Myers module in `rapidr-value`, which followed GNU diff's structure, is
  gone (docs/licensing.md §8 item 6).
- **The `.rr` examples no longer include RAPIDQ.INC**: a RapidR program has
  RapidQ's constants built in, so the 18 examples that still had
  `$INCLUDE "RAPIDQ.INC"` (and the RMarkdownView snippet in the manual) lose the
  line and run the same on every runtime; `.bas` programs and `examples/rapidq/`
  keep it, as RapidQ needs it.
- **RapidR's own words** (Robert, 2026-10-21): the code comments, docs,
  manual, plans, examples and Studio's help text describe RapidR in RapidR's
  own words, with no other IDEs or products as models or comparisons;
  RapidQ's behaviour is stated as RapidQ's. A check
  (`tools/check_words.py`, in `tools/regress.sh`'s unit stage and alone as
  the `words` stage) fails when such a word comes back outside
  `tools/words-allowlist.txt` (fonts' names, build tools a platform needs,
  legal and history texts, fixture data).
- **Another company's trademark for code completion is gone from this
  changelog and the v2.117 and v2.118 release notes** (legal review of
  2026-10-22): they say "code completion" and "code help" in plain words. The
  words check now scans `CHANGELOG.md` and `docs/release-notes/` too, so the
  trademark cannot come back.
- **Notices: every file in a licence-named folder is a notice** (legal review
  of 2026-10-22). `rapidr notices` used to pick up licence files by their
  names only, so the two MIT notices `faster-hex` (a gitoxide dependency)
  keeps in `LICENSE-THIRD-PARTY/fast-hex` and `LICENSE-THIRD-PARTY/Rust Project
  Developers` would have been left out of `THIRD-PARTY-NOTICES.txt` once
  Versions joins Studio. A folder called `LICENSE-THIRD-PARTY`, `licenses`,
  `LICENSES` or the like now gives every file inside it. Across the 711
  packages of the workspace's graph this adds exactly those two files; a unit
  test with a folder shaped like `faster-hex`'s keeps it so.
- **Cryptography: the legal texts say what is really there, and the Linux
  keyring uses the system's OpenSSL** (legal review of 2026-10-22). LEGAL.md,
  LICENSES.md and docs/licensing.md said no encryption code was compiled into
  a program. That was wrong: the MySQL client (`mysql_common`) has its own
  RSA-OAEP and SHA-1/SHA-256, and on Linux and the BSDs the keyring client
  (`secret-service`) had AES-CBC, HKDF and a Diffie-Hellman key exchange
  compiled in. They now list exactly what remains (docs/licensing.md §2.5),
  and README, the announcement, the manual and the release templates say "no
  TLS library" instead of "no encryption code". On Linux and the BSDs the AI
  keys' keyring session now takes its AES and key derivation from the system's
  OpenSSL 3, which HTTPS loads anyway (`rapidr-ai` asks for `crypto-openssl`
  instead of `crypto-rust`): `aes`, `cbc`, `hkdf` and their helpers are gone
  from `Cargo.lock` and banned in `deny.toml`, `notices.rs` and
  `tools/check_notices.py`. Checked on Ubuntu 24.04: a key stored in GNOME
  Keyring reads back and deletes. LEGAL.md has a new "Export" section: open
  source, standard cryptography, mostly the system's.
- **LEGAL.md has a "Trademarks" section** (legal review of 2026-10-22) for the
  products RapidR works with: Git, GitHub, GitLab, Bitbucket, SQLite, MySQL,
  MariaDB and PostgreSQL, each with its owner's own attribution wording
  (taken from the owner's trademark page), the symbols the owners ask for,
  and the line that RapidR is not affiliated with or endorsed by them. Names
  appear as plain text, never as logos and never inside a RapidR name.
- **An open licence question and a rule for Versions' diff and merge code**
  (legal review of 2026-10-22, docs/licensing.md §8, item 6): the blob diff
  in `gix-imara-diff` (Apache-2.0) has comments that name Git's xdiff
  internals and slider-heuristic constants that match Git's, so where it
  comes from is recorded as open; nothing of it ships yet. RapidR's own diff
  and merge code (Versions lane G2) is to be written from Myers' 1986 paper,
  never from xdiff or any other implementation.

### Fixed
- **A FUNCTION's parameter or local named `Result`** means the FUNCTION's
  own value (RapidQ's RESULT) on every runtime, as RC.EXE has it: in
  `FUNCTION F (Id AS STRING, Result AS STRING)`, `F = Id + Result` adds
  the result so far, and `Result = x` sets the value returned. The
  interpreter (and so the web) read the parameter while native builds read
  RESULT, so a program answered differently built (RapidR Database
  Studio's MCP answers came back empty only in its built app). One pass,
  `rapidr_ast::function_result`, for both compilers; a global `Result`
  doesn't reach inside a FUNCTION either (native builds read it). The
  editor warns about such a parameter or DIM (`hidden-result`).
  Conformance case `function_result_name` (RC.EXE's output, the Windows VM).
- The studio flow `ai-designer-rai` failed on the web now and then: its 13
  steps, 400 ms apart, outlasted its 5-second capture on a slow start, so
  the Model pick looked lost. It now waits for the model list
  (`ai:designwait`) and captures at 8 s. Not a product bug.
- RAI's models list from a server running with no model yet (Ollama answers
  `"data": null`) is an empty list, not "not a models list"; Set Up RAI
  says the server has no model yet and to download one in it first.
- **No test run reaches the real keychain.** `tests/studio_shell.mjs` ran
  Studio without `RAPIDR_AI_KEYSTORE=memory`: its `settings-ai` scene asked
  the keychain for the AI keys, macOS asked for the login keychain's
  password, and each run timed out with no capture. A redirected registry
  (`RAPIDR_REGISTRY`) now means the memory keystore unless
  `RAPIDR_AI_KEYSTORE=system`; the shell and add-form harnesses set it too,
  and the shell's runs keep the AI's chats in their own folder.
- **The debugger names a parameter by its source name** in a SUB that takes
  VARPTR of a global with the same name: `MyMsg` in Variables and in the
  expressions typed there, not the internal `MyMsg__param`.
- **INSTR with a start that isn't whole** gives RapidQ's real place (RC.EXE):
  the search begins where the start rounds half to even and the place counts
  from the start as given, `INSTR(2.5, "abcabc", "c")` = 3.5
  (`rapidq_instr_real_start`).
- **FILEEXISTS with wildcards** (RC.EXE): `FILEEXISTS("*.bas")` is 1 when the
  first name the folder lists for the pattern is a file, as in RapidQ (`*` and
  `*.*` meet the folder's own `.` first and are 0, a pattern only a folder
  matches is 0), on the desktop and, among the program's files, on the web. A
  `?` at the end of a name or before its dot may match nothing, as Windows reads
  it — for DIR$ and QFILELISTBOX's masks too.
- **`RINSTR(text, find)` compiles natively again with any generation of the
  compiler** (found by RapidR Database Studio): the two-argument form keeps its
  runtime function `rp_rinstr(text, find)`, the form with a start has its own
  (`rp_rinstr_from`); both agree with RC.EXE on the VM, native builds and the web
  (`rapidq_rinstr_two_args`).
- **An event handler's STRING or number parameter gets "" / 0, not the Sender**
  (RC.EXE): `OnClick = H` with `SUB H(T AS STRING)` reads "" in T when the event
  fires (a number parameter 0), as RapidQ's first call does; an object parameter
  is still the Sender. The same on the VM, native builds and the web — they gave
  the Sender's name ("CK" interpreted, "ck" native).
- **`True` in a RapidQ program (`.bas`) is RapidQ's** (RC.EXE): without
  `$INCLUDE "RAPIDQ.INC"` it is an undeclared name that reads 0 (`IF True THEN`
  doesn't run), with the code editor's "True is a RAPIDQ.INC constant" warning as
  for mbYes; with the include it is 1. A RapidR program (`.rr`) keeps `True` = -1
  built in. Four test fixtures that meant RapidR's True now include RAPIDQ.INC.
- **RapidQ-exact against RapidQ's own compiler (lane COMPAT, docs/rapidq-ground-truth.md)**:
  `RINSTR(start, s, find)` uses its start on every runtime, as RapidQ does (only an
  occurrence that ends by character `start` counts: `RINSTR(4, "hehe they", "he")` is 3; an
  empty `find` gives 0); `VARPTR(name)` inside a SUB whose parameter has a global's name is
  the global's address, as in RapidQ (a parameter is invisible to VARPTR there); a
  `RUSTSTART … RUSTEND` block in the main program, or with code after it in a SUB, no longer
  stops the parser at the next line (native builds run it, the interpreter and the web say
  it's native-only). Confirmed equal: FILEEXISTS (1 / 0, so `NOT FILEEXISTS` of a file is -2,
  true), Rectangle (outline only) / RoundRect (filled) / Circle (outline and fill colours, the
  right and bottom edges left out), RAPIDQ.INC's names reading 0 in a `.bas` without the
  include, and an OnClick handler with a number parameter. New conformance cases
  `rapidq_rinstr_start`, `rapidq_varptr_parameter`, `rapidq_handler_wrong_params`,
  `rapidq_fileexists`, `rapidq_shapes`; the reference documents Rectangle, RoundRect and
  Circle as RapidQ draws them.
- **A project's executable is named after the project**: RapidR Database
  Studio's build made `Contents/MacOS/main`, after `src/main.rr`. A build of a
  project now names the executable after the `.rrproj`'s `name` (letters,
  digits, `-`, `_` and `.` kept, any other run one `-`, Windows device names
  made `-app`, 64 characters at most: `rapidr_package::exe_name_for`) — the
  Mac app's `Contents/MacOS/<exe>` and `CFBundleExecutable`, the Windows
  `.exe`, a Linux AppDir's `usr/bin/<exe>` and desktop entry, native and
  `--interp` alike — and a single file keeps the file's name. `--name` and
  `app_name` still change only what people see. The project is found beside the
  source or one or two folders above it when its `main` is exactly that file
  (`src/main.rr`); `--project` says it outright. There is no `$OPTION EXENAME`
  in RapidR's options, so none was added. Tests:
  `the_executable_is_named_after_the_project` (all three systems, the plist and
  the desktop entry), `a_projects_executable_name`,
  `a_project_above_its_main_file_is_found`; checked by building a project
  interpreted (`Database Studio.app/Contents/MacOS/Database-Studio`).
- **A program's standard input is read line by line as it arrives**:
  `OPEN "/dev/stdin" FOR INPUT AS #f` read the whole input before the first
  `LINE INPUT #f` returned, so a program that answers one line with one line
  (an MCP stdio server) never answered until its client closed the pipe. The
  file is now read as it comes: `LINE INPUT #f, s` returns once its line has
  arrived, `EOF(f)` waits for the next byte or the end, and what is behind the
  position is dropped, so a server that runs for days does not grow
  (`rapidr_value::basic_files`, the one implementation of the VM, the native
  builds and the web runtime). The console's `INPUT s` already did. The web has
  no standard input: there the name is a file that is not there (the manual's
  Files page says so). Tests: `a_stream_gives_each_line_as_soon_as_it_has_come`
  and `tests/stdin_lines.mjs`, which pipes two lines with a pause and checks
  that the first is answered before the second is written, on the VM, in an
  interpreted build and (with `--runtimes native`) in a native one.
- **RPlot bar names are tested at every scale** (Database Studio reported
  every other name dropped at `RAPIDR_SCALE=2` where scale 1 showed all 8):
  not reproduced with an 8-bar chart in a fixed rectangle, aligned to the
  client area, bound to a data frame (`examples/data/dataframe.rr`) or after a
  scripted resize — the chart's labels are fitted in logical units (its ops
  are made at the component's logical size, the CPU renderer only scales them)
  and the same names show at 1x, 1.5x, 2x and 3x. Two guards keep it so: the
  kernel test `the_names_shown_are_the_same_at_every_scale` (the display list
  at four scales over widths 150 to 500) and the renderer's
  `a_charts_names_are_drawn_at_every_scale` (each name's place holds ink at
  1x, 1.5x and 2x). If the program that showed it is sent, it is the next
  thing to try.
- **A scripted `lv.__item_N` picks a list view's item**: `RAPIDR_TEST_EVENTS`
  picked an item of an RListBox, an RComboBox and a tree but did nothing for an
  RListView, so a test could not select a report row. The list view answers
  `__item_N` like the others: a click on the item's caption (any view; an item
  below the fold is brought into view first), then OnClick
  (`ListView::click_point`). Test `a_scripted_item_pick_selects_a_list_view_item_in_every_view`
  and the parity fixture `listview_item_pick`.
- **A message box is as wide as its title needs**: `MessageDlg` (mtInformation,
  one OK button, a short text) made a window 99 pixels wide, narrower than the
  title bar needs for "Information" with its icon and close box, so the title
  was cut off. The width now also fits the title (`frame::inside_for_title`:
  the icon, the title in the frame's own font, the buttons; `message_layout`
  takes it as `min_w`), in RapidR's light, dark and high-contrast looks and in
  the classic one; a wider text or more buttons keep their own width, and the
  input box too. A real macOS window has the system's title bar, which keeps
  room for the traffic lights and a margin and cut the title to "Infor…" even
  so: the desktop tells the dialogs what it keeps (`Windows::title_reserve`,
  `rapidr_ui_host_winit::platform::title_reserve`: 84 and 20 pixels on macOS,
  none under a test, so the captures are the same everywhere). Checked on the
  real screen: "Information" in full. Tests:
  `a_dialog_is_as_wide_as_its_title_needs`, `a_system_title_bar_keeps_room_around_the_title`.
- **A file opens with its squiggles**: a `.bas` with warnings listed them in
  Problems but the editor underlined nothing until the first edit, because
  diagnostics were only asked for after typing. A text set from outside
  (`LoadFromFile`, `Text =`) is now diagnosed at the editor's next tick, before
  it is even drawn (`Model::diagnose_wanted`). Flow `editor-squiggles-on-open`.
  The same pass gives the file's semantic colours at once (a SUB's name, a
  component's, a constant's), so the Studio goldens of the scenes that open a
  file (`debug-variables`, `-callstack`, `-breakpoints`, `-watch`, `-datatip`,
  `-error`, `import-report`; light, dark and high contrast, 1x and 2x: 24
  images) were approved: only the code area differs, in those colours.
- **The Assistant's token meter is never cut off**: with cached tokens and
  prices the line (`2,000 tokens sent (200 from the cache), 40 received · about
  $0.0040`) is longer than the pane is wide, and the label, one line high
  above 360 pixels, hid "· about $…" under the note. The meter's height now
  follows its text wrapped at spaces (`AIMeterHigh`, `AIShowUsage` in
  `ide/ai.inc`: one line is 18 pixels as before, each more adds a line).
  Flow `ai-meter-long`, in the three looks.
- **The web conformance run has the translations**: `i18n_tr`, `i18n_trn` and
  `i18n_broken_po` failed on the web because the page was handed no `lang/`
  catalogs. The runner now gives a program that translates the case folder's
  `lang/*.po` as its project files, as a web build carries them
  (`rapidr_project::ship`).
- **The debugger's Immediate and Watch see a `.rr` program's built-in
  constants**: `? mrYes` printed nothing while the program itself read 6. What
  the debugger compiles (Immediate, Watch, data tips, a changed value) now goes
  through the same built-in constants pass as the program
  (`rapidr_ast::builtin_constants::lower_snippet`, called by the debugger's
  `compile_snippet`), the program's own names (locals, globals, routines, a
  `DIM mbYes`) winning. The compiled module says it came from a `.rr` (bit 2 of
  the `.rrbc` flags; an older runtime ignores it). RapidQ's value of `mbYes`
  is 1 (`mrYes` is 6); a `.bas` program has none, as before.
- **Reference entries that taught a wrong call** (found by the new check,
  `rapidr lang truth`; the language reference, its manual pages and the
  assistant's language card say what the runtimes do now): `RDataFrame.Iloc` is
  `Iloc(start, [end])` (`Iloc 1, 3` keeps two rows), `Select` takes ONE text,
  `Select "name,city"`, `GroupBy` is `GroupBy(column, [aggFunc])` (there is no
  `aggColumn`), `Nlargest` and `Nsmallest` are `(column, [n])`, `Apply` takes a
  `[decimals]` for `round`, `Sort` names its columns in one text; `RNum.FromList`
  takes one comma-separated text, `FromList "5,6,7"`, `Randn` has `[mean]` and
  `[stddev]`; `QUICKSORT`'s last argument is `ASCEND|DESCEND` (0 or 1); `INSERT$`
  takes the text to insert first (`INSERT$(Part$, Source$, Index)`); the parameters
  of the aliases (`Group_by`, `Join`, `SetItem`, `Clamp` …) are shown, optional
  ones are in brackets, a `TYPE … EXTENDS QOBJECT|component` clause reads as one,
  and parameter names no program can use as a variable (`String`, `Replace$`,
  `Insert$`, `Str$`, `function`) are renamed (`Text$`, `Overwrite$`, `Part$`).
- **RapidR Studio's panes at start-up** (Preview 4 S8): a pane placed while
  the window was still being given its size (restored, maximized or scaled
  at start-up) kept its contents at their old size until the window was
  resized — the Breakpoints list a narrow strip with its "When" field under
  it, the Assistant's input row. The dock manager now lays out what is
  inside every pane it gives a new size (its aligned and anchored
  controls, its scroll bars) and fires the pane's OnResize, on the desktop
  and the web.
- **Two wrong reference entries** (found by RapidR Database Studio, which
  followed them): a data frame's `Cell` and `SetCell` take the **row first**,
  `Cell(row, col)` and `SetCell(row, col, value)`, as the code always did —
  the reference said `(col, row)`; and `CONST` is one constant to a statement,
  `CONST name [AS type] = expression`, as in RapidQ — the reference offered a
  list (`CONST a = 1, b = 2`), which was always an error. The language
  reference, the manual's reference pages and the assistant's language card
  now say what the code does.

### Changed
- **RapidQ's standard constants are built into RapidR programs** (master
  plan M0, lane M0-CONST): a `.rr` program uses `mbYes`, `clRed`,
  `crHandPoint`, `alClient`, `VK_ESCAPE` … and the `QBColor` array (an
  assignable INTEGER array, 0 TO 15) with no `$INCLUDE "RAPIDQ.INC"`, the
  same numbers the include gives. A name the program declares or stores
  into itself wins where it does (`DIM mbYes AS INTEGER = 5` prints 5, a
  SUB's parameter or local is the SUB's); only the constants used are
  compiled and the debugger lists none of them; `True` stays -1. Resolved
  once in the front end both backends share (interpreter, native builds and
  the web alike); the line in a `.rr` stays harmless. RapidQ programs
  (`.bas`, `.rqw` …) are unchanged: the constants come with the include,
  without it the names are RapidQ's undeclared 0s and Studio's warning and
  quick fix stay. In a `.rr` program Studio's completion offers them as
  built-in constants (no include added), hover says so, the designer writes
  their names, and RAI's instructions and recipes no longer include
  RAPIDQ.INC. Conformance cases rr_builtin_constants, rr_shadow_constant and
  bas_implicit_constant; the manual's [Constants](docs/manual/language.md#constants-clred-mbyes-vk_escape).
### Fixed
- **RAI after Robert's evening with DeepSeek** (docs/preview4-plan.md R3,
  R10, R11; the assistant in Studio and RAI in programs):
  - **Typing while RAI works queues the message** (R3): Send or Enter
    during an answer shows *Queued: …* above the box (× takes it back) and
    sends it when the answer ends; it never stops the answer. **Stop** is its
    own button, and gives a queued message back to the box, unsent.
  - **The answers stay in the user's language** (R10): every request ends by
    naming the language of the user's latest message (kept while a message
    is too short to tell; Studio's or the program's language before the
    first); a long chat's summary is written in it and says it. Two answers
    had come in German to a user who wrote English. RAI's new
    **ReplyLanguage** says how: `""` nothing (the default, as before: a
    translator's answers aren't in its questions' language), `"auto"` the
    user's (Studio's assistant), or one language always; **UserLanguage**
    is the language RAI read in the latest message.
  - **The apply/reject log says only what happened** (R11): each decision
    once (a review opened again said *You rejected* for changes just
    applied); *wasn't applied: the file changed* instead of *rejected*; *the
    change … was replaced by a newer proposal*; a waiting change whose file
    you edited is set aside, said. A change that adds a blank line and a new
    SUB after a file's last line applied nothing while the chat said *You
    applied the change* (its patch started past the file's end): it applies
    now, and a patch an editor refuses is said as such. After applying, a
    module changed and not saved counts as its editor has it (Studio said
    *the program has 1 error* of a program that had none).
  - **Targeted edits** (R11): a few lines' change sent as a whole file of
    more than 40 lines is refused, and the model is told to send the parts
    (propose_edit's search-and-replace edits); the cards count the lines
    that really change (a 2-line fix had shown `+382 −384`).
  - **A cut-off or broken change is refused** (R11): a file ending in the
    middle of a line (`END FUNCTI`) or with a SUB still open, or with a
    syntax error, is never proposed — in a module too —, a × card says so.
  - **Waiting changes are checked together** (R11): changes not yet
    reviewed wait across questions; the model reads them, and the compiler
    checks the project with all of them applied (main.rr calling a SUB a
    waiting module change adds compiles; it was refused).
  - **A request that never reached the provider is tried once more**
    (R11): the name not found (DNS), no connection, a connection closed
    before any answer — 2 seconds later, by itself; never a refusal.

## [2.120.0] — 2026-10-19 (Preview 3: RAI)

### Added
- **A new program from the assistant opens straight in Studio**
  (docs/ai-ux-plan.md §1): "Build a scientific calculator" ends with the
  program open — its form in the designer — in your projects folder
  (Documents/RapidR/Projects), in a folder named from its title ("Scientific
  Calculator" → `ScientificCalculator`, then `ScientificCalculator2` …), with
  a project file that keeps the title. No dialog, no folder question. A
  banner across the top says what happened, with **▶ Run It** (the keyboard
  is on it: Enter or F5 runs), **↺ Undo** and **What Did It Do?**; a program
  that still has errors opens all the same, with **Run Anyway** and **Ask It
  to Fix**. Undo closes it and removes exactly the files Studio wrote for it
  (an explicit list, each checked to be inside its folder), asks first when
  you changed or added files there (yours stay), and opens the project you
  had before. Nothing runs until you press Run. Your unsaved changes are
  asked about before anything opens. Tools > Settings > AI: **Ask before the
  assistant creates or changes files** (off by default) keeps the preview
  first.
- **The assistant's chat tells the truth, in cards** (docs/ai-ux-plan.md
  §3): what the tools did is written by Studio as small cards — the recipe
  it started from, the program created (Open, Undo), a change proposed (+/−
  lines, Review), the errors fixed — and the model's own short reply comes
  after them. A reply that claimed a change before any tool made it shows
  as one muted line ("An earlier draft said this was done before it was —
  Studio asked it to actually do it"), never as the claim. One status line:
  while the assistant works it is a short progress checklist ("✓ Found an
  example · ✓ Wrote the program · … Working…"), read aloud by screen
  readers once a step. RAI's new **Steps** property lists them.
- **Recipes: ten short working programs to start from** (`examples/recipes/`):
  a calculator with square root, x², log, sin, cos and tan (one handler for
  every key), a tip calculator, a stopwatch, a to-do list, a unit converter, a
  quiz, a drawing pad, a password generator, a CSV viewer and a server manager
  (a list with Start / Stop / Restart). Each is clicked by the examples suite
  on every runtime. When you ask RapidR Studio's assistant for a new program,
  Studio finds the closest recipe itself (a keyword search on your computer,
  in English, Spanish and Portuguese) and sends it with your request, so the
  model adapts a program that works instead of inventing one; a
  `find_example` tool finds one later. *What was sent* shows it. Studio lists
  them under **Open an Example** too.
- **Friendly hints in the assistant's code checks**: under the compiler's own
  messages, the exact fix for habits from other languages (`!=`, `||`, one
  `AS` per name, braces, `[ ]` lists, `//` comments, `BOOLEAN` and `COLOR`
  types, an event's argument types), for a name never given a value (with the
  closest real one), a RapidQ constant used without `$INCLUDE "RAPIDQ.INC"`
  (`alClient` is 0 then), a window never shown (`Form.ShowModal` missing),
  and a handler that does nothing yet or only says what "a real
  implementation would" do; a component's members beside a property it
  doesn't have. The hints are their own module, for Learn with RAI later. The
  compiler and `rapidr run` are unchanged.
- **The assistant's layout help**: a `layout` tool lays out a grid of buttons
  (a keypad, a toolbar) with exact positions and one shared handler, and says
  the client size the form needs; once a program compiles, its check says
  what a person would see wrong — a control cut off by its form, controls on
  top of each other, a caption longer than its button.
- **Click-tested AI evals** (`tests/ai-eval/apps/`, `ai_eval.rs`): ten app
  requests, each program run headless after the conversation and used as a
  person would (buttons found by their captions); score = it compiles and the
  click test passes. Results in [docs/small-models-plan.md](docs/small-models-plan.md#results).
- **Showcase: eight bigger programs, written from ideas alone** in
  `examples/showcase/` (the data is made up by the programs; nothing is copied).
  Fireworks (thousands of sparks, each with its own physics, shapes worked out
  with `RNum`), a lit 3D planet with a ring and a moon, pi by throwing
  darts (`RNum` throws and counts up to 300,000 a second), four sets of points
  with the very same statistics, a Markdown notes app (live preview; files on
  the desktop, the browser's storage on the web), a snake game for the
  keyboard alone (the computer can play), a signal generator with an
  oscilloscope and an FFT written in BASIC, and a MIDI visualizer that reads
  the `.mid` file itself and makes a 3D scene dance to it (`song.mid` is an
  original loop made by `make_song.rr`). Each runs as `rapidr run`, interpreted,
  native and in the browser; Studio lists them under **Open an Example** (the
  palette; the Welcome page keeps its cards); the manual has a
  [Showcase page](docs/manual/showcase.md) with a picture and the best lines of
  each. The examples test can now compare a written file with the shipped one
  (`sameFile`), copy another example's file beside a program (`extra`) and open
  the web page again to check what the browser kept (`reload`).

### Fixed
- **`UNLOADLIBRARY(DLLName)` works** (a user importing `Rapidq2.inc`, whose
  programs end with it): a DLL the program declared leaves memory, by the
  name `DECLARE … LIB` wrote (`"USER32"`, not `"USER32.DLL"`, as RapidQ), a
  later call loads it again; a name never declared does nothing; off Windows
  and on the web it does nothing, without an error. Interpreter and native
  builds.
- **The assistant's banner and status tell the truth about errors.** The
  count after a new program opens, and after changes are applied to a
  project (also one the assistant made earlier, under its banner), is Run's
  own: "…it still has N errors", *Run Anyway*, *Ask It to Fix*, and the
  status says the program has them.
- **The banner's "What Did It Do?" works**: the Assistant pane opens with the
  chat at the first card of the turn that made the program, and the keyboard
  on the chat (its links are Tab stops).
- **Examples for a request are chosen by what a program is, not by its words:**
  a recipe's title and keywords decide; its code only breaks a tie; words
  every request has ("just", "functions", "with") count for nothing; a word
  repeated counts once; and a request no recipe's title or keywords match
  gets no example, rather than a wrong one ("a scientific calculator with
  graphs" sent the Password generator once).
- **A list box scrolls to the item your program picks.** Setting `ItemIndex`
  (to the newest message of a message list, say) left the list at the top,
  the picked item out of sight. Now `ItemIndex` scrolls the least that shows
  the item, `TopIndex` makes an item the first row (never scrolling past the
  end), and `TopIndex` reads the first row shown, also after the user
  scrolled — in plain, `Columns` and owner-drawn lists, on the desktop, in
  native builds and on the web.
- **Studio for the web: Undo of the assistant's new program really removes
  its files.** The browser's saved copy was not told, so the project came
  back at the next visit; a file a program deletes with `KILL` now goes from
  Studio's saved copy too.
- **The chat's card links (Open, Undo, Review) work by the keyboard.** A
  Markdown view's links are Tab stops inside it, after its text and before
  the next control: a ring round the link, the view scrolling to it, Enter
  or Space follows it, Shift+Tab comes back from the box to the last link;
  a screen reader hears a link with its name. This holds for every
  `RMarkdownView`, Studio's Markdown preview and your own programs.
- **The assistant never refuses a program for its title** (Robert's walk,
  2026-10-18): "Scientific Calculator" was turned down as "a name that can't
  be a folder"; any title now makes the program, its folder made from it
  (accents dropped, words joined).
- **The example Studio sends with a request is labelled as Studio's**, "not
  from the user", so a model no longer thanks the user for "the recipe you
  provided"; the instructions ask for one or two sentences after the tools,
  without repeating names and paths the cards show.
- **One status in the Assistant pane**: the bar above the input no longer
  repeats the status sentence; it is one button that says what it opens
  (**Review the Changes…**, **Preview the New Program…**).
- **A new program the assistant proposed stays fixable until you create it.**
  A small model that found errors in its new program tried to read and fix
  it before you pressed Create, and got "no project open" — then lost its
  way. Now reading, checking and changing work on the proposed program's
  files, the Preview shows the latest version, and it stays across questions
  until Create (or New Chat). `create_project` without a name or main file
  takes them from its files. When the assistant only described a change, the
  words are plainer: *"The assistant didn't make the change you asked for.
  Ask again, or try a larger model."*
- **RapidR Studio's assistant works with small models on your own computer**
  (a 4B or 9B model served by mlx-serve, Ollama, LM Studio). A reply asked
  for up to 65,535 tokens on a model whose whole window was 12,000–26,000
  (and a reply nobody waited for kept the local GPU busy for many minutes);
  the server's own token counts were never read; the whole conversation,
  every file read and written, was sent again each time; an answer that
  said "I am proposing the edit, please review it" with no change behind it
  ended with "Answered."; an edit that missed only said to read the file
  again. Now: the window comes from the server's models list (the loaded
  model's, asked before a question), each reply gets a quarter of it (4,096
  at most on this computer, 300 seconds of quiet allowed); usage is asked of
  every compatible server (`stream_options`, asked again without it when a
  server refuses the field) and written to the audit log for each request;
  past 70% of the window earlier tool results and the file texts in earlier
  tool calls become short notes, then old exchanges go, never the
  instructions, the tools or the last question; an answer that claims a
  change no tool made, or no change after "go ahead / just do it", is sent
  back once ("call propose_edit, write_file or create_project now"), and
  if nothing comes the status says *"The assistant described a change but
  didn't make one. Ask again, or try a larger model."*; every proposal is
  compiled at once and a change that doesn't compile says so (to the model,
  which is asked once to fix it, and in the status); an edit that misses
  shows the file's closest lines, numbered, fits lines that differ only in
  indentation, and after two misses points to write_file; a file read twice
  unchanged in one answer isn't sent again; write_file with
  create_project's arguments does what they say; check_code names a
  property no component has ("RListBox has no property ColumnWidths",
  "did you mean ColCount?") — for the assistant only, the compiler and
  `rapidr run` unchanged; the instructions say text alone changes nothing,
  that it can't run programs (it sees the last run's output, its last 20
  lines, as data: `RAIProjectTools.RunOutput` / `AddRunOutput`), and the
  usual VB.NET / Python habits RapidR BASIC doesn't have. A headless replay
  of a conversation against a real model, `crates/rapidr-studio/examples/ai_eval.rs`
  with scripts in `tests/ai-eval/` (never in the suites), measured it.
- **A form's WndProc hears its window's Windows messages on Windows.** It
  heard only the system tray icon's, so a program waiting for `WM_USER + 1`
  from `PostMessageA(Form.Handle, …)` or for a MIDI keyboard's `MM_MIM_DATA`
  (winmm's `midiInOpen` with `CALLBACK_WINDOW`) heard nothing. Now, as in
  RapidQ, `WndProc (hWnd, uMsg, wParam, lParam)` gets every message Windows
  sends or posts to the form's window — the system's too (`WM_SIZE`,
  `WM_ACTIVATE`, the mouse, the keys) —, in native builds and interpreted;
  wParam and lParam are 32-bit numbers, what the handler returns is ignored,
  and a `DOEVENTS` inside it lets the next message in (all checked against
  RapidQ's own compiler). RapidR's redraws of a form's controls are not handed
  over as `WM_PAINT`, so a handler that changes a control on every message
  doesn't run for ever. Only forms with a WndProc are listened to; macOS, Linux
  and the web are unchanged (there WndProc hears the tray icon). New examples
  in `examples/windows/`: the messages of a form as they come, `PostMessageA`
  and `WM_USER + 1`, and a MIDI keyboard's notes; the manual's
  [Windows messages](docs/manual/components.md#windows-messages-a-forms-wndproc)
  section explains it.
- **RapidR Studio's examples work where RapidR is installed.** The examples
  there can't be changed (the macOS app, `/usr/lib/rapidr`, Program Files), and
  `million_rows.rr` "wrote" its data and said so while nothing was written. Now
  opening one — a Welcome card, File > Open Example, the palette, Recent, File >
  Open, the command line — opens your own copy in
  `Documents/RapidR/Examples/<name>`, made the first time with its include and
  data files (the very files `rapidr examples copy` takes: one implementation,
  `rapidr_project::program_files`) and never overwritten after; the Output says
  where it is, with **Copy again from the original** (also File > Copy Example
  Again from the Original…, which asks first). An install's examples are copied
  even where the user could write them (a macOS app dragged to Applications:
  nothing is written inside RapidR). A checkout's examples still open where
  they are. `RAPIDR_DOCUMENTS` names another Documents folder (the tests').
- **Built apps find their data files.** `rapidr build` (native and interpreted,
  every system) puts the data files the program names with the app — a macOS
  app's `Contents/Resources`, beside the `.exe`, an AppDir's `usr/bin` — and says
  so; an app the desktop starts (Finder and `open` start it in `/`, a Linux
  launcher in the home folder) works in that folder, as a RapidQ program
  double-clicked on Windows works in its `.exe`'s. Started from a terminal,
  programs keep the terminal's folder, and an RCSV or RJson that finds no file
  there reads the app's own. Console programs never move.
- **The data examples say when something failed**: `million_rows.rr` run where
  it can't write says so in its window (with why, the buttons off) instead of
  an empty grid and a "written" line; `linked_data.rr`'s and `json_source.rr`'s
  Save say "Couldn't save staff_edited.csv: this folder can't be written…";
  a missing `staff.csv` or `products.json` is said in the window, not "Row 1 of
  0"; `basics/files.rr` and `data/json.rr` too.
- `json_source.rr`: letters typed into the numeric Stock edit are refused (the
  status says why), the arrow keys in the grid update the status line, and
  grouped by category the first edit shows the category instead of going blank.
- `rapidr examples copy rapidq/shapes` (an example by its topic) works as
  `copy shapes` did; `rapidr examples` names the manual's pages where they are
  (an install's `share/doc/rapidr/manual`, a checkout's `docs/manual`, else
  on the web).

### Added
- `RDataFrame.Error` (why its last LoadFromCSV / SaveToCSV / LoadFromJSON /
  SaveToJSON failed, in plain words), `RJson.Error` (why its records couldn't be
  read as a data source), `REdit.DataError` (why what the user typed isn't its
  number field's value — the field keeps its value): RapidR's, every runtime,
  in the language registry.

### Changed
- **Events fire as RapidQ fires them** (checked against RapidQ's own compiler,
  RC.EXE, in the Windows VM). Setting a property from code now fires the event the
  user's change fires, inside the assignment: an `REdit`'s Text or SelText that
  changes, its OnChange; an `RRichEdit`'s Text, OnChange (twice once its form has
  shown, as RapidQ's); an `RCheckBox`'s Checked, OnClick; an `RRadioButton` turned
  on, OnClick; an `RScrollBar`'s Position, OnChange. A combo box's or list box's
  ItemIndex, a track bar's Position and a tab control's TabIndex fire nothing, as in
  RapidQ. Every runtime (the interpreter, native builds, the web). The table:
  docs/manual/components.md, "Events your own code fires".
- **A run-time error in an event handler no longer ends the program**: as in
  RapidQ, the handler stops at the line — with the handlers that fired it — and the
  program goes on answering its events, its timers ticking; the error is written to
  the program's error output (Studio's Output). In an event run by `DOEVENTS` the
  error also stops the code that called it (a main program's `DOEVENTS` loop ends,
  as RapidQ's). Under the debugger, Continue at such an error does the same — the
  next click stops at the error again. Desktop, native builds and the web alike.

### Added
- **OnEnter and OnExit on every component that takes the focus** (RapidR's, as
  Delphi's; RapidQ has OnEnter only on its lists, where it never fires, and no
  OnExit): the component left hears OnExit, then the one reached OnEnter, before a
  click's own events — by Tab, a click, SetFocus, a mnemonic. In the language
  registry (27 components) and RapidR Studio's Events tab; a breakpoint in either
  stops every time. Example: `examples/gui/focus_events.rr`.
- **The debugger's scenarios as flows on both hosts**: stepping inside a handler
  (Step Into, Step Out, Run to Cursor, Set Next Statement), a breakpoint on a loop's
  body (every pass) and on a one-line loop (once), a recursive FUNCTION (every
  level, the Call Stack listing them all), a breakpoint taken off and put back while
  the program runs, a breakpoint in an `$INCLUDE`d file (it opens there).

### Fixed
- **RapidR Studio on the web: a click in Studio while the program runs gives Studio
  the keyboard back** (the program's frame kept it: F9 or Ctrl+G went to the
  program).
- **A stop right after Continue shows in Studio on macOS** (Tab from a field into
  an OnExit breakpoint, Continue, the next field's OnEnter breakpoint): the system
  made the program active only after it had given the front back, so the stopped
  program stayed in front of Studio. A program stopped then hands the front back
  again for a moment (found walking the debugger on the real screen).
- **RapidR Studio's status bar shortens a long file name in its middle** (the line
  number kept: `Breakpoint: a_pro…he_status_bar.rr:6`); resting the mouse on
  the bar shows the whole. It was cut off at the end, the line lost.
- **RapidR Studio's dark and high-contrast themes are readable everywhere.** The
  debugger's Variables, Watch, Call Stack and Breakpoints rows and the Breakpoints
  pane's When / Hit / Log / On labels were black or dark grey on the dark ground:
  a QFONT assigned to a parent (Studio's `Dock.Font = UIFont`) counted as a chosen
  black. A font colour that is Windows' text colour (clWindowText, a new QFONT's)
  is now the theme's text, in every program; a colour set as RGB is painted as
  written, as before. Changed values take the theme's `debug.changed` colour, the
  debugger's trees the chrome's font and row height (as the Outline), and the
  Breakpoints strip's labels clear of their boxes.
- **Every Studio text reads at WCAG AA** in RapidR light, dark and high contrast:
  placeholders and values at their default (the new `secondary_text`, 4.5:1 —
  they were the disabled grey), the code editor's line numbers, every token on the
  debugger's line and on a run-time error's line, a run-time error's message, the
  Problems badge, the document view switch, the New Project dialog's picked card;
  a disabled tree draws no selection under its grey text; the designer's backdrop
  is black in high contrast (it was white, the empty designer's text unseen on it).
- **The designer's anchor pins** have a test on both hosts in every theme (the
  new read-only `RDesignSurface.Pins`: "Left on, Top on, Right off, Bottom off").

### Added
- **Approved images of RapidR Studio itself** (`tests/visual/studio/`, 75 PNGs, 12 MB):
  `tools/visual/gallery.py studio check` captures 21 scenes — Welcome, the designer
  with a control selected (anchor pins, live data on `dataframe.rr`) and after a
  drag, the editor's completion / hover / squiggle with Problems, the debugger paused
  on `shop.rr` (Variables, Call Stack, Watch with a changed value, Breakpoints, an
  expanded data tip, a run-time error), the AI panel and Settings > AI (mock
  provider, made-up key), Markdown, the rich-text example, a RapidQ import, the save
  prompt, Project Options, Find in Files, the command palette — in RapidR light, dark
  and high contrast (four also at 2×) and compares them with the approved images to
  the pixel. Any visual change to Studio now fails until a person has looked at it
  and approved it (`studio approve <scene>`); a failure names a side-by-side, an
  enlarged crop of the change and a diff image. `tools/regress.sh studio-goldens`
  (also part of the `studio` stage); tests/visual/README.md explains the scenes, what
  is held still (fonts, size, paths, settings, the mock provider's port) and how to add
  one. The check gives the same pictures from another folder, another locale and time
  zone, and twelve runs at once.
- **A contrast check on what is drawn** (`rapidr_ui_kernel::contrast`): each text
  run of a window's display list against the fill under it; the test hooks write
  the runs below AA (`RAPIDR_TEST_CONTRAST`, the web page's test results).
  `tests/studio_shell.mjs` and `tests/studio_flows.mjs` fail on any in RapidR's
  three looks — every Studio scene, the debugger paused (Variables, Watch,
  Breakpoints, Call Stack, a data tip, an error) on the desktop and in the browser.
- **`RPlot` bound to data**: `DataSource` (a data set or an RDataSource), `Kind`
  (bar, barh, line, scatter, area, step, pie, hist), `X`, `Y` — drawn again as the
  data change, with the real rows in Studio's designer.
- **`RCSV.FileName` can name a `$RESOURCE`**: the file built into the program, so
  a linked program runs from anywhere; the designer reads the file it names.
- `examples/data/dataframe.rr` is linked: RCSV → two RDataFrames → an RDBGrid and
  an RPlot; its buttons Filter, Sort and GroupBy the frames and both follow.

## [2.119.0] — 2026-10-15 (Preview 2)

The second Preview. Where the first one gave you the IDE, this one gives it an
AI assistant, a data engine, a deeper debugger and a lot more of RapidQ's
own programs running as they do under RapidQ.

### Highlights
- **An AI assistant in RapidR Studio, and `RAI` in your own programs.** Choose
  Anthropic (Claude), OpenAI, DeepSeek or any OpenAI-compatible server — a model on
  your own computer too — and ask about your project, have the assistant propose
  changes as diffs you accept or reject, or say "Build me a tip calculator" and see
  the program before anything is written. It is **off until you turn it on**, keeps
  your key in the system's keychain (never in a project or a program), and shows
  exactly what it sent. [The manual's chapter](docs/manual/ai-assistant.md).
- **Data science, fast and easy.** `RDataFrame` runs on RapidR's own typed, multicore
  engine: a million rows load, filter, sort and group in **23 ms** on an Apple M4 Max
  (pandas 3.0.1: 372 ms on the same table; ten million rows take 0.27 s). New linked
  components — `RCSV`, `RDataSource`, `RDBGrid`, and `DataSource` / `DataField` on
  edits and labels — put a CSV or JSON file in a grid with no code, and **RapidR
  Studio's designer shows the data live** while you design.
  [Data science with RapidR](docs/manual/data-science-with-rapidr.md).
- **A debugger that opens your data.** Rest the mouse on any value, or look in
  Variables or Watch, and open it like a tree: arrays, TYPEs, forms and their
  components, data frames as tables, as deep as you like, with changes in blue.
  **Set Next Statement** (retry or skip a failed line), **Step Into Specific**,
  and a leaner VM: a 10-million-iteration loop in about 0.6 s (it was 1.09 s), Studio
  waiting for you at 0.25 % of a core (it was 10 %).
  [Debugging](docs/manual/debugging.md).
- **Rich text.** `RRichEdit` formats each character and paragraph (`SelAttributes`,
  `Paragraph`), keeps the formats while the user types, undoes and pastes them, reads
  and writes RTF files, and tells screen readers what it looks like — settled
  against RapidQ's own compiler.
  [Rich text](docs/manual/rich-text.md).
- **Studio in the browser is the same Studio.** A program window bigger than the page
  is never cut off (the page scrolls), the desktop's and the browser's windows are
  pixel for pixel the same, screen readers read the web inspector, and a debugger step
  takes 15 to 30 ms on both (the budget is 50).
- **More of RapidQ runs as RapidQ runs it.** `$INCLUDE` finds files where RapidQ finds
  them, on every system, and the importer copies every include; Windows file names
  (`data\notes.txt`, `OPEN.BMP`) work on macOS and Linux; and RapidQ's 428 examples were
  run one by one beside RapidQ's own compiler in Windows, then on macOS, Linux and the
  web — a long list of small differences fixed, each one checked against RC.EXE.
  [Windows](docs/rapidq-corpus-status.md), [macOS, Linux and the web](docs/rapidq-corpus-status-unix.md).
- **New examples**: `data/linked_data`, `data/million_rows`,
  `data/json_source`, `debugging/shop`, `gui/richtext`, `gui/sales_dashboard`,
  `ai/rai_models`, `ai/ask`, `rapidq/shapes`.
- **A new manual page, [Why RapidR](docs/manual/why-rapidr.md)** (what RapidR does best,
  with measured numbers and their conditions), chapters for the AI assistant, data
  science and rich text, a clearer Getting started (which download to take; `run` against
  `build-bc` against `build`), and a README and examples list in reading order.

### The manual, the README and the examples (DOCS-P2)
- docs/manual/README.md lists every chapter in reading order (start, RapidR Studio, the
  language and its components, data science and rich text, building and importing, when
  something differs or goes wrong); Getting started opens with three lines — the SDK or
  the Runtime ("most people want the SDK"), `rapidr run` against `rapidr build-bc` (a
  `.rrbc` runs with `rapidr file.rrbc` or a double click) against `rapidr build`
  (`--interp` needs no Rust; native needs it once, `rapidr setup`) — a Preview user had
  found that confusing.
- New: docs/manual/why-rapidr.md; the README's "Why RapidR" block with pictures, and
  "Coming next". Stale statements fixed: the CLI chapter has `rapidr examples`, `serve`,
  `ai`, `lsp`, `dap`, `lang export` and `--csp`; the web IDE builds web apps and debugs;
  RapidR's own look is the default (the classic one with `$THEME Classic`); the
  differences page counts the corpus as it stands.
- examples/README.md in learning order, one line an example; the headers of `themes.rr`,
  `form_designer.rr` and `ide.rr` say what they show and how to run them.
- `tools/check_doc_links.py` checks every link and heading anchor in the README, the
  examples' README and docs/ (`tools/regress.sh unit` runs it).
- The Assistant pane opens 380 pixels wide (the dock's default side is 240): the group
  it shares with Properties widens the first time the assistant shows, unless the user
  has dragged it. `tools/lang_dispatch.py` knows that `RRichEdit.Paragraph`'s members
  (`LeftIndent`, `FirstIndent`, `RightIndent`, `Numbering`) are the `ParaAttributes` item's
  and that `SelAttributes`' are an `RFont`'s: its `--check` is clean again.

### Details, by area
The sections below say what each piece of work changed.

### The AI assistant in RapidR Studio, and RAI in your programs (AI-PROVIDERS, I8 phases 1–2)
- **An AI assistant, off until you turn it on** (Tools > AI Settings). Choose Anthropic
  (Claude), OpenAI, DeepSeek or any OpenAI-compatible server (Ollama, LM Studio, xAI,
  OpenRouter); paste your key — kept in the macOS Keychain, the Windows Credential Manager or the
  Secret Service, on the web only for the tab unless you ask to remember it, never in a project;
  press Fetch Models and pick one from the provider's own list. No model is chosen for you: the
  ones whose names say they are small are marked "fast · low cost" and listed first, the costlier
  ones "costs more", and "See prices" opens the provider's price page. The pane and its toolbar
  button appear once a provider, a key and a model are set. The Assistant pane (View > AI
  Assistant, Ctrl+I) answers about the open project as it streams; it reads the project with
  tools when it needs to (its files, its forms' components and properties through the
  introspection interface, RapidR's registry, the compiler's check) and changes nothing itself:
  its changes are reviewed as diffs (each accepted or rejected) and applied as one undo step per
  file; "Build me …" makes a new program, previewed (its code and its form) before it is created.
  What Send shares is said under the box; "What was sent" shows the last request (never the key);
  every request, tool use and decision goes to a local audit log. A token meter shows each
  session's tokens — and their cost, with the prices you give. Read-only or propose, a reply's
  length cap, 25 tool uses a question at most; what the project says is data for the model, never
  instructions. Desktop and web alike.
- **Fewer tokens.** RapidR's instructions are the same for every project and come first, so the
  providers' caches serve them again; the project is read through tools on demand, not sent;
  earlier tool results are shortened at the next question, a long chat is summarized. "Build me a
  hello-form program" now takes about 7,600–22,000 tokens sent (a third to two thirds from the
  cache) and 400–2,000 received with each provider's fast model — half of what it took before
  with OpenAI and DeepSeek, a sixth less with Anthropic.
- **RAI**: the same AI in your own programs — Studio's assistant is an RAI. Providers and their
  live model lists (`RefreshModels`, `OnModelsLoaded`, `Models`, `ModelName`), questions with the
  answer streamed (`Ask`, `OnToken`, `OnMessage`, `OnDone`, `Cancel`, `NewChat`), errors in plain
  words (`OnError`), `Usage` and `Cost`, an optional audit log. **Live in the designer**: pick the
  Provider, type the key in APIKey — kept in your keychain, while the code gets only where it is
  (`APIKeySource = "keychain:RapidR/<project>/AI|ask"`) — and the Model row lists the provider's
  models. Keys never in a program: `APIKeySource` takes `keychain:`, `env:` and `ask`
  (`OnKeyNeeded`); `APIKey` can be set, never read back; builds warn about a key written in a
  program, and a web build explains that its page asks its user for theirs. In a console program,
  `DOEVENTS` hears the answer. Two examples: `examples/ai/rai_models.rr` (a window: provider, key,
  models, a question, the answer streamed) and `examples/ai/ask.rr` (the console).
- **`rapidr-ai`**, RapidR's provider layer: Anthropic's Messages API and OpenAI-compatible Chat
  Completions over plain HTTPS with our own client (ureq and the system's TLS on the desktop,
  `fetch` in a browser), streaming and tool calls normalized, rate limits and overloads retried,
  errors explained in plain words (a refused key, no credit, a missing model, Anthropic's
  workspace). `rapidr ai providers | models | ask` uses it from the terminal.
- **RMarkdownView.FollowEnd** (a chat or a log stays at its end as it grows) and **OpenLink**; the
  inspector's **SetChoices** / **SetNote** (a program's own list and note on a property), and
  secret properties shown masked and never written into code.
- Manual: [The AI assistant and RAI](docs/manual/ai-assistant.md).

### The AI, used as you use it (AI-REAL, before the release)
Robert's run of the Preview 2 build found what the tests hadn't: they checked that things
compiled and returned, not what a person sees. Each of these was walked on the real screen —
with the three providers' real model lists and answers, and a local stand-in for the errors.
- **Fetch Models fills the Model list.** It said "Anthropic lists 14 models" over an empty list
  (the row refresh after it emptied the list again); now the list has them, the fast, low-cost
  ones first, and the model you pick is the one Settings shows when you open it again.
- **A refused key no longer locks Fetch Models.** After a 401 the button stayed greyed until
  Studio was restarted. Now every way a request can end gives it back at once: the list, a
  refusal, a server that answers with a web page, a provider that doesn't answer (it is given 30
  seconds; 10 to connect), another provider picked meanwhile, the dialog closed. What happened is
  said in plain words under the list, all of it — however many lines it takes.
- **Cancel leaves the assistant as it was.** Settings has its own RAI now: trying other providers,
  fetching their lists, trying a key you typed change nothing of the assistant until OK (they used
  to switch it, losing its key and its models). A key typed is only tried until OK or Save Key
  keeps it; Save Key and Remove act on the keychain at once. After OK the Send button is enabled
  (it waited for something else to happen first), and the Assistant pane's model list is the one
  Settings fetched.
- **Anthropic's workspaces.** When Anthropic wants to know the workspace of an organisation's
  key, Settings says so and puts the cursor in Workspace ID. The workspace is only sent to
  Anthropic, and the address typed for an OpenAI-compatible server only to that server (a
  cloud provider used it too after you'd once set one).
- **examples/ai/rai_models.rr ("Ask an AI")**: a Workspace ID row for Anthropic, with Studio's
  wording; its layout fits its window at the default size in every look and on the web (no
  scroll bar, nothing cut), follows the window when it's made bigger, and says what went wrong in
  up to four lines; link colours from the theme; List Models comes back after an error;
  switching provider forgets the model of the other one.
- **Tidier rows**: Settings > AI's rows close up for each provider (no gap where the Workspace ID
  or the address is hidden); the Assistant's empty text and the review bar's sentence wrap in a
  narrow pane; the inspector's lists (an RAI's Model row) are as wide as their longest name and
  stay inside the window; "Nothing was changed" when a review applies no change.
- **RAI**: a models list on its way is dropped when Provider, BaseURL, WorkspaceID or the key
  change, and a newer RefreshModels replaces it (OnDone("cancelled")); errors no longer name
  Studio's menus (your program's users see them too); a provider's own words in them are cut at
  160 characters. On the web a request that gets no answer is given up after the same time
  limits as on the desktop (a browser's `fetch` has none).
- **Timers keep ticking after a dialog closes.** The end of any ShowModal stopped every timer of
  the program — Studio's own too, after Settings or any other dialog. Now only the program's last
  window does, as under RapidQ (RC.EXE: a timer ticks on after a dialog it showed is closed).
- **`RComboBox.Clear` empties the text too**, chosen or typed, every Style — as RC.EXE does (the
  "Ask an AI" example's model list kept showing the other provider's model).
- **Tests that use it as you do**: a local stand-in for Anthropic, OpenAI and DeepSeek
  (`RAPIDR_AI_TEST_SERVER`, accepted for 127.0.0.1, [::1] or localhost only) with each one's real
  error shapes, chosen by the made-up key typed: refused, failing, silent, a web page, empty, an
  organisation's key. Studio flows click and type through the settings dialog on the desktop
  and the web — the list's items, the button back after a 401 and a retry that works, Cancel, a
  silent provider, the 30-second limit, the workspace header sent — in light, dark and high
  contrast with the contrast gate; the example's tests cover the workspace and a refused key.

### Running and debugging, used as you use it (DEBUG-RUN, before the release)
Robert's run of the Preview 2 build, paused at a breakpoint in examples/gui/themes.rr after a
click on its Dark button: the ▶ button was greyed, and the program's window vanished once he had
switched to another app. Both fixed and walked on the real screen, in light, dark and high contrast.
- **▶ continues a paused program.** Run (F5, the ▶ button, Run ▸ Run) is *Continue* while the
  program is paused — the menu item says so and the button's tip is "Continue (F5)" — and F5
  belongs to it alone (Debug ▸ Continue shows the key). Every Run and Debug command is now enabled
  exactly when it can act: Run, Stop and Restart right after a run that ended with an error (Run
  was greyed, Stop enabled with nothing to stop); the steps and Run to Cursor off while the program
  runs. A new state table — not running, running, stepping, paused at a breakpoint or a run-time
  error, ended, closed by its own close box — is checked on the menus, the tool bar and the command
  palette, driven by real keys and clicks on the desktop and in the browser
  (`tests/studio_flows.mjs` debug-commands-*).
- **A paused program's windows stay on the screen and reachable.** Stopped for the debugger, the
  program kept its main thread waiting for Studio: no window-system events, so its window couldn't
  be raised, repainted or brought back (and looked hung). Now it keeps serving the window system
  while it's paused — painted from what it shows, activated with a click or Cmd-Tab / Alt-Tab,
  moved, resized, minimized — without running any of its code: no handler, no timer. Clicks and
  keys to its windows are dropped while it's paused (not queued to surprise it later), on the
  desktop and in the browser alike; closing its last window ends it, as Stop does (Studio back to
  Ready, Run enabled); a value set from Variables or the Immediate pane shows in its windows at
  once.
- **The right window in front.** The program's first window comes up in front of Studio (it
  opened behind it); a stop while you were using the program brings Studio to the front;
  Continue, Run to Cursor and a step that takes longer than half a second bring the program back.
  **Debug ▸ Show Program Window** brings it forward any time it runs or is paused
  (`RProgramSession.ShowProgram`). macOS (cooperative activation), Windows and Linux on X11; on
  Wayland the system lets no program raise another's windows.
- **Event handlers, debugged as you debug them** (Robert's scenarios 1–6). A breakpoint in a
  handler stops on every event — each click, each letter typed (hello_form.rr's one-line OnChange
  stopped once, then never again), each OnTimer, OnResize, OnMouseMove, OnClose, OnKeyPress and
  focus event — after Continue and after a Step Over out of the handler; a handler that opens a
  dialog steps over it and stops on the next line; a handler that fires another stops in the inner
  one with both on the call stack; a run-time error in a handler stops at its line. In the
  browser: a timer whose OnTimer was stopped ticks again after Continue, and the paused program's
  window no longer takes the keyboard back from Studio (F5 and F10 reached the stopped program and
  were dropped — a nested OnResize looked as if it stopped forever). Each case is a flow on the
  desktop and in the browser (`debug-events-*`), three stops each.
- **Globals shows your variables first.** The constants an `$INCLUDE` brings (RAPIDQ.INC's 484
  colours, keys and button codes buried a program's own globals) are now under one closed item per
  file after them: `$INCLUDE RAPIDQ.INC (484 names)`.
- Manual: [Debugging](docs/manual/debugging.md) — "Which commands you can use when", "Your
  program's window and Studio's", "Breakpoints in event handlers" and Variables' Globals.

### Themes that follow the system, used as you use it (THEME-LIVE, before the release)
Robert's run of the Preview 2 build on a Mac in Dark: "Welcome screen is not responding to theme
changes, it's always light." Studio set its look before its window existed, when RapidR still
answered "light" for the system; half a second later it turned dark, but every colour Studio had
taken from the theme — the Welcome page, its cards, headings and links — stayed light. Fixed, and
walked on the real screen.
- **Programs open in the system's look.** A program in RapidR's look (the default, `$THEME
  rapidr`) reads the system's light, dark or high contrast from its first line — before its first
  window — on macOS, Windows and Linux; on the web, the page's colour scheme.
- **`Application.OnThemeChange`** (RapidR's, additive): fires once the look has changed — the
  program set `Application.Theme` to another look, or the system switched (macOS' Appearance or
  Increase Contrast, Windows' app mode or High Contrast, the browser's colour scheme or forced
  colours) while the program follows it. Once per change, after it, from the moment it's bound.
  Colours read with `Application.ThemeColor` are numbers: take them again there. Desktop, native,
  interpreter and web; its language-registry entry with its AI facets.
- **RapidR Studio follows the system, and every View ▸ Theme switch,** everywhere at once: the
  Welcome page and its cards, the panes, the status bar, the designer, the debugger's panes and
  changed values, and the dialogs — Settings, New Project, Project Options and the save question
  — even while they are open. A guard (`tests/studio_theme_follow.mjs`) switches the look while
  Studio runs and compares it, pixel for pixel, with Studio started in that look: the Welcome page
  (and scrolled), the designer, those dialogs open, the debugger paused; desktop and web; and the
  browser's own colour-scheme switch on a live page.
- **The Welcome page scrolls, and its cards are Tab stops.** At Studio's default size six of its
  sixteen example cards were cut off or hidden, with no way to reach them. The page now scrolls
  (the wheel, the bar), Tab and Shift+Tab go from card to card — the page scrolls to the one
  reached — and Enter or Space opens it; screen readers hear each as a button. Studio still starts
  with the keyboard in the Project pane.
- **`TabStop` on an `RPanel`** (RapidR's): the panel takes the focus — Tab, the theme's focus ring,
  Enter or Space for `OnClick`, a button to screen readers. RapidQ's panels never take the focus,
  and one without `TabStop` still doesn't.
- **A scroll box scrolls to the component Tab reaches** (Delphi's AutoScrollInView, so RapidQ's
  QSCROLLBOX does): `RScrollBox`, and a form's own scroll bars.
- New Project's and Project Options' fields keep the 28-pixel height their layout gives them
  (RapidQ's QEDIT AutoSize made them the font's height again when the dialog first showed, so a
  look's change while one was open made them jump).
- `examples/gui/themes.rr` follows the system: its card takes the theme's colours again and its
  last line names the look each time it changes; System, Light, Dark, High contrast and Classic
  pick one while it runs.
- Tests: `RAPIDR_TEST_SYSTEM_LOOK` (light, dark, contrast; `system`: the real system's or page's
  look under a test) and the test step `__look_dark` / `__look_light` / `__look_contrast`.

### AI-ready by design (AI-ARCH, phase 0)
- **The design**: [docs/ai-architecture.md](docs/ai-architecture.md) — what every language-registry entry
  carries for AI (meaning, types, ranges and units, side effects and the risk level they give,
  examples, where it runs, secrets), one introspection / command interface answered alike by the
  project's source in RapidR Studio and by the running program (describe, list, get, set, call,
  create, connect, destroy, subscribe, transactions with previews, undo), `rapidr mcp` and RAI on
  top of it, permission tiers on the risk levels, prompt-injection defences, a threat model, phases
  and acceptance tests. docs/ide-ai.md points to it where they meet.
- **The registry's AI facets**: `effects` (none, ui, wait, state, clipboard, file-read, network,
  device, registry, db-read, file-write, db-write, printer, exit, process, native), `risk`, `range`,
  `unit`, `example` and `secret`, checked by the registry's build like every other key; ranges and
  units for the commonest properties by name; RButton, `MID$`, `KILL` and `SHELL` complete as the
  reference entries; RMySQL's `Password` never shown to an AI. `rapidr lang export --json` carries
  them, and the AI prompt marks what reaches outside the program (`[file-write]`).
- **`rapidr lang ai-check`** lists every entry lacking a facet (`--summary`, `--json`, `--strict`
  exits 1 when anything is missing; without it, a report): 2,437 of 4,360 entries complete today.
- **`rapidr-introspect`** (new crate, wasm-safe): the interface's protocol and its read side at
  design time — `describe` (every facet, a JSON Schema per value), `list` (ids are the components'
  own names, qualified only when two share one; paged; a revision that follows the source) and
  `get` (typed values with where they come from and their line; secrets redacted) — behind the
  cargo feature `design`; `rapidr introspect FILE…` only in a CLI built with
  `--features ai-introspect`. Nothing in Studio or the default CLI uses it yet.

### Linked data, live in the designer (DATA-ARCH, I7 phase 1)
- **The plan**: [docs/data-science-plan.md](docs/data-science-plan.md) — the columnar engine, sources as
  linked components (CSV, JSON, Excel, Parquet, Arrow, Avro, Delta Lake, SQLite, MySQL, PostgreSQL /
  Redshift, SQL Server, S3 / Azure), data-aware components, the BASIC API, the web's limits, the licence
  of every crate, performance budgets and a benchmark suite against pandas and Polars, the registry for
  code completion / MCP, phases with sizes.
- **RCSV** (new): a CSV file as a data source — `FileName`, `Delimiter` (`,` `;` tab `|` …),
  `HasHeader`, `Text`; `RowCount`, `ColCount`, `Columns`, `Error`; `Load`, `Save [file]`, `Clear`;
  `OnLoad`, `OnError(Message)`. Read once, when its table is first needed; a relative file is found in
  the current folder or beside the program.
- **RDataFrame.Source** (an RCSV, an RJson's records, another RDataFrame): the frame takes the rows at
  once and again whenever the source changes; `RDataFrame.Refresh`.
- **RJson's records are a data source** (`[{…}]`, JSON Lines, columns) — taken again when the
  document changes.
- **RDataSource** (new): a current row shared by the controls bound to it — `DataSet`, `Row`,
  `RecordCount`, `EOF` / `BOF` (Delphi's: `WHILE NOT DS.EOF … DS.Next WEND`), `Enabled`, `State`;
  `First`, `Prior`, `Next`, `Last`, `MoveBy`, `Field(name)`, `SetField`, `Refresh`; `OnDataChange`.
- **RDBGrid** (new): a grid drawn by the UI kernel straight from a data set's frame, only the rows on
  screen (a million rows scroll smoothly): headings, row numbers, numbers right-aligned, a missing
  value dimmed as `null`, the current row (click, arrows, Page Up / Down, Home / End), a heading's click
  sorts the grid's view (the data unchanged), `OnCellClick(Row, Col)`, `OnTitleClick(Col)`; a grid for
  screen readers. Pixel-identical on the desktop and the web.
- **`DataSource` + `DataField` on REdit and RLabel** (RapidR's own, additive): the current row's
  field; what the user types in a bound edit goes back into the table.
- **Live in RapidR Studio's designer**: the designed form's RCSV / RJson / RDataFrame / RDataSource are
  read by the designer itself (files inside the project only, read-only, cached), so grids, edits and
  labels show the real data while you design; RCSV is in the toolbox's Data group, with the data
  file's icon.
- One model for every runtime (`rapidr_value::datascience::link`): native builds, the interpreter, the
  web and the designer. Registry entries for every member (code completion, the manual, MCP).
- Examples: `data/linked_data.rr`, `data/million_rows.rr`, `data/json_source.rr` (on Studio's
  Welcome page); the manual's new chapter **Data science with RapidR**, with screenshots.
- The inspector's **DataField** drops the bound data set's columns.
- Fixed: a non-visual component inside a form (an RDataFrame, whose Width / Height are its columns
  and rows) no longer counts toward the form's scroll range on the desktop (the web already
  ignored it).

### Data science: a typed, multithreaded RDataFrame engine (DATA-ENGINE)
- RDataFrame's engine stores columns typed (whole numbers, fractions, true / false as numbers; a
  validity bitmap where cells are null; buffers shared until changed) and shares big work out to
  every core: on a million rows (M4 Max) load 100 → 12 ms, filter 39 → 3 ms, sort 203 → 10 ms,
  group-by 84 → 5 ms, join 63 → 3 ms, describe 86 → 10 ms, save 82 → 7 ms; load + filter + sort +
  group-by 267 → 23 ms (ten million rows: 4.4 s → 0.27 s; on one thread, as in a browser:
  0.12 s for a million). Nothing a program sees changes: a column is typed only when every cell's
  text is its value's own (`"007"`, `"1.50"`, `"True"` keep theirs), and frames printed, cells,
  types, CSV and JSON written, warnings are byte-identical — checked by a differential test against
  the previous engine over tens of thousands of random programs (`tests/frame_oracle.rs`).
  `Frame::sorted_order` gives a grid the sorted order without moving rows. The benchmark suite is
  `tools/bench/` (frame_bench with the plan's budgets — also in `tools/regress.sh perf` — the same
  work as a RapidR program, pandas and Polars scripts, `compare.sh`).

### The debugger: values as trees, Set Next Statement, a leaner VM (DEBUG-VM)
- **Data tips open like a tree** (RapidR Studio, desktop and web): rest the mouse on a value while
  paused and open it to any depth — an array's elements, a TYPE's fields (nested TYPEs too), a
  component's properties and the components it holds, an `RDataFrame` as a **table** (its columns
  over its rows, a row opening to its cells), an `RNum`'s numbers. A name through an element
  (`orders(i).Qty`, `Buyer.Name`) shows that part; a data tip never runs a SUB, FUNCTION or method.
  Each row's buttons copy the value (whole), add it to the watches, or show a string's whole text
  in a text viewer. Values that **changed since the last stop are in blue** (bold in a tip), in the tip and in
  Variables and Watch. Big arrays, frames and RNums show 100 at a time, then "… more".
- **Variables** has the program's **Components** (its forms, frames, timers …, each opening to its
  parts and properties); right-click a value for Copy Value (Ctrl+C), Copy Expression, Add Watch,
  View Text; F2 sets nested fields (`orders(1).Buyer.Name`). Component types show RapidR's names
  (`"Greet" (RButton)`).
- **Set Next Statement**: drag the yellow arrow in the margin, or Ctrl+Shift+F10 on a line — the
  program goes on from there (within its SUB or FUNCTION). At a **run-time error**, fix the value
  and put the arrow back: the line runs again and the program goes on instead of ending.
- **Step Into Specific** (Ctrl+Alt+F11): a menu of the calls the paused line still makes; into the
  one picked. **Step Over** a line that calls a SUB now stops at the next line (it stopped again in
  the middle of the same line, where the call returned).
- The session protocol (`rapidr-session`): `stepInTargets`, `stepIn { target }`,
  `setNextStatement` (then `stopped` with reason `goto`), `evaluate` contexts `clipboard` and
  `hover`, a `Components` scope; variables carry `evaluateName`, a table's `columns` and a row's
  `cells`; `evaluate` replies carry `count` and `columns`. `rapidr dap` answers `stepInTargets`,
  `gotoTargets` and `goto` (VS Code's Step Into Target and Jump to Cursor).
- The VM (`rapidr-vm`): `Vm::set_next_statement`, `step_in_targets`, `step_into_target`; a
  program under the debugger with nothing to watch for checks only Studio's interrupt (a line's
  code range remembered instead of a search per instruction).
- **Nothing polls any more while a program waits**: a program under the debugger waiting for its
  events used to wake every 50 ms, RapidR Studio every 20 ms while a program ran (10 % of a core,
  measured); now the program's session wakes it when Studio asks something, and Studio — woken
  by what its program says (WEB-FRAME's waker, and once more just after the program ends) — looks
  on its own only once a second while a program runs.
- The **Immediate window** answers again: its box (an RRichEdit, now with RapidQ's CR LF line
  breaks) has taken the Enter when Studio hears it, and the line read was the empty one below.
- **Faster VM** (tools/bench/vm_bench): errors built only when they happen, opcodes decoded from a
  table, arithmetic and comparisons in place, builtins' names resolved once and their arguments
  read where they are on the stack; `RNum.Get` / `Set` / `Count` no longer copy the whole array.
- **tools/bench/vm_bench** measures start-up, memory, a 10 M-iteration loop (with and without the
  debugger), strings, calls, arrays, and idle CPU (a waiting form, under the debugger, RapidR
  Studio running it) against `tools/bench/vm_budgets.json`; `tools/regress.sh perf` runs it.
- New example **examples/debugging/shop.rr** (nested TYPEs, an RDataFrame, an RNum, a form, a loop
  to step through), on Studio's Welcome page; manual: docs/manual/debugging.md.

### Rich text in RRichEdit (RICHEDIT)
- **RRichEdit (RapidQ's QRICHEDIT) is a rich text editor now, on every runtime.** Its text keeps
  a format per character — font, size, colour, bold, italic, underline, strike-out — and per
  paragraph (alignment, indents, a bullet), in one shared model (`rapidr_value::objects::richtext`)
  that native builds, the RapidR Runtime and the browser use alike; the UI kernel draws it as
  parley's styled runs. RapidQ's rules, settled against RC.EXE in the Windows VM (window probes
  that format a rich edit and save it as RTF, every step compared byte for byte):
  - `SelAttributes` sets the selection's format — a whole QFONT, or a member: `Color`, `Name`,
    `Size`, `Bold`, `Italic`, `Underline`, `StrikeOut`, `Charset`, `AddStyles` / `DelStyles`; at
    a caret it is the format of what is typed or added there next, kept until the caret moves (so
    `SelAttributes.Color = …: AddStrings …` colours a log's line, as RapidQ's chat examples do).
    Reading a member gives the selection's last character's format (the caret's at a caret; an
    automatic colour reads &H80000008). `SelFont = Font` is its other name.
  - Typed or inserted text takes the format of the character before it (at a paragraph's start,
    the one after it); text typed over a selection, the selection's first character's.
    `Text =` gives the new text the old first character's format; `Clear` leaves the last one's
    for what is typed next. `Font` reformats text never formatted (all of it while nothing is);
    `Alignment` aligns the paragraphs without their own and moves the caret to the start, as
    `WordWrap` and the form's `Center` do (RapidQ makes the control again).
  - `AddStrings` and `Line(i) =` change the text as RapidQ's Lines do (fixed): a line is added
    after the last break, or with a break before it when the text doesn't end with one
    (`"Hello world"` + `AddStrings "added"` is `"Hello world" CR LF "added"`, not one more CR LF),
    and the caret ends after what was added — for QMEMO too.
  - `SaveToFile` / `SaveToStream` write RTF while `PlainText` is False — byte for byte what
    RapidQ's rich edit writes, its closing NUL included — and `LoadFromFile` / `LoadFromStream`
    read any RTF (font and colour tables, `\'hh`, `\uN`, paragraph formats, bullets; fields,
    pictures and other destinations skipped); a file that isn't RTF loads as text (up to a NUL).
    RapidR's own RTF reader and writer (`rtf.rs`): no new dependency.
- **Editing keeps the formats**: what the user types takes RapidQ's format for it, Ctrl+Z
  gives back the text and its formats, and text copied from a rich edit pastes into a rich edit
  with its formats (keys, the context menu, CopyToClipboard / PasteFromClipboard; the clipboard
  holds the plain text, its RTF kept beside it).
- **RapidR's additions** (registry `origin = "rapidr"`): `Paragraph.Alignment`, `LeftIndent`,
  `FirstIndent`, `RightIndent` (pixels) and `Numbering` (bullets) for the paragraphs the
  selection is in; `TextRTF`, the RTF as a string (read and set).
- **Screen readers** get the text as runs carrying their formats (AccessKit's font family, size,
  weight, italic, underline, strike-through and colour: VoiceOver's and NVDA's "report
  formatting"), and the field's description names the format at the caret ("At the caret:
  Courier New 12 point, bold, red"; the browser's `aria-description`). In a high-contrast theme
  the text's colours are the theme's.
- RapidQ's `richedit/syntax` example (William Yu's syntax highlighter) shows its highlighting as
  under RapidQ: its saved RTF is byte-identical to RC.EXE's, and its window matches RC.EXE's
  capture. GUI case `rich_text` (typing in a formatted word, Ctrl+Z, SelAttributes read back,
  TextRTF) passes natively, interpreted and in the browser with byte-identical windows and equal
  accessibility trees; conformance case `richedit_grid_streams` now holds RC.EXE's stream bytes;
  visual case `richtext` beside RapidQ's own window.
- New example [examples/gui/richtext.rr](examples/gui/richtext.rr) — a small rich-text editor (B,
  I, U, colour, font, alignment, bullets, open / save `.rtf`) — and a manual chapter,
  [docs/manual/rich-text.md](docs/manual/rich-text.md). The test hooks' `__key_N_16` is the
  shortcut key too (an edit's Ctrl+Z on every host).
- Differences left from RapidQ: its `LineCount` / `Line(i)` count wrapped screen lines with
  WordWrap on (RapidR's count paragraphs); after `SelectAll` its SelLength counts the text's end as
  two more; a loaded document is saved with RapidR's RTF header; `\fcharset0` details of a few
  insertion formats.

### Studio on the web: big windows, the same pixels, screen readers, fast steps (WEB-FRAME)
- **A program window is never cut off in RapidR Studio on the web** (the Preview's known issue):
  a window bigger than the page — `examples/gui/sales_dashboard.rr`, a new 1200 × 800 sales dashboard
  example — opens with its title bar at the top of the page, and the program's desktop scrolls to
  the rest: a thin scroll bar along the page's right or bottom edge while a window reaches past it
  (drag its thumb, or turn the wheel over it or over a window's title bar). Windows can't be dragged
  above or left of the page's corner any more (a title bar dragged off the top was lost for good);
  maximized, a window fits the part of the page you see. The same rules in every web build
  (`rapidr bundle-bc`, Build Web App). A double click on a window's title bar now maximizes it on
  the web (it never did). Tests: `studio_flows` `program-window-big` (a 1280 × 720 page, 1× and
  2×), `program-window-big-fits`.
- **Studio's desktop and web captures are byte-identical**: the desktop's CPU renderer now runs
  vello_cpu's portable SIMD level, which rounds as wasm SIMD does (Neon's and AVX2's fused
  multiply-add rounded glyph edges one level apart). The command palette's items keep the same
  accessibility ids on both hosts. `studio_shell` no longer allows "SIMD rounding".
- **Screen readers and the web inspector**: the inspector is a tree grid whose rows read as
  "Caption, Button1"; a read-only property says "read-only" (its row's description, its value's
  cell read-only). Before, the web's rows had no value and no read-only state.
- **Debugger steps are fast on a loaded computer**: Studio on the desktop hears the program the
  moment it stops instead of at its next 20 ms poll — a step with its line and values shown took
  85 ms, now about 15 ms (budget 50 ms; the web's ~20 ms).

### RapidQ include files found everywhere, imported with the program (IMPORT-INC)
- **`$INCLUDE` finds files as RapidQ does, on every system.** Checked against RapidQ's own compiler:
  every `$INCLUDE` — a nested one too — is looked for in the program's folder (where RapidQ
  compiles), then in the include folders, in order. RapidR then also looks beside the including
  file and above an include folder. Names match in any case, with `\` or `/`: `"Lib\MyLib.INC"`
  finds `lib/mylib.inc` on macOS and Linux. A Windows path from another computer
  (`C:\rapidq\include\x.inc`, `\RapidQ\include\x.inc`) is found by its ending, and on Windows an
  absolute path is read as it is. The compiler, the language service (go to an include too),
  RapidR Studio's project and the importer use this one search. Files are named as on disk
  (`lib/mylib.inc`, not as the line wrote it). An include that isn't found says what to do.
- **Include folders**: RapidR Studio's **Tools > Settings…** (Include folders: Add Folder…,
  Remove), kept per user and used for Open, the problems, Run, Build and Import (the programs Studio
  runs and builds get them too). On the command line: `--include-dir <folder>` on any command
  (before a program's own arguments), and `RAPIDR_INCLUDE_PATH`. The palette's View commands are a
  button away in Settings ("Themes and Views…").
- **Import copies every include** (`rapidr import-rapidq`, File > Import RapidQ Project or File…):
  however deep, names converted. The program's own keep their folders; an include folder's go into
  the copy's folder as they were there (`GL\QGL.INC` → `GL/QGL.INC`). A line that would find the
  original instead of the copy (`..\shared\x.inc`, a `C:\` path) is changed to name the copy. The
  copy compiles on its own, and the proof now also checks that it reads its own files.
  RAPIDQ.INC and the other includes RapidR has built in (`QMidi.inc` … `QDockForm.inc`) are never
  copied, a user's own copy of one neither (the report says so, and when the program uses
  something only that file has, which folder to keep). The report's new **Include files** table
  lists each `$INCLUDE`, where it was found and its copy. **A missing include is never passed
  over**: listed under *Not found* with what to do, in Studio's Output (the status line red), and
  printed by `rapidr import-rapidq` (exit code 1).
- Opening a `.bas` in Studio lists its includes in the project tree as the compiler finds them (in
  any case, a nested one from the program's folder). On the web, File > Import RapidQ Project or
  File… takes several files at once — the program and its includes — and an imported folder brings
  its subfolders' includes.
- `rapidr examples copy` copies an example's include files with it.
- RapidQ's 428-program corpus imported whole: 341 programs compile to identical bytecode (0
  different, 0 copies failing; the rest don't compile in RapidR yet), 129 include files copied,
  27 not found (named in the report); the include search reads every corpus program as before.
- New example: `examples/rapidq/shapes` — a RapidQ program whose library and window are in `lib/`
  and `forms/` (RapidQ's compiler builds it too). Manual: importing-rapidq.md "Include files";
  cli-and-runtime.md (`--include-dir`).

### RapidQ's examples on macOS and Linux (CORPUS-PORT)
- **Windows file names work on macOS and Linux.** A RapidQ program names its files as Windows does:
  `data\notes.txt`, `Application.Path + "\save.dat"`, `OPEN.BMP` for an `open.bmp`. Off Windows a
  `\` was part of the name and Linux told `OPEN.BMP` from `open.bmp`, so such programs found none
  of their files (pictures missing, `Cannot open file …`). Now `\` separates folders and names are
  found in any case, as on Windows and as the web runtime already did — for every file and folder a
  program names (QFILESTREAM, LoadFromFile / SaveToFile, pictures, sounds, OPEN, FILEEXISTS,
  DIREXISTS, FILELEN, DIR$, KILL, MKDIR, RMDIR, RENAME, CHDIR, QFILELISTBOX, QDIRTREE), interpreted
  and native. A name that exists as written is taken as written. On Linux, RapidQ's examples
  `listbox/owndraw`, `games/puzzle` and the two vector-font demos show their pictures and fonts
  again, and six DirectX ones their models and textures (`rapidr_value::paths`, conformance case
  `rapidq_windows_paths`, checked interpreted, native and on the web).
- **Owner-drawn list boxes of variable height draw at once, interpreted.** A `lbOwnerDrawVariable`
  QLISTBOX asked each item's height (OnMeasureItem), then waited for the system's next event —
  the mouse moving over the window — before its OnDrawItem drew the items: RapidQ's
  `listbox/owndraw` example showed plain text where its pictures belong. The handlers the
  drawing makes due now run in the same turn (native builds and the web already did; GUI case
  `owner_list_variable` on all three).
- **A TYPE's component CREATEd inside a form's CREATE is made where it stands.** RapidR made such
  instances (`CREATE RichEdit AS QREdit` inside `CREATE Form`) after the whole form block — after a
  `ShowModal` at its end had returned, so RapidQ's `richedit/syntax` example showed an empty
  window. Now the instance is made in its place and the block goes on after it, as RC.EXE does
  (its form's OnShow sees the instance; GUI case `nested_typed_create`, all three runtimes).
- **A rich edit's SelStart and SelLength count a line break as two characters**, CR LF, as its
  Text has them — as RC.EXE's QRICHEDIT does (probe: in `"ab" CR LF "cd"` SelStart 4 is the `c`;
  a position inside the break reads as the break's start). A position INSTR finds in Text now
  selects that text; RapidQ's syntax-highlighting example looped forever where it counted one.
  QEDIT and the code editor are unchanged. (Rich text formatting itself: "Rich text in RRichEdit"
  above.)
- **RAPIDQ.INC's `QBColor` array.** `$INCLUDE "RAPIDQ.INC"` (RapidR's built-in one) now also gives
  `QBColor(0 TO 15)`, the INTEGER array of QBasic's 16 colours as &HBBGGRR numbers that RapidQ
  programs index (`VoorKleur = QBColor(12)`, bright red) — its bounds, type and values as RC.EXE
  reads them, on all three runtimes (conformance case `rapidq_qbcolor`, whose expected output is
  RC.EXE's). Included twice, RAPIDQ.INC is given once, as RapidQ's own file guards itself.
- **docs/rapidq-corpus-status-unix.md**: every portable RapidQ example on macOS, Linux (Ubuntu ARM)
  and the web, interpreted and native — 114 run on the desktop with no problem on any of the four
  (105–106 ok, the rest as RC.EXE's own builds), the web the same as the desktop; what was fixed,
  what's left. Made by the new `tools/corpus_status.py` from the sweeps (`tools/corpus_run.py
  --list`).
- `tools/rc_probe.sh`: `-w` gives a run its own work folder in the Windows VM (two lanes no longer
  copy RC.EXE over each other), and `w_*.bas` probes are GUI programs whose findings file is
  printed.

### RapidQ's examples on Windows (CORPUS-WIN)
- `CONST 1 = 5` compiles and changes nothing, as in RapidQ: what `$DEFINE TRUE 1` (RapidQ's
  `windows.inc`) makes of `rapidq2.inc`'s `CONST True = 1` — programs including both
  (`Mouse/Mouse_Wheel.bas`) stopped with "Syntax error in CONST statement". Checked against
  RC.EXE (`rapidq_const_defined_name`).
- More of RapidQ's examples compile (344 of 428, from 336), each form checked against RC.EXE
  (`rapidq_corpus_forms`, both backends):
  - `&` before a name is the name: `&x` is x, `.WriteNum(&ForwardTo, 4)` (RapidQ's
    `CallBack_*.inc`). `&O` / `&B` (RapidR's octal and binary) are numbers only with a digit after
    them, so `&Bar` is the variable Bar as in RapidQ.
  - `DEFINT R()` and `DIM A() AS …`: an array REDIM sizes later (R(0) until then, as RC.EXE).
  - A `$` inside a name: `Encrypted$YY` is a variable of its own (`Encryption/RC4.bas`).
  - A DECLARE inside a block (`IF … : DECLARE FUNCTION f LIB …`) declares for the whole program.
  - A DECLAREd SUB or FUNCTION takes its parameters' names from the DECLARE, as RC.EXE does:
    `DECLARE FUNCTION F (a AS LONG)` then `FUNCTION F (b AS LONG)` — inside F the argument is `a`
    (RapidQ's callback includes depend on it).
  - `SizeOf(Short)` (a type's name) under `$TYPECHECK ON`.
  - `$RESOURCE NAME AS <file>`, as `$INCLUDE <file>` (`AVI/aviplay.bas`).
  - A component named like a statement passed to a call without parentheses:
    `File.AddItems Open, Break1, ExitItem`.
- And more (351 of 428 compile), each against RC.EXE (`rapidq_corpus_forms2`, both backends):
  `AS STRING * MAX_PATH * 2` (a length worked out from CONSTs: 2 × MAX_PATH, where only a plain
  number counted before); `DefStr s =` with nothing after the `=`; a `-` with nothing after it
  (`IF Buttons > - THEN`: RC.EXE negates what came before — `5 + -` is -5); `#` before a name
  (`.Color = #H000000` is the undeclared variable H000000, 0); a SUB named like a directive
  (`SUB $XP_MANIFEST`, called as `$XP_MANIFEST`); END SUB closing a TYPE's PROPERTY SET.
- **A `$DEFINE` is now replaced on every line**: a line that also named a string function
  (`STR$(…)`, `CHR$(…)`, anything with a `$`) kept the defined name as it was — an undeclared
  variable, 0 — so `If gFile < Z_OK Then … str$(gFile)` compared with 0 by luck and other programs
  silently computed with 0. Only directive lines are left alone now (`rapidq_define_string_lines`,
  RC.EXE's output).
- Also as RC.EXE takes them (`rapidq_corpus_forms3`): a call in a WITH whose arguments start with
  the WITH's members after a space (`MemCpy .TB, .Button(i), n`, RapidQ's QToolBar.inc); a `_`
  alone inside a line (an operand of its own: `t _ (3 + 1)` is t, RapidQ's asteroids games); READ
  into a method (`Read L.AddItems N` adds the DATA items); `$INCLUDE "file` with no closing quote;
  `...` closing a DECLARE's parameters (zlib.inc's `gzprintf`; a call with more arguments is still
  an error, as in RC.EXE). 355 of the 428 examples compile.
- Running RapidQ's examples side by side with RC.EXE's builds in Windows found, and these now behave
  as RapidQ's (each checked against RC.EXE; desktop, native and web share the code):
  - A rich edit's SelStart / SelLength counting a line break as two (CORPUS-PORT's and RICHEDIT's,
    below) is what RapidQ's syntax-highlighting example needed: it looped forever
    (`rapidq_richedit_positions`).
  - **An array written past its end reads back what was written** (RapidQ doesn't check limits):
    RapidQ's GMDrum fills `DEFSTR D(16)` up to 28 from DATA; its last twelve instrument names were
    blank. The elements are kept beside the array, memory-safely (`rapidq_array_past_end`).
  - **A QTRACKBAR turned (`Orientation = 1`) swaps its Width and Height**, as RapidQ's: GMSynth's
    160 × 40 sliders become tall ones (`rapidq_trackbar_orientation`).
  - A TYPE's instance CREATEd in a component's block deeper down (a QFolderTree on a panel on a
    form) is made where it stands too (CORPUS-PORT's rule, below, one level further): it stopped
    with "this variable does not refer to an object" (`typed_create_in_panel`).
  - `RESOURCE(MY_JPG)` — a `$RESOURCE` name where an index goes — is the first resource, as
    RC.EXE reads the name there as 0 (RapidQ's IPicture demo found no resource).
  - A control's text ends at a NUL, as Windows' controls and message boxes end it: a message built
    from a path made for a DLL (`… + CHR$(0)`) no longer shows a box character.
  - A QOVALBTN's `Color = 0` is black (it was the face colour): Wav-Scratcher's turntable.
  - A tall QEDIT (an `alClient` one) keeps its text at the top, as Windows' edit does.
  - **A QEDIT's AutoSize** (True until set) fits its Height to its font, as RapidQ's: when its form
    first shows (`Height = 50` set before reads 21 after), when its font changes (14 points: 31;
    RC.EXE 32 — Windows' face is a pixel taller) and when AutoSize goes from 0 to 1; a Height set
    afterwards, or with AutoSize 0, stays (`rapidq_edit_autosize`).
  - A component passed to a BYREF parameter (`BYREF ptable AS QSTRINGGRID`, RapidQ's sqlite3.inc)
    builds natively (`rapidq_component_byref`).
  - `array_bounds_rapidq` now expects RC.EXE's output (a write past the array reads back).
  - `For N = To 29` (no start value) counts from 0, as RC.EXE builds it (`rapidq_for_no_start`).
  - A `CASE` left after its `END SELECT` in a TYPE's method compiles and does nothing, as RC.EXE
    builds it (`VideoCapture/WebCam_Class.bas`).
  - **A Close (or a ModalResult) in a form's OnShow under ShowModal is lost**, as VCL's ShowModal
    sets ModalResult to 0 once the form has shown: the form stays, with no OnClose until it's closed
    later. RapidQ's OpenGL demo closes its form in OnShow when a texture is missing, and RC.EXE's
    window stays; RapidR's closed at once (`rapidq_modal_close_onshow`, RC.EXE's output).
  - A second ELSE in one block IF compiles, and what follows it never runs, as RC.EXE builds it
    (`forms/Window Sizer.BAS`; `rapidq_two_else`).
  - QSOCKET's `GetHostIP` / `GetHostName` without parentheses call the method
    (`Showmessage Sock.GetHostIP` showed an empty box; `rapidq_socket_host`).
  - **`DIR$(path + "*.", faDirectory)` lists the folders**: a pattern ending in a dot names no
    extension, as Windows' FindFirstFile reads it (it was a literal dot: nothing matched); QFILELISTBOX's
    Mask reads it alike. On Windows, DIR$, QFILELISTBOX and QDIRTREE leave out hidden and system files
    and folders unless asked for (faHidden / faSysFile, ftHidden / ftSystem): `pagefile.sys` and
    `Documents and Settings` showed (RapidQ's QFolderTree).
  - A QDIRTREE's closed folder shows its `+` only when it has folders in it, as RC.EXE's.
- 358 of RapidQ's 428 examples compile now (from 336 at the start of this work).
- `tools/windows/corpus_win.py` runs RapidQ's examples on Windows three ways (RC.EXE, RapidR
  interpreted and native) and captures their windows; `tools/windows/corpus_win_report.py` writes
  the table of [docs/rapidq-corpus-status.md](docs/rapidq-corpus-status.md).

### Release tooling: the VMs stay clean, and no console window flashes
- `windows-vm.sh clean` and `linux-vm.sh clean` remove everything the release and smoke scripts leave
  in the VMs: the release work folder (the cargo target alone was 10 GB), the build cache, the smoke
  tests' temporary folders, their test apps and, in Windows, the file types, Uninstall entry and
  Start menu shortcut an interrupted smoke test left registered. `--dry-run` lists with sizes,
  `--keep-out` keeps the packages; tools, an installed RapidR and the user's files stay. The release
  README's last step runs both.
- The smoke tests remove what they made even when a check fails: `smoke.ps1` keeps its build cache
  inside its temporary folder (it had been filling `%LOCALAPPDATA%\RapidR\Cache`) and cleans up in a
  `finally`; `smoke.sh` also on ctrl-C or kill.
- `windows-vm.sh` opens no window on the Windows VM's desktop any more: every call runs as SYSTEM
  (`vmexec.sh -S`) and jobs start as a headless console, where the 20-second poll and the file fetch
  each opened a console window in the user's session. Fetch uses pieces of 6 MB instead of 400 KB.
  `windows-vm.sh flashtest control` checks it with a window watcher.
- `ubuntu-vm.sh root` works while the VM's screen is locked or nobody is logged in
  (`--current-user` answers "Unable to authenticate the user").

### The test harness: no leftovers, no flakes (HARNESS)
- Studio's web build (`tools/build_studio_web.sh`) ships the examples git lists — committed, or new
  and not ignored — not whatever a run left in `examples/` (a program's `*.tmp.*` files, which a
  capture test that ends an example early never gets to `KILL`): Studio's "run with data" flow
  counted a stray `staff.tmp.csv` as a third file of `dataframe.rr`'s project. The flows' copy of an
  example's folder leaves `*.tmp.*` out too, and the visual gallery removes the ones its runs made.
- `tests/gui_captures.mjs` hands the cases their `RAPIDR_TEST_DROP` files (the web parity case
  `drop_files` had empty labels on the desktop side) and runs the `inWork` cases in a folder of
  their own, removed after: `drop_a.txt` / `drop_b.csv` / `dirlv/` no longer land in the checkout.
- The QDIRTREE unit test (`dirtree::browses_directories`) is rooted in a folder of its own: the tree
  lists the folders above the selected one too, so other tests' folders coming and going in the
  shared temp folder shifted its rows between two calls.

## [2.118.0] — 2026-10-09 (Preview)

### Triage of the full regression run (TRIAGE)
- A grid cell's editor no longer covers its **list / ellipsis button**: in a gcsEllipsis or gcsList
  column the editor box stops where the button starts, so the "..." (or the arrow) shows beside
  the editor, as RapidQ's (it had been drawn under the editor since the button moved to the
  editor-only rule).
- `Project.RustReady` reads without brackets (`IF Proj.RustReady`), like the other Project
  methods that return a value.
- `examples/form_designer.bas` is now `examples/studio/form_designer.rr`, in the examples' index and
  test table. The stale test expectations of themes, RC.EXE's defaults and the grid's editor-only
  button were brought up to date.

### Build Web App (WEB-BUILD)
- **Run > Build Web App (.zip)** in RapidR Studio on the web: the program compiled to bytecode and
  zipped with the web runtime, its page and strict per-program Content-Security-Policy (the same
  template and policy `rapidr bundle-bc` uses: one `rapidr-webbundle`), the notices, the fallback
  fonts and the project's files (a CSV, pictures), downloaded as `<project>-web.zip`: unzip on any
  web host. The page zips it itself (a stored `.zip`, no library, nothing from a CDN). The Output
  panel says "✓ Built Notes-web.zip — unzip on any web host". The web page's Run menu no longer
  lists Build, Build Native App, Build Interpreted App and Reveal (those make apps for a computer).
- **Run > Build Web App** in RapidR Studio on the desktop: `rapidr build --web --interp` into the
  project's output folder as `<project>-web.zip` (the log's "Web:" line is a link, with Reveal in
  Finder / File Explorer / Files).
- The inspector shows a form's **Enabled** (declared an INTEGER by RapidQ, 1 or -1 and 0) and
  **Visible** / **ShowHint** as check boxes, True for any non-zero value (a running program's form
  showed "-1").
- Manual: building-apps.md "Build a web app".
### Installers: RapidQ file types, leaner vendored sources (FINAL-PKG)
- `.rqw`, `.rqb` and `.rq` open in RapidR Studio like `.bas` (macOS document type, Windows "Open with" and the
  `basdefault` task, Linux MIME type); the Windows entry is named "Open in RapidR Studio".
- The vendored crates the packages ship leave out their tests, benches, examples, test data and any prebuilt
  `.dll`/`.exe`/`.so`/`.dylib` (`tools/release/home.py`); what the build compiles is unchanged.

### RapidR Studio speaks RapidR's names, and imports RapidQ programs (R-NAMES, phase 2)
- **File ▸ Import RapidQ Project or File…** (and **Import RapidQ Folder…**): pick a RapidQ program
  (`.bas`, `.rqw`, `.rqb`, `.rq` or `.inc`, with the files it includes) or a folder; Studio writes
  a **copy** with RapidR's names beside it (`<name>-rapidr`, the next free name), proves each
  program compiles to the same bytecode as its original, and opens the copy as a project with its
  report beside the code. The original is only read. On the web it works on the files the user
  picked (the page's store) with the same engine — the same copy, byte for byte.
- `rapidr import-rapidq <src> [dest]`: the copy's folder may now follow the source (`-o` still
  works).
- **Only RapidR's names in Studio**: the toolbox lists `RButton`, `RLabel`, `RForm` … grouped by
  purpose (Standard, Additional, Dialogs, System, Network, Data …: no "RapidQ" group any more);
  an item's card gives its RapidQ name. The inspector's header says "Button1  RButton (RapidQ name:
  QBUTTON)", the designer tells a screen reader "Button1 (RButton)", F1 Help shows
  `RButton.Caption`, and completion offers `RButton`. The templates (but "RapidQ program"), Add
  Form, the About box ("Runs RapidQ programs, and takes them further"), the status bar ("RapidR
  BASIC"), View ▸ Theme ▸ Classic (Windows) and Studio's own code use RapidR's names (Studio's code
  converted by the importer: the same bytecode).
- **A file never mixes the two**: what the designer adds and what completion offers follow the
  file's own style — RapidR's names, or RapidQ's in a file written with them (a RapidQ program
  stays RapidQ-style); names already written are never changed.
- `.rqw`, `.rqb` and `.rq` files open in Studio (File ▸ Open, Open Folder, the language
  service), and the language service reads a project's include files on the web too.
- The manual: a new page, [Importing RapidQ programs](docs/manual/importing-rapidq.md) (what
  changes, the copy, Studio's steps with screenshots, the CLI), and "RapidR's names and RapidQ's
  names" in Getting started and Differences. Its screenshots are made again by
  `node tools/manual/shots.mjs` (scenes in `tools/manual/scenes/`).

### RapidR's names everywhere, and a RapidQ importer (R-NAMES, phase 1)
- **RapidR's names are the default**: `RButton`, `RLabel`, `RForm`, `RStringGrid` … (mixed case) in
  the examples, the docs and the manual, the new-project template, completion and hovers. RapidQ's
  names (`QBUTTON`) appear as "RapidQ name: QBUTTON". RapidR is compatible with RapidQ, not a copy
  of it: **the compilers keep accepting both names**, always, mixed, in any case — old RapidQ
  programs run unchanged (docs/q-and-r-components.md).
- **`rapidr import-rapidq <file | folder | project.rrproj> [-o out_dir]`**: a copy of a RapidQ
  program with RapidR's names, and a report (`rapidr-import-report.md`: every change by file, line
  and column, and what wasn't converted and why). The original is never touched. Token-aware, on
  the compiler's own parse: only type names (`AS …`, `EXTENDS …`) change — never strings, comments,
  your own names (`QButtonCount`), your own TYPEs or `$DEFINE`d names. Includes are followed into
  the copy; RapidQ's RAPIDQ.INC file stays out (RapidR supplies its constants; the
  `$INCLUDE "RAPIDQ.INC"` line stays, since the constants come with it). **Each program is proved**:
  it and its copy compile to the same bytecode, byte for byte. On RapidQ's own 428 example
  programs, all 175 that RapidR compiles today compile identically after the conversion
  (`tools/rapidq_import_check.py`).
- **`rapidr upgrade-names <file> [--dry-run]`**: the same for your own files, in place (`--dry-run`
  shows the diff); refused if the program would compile differently.
- **RapidQ's other source extensions, `.rqw`, `.rqb` and `.rq`,** are source files: `rapidr run` / `build` / `build-bc` / the `rapidr file.rqw` shortcut,
  `$INCLUDE`, projects, `rapidr dap`, the launcher, the VS Code extension and the installers' file
  associations ("RapidQ BASIC source") know them.
- The manual's component tables lead with RapidR's names (a *RapidQ name* column); the registry's
  JSON has each component's `spelling`; the AI prompt says to write RapidR's names.
- The language service: completion after `AS` lists `RButton` with "RapidQ name: QBUTTON" (in a
  RapidQ-compatible project RapidQ's names, with "RapidR name: RButton"); hovers say "RButton —
  RapidQ name: QBUTTON"; a RapidQ-compatible project now also warns on RapidR's names of RapidQ's
  components ("RButton is RapidR's name: RapidQ's compiler knows it as QBUTTON", with the fix).
- **Fixed**: `TYPE T EXTENDS RObject` (RapidR's name of RapidQ's QOBJECT) was a syntax error;
  RapidR's names in mixed case (`EXTENDS RPanel`, `AS REdit`) are now read exactly as RapidQ's
  (the same bytecode as `QPANEL`).
### RapidR Studio: the code editor works
- **Code completion and code help in Studio** (desktop and web alike): completion after `.` (members by type, ranked
  fuzzily, docs beside the list), signature help, hover, F12 (into another file too), Shift+F12
  (uses selected and listed in Output), F2 rename, squiggles with the Problems panel, Ctrl+. quick
  fixes, keywords in upper case as you type (`dim` → `DIM`), snippets.
  - Members after `.` for every kind of object: RapidR's and RapidQ's component names, arrays of
    components, the program's TYPEs (and TYPEs extending a component), WITH blocks, CREATE bodies.
  - After `AS`, the file's own style: RapidR's names (`RButton`, `RForm`); RapidQ's (`QButton`)
    still complete once you start typing one (`QBu…`), after RapidR's, and are always accepted. A
    file written with RapidQ's names keeps to them.
  - RAPIDQ.INC's constants (`mbYes`, `clRed`, `fmOpenRead` …) are offered even before the program
    includes it: choosing one adds `$INCLUDE "RAPIDQ.INC"` at the top in the same undo step.
  - **F12 is Go to Definition** on a name, in a form's code too; off a name, and in the
    designer, it switches between the form and its code (Delphi's; F7 / Shift+F7 too).
- **RAPIDQ.INC constants without the include are flagged** (squiggles, the Problems panel,
  Ctrl+.): "mbYes is a RAPIDQ.INC constant — add $INCLUDE "RAPIDQ.INC"", a warning — without the
  include RapidQ reads `mbYes` as a variable that is 0 (Robert's save prompt showed only OK). The
  quick fix adds the include after the file's header comments. How programs run doesn't change.
- **The manual's new page [The code editor in RapidR Studio](docs/manual/studio-editor.md)**:
  completion, Ctrl+Space, parameter info, hover, F12, problems and quick fixes, the shortcuts,
  with screenshots made again by `node tools/manual/shots.mjs` (scenes in `tools/manual/scenes/`).
- The code editor's BASIC is `rapidr-basic` ("RapidR BASIC", also in Studio's status bar);
  `rapidq-basic`, its old id, still works wherever a language id is given (RCODEEDITOR's
  `Language`, RDIFFVIEW's, saved settings).
- **Tab no longer types a stray character.** A TAB in any QMEMO / QRICHEDIT was drawn as the font's
  missing-glyph box; it is now the blank to the next tab stop. In the code editor, Tab / Shift+Tab
  indent and outdent by the file's unit.
- **Edit menu**: Find Next / Previous, and Edit ▸ Advanced (indent / outdent lines, select next
  occurrence, go to matching bracket, fold / unfold, parameter info, hover, word wrap).
- **RCODEEDITOR**:
  - `ApplyPatches(Patches, [Continues])` applies a designer change to the code as one undo step of
    the editor's history;
  - `GotoMatchingBracket`;
  - the debugger's marker kinds (breakpoint, conditional, logpoint, disabled, current, frame,
    exception) with notes at a line's end;
  - `DebugHover` and `WordAt`;
  - OnChange after ApplyPatches / Undo / Redo.
- Fixed: an undo that removed lines could crash the code editor's view.
- Fixed: the semantic model took the first use of an undeclared name for its declaration (the
  compiler's own `DIM … AS DOUBLE` for it spans the whole program); INPUT and SWAP now count as
  storing into their variables.
- `examples/basics/files.rr` includes RAPIDQ.INC for `fmOpenRead`.
- Code shows each character: JetBrains Mono's ligatures (`->`, `<=`, `<>`) are off in the code
  editor and RDIFFVIEW; other text keeps its fonts' contextual alternates (Inter's colon between
  digits, `0:00.0`, as the approved images have it).

### Changed
- **The right mouse pointer everywhere, by default** (Robert: hovering the property inspector's divider
  didn't change the cursor, so you couldn't tell it drags): every component answers "what pointer is
  here?" from its own hit test (`ComponentKind::pointer`), so any program gets it with no code, on the
  desktop (winit's cursor) and the web (the canvas' CSS cursor) alike. Column-resize (`col-resize`)
  over a `RSPLITTER` between columns (row-resize between rows), a grid's column edge with goColSizing
  (and, new, goRowSizing: a fixed column's cell border drags the row's height — row-resize), a list view's and a header's section edges, Studio's inspector divider, dock splitters and split
  editors; the sizing arrows over a `RFORMMDI` child's edges and corners and a status bar's grip; the
  I-beam over text boxes, memos, rich edits, a code editor's text (not its gutter or bars), an
  editable combo box's text, an open in-place editor and Studio's search boxes; the hand over links in
  the Output panel; the form designer's handles, move pointer and placing cross; the closed hand
  while a dock tab, document tab, project file or toolbox item is being carried. A `Cursor` the program
  sets still wins (RapidQ); a disabled component shows the arrow; `crHSplit` / `crVSplit` are now the
  col-resize / row-resize pointers (they were the plain double arrows). In RapidR's look the divider
  and splitter under the mouse light up in the accent. Test hook: `comp.__cursor_X_Y` in
  `RAPIDR_TEST_DUMP` prints the CSS name of the pointer at (X, Y) of the component.
- **`rapidr build` and Studio's Build make the release build, and the output folder gets only the app**
  (Robert: a build left the generated Rust, cargo's files and a debug executable beside the program).
  A build is optimized by default (on macOS universal: Apple silicon and Intel; symbol tables stripped:
  `hello_form.app` 62 MB before, 50 MB now); `--debug` asks for a quick one, and Studio's Run (F5)
  stays the one for debugging. The output folder is the one named (`rapidr build prog.bas dist`,
  `--output`), else the project's (`[build] output`, `build` by default), else the source's (RC.EXE's
  way). The generated Rust and cargo's target folder live in the **build cache**
  (`~/Library/Caches/RapidR/build`, `%LOCALAPPDATA%\RapidR\Cache\build`, `~/.cache/rapidr/build`,
  `RAPIDR_BUILD_CACHE`): the Rust is deleted after the build, the program's own files in the shared
  target too, the compiled runtime stays — a GUI program's universal release build took 6 min the
  first time and 22 s the next (before: the debug build in `hello_form_rust/`, 1.7 GB beside the
  source, every time). A RapidR version gets its own cache; one unused for 30 days is removed.
  **Keep the generated Rust source** (`--keep-rust`, `[build] keep_rust`, Project Options; off by
  default) leaves it in `<output>/<program>-rust-source` with a README. The generated code's own
  warnings are no longer printed. `rapidr codegen` still writes `<program>_rust` for those who want it.
- **Studio's Build log**: the Output panel's Build page shows `rapidr build` as it runs; the app's
  path and the kept Rust source are links, and "✓ Built Notes.app in 27 s — Reveal in Finder" (File
  Explorer, Files) shows the app selected; a failed build says so in red. The console takes
  terminals' OSC 8 hyperlinks (any program's output may carry them; OnLinkClick gets the URI).
  **Project Options** has the output folder and Keep the generated Rust source, says what Build makes
  (a release build; Run is for debugging), and the Rust source option is off for interpreted builds.
- **Save prompts as macOS and Xcode ask them**: "Do you want to save the changes you made to main.rr?"
  with Save (Enter), Don't Save and Cancel (Escape); quitting with several documents changed asks once:
  Save All (Enter), Review Changes… (each document in turn), Discard Changes, Cancel. Windows gets
  Save / Don't Save / Cancel in its own order. Every choice quits or stays as it says, on the desktop
  and on the web, where the page then says "RapidR Studio has closed" with a button to open it again.
- **`Form.Modified`** (RapidR's): a window with changes not saved — macOS draws the dot in its close
  button, and a web page asks before it's left (the browser's own "Leave site?", `beforeunload`).
  Studio sets it while a document has changes.
- **Real bold and italic faces** instead of the regular letters drawn heavier and slanted: Liberation
  2.1.5's own Bold, Italic and Bold Italic (Sans, Serif, Mono; SIL OFL 1.1, from the official release,
  SHA-256 checked) are built in, cut to the Latin scripts without hinting (36-39 KB each) and renamed
  "RapidR Text Sans / Serif / Mono" as the licence asks; RapidR Sans has a Bold of its own (MS Sans
  Serif Bold's widths, a pixel wider a character, and its two-pixel stems). Bold Arial text is as wide
  as Arial Bold's ("Pantry" 9 pt bold 37 pixels, "Hello" 12 pt bold 39, as RC.EXE measures), italic is a
  designed italic; a character the styled face lacks (Greek, Cyrillic, ...) is drawn from the Regular
  face, made bold or slanted, as before. The built-in fonts are now in a program once (statics, not
  copies of consts): the interpreter and the web runtime are 1.3 MB smaller in spite of the new faces.
  The approved images of fonts, words, spacing and the data-science examples changed with them
  (bold looked at beside RapidQ's, all themes, 1x and 2x, web identical to the desktop).

### Look
- **RapidR's look is every program's default** (`$THEME rapidr`, or no
  `$THEME` at all): RapidR Studio's own look, one set of tokens for Studio
  and programs, made from the brand palette (Ink, Paper, RapidR Blue, Blue
  Deep, Blue on Dark, Slate, Mist, Board; `docs/theme-tokens.md` maps every
  token). Light, dark (on the brand's Ink) and high contrast, following the
  system's setting — and its changes — unless the program names one;
  `Application.Theme` reads `rapidr light`, `rapidr dark`, `rapidr high
  contrast` or `classic`. RapidQ's default font is drawn in Inter at MS Sans
  Serif's 11 pixels (chosen by the clipping audit: 26 more of the RapidQ
  corpus' 539 fixed-size captions clip at 11 px, 56 at 12, 78 at 13); fonts
  a program names keep their face.
- Refined controls: 5-pixel corners on controls, 8 on menus, drop-down
  lists and windows; soft shadows under menus, drop-down lists and tooltips
  (a new `Op::Shadow`, the same on the desktop and the web) and around a web
  page's windows; buttons with a hairline rim a step darker along the
  bottom; one focus ring (2 pixels, RapidR Blue) on text boxes, buttons and
  combo boxes, a 1-pixel accent border on lists, trees and grids; the
  selection in the accent while a list has the focus, grey without it;
  grids with light lines, a tinted selection, the current cell ringed and
  text centred in its row; tabs as words underlined in the accent.
- **Sizes never change between themes**: an AutoSize label's and a tab's
  sizes are measured as RC.EXE measures them in every theme (the text runs
  on rather than being cut where Inter is wider). `tests/theme_geometry.mjs`
  runs every GUI program of `examples/` (and, with `--corpus`, RapidQ's own
  examples) in all four looks and compares every component's place.
- **`$THEME classic`** is RapidQ's look, and a web page's window frame in it
  has Windows' own 16 × 14 caption buttons (they were 26 × 23, huge at 2×).
  The web frame is now the kernel's (`window_frame`), as QFORMMDI's children
  are on every runtime. The classic names stay classic (`System`, `Light`,
  `Windows`, `Win95` …); `modern`, `dark` and `highcontrast` are RapidR's
  light, dark and high-contrast looks.
- RapidR Studio: **Preview in Classic** (View menu, tool bar, command
  palette; saved with the settings) runs the program and draws the designer
  in RapidQ's look without editing the source (`RPROGRAMSESSION.Theme`,
  `RDESIGNSURFACE.Theme`). Studio's status bar is quiet at rest and takes a
  state's colour while the program runs, pauses or fails.
- Text a program left uncoloured on a colour reads at WCAG AA in RapidR's
  look (a gauge's percentage white on the accent, the theme's text on the
  rest); RapidR Studio's panels and icons follow the theme's palette in
  dark and high contrast (they fell back to the light icons).
- RC.EXE pixel comparisons (`tests/native_gui_events.mjs`, the kernel's unit
  tests that check ops) name the classic theme explicitly; the visual
  gallery's themes are `classic`, `rapidr-light`, `rapidr-dark`,
  `rapidr-high-contrast`.

### Fixed
- **Examples with data files run in RapidR Studio on the web** (Robert: dataframe.rr said "$RESOURCE
  STAFF_CSV: file not found in the project's assets"). The files a program names — a `$RESOURCE`'s
  file, a CSV it loads (`"staff.csv"`), any file beside it whose name is written in its source — belong
  to its project: the project tree lists them (Resources, Assets, Data), and on the web Studio builds
  the `$RESOURCE`s in from them and gives the others to the program's frame as the files of its
  folder. All 30 examples open and run from Studio on the web.
- **RPLOT.Clear keeps the chart's size** (as Matplotlib's `clf`): a chart on a form no longer jumps to
  640 × 480 when cleared.
- **`rapidr build` of a program whose name starts with a digit** (`3dcube.bas` -> `3dcube.app`)
  failed on macOS with "3dcube.app: No such process": `codesign --verify` reads a relative
  argument starting with a digit as a process id. The app is now signed and verified by its
  absolute path (a test builds a digit-named app in the current folder, signs and verifies it).
- **The release's home** ships the files its crates include from outside their folders (the program
  icon masters, the icon inventory) and drops the crates' `[dev-dependencies]`: the vendored
  workspace failed to build on Linux and Windows. The Windows installers (x64, arm64) and the Linux
  packages (`.deb` and `.tar.gz`, x86_64 and aarch64) build from it; the Linux ones pass their smoke tests.

### Release packages
- **"RapidR Studio" everywhere**: the macOS app is `RapidR Studio.app` (it was `RapidR.app`; the VS Code
  extension finds both), the Windows Start menu entry and the "Open in" actions, and the Linux menu entry
  say RapidR Studio.
- **Project files (`.rrproj`) open in RapidR Studio** on a double click: a file type on Windows (SDK
  installer), macOS (Studio.app's document types; `rapidrw` hands them to `rapidr ide`) and Linux (a
  shared-mime-info type and the menu entry's MimeType). The smoke tests check them, the menu entry's
  name and the Start menu entry (and that uninstalling removes them).
- `tools/release/README.md`: the release flow in one page (one entry script per system, where the files
  land, the checks, the signing hooks, the fresh-user journey); `macos.sh`'s header documents the
  Developer ID and notarization hooks.

### Documentation
- **Manual: installing** on Windows (which installer, each page of the wizard, the PATH and `.bas`
  options, uninstalling) and on Ubuntu or Debian (`sudo apt install ./rapidr_<ver>_<arch>.deb`, what
  goes where, SDK and Runtime packages replace each other, removing it), with the matching
  troubleshooting entries; **building apps**: the app's name (digits first, the bundle ID it makes,
  checking a signature by hand).

### Added
- **The CSV Explorer** (`examples/data/csv_explorer.rr`, the first example on Studio's Welcome page):
  drop a CSV file on the window (or Open CSV…, or the built-in sample `shop.csv`) and see it as a
  table sorted by a click on a heading, filtered by any text, each column's count / min / max / mean,
  and a live RPLOT bar, line or scatter chart of any two columns — native, interpreted and in a
  browser alike. The manual's [Data science](docs/manual/data-science.md#tutorial-the-csv-explorer)
  page walks through it, with screenshots.
- **`Form.OnDropFiles (Files AS STRING)`** (RapidR's; RapidQ has no file drop event): files dragged
  from Finder, File Explorer or a file manager — in a browser, from the computer — and dropped on a
  window; their paths one a line, one event for a drop of several. In a browser each file is read into
  the program's own files under its name. A form without the handler ignores drops (the browser never
  replaces the page with the file). Test hook `form.__drop` with `RAPIDR_TEST_DROP=a;b`.
- **RPLOT in the designer**: a chart with no data yet shows sample bars under its Title, so the form
  shows where the chart is (dataframe.rr's chart is now an RPLOT on the form).
- **Studio builds apps without Rust.** Run has **Build Native App** (needs Rust, free) and **Build
  Interpreted App** (no Rust), beside Build (Ctrl+Shift+B), which does what Project Options says.
  On a computer without Rust, Build Native App asks first: Build Interpreted Instead (Enter), Install
  Rust... (runs `rapidr setup`, its output on the Build page) or Cancel; a new project there is set to
  build interpreted, and Project Options says "(Rust not installed)" beside Compiled. The log names the
  kind: "✓ Built Notes.app (interpreted) in 4 s". `rapidr build` native without Rust says so plainly
  (use `--interp`, or `rapidr setup`) instead of failing in cargo; `rapidr setup --rust` answers
  whether native builds can run (exit code), and RAPIDR_NO_RUST=1 pretends there is no Rust, to test
  those paths. New RPROJECT members RustReady and InstallRust. Manual: building-apps.md, "Do I need Rust?".
- **Serial ports for ESP32 / Arduino / IoT boards** (RComPort, RapidQ's QCOMPORT; RapidQ's members
  unchanged, RapidR's extras added, the same in native builds, the interpreter and the browser):
  **ListPorts** with each port's USB vendor / product IDs, description, maker and serial number
  (IOKit on macOS, SetupAPI on Windows, sysfs on Linux, Web Serial's granted ports in the browser;
  CP210x, CH340, FTDI, ESP32 USB JTAG/serial … named from their IDs where the system says nothing),
  **FillList** fills a list or combo box; the **DTR / RTS** lines set and read back, **CTS / DSR /
  CD / RI** read, **SendBreak**; **ReadLine**(Timeout) with a configurable **LineEnd** and
  **HasLine**, the **OnLine** event; **OnPortsChanged**(Added, Removed) when a USB adapter is plugged
  in or out. Closing a port in the browser now closes the Web Serial port (it can be opened again).
  Tested on a real ESP32 (an M5StickC Plus on FTDI): reset through DTR / RTS, its ROM boot log read,
  interpreted, native and in Chrome. Example: `examples/iot/esp32_monitor.rr`; manual: "Serial
  ports and IoT boards"; `tools/esp32_check.sh` checks a real board by hand (never writes to it);
  the scripted test ports gain `esp32` (a board that prints its boot log when reset).
- **RapidR Studio's panels as public components** (docs/ide-plan.md I1, L-PANELS), the same on the
  desktop, in native builds and on the web: **RPROPERTYINSPECTOR** (Delphi's object inspector: typed
  editors from the language registry, a visual Anchors pin editor, colours, fonts, lists, the Events
  page, defaults dimmed and reset, "RapidR extensions", multi-selection, a designer's selection
  followed), **RTOOLBOX** (RapidQ's and RapidR's components with RapidR's icons, search, drag and
  drop), **RPROJECTTREE** (.rrproj projects by kind, forms' components, rename, delete, reorder),
  **ROUTPUTCONSOLE** (ANSI output, build log, problems, `file:line` links, search), **RTOOLBAR** as
  a real toolbar (icon buttons, toggles, overflow, customizable) and **RCOMMANDPALETTE** (fuzzy
  commands). Example: `examples/studio/panels.rr`.
### RapidR Studio: the debugger
- **Run and debug on both hosts, the same way**: F5 runs the project under the debugger (its own
  process on the desktop, a sandboxed frame on the web), Ctrl+F5 without it, Stop, Restart, Run in
  Browser (desktop: `rapidr serve` on 127.0.0.1 under a random path). On the web the program's
  windows float over the whole page and can be dragged to every edge, never clipped.
- **Breakpoints**: F9 or a click in the gutter, in any file; conditions, hit counts, logpoints
  (`{expr}`), on / off, each its own marker; the Breakpoints pane lists and edits them. One on a line
  without code moves to the line the program stops at (one after the last code says it never
  stops). The project's breakpoints and watches come back when it is opened again.
- **Paused**: the line marked, in whichever file; Step Into / Over / Out (F11 / F10 / Shift+F11),
  Run to Cursor, Pause — which stops a program waiting for its events (its ShowModal, a dialog) at
  once, at the line that waits. Run-time errors stop at their line, the message beside it.
- **Variables, Watch, Call Stack, Immediate, data tips**: locals and globals, arrays, TYPEs and
  components open to their children; F2 sets a value in place and the program runs on with it; the
  Call Stack takes the toolbox's place while paused (picking a frame shows its line and locals);
  `? expr` and statements in Immediate; a resting mouse shows a value.
- RPROGRAMSESSION: `StopOnEntry`, `StopReason`, `StopMessage`, `Frame`, `LocalsRef`, `GlobalsRef`,
  `Watches`, `BrowserURL`; `SetBreakpoint`, `ClearBreakpoints`, `RunToCursor`, `StackTrace`,
  `Variables`, `WatchValues`, `Expand`, `Evaluate`, `SetVariable`, `RunInBrowser`, `StopBrowser`;
  `OnVariables`, `OnEvaluate`, `OnBreakpointPlaced`. RCODEEDITOR's TriggerHover gives the data tip
  while paused; a hover is as wide as its text.
- The Immediate pane answers every line under it, however fast lines are typed (it ran the last
  line, not the caret's, and kept only the newest answer).
- Manual: **Debugging in RapidR Studio** (`docs/manual/debugging.md`), step by step with
  screenshots made again by `tools/manual/shots.mjs` (scenes in `tools/manual/scenes/`).
- Fixed: RDIFFVIEW's columns measured with JetBrains Mono again (a merge had them measure RapidR
  Sans); the supply-chain check follows the bold faces' code (ttf-parser still parses only the
  built-in fonts).
### RapidR Studio: documents as tabs, Find in Files, F1 Help, templates
- **No window inside the window**: Studio's documents are tabs, as in Xcode, VS Code and Delphi —
  close buttons, a dot for changes not saved, middle click closes, drag along the strip reorders. A
  file with a form is one tab with a **Design | Code** switch (and both side by side); F12 toggles,
  F7 / Shift+F7 pick. A tab dragged to a side of the documents splits them into **groups** (or
  Window ▸ Split Right / Split Down), with drop outlines and splitters. The layout, the open files
  and each file's view come back when the project is opened again; the window's place too.
- **RDOCKMANAGER** (tabbed documents): `AddView`, `DocumentView`, `DocumentModified`,
  `SplitDocument`, `DocumentGroupCount`, `OnDocumentView`; groups and views in `SaveLayout`.
- **QFORMMDI children resize by every edge and corner** (Windows' sizing border, resize pointers,
  never under Windows' 136 × 39 least size — read with RC.EXE on Windows 11), desktop and web.
- **Find in Files** (Ctrl+Shift+F): match case, whole word, regular expressions, results by file
  linking to the line, Replace All (RPROJECT `Find`, `Replace`, `FileText`).
- **F1 Help** pane from the language registry (RLANGUAGESERVICE `Help`) for the word at the caret,
  the inspector's row, the toolbox's item, the designer's component; Insert types the syntax.
- **New Project gallery**: Form app, Console, RapidQ-compatible form app (main.bas, compat on), Data
  dashboard, MDI app with menus. Closing a changed file asks Save / Don't Save / Cancel.
### RapidR Studio: Markdown files formatted (Preview | Source)
- **A `.md` file opens formatted** — a README, a RapidQ import's report — with a **Preview |
  Source | Side by Side** switch on its tab (F12 toggles, F7 the source, Shift+F7 the preview);
  what is typed in the source shows in the preview at once; the Outline lists the headings (a
  click scrolls there); a link to another file opens it (a Markdown one in Preview, at its
  `#heading`), a web link opens in the browser; File > Open takes a `.md` on its own. The source
  isn't coloured as BASIC (`RCodeEditor.Language`, new, `"basic"` by default). Desktop and web.
- **`RMarkdownView`**, a public component on the UI kernel (the same on the desktop and the web):
  CommonMark with GitHub's tables and strikethrough (pulldown-cmark, MIT) laid out with parley —
  headings, bold / italic / struck out, inline code, nested lists, block quotes, code blocks (BASIC
  ones coloured), tables, rules, links; scrolls, selects and copies (a table's cells by tabs);
  light, dark and high contrast; screen readers get its headings (with their level), lists,
  tables and links (new accessibility roles on AccessKit and the web's ARIA mirror). `Text`,
  `LoadFromFile`, `OnLinkClick`, `OpenLinks`, `ScrollTo`, `Headings`, `PlainText`, `SelText`.
  A 7,700-line report lays out in ~80 ms; scrolling draws only what's in view. Manual:
  docs/manual/markdown.md.

### RapidR Studio: the panels work
- **The real panels replace the stand-ins** (`ide/panels.inc`): the tool bar (RTOOLBAR, buttons with
  tooltips), the command palette (RCOMMANDPALETTE), the project tree (RPROJECTTREE), the toolbox
  (RTOOLBOX), the property inspector (RPROPERTYINSPECTOR) and the output console (ROUTPUTCONSOLE:
  output, build log, problems with links). Desktop and web byte-identical.
- **Properties work**: select a component on the form (or in the project tree, or by name in the
  palette) and the inspector shows its properties as its CREATE block writes them (defaults
  dimmed; Left / Top / Width / Height as laid out), by category or A–Z, searched, documented under
  the rows. A change is one undo step in the designer, written into the code as the smallest edit;
  the code edited reaches the inspector. Values are written as RapidQ needs them: Booleans as 1 / 0,
  and in a `.bas` program without RAPIDQ.INC a RapidQ constant as its number (RC.EXE reads an
  undefined `clRed` or `True` as 0).
- **Events**: double-click an event's row: its SUB is written with the registry's parameters (a
  DECLARE beside the file's, or the SUB before the form, as RC.EXE needs) and bound, or found; the
  caret goes inside it. The Events page offers the file's SUBs that fit.
- **Toolbox**: the registry's components by group with RapidR's icons; search also by what a
  component is ("chart" finds RPLOT); a card as the tooltip; double-click or Enter adds, a click arms
  the placing tool, a drag drops on the form.
- **Project tree**: forms with their components (selecting one selects it in the designer), files
  with changes marked with a dot, Project > Add Form / Add Module, rename (F2), take out of the
  project, reorder.
- **One search** (Ctrl+P, Ctrl+Shift+P): commands, examples, the project's files, the file's
  symbols and its form's components; `:N` goes to line N (the palette's AddPrefix).
- Empty panels say what to do (`EmptyText`).
- On the web an RTOOLBAR starts 32 pixels high, as on the desktop.

### RapidR Studio: the form designer works
- A file's [Design] tab shows its form as the running program shows it, at its own size, read from the code (`RDESIGNSURFACE.Source` on rapidr-designer's Document); the stand-in scanner and its boxes are gone.
- Drag the form's right edge, bottom edge or corner to resize it: anchored and aligned components follow live, and the new size (and where the anchors moved them) is written into the CREATE block, one undo step.
- Add components: click a type then click or draw on the form (`PlaceType`), drag one in (`DragComponent`), or `AddComponent(Type, X, Y)`. Names are unique in the whole file; typing right after adding sets the Caption; dropping on a panel puts it inside.
- Select, move, resize, delete, copy / paste / duplicate, align and arrange with the mouse or the keyboard; every change is announced to screen readers, written into the code as the smallest edit (`OnSourceEdit`), and undone to the exact bytes. Double-click makes or finds the event handler (`CreateHandler`). While the code has errors the designer is read-only under a banner.
- The menu editor on the form's own menu bar (Format ▸ Menu Editor): Type Here, `&` mnemonics, separators, submenus, ShortCut typed as the keys pressed, Checked, drag to reorder; each item a QMENUITEM CREATE block, one undo step.
- The Tab-order editor (Format ▸ Tab Order): click the components in the order wanted; each click writes `TabOrder`.
- Captions edited in place with F2 or a second, slower click (a double click still makes the event handler).
- Zoom from 25 % to 400 %: View ▸ Zoom, Ctrl / ⌘ + = − 0, Ctrl / ⌘ + the wheel, a trackpad's pinch; crisp at any zoom, the program's pixels unchanged.
- The tray also shows the dialogs a program creates outside its form (notepad's OpenDialog and SaveDialog), inspected and edited like the rest.
- **Add new forms** (Project ▸ Add Form, as Delphi's File ▸ New ▸ Form): the project tree names the new file (Form2.rr, Enter), it is written as `CREATE Form2 AS RForm` with its Caption and size, added to the project, **included by the main program** (`$INCLUDE "Form2.rr"` after its other includes or its opening directives — one undo step in its editor) and opened on its designer; Project ▸ Add Module the same for a module. A program of several forms runs on every runtime: a button's OnClick with `Form2.Show` shows it (`tests/studio_add_form.mjs`: made in Studio and run, desktop interpreted and native, and in Studio's run frame on the web).
- Renaming a form or module in the project tree (F2) makes the main file's `$INCLUDE` follow it; taking it out of the project takes its `$INCLUDE` out too (RPROJECT `IncludeRenameEdit`, `IncludeRemoveEdit`). In a RapidQ program (main.bas) new forms and modules are `.bas` files.
- **RForm in the toolbox adds a form** (Robert's report: clicking it did nothing): click or double-click is Project ▸ Add Form, RFormMDI adds an MDI main window; dragged onto a form it is never nested — a new window, and the status bar says why; dragged onto an RFormMDI it becomes one of its child windows, RapidQ's way: a panel on the MDI form, named as a form, and `Main.AddChild(Form1.Handle, "Form1", 0, 0, 0, 0, 0, 1)` after it (one undo step; `RDESIGNSURFACE.AddMdiChild`). The items' tooltips say so. RFormMDI forms are designed like any form (their menu bar on the canvas).
- **The form itself in the inspector** (Robert's report: its Events page was empty): with nothing selected the inspector shows the form — its properties and all its events (OnShow, OnClose, OnResize, OnPaint, OnKeyDown, the mouse's …); double-clicking one writes `SUB Form1Show`, binds `OnShow = Form1Show` and goes to it, as for a component.
- New manual page, [Designing forms in RapidR Studio](docs/manual/designing-forms.md): a tutorial (a program of two forms, step by step, with screenshots made by `tools/manual/shots.mjs`), the toolbox, moving and aligning, the inspector and events, programs of several forms, the code the designer writes.
- Component names are unique in the whole program (a RapidQ program's names are global): Form2's Button1 makes Form1's next button Button2 (`RDESIGNSURFACE.ReservedNames`, `RPROJECTTREE.ComponentNames`), and a new form file is never named after a component the program has.
- The designer writes RapidR's names — `RForm`, `RButton`, `RLabel`, `REdit` … — in a file written with them (the toolbox shows them too); a file written with RapidQ's names (`QFORM`) keeps getting RapidQ's, never mixed. New Form, Data and MDI projects start with RapidR's names.
- A file without a form offers "Add a Form" (and Project ▸ Add Form on it): `CREATE Form1 AS RForm` (QFORM in a RapidQ file) and `Form1.ShowModal` written at its end; its tab gets the Design | Code switch.
- Every component of the toolbox can be added (a unit test adds each one to a RapidR and a RapidQ form and compiles the program on both backends): Enter / double-click puts it at the first free place in reading order, never on another one, as large as fits when its default size doesn't (a 640 × 480 RPlot on a small form); a new label sizes itself to its caption (RapidQ's AutoSize: no Width written after its Caption).
- The inspector edits a component's **Font** by its parts (Name, Size, Color, Bold, Italic, Underline, StrikeOut), written as RapidQ's programs write them in a CREATE block (`Font.Name = "Arial"`, `Font.Bold = 1`), drawn at once in the designer. RapidQ's constants a program doesn't define — without RAPIDQ.INC, in a `.rr` file too, where `clYellow` reads as nothing — are written as their numbers (`Color = &H00FFFF`, `Alignment = 2`), so what the inspector sets is what runs.
- A form or module that the main program includes is analysed as part of the program: Form1 and the main file's SUBs are known in Form2.rr (no false errors, completion of Form1's members), and Run checks the whole program's errors whichever file is shown.
- On the web, the language service and the designer read `$INCLUDE`d files from the page's files (the preprocessor's source reader), as the desktop reads the disk: no "missing include" error in Studio in a browser.
- A form's file opens on its designer and the designer takes the keys (the desktop's `SetFocus` on a design surface did nothing); F12 in the code on a name goes to its definition (VS Code), elsewhere it switches between the form and its code (Delphi); the outline names types as the file writes them.
- Dragging from the toolbox shows the component itself at 60 % and a "not allowed" pointer outside a designer; a drop settles in for 100 ms. The form sits on the designer's backdrop with its own scroll bars, never clipped.
- One undo history per file: the designer's changes are the code editor's undo steps (`OnSourceStep`, `SharedUndo`, `OnUndo`; RCODEEDITOR's `ApplyPatches`), so typing in the code no longer clears them and Undo goes back in the order things were done.
- Fixed: RCODEEDITOR's OnChange now fires after `ApplyPatches`, `Undo` and `Redo` on the desktop and the web (a code editor took the text edits' path, which returned first).
### Removed: the old HTML / Monaco web IDE
- **RapidR Studio is the web IDE** (Robert's decision, 2026-10-08): the old HTML / Monaco IDE (`web-ide/`, ~10,000 lines of JS / HTML / CSS plus the vendored Monaco) is deleted, with `rapidr lang export --web-ide` and its generated `lang-data.js`, the brand export's copy of the icons into it, and every reference (regress, docs, release scripts, `tools/lang_seed.py`). Nothing shipped it any more (SEC-17).
- What a program's run frame needed from that IDE's page is Studio's now (`ide/web/studio.js`): the browser's Open / Save pickers shown for the sandboxed frame (writes only to files the user picked during that run) and the program's RWEBSTORAGE kept per program (1 MB). Studio's web page has RapidR's icons (favicon, home-screen icon).
- Its browser suites, sorted (docs/studio-wow.md §7): the web runtime's run programs on the runtime's own page (`tests/web_run.mjs`: `web_align`, `web_canvas`, `web_components`, `web_dialogs`, `web_grid`, `web_grid_draw`, `web_lists`, `web_objects`, `web_owner_list`, `web_picture`, `web_reentrant_events`, and the conformance, SQLite, VM-yield, modal-focus and corpus runners) or in Studio's run frame (`tests/studio_run_frame.mjs`: isolation, storage, 1:1 pixels); new `web_debugger`, `web_multiform`, `web_pixels`; new Studio flows `problems` and `run-ansi` (CLS / COLOR / LOCATE in Output). IDE features Studio doesn't have yet are listed against their checklist items (the assets manager is the new PRJ-5); the old IDE's own probes are deleted. `console_ansi` passes on the web (no xfail).
- Studio: Build with errors shows Output's Problems page (it named a pane that no longer exists).
### Security
- **Every web page RapidR builds has a Content-Security-Policy**, made from
  what the program uses (SEC-15): no inline scripts, `'unsafe-eval'` only for
  RJAVASCRIPT, frames only for RWEBVIEW, other servers only for the network
  components and the addresses the program spells out; `rapidr build --web
  --csp "…"` (and `bundle-bc --csp`) adds the author's own. Builds also write
  `_headers` and `.htaccess` with the same policy, `frame-ancestors 'self'`,
  `nosniff`, COOP and CORP. A program showing someone else's markup through
  RDOM no longer runs its scripts.
- **RWEBVIEW's page runs at an origin of its own** (SEC-12): the frame's
  default sandbox leaves `allow-same-origin` out (`Sandbox` opts in), and its
  `Html` runs in `rapidr-webview.html`, a frame every web build ships, with
  its scripts working as before.
- **Names in built pages are escaped for where they go** (SEC-16): a project
  or file named like markup or script is text in the page; no name is
  written into a script any more.
- **The release's web bundle is RapidR Studio** (SEC-17): `tools/release/web.sh`
  ships `tools/build_studio_web.sh`'s site; programs' bundles no longer carry
  any of the old web IDE's files.
- **`rapidr lsp` reads only the workspace** the editor opened (and the
  folders of the files it opened); **`rapidr dap`** launches only RapidR
  programs and never passes loader variables (`LD_PRELOAD`, `DYLD_*`,
  `PATH`, …) to them (SEC-18). The authentication `rapidr mcp` will need is
  designed (docs/security-audit.md §7).
- Supply chain (SEC-20): memmap2 0.9.11, anyhow 1.0.104; cargo-deny fails on
  unsound crates; `cargo deny` and `cargo audit` clean. Signing the release
  checksums with minisign is proposed (docs/security-audit.md §8).
- A `security` stage in `tools/regress.sh`, with a regression test for each.

### RapidQ's DirectX and Direct3D examples (checked against RC.EXE)
- **Direct3D draws on the GPU, on every system**: a QDXSCREEN's `Render` is
  rasterized by wgpu — Metal on macOS, Direct3D 12 on Windows (its WARP
  where there's no GPU), Vulkan or OpenGL on Linux, and WebGL 2 in the
  browser — and read back into the screen's back buffer, so the FPS text
  drawn after it, `Pixel` and `Flip` work as before. Windows has had no
  `d3drm.dll` since XP, so this is how RapidQ's Direct3D programs run there
  too. D3DRM's own model (frames, lighting per face or vertex, wraps,
  shadows) is unchanged and shared by every runtime; the software
  rasterizer it replaces was the reference it was checked against, then
  removed (one renderer). RapidQ's largest model (`Park.x`, 16,000
  triangles drawn) takes 3.7 ms a frame at 640 × 480 and 5.3 ms at twice
  that, against 8.2 and 16.5 on the CPU; the corpus's programs, every `.X`
  model and the probes look as before (and as RC.EXE's beside them), and
  the web's pictures equal the desktop's. The web runtime grows by 2.6 MB
  (WebGL 2 through wgpu).
- **One OnResize for one resize in the browser**: dragging a window's edge
  or a status bar's size grip fired OnResize for the new width and again
  for the new height; now once, as on the desktop.
- **`WITH v.R … END WITH`** on a QRECT field of an object reaches the field
  (`.Bottom = 99` stored into `v.R`), as RC.EXE does; RapidR stopped with
  "nested member-access store not yet supported".
- **QRECT (and QNOTIFYICONDATA, QFONT) as a field of an object**: `TYPE T
  EXTENDS QOBJECT … R AS QRECT` compiles and behaves as RapidQ's — each
  instance its own record, by reference to SUBs, through This in the TYPE's
  code; `Datatype QRECT not supported in STRUCT` only for a TYPE / STRUCT
  without EXTENDS, as RC.EXE says it. 8 of RapidQ's Direct3D examples (their
  `RapidQ_D3D.inc`) compile again.
- **A TYPE no DIM makes isn't compiled**, as in RapidQ: its methods may call
  what isn't declared (RapidQ's `Lights_pyramid.bas`).
- **`Obj.Method = a, b, c`** calls the method (RapidQ's `QMORP.BAS`), `= a`
  passing a number as itself and text as ""; RC.EXE's errors for a property
  or a method taking nothing.
- **OnResize** fires as RapidQ fires it: around OnShow when a form's window
  is made, then for every change of its size (it fired only for the user's
  drag) — RapidQ's `CoolGauge.bas` shows its whole gauge.
- **QBITMAP.PixelFormat** reads the loaded BMP's (8-bit 3, 24-bit 6), as
  RapidQ's terrain examples check.
- **QD3DANIMATION and QD3DANIMATIONSET** (RC.EXE's: Parent only, arrays
  allowed), **QD3DMESH.MaxY / MinY** (read-only), **QD3DFRAME.FogEnabled**
  a property.
- **A full-screen QDXSCREEN covers its form** (DirectDraw's exclusive mode):
  the picture fills the window, letterboxed, over the form's other
  components, desktop and web.
- Native builds: a CREATE inside a SUB that sets its object's `Font.Name`
  compiles (RapidQ's `3DPong_aDelic2.bas`).
- **RapidQ's own Direct3D, seen at last**: RapidQ's examples ship the
  `d3drm.dll` Windows dropped after XP; beside it RC.EXE's programs run in
  the Windows 11 VM, and RapidR's 3D was compared with RapidQ's for the first
  time — the corpus's programs (windowed copies of the full-screen ones),
  probes, and every `.X` model of the corpus. What it showed is fixed:
  **D3DRM makes a zero vector unit as (1, 0, 0)** — `SetRotation(0, 0, 0, a)`
  turns about x (RapidQ's `Lights_pyramid.bas` tumbles), a camera whose up
  is (0, 0, 0) is the one with up (1, 0, 0) (`RapidQ_D3D.inc`'s QD3DCAMERA:
  `3DPong` sees its court as RapidQ did); colours past 1 held to 1 (RapidQ's
  `Lights_terrain` sky); **texture filtering** only for
  D3DRMTEXTURE_LINEAR (the mipmap qualities draw nearest); and **specular highlights** from
  a `.X` material's power and specular colour (RapidQ's `myearth.x`).
- **The `.X` models**: all 55 of the corpus (42 different; text and binary)
  draw as RapidQ's D3DRM draws them — a gallery beside RC.EXE's captures.
- **A type nothing defines is RC.EXE's error**, in its words for each place
  (`Unknown data type QBITMAPEX`, `Unknown type FOOBAR`, `… is not a valid
  data type for your FUNCTION`, `Create Method only works for QObjects, not
  …`, `You can only extend QObjects`); RapidR accepted any name.
- **An object's name as a type**: `DIM x AS Lst` after `DIM Lst AS
  QSTRINGLIST` makes another QSTRINGLIST, `SUB S(p AS Lst)` takes one (RapidQ's
  `Choosecolor.bas`: `Sender AS BUTTON`).
- **`$DEFINE` matches in any case**, as RC.EXE (`$DEFINE GLint integer`, then
  `AS glInt`; RAPIDQ2.INC's BOOLEAN), but leaves a member's name after a
  `.`, an object TYPE's fields and a CREATE block's properties alone
  (RAPIDQ2.INC's TRANSPARENT and its `Transparent` field).
- **A QDXSCREEN is set up on its shown window** (OnInitialize,
  OnInitializeSurface once the form's window shows, before OnShow): RapidQ's
  3DPong runs its game loop inside OnInitializeSurface, and its native
  build never showed a window.
- **`WITH TF.Bar` in a TYPE's own code** reaches the field's object (it set
  the form's Width: RapidQ's `newform.bas` shrank to nothing).
- A routine named after its object (`FUNCTION Screen.GetPixelDepth` in
  RAPIDQ2.INC) counts as called, so the Windows API it calls is reported when
  the program compiles, not when it runs.
- Of the corpus's 32 DirectX programs (19 compiled before this work), RapidR
  compiles and runs the 26 RC.EXE compiles that call no Windows API; the
  rest don't compile in RapidQ either (two include files the corpus lacks,
  a syntax error, QBITMAPEX) or call the Windows API (RAPIDQ2.INC's
  `SetLastError`, `GetDC`) — docs/directx-plan.md, "The corpus's DirectX
  programs", has the before / after table, RapidR's captures (interpreted,
  native, the web) beside RapidQ's.

### Legal
- **A review of RapidQ's terms, rights and trademarks, and of everything
  RapidR takes from RapidQ** (`docs/legal/rapidq-review.md`): RapidQ's
  freeware terms (free use, programs may be sold; only selling RapidQ itself
  is forbidden), the rights sold to REAL Software (now Xojo, Inc.) in 2000,
  trademark searches (no RapidQ or RapidR mark for software anywhere
  searched), the law on re-implementing a language and its interface, and
  the use of RC.EXE as a black box for testing. Every blob of the git
  history was compared with RapidQ's distribution and manual: no file of it
  was ever committed.
- The built-in `RAPIDQ.INC` constants (what `$INCLUDE "RAPIDQ.INC"` gives
  without the file) are regrouped by public origin — Windows SDK numbers,
  Delphi VCL types, RapidQ's own — in RapidR's own order and words; the
  same 483 names and values, pinned by a test. Programs see no change.
- What still came from RapidQ material is rewritten: a 9-line routine from
  one of RapidQ's examples in a conformance case, the manual's example data
  in three test fixtures and a unit test, and manual quotes in comments and
  docs (paraphrased). RC.EXE's output of RapidQ's own example programs moved
  out of the repository (`.reference/rapidq_golden/`, `RAPIDQ_GOLDEN`).
- `CONTRIBUTING.md` (no RapidQ code, text, includes, examples or media; what
  may be used for compatibility), `NOTICE` (shipped in every package),
  `docs/legal/clean-room.md` (how RapidR is developed independently), and a
  clearer non-affiliation and trademark statement in `LEGAL.md`, the README
  and the release notes.

### Added
- **Windows DLL calls on Windows** ([docs/windows-dll-calls.md](docs/windows-dll-calls.md)):
  a RapidQ program that declares routines of Windows' DLLs (`DECLARE
  FUNCTION GetDC LIB "user32" …`) calls them when it runs on Windows, in
  native builds and in the interpreter, with RapidQ's rules checked
  against RC.EXE (numbers by value unless BYREF, strings and TYPEs by
  address, what the DLL writes seen in the variable); on macOS, Linux and
  the web the call is a run-time error that names the function, says it
  needs Windows and gives the portable alternative where there is one.
  Such programs compile everywhere (the compiler refused them before).
  Up to 16 arguments, of which up to 8 DOUBLE / SINGLE (GDI+, OpenGL);
  a 64-bit pointer a DLL returns or writes into a BYREF LONG becomes a
  32-bit stand-in that turns back into the pointer when handed to a DLL;
  a 32-bit DLL (most DLLs shipped with RapidQ's examples) is named as such
  instead of failing to load; x86 machine code run through CallWindowProc
  and SUBs handed to a DLL as callbacks are clear errors. Checked on the
  Windows 11 VM with the 175 corpus programs that call DLLs, interpreted
  and native, and user32 / kernel32 / gdi32 programs side by side with
  RC.EXE's builds (the same volume serial, short path names, window spy
  data, cursors; docs/windows-dll-calls.md §6, which lists what still
  differs).
- RapidQ's **SENDMESSAGE, POSTMESSAGE and KILLMESSAGE** are user32's
  SendMessage / PostMessage / PeekMessage(PM_REMOVE) on Windows (11 more
  corpus programs compile), the same "Windows only" error elsewhere.
- `RAPIDR_SANDBOX`: a run with it set loads and calls no DLL (the gate a
  run started for someone else sets; SEC-19 reviewed in
  docs/security-audit.md §6).
- **One address space**: around a DLL call the program's memory (VARPTR
  addresses, TYPEs, streams, the strings inside structures) is real at
  its own addresses on Windows, so an API fills the program's buffer.
- **`Form.Handle` is the window's HWND** on Windows once the form is shown
  (SetWindowPos, SetForegroundWindow, GetDC on a form work).
- **PEEK / POKE / PCOPY** as RapidQ's (the console's pages, checked on
  screen with RC.EXE) plus RapidR's own: PEEK / POKE on the program's
  memory through VARPTR, on every runtime including the web, with a clear
  error for any other address. INP / OUT are a run-time error, as on every
  Windows since 2000.
- **The streams' last RapidQ members**, each checked against RC.EXE:
  QMEMORYSTREAM's `MemCopyFrom` / `MemCopyTo` (bytes between the stream
  at Position and an address of the program's: VARPTR of a variable, an
  element, a TYPE, a stream's Pointer — bounds-checked, on every runtime)
  and `SetSize` (Size, only written); `SaveUDTArray` / `LoadUDTArray` on
  both streams (a TYPE's array field, laid out as RapidQ does);
  QFILESTREAM's `ReadByte` / `WriteByte` (26, DOS's end-of-file mark, past
  the end, as RapidQ). `CopyFrom`, `LoadArray` and `SaveArray` were
  already there; the registry and the manual no longer call them missing.
- A program that DECLAREs `RtlMoveMemory` itself (`BYVAL Dest AS LONG …`)
  calls that DLL routine as declared — on Windows the copy happens, elsewhere
  the "Windows only" error — instead of RapidR's own RTLMOVEMEMORY, which
  copied between the wrong variables without a word.
- `&hHE` is 14, as RC.EXE reads it (the non-hex letters dropped).
- **Every build is an app for its system, with an icon** (`rapidr build`,
  `crates/rapidr-package`, docs/manual/building-apps.md). A program with
  windows becomes `Name.app` on macOS (Info.plist with its name, bundle ID,
  version, `NSHighResolutionCapable`, `LSMinimumSystemVersion`; its icon as
  `.icns` from 16 to 1024 px; signed ad hoc so it opens on Apple silicon;
  an interpreted program's bytecode in `Contents/Resources`), an `.exe`
  with its icon (16 to 256 px) and version information on Windows, and
  `Name.AppDir` on Linux (desktop entry, hicolor icons 16 to 512 px;
  `rapidr install-app` puts it in the applications menu). Console programs
  stay plain executables (`--bundle` makes apps of them; `--no-bundle`
  keeps any program a plain executable). Native and interpreted builds
  alike, cross builds too: the resources are written in pure Rust (editpe,
  BSD-2-Clause), so a Mac makes a Windows `.exe`'s icon.
- **The icon**: `--icon`, the project's `[build] icon` (`.rrproj`, with
  `app_name`, `bundle_id`, `version`, `company`), `$OPTION ICON`, RC.EXE's
  `-g<icon>` — in that order — as `.icns`, `.ico`, `.png` or `.svg`, made
  into every size each system wants (clear errors for files that aren't
  pictures, a note for small ones). Without one, RapidR's new **program
  icon** (`design/brand`: the Runtime's Ink tile, an app window, the amber
  Run triangle; `RapidR-App.icns`, `rapidr-app.ico`, `apps/rapidr-app`).
  `rapidr build app.rrproj` builds a project's main file with its settings.
- **RapidR Studio: Run > Build makes the app** for the system Studio runs on
  (RPROJECT.Build: `rapidr build` in the background, its lines in Output),
  **Run > Reveal in Finder / File Explorer / Files**, and **Project >
  Project Options**: the app's name, bundle ID, version, company, icon (with
  a preview) and native or interpreted build, saved in the project
  (`ide/build.inc`; RPROJECT's Icon, AppName, BundleID, Version, Company,
  BuildKind, Building, BuiltPath, FileManager, Build, StopBuild, Reveal,
  IconPreview, OnBuildOutput, OnBuildDone).
- `tools/studio_app.sh`: RapidR Studio as `RapidR Studio.app` from a
  checkout (its icon in the Dock, this checkout's Studio).
- **Every RapidQ member now works in RapidR.** The 101 RapidQ properties,
  methods and events that RapidR didn't answer before now work in native
  builds, the interpreter and the web. Each was checked against RapidQ's own
  compiler, RC.EXE:
  - **Streams.** QFILESTREAM has ReadByte and WriteByte. ReadByte gives 26
    past the end, as RapidQ does. QMEMORYSTREAM has `SetSize = n`,
    MemCopyFrom and MemCopyTo (with VARPTR or Pointer addresses). CopyFrom
    reports a stream read error when it asks for more bytes than are left.
  - **String lists and rich edits.** QSTRINGLIST.LoadFromStream splits lines
    at CR LF, LF or CR. Its SaveToStream reads the list from the stream,
    exactly as RapidQ's does. QRICHEDIT has LoadFromStream and SaveToStream.
    QSTRINGGRID has DeleteColumn, another name for DeleteCol.
  - **Drawing.** TextRect draws text clipped to a rectangle on forms, images,
    canvases, bitmaps, headers, the printer, and owner-drawn lists and grids.
    Rotate turns an image, canvas or bitmap by degrees. QCANVAS has Get and
    Put, QIMAGELIST has Draw, and QIMAGE has Repaint. Lists, combo boxes and
    grids take RoundRect, CopyRect, StretchDraw, TextOut, TextWidth and
    TextHeight in their owner drawing.
  - **Forms.** HideTitleBar and ShowTitleBar remove and restore the title
    bar. ShapeForm cuts a window to a bitmap's outline (not on Wayland). A
    QFORM has Cascade, Tile, Next, Previous and ArrangeIcons. QFORMMDI has
    ActiveNextChild and ActivePreviousChild.
  - **Events that never fired now do.** OnHint fires when the mouse brings a
    new hint, and RapidR now shows tooltips (ShowHint, HintPause,
    HintColor). OnEnter fires when a list box or file list box gets the
    focus. OnStartDrag and OnEndDrag fire around a button drag, and
    StartDrag moves the control with the mouse. QCOMPORT fires OnBreak,
    OnRing and OnTxEmpty. A control's WndProc can be bound but is never
    called, as in RapidQ. A form's WndProc still gets its tray icon's
    messages.
  - **QMYSQL.** RealConnect, CreateDB, DropDB, Refresh, FetchLengths (with
    Length), RowBlob, LoadBlob and SaveBlob work. An empty host means this
    machine, as in RapidQ. EscapeString escapes the way MySQL's C client
    does.
  - New conformance cases (most with RC.EXE's own output as the expected
    output) and new GUI cases for the desktop and the web.
- **RapidR Studio: the IDE's shell** (`ide/`, docs/ide-plan.md I1 /
  L-SHELL + L-WEB). One RapidR program on RapidR's public components — the
  same bytecode on the desktop (`rapidr ide [file]`) and in the browser
  (`tools/build_studio_web.sh` → `target/studio-web`, drawn by the UI
  kernel on a canvas); their captures are byte-identical and their
  accessibility trees equal (`tests/studio_shell.mjs`: two scenes, four
  themes, 1× and 2×).
  - **The window:** a menu bar, a tool bar of RapidR's icons and the
    command palette (Ctrl+Shift+P), all made from one command table (VB6 /
    Visual Studio keys: F5 Run, Ctrl+F5, Shift+F5 Stop, F9, F8 / F10 /
    F11, Ctrl+S, Ctrl+O, Ctrl+N); RDOCKMANAGER with Project / Toolbox,
    Properties / Outline and Output / Problems / Immediate around an MDI
    documents area (tabs one click away); a status bar that turns green
    while the program runs and orange while it's paused.
  - **What it does:** opens a project, a source file with what it
    includes, or a folder; makes new projects; saves files and the
    `.rrproj`; runs and stops the program — in its own process on the
    desktop (its forms real windows), in a sandboxed frame on the web — with
    its output in the Output pane; shows each file's outline and the
    compiler's problems as you type; the Immediate window evaluates in a
    paused program; theme switch, MDI or tabs, the dock's layout and recent
    projects kept per user; `--do` runs commands (`rapidr ide --do
    run.start file.rr`).
  - **On the web:** the File System Access API for files and folders (saved
    back to the user's disk), Studio's own files kept in the browser's
    private file system between visits, and F5 / F6 / Ctrl+Tab as the
    IDE's keys.
  - **RapidR's look** is Studio's default — as the system is: light, dark
    or high contrast (`Application.Theme = "rapidr"`, `"rapidr light"`,
    `"rapidr dark"`, `"rapidr high contrast"`); classic stays one click
    away.
- **RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION** — RapidR Studio's
  services as public non-visual components (`crates/rapidr-studio`), the
  same on every runtime: a project (`.rrproj` v2 or v1, a source file and
  its includes, a folder), the language service's outline and diagnostics,
  and a run of a program under development (start, stop, pause, step,
  breakpoints, evaluate; OnOutput, OnStopped, OnExit …).
- **Tooltips**: a component's `Hint` shows in a small box when its
  `ShowHint` is True — after half a second, under the mouse or under a
  component the keyboard reached — as in RapidQ; drawn by the UI kernel on
  every host.
- **Inter and JetBrains Mono** built in (SIL OFL, Latin subsets): RapidR's
  UI and code faces — the RapidR look's menus, MDI titles, tooltips and
  code editor; programs can name them.
- `Application.ThemeColor(Name)`: a colour of the current theme by name —
  its own tokens and an IDE's (tool bar, status bar per run state, start
  page, editor colours).
- `QOPENDIALOG.PickFolder` chooses a folder; `QSTATUSBAR.Color`;
  `QTREEVIEW.ItemHeight` (QOUTLINE's) sets the rows' height;
  `RDOCKMANAGER.DocumentState`; a tab in a menu caption shows key text
  without binding it (Windows' menus' rule).
- **RapidR Studio's form designer model (I4, L-DMODEL), and anchoring that
  is the running program's.** The designer reads a program's
  `CREATE … END CREATE` blocks into a component tree (parents, z-order,
  Tab order), changes it with undoable commands (add, delete, move, resize,
  align, distribute, same size, bring to front / send to back, Tab order,
  cut / copy / paste, duplicate, rename, anchors, any property from the
  inspector), and writes each change back as the smallest text edit:
  a property's value, a new line in the block's own indentation, a nested
  CREATE before its parent's `END CREATE` — comments, blank lines, other
  code and everything outside the blocks stay byte for byte, and Undo
  gives the exact bytes back. Every form of `examples/` and of RapidQ's 386
  examples opens and saves unchanged; moving a component 8 pixels changes
  exactly one value. Where components are comes from the runtimes' own
  layout (Align, Anchors, Constraints, a QLABEL's AutoSize, scroll bars,
  the default sizes): dragging the designed form's corner previews it at
  another size with every component where the running program puts it —
  checked against the program itself resized, native, interpreted and in
  the browser (`tests/fixtures/designer_anchors.bas`).
- **RDESIGNSURFACE on the designer model**: Shift / Ctrl+click and a rubber
  band select several components; drags snap to the grid and to smart
  guides (siblings' edges, centres and text baselines, the form's centre
  lines, 8-pixel margins, equal spacing — Alt for none); eight handles; round
  anchor pins on the selected component's sides (click to anchor that
  side); the form's corner drags the resize preview; moves and resizes are
  one undo step each. New members: `Undo`, `Redo`, `AlignSelection(How)`,
  `SelectAdd(Index)`, `SelCount`, `PreviewWidth` / `PreviewHeight`,
  `ShowGuides`, `SnapToGrid`, `GridSize`. Its chrome follows the theme.
  `examples/studio/form_designer.rr` shows it. Existing programs' calls answer as
  before.

- **An RPLOT on a form shows its chart, the same on the desktop and the
  web.** Put an RPLOT in a form's CREATE block and the chart is drawn in
  its place, filling its Left / Top / Width / Height, and follows Align
  and Anchors as the form is resized. It is sharp at any screen scale
  (1×, 1.5×, 2×) and drawn in the program's theme (classic, modern,
  dark). It is drawn again whenever the chart changes: a new series
  (`Plot`, `Bar` …), a property (`Title`, `Grid` …), `Render` / `Show`.
  Native builds, interpreted programs and the browser show the same
  pixels, and a screen reader hears an image named by the chart's Title.
  On the web the chart is no longer a separate page element: it is drawn
  on the form like every other component (the old element is gone). A
  chart that is never placed on a form works as before (`SaveFig`,
  `Image.LoadFromPlot`). A new RPLOT is 640 × 480, the chart's own size,
  everywhere.
- **Automatic keyword case in VS Code, as in QuickBASIC and VB.** Type
  `dim x as integer` and it becomes `DIM x AS INTEGER` as you go: each
  word is put in BASIC's case when you finish it (space, Enter, Tab, `(`,
  `)`, `,`, `:`, `=` or an operator) — keywords and statements, type
  names, directives (`$INCLUDE`) and builtins (`MID$`). Strings, comments,
  `$INCLUDE` paths and your own names are never touched, and one Undo
  gives back what you typed. Format Document and Format Selection apply
  the same rules to the whole text. Two settings:
  - `rapidr.keywordCase`: `upper` (the default), `lower`, `proper`
    (`Dim x As Integer`) or `preserve` (off);
  - `rapidr.identifierCase`: `declaration` writes every use of your
    variables, SUBs, FUNCTIONs and constants as their declaration does,
    and a component's members as RapidR spells them (`Form.Caption`), as
    VB did; `preserve` (the default) leaves them as typed.

  BASIC ignores case, so a program never changes: a test formats every
  conformance program and example and checks that it compiles to the same
  code, its strings and comments byte for byte. Format on type is now on
  by default for RapidR files (`editor.formatOnType`). RapidR Studio's
  editor will use the same rules.
- **The language registry** (`crates/rapidr-lang`, RapidR Studio I0): one
  description of the language (every component, property, method, event,
  builtin, statement and constant, with types, defaults, signatures and
  short docs in RapidR's own words), compiled from `data/*.toml`. The
  compilers' component list, the runtimes' name tests, the editor's
  keyword groups, the icon tools, the manual's reference pages and the
  web IDE's completion data all come from it; `rapidr lang export` writes
  them, `tools/lang_dispatch.py --check` proves every name the runtimes
  answer is in it, and `tests/lang_conformance.mjs` runs one program per
  component on every runtime. RDOCKMANAGER and the icon methods
  (`Bitmap.LoadIcon`, `ImageList.AddIcon`) are in it.
- **The user manual** (`docs/manual/`): getting started, the language,
  components (RapidQ's Q names and RapidR's R names), the CLI and the
  Runtime, the web, databases, data science, DirectX and media,
  differences from RapidQ, troubleshooting. Its reference pages (components,
  builtins, members, statements, constants, data-science members) are
  generated from the language registry by `rapidr lang export --manual`
  (`cargo test -p rapidr-lang` fails when they're stale).
  Every SDK installs it in `share/doc/rapidr/manual/`.
- The release notes of v2.117.0, the first public release
  (`docs/release-notes/v2.117.0.md`).
- Conformance cases `datascience_num`, `datascience_frame` and
  `datascience_plot`: every documented RNUM, RDATAFRAME and RPLOT member,
  checked on the VM, native builds and the web (`tests/web_conformance.mjs`).
- Data-science members the desktop or the web lacked now work everywhere:
  RNUM `create`, `set`, `get`, `push`, `avg`, and `Sum` / `Mean` / `Min` /
  `Max` / `Std` / `Count` as properties; RDATAFRAME `create`, `addrow`,
  `savetocsv`, `loadfromjson`, `savetojson`, `iloc`, `sort_values`,
  `query`, `groupby`, `value_counts`, `nunique`, `corr`, `nlargest`,
  `nsmallest`, `dtypes`, `merge`, `concat`, `apply`, `replace` on the web;
  RPLOT `addseries`, `settitle` / `setxlabel` / `setylabel`, `show` /
  `render` on the desktop, and `xscale` / `yscale`, `xlim` / `ylim`, `DPI`
  on the web. `LoadFromCSV` / `LoadFromJSON` also take the data itself;
  `filter` also takes `=`, `<>`, `startswith`, `endswith`, and `contains`
  is a substring test on every runtime; `groupby` also takes `median` and
  `std`; `cell` / `setcell` also take a column name.
- **The examples, curated for the first release** (`examples/`, indexed by
  `examples/README.md`): 26 small commented programs by topic — basics/
  (hello, INPUT, files, the language), gui/ (a form and its events, menus,
  dialogs, a timer, a list and a grid, themes, the tray icon), graphics/
  (canvas), directx/ (QDXSCREEN sprites, a Direct3D cube from a `.X`
  model), media/ (QMIDI with the built-in synthesizer, QWAVE, QVIDEO), data/
  (SQLite with parameters, JSON, RNum, RDataFrame and RPlot), network/
  (RHTTP + RJSON, QDOWNLOAD), rapidq/ (a notepad in plain RapidQ code),
  web/ (browser storage, RJAVASCRIPT and RROUTER) — plus the IDE. All of
  them RapidR's own; their media and models are made by
  `tools/make_example_media.py` (nothing downloaded). The old test scraps
  and demos went (and the scripts that drove them: `tools/native_examples.sh`,
  `tests/full_matrix.sh`, `tests/bc_smoke.sh`, `tests/web_matrix.mjs` and the
  demos' ad-hoc page scripts); the web IDE's examples list shows the new ones,
  with the files their `$RESOURCE` lines name.
- `tests/examples_run.mjs` (the `examples` stage of `tools/regress.sh`, in
  place of `tools/native_examples.sh`'s `cargo check`): every example RUNS on
  every runtime it claims — `rapidr run`, interpreted and native executables
  (each native build dropped after its run), the web (the UI kernel's page)
  — GUI ones through the test hooks (events, then the components' properties
  read back), network ones against the tests' own local server; the table
  must list every program and the README every entry.
- `rapidr examples` lists the examples and `rapidr examples copy <name|all>
  [folder]` copies one (with the data files it names) or all of them; SDK
  installs ship them in `lib/rapidr/examples/` (the home's, as a checkout's
  `examples/`: `tools/release/stage.py`).

- **RapidR's own icon set** (IDE plan decision D8; `design/icons/`). There
  are 344 icons, our own drawings (MIT), each drawn once on a 24 px grid and
  hinted at 16, 24 and 32 px:
  - every action and command of RapidR Studio;
  - every component type (QBUTTON and RBUTTON share one), the IDE plan's
    planned components and RPLOT's chart kinds;
  - file types (the brand's R on `.rr`, `.rrbc` and `.rrproj`), symbol
    kinds, toolbox groups and glyphs.

  Monochrome action icons follow the theme; component icons are two-tone,
  in colour tokens for the classic, modern, dark and high-contrast themes
  (3:1 or better everywhere, monochrome in high contrast). The design system
  is in `design/icons/README.md`.
  - The new crate `rapidr-icons` themes and renders the icons through
    resvg, picks the drawing hinted for the device's size (crisp at 1×,
    1.5× and 2×) and builds for wasm32. Its SVGs are stored deflated
    (42 KB) and inflated on first use.
  - The UI kernel draws an icon with `Painter::icon`.
  - New in programs (additions; RapidR's own): `Bitmap.LoadIcon(Name$
    [, Size [, Theme$]])` and `ImageList.AddIcon(Name$ [, Theme$])`, by
    icon name (`"run"`), component type (`"QBUTTON"`) or command id
    (`"file.open"`).
  - `design/icons/inventory.toml` lists what has an icon. Components come
    from `COMPONENT_TYPES`; the toolbox groups keep RapidQ's components
    under "RapidQ" and the rest under "RapidR". The build and the crate's
    tests fail when a component type or a command has no icon.
  - The manual's component reference shows each component's icon, and
    `docs/manual/icons/` holds every icon as SVG (light and dark), PNG and
    an HTML catalog.
  - `tools/regress.sh unit` checks that everything is up to date.

### Changed
- **`$OPTION ICON` as RapidQ's compiler has it** (checked against RC.EXE):
  an icon file that isn't there is a compile error, `ICON file x does not
  exist.` (it used to leave the default icon); the file name may be
  unquoted; with several, the last one wins. The icon is now also the built
  executable's / app's. RapidR still takes any `.ico` (RapidQ only 766-byte
  32 × 32 ones), `.icns`, `.png` and `.svg`.
- QDXSCREEN.TextRect now uses the same drawing as every other TextRect: a
  background colour fills the whole rectangle, not just the text.
- `SetFocus` on the desktop now moves the keyboard focus, as it already did
  on the web.
- **`rapidr ide` opens RapidR Studio** (`ide/studio.rr`; an install's
  `ide/rapidr-ide.rrbc` is compiled from it); the old `examples/ide.rr`
  and the HTML web IDE stay until Studio reaches their features
  (docs/ide-plan.md I1's acceptance).
- **RapidR's look (modern, dark, high contrast) draws its chrome in its own
  fonts and shapes**: menus, MDI titles and tooltips in Inter; the code
  editor in JetBrains Mono on the theme's editor colours (keywords readable
  in the dark look); MDI windows rounded with a light title bar; the dock's
  headers 28 pixels with their buttons where the mouse or the focus is;
  flat tool buttons a soft rounded fill under the mouse; the menu bar's
  access keys underlined from the keyboard only. The classic look and every
  program's own metrics are unchanged.
- **The dialogs the UI kernel draws** (message and input boxes, the colour
  and font dialogs) use the look's chrome font: Inter in RapidR's look,
  MS Sans Serif in the classic one (unchanged). RapidR Studio's own
  dialogs follow it, with fields and buttons sized for it.
- RDOCKMANAGER's headers are 28 pixels in every look (a pane's inside is
  2 pixels shorter than before).
- **On a Mac, a menu's `Ctrl+` ShortCut is ⌘** (the system menu bar's
  key equivalent; in-window menus take either), as Mac programs' keys are.
- A flat button whose only glyph is a picture (no disabled frame) is drawn
  greyed while disabled, as Windows greys a speed button's glyph.
- RCODEEDITOR reads a UTF-8 source as UTF-8 (`LoadFromFile`) and writes it
  back so; other files stay a byte a character, as RapidQ's text boxes.
- **One layout sequence for every runtime.** When Align and Anchors place
  components (a property set, a container resized) is
  `rapidr_value::layout::engine`'s, run by the desktop and web runtimes
  and by the designer; nothing a program sees changes.
- The language registry's component sizes are the runtimes' (RapidQ's
  measured defaults); a test keeps them the same.
- **The web answers every member the desktop does, and members RapidQ
  doesn't have are refused as RapidQ refuses them.**
  - QSTATUSBAR `Clear` removes every panel (panels added afterwards start
    again at `Panel(0)`, as RapidQ does); QMEMORYSTREAM `Clear` empties the
    stream (`Size` and `Position` 0, as RapidQ). Both did nothing before,
    and the browser didn't know them.
  - QIMAGE `Clear` / `Cls` remove the picture and `Load` loads one, as
    `LoadFromFile` does, in the browser too.
  - RapidQ's drawing methods on lists and grids draw what RapidQ draws
    (checked against programs built by RapidQ's own compiler): `Paint`
    flood-fills in an owner-drawn list box's, combo box's or grid's
    handler; `Line`, `FillRect`, `Circle`, `Paint` and the rest on a list
    box that isn't owner-drawn draw on the list, which keeps the drawing
    until it paints those rows again; on a combo box that isn't
    owner-drawn nothing shows. Pixel for pixel the same in the browser and
    on the desktop, at normal and high-DPI scale.
  - `Click`, `SetParent`, and other names RapidR's desktop runtime used to
    accept on any component (`AddItem` on a list view or popup menu,
    `AddItems` on an edit, `Rect`, `SetPixel`, `Ellipse`, `DrawText` on a
    list, grid, bitmap or DirectX screen, `Clear` on a bitmap …) are now
    the compile error RapidQ gives: "Member CLICK not part of class BTN".
    They did nothing, or nothing useful, before.
  - RSERVERSOCKET stays desktop only: a web page can't listen for network
    connections.

- **The VS Code extension and RapidR Studio know the language from the
  language registry.** Completion, hover, signature help and the
  compatibility warnings come from the one description of the language
  RapidR's compilers and IDE use (the language service's own copy of it is
  gone). What you see:
  - hovers give a property's type, whether it is read only and RapidQ's
    default, the include file a RapidQ name comes from, and say when
    something is a RapidR extension, works on the desktop or the web only,
    or isn't implemented in RapidR yet;
  - `Screen.`, `Application.`, `Printer.` … complete and hover, and so do
    the objects an indexed property gives (`Tree.Item(0).`);
  - what RapidR doesn't have yet isn't offered in completion, and where a
    program uses it there is a warning ("RapidQ's QForm.ShapeForm is not
    implemented in RapidR yet"); a member or builtin only one runtime has
    gets a note;
  - in a RapidQ-compatible project (`rapidr.rapidqCompatible`), every
    RapidR extension is reported, not only components: members of RapidQ's
    components (`QForm.Anchors`), builtins, statements (`OPEN`,
    `LINE INPUT`, `PRINT #`), the `$THEME` directive, types (`INT64`) and
    RapidR's constants — "… is a RapidR extension: RapidQ's compiler
    refuses it";
  - signature help for builtins shows their syntax with the parameter you
    are typing highlighted, also for statements written without
    parentheses (`LOCATE 1, 2`).
- **RNUM, RDATAFRAME and RPLOT are one implementation for every runtime**
  (`rapidr_value::datascience`, docs/ide-plan.md decision D7): native
  builds, interpreted programs and the browser run the same arrays, frames
  (with their CSV and JSON readers and writers) and chart model; a runtime
  adds only PRINT and filling a QSTRINGGRID. Charts are drawn by the UI
  kernel, pixel-identical on the desktop and the web and crisp at every
  screen scale (category axes, integer ticks); frames are a columnar
  engine of RapidR's own (a million rows: load 114 ms, sort 182 ms, group
  80 ms, join 60 ms). polars, ndarray and plotters are no longer
  dependencies — 89 fewer crates in the tree, smaller native builds — and
  THIRD_PARTY_NOTICES.md is regenerated. Where the two implementations
  disagreed, one behaviour was chosen: `setcell(row, col, value)` (the
  web took the column first), `randint` includes both ends, `reciprocal`
  of 0 is INF, a missing array reads as empty, `Shape` of an RNUM is
  `(3,)`, `groupby` puts the group column first and orders groups by key,
  `describe` summarises the numeric columns (count, mean, std, min,
  quartiles, max), `transpose` names its first column `column`, charts
  default to 640 × 480 px without a grid, and computed numbers print as
  RapidR prints numbers (`0.3`, not `0.30000000000000004`).
  `examples/web_datascience.rr` and the web IDE's hover help follow.
- **RPLOT's charts look the same, and better, on the desktop and the
  web**: the axes' ranges, bar widths and legend place come from the shared
  model (`Plot::ranges`, `nice_ticks`, `legend_corner`) — round 1-2-5
  ticks without `.0` on whole numbers, a light grid, 2-pixel lines, real
  dashed lines (`--`), bars wholly inside the axes standing on 0, a line
  chart fitting its data, the legend framed in a corner free of data (the
  y axis reaching higher when none is, as Matplotlib's headroom), pies from twelve o'clock with labels outside and white between
  slices. Desktop charts are drawn twice as fine and averaged down (smooth
  edges), with text at the web's sizes.
- **Charts are sharp on high-DPI screens**: `Image.LoadFromPlot` keeps
  what draws the chart again at the screen's scale (`Bitmap::set_redraw`,
  as an SVG picture is), and the web's chart canvas draws at the page's
  scale; the pixels a program reads stay the 1× ones.
- `ToGrid` makes the header the grid's fixed row and no column fixed, so
  every column of the frame shows as data.
- **Web: Open and Save use the user's real files**, through the browser's
  own pickers, as the desktop uses the system's dialogs; the in-page "Save
  As" list of the page's files with its Upload… button is gone.
  QOPENDIALOG / QFILEDIALOG open the system's Open dialog
  (`showOpenFilePicker` in Chrome / Edge, a file input in Firefox / Safari)
  and read the files picked before `Execute` returns; QSAVEDIALOG opens the
  system's Save dialog in Chrome / Edge (`showSaveFilePicker`, FileName and
  DefaultExt proposed). The program reads and writes them with its ordinary
  file I/O: a name a dialog answered is the real file's (its writes go to
  it), any other name stays in the browser's store. Firefox and Safari have
  no save dialog: RapidR asks for the name in a box of its own, and the
  program's writes to it are a download. `Filter` is the pickers' file
  types (the FilterIndex one first; "All files" only when the Filter has
  it — and a file input, which takes one group only, lets every file
  through then, so none is greyed out); Cancel returns 0. Execute with no
  user gesture left (from a timer) shows a small box whose button opens the
  picker. In the web IDE the IDE shows the pickers for the program's
  sandboxed frame and writes back only to the files picked during that run.
  docs/manual/web.md says what each browser does.
- **Web IDE: the run window** follows the IDE's theme (no more Windows-blue
  title bar around the program's own windows, which draw their frames in
  the program's theme), and is big enough for the program's windows and
  dialogs, not only the startup form's design size.
- INPUT's box and the host's prompts select their proposed text, so typing
  replaces it, as Windows' boxes do.
- README rewritten for newcomers (install from the releases first, a quick
  start, what's in it, the platforms checked); COMPILER_MANUAL.md is now
  the contributor manual for today's architecture, and its outdated PDF is
  gone.
- The git history no longer holds the earlier versions of QDockForm and
  QDirListView (close ports of user-contributed RapidQ code) or of the four
  conformance cases and the `tab_control` fixture that reused the manual's
  examples: every commit carries the clean versions instead (same commits,
  authors and messages; today's tree unchanged). Clones made before
  2026-10-06 should be cloned again.

### Fixed
- **RapidQ's examples at run time** (`tools/corpus_run.py`; its report stays
  on this machine, what it found in `tools/corpus_run_notes.md`): every example that compiles is run on the
  interpreter and as a native build, GUI ones captured beside RC.EXE's
  windows, and what went wrong fixed as RapidQ does it (each checked with
  RC.EXE): `SLEEP .1`; a program's own `TYPE QToolBar`; `Font.AddStyles` /
  `DelStyles` on components and on a QBITMAP's font (`= n` too); a form's
  first Show fires OnResize, OnShow, OnResize; QTIMER on Windows' ticks;
  RLE4 / RLE8 / 16-bit BMPs; aligned controls placed in RapidQ's order at
  the first Show and inside a panel's bevels; status panels 50 wide; a
  QBUTTON's glyph; `ImageList.Handle =`; canvases made in OnShow paint;
  QSTRINGGRID's FixedColor and owner drawing; a QFILESTREAM that can't open
  its file stops the program (`Cannot open file x.`); a borderless form
  never shows scroll bars; QFILELISTBOX's order and `[.]`; hex numbers with
  `?` / `@` digits (`&HFFFF0000???` in RapidQ's CommCtrl.inc), `&HH1`, and
  their low 32 bits past 8 digits; QFONTDIALOG's colours (clWindowText at
  first, system colours kept). RapidQ IDE's `.rqw` window programs are part
  of the corpus now.
- **RapidQ's console box characters**: a RapidQ program's console shows characters 128 to 255
  in the DOS code page 437, as RC.EXE's did — `CHR$(201)` ╔, the byte 218 ┌ — so `3DBOX` and
  `BATTLE` draw their boxes again instead of É, Ú, ³. A RapidQ program is a file that isn't
  UTF-8, or a `.bas` / `.rqb` / `.rq` / `.rqw` file of plain ASCII; RapidR's own programs (`.rr`,
  UTF-8 with accented letters) print Unicode as before. Interpreter, native builds and the web
  (`rapidq_console_cp437`, RC.EXE's bytes).
- **A form's first OnPaint comes when the program next waits** (DOEVENTS, ShowModal), after
  `OnResize`, `OnShow`, `OnResize` — not inside `Show` — as RC.EXE sends it; a scale change no
  longer paints twice; the web's DOEVENTS delivers the paints too (`rapidq_form_show_events`).
- **RapidR Studio's panel components read true as 1** (`ShowTabs`, `ShowFiles`, `ShowEvents`,
  `ShowCaptions` …: they read -1, against the language registry and RapidQ's rule) and
  RTOOLBAR reads its Anchors (akLeft + akTop) like the other visual components.
- **RapidQ program forms RC.EXE takes, found running the corpus' `.rqb` / `.rq` programs**
  (each checked with RC.EXE; conformance cases `rapidq_dotted_routines`,
  `rapidq_const_forms`, `rapidq_input_bare`): a FUNCTION named with a dot returns what's
  assigned to its whole name (`FUNCTION Calc.Twice` … `Calc.Twice = N * 2`; both backends
  returned nothing);
  `&` stuck to decimal digits is the number (`CONST Null=&0`); a CONST may have a dotted
  name (`CONST Application.Path = …`, read back by that name); a statement whose argument
  starts in parentheses (`SLEEP(T * 11.2) / 600`); a bare `INPUT$` reads a line, as RC.EXE
  does (natively it didn't build, interpreted it didn't wait).
- **The font dialog names a system colour by its colour**: given clWindowText (a new
  QFONTDIALOG's Color, and every font that hasn't set one) its colour list showed "Custom"; it
  shows "Black" now (clWindow "White"; one that isn't among the 16, such as clBtnFace, stays
  Custom), the sample drawn in the theme's text colour, and OK keeps Color clWindowText unless
  another colour is picked — so a label given the font back still follows the theme. (RapidQ's
  dialog shows Custom: a deliberate difference.)
- **`QFONTDIALOG.SetFont(Label.Font)` / `GetFont(Label.Font)`** change and
  read the component's own font, and `Label2.Font = Label.Font` copies it
  (an addition: RapidQ's compiler refuses a component's Font there).
- **`Form.Center` centres the window** when the form shows (`Show`,
  `ShowModal`), as RC.EXE does: Left / Top read 0 until then and the
  screen's middle after, by the form's outer Width × Height (RC.EXE and
  RapidR both put a 300 × 200 form at 714, 401 on the VM's screen). Every
  centred form opened in the screen's top-left corner, on every desktop
  (the position was sent before the window existed).
- RapidQ's QRECT given to a DLL (`GetClientRect(hWnd, r AS QRECT)`) is a
  RECT, and what the DLL wrote is in the QRECT after the call (the
  record's name went over as a string); a QNOTIFYICONDATA, whose handles
  are 64 bits in Windows' 64-bit structure, is refused with an error that
  says so.
- A routine defined with a dotted name (`SUB Draw.3DBox`, RapidQ's
  console/3dbox example) is called in native builds as in the interpreter
  and RC.EXE (it went to an object method and printed a warning).
- Windows' console shows what CLS, LOCATE and COLOR print (escape
  sequences were printed raw in a classic console window).
- The web's console fills a coloured cell's whole line (`COLOR , 7` left
  dark gaps between rows).
- RapidR's own Handles (a control's, an icon's) are never a real window's,
  icon's or cursor's: a DLL call given a control's Handle can't reach
  another program's window (the first numbering overlapped the desktop
  window's).
- **Native builds read TRUE / FALSE as the program defines them**: RAPIDQ.INC's `CONST True = 1`
  was ignored by native builds (TRUE stayed -1) while the interpreter and the web took it, so
  `IF Port.Connected = TRUE` failed natively in RapidQ's own ComPort example
  (`const_true_redefined`).
- **Text in RapidQ's default font was cramped, letters running together**
  ("program", "start", "Bread", "Price", "Right-click" in labels, edits,
  grids and status bars; worst on a Retina or 150 % screen and on the web).
  RapidR Sans, the font RapidR draws MS Sans Serif with, had Liberation's
  letters made larger twice over (12 % instead of 6 %) and squeezed into MS
  Sans Serif's bitmap widths (an r is 3 pixels there): "Br", "pr", "ar"
  overlapped by up to a pixel, and 816 of 2,028 letter pairs were closer
  than half a pixel at 8 pt. Now every letter is Liberation's own shape, all
  one size (95 %, never narrowed, no letter smaller than the next), with at
  least 0.8 of a pixel between letters: 47 pairs are closer than half a
  pixel (Liberation Sans itself, Arial's spacing, has 85) — the cross-bars
  of t and f, the points of A, V, w. Readability comes first: r, x, y, j,
  C, the brackets, & and % are a pixel wider than RapidQ's bitmap, so text
  measures a little wider than in RapidQ — "program" 40 pixels (RapidQ 38),
  "start" 21 (20), "Password:" 50 (49), "Right-click" and "Notepad -
  untitled" unchanged, a long sentence about 1–2 % (TextWidth and AutoSize
  report what is drawn); tab stops stay RapidQ's. New gallery cases `hello`,
  `spacing` and `words` with RC.EXE's captures beside them, and
  `tools/visual/words.py` (word by word, old | new | RC.EXE, 1×, 1.5×, 2×).
- **Bold Arial ran together too** (a web window's title, "Pantry"; a
  chart's bold title): its bold is the regular letter made heavier, about a
  pixel wider at 12 px, which ate the space between letters. Each bold
  character now takes half a pixel more at 12 px, as Windows' Arial Bold
  is wider than its regular (TextWidth too; RC.EXE: Arial 9 bold "Pantry"
  37 pixels, RapidR 38, before 36).

- **Every component now starts with RapidQ's values, on every runtime.**
  Reading a property right after creating a component gave nothing for 252
  properties that RapidQ gives a value — a button's Cursor, Kind,
  ModalResult and Spacing, a label's Enabled, a form's KeyPreview, Align
  and ShowHint almost everywhere. They now read what RapidQ's own compiler
  reads, the same in native programs, the interpreter and the web. Checked
  against RapidQ, a few values changed: a new form's Left and Top are 0 (it
  opens at the top left of the screen, as in RapidQ, where RapidR used 100),
  CopyMode reads cmSrcCopy, a QDXSCREEN's AllowStretch is off, and
  true/false properties of lists, trees, list views and scroll boxes read 1
  rather than -1 (so `= True` works with RAPIDQ.INC). A button, gauge,
  scroll box or tab control on a form now reads its form's Color, as in
  RapidQ.
- **Memory and file streams read exactly as RapidQ.** Checked against
  RapidQ's own compiler:
  - `ReadStr(n)` always gives n characters, spaces where the stream has no
    more bytes; a QFILESTREAM's `ReadStr(n)` and `Read(S$)` give one more
    character, a space, as RapidQ's do (`ReadBinStr` doesn't).
  - `ReadLine` removes only the CR right before the LF (others stay); a NUL
    in the line ends its text and moves `Position` to the end, as in
    RapidQ. `LineCount` counts the LFs (a last line without one isn't
    counted).
  - `Position` can be set past the end (and, on a memory stream, before
    the start); reads there get no bytes and leave it where it is; a write
    past the end fills the gap with zeros, one before the start writes
    nothing; a `Size` that leaves `Position` past the new end moves it to
    the old end.
  - `ReadAll` (RapidR's) after the stream's start returned nothing, and
    crashed debug native builds; it now gives the rest of the stream. The
    same fix makes `LoadFromStream` (bitmaps, grids, image lists) read the
    rest of a stream that was already read from.

- The web runtime's `Application.Theme` read "modern" after `auto`
  (now `rapidr`) chose the dark or high contrast look; it reads the look
  drawn.
- A docked group's tabs were measured in the regular face but drawn bold:
  the shown tab's title was cut short ("Out…").
- **A window shown from a modal form ignored the user.** Under
  `Form.ShowModal`, `Form2.Show` opened a window that took no clicks or
  keys (and gave the focus straight back). As in Windows, a modal form
  now blocks only the windows that were open when it went up.
- On Linux's Wayland sessions `Form.Hide` left the window on screen (it
  now goes, and `Show` brings it back).
- Keys still held when a window got the focus were typed into it (on
  Windows, the P of a Ctrl+Shift+P that opened a window).
- `RDOCKMANAGER.PaneTitle` of a document that wasn't the active MDI window
  renamed the active one.
- **Debugger: a SUB's own variables showed up in Globals under made-up
  names.** A SUB's STATIC variables and the variables a SUB uses before
  the main program does (RapidQ keeps those between calls) were listed in
  the Globals scope as `S__p` or `SUB S::hits` (with a `#init` twin). They
  are now in that SUB's Locals under the names the program gives them, and
  Globals lists only the program's real globals. A watch or the debug
  console can read and set a SUB's STATIC variables too. The same in VS
  Code (`rapidr dap`) and RapidR Studio.
- **VS Code: an error right after a dot (`Form.` at a line's end) was
  underlined over nothing.** It now underlines the dot.
- **VS Code: Format Document re-indented the second line of a string
  continued with `_` (under `$ESCAPECHARS`)**, which changed the string.
  A string that spans lines is now left exactly as written.

- **Web IDE: an example or opened file showed nothing.** Picking an
  example (Menus, Hello form, …) said "loaded", but no tab opened, the
  Project panel stayed empty and the designer was blank. The examples are
  written with RapidQ's component names (`CREATE Form AS QFORM`, `QMEMO`,
  `QMAINMENU`), and the IDE's reader only knew a form as `RFORM`: it found
  no form, so it dropped every component and the code with them. Now a
  RapidQ name is the RapidR component it stands for, as the compiler reads
  it (the language registry's entries in `lang-data.js` now say which
  component they are), and the designer shows the form: its components,
  the main menu's own menus, a status bar's text, components docked with
  `Align` (and a status bar at the bottom), several properties on one line
  (`Caption = "&New": OnClick = NewText`). A program with no form (the
  console examples) opens in a code module. Opening a local `.rr` or `.bas`
  file does the same, and it runs as it is written, as an example does.
  The run window was already drawing programs at 1:1 (checked at pixel
  ratio 1 and 2, the same pixels as the standalone web runtime); a test now
  keeps it so (`tests/web_ide_examples.mjs`).
- **Web: typing in a Save As dialog went into the program's window
  below.** In the web IDE, Notepad's File > Save As showed an in-page
  dialog; a click in its file name field lost the focus at once, and the
  keys went into Notepad's editor. The dialog was a page element, not a
  window of the UI kernel, and the Open / Save wait put the program's own
  form on the modal list as the innermost modal window: each repaint of it
  (the caret's blink) synced its accessibility mirror, which moved the
  page's focus back to its editor. The in-page dialog is gone (see Changed),
  and the form now goes on the modal list *before* the host is asked, so a
  box the host shows for the dialog is the innermost window and keeps the
  focus, the keys and the clicks. The modal rule is checked with real input
  on the web for every dialog — MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE,
  INPUT's box, QCOLORDIALOG, QFONTDIALOG, a ShowModal form and the Open /
  Save boxes: a click on the window below is refused and the focus stays
  while it repaints, keys never reach it, Tab / Shift+Tab cycle inside the
  dialog, Enter presses its default button, Escape cancels, and the focus
  goes back to the control that had it (`tests/web_modal_focus.mjs`,
  `tests/web_file_dialogs.mjs`).
- **Desktop (macOS): a click on a window under a modal dialog brought that
  window over the dialog.** Its input was refused, but the window took the
  keyboard and covered the box (MESSAGEBOX, colour, font, a ShowModal
  form), native and interpreted alike: the host gave the modal window the
  focus back inside the focus event, and the click's own activation won.
  It now gives it back again once the click is over, so the dialog stays in
  front with the keyboard, as Windows keeps a modal dialog over its owner
  (checked with real clicks and keys on both builds).
- QSTRINGGRID with `FixedCols = 0` (or `FixedRows = 0`) showed from column
  (row) 1: the first scrollable column stays the first one shown, as in
  Delphi's grid.
- RDATAFRAME's `Cell`, `At` and `SetCell` gave strings in quotes on the
  desktop (`"Alice"`); now as the web gives them.
- RJSON on the web: `LoadFile` / `SaveFile` work (the page's files, as
  OPEN's), values `Set` as numbers / booleans as on the desktop; on the
  desktop an object's keys keep their order (as the browser's), `Remove`
  too.
- RDATAFRAME's `LoadFromCsv` on the web reads a file the program saved
  (`EXTRACTRESOURCE`, OPEN), not only the project's.
- **The main program's end is the program's**, as in RapidQ (RC.EXE,
  checked in the Windows VM): a program whose main code ends after
  `Form.Show` (no ShowModal, no DOEVENTS loop) runs OnShow inside Show and
  then ends — its form goes, its timers never tick. The interpreter kept
  such a program's windows open with their timers stopped (so a QTIMER
  never ticked), the web kept them open and ticking; native builds already
  ended. The loop after the main program is gone from the interpreter, the
  debugger and both web builds (`rp_run_app`, `Wait::App`). Conformance
  case `main_ends_after_show`, `tests/web_main_end.mjs`.
- **`Form.` with no member right after the dot** is RC.EXE's `Member  not
  part of class FORM` (the member it read is empty; `Form.Font.` is
  `Member FONT. not part of class FORM`): the next line was joined to it,
  so `Form.` then `Nope x` compiled. A space after the dot (`Form. Caption`)
  and WITH's `.` alone are the same error, as in RapidQ; `Form._` with
  the member on the next line still joins. Cases `member_dot_alone`,
  `member_dot_continued`.
- **`$TYPECHECK ON` checks the names a program reads**, not only those it
  stores into: an undeclared name passed to a method (`List.AddItems
  itme`), a SUB or FUNCTION, printed, or used in an expression or a
  condition is RC.EXE's `Undefined symbol ITME`. A name stored into while
  the check was off is a variable from there on, as in RapidQ; True /
  False and RapidR's own constants stay known (RapidR's additions). Cases
  `typecheck_reads`, `typecheck_reads_ok`.
- **A SUB or FUNCTION written without its parameters has the ones it was
  DECLAREd with** (`DECLARE FUNCTION G (a AS INTEGER) AS INTEGER` then
  `FUNCTION G` reads `a`; RC.EXE's G(4) is 8): `a` was an implicit
  variable there (0). RapidQ's `forms/MinToTaskbar.bas` is written so.
- **A program sees only its own command line**, however it runs. `rapidr
  run-bc prog.rrbc a b` handed the program the runner's own arguments
  (`COMMAND$` was "run-bc prog.rrbc a b"; the IDE opened "run-bc …" as a
  file), and so did a standalone runner's `--bytecode <file>`; now they
  set the program to the file with the arguments after it, as `rapidr run`,
  `rapidr open`, the IDE and built executables do. An old macOS's `-psn_…`
  argument (Finder's process number) is no argument either.
- **`COMMAND$(n)` and `CommandCount`**, as RapidQ's (RC.EXE, checked in
  the Windows VM): `CommandCount` was an unknown name (0) and `COMMAND$(n)`
  ignored `n`. Now `COMMAND$(0)` is the program's file, `COMMAND$(1)` …
  its arguments (`"b c"` one), any other index ""; the bare `COMMAND$`
  stays RapidR's (the arguments joined with spaces). On the web the
  arguments are the page's query string's parts (`?a&b%20c`: `a`, `b c`),
  decoded, or the ones a page hands its program (`window.RAPIDR_ARGS`):
  the web IDE's preview gives none, never its frame's own query string
  (a program there saw "role=run&v=…"). Shared by every runtime (`rapidr_value::command_line`);
  conformance cases `command_line_args` (with the runner's new
  `<case>.args`) and `command_line_none`, the web bundle's query string in
  `tests/web_bundle_console.mjs`.
- `RDataFrame.ToString` / `Print` on the desktop showed only
  `shape: (2, 2)` and a note about polars' `fmt` feature, and `Print`
  printed twice: a frame now prints once, as a plain-text table that is the
  same on every runtime (a header, a rule, the rows — numbers
  right-aligned, `null` for a missing value, the first and last ten rows of
  a longer frame — and `[2 rows x 3 columns]`); `ToString` returns it
  without printing.
- `RDataFrame.Cell` / `CellByName` returned text cells in double quotes
  (`"bob"`) on the desktop: they return the plain text.
- `RNum.Shape` printed "RNum.shape() not implemented" instead of the
  array's shape; a property read the way methods are (`PRINT a.Shape`)
  now reads the property on every data-science component.
- On the web, `RDataFrame.SaveToCSV` / `SaveToJSON` before any other file
  I/O wrote nowhere: data frames use the page's files from the start.
- QSTRINGGRID with `FixedCols = 0` (or `FixedRows = 0`) showed from column
  (row) 1: the first scrollable column stays the first one shown, as in
  Delphi's grid.
- RJSON on the web: `LoadFile` / `SaveFile` work (the page's files, as
  OPEN's), values `Set` as numbers / booleans as on the desktop; on the
  desktop an object's keys keep their order (as the browser's), `Remove`
  too.
- **A FUNCTION's own name inside it** follows RapidQ's compiler (RC.EXE):
  without parameters it is a call of itself (`G = G + 1` recurses);
  with parameters, reading it without arguments is the compile error
  `Expected ( but got "+"` on every runtime. The interpreter read the
  result variable instead, and native builds failed to compile such a
  program. `RESULT` reads the result. Conformance cases
  `function_self_name`, `function_self_read` (checked against RC.EXE).
- `ANNOUNCEMENT.md` announced "1.0.0" and called RapidR a reimplementation:
  it now announces 2.117.0 as the release notes do — RapidR is compatible
  with RapidQ, an original implementation written from the ground up in
  pure Rust.

## [2.117.0] — 2026-10-06

### Added
- **RapidR's brand** (`design/brand/`): an original logo (an R whose
  counter is a play triangle, on a tile), the app icons for RapidR and the
  RapidR Runtime and the file icons for `.rr`, `.bas` and `.rrbc` on macOS
  (`.icns`), Windows (`.ico`), Linux (hicolor) and the web (favicons), the
  README banner and the GitHub social preview — original vector art, MIT,
  wired into the installers and the web IDE.
- **For testers and supporters:** GitHub issue forms (bug report, RapidQ
  compatibility, test report; security reports go privately through
  SECURITY.md), and "Support RapidR" (Buy Me a Coffee) in the README and
  `.github/FUNDING.yml`.
- **The IDE plan** (`docs/ide-plan.md`, ROADMAP Phase 3 as stages I0–I9):
  RapidR Studio on the UI kernel, desktop and web — the designer,
  code completion, Delphi-style linked data and data-science components, a
  debugger, AI through an MCP server in the IDE; every IDE block a public
  component (`docs/ide-components.md`, `docs/ide-ai.md`,
  `docs/q-and-r-components.md`).
- **Every program RapidR builds carries its open-source notices.** A
  `THIRD-PARTY-NOTICES.txt` is written beside every native and interpreted
  executable and into every web build (the zip's root for `bundle-bc` and the
  web IDE's **Build**, `<stem>_web/` for `build --web`; `index.html` links it
  with `<link rel="license">`). It lists RapidR's runtime and every component
  compiled in — the crates of the build's real dependency graph for that
  target, Rust's standard library, the Liberation fonts, SQLite and the C
  code crates bundle, the system's and toolchain's pieces — with every
  licence text in full, so shipping the program with that file is all the
  licences ask. `rapidr notices [<os>-<arch>|web|tools-<os>]` prints one; an
  install ships them (`lib/rapidr/notices/`), so this works offline without
  Rust. Programs see no difference (no new command-line switch).
- **LEGAL.md** (what you may do with RapidR and the programs you build, what
  to ship, trademarks and no affiliation with RapidQ's author or any vendor,
  no warranty) and **docs/licensing.md** (what each output contains, licence
  by licence; what each licence asks; codecs and patents; open questions for
  a professional review). Linked from the README, the web IDE's About and
  Help > Legal…, the desktop IDE's About and `rapidr about`; the installers
  include it (the Windows installer shows it).
- `tools/regress.sh legal`: `cargo deny check licenses`, the repository's
  notices, and `tools/check_notices.py` (builds a program each way and checks
  its notices list every crate `cargo tree` finds).
- **The RapidR Runtime and release packaging** (docs/release-packaging.md,
  `tools/release/`): an installed RapidR finds its files by one rule
  (`RAPIDR_HOME`, `<exe>/../lib/rapidr` with its `release.toml`, else the
  source checkout). Programs run, the IDE works and interpreted executables
  build with no Rust installed (shipped runners, `--target <os>-<arch>`);
  native builds compile offline against the shipped runtime sources, and
  `rapidr setup` installs Rust through rustup only after asking. New
  commands `rapidr run`, `open`, `info`, `ide`; `rapidrw`, the windowed
  launcher. Bytecode format 3 records the oldest runtime a program needs and
  its app type — an older runtime says "this program needs RapidR Runtime
  ≥ x.y" (format-2 files still run). `#!/usr/bin/env rapidr` scripts on
  macOS and Linux. File types per user: `.rrbc` runs on a double click,
  `.rr` / `.bas` open in the IDE with a Run action; a downloaded file
  (quarantine / Mark of the Web) asks once before it runs. Windows GUI
  programs build as windowed executables (no console window). Local build
  scripts for the installers: macOS `.dmg` (the IDE + SDK, and the Runtime
  alone), Windows (Inno Setup), Linux `.tar.gz` / `.deb`, the web bundle,
  SHA256SUMS and an SBOM.
- **The system tray (QNOTIFYICONDATA) on every platform.** RapidQ
  programs put an icon in Windows' notification area with QNOTIFYICONDATA
  and `Shell_NotifyIcon` (shell32) and hear its clicks in their form's
  WndProc. RapidR keeps that one call: the icon shows in the menu bar on
  macOS, the notification area on Windows, StatusNotifierItem on Linux (no
  system package needed) and a small strip at the page's bottom right on the
  web; its clicks reach the form's `WndProc (hWnd, uMsg, wParam, lParam)` as
  Windows' mouse messages. `Application.Icon` reads as a number (RapidQ's
  icon handle), so `NI.hIcon = Application.Icon` shows the program's icon.
- **QDIRLISTVIEW and QDOCKFORM built in**: RapidR's own versions of RapidQ's
  QDirListView.inc and RAPIDQ2.INC's dockable form (theirs call Windows),
  written in BASIC on RapidR's components, so they behave the same on every
  runtime. QDIRLISTVIEW lists a folder (Name, Size, Type, Date Modified),
  goes into folders and up, fires OnFileSelect; QDOCKFORM docks, floats in
  its own window, docks at its alternative place, with the library's title
  and grip styles, a close box, OnDock / OnClose. A program that includes
  QDirListView.inc or RAPIDQ2.INC gets RapidR's.
- **QGLASSFRAME** with RapidQ's compiler's members and values (Transparency
  a byte, Moveable, TransparentColor; 105 × 105): its glass colour over what
  is under it at 100 − Transparency percent (the form's background — RapidR's
  windows aren't see-through), and Moveable: dragging it moves its form.
- **FileRec** (RapidQ's DIR$ companion): FileName, ShortName, Date, Time,
  Size, FileTime of the file DIR$ found last.
- **QCGI**, RapidQ's CGI object (its manual's Appendix A: QCGI.INC 1.6),
  built in on every runtime: the CGI variables as read-only properties
  (Accept, ContentLength, Cookie, QueryString, RemoteAddr, ServerPort,
  UserAgent, …), MaxInput, AutoConvert, Parse and `Get(Name, Value)` —
  pairs read from a GET's QUERY_STRING or a POST's body, `%xx` and `+`
  decoded exactly as the library does (checked against RapidQ itself:
  RC.EXE running the library, `tests/conformance/cases/cgi_*`). A program
  that `$INCLUDE`s `qcgi.inc` gets RapidR's QCGI (the file's TYPE is left
  out, its constants kept; without the file, its constants are built in).
- **QCOMPORT**, a serial port as RapidQ programs have it (RAPIDQ2.INC's
  COMPORT, the manual's QCOMPORT — RC.EXE's own can't be built): Port,
  BaudRate, DataBits, Parity, StopBits, buffers, Connected, Handle,
  BytesNotRead / InQue …; Open, Close, PurgeIn / PurgeOut, WriteString,
  Write, ReadString, Read; OnComError (the library's messages with
  Windows' own text, checked against RC.EXE), OnOpen, OnClose,
  OnWriteString, OnReadString, and OnRxChar when bytes arrive. Ports
  through serial2 on the desktop (nothing extra to install on Linux), Web
  Serial in the browser; tests use scripted ports (`RAPIDR_TEST_COMPORT`),
  never a real device.
- **QDOWNLOAD** (Qdownload.inc): LeechFile fetches Server / Port / File
  into OutVar or OutFile with the library's checks and error messages,
  progress in State or its own StateGauge and SpeedLbl — and the program
  no longer freezes while it waits: its windows paint and its timers tick
  (native, interpreted and in the browser). Port 443 is HTTPS; redirects
  are followed.
- **QMIDI, QWAVE, QCDAUDIO and QVIDEO**, D. Glodt's media objects (RapidQ's
  `QMidi.inc`, `QWave.inc`, `Qcdaudio.inc`, `QVideo.inc`), on one model:
  Open / Close / Play / Stop / Pause, State, FileOpen, Lenght, Error (MCI's
  own texts, checked in the Windows VM), Volume, their Timer and OnChange,
  the libraries' rules exactly; positions by the clock, with or without a
  device. QMIDI reads standard MIDI files with MCI's timing and plays them
  on the system's MIDI output (midir; Web MIDI); QWAVE plays WAV files,
  records (New, Record from the default input — cpal, getUserMedia), Saves
  and Deletes. QCDAUDIO answers as a machine with no disc does. RapidQ's examples `midi.bas`, `wave.bas`, `Cd.bas`, `video.bas`
  compile and start with RapidQ's include folder.
- **QVIDEO plays AVI files** on every runtime with decoders of RapidR's
  own (`objects::avi`): uncompressed DIB (1–32-bit, palettes), Microsoft
  RLE8 / RLE4, Microsoft Video 1 (CRAM), Cinepak and Motion JPEG, and the
  file's PCM sound. Frames follow the clock; the picture is drawn on a
  canvas on Parent's form or in a window of its own (popup / overlapped
  sizes, the picture stretched, seeks landing on key frames — all as MCI
  does in the Windows VM); the browser runs the same decoder in wasm.
  `tools/make_avi_fixtures.py` writes the test clips.
- **QMIDI plays without a system synthesizer** (macOS, Linux, browsers
  without Web MIDI): a built-in General MIDI synthesizer of RapidR's own
  (`objects::synth`) — procedural instruments and drum kit, no SoundFont
  bundled — on the sound device (rodio; Web Audio in the page).
- The media libraries' **Error** keeps MCI's NUL (`LEN` one more than the
  text) and an **Open while a file is open** leaves that file open, as
  RC.EXE runs QMidi.inc / QWave.inc / QVideo.inc; QVIDEO's Caption is MCI's
  128-character buffer.
- A stray **`END STRUCT` / `END TYPE`** is RC.EXE's END (the program ends
  there): RapidQ's `Network/Download/qdownload.bas` compiles.
- A binary operator with nothing after it (`F("a"+, 5)`, `x = 2 +`) is read
  as RapidQ's compiler reads it: dropped, the value is what came before
  (`Qcdaudio.inc` has it four times).
- **The `ENVIRON "name=text"` statement** sets an environment variable
  (it was a silent no-op): split at the first `=` (or, without one, a
  space), names found in any case by `ENVIRON$` as on Windows; in the
  browser a table of the page's own.
- **One web host: the old DOM host is deleted** (docs/web-host-plan.md, "W11"): the UI kernel draws every program's windows on the web as on the desktop, with no `?host=dom`, no `kernel` feature and no `rapidr-rrcss`; the web runtime is 0.4 MB smaller.
  - RWEBNOTIFICATION.Show shows the notification again (a form's Show had taken it), and RROUTER.Route / Hash read the address.
  - QIMAGE.LoadFromPlot loads the chart's pixels, as on the desktop.
  - Application.Icon is the page's icon too.
  - A component bound with DataSource / DataField writes the user's edits to its field.
  - A QIMAGE given a non-BMP file warns and loads nothing, as on the desktop.
  - SLEEP holds a web program whole, as on the desktop. Its timers fire once it waits again, so a timer's handler that opens a box right after a SLEEP gets its answer.
- **Fallback fonts on the web (W7)** (docs/web-host-plan.md, "W7"): text the built-in Liberation fonts can't draw — ✓ and other symbols, Chinese, Japanese, Korean, emoji (in colour) — is drawn with Noto fonts (SIL OFL 1.1) instead of boxes.
  - The fonts ship beside the web runtime, split by Unicode range. A page fetches a chunk the first time its text needs it: labels, edits and window titles alike.
  - Bundles (`bundle-bc`, the IDE's Build) and `rapidr build --web` sites carry the chunks; an installed RapidR never downloads them.
  - `python3 tools/fonts.py` fetches the CJK sources (official Noto releases, pinned by SHA-256) and builds the chunks. Noto Sans and the Symbols fonts are in the repository.
- **The web IDE loads no Google Fonts** any more: its interface uses the system's fonts, and nothing in the IDE, the runtime or a bundle contacts a third party unless the program does.
- **The UI kernel is the web's host by default** (docs/web-host-plan.md, "The kernel host by default, and W6"): the IDE's preview, `bundle-bc` bundles and `rapidr build --web` pages draw the program's forms with the kernel, with no query parameter. `?host=dom` keeps the old DOM host until it is deleted.
  - **Web-only components over the canvas (W6):** RWEBVIEW, RDOM, RWEBAUDIO / RWEBVIDEO and RPLOT are the page's own elements (`#rr-<name>`), placed by the kernel at their components' places, clipped to their parents and hidden with them. Drop-down lists and menus open on a layer above them.
  - **INPUT** with windows shown asks in the kernel's input box.
  - **END** in a native web build closes its windows.
  - **Combo boxes:** typing in one sets ItemIndex as Windows does: -1 while the list is closed, the first matching item while it's dropped down (desktop too).
  - **Grids:** a grid's in-place editor and its drop-down list are in the accessibility tree, and the editor has the focus there (desktop too).
  - **Tests:** every web suite runs on the kernel host (`tests/web_kernel_page.mjs`: mirror elements, real clicks, the canvas's pixels); `tests/web_overlays.mjs` is new.
- **The web on the UI kernel, Stage W4** (docs/web-host-plan.md, "W4 results"): every browser GUI case now runs on the kernel host (`?host=kernel`), 70 of 70, with the desktop's dumps.
  - **Waits:** ShowModal, DOEVENTS, INPUT$ / WAITKEY and dialogs suspend the browser's VM and resume it as the desktop's interpreter serves them (`rapidr_ui_app::waits`), nested ones in order.
  - **Dialogs:** MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE, MSGBOX, the colour and font dialogs are the kernel's own, pixel-identical to the desktop's. Open / Save go through the kernel's file-dialog request, answered by the page's picker.
  - **Timers** run on the shared timer heap, as on the desktop (started by modal waits, held back while a handler waits).
  - **The host:** windows size from every edge and corner; minimized windows line up along the bottom; an edit's `AutoComplete` property becomes its field's autofill hint.
  - **Native web builds** (`rapidr build --web`) build the runtime with the kernel host and wasm SIMD.
  - A grid scrolls a newly selected cell into view before its accessibility tree is made, so the tree's cells are where they're drawn (desktop too).
  - `tools/regress.sh` makes the desktop's GUI captures and compares the kernel host's windows and trees with them at 1× and 2×; `tests/gui_captures.mjs` runs QDOWNLOAD / QMIDI / QWAVE against the tests' own server and scripted devices.
- **The web on the UI kernel, Stage W3** (docs/web-host-plan.md, "W3 results"): with `?host=kernel` in a page's address, or in the web IDE's (it passes it on to its preview), the program's forms are drawn by the same UI kernel as on the desktop. The DOM host stays the default.
  - **Windows on the page** (`rapidr-ui-host-web`'s `WebHost`): the kernel's display list, drawn by the shared CPU renderer at `devicePixelRatio`, with a kernel-drawn frame in the current theme. Windows stack, move by the title bar, size by their edges, and maximize, minimize, restore and close as on a desktop. A scale change redraws them and fires OnScaleChanged; a lost canvas is redrawn.
  - **Input:** the pointer, the wheel, keys, the clipboard events, input methods (the candidate window at the kernel's caret), a phone's keyboard, and autofill.
  - **Accessibility:** an ARIA mirror of the kernel's tree, written in Rust, with the DOM focus on the kernel's.
  - **In the runtime:** the runtime's `Program` / `Windows` for `rapidr-ui-app` and a `WebStore` (`rapidr-runtime-web` feature `kernel`, on in the web runtime's wasm).
  - **Results:** 64 of the 68 browser GUI cases give the desktop's dumps on it. Their windows are byte-identical to the desktop's captures at 1× and 2×, and their accessibility trees match the desktop's: Chrome's tree over the mirror is the kernel's.
  - **Tests:** `RAPIDR_WEB_HOST=kernel` for `tests/web_gui_parity.mjs`, `tests/web_a11y.mjs` and `tests/web_gui_run.mjs`; `RAPIDR_DESKTOP_CAPTURES` compares pixels and trees. `tests/gui_captures.mjs` keeps every GUI case's captures, trees and dumps.
- `rapidr_value::component_defaults`: what both runtimes' registries give a new component, the desktop's own defaults, and the kernel stores' "unset" rule, in one place (the registry's step 1; no property changed).
- **RapidQ's own compiler as the ground truth, at scale**
  (`docs/rapidq-ground-truth.md`): `tools/rc_probe.sh` runs a list of
  programs through RC.EXE in the Windows VM, each in its own folder with its
  `.input` as standard input, and returns RapidQ's exact output;
  `tools/rapidq_truth.py` stages the conformance cases, the console programs
  of RapidQ's examples or a folder of probes, runs them through RC.EXE and
  RapidR (VM, `--native` too) and lists every difference; `--write-expected`
  pins RapidQ's output as a case's `.expected`. Programs that could print or
  touch the registry are never run in the VM. `tests/rapidq_golden/`: RapidQ's
  own output of console examples (`tools/rapidq_truth.py golden`).
- Conformance cases that stop with a run-time error:
  `name.expected-runtime-error` (both backends and the browser), and new
  cases whose `.expected` is RC.EXE's output: `rapidq_print_doubles`,
  `rapidq_int_rounding`, `rapidq_numeric_stores`, `rapidq_operators_int`,
  `rapidq_text_functions`, `rapidq_if_print_else`, `rapidq_booleans`,
  `rapidq_division_by_zero`.
- **QBEVEL and QDIGDISPLAY built in.** The manual documents them as
  components, but in RapidQ they are TYPEs of its include libraries
  (QBevel.inc, QDigDisplay.inc) — the manual's own QBEVEL example doesn't
  include it. RapidR now has both on every runtime, behaving as the
  libraries do: QBEVEL's Shape / Style set its bevels (bsBox, bsFrame) or
  draw two lines at an edge (bsTopLine … bsRightLine); QDIGDISPLAY shows
  Display in 12 × 24 seven-segment cells (characters 32 … 64), in the
  library's colours, sized to it. A program that includes the library
  gets the library's own TYPE, as in RapidQ (QDigDisplay with its own
  bitmaps). Screen readers: a pane named by its caption, an image named
  by its digits.
- **QRECT and QNOTIFYICONDATA as RapidQ's compiler has them** (checked
  against RC.EXE): one shared record model on every runtime — fields are
  32-bit integers cut toward zero (`3.7` → 3), a string stored into one is
  0, out-of-range values -2147483648; QNOTIFYICONDATA's cbSize is 88 and
  read-only, uID starts as the instance handle 4194304, szTip keeps 64
  characters up to a CHR$(0); `SIZEOF` 16 and 24. RapidQ's errors: `Member
  WIDTH not part of class R`, `Component assignment is not yet supported.`,
  `Datatype QRECT not supported in STRUCT`, `N.CBSIZE is a read-only
  value.` (QRECT's Left rounded before, and unknown members read empty).
- **QDXJOYSTICK**, RapidQ's joystick object (missing from its manual; its
  compiler has it): Update, IsLeft / IsRight / IsUp / IsDown, Button(n) —
  plus RapidR's Index, Connected, Name, X / Y / Z / R / U / V (winmm's 0 …
  65535), Buttons, POV and the events OnButtonDown / OnButtonUp / OnMove.
  Gamepads come from gilrs on Windows and macOS, the kernel's evdev on
  Linux (no extra system package to build) and the Gamepad API in the
  browser, all laid out the same way. No joystick reads "not connected",
  never an error. `DECLARE … joyGetPosEx` (winmm) now names QDXJOYSTICK.

### Changed
- **Linux needs OpenSSL 3** (HTTPS uses the system's TLS): RapidR's Linux
  packages run on Ubuntu 22.04 / Debian 12 and newer (`libssl3`); Ubuntu
  20.04 and Debian 11 are no longer supported.
- **Nothing in a program you build is copyleft, cryptographic or
  data-licensed any more** — removed, not just documented (LEGAL.md,
  docs/licensing.md):
  - **HTTPS uses the operating system's TLS everywhere** (`RHttp`,
    `QDownload`): Security.framework on macOS, SChannel on Windows as
    before, and on Linux the system's OpenSSL 3 (`libssl.so.3`, linked
    dynamically, never shipped), with the system's certificates. `ring`,
    `rustls`, `rustls-webpki` and `webpki-roots` (CDLA certificate data) are
    gone: no cryptographic code is compiled into a program. **Linux:**
    programs need the system's `libssl3` (every current distribution has
    it; the `.deb`s depend on it), and building needs `libssl-dev`
    (docs/release-packaging.md).
  - **MP3 is decoded by `nanomp3`** (MIT OR Apache-2.0, pure Rust, a port
    of the public-domain minimp3) instead of rodio's `symphonia` (MPL-2.0);
    WAV, Ogg Vorbis and FLAC stay rodio's. PLAYWAV / PLAYSOUND play the same
    files.
  - **RPLOT's charts draw their text in the built-in Liberation Sans** (with
    `ab_glyph`, through plotters' backend interface) instead of the
    system's fonts through `font-kit`: the same size and placement as
    before, the same on every system. With font-kit went `dwrote` and
    `option-ext` (MPL-2.0), FreeType (whose licence asks for credit in the
    documentation) and the charts' fontconfig link; the CLI no longer uses
    `dirs`.
  - **winit's KDE blur bindings are RapidR's own**: crates.io's
    `wayland-protocols-plasma`, generated from KDE's protocol files (some
    LGPL-2.1-or-later), is replaced in every build — the workspace, every
    generated program, an install's home — by
    `crates/patches/wayland-protocols-plasma` (MIT): the same Rust names,
    from RapidR's own protocol file, which no compositor announces (winit
    finds no blur manager; RapidR never asked for blur).
  - The Linux notices now also carry the notices of what crates' code is
    generated from or built with: every Wayland protocol description
    (MIT, HPND-sell-variant), xcb-proto (X11), the Cantarell font in
    winit's Wayland title bars (OFL-1.1) and, on every target, the Adobe
    Glyph List in `read-fonts` (BSD-3-Clause).
- **The licence guard is strict**: what a program may contain is the
  permissive list only — MIT, Apache-2.0 (and WITH LLVM-exception),
  BSD-2/3-Clause, ISC, Zlib, 0BSD, BSL-1.0, Unlicense, Unicode-3.0, CC0-1.0,
  and OFL-1.1 for fonts. deny.toml allows only that (for the whole
  workspace, tools included) and bans the replaced crates; the notice
  generator refuses any kind of output with anything else (or a replaced
  crate, or a Wayland protocol file under another licence); and
  `tools/check_notices.py` checks every kind's graph against its own copy
  of the list. `tools/regress.sh legal` exits 1 on any failure, and only the
  web stage needs the repository served (`RAPIDR_URL`, else port 8765).
- QDirListView and QDockForm's built-in libraries rewritten as RapidR's own
  code (their bodies followed user-contributed RapidQ libraries too closely);
  same members, events and behaviour. Four conformance cases that reused the
  RapidQ manual's examples replaced by our own programs covering the same.
- Web bundles carry `THIRD-PARTY-NOTICES.txt` instead of `LICENSE-RapidR.txt`,
  `THIRD_PARTY_NOTICES.md` and `LICENSES.md` (it holds RapidR's licence and
  the full texts the others lacked).
- The SDK's cut-down vendored crates (those only other platforms compile)
  keep their licence files.
- **DIR$ as RapidQ's**: attribute 0 lists files only, faDirectory (&H10)
  folders too with `.` and `..` first, names sorted without regard to case,
  wildcards (`*`, `?`) of any case, `\` as a folder separator; dot files
  only with faHidden.
- **Hiding a modal form no longer ends its ShowModal** (VCL's, so RapidQ's):
  only Close or ModalResult do — a program can hide its window into the tray
  and show it again (desktop, interpreter, and the web on both its hosts).
- The desktop host's `Desktop` (the forms' kernel sides, stacking, the modal list, the window command and event queues, the input entry points) and runtime-core's window-command and test-input glue now live in `rapidr-ui-app`, shared by the desktop and web hosts. The desktop is unchanged: every GUI case's captures, accessibility trees and dumps are byte-identical at 1× and 2×.
- The web runtime's wasm is built with wasm SIMD (`tools/build_web_artifacts.sh`, its own `target/wasm-simd`). With the kernel host it grows from 6.27 to 10.18 MB raw (1.95 → 2.70 MB brotli).
- **Operands side by side read as RapidQ's compiler reads them.** RC.EXE
  takes `SetRenderMode(A A OR B)`, `-9(COS(x))`, `x = 16 374739` and the
  like — its operator stack runs on, the value is the operand stack's
  bottom (`A B OR C` is A, `-9(COS(x))` is 9, `(2 3) + 1` is 2) and the
  other operands are still worked out (a FUNCTION among them is called).
  RapidR refused them; both compilers now read them the same way (checked
  line by line against RC.EXE: `tests/conformance/cases/
  juxtaposed_operands.bas`). So `CASE 4, 7<TAB>C& = -2` (RapidQ's
  `reminder/dayfunction.bas`) is the list 4, 7 with no body, as in RapidQ —
  RapidR used to run the assignment.
- **RapidQ-exact numbers and more, checked against RC.EXE** (native,
  interpreted and web alike; details and evidence in
  `docs/rapidq-ground-truth.md`):
  - PRINT shows a fractional number with 9 decimals (`3.500000000`), Delphi's
    digits, and a whole one as a 32-bit integer (beyond: `-2147483648`, as
    for an infinity or NaN); STR$ has 9 significant digits (`0.333333333`,
    `1.23456789E9`, `1E-5`). They were the shortest form / 15 digits.
  - PRINT's (and LPRINT's) comma is the semicolon — no 14-column zones; a
    leading separator is accepted (`PRINT , "x"`).
  - INT and FIX truncate toward zero; ROUND, CINT and CLNG are `INT(x +
    0.5)` (2.5 → 3, -2.5 → -2, -2.7 → -2); ROUND, CINT, CLNG, CEIL and FLOOR
    are 32-bit.
  - A store into an integer variable truncates (2.7 → 2), beyond 32 bits it
    is -2147483648; a BYVAL parameter rounds half to even; a FUNCTION's
    result isn't converted; DWORD is 32-bit signed; SINGLE is a real 32-bit
    float. They rounded half to even, wrapped, converted results, kept DWORD
    unsigned and SINGLE double.
  - `\` rounds its operands as CINT; MOD, AND, OR, XOR, NOT, SHL, SHR take
    32-bit operands rounded half to even (`7.5 MOD 2` = 0); `\` and MOD by
    zero stop the program ("Division by zero"), `/` by zero is an infinity
    (all three gave 0); a NaN compares equal and less; `&H80000000` …
    `&HFFFFFFFF` are negative; INV is -1 without an inverse.
  - VAL skips spaces and reads the number at the start (`"12abc"` = 12);
    HEX$ has 8 digits; BIN$ 32 bits; REPLACE$ appends past the end;
    INSTR never finds an empty needle and counts from a start before the
    text (`INSTR(-5, "abc", "b")` = -4); CHR$ / STRING$ round a real code;
    CONVBASE$ of a negative number gives its 32 bits;
    Clipboard.GetAsText(n) gives n - 1 characters.
  - A component's Boolean property or method result reads 1 when true
    (`IF Check.Checked = True`, RAPIDQ.INC's True = 1); FILEEXISTS and
    DIREXISTS give 1; a QFORM starts Enabled.
  - An undeclared variable a SUB uses before the main program does is the
    SUB's own (kept between calls), as RapidQ's one-pass compiler has it;
    `DIM m` without AS is a DOUBLE; `""` inside a string is no escaped quote
    (two strings side by side — the web IDE's designer writes quotes as
    `CHR$(34)` now); STRING * n is always n characters (padded with spaces,
    n spaces at first; `STRING * 0` empty).
  - A PRINT right before the ELSE of a single-line IF stays on its line;
    `CASE IS = "l" AND x = "d"` compares first, then ANDs; inside a TYPE's
    own code a store into its property field doesn't call the setter.
  - A component's Color before the program sets one reads RapidQ's
    system colours: clBtnFace (`&H8000000F`) for a QFORM / QPANEL, the
    parent's Color for a QLABEL / QCANVAS / QGROUPBOX (clWindow without
    one), clWindow (`&H80000005`) for the others — it read white. A system
    colour is drawn in the theme's colour (clBtnFace: F0F0F0), so a form's
    Pixel reads the F0F0F0 it shows, and TextOut blends over it. A QCANVAS
    shows its parent's colour where nothing is drawn, not its own Color
    (RapidQ's TPaintBox). The desktop's "creation white" rule is gone.
  - Font.Color reads clWindowText (`&H80000008`) until set — a new QFONT's
    Color too — and a parented component its parent's (ParentFont, drawn
    so too); it read 0. Form.Pixel reads -1 before Show, after Close,
    outside the form and over a window of its own (a panel, a button …),
    and a label's / canvas's / image's pixel over those.
  - TIMER is the seconds since local midnight (it was since 1970 on the
    desktop, since the page loaded on the web); TIME$ and DATE$ are local.


### Fixed
- Inside a TYPE extending a component, `DIR$`, `TIMER`, `DATE$` and the
  other builtins written without parentheses, and CALLFUNC, were taken for
  the component's members (a native build's DIR$ loop stopped early).
- In `CREATE x AS Type` (a TYPE), a nested CREATE's own `Parent = …` was
  overwritten, and `Field.Member = …` (`AltPanel.Parent = Form`) wasn't the
  instance's field.
- What a program draws before its form shows (a QCANVAS's picture, a
  QDIGDISPLAY's digits) is made at the screen's scale from the start: a
  form is drawn at the host's scale before its window exists, and a window
  that opens on a screen of another scale says so (OnScaleChanged). Before,
  a high-DPI window could show (and a test capture) an enlarged, smoothed
  1× picture until something drew again.
- The GUI cases `glass_frame`, `dock_form` and `bevel_display` give their
  forms' ClientWidth, so their pixels are read at a real screen's scale
  (they failed with real windows on a 200 % Windows screen).

## [2.116.0] — 2026-10-05

### Added
- **Themes for the UI kernel's look**, beside RapidQ's classic one (still
  the default, byte for byte): `modern` (flat, Windows 11-like: rounded
  controls, an accent colour, focus rings, thin scroll bars), `dark` (the
  same in Windows 11's dark colours) and `highcontrast` (Windows' High
  Contrast Black: 7:1 text, 3-pixel focus rings). `$THEME name` picks one,
  `Application.Theme = "dark"` switches at run time (RapidR's; RapidQ's
  Application has no Theme), `RAPIDR_THEME` sets a default for programs
  that name none, and `auto` follows the system's dark / high-contrast
  setting. A theme never moves or resizes anything, nor changes fonts; the
  program's own colours (`Color`, `Font.Color`) win. The old FLTK theme
  names keep working (classic or modern). Real windows get a light or dark
  title bar to match. The web keeps its own look for now and reads the
  names back.
- GUI test hooks: `RAPIDR_TEST_MESSAGE_DIALOG` (what each message box
  answers, by button caption) and `RAPIDR_TEST_DIALOG_HOLD=ms` (a dialog a
  hook answers stays open, waited for as the user's, before the answer);
  the `dialog_timers` case checks timers during every dialog, native and
  interpreted.
- **Direct3D Retained Mode** (RapidQ's QD3DFRAME, QD3DMESHBUILDER,
  QD3DMESH, QD3DFACE, QD3DLIGHT, QD3DTEXTURE, QD3DVISUAL, QD3DWRAP,
  QD3DVECTOR and QDXSCREEN's 3D methods) on native, interpreted and web
  builds: frames, faces, `.X` models (text and binary — every model of
  RapidQ's examples loads), lights, the camera, textures, wraps, shadows
  and blended transparency, drawn by RapidR's own software rasterizer into
  the screen at Render — lit flat or Gouraud, back faces culled, sharp at
  any display scale.

### Changed
- **QREGISTRY on Windows is Windows' own registry**, as RapidQ's was, in
  native and interpreted builds alike (Microsoft's `windows-registry`
  crate). It answers as the per-user store does — both are worked out in
  one place (`rapidr_value::registry`): the same paths, data types, sizes
  and 1 / 0. Keys a program may only read (HKEY_LOCAL_MACHINE without
  elevation) open and read; changes to them answer 0 and change nothing.
  HKEY_PERFORMANCE_DATA and Windows 9x's HKEY_DYN_DATA aren't keys there.
  `RAPIDR_REGISTRY=<file>` still puts the keys in that file, on Windows
  too, so test runs never touch the machine's registry; macOS, Linux and
  the web keep the per-user store. Checked on Windows 11
  (`tools/windows/registry_check.ps1`: one program interpreted, as an
  interpreted build and as a native build, the same as the store's run,
  what it left checked with `reg query`).
### Fixed
- Interpreted desktop builds run timers' handlers while a dialog waits for
  the user — MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE, MSGBOX, the Open / Save /
  colour / font dialogs, INPUT$, a kernel-drawn pop-up menu — as native
  builds and RapidQ do, instead of in a burst once it closed. Each is a wait
  the interpreter serves between instructions, as ShowModal is, and ends
  with the dialog's answer. A timer's handler may open a dialog of its own
  over another, or close a modal form under a box.
- A timer whose handler waits (a dialog, ShowModal) doesn't fire again
  until that handler returns, and a timer never waits behind another
  handler that waits (two timers due together, a click and a timer): each
  fires once the handler before it has run, or inside its wait —
  interpreted builds now agree with native.
- A dialog shown before the first ShowModal or DOEVENTS starts the
  program's timers, as ShowModal does (RapidQ's timers tick during any
  dialog).
- Linux (Wayland): a window drawn on the GPU no longer stops its program
  while it can't be seen — minimized, on another workspace, the screen
  locked. A FIFO swapchain waited in present for the compositor's frame
  callback, which doesn't come then, and the whole program (its timers
  too) waited with it; on Wayland windows now present in Mailbox mode.
  Found in an Ubuntu VM whose session had locked.
- A computer whose only GPU for vello is a software one draws on the CPU
  even when an OpenGL adapter is also there (an Ubuntu VM: Mesa's software
  Vulkan beside virgl's GL, which vello can't use): only Vulkan, Metal and
  DX12 adapters count.
- QREGISTRY as TRegistry: GetDataType is 0 (unknown) for the registry's
  other kinds of value (REG_MULTI_SZ, REG_QWORD, …; they were binary) and
  the store's file keeps their kind; KeyExists of a root (`""`, `"\"`) is
  always 1; RenameValue puts the value last (TRegistry deletes, then
  writes); MoveKey into the key's own sub-key is refused whatever the case
  of the names.

## [2.115.0] — 2026-10-05

### Added
- RapidR is checked on Windows 11 (ARM) and Linux (Ubuntu 24.04 with
  Wayland) besides macOS: the conformance suite on both backends, the GUI
  events headless and with real windows, the unit tests, and on Windows
  what a screen reader sees (`tools/windows/uia_probe.ps1`; macOS:
  `tools/macos/ax_dump.swift`). The fixes below came from these runs.
- **RapidQ's DirectX 2D objects on every runtime** (native, interpreted,
  web; docs/directx-plan.md): QDXSCREEN draws on an off-screen surface that
  shows on `Flip` (Init, AutoSize, AllowStretch, Fill's DirectDraw colours,
  Pixel, text, Draw / StretchDraw / CopyRect, TextRect, Rotate, View.*;
  OnInitialize / OnInitializeSurface when its form first shows, or once it's
  put on a form already shown; FullScreen covers the screen with the surface
  scaled to fit). QDXIMAGELIST draws DelphiX image libraries (`.DXG`) with
  their transparency and patterns. QDXTIMER keeps FrameRate, fires once a
  frame with Interval 0, and with ActiveOnly only while the program is
  active. Screen text is MS Sans Serif 8, as in RapidQ.
- QDXSOUND (DirectSound) on every runtime: a WAV file played at its
  Frequency, Volume and Pan (DirectSound's decibels), Looped or once;
  Playing, Position and Size; Play, Stop. rodio plays it on the desktop,
  Web Audio in the browser.

### Changed
- The host-neutral half of the desktop's program glue (the kernel's events,
  forms, the modal list, timers, waits, lists, menus, the GUI test script
  and hooks, the dialogs' requests and answers) is a crate of its own,
  `rapidr-ui-app`, behind two traits (`Program`, `Windows`), so the web host
  can share it (docs/web-host-plan.md, W2). Nothing a program sees changed:
  every GUI case's captures, accessibility trees and dumps are byte-for-byte
  the same.
- The test runners delete each native build once its program ran
  (`tests/cargo_builds.mjs`): a full run left 50–100 GB behind.

### Fixed
- macOS: a borderless form (BorderStyle bsNone) no longer can freeze its
  program. Asking whether such a window was maximized gave it a title bar
  for a moment, which resized it, which asked again; it is now never asked
  (the user can't maximize it).
- Native builds: `Obj.Sub.Method(…)` (`DX.View.SetFront(10)`,
  `Printer.Font.DelStyles(3)`) calls the sub-object's method by its
  combined name on the object, as the interpreter does; it went to a value
  read from the object instead.
- Windows: programs built with `rapidr build` are `.exe` files. An
  interpreted build was written without the extension, and a native
  build's copy next to the source was silently skipped.
- Windows on ARM builds without clang: RHTTP's TLS is Windows' own
  (SChannel, kept current by Windows Update); elsewhere it stays rustls.
  `ring`, rustls' crypto, needs clang to build for Windows on ARM.
- Windows: native builds work when RapidR lives under a Windows path (the
  generated Cargo.toml's paths are escaped; `\U…` was read as an escape).
- Windows: QDIRTREE's Directory is a plain path (`C:\Users\…`), not the
  `\\?\C:\…` form `canonicalize` gives — the tree never found its
  directory under its root (`C:\`), so nothing was selected.
- Windows: INPUT$ from a pipe or a file reads its characters instead of
  waiting for the console's keys.
- Screen readers hear labels: a QLABEL's caption (and a status bar's
  text) was given to AccessKit as a name, which it reads from the value
  for labels — Narrator / NVDA (UI Automation) and VoiceOver got empty
  text. Checked on Windows 11 with UI Automation (names, values, toggles,
  ranges, Invoke / Toggle / SetValue running the handlers) and on macOS.
- Computers whose only GPU is a software one (Windows' WARP, Mesa's
  llvmpipe: virtual machines, remote desktops) draw on the CPU: WARP
  crashed in vello's shaders (an access violation in d3d10warp.dll), and
  vello_cpu is faster there anyway. `RAPIDR_RENDERER=gpu` still forces the
  GPU.
- Wayland: a size the window system applies at once (the status bar's
  size grip, a program setting Width / Height) is a resize the program
  hears about (OnResize, Width / Height).

## [2.114.0] — 2026-10-04

### Changed
- **FLTK is gone: the UI kernel is RapidR's only desktop host**, with no
  fallback. `RAPIDR_HOST` and `RAPIDR_THEME` are no longer read
  (`$THEME` stays and draws the classic look); `rapidr build --host kernel`
  only notes it's no longer needed, `--host fltk` is an error. Smaller
  executables (the CLI −2.8 MB, every interpreted program −2.1 MB) and
  CMake is no longer needed to build.
- RSQLITE is SQLite itself on every runtime: the web runs SQLite compiled
  to wasm (built with the desktop's options) instead of a hand-written
  imitation, and opens `.db` assets with SQLite's own reader. A query
  returns rows when the statement has columns (`WITH … SELECT`, `EXPLAIN`,
  `INSERT … RETURNING`); several statements in one Query all run.
  OnConnect, OnQueryDone, OnDisconnect and OnError fire on every runtime;
  QueryScalar works everywhere.

### Added
- SQL parameter binding (SEC-07): `DB.Query(sql, value1, value2, …)`
  binds values to `?` placeholders (arrays are flattened), `AddParam` /
  `ClearParams` queue them — RSQLITE on every runtime, RMYSQL on the
  desktop. Bound values can't change the statement (no SQL injection).
- The web host plan (`docs/web-host-plan.md`) and its spike: the UI kernel
  drawn in the browser on a canvas, byte-identical to the desktop, with an
  ARIA mirror for screen readers; the drawing code shared by both hosts
  (`rapidr-ui-render`).
- `tools/linux/check.sh`: the committed tree built and tested on Ubuntu in
  Docker, headless and on X11 windows — no remote CI needed.

### Fixed
- Timers keep firing, and their changes are drawn, while a native menu is
  held open (context menus and the macOS menu bar), as on Windows; no CPU
  spin while a menu is open.
- A console program never starts the windowing system (it no longer needs
  a display on Linux servers, nor becomes a GUI app on macOS); without a
  display, programs run on without showing windows.
- Screen readers get a window's contents on their first question (macOS).
- macOS: no size grip drawn in the window's rounded corner (dragging there
  still resizes).
- Web: RowSeek was one row off. Desktop: only the first of several
  statements in a Query ran.

## [2.113.0] — 2026-10-04

### Added
- The IDE written in RapidR (`examples/ide.rr`) runs on the UI kernel host:
  RDESIGNSURFACE and RCODEEDITOR are shared models (FLTK, the kernel and the
  web answer from them), the code editor is the kernel's editor with BASIC
  colours and a line-number gutter, and the editor can carry styled runs
  (colour, bold, italic, underline, font) for QRICHEDIT's next step.
- Screen readers on the web: ARIA roles, names, values and states from the
  same accessibility rules as the desktop kernel (lists, trees, grids, tabs,
  menus, status bar panels), a polite live status bar, Tab order by
  TabOrder / TabStop, Alt + `&` letters, Enter → Default, Escape → Cancel.
  `tests/web_a11y.mjs` compares the browser's tree with the kernel's for
  every fixture.
- MESSAGEBOX / MESSAGEDLG show their type icons (warning, error,
  information, question) with Delphi's layout and captions on FLTK, the
  kernel and the web; MESSAGEBOX beeps on the desktop.
- QCOLORDIALOG: Style, `Colors(1 TO 16)` and the custom colour editor.
  QFONTDIALOG: Name / Size / Color, styles and effects, Min / MaxFontSize,
  options, OnApply, GetFont / SetFont.
- QFORM.WindowState (normal, minimized, maximized) on all three runtimes.
- Kernel host: double clicks in the VCL's order (also fixed on FLTK and the
  web), the real editor for tree / list view / grid in-place edits, edit
  after a pause on the selected item, F10 / Alt for the menu bar, drag
  selection by words and paragraphs with auto-scroll, QSTATUSBAR's size
  grip (also on FLTK), the wheel on grids and list views.

### Fixed
- `Obj.Member` without parentheses: the interpreter and the web read a
  property where RapidQ calls a method (`WHILE MySQL.FetchRow` never looped),
  and native builds called a method where it's a property (`UpDown.Max`,
  `ProgressBar.Max`, `ScrollBar.Min / Max`, `ListBox.Columns` read nothing).
  Which one it is now depends on the object's type, the same everywhere.
- Web: Checked reaches check boxes and radio buttons; ReadOnly edits are
  read-only; controls in a disabled panel are disabled; a group box's
  caption set later keeps its components.
- FLTK: OnClick / OnDblClick for panels, labels, group boxes, scroll boxes
  and forms; a designed component's colour from a QCOLORDIALOG; code editor
  colours after non-ASCII text.
- The CPU renderer's synthetic italics lean forward.

### Changed
- MySQL client 28 without derive, TLS or system zlib (drops the
  unmaintained proc-macro-error2).

## [2.112.0] — 2026-10-04

### Added
- The UI kernel host (`RAPIDR_HOST=kernel`) now runs every desktop GUI test
  program identically to FLTK (native and interpreted, 1× and 2×):
  - drawing surfaces — QCANVAS, a form's own drawing, QIMAGE (BMP, $RESOURCE,
    SVG, icons), speed buttons' glyphs — sharp at 2×;
  - text — the kernel's own editor for QEDIT (PasswordChar, Alignment,
    HideSelection, MaxLength, CharCase, word / all selection, undo, the
    Cut / Copy / Paste menu), QMEMO and QRICHEDIT (plain text, word wrap,
    scroll bars), the editable QCOMBOBOX box; the mouse wheel; IME;
  - dialogs — MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE drawn by the kernel,
    the system's Open / Save dialogs (as sheets on macOS), colour and font
    dialogs;
  - the platform — window frames from BorderStyle / BorderIcons, cursors
    (Screen.Cursor, each component's, I-beams, resize arrows), the mouse on
    the screen, the work area.

### Fixed
- Desktop: QCOLORDIALOG's Color is RapidQ's integer (`&HBBGGRR`), not a
  `"#RRGGBB"` string.
- "MS Sans Serif" / "Microsoft Sans Serif" are drawn and measured with the
  sans face.

## [2.111.0] — 2026-10-04

### Added
- The UI kernel host (`RAPIDR_HOST=kernel`) draws and drives most of
  RapidQ's components, the same as on FLTK (native and interpreted, 1× and
  2×): panels, group boxes, scroll boxes and a form's scroll bars, splitters,
  status bars, MDI child windows; check boxes, radio buttons, cool / oval
  buttons, gauges and progress bars, up-downs; the in-window main menu and
  pop-up menus (the macOS menu bar natively); list and combo boxes (owner
  drawn too), list views, string grids (OnDrawCell, editing, drop-down
  columns), headers, tree views and outlines, directory trees and file
  lists. 43 of 54 GUI test programs now run identically on both hosts.

### Fixed
- Kernel host: a Color the program sets is painted, even white.

## [2.110.0] — 2026-10-04

### Added
- Desktop host Stage 3: `crates/rapidr-ui-host-winit` — RapidR's own
  desktop host, next to FLTK (`RAPIDR_HOST=kernel`; FLTK stays the
  default): winit driven by the program's own loop, vello on the GPU or
  vello_cpu on the CPU (`RAPIDR_RENDERER=cpu|gpu`, falling back to the CPU
  by itself), a screen-reader tree per window (AccessKit), a headless host
  for tests. ShowModal (nested, native and interpreted), DoEvents, timers,
  OnShow / OnLoad / first OnPaint / OnClose / OnResize, the test hooks, and
  `RAPIDR_TEST_A11Y` (each form's accessibility tree as JSON).
  `rapidr build --host kernel` builds a native program with both hosts.
  16 GUI test programs already run on it, natively and interpreted, with
  the same results as on FLTK (`tools/regress.sh` runs the hosts matrix).

## [2.109.0] — 2026-10-04

### Added
- Desktop host Stage 1: the runtime reaches the desktop GUI only through
  `rapidr-runtime-core`'s `ui` facade (63 functions + the platform's), with
  a host switch (`RAPIDR_HOST=fltk|kernel` once both are built), a safety
  net that queues program handlers fired inside a window callback, and the
  test hooks' parsing shared by both hosts — FLTK's behaviour unchanged.
- Desktop host Stage 2: `crates/rapidr-ui-kernel`, the GUI-free UI kernel
  (forms built from the component store, focus with TabOrder / TabStop,
  input routing to events in RapidQ's order, display lists, parley text at
  RapidQ's sizes without kerning, label / button / track bar / tab control /
  single-line edit with IME, an accessibility tree); it builds for the web
  too. Shared drawing ops (`rapidr_value::objects::ops`) and accessibility
  nodes (`objects::a11y`) for every runtime.

## [2.108.0] — 2026-10-04

### Changed
- wasm-bindgen 0.2.129 (js-sys / web-sys 0.3.106) across the workspace and
  in generated web projects — install the matching CLI with
  `cargo install -f wasm-bindgen-cli --version 0.2.129`. It's what the new
  desktop host's wgpu needs, so both can share one workspace.
- `crates/rapidr-ui-proto`: the desktop host's Stage 0 — a program's loop
  driving winit (`pump_app_events`) with nested modals, timers and live
  resize; a CPU renderer (vello_cpu) and a headless host for tests.

### Fixed
- Web: a component's FontSize is points, as on the desktop and in RapidQ
  (12 is 16 pixels; it was 12 pixels). Font sizes become whole pixels the
  way Windows rounds them (10 pt is 13 pixels) everywhere text is drawn or
  measured — TextWidth / TextHeight measured at 13.33 pixels while text was
  drawn at 13.

## [2.107.0] — 2026-10-04

### Added
- `docs/desktop-host-plan.md`: the staged plan for moving the desktop from
  FLTK to RapidR's own UI kernel (winit + vello + parley + AccessKit).
- `Anchors` and `Constraints` on every visual component (RapidR's, as in
  Delphi; RapidQ had Align only): `Anchors = akLeft + akRight` keeps a
  control's distances to its parent's edges as the parent resizes
  (stretching with both, keeping its centre with neither); `MinWidth` /
  `MinHeight` / `MaxWidth` / `MaxHeight` (also as `Constraints.MinWidth`)
  bound every size it takes — set by the program, by Align, by Anchors, or
  by the user (a form can't be dragged smaller). One model in
  `rapidr_value::layout`, the same on every runtime; `akLeft` … `akBottom`
  need no include, and a program's own names win.

### Fixed
- Web: a text box narrower than 8 pixels keeps the width the program gives
  it.

## [2.106.0] — 2026-10-04

### Added
- Web: the VM runs in time slices (~10 ms): a busy loop — `cpuhog: GOTO
  cpuhog`, a game loop without DoEvents, a long computation — no longer
  freezes the page, what it draws and prints shows as it runs, and the
  IDE's Stop always works; events still run only when the program waits
  (DoEvents, ShowModal, a dialog, the end of its main code), as on the
  desktop. DoEvents and an empty INKEY$ no longer cost 4 ms each. The
  desktop interpreter is unchanged (the check is compiled out there).
- `tools/regress.sh` runs the desktop GUI events at 2× too (`RAPIDR_SCALE=2`).
- `Screen.Scale` and `Form.Scale` (RapidR's): device pixels per pixel (2 on
  a Retina screen, 1.5 at 150 %) — programs keep RapidQ's pixels, this
  tells them how fine the screen is; `Screen.PixelsPerInch` reads RapidQ's
  96. `OnScaleChanged` (RapidR's): a form moved to a screen with another
  scale is told, then drawn again (OnPaint).

### Fixed
- A QFORM's Visible is RapidQ's: False until the form shows, then whether
  it shows (it read True from the start); `Form.Visible = True` shows the
  form with its OnShow on the desktop too (it did nothing for a form not
  shown yet) — inside the form's own CREATE as well, where the form shows
  once the program waits (Splitter.bas' "required" `Visible = 1`), as in
  the browser.

## [2.105.0] — 2026-10-03

### Added
- `crates/rapidr-ui-proto`: a prototype of RapidR's own desktop host on
  winit + wgpu/vello + parley + AccessKit, with muda / rfd / arboard for the
  menu bar, dialogs and clipboard — a QFORM with QLABEL, QBUTTON, QEDIT,
  QTRACKBAR and QTABCONTROL drawn from the shared models at the screen's
  scale, keyboard focus, a screen-reader tree, an offscreen capture. Its own
  lockfile, outside the workspace (wgpu needs a newer wasm-bindgen than the
  web build's); every dependency permissive (MIT / Apache-2.0 / BSD / Zlib).
- `tests/corpus_web_compare.mjs`: RapidQ's portable example programs run in
  the browser (the web IDE's compiler and VM) and on the desktop
  interpreter side by side; what they print and every form's and
  component's properties, as the program reads them, are compared. The
  browser gets the desktop's screen size, and a program that never yields
  to the page is reported, not waited for.
- `RAPIDR_MENU=window`: a QMAINMENU is a bar inside its form on macOS too,
  as on Windows, Linux and in the browser (the same ClientHeight
  everywhere); without it, macOS keeps the system menu bar.

- A component type's name used as a value is the newest object of that
  type, as in RapidQ: `Parent = QFORM` in a toolbar or main-menu include puts
  it on the form created last (RapidQ's ToolTest example), and in a TYPE's
  constructor `Parent = QFORM` is the newest form.

### Fixed
Found by running the portable corpus in the browser against the desktop:
- Web: OnShow fires — on each ShowModal and when Show shows a hidden form,
  as on the desktop (it never fired in the browser); the desktop now fires
  it too when a form is shown again after Hide / Close.
- Desktop: SHOWMESSAGE is RapidQ's message box with an OK button, titled
  with Application.Title (it only printed the message); under a test's
  hooks it still prints and goes on.
- Web: `$RESOURCE` files are found in any case (`BACK1.BMP` for back1.bmp),
  as on Windows.
- Web: only the forms a program shows appear (Show, ShowModal,
  Visible = True), as on the desktop and in RapidQ, where a form starts
  hidden; a dialog form created at start no longer shows over the main one.
- Web: FILEEXISTS finds the program's files (the project's assets and the
  files it saved), and files are read from the project's assets, with
  Windows' rules (any case, `\` or `/`) — a program loading its
  bitmaps or data files works in the browser (the VECTOR font demo, the
  file splitter's tiled background).
- Web: a component without a Parent isn't shown (it went into the first
  form), as on the desktop.
- Web: a button, edit or combo box is exactly the size the program gives it:
  the theme's padding no longer widens narrow ones (a 23-pixel `...`
  button was 30).
- The main menu bar is the same height (28) in the browser and on the
  desktop (it was 30 on Windows and Linux).
- A component's Left / Top / Width / Height are integers, as in RapidQ:
  `Height = ClientHeight / 2 + 20` stores 304, rounded half to even (the
  desktop kept 304.5).

## [2.104.0] — 2026-10-03

### Fixed
Found by running RapidQ's portable example programs on the native build and
the interpreter side by side (`tools/corpus_compare.mjs`):
- `REPLACE$(source, replacement, index)` is RapidQ's: it writes over the
  string at a position (`REPLACE$("Hello", "J", 1)` is "Jello"); it was
  Visual Basic's find-and-replace (that's REPLACESUBSTR$). RapidQ's PRINT
  USING formatter example and the Blocks game depend on it.
- Variables whose names differ only by type suffix are different variables,
  as in RapidQ (`i%` and `i$` side by side); a variable spelled like a
  function with another suffix (`day&` in `FUNCTION Day`) is not the
  function's result.
- A FOR loop's undeclared variable inside a SUB or FUNCTION is the
  routine's own on native builds too, as it was in the interpreter: two
  SUBs looping on `i` no longer move each other's counters (the Blocks game
  hung natively).
- A form's first OnPaint comes once its window shows (after OnShow, as
  Windows' WM_PAINT), when the screen's scale is known: what it draws is
  sharp on a high-DPI screen and the same on native and interpreted builds
  (text and lines drawn before were enlarged from 1×).
- `tools/corpus_compare.mjs` counts programs that wait with SLEEP as
  clock-dependent, and never touches the user's QREGISTRY store.

Portable corpus, native vs interpreted: 95 behave identically, 0 differ
otherwise; the rest depend on random numbers or the clock (12) or use the
network (16, not run).

## [2.103.0] — 2026-10-03

### Added
- RapidQ's POSTFIX (RPN) expressions from its manual (Appendix C): every
  operand and operator in its own parentheses, `(4) (7) (*) (4) (1) (-) (6)
  (^) (+)` = 757.
- `a NOT > b` (and `NOT <`, `NOT >=`, `NOT <=`) as RapidQ reads it — NOT
  binds looser than a comparison, so it's `NOT (a > b)` — as `a NOT= b`
  already was.
- A trailing comma leaves the last argument out (`AddItems " ",`); an
  argument left out adds no item to a list.
- `tools/rapidq_corpus.py` sorts RapidQ's examples by what they need
  (portable / Windows DLLs / DirectX / OLE / hardware ports / incomplete /
  not RapidQ): every portable one compiles.

### Changed
- Text after the end of a statement is reported in RapidQ's words:
  "Expected end-of-line but got …".

### Fixed
- `IF x THEN CALL Sub(1, 2): x = 0` and `… THEN CALL A ELSE CALL B`: a CALL's
  arguments end at `:` and ELSE.

## [2.102.0] — 2026-10-03

### Changed
- Components start with RapidQ's sizes, the same on every runtime (one
  table, `rapidr_value::layout::default_size`): QFORM 320 × 240, QBUTTON
  and QLABEL 75 × 25, QEDIT 120 × 25, QCOMBOBOX 145 × 25, QPANEL /
  QTABCONTROL / QGROUPBOX / QSCROLLBOX / QSTRINGGRID / QLISTVIEW 150 × 100,
  QCHECKBOX / QRADIOBUTTON 100 × 20, QCANVAS / QIMAGE 100 × 100,
  QPROGRESSBAR 250 × 25, … The desktop and the web had their own (a button
  was 80 × 25 on one, 100 × 30 on the other; a form 640 × 480).

## [2.101.0] — 2026-10-01

### Added
- QREGISTRY as RapidQ's manual has it (its own example runs as shown):
  `RootKey` (HKEY_CURRENT_USER by default), `OpenKey(Key, CanCreate)`,
  `CloseKey`, `CreateKey`, `DeleteKey` (with its sub-keys), `KeyExists`,
  `ValueExists`, `DeleteValue`, `RenameValue`, `MoveKey(Old, New, Delete)`,
  `KeyItem(i)` / `ValueItem(i)`, `KeyItemCount`, `ValueItemCount`,
  `HasSubKeys`, `CurrentKey`, `CurrentPath`, `GetDataType` / `GetDataSize`,
  `ReadString` / `ReadInteger` / `ReadFloat` / `ReadBinary` and their
  `Write…` twins. Paths are relative to the open key (`\` first: from the
  root), names ignore case, ReadBinary keeps RapidQ's index-from-−1 bug. The
  keys live in a per-user store, the same on every platform and nothing
  outside it touched: a Regedit-format text file in the user's settings
  folder (`RAPIDR_REGISTRY` names another), the page's local storage on the
  web. Native, interpreter and web.
- RapidQ's "Array of QREGISTRY is not supported!" (and the same for its other
  objects with no arrays: QMainMenu, QFileStream, QSOCKET, QRECT, …).

## [2.100.0] — 2026-10-01

### Added
- QFORM and QSCROLLBOX scroll as RapidQ's (Delphi's TScrollingWinControl),
  from one shared model (rapidr_value::scrollbars) that the desktop and the
  web draw and drive the same way: `AutoScroll` (on by default: a bar shows
  when the components reach past the client area), `HorzRange` /
  `VertRange`, `HorzPosition` / `VertPosition`, `HorzIncrement` /
  `VertIncrement` (8), `HorzMargin` / `VertMargin`, `HorzTracking` /
  `VertTracking`, `HorzVisible` / `VertVisible`. Scrolling moves the
  components (their Left / Top change, as Delphi's ScrollBy); a shown bar
  takes 17 pixels from ClientWidth / ClientHeight; an aligned component
  fills the scrolled area. The arrows move Increment pixels and repeat while
  held, a click in the track a page, the thumb drags (the components follow
  it with Tracking, else when it's let go), the mouse wheel scrolls; a click
  on a bar fires no OnMouseDown. A QSCROLLBOX has its sunken edge
  (bsSingle) with its components inside it. A QFORMMDI doesn't scroll its
  own components (its children's MDI area is Windows' own). Native,
  interpreter and web.

### Fixed
- Native builds named components after Rust keywords differently from the
  interpreter (`Box` was `box_`), so anything naming them (test hooks, a
  component found by name) missed them.
- The web showed the browser's own scroll bars on a form whose components
  didn't fit (the desktop none), and a QSCROLLBOX scrolled with the
  browser's / FLTK's bars without HorzPosition, VertPosition or the ranges.

## [2.99.0] — 2026-10-01

### Added
- QTABCONTROL as RapidQ's (Windows' tab control): a row of tabs over an
  area the program fills itself (no pages: OnChange shows and hides its
  components, as the manual's example does), from one shared model
  (rapidr_value::objects::tabcontrol) that the desktop and the web lay out
  and draw the same way — the tabs measured with the built-in fonts.
  `AddTabs`, `InsertTab(Index, Caption)`, `DelTabs(Index…)`, `Tab(i)` (read
  and set), `TabIndex`, `MultiLine` (rows, the selected one moved next to
  the area), `ScrollOpposite` (the rows before it moved to the other side),
  `TabPosition` (bottom / right), `VerticalTabs` (on the left or right,
  their captions turned), `ButtonStyle`, `FlatButtons`, `FlatSeperators`,
  `FocusButtons`, `HotTrack`, `TabWidth` / `TabHeight`, `TabInactiveColor`,
  `TabInactiveFont`, `Color`; a single line too long scrolls with its arrow
  buttons. Windows' rules: the first tab added is selected, a tab inserted
  before the selected one keeps it selected, deleting the selected tab
  selects none (-1); the program setting TabIndex doesn't fire OnChange, a
  click on a tab or the arrow keys do. ClientWidth / ClientHeight are the
  whole control; an aligned component fills the area inside the frame
  (TCM_ADJUSTRECT). Native, interpreter and web.

### Fixed
- QTABCONTROL's components were put on FLTK pages (one per tab) on the
  desktop and its tabs were plain buttons on the web; TabIndex, Tab(i),
  InsertTab, DelTabs and OnChange from the keyboard were missing.

## [2.98.0] — 2026-10-01

### Added
- QTRACKBAR as RapidQ's manual has it, from one shared model
  (rapidr_value::objects::trackbar) that the desktop and the web draw the
  same way (vector shapes, sharp at any scale): `Frequency`, `LineSize`,
  `PageSize`, `Orientation` (tbHorizontal / tbVertical), `TickMarks`
  (tmBottomRight / tmTopLeft / tmBoth, the thumb pointing at them),
  `TickStyle` (tsNone / tsAuto / tsManual), `SelStart` / `SelEnd` (the
  selection range shown in the channel) and `SetTick(Pos)`. The arrows move
  it by LineSize, Page Up / Page Down by PageSize, Home / End to the ends; a
  click beside the thumb moves it a page toward the click, and the thumb
  drags. OnChange fires when the user moves it. Native, interpreter and web.

### Fixed
- QTRACKBAR's defaults are RapidQ's (Max 10, PageSize 2, Frequency 1,
  150 × 45), not 0–100; Position stays within Min..Max; setting Position
  from the program now moves the thumb on the desktop.

## [2.97.0] — 2026-10-01

### Added
- QEDIT and QRICHEDIT keep their text and selection in one shared model
  (rapidr_value::objects::textedit), as RapidQ: `SelStart`, `SelLength`,
  `SelText` (read, and set to replace the selection), `Line(i)` (read and
  set), `LineCount`, `AddStrings` / `AddLines`, `Modified`, `WhereX` /
  `WhereY`, `SelectAll`, `ClearSelection`, `CopyToClipboard` /
  `CutToClipboard` / `PasteFromClipboard`, `ReadOnly`, `CharCase`. A
  multi-line edit's Text joins its lines with CR LF, as on Windows. What
  the user typed and selected is read from the widget before the program
  reads it, and the model is shown again after the program changes it.
  Native, interpreter and web.

### Fixed
- A QRICHEDIT read before its form showed (`Text`, `Line(i)`,
  `LineCount`) was empty.

## [2.96.0] — 2026-10-01

### Added
- QSTRINGLIST as RapidQ's manual has it (its own examples run as shown):
  `AddList(Other)`, `Parse(Source$, Delim$)` (the list becomes its pieces;
  returns how many), `Build(Start%, End%, Delim$)`, `Exchange(I%, J%)`, and
  `Duplicates` for a sorted list (dupIgnore, the default: a string already
  there isn't added again; dupAccept; dupError). Native, interpreter and
  web.

## [2.95.0] — 2026-10-01

### Added
- `ShowModal` returns the form's `ModalResult`, as RapidQ: setting it (to
  mrOk 1 … mrAll 8) closes the modal form; a button's `ModalResult` gives
  it to its form when clicked; closing the form otherwise gives mrCancel
  (2). QBUTTON `Kind` (bkOK, bkCancel, bkYes, bkNo, bkAbort, bkRetry,
  bkIgnore, bkAll, bkHelp, bkClose): its standard caption and ModalResult
  (bkClose closes the form). Native, interpreter and web.

### Changed
- Key events as RapidQ (Delphi) sends them: a form hears the keys typed in
  its controls only with `KeyPreview` on — and then before the control
  (they used to reach the form after the control, always). Within a key,
  every OnKeyDown comes before the OnKeyPresses. Desktop and web.

## [2.94.0] — 2026-10-01

### Added
- Menus as RapidQ has them, from one model for the desktop and the web
  (`rapidr_value::objects::menu`): QMENUITEM `ShortCut` ("Ctrl+N", "F2",
  "Shift+Del", … — the key works and shows in the menu), `Checked`,
  `RadioItem` (checking one unchecks the others), `Enabled` (greyed),
  `Visible`, `Hint`, `Command`, `MenuIndex` (setting it moves the item),
  `Count`; `AddItems` / `Insert` / `DelItems` / `DelIndex` on menus and
  items; separators ("-"); submenus at any depth. Every change shows at
  once (the menus used to be built once, without shortcuts or check marks).
  QPOPUPMENU `Popup(X, Y)` (screen coordinates), `OnPopup`, `Alignment`,
  and `AutoPopup`: a right click on a component whose `PopupMenu` is that
  menu. Native, interpreter and web.

## [2.93.0] — 2026-10-01

### Added
- RapidQ's QSOCKET as its manual has it: numbered sockets — `Sock% =
  S.Connect(Server$, Port%)`, `S.Open(Port%)` (a server), `ConnectionReady`,
  `Accept`, `IsServerReady` / `IsClientReady` (data waiting, without
  blocking), `Read` / `Peek` / `ReadByte` / `ReadLine`, `Write` /
  `WriteByte` / `WriteLine` (CR LF), `Close`, `GetPeerName`, `GetHostName`,
  `GetHostIP`, `MySocket` and `Transferred`. RapidR's own RSOCKET methods
  (Host / Port properties) are unchanged: the two are told apart by their
  arguments. Native and interpreter; a web page can't open TCP sockets, so
  there the calls fail as RapidQ reports it (-1, nothing ready).
- QFORM `AddBorderIcons` / `DelBorderIcons` (biSystemMenu, biMinimize,
  biMaximize, biHelp): the web greys out the title bar's buttons; the
  desktop keeps the set (its title bar is the system's — RapidQ's manual
  allows an icon to stay, greyed out).

## [2.92.0] — 2026-10-01

### Changed
- Arrays as RapidQ has them ("There is no checking for limits on arrays"):
  an index past one dimension is the next row's element (arrays are stored
  by the last subscript), and past the whole array a read gives the type's
  zero and a write is dropped — memory-safe, where RapidQ would touch other
  memory. Programs from RapidQ's examples that stopped with "Subscript out
  of range" (3dcube, COLUMNS, dayfunction) run.

### Fixed
- QSTRINGGRID's OnDrawCell fires when the grid is on screen, as in RapidQ,
  not while its form is still hidden (it ran before the program created
  what the handler reads: QStringGridsTwoLinesBitMap). Desktop and web.
- macOS: a form with a main menu shown after another form closed crashed in
  FLTK's "Window" menu (msweep.bas); RapidR's menu bar has no Window menu
  (RapidQ has none).
- Tests never reach a real printer: `tools/regress.sh` sends documents to
  PDFs, and the test calling every shared built-in (LFLUSH among them)
  prints nowhere.

## [2.91.0] — 2026-10-01

### Added
- `LPRINT` (as PRINT, `;` and `,` included) and `LFLUSH`, as RapidQ: the
  text goes on printer pages (A4, 10 pt) printed at LFLUSH, or when the
  program ends — through the same printing as `Printer.EndDoc` (the
  system's printer on the desktop, the print dialog on the web,
  `RAPIDR_PRINT_TO` a folder or file). Native, interpreter and web.
- Native builds: a line label or GOSUB inside WITH or CREATE (it used to be
  a build error there; the interpreter already had it).

### Fixed
- Inside a CREATE, a property read bare is the object's, as in RapidQ:
  `Left = (Screen.Width - Width) \ 2`, `PRINT ItemCount` (they read an
  empty variable before) — unless the program has a variable, constant,
  routine or component of that name. Native, interpreter and web.
- An undeclared variable that is only read is RapidQ's DOUBLE 0 (`PRINT zz`
  printed nothing). RapidQ's include folder: 81 of 108 libraries compile.

## [2.90.0] — 2026-10-01

### Added
- RapidQ built-ins that were missing (the corpus now compiles 158 of 386): `QUICKSORT(A(first), A(last),
  ASCEND|DESCEND)` (sorts that range of the array in place, numbers by
  value, strings by character; with or without parentheses), `PRINT TAB(n)`,
  `ATAN` (ATN), `GET$(n)` (bytes from standard input, for CGI programs),
  `SETCONSOLETITLE` (the terminal's title; the page's on the web) and
  `CHDRIVE` (Windows). Native, interpreter and web.

### Fixed
- Native builds call the program's own FUNCTION or SUB named like a
  built-in (`FUNCTION Get$`), as RapidQ and the interpreter do.

## [2.89.0] — 2026-10-01

### Added
- RapidQ syntax from its examples (the corpus now compiles 155 of 386):
  `STRUCT name … END STRUCT`, a user-defined type as TYPE; `""` inside a
  string is a quote (`"[:"":>"` is `[:":>`); BYTE / WORD / DWORD suffixes
  on number literals (`0??`); a `_` line continuation inside a string under
  `$ESCAPECHARS ON` (`"Accept: _` … on the next line).

## [2.88.0] — 2026-10-01

### Changed
- INKEY$'s extended keys (arrows, Home/End, Page Up/Down, Insert/Delete,
  F1–F12) are CHR$(27) + the QBasic scan code, as RapidQ's manual has them
  (chapter 6.5: "the first byte is an ESC character"), no longer QBasic's
  CHR$(0) + the scan code. Up is still `RIGHT$(k, 1) = "H"`.

### Added
- `$OPTION INKEY$ TRAPALL` / `$OPTION INKEY$ DEFAULT`: INKEY$ also returns
  Shift, Ctrl, Alt, Caps / Num / Scroll Lock and the menu key (CHR$(27) +
  their scan code: 42, 29, 56, 58, 69, 70, 93), switched on and off
  anywhere — in the program's windows and the page (a terminal can't tell
  these keys). Native, interpreter and web.

## [2.87.0] — 2026-10-01

### Changed
- A property RapidQ's manual lists as read-only (`Handle`, `ItemCount`,
  `SelCount`, `ColumnsCount`, `VisibleRowCount`, a stream's `EOF` / `Size`
  / `LineCount`, …) can't be assigned, in RapidQ's words: `Property
  ItemCount of List is read-only.` — by name or inside its CREATE (the
  assignment used to be ignored). None of the 386 RapidQ examples or its
  include libraries trips it. Native, interpreter and web.

## [2.86.0] — 2026-09-30

### Changed
- More of RapidQ's compile-time checks, in its compiler's words (they used
  to pass silently: extra arguments were dropped, missing ones empty, a
  second DIM reset the variable):
  `Too many actual parameters for S`, `Too few parameters for S` (a
  `DECLARE SUB`'s own signature is accepted too, as RapidQ does),
  `Identifier a already used, try another name` (`i%` and `i$` are two
  names), `Trying to assign return value while not in FUNCTION`. None of
  the 386 RapidQ examples or its include libraries trips them. Native,
  interpreter and web.

## [2.85.0] — 2026-09-30

### Added
- `$TYPECHECK ON` / `$TYPECHECK OFF` and `$OPTION EXPLICIT`, as RapidQ: while
  on, a variable stored into (`x = …`, `FOR x`, `INPUT x`) must be declared
  first (DIM, CONST, a parameter; `n%` is the DIMmed `n`), or it is RapidQ's
  `Undeclared identifier x`. It can be switched on and off around parts of
  a program. None of the 159 RapidQ examples that use it, nor its include
  libraries, trips it. Native, interpreter and web.

## [2.84.0] — 2026-09-30

### Changed
- Undeclared variables are DOUBLE, as in RapidQ (a variable without a
  suffix or declaration is a DOUBLE there), no longer
  VARIANT: `n = 7 / 2` is 3.5 as before, but a string can't go into one.
  `$OPTION DIM VARIANT` brings back the old behavior; `DIM v` without AS is
  still a VARIANT (RapidQ manual, DIM).
- RapidQ's compile-time type check: a string stored into a numeric
  variable (`DIM n AS LONG : n = "12"`, or an undeclared `x = "hi"`) is an
  error, in RapidQ's words: `Type mismatch, expecting type LONG, but got
  STRING` (it used to convert silently). VAL converts. None of the 386
  RapidQ examples trips it. Native, interpreter and web.

## [2.83.0] — 2026-09-30

### Added
- `INPUT$(n)`: waits for n keys (not echoed) and returns them the moment
  the n-th is pressed, as RapidQ — no polling: a console program sleeps in
  the terminal (or reads n characters from a pipe, returning what came if
  it ends first), a program with windows serves its events (timers,
  repaints) until the keys come, and the web suspends the program until the
  page's keydown. Native, interpreter and web.

## [2.82.0] — 2026-09-30

### Added
- RapidQ syntax from the example corpus (now 153 of 386 compile):
  a keyword as a variable declared by DEFSTR/DEFINT/… (`DEFSTR return`,
  qcgi.inc; `DIM step` stays an error); an array
  field without bounds (`Hint() AS STRING`, QtoolBar.inc: room for 0–255,
  REDIM for another size).
- `$OPTION DIM type` (BYTE … STRING, VARIANT): variables the program never
  declares, and a DIM without AS, are of that type; `$OPTION DECIMAL ","`
  (or a character code) is the decimal character VAL reads from then on.
  Native, interpreter and web.

### Fixed
- Inside a component's CREATE, its own indexed members (`Panel(0).Width`
  of a QSTATUSBAR, a QLISTVIEW's `Column(i)`, …) are the object's even when
  the program has an array of that name.

## [2.81.0] — 2026-09-30

### Added
- Templates (manual 10.8) on both backends: `TYPE Holder<DataType, Size>`
  … `END TYPE` and `DIM A AS Holder<INTEGER, 10>` — a TYPE made for each
  set of arguments, the parameters replaced in its fields' types and sizes
  and in its code (parameters and FUNCTIONs taking one too).
  QStringGridEx.inc compiles now.
- `Obj.Inherit<Event>` (manual 10.4): a TYPE's own `EVENT OnClick`, after
  the program gave the object its own handler, runs from that handler
  (`C.InheritOnClick`).

### Fixed
- Custom events (manual 10.9): a SUB given to an `AS EVENT(Template)`
  field is stored as its pointer (`> 0`, fired with CALLFUNC); it was
  taken for a component's event and the field stayed 0.
- Web: a component whose event handler was given twice (a TYPE's EVENT,
  then the program's own) ran it twice per event.

## [2.80.0] — 2026-09-30

### Added
- `INKEY$` on native, interpreter and web — the next key pressed, or ""
  without waiting, as QBasic's: a character, Enter CHR$(13), Escape
  CHR$(27), and the arrows, Home / End / Page Up / Page Down, Insert /
  Delete and F1–F12 as CHR$(0) + their scan code. A console program reads
  its terminal (a key at a time, no echo; INPUT gets line mode back), a GUI
  program the keys pressed in its windows, the web the keys pressed in the
  page (not those typed into fields). It was an unknown variable ("").
- `DOEVENTS` on the desktop lets the program's windows, events and timers
  run (it did nothing); on the web it pauses the program so the browser
  goes on — a `DO … DOEVENTS … LOOP` no longer freezes the page.

### Fixed
- **`SLEEP` counts seconds**, as RapidQ's manual says (`SLEEP 1.5`: one and
  a half seconds); it took milliseconds and dropped fractions, so `SLEEP 1`
  hardly paused. On the web it now really waits (the browser goes on).
- A QTIMER enabled again after being disabled ticks again; its Interval
  is read at every tick; timers run in a DOEVENTS loop too, not only once
  a form is shown modally.

## [2.79.0] — 2026-09-30

### Added
- `$OPTION ICON "app.ico"`: the program's icon, built into it and made
  the application's (every form without its own shows it) on native,
  interpreter and web; an icon that isn't there leaves the default one
  instead of failing the build.
- `$OPTION BYREF`: from that line on, parameters without BYVAL are passed
  by reference (RapidQ's default is BYVAL), on both backends.

## [2.78.0] — 2026-09-30

### Added
- QSTRINGGRID on native, interpreter and web: goColMoving / goRowMoving
  (dragging a fixed row's cell moves its column, a fixed column's cell its
  row — cells, width / height, column style and list with it; the selected
  cell follows), `VisibleRowCount` / `VisibleColCount` (rows / columns shown
  whole), and RapidR's `MoveCol From, To` / `MoveRow From, To`.
- `tools/real_input.py`: `d:x1,y1,x2,y2` drags.

## [2.77.0] — 2026-09-30

### Added
- **QLISTVIEW as RapidQ has it**, on native, interpreter and web alike:
  the control is laid out and painted by the shared model (sharp on
  high-DPI screens) instead of a plain text list / an HTML table.
  - `ViewStyle`: vsIcon (RapidQ's default: large icons with captions under
    them), vsSmallIcon, vsList (columns) and vsReport (rows under a column
    header); `LargeImages` / `SmallImages` / `StateImages`, `ImageIndex` /
    `StateIndex`.
  - `CheckBoxes` (click the box or press Space; `Item(i).Checked`),
    `MultiSelect` (Ctrl / ⌘-click, Shift-click, Ctrl+A; `SelCount`,
    `Selected(i)`), `SortType` stText (kept sorted by caption), `RowSelect`,
    `GridLines`, `HotTrack`, `HideSelection`, `BorderStyle`.
  - The header: its buttons fire OnColumnClick (`ColumnClick`), its edges
    resize the columns; scroll bars, the mouse wheel, and the keyboard
    (arrows, Home / End, Page Up / Down).
  - In-place caption editing when `ReadOnly` is False (as Windows): F2, or
    a click on the item already selected.
  - OnChange (Index, Change: ctText 0 / ctState 2) for each item whose
    selection, check or caption the user changed, then OnClick /
    OnDblClick.

### Changed
- A QLISTVIEW without `ViewStyle` now shows RapidQ's default, the icon
  view (it always looked like a report before); items have no state image
  until `StateIndex` is set (-1).

## [2.76.0] — 2026-09-30

### Added
- JPEG pictures wherever a bitmap goes (QBITMAP, QIMAGE, QIMAGELIST,
  `$RESOURCE`, `Draw`), decoded by the shared model on native,
  interpreter and web (jpeg-decoder, already part of the build).
- QIMAGELIST `AddICOFile` / `AddICOHandle` / `InsertICOFile` /
  `InsertICOHandle` (an icon is scaled whole to the list's size, its
  see-through parts kept) and `GetICO`; QIMAGE `ICOHandle` / `Icon`.

### Fixed
- A SUB / FUNCTION parameter or local named like a global component (`b`
  while a QBITMAP `B` exists; names are case-insensitive) is the variable
  in its routine, not the component — on both backends.

## [2.75.0] — 2026-09-30

### Added
- RapidQ's dotted TYPE fields (CommCtrl.inc, qdataBaseSQL.inc):
  `hdr.hwndFrom AS LONG`, `Table.Name(150) AS STRING`, nested as deep as
  written, are a record inside the record — `N.hdr.hwndFrom`,
  `N.Table.Name(2)` — on both backends.

### Fixed
- High-DPI on the web: a QSTRINGGRID's owner-drawn cells (OnDrawCell)
  are drawn at the screen's scale — lines, text and images sharp at 2×.

## [2.74.0] — 2026-09-30

### Added
- QTREEVIEW on native, interpreter and web: `StateImages` (a node's
  `StateIndex` image beside its own; index 0 is none, as in Windows),
  OnGetImageIndex / OnGetSelectedIndex (Index) — the program sets
  `Item(Index).ImageIndex` / `.SelectedIndex` as nodes are shown, asked
  again when the shown nodes or the selection change — `HideSelection`
  (no selection shown while the tree hasn't focus), and `GetItemAt(X, Y)`
  (the manual's hot-tracking example).

### Fixed
- Setting a tree's `Images` after it was shown now shows the icons.
- The web no longer warns about QIMAGELIST as an unknown component.

## [2.73.0] — 2026-09-30

### Added
- Window icons on native, interpreter and web: `Form.Icon` (a file) /
  `Form.IcoHandle` (a `$RESOURCE`), and `Application.Icon` / `IcoHandle`
  for every form without its own — an ICO, BMP, PNG or SVG. The web shows
  them in the forms' title bars and as the page's icon. (macOS shows no
  window icons; Windows and Linux do.)
- ICO and PNG pictures wherever RapidQ takes a bitmap (QBITMAP, QIMAGE,
  QIMAGELIST, `Draw`, `$RESOURCE`), decoded by the shared model with their
  see-through parts — every icon of RapidQ's own icon folder reads.

### Fixed
- `CopyRect(D, Image, S)` with `DIM R AS QRECT` rectangles copied nothing
  (a DIMmed QRECT had no value to pass).
- A QIMAGE whose size the program hasn't set takes its first picture's
  size, as RapidQ's manual says (it stayed 100 × 100).

## [2.72.0] — 2026-09-29

### Added
- **QHEADER** on native, interpreter and web: column headers the user
  clicks and resizes. `AddSections`, `Clear`, `SectionsCount`, and
  `Sections(i).Caption` / `Width` / `MinWidth` / `MaxWidth` / `Alignment` /
  `AllowClick` / `Style`; dragging a section's edge (the resize cursor
  shows there) fires OnSectionTrack (Index, Width, State: begin, move,
  end) and then OnSectionResize; a click fires OnSectionClick. Sections
  with `Style = hsOwnerDraw` are drawn by OnDrawSection (Index, Pressed,
  Rect), with the header's own canvas methods (`Sender.FillRect`, …).

## [2.71.0] — 2026-09-29

### Added
- **QOUTLINE** on native, interpreter and web — Windows 3.1's tree, shown
  as a tree view: `AddLines` (each leading space a level deeper),
  `AddChild(Index, S)`, `Insert`, `DelLines`, `Item(i)` read and written,
  `Row`, `LineCount`, OnClick / OnDblClick.
- **QFILEDIALOG** (RAPIDQ2.INC's): `Mode` fdOpen / fdSave, `MultiSelect`,
  `Files(0)` the folder then the picked names, `SelCount`, `FileTitle`,
  `DefaultExt`, `WarnIfOverWrite`, `Caption`.
- Web file dialogs: Open / Save now wait for the user (as on the desktop)
  in a dialog in the page listing the program's files, with a name field
  and Upload… for a file from the computer.
- Desktop test hook `RAPIDR_TEST_FILE_DIALOG` (what the dialogs answer).

### Fixed
- QOPENDIALOG / QSAVEDIALOG: RapidQ's `Filter` ("Pictures|*.bmp;*.ico|All
  Files|*.*") and `FilterIndex` work (the filter went to the dialog
  unconverted); `InitialDir`, `Caption`, a preset `FileName`; the web's
  Open returned True before anything was picked.
- The desktop test actions `__node_i` / `__toggle_i` fire OnClick as a
  click does.
- `tools/regress.sh` keeps its build folder bounded (it grew ~35 GB per
  run and filled the disk) and reports a web test that fails by its exit
  code; the IDE project-reload test counts forms, not their controls.

## [2.70.0] — 2026-09-29

### Added
- **High-DPI screens** (Retina, a browser at 2×) on native, interpreter
  and web: canvases, form surfaces, QIMAGE pictures, owner-drawn list and
  combo items, grid images and tree icons are shown at the screen's scale
  instead of enlarged — text, diagonal lines and ellipses are drawn from
  the device pixels, SVGs at the scale (drawn again when the scale is
  learned or changes). The pixels a program reads and saves (`Pixel`,
  `.BMP`, flood fills, SaveToFile) are exactly the 1× ones: each bitmap
  keeps what the screen shows next to them. `RAPIDR_SCALE` forces a scale
  on the desktop (tests); the whole desktop GUI suite and both web suites
  pass unchanged at 2×, and `tools/regress.sh` runs the web GUI suite at
  2× too.
- An image that is an SVG's (not drawn on) hands out the SVG itself as its
  `.BMP` / `GetBMP` data, so what it's drawn onto can draw it sharply.

## [2.69.0] — 2026-09-29

### Added
- `RUN "program"`: starts a program without waiting (its process ID), on
  native and interpreted desktop builds (the browser starts none).
- `INITARRAY(A, v1, v2, …)`: the first elements of A get the values (both
  backends).
- A method of any object expression (`This.Names.Item(2)`, a sub-object's
  `printer.Font.DelStyles(3)`); SUBI / FUNCTIONI closed by `END SUB` /
  `END FUNCTION` (QAVI.inc).
- Include folder: 85 of 108 libraries compile; example corpus: 148 of 386.

### Fixed
- SHELL / SHELLWAIT / RUN run the command through `cmd /C` on Windows
  (they used `sh -c` everywhere).

## [2.68.0] — 2026-09-29

### Added
- RapidQ syntax used by its include libraries and example programs, on
  both backends: an object field's own properties (`P.MoverRect.Top`);
  keywords as a TYPE's field and method names (`Step AS DOUBLE`,
  `Data AS QStringGrid`, `FUNCTION Create`, `SUB Close`), as parameters
  (`select`, `case` — SELECT CASE keeps working) and as variables
  (`type = 2`); `END PROPERTY SET`; `STRUCT … END STRUCT` (a TYPE);
  `ByVal` in a call's arguments; `_` stuck to a name at the end of a line
  continues it; comment lines inside a continued statement.
- RapidQ's include folder: 83 of 108 libraries compile on their own (72
  before); the example corpus: 140 of 386 programs (135).

## [2.67.0] — 2026-09-29

### Added
- **SVG images** wherever RapidQ takes a bitmap — QBITMAP and QIMAGE
  (`LoadFromFile`, `BMP`, `BMPHandle`), QIMAGELIST (`AddBMPFile`,
  `AddBMPHandle`), `Canvas.Draw`, `$RESOURCE` — on native, interpreter
  and web: drawn by resvg (pure Rust, Apache/MIT) in the shared image
  model, with soft edges (each pixel's opacity is kept, blended when
  drawn, and survives `.BMP` as a 32-bit BMP). RapidQ's BMP handling is
  unchanged.

## [2.66.0] — 2026-09-29

### Added
- QPANEL `BevelOuter`, `BevelInner`, `BevelWidth`, `BorderWidth` drawn as
  RapidQ does (a raised outer bevel by default) on desktop and web, from
  one shared model (`rapidr_value::objects::bevel`).
- `examples/digdisplay`: a clock on RapidQ's QDigDisplay.inc, with
  RapidR's own seven-segment bitmaps (the originals were never
  distributed; `tools/make_digit_bitmaps.py` makes them).

### Fixed
- RapidQ's include libraries (QBevel.inc, QDigDisplay.inc, …), both
  backends: an instance of a TYPE extending a component created inside a
  form stopped with "not an object"; `.Field = x` in its PROPERTY SET was
  lost; fields redeclaring the component's properties (`Width AS LONG`)
  never reached the widget; FOR counters and `Result` in its code were
  taken for the component's properties.
- A canvas setting its own size in its OnPaint (QDigDisplay) repainted
  forever: a size change repaints only when the size really changes.

## [2.65.0] — 2026-09-29

### Added
- **QTREEVIEW in-place editing** on native, interpreter and web: F2, or a
  click on the node already selected (a double click doesn't), asks
  `OnEditing (Index, AllowEdit)` — which may refuse — and opens an editor
  over the node's text; Enter or leaving it asks `OnEdited (Index, S)`,
  whose `S` answers back the node's new text; Escape drops the edit, and
  `ReadOnly` allows none. Test actions `tree.__edit` / `__enter` /
  `__escape` (desktop hooks and the web harness); fixture `tree_edit.bas`.
- **RapidQ's global objects** from one shared model
  (`rapidr_value::globals`) on native, interpreter and web — they were
  property bags, so `Screen.Width` read 0: `Screen` (`Width`, `Height`,
  `ClientWidth` / `ClientHeight`, `MouseX` / `MouseY`, `Monitors`,
  `Cursor` over every form), `Application` (`ExeName`, `Path`, `Title`,
  `Terminate`, `Minimize`, hint settings kept), `Clipboard` (`Text`,
  `SetAsText`, `GetAsText(n)`, `Clear`, `HasFormat(CF_TEXT)`,
  `FormatCount`, `Open` / `Close`) and `Mouse.X` / `Y`. The desktop uses
  the system clipboard (arboard, MIT/Apache); the web keeps the
  program's own and writes it to the browser's when allowed. Tests set
  `RAPIDR_TEST_CLIPBOARD` and never touch the user's clipboard.
- Component `Cursor` on the desktop (it was ignored), with RapidQ's codes
  (`crDefault` 0 … `crHandPoint` -21) shared by both runtimes.

### Fixed
- Web: `Cursor` used made-up numbers (1 = hand); it takes RAPIDQ.INC's
  `cr*` codes now.
- A `QRECT`'s (or any object's) `Left` / `Top` / `Right` / `Bottom` read
  0 until set (they were empty); on the web a DIM'd object's fields were
  not kept at all.

## [2.64.0] — 2026-09-29

### Added
- **QTREEVIEW** from a shared model (`rapidr_value::objects::tree`) on
  native, interpreter and web: nodes numbered depth-first as RapidQ does,
  `AddItems`, `AddChildItems`, `InsertItem`, `DelItems` (with the node's
  subtree), `Clear`, `Sort`, `Expand` / `Collapse` (recursive),
  `FullExpand` / `FullCollapse`, `GetItemAt`, `Item(i).Text` /
  `ImageIndex` / `SelectedIndex` / `StateIndex` / `HasChildren` /
  `Selected` / `Expanded` / `Count` / `Level` / `IsVisible` / `Parent` /
  `Handle`, `ItemCount`, `ItemIndex`, `TopIndex`, `ShowButtons`,
  `ShowLines`, `Indent`, `Images` (node icons), `LoadFromFile` /
  `SaveToFile` (tab-indented lines). What the user does asks the program
  first — `OnChanging (Index, AllowChange)`, `OnExpanding`,
  `OnCollapsing` — then `OnChange`, `OnExpanded`, `OnCollapsed`;
  `OnDeletion` for every deleted node; `OnClick`, `OnDblClick`. RapidR's
  older names (`AddRoot`, `AddChild` by text, `SelectedItem`) still work.
  It was a bare FLTK tree / HTML list with a few RapidR-only methods.

### Fixed
- Desktop: **a click on a button fired OnClick twice**, and every widget
  with its own event handling (buttons, check boxes, edits, grids, lists,
  canvases, splitters, MDI frames) also got FLTK's handling first: fltk-rs
  runs a widget's own handler before a custom one by default. RapidR's
  handlers run first now (`super_handle_first(false)`), as they were
  written to.
- Desktop: a click on an owner-drawn list box's scroll bar now scrolls it.

## [2.63.0] — 2026-09-29

### Added
- QLISTBOX `ExtendedSelect` (on by default, as in RapidQ): in a
  MultiSelect list, Shift+click selects a range, Ctrl+click toggles, a
  click selects one item; off, a click toggles. `TabWidth`: tabs in items
  go on to its stops (dialog units; 32 by default). In the shared list model
  (`ItemList::click`), for drawn lists on the desktop and the web.

## [2.62.0] — 2026-09-29

### Added
- **`$THEME` in interpreted programs** (only native builds honored it) and
  a `RAPIDR_THEME` environment variable for programs without one. Names:
  fltk-theme's themes (`classic`, `aero`, `metro`, `aquaclassic`,
  `greybird`, `blue`, `dark`, `highcontrast`) and schemes (`aqua`,
  `fluent`, `clean`, `gleam`, `svg`, `sweet`, `fleet1`, `fleet2`, in light
  colors), FLTK's (`base`, `gtk`, `plastic`, `oxy`), and platform names
  (`windows`, `mac`, `linux`, `win7`, …).

### Changed
- Linux (and other non-macOS / Windows systems) default to a light look
  (Gleam) instead of Dark: RapidQ programs set their colors for a light
  look. macOS (classic Aqua) and Windows (Metro) keep theirs.
- Native debug builds keep line tables only (`[profile.dev]`): full debug
  info made every program's build hundreds of megabytes.

### Fixed
- Native builds: a `$THEME` value with a quote in it broke (or injected
  into) the generated program; it is escaped.
- `$THEME crystal` crashed (fltk-theme's scheme panics): it gives `clean`.

## [2.61.0] — 2026-09-29

### Fixed
- Web: a `ShowModal` in the main program didn't wait — the statements after
  it ran at once. It waits for the form to close, as on the desktop; the web
  IDE shows the project's startup form after the program's own statements
  (as RapidQ's designer places it), not before them.
- Web: a program whose main body finishes with no form open ends (its
  timers stop, no event reaches it), as the desktop program exits.
- `Form.Repaint` (`Refresh`, `Update`, `Paint`) fires the form's OnPaint on
  the desktop (it was "not implemented") and the web.
- Web: a QPANEL's `Caption` read back the text of the controls inside it.

### Added
- Web GUI parity checks for a resized form and a dragged QSPLITTER
  (`align_layout`, through `rapidr_test_resize`), a ShowModal in the main
  program and Form.Repaint (`startup_modal`).

## [2.60.0] — 2026-09-29

### Added
- **QLISTBOX `Columns`**: items flow down each column and into the next,
  that many columns showing, scrolled sideways; the arrow keys move within
  and across columns. Owner-drawn lists can have columns too (OnDrawItem's
  Rect is the item's cell).
- **Owner-drawn QCOMBOBOX** (`Style = csOwnerDrawFixed / csOwnerDrawVariable`):
  OnDrawItem draws each item (FillRect, TextOut, Draw, … on the combo box),
  OnMeasureItem sizes them (Variable); the box shows the selected item as
  drawn, the drop-down lists the items as drawn, a pick sets ItemIndex and
  fires OnChange, Up / Down pick the item before / after.
- Both from the shared list model on native, interpreter and web
  (`ItemList::item_rects`, `item_at`, `set_view`).

### Fixed
- Desktop: QSTRINGGRID and owner-drawn QLISTBOX scroll bars weren't drawn
  (FLTK's table left them unpainted under the theme); an owner-drawn list
  box's background showed the theme's shading past its items; clicking an
  owner-drawn list box redrew only part of it.

## [2.59.0] — 2026-09-29

### Added
- **Keyboard events on the desktop** (there were none): `OnKeyDown(Key,
  Shift)` and `OnKeyUp(Key, Shift)` with the Windows virtual-key code
  (`A` = 65, arrows 37–40, F1 = 112, …) and RapidQ's Shift state (`ssShift`
  256, `ssCtrl` 16, `ssAlt` 1); `OnKeyPress(Key)` with the character typed
  (Enter 13, Backspace 8, Tab 9, Escape 27). They go to the focused
  component, then to its form — the same on the web, where a click lets
  a form or canvas take the keyboard.
- **Mouse events on every component** with RapidQ's arguments: 
  `OnMouseDown` / `OnMouseUp(Button, X, Y, Shift)` (`mbLeft` 0, `mbRight`
  1, `mbMiddle` 2) and `OnMouseMove(X, Y, Shift)`, to the component under
  the mouse (the one pressed while a button is held), X and Y in it. On the
  desktop only QCANVAS (with just X, Y) and QIMAGE had them.
- One set of rules for both runtimes (`rapidr_value::input`: key codes,
  typed characters, Shift bits, argument lists).
- The web IDE's handler stubs use RapidQ's parameters (OnClose's Action,
  keys, mouse, OnSelectCell, OnMeasureItem, …).

### Changed
- Web: key events passed (keyCode, shift, ctrl, alt) and mouse events (X,
  Y, Button) — five values, so a `Sender` parameter got the wrong one; a
  form also got the mouse events of its controls. The examples
  (`web_canvas.rr`, `web_ide.rr`, `strip_ide.rr`) use RapidQ's order.

### Fixed
- Desktop: clicking a QSTRINGGRID's header left the grid blank but for a
  few cells (FLTK's table handles the click first and redrew only part of
  itself); the grid is drawn whole after a click.
- Desktop: redrawing a form's drawing surface painted over its controls.
- Web: an owner-drawn QLISTBOX showed its items as they were before
  OnDrawItem ran (a newly selected item didn't show as selected).
- The `event_answers` test gives its list items readable heights.

## [2.58.0] — 2026-09-28

### Added
- **Event parameters come back to the runtime**, as in RapidQ (which passes
  them by reference), on native builds, the interpreter and the web:
  - QFORM `OnClose(Action)`: `Action = caNone` (or `False`) keeps the form
    open, `caMinimize` minimizes it; it starts as `caHide`;
  - QSTRINGGRID `OnSelectCell(Col, Row, CanSelect)`: `CanSelect = 0` puts
    the selection back (also from a TYPE's `EVENT OnSelectCell`);
  - QLISTBOX `OnMeasureItem(Index, Height)` (`lbOwnerDrawVariable`): each
    item is as tall as its handler says; OnDrawItem's Rects follow;
  - QSTRINGGRID `OnListDropDown(Col, Row, S)`: a gcsList column's
    drop-down shows the items the handler leaves in `S`.

  The continuation of an event (v2.57.0) now gets the arguments as the
  handler left them: the VM hands over the handler's parameters when it
  returns; native builds bind handlers so their parameters write back
  (`rp_bind_event_out`; a SUB bound to an event takes its parameters by
  reference from the runtime, still by value when the program calls it).
  One handler type and calling rule for both runtimes
  (`rapidr_value::events::Handler`, `call`).

### Fixed
- Desktop: the window's close button fired no OnClose (FLTK hid the window
  itself; Escape closed forms too); `Form.Hide` fired OnClose, which only
  `Close` does.
- Web: `Form.Caption` read back the whole window's text (title bar
  buttons, controls); it reads the title.

## [2.57.0] — 2026-09-28

### Added
- **QFORMMDI** (RAPIDQ2.INC's MDI form) on the desktop and the web: child
  windows inside the form, each showing one of the program's components
  (`AddChild(Edit(i).Handle, "Title", i, left, top, width, height,
  DefaultSize)`), with a title bar, minimize / maximize / close buttons,
  moved by the title bar and sized by the corner, double-click to
  maximize; `CloseChild`, `CloseAllChild`, `CascadeChild`, `SetHorzChild`,
  `SetVertChild`, `IconArrangeChild`, `MinimizeAllChild`,
  `MaximizeAllChild`, `RestoreChild`, `ActiveNextChild`,
  `ActivePreviousChild`, `ActiveChild`, `GetChild`, `ChildExist`,
  `FreeChild`; `ChildCount`, `ChildMax`, `ChildCaption`, `ChildHandle`,
  `ChildLeft` / `Top` / `Width` / `Height`, `ChildState`,
  `ComponentIndex`, `ChildResult`; `OnChildActive`, `OnChildClose` (setting
  `ChildResult` to False keeps the child open), `OnChildResize`. One model
  (`rapidr_value::mdi`, the same code applies it in both runtimes); it was
  a stub (AddChild only counted) on the desktop and nothing on the web.
- **`Component.Handle`**: a stable number for each component (it was
  empty), and the way back from it (`rapidr_value::handles`).
- **Events whose handler the runtime waits for** (`rp_fire_event_then`,
  `rapidr_value::events`): the runtime goes on once the handler has run —
  at once in a native build; in the interpreter the continuation travels
  with the queued event and the VM hands it back when the handler returns,
  even if it waited for a dialog. OnChildClose's `ChildResult` uses it; the
  by-reference event results (OnClose's Action, …) will.

### Fixed
- Desktop: a component given a parent after its form is shown (`Late.Parent
  = Form` in an event handler) got no widget; it gets one then, with its
  children (the web did this already).
- The interpreter runs the events a host operation queued until none is
  left (events a handler's continuation fires included).

### Tests
- **GitHub Actions no longer run on push** (they cost minutes and failed on
  Linux): the CI workflow runs only when started by hand; every check runs
  locally before a commit — `tools/regress.sh` (unit, conformance on both
  backends, native examples via `tools/native_examples.sh`, desktop GUI
  events, the web suites).
- GUI parity (desktop native + interpreted + browser): `mdi_children`
  (adding, tiling, next, closing with a veto, FreeChild / GetChild /
  ChildExist, the components' places), `late_parent`; the test hooks read
  `name.__shown` (a widget the user can see). Unit tests for the MDI model
  and the event continuations.

## [2.56.0] — 2026-09-28

### Added
- **Memory functions, memory-safe, on every backend**: `VARPTR`,
  `UDTPTR`, `VARPTR$`, `MEMCPY`, `MEMSET`, `MEMCMP`, `RTLMOVEMEMORY`,
  `SIZEOF`, `QMemoryStream.Pointer`, `@x` passed to a DLL — natively,
  interpreted and in the browser (`rapidr_value::memory`,
  `rapidr_ast::memory`). An address is an ordinary number (it fits a LONG;
  `ptr + 4` works) inside a live view of the program's own data: an array's
  elements, a TYPE's fields, a stream's buffer, or a plain variable's bytes
  (copied back into the variable after a statement that writes memory).
  Bytes are laid out as RapidQ stores them: BYTE 1, WORD / SHORT 2,
  INTEGER / LONG / DWORD / SINGLE 4, DOUBLE 8, `STRING * n` n bytes, a
  STRING as the address of its characters, TYPE fields packed. An address
  that isn't the program's memory, runs past the end of it, or belongs to
  something that no longer exists is a run-time error, never a crash. The
  interpreter refused these, and native builds returned 0 for `VARPTR`.
- **`Stream.WriteUDT` / `ReadUDT`** write and read a TYPE's bytes (the
  manual's `S AS STRING*8, N AS INTEGER` is 12 bytes).
- **Native DLL calls get real memory**: an argument that is an address the
  program got (`VARPTR`, a stream's `Pointer`, a TYPE) is passed as a real
  buffer holding those bytes, and what the DLL writes there is copied back
  (checked with the C library's `strlen` / `memset`).

### Fixed
- `SIZEOF(INTEGER)` is 4 (it was the length of the text "INTEGER", 7),
  `SIZEOF(SHORT)` 2, `SIZEOF(TMyType)` the TYPE's packed size, a STRING
  variable its length.
- The web IDE's help for `VARPTR` / `VARPTR$`, and entries for `UDTPTR`,
  `MEMCPY`, `MEMSET`, `MEMCMP`, `SIZEOF`, `RTLMOVEMEMORY` and `CBOOL`.

### Tests
- Conformance `memory_functions` (the manual's examples: variables, strings,
  arrays, TYPEs, streams, a SUB swapping two variables through pointers);
  unit tests for the layout, copies and bad addresses.
- 136 of 386 example programs compile (was 134), all build natively.

## [2.55.0] — 2026-09-28

Closing open items before adding new syntax: an audit of the roadmap's
compatibility list, every fix made on native, interpreted and web together.

### Changed
- **One builtin library for all three runtimes**: the 63 string / math /
  conversion builtins that the desktop and web runtimes each had a copy of
  now live once in `rapidr_value::builtins` (about 900 duplicated lines
  gone), so a fix reaches native builds, the interpreter and the browser at
  once.
- **Numbers print as RapidQ shows them** (Delphi's `FloatToStr`, which the
  manual's `FORMAT$` / `STRF$` point to): 15 significant digits, so
  `0.1 + 0.2` prints `0.3`, `1 / 3` `0.333333333333333`, `2 ^ 70`
  `1.18059162071741E21`, `1E-7` stays short; `NAN` / `INF`.
- **QTIMER is enabled by default**, as the manual says: a timer with only
  `Interval` and `OnTimer` set now ticks (it never did, on any runtime).

### Fixed
- **`INSERT$(insert, source, index)`** takes RapidQ's argument order
  (`INSERT$("hi", "Hello", 3)` = `Hehillo`); it inserted the wrong way round.
- **`FORMAT$`** is Pascal's `Format` with any number of arguments: `%d %u
  %x` (`.prec` = at least that many digits), `%e %f %g %n %m`, `%s` (`.prec`
  = at most), width, `-`, `%1:d` argument index, `*`, `%%`. It formatted one
  number with a VB-like picture.
- **`STRF$(v, format, precision, digits)`** is `FloatToStrF` (ffGeneral,
  ffExponent, ffFixed, ffNumber); it ignored its arguments. Decimal digits
  round half away from zero, as in RapidQ (`%.1f` of 2.25 is `2.3`).
- **`RANDOMIZE seed`** repeats the sequence (it was ignored everywhere), with
  one generator on every runtime; the data-science components (`shuffle`,
  `rand`, `randn`, `choice`, …) draw from it too.
- **Type suffixes declare types** (manual): `?` BYTE, `??` WORD, `???`
  DWORD, `%` SHORT, `&` LONG, `!` SINGLE, `#` DOUBLE — `q% = 40000` holds
  -25536, `b? = 300` holds 44; `DIM n%`, parameters and `FUNCTION f%`
  without `AS` take the suffix's type. `b?` / `w??` didn't parse.
- **QCOOLBTN / QOVALBTN groups** (`GroupIndex`, `Down`, `AllowAllUp`):
  pressing one releases the others of its group, `Down = True` from the
  program too, the button that's down stays down unless `AllowAllUp`; OnClick
  reads the new `Down`. The desktop only toggled the pressed button, the web
  didn't toggle at all. One rule (`rapidr_value::toggle_group`) for both.
- Desktop buttons, cool buttons and images click only when the mouse went
  down on them and came up over them: a stray release (macOS can deliver one
  when a window appears) clicked a focused button twice. Space clicks a
  button once, when it's released; Enter at once.
- **`END` in the browser** stops the program as on the desktop — in the web
  interpreter and in compiled web builds (`rapidr build --web`, where END
  only logged and the program went on): what was written to files is kept,
  the forms close, timers stop, no event reaches the program any more.
- The web read `Enabled` (and other visual properties) of a component with
  no element, such as a QTIMER, as empty.

### Tests
- Conformance: `builtins_manual`, `type_suffixes`, `early_findings` (the
  first review's findings, all fixed), `inv_instr_redim`; GUI parity
  (desktop native + interpreted + browser): `coolbtn_group`,
  `timer_default`; `tests/web_end_timer.mjs` runs END in both web builds —
  the first test of a compiled web build.
- Desktop GUI tests ignore the real mouse and keyboard (only the test's own
  events drive the program): a key typed elsewhere during a run landed in
  the test window and clicked its focused button, making results vary.
- The desktop GUI test hook fires one click per turn of the event loop, as
  real clicks come (the interpreter runs handlers after the click returns);
  `node tests/native_gui_events.mjs <name>` runs one case.
- Builds are warning-free: unused code removed (the old QSTRINGLIST, a
  widget kind nothing created, unread fields), deprecated web-sys call.
  QFORMMDI, found to be a stub, is on the roadmap.
- `rand` is no longer a dependency.

## [2.54.0] — 2026-09-28

### Added
- **`DIM s AS STRING * n`** and `Name AS STRING * n` inside `TYPE`: a store
  is cut to `n` characters (RapidQ keeps a fixed string in an `n`-byte
  buffer; what a shorter value leaves unused isn't part of the text), for
  variables, array elements, TYPE fields and locals. Shared by both
  backends and the browser (`__to_fixed`, `rapidr_ast::numeric`).
  `STRING * 0` (an API buffer in old code) stays unbounded.
- **`CBOOL(x)`**: true for a non-zero number or numeric string, and for any
  other non-empty string. Interpreter, native and web.
- **`ON ERROR RESUME NEXT` / `ON ERROR GOTO label|0`** (VB code, not RapidQ)
  is accepted and ignored — a run-time error still ends the program — so
  such sources compile instead of failing on the first line.
- **A `WITH` left open** is closed by `END SUB` / `END FUNCTION`, as RapidQ
  allows.
- **Routines that differ only by type suffix** (`FUNCTION Day$` and
  `FUNCTION Day`) are different routines on both backends
  (`rapidr_ast::suffix_routines`); the interpreter used to run the last
  one for both, native builds didn't compile.
- **Native builds: `GOSUB` / labels inside `SELECT CASE`** (the selector is
  kept across states, each branch is a state). A label inside `WITH` or
  `CREATE` is still refused with a message.

### Fixed
- Native builds: a local variable sharing a global array's name (`month&`
  next to `DEFSTR MONTH$(1 to 12)`) made the array unknown in every routine
  after it (`cannot find value month`); a routine's own declarations no
  longer change what a name means elsewhere.

### Tests
- Conformance: `fixed_strings`, `on_error_accepted`, `with_unclosed`,
  `suffix_names`, `local_shadows_array`, `gosub_in_select` (interpreter and
  native); unit tests for the parser, `rapidr_value` and the renaming pass.
- 134 of 386 example programs compile (was 130) and all 134 build natively.

## [2.53.0] — 2026-09-28

### Added
- **`INPUT #n, a, b` and `LINE INPUT #n, s`** read from a file opened
  `FOR INPUT AS #n`: a field is a quoted string or text up to the next
  comma or line break; `LINE INPUT` takes the rest of the line. Both
  backends and the browser.
- **BASIC file I/O works in the browser**: `OPEN … FOR INPUT | OUTPUT |
  APPEND | BINARY AS #n`, `PRINT #`, `WRITE #`, `EOF`, `LOF`, `SEEK`,
  `CLOSE`, `KILL`, `FILELEN` use the page's own files (the same store the
  stream objects use); they were stubs, and `PRINT #` stopped the program.
  It is one shared implementation (`rapidr_value::basic_files`) for the
  interpreter, native builds and the web: a file is read whole when opened,
  and what's written goes back when it is closed, when its length is asked
  for, or when the program ends — a file the program never closed keeps its
  data (the desktop lost it before).
- `FREEFILE` without parentheses (`n = FREEFILE`) is the function, not an
  undeclared variable.
- **Keywords as names**: `DIM New AS QMENUITEM, Open AS QMENUITEM`,
  `Open.Caption = "&Open"`, `File.AddItems New, Open, Save`. A statement
  keyword (`Open`, `Close`, `Write`, `Seek`, `Kill`, `Input`, …) followed
  by `.`, declared with `AS`, or used as an argument is an identifier.
- **`CREATE cells(0 TO 9, 0 TO 4) AS QBITMAP … END CREATE`** makes an array
  of components, like the same `DIM`.

### Fixed
- **Native builds ignored `bups.OnPaint = bups.paint`** when the handler is
  a SUB with a dotted name (`SUB bups.paint`): the event was set to null, so
  the corpus's Sokoban (GB) drew only its buttons natively. Native and
  interpreted builds now draw the same window.

### Tests
- Conformance `basic_file_io` and `create_array` (both backends; the
  first also in the browser), `dotted_paint` (desktop and web), unit tests
  for the file module.

## [2.52.0] — 2026-09-28

The web IDE kept in step with the desktop, native and interpreter builds.

### Added
- **The conformance suite runs in the browser**: `node tests/web_conformance.mjs`
  compiles every console case of `tests/conformance/cases` with the web
  IDE's wasm compiler, runs it in the wasm VM, and compares with the same
  `.expected` file the interpreter and the native build are held to. 53 of
  55 match; two can't be compared in a browser (ANSI terminal codes, the
  file system) and carry a `' xfail: web — reason` marker.
- **`$RESOURCE` in the web IDE**: the files come from the project's assets
  (found by their last path part, `resource_files\two.bin` → `two.bin`), and
  are built into the program as on the desktop. A missing one is a compile
  error that names it. The wasm `compile()` takes the assets as a third
  argument.

- **The desktop's GUI fixtures run in the browser too**:
  `node tests/web_gui_parity.mjs` fires the same events on the same
  `tests/fixtures/*.bas` as `tests/native_gui_events.mjs` (both read the
  table in `tests/gui_parity_cases.mjs`) and compares the same properties
  with the same expected values. 21 checks match; two cases don't apply to a
  browser (a window resized by the user, a directory listing).
- `rapidr_get_prop(name, prop)` on the wasm module (and
  `window.__rapidr_rt` in the preview): a property as the program reads
  it, for tests and tools.

### Fixed
- **`ShowModal` waits on the web when a handler calls it**: a second form
  opened from a button handler now blocks that handler until the form
  closes (the VM suspends it, as the desktop's wait does) — the code after
  `Form2.ShowModal` ran at once in the browser before. Method calls can now
  suspend the VM like builtins do. The startup form's `ShowModal` in the
  main body still doesn't wait: the IDE puts it before the program's own
  statements.
- `Label.Caption` and the other captions read back what the program set;
  the browser returned the shown text with its `&` accelerator marks
  taken out (`"a & b"` came back as `"a  b"`).
- A new QLABEL / QBUTTON has an empty caption on the web as on the
  desktop (it was "Label" / "Button").
- **QSTRINGLIST was two different implementations**: the desktop's had
  `Item(i)`, the web's had `Get(i)` only, so RapidQ programs that call
  `Names.Item(1)` got "" in the browser. It is now one shared model
  (rapidr-value, the list box's items) both runtimes use, with `Add`,
  `Insert`, `Delete`, `Item(i)` read and write, `Count`, `IndexOf`, `Sort`,
  `Text`, `Clear`, `LoadFromFile` and `SaveToFile`.
- `Form.Font.Size = 12`, `Font.Name`, `.Bold`, `.Italic` and `.Color` set
  the flat font properties on the web too, as they do on the desktop, so
  text drawn on a form uses the size the program set.
- Conformance case `stringlist`, `resources_memory` and the web run above.

## [2.51.0] — 2026-09-28

RapidQ syntax that real programs use and RapidR refused. Of the 386 programs
in RapidQ's own example folder, 130 now compile (116 before); most of the
rest call Windows APIs, which RapidR doesn't emulate.

### Added
- **Whole arrays as arguments**: `Fill M(), 5`, `Total(M(), n)`,
  `CALL Sort(Names())` pass the array to a SUB / FUNCTION with an array
  parameter (`A() AS INTEGER`). `M()` was read as a call of a function `M`.
  A shared pass, so both backends agree.
- **Multi-dimensional array fields in a TYPE**: `vertex(9, 2) AS SINGLE`,
  `tag(1 TO 2, 0 TO 1) AS STRING` (the OpenGL, CGI and toolbar includes
  use them), on both backends.
- `IF c THEN: a: b: END IF` and `IF c THEN :a` with `ELSE :b` on the next
  line: a colon right after THEN starts a block. A statement after a colon
  may be a comment (`… : ' note`), and a redundant `END IF` may end a
  one-line IF.
- `=>` and `=<` for `>=` and `<=`.
- `&H1&`, `&HFFFF&`: hex literals with the long-integer suffix, as the
  Windows includes write them.
- `CASE 4, 7  C = -2`: the body may follow the case list without a colon.
- A line number in front of `NEXT`, `WEND` and the other ends of a block
  (`310 NEXT I`), and in front of `DATA` (`130 DATA 1, 2`).

### Fixed
- **Native builds now share undeclared variables like the interpreter.** A
  variable that is never DIMmed is global — one variable for the main
  program and every SUB / FUNCTION, kept between calls. Natively each
  routine got its own copy, so `q = 5` in a SUB never reached the main
  program and `w = w + 1` restarted at 0 on every call.
- A program that uses `pi` as a variable of its own (`3dcube.bas`) built
  natively with an error; `pi` is a constant only while nothing assigns it.
- **`$ESCAPECHARS ON` belongs to its own file.** A program that turned it
  on before `$INCLUDE "RapidQ2.inc"` broke the include's plain `""`
  strings; an include file now starts with it off, and the includer's
  setting comes back after it.

### Tests
- Conformance: `syntax_forms`, `array_refs`, `type_multidim_fields`,
  `implicit_globals`, `pi_variable`, and a preprocessor test for the
  `$ESCAPECHARS` scope.

## [2.50.0] — 2026-09-28

### Added
- **Owner-drawn list boxes**: QLISTBOX with `Style = lbOwnerDrawFixed` or
  `lbOwnerDrawVariable`, `ItemHeight` and `OnDrawItem(Index, State, Rect)`,
  on desktop and web, natively and interpreted. RapidQ's *OwnerDraw ListBox*
  example now looks as intended.
  - `FillRect`, `TextOut`, `Line`, `Rectangle`, `Circle`, `Pset` and `Draw`
    on the list draw the item they land on (`Rect` is the item's slot in the
    list); the items are made from the shared bitmap model, so the desktop
    and the web draw the same pixels, text in the built-in Liberation fonts
    and the list's `Font`.
  - `OnDrawItem` fires for every item when the list changes (items,
    selection, height): `State` is 0 for the selected item and 1 for the
    others, as RapidQ programs test it. An item nothing was drawn on shows
    plainly (the selected one white on blue).
  - Clicks and the arrow keys, Home, End, PageUp and PageDown select
    (OnClick / OnDblClick; a MultiSelect list toggles).
  - Every item is `ItemHeight` tall: `OnMeasureItem`'s answers aren't read
    yet (an event can't return a value), and combo boxes aren't owner-drawn.
- **330 constants of RapidQ's `RAPIDQ.INC` were missing** and silently
  evaluated to 0 or "": now `lbStandard` / `lbOwnerDrawFixed` /
  `lbOwnerDrawVariable`, `csOwnerDraw…`, `gcsList…`, every `go…` grid option,
  `ta…` alignments, `cr…` cursors, `ss…`, `vs…`, `bk…`, `ft…`, `sc…`, `os…`,
  the rest of the `cl…` system colors (their usual Windows values) and more.
  A test keeps names unique and checks a sample of values.
- Tests: a unit test for owner drawing, `tests/fixtures/owner_list.bas` (native
  and interpreted agree), `tests/web_ide_owner_list.mjs`.

### Fixed
- `clGreen` and `clPurple` had Delphi's values; RapidQ's are `&H00FF00` and
  `&HFF00FF`.

## [2.49.0] — 2026-09-28

### Added
- **Drawing on a QFORM itself**: `Form.TextOut`, `Line`, `Rectangle`,
  `FillRect`, `Circle`, `Pset`, `Pixel`, `Draw`, `TextWidth`, `TextHeight`,
  `Cls`, … — the RapidQ "Hello World" that draws in its form's `OnPaint`
  now shows its text.
  - Same shared bitmap model as QCANVAS, so the desktop, the web, native
    and interpreted builds draw the same pixels.
  - The form gets its surface the first time it's drawn on. It lies under
    the form's controls, the form's own `Color` shows through where
    nothing is drawn, and it takes no mouse events.
  - Text uses the form's `Font` (name, size, color, styles).
  - The surface is the form's client area (`ClientWidth` × `ClientHeight`).
- A form paints again (`OnPaint`) when its size changes: the user drags
  its edge, or the program sets `Width` / `Height`. Sizes stored while the
  form is being declared or laid out don't.
- Tests: `form_surface` (conformance, both backends), `form_draw.bas`
  (desktop, native and interpreted agree) and checks in
  `tests/web_ide_canvas.mjs` (the browser canvas under the controls shows
  the model's pixels).

### Changed
- `Form.Repaint` / `Refresh` don't fire `OnPaint` (a handler that calls
  them would loop); a canvas's do.

## [2.48.0] — 2026-09-28

### Added
- **`OnPaint`** for QCANVAS and QFORM, on desktop and web, natively and
  interpreted. RapidQ programs draw their canvases there, and it was
  never fired, so those programs showed nothing.
  - It fires once when a form is built (the form, then each canvas),
    again when a canvas is resized, and when the program calls the
    canvas's `Repaint`, `Refresh`, `Update` or `Paint` (no arguments).
  - It isn't fired when the window is only uncovered: a canvas keeps
    what's drawn on it, so there is nothing to redraw. A handler that
    draws can't make more `OnPaint` events.
- Tests: `tests/fixtures/canvas_onpaint.bas` (native and interpreted agree)
  and new checks in `tests/web_ide_canvas.mjs`.

### Not yet
- Drawing on a QFORM itself (`Form.TextOut`, `Form.Line`): its `OnPaint`
  fires, but the form has no surface to draw on yet.

## [2.47.0] — 2026-09-28

### Added
- **QCANVAS is drawn by the shared bitmap model**, like QBITMAP and a
  QIMAGE's picture: every drawing method draws into one surface the
  control's size, and each runtime only shows its pixels. The desktop, the
  web, native and interpreted builds now draw the same pixels.
  - `TextOut(x, y, text, color, background)`, `TextWidth`, `TextHeight` and
    `Font` / `Font.*` on a canvas, in the built-in Liberation fonts (as
    on bitmaps since v2.46.0).
  - `Pixel(x, y)` reads a canvas back; `RoundRect`, `Paint(x, y, color,
    border)` (flood fill), `CopyRect` and `StretchDraw` work on canvases
    as they do on bitmaps.
  - The canvas keeps its `Color` as background: `Cls` fills with it, and
    a resized canvas shows it in the new area.
  - `PenColor` / `BrushColor` / `FontColor` / `FontSize` are the colors
    and size drawing uses when none is given.
  - RapidR's own forms are kept: `DrawText`, `Cls`, `Circle(cx, cy, r
    [, color])`, `FillCircle`, `Ellipse`, `SetFont`, `SetPixel`.
- Tests: `canvas_surface` (conformance, both backends) and
  `tests/web_ide_canvas.mjs` (the HTML canvas shows exactly the model's
  pixels).

### Fixed
- **Desktop `Rect` / `FillRect` read their arguments as x, y, width,
  height**; RapidQ's (and the web's) are the two corners, x1, y1, x2, y2.
  Programs that filled rectangles (the corpus's games) drew the wrong size
  on the desktop.
- Compiler warnings on the web build (unused variables in the file list).

## [2.46.0] — 2026-09-28

### Added
- **Text on bitmaps**: `TextOut(x, y, text, color, background)`,
  `TextWidth`, `TextHeight`, `Font` and `Font.*` on QBITMAP, and on a
  QIMAGE's picture.
  - The same pixels on desktop and web, natively and interpreted.
  - It uses the fonts now built into RapidR: the **Liberation fonts** (Sans,
    Serif, Mono; SIL Open Font License 1.1). They have the same character
    widths as Arial, Times New Roman and Courier New, the Windows fonts
    RapidQ programs name, so their text layouts come out the same.
  - A font name with Courier / mono picks Liberation Mono; Times / serif /
    Roman picks Liberation Serif; anything else, Liberation Sans.
  - Sizes are points at 96 dpi, as on Windows screens.
  - Bold, italic, underline and strike-out are drawn from the regular
    faces.
  - The font files are read with `ttf-parser` (MIT OR Apache-2.0) and
    filled by RapidR's own anti-aliased rasterizer.
  - They're credited in `LICENSES.md`, with the license in
    `crates/rapidr-value/fonts/`. The web runtime grows by about 1.1 MB.
  - The corpus's *Print QBitmap* now prints its numbers.

### Tests
- New conformance case `bitmap_text` (widths, heights and the exact
  pixels drawn); 104/104 pass on both backends. New rasterizer unit
  tests.
- The 24 corpus programs that draw or measure text run the same natively
  and interpreted.

## [2.45.0] — 2026-09-28

### Added
- **The PRINTER object**, the same in both backends, from one shared model
  (`rapidr_value::objects::printer`).
  - `BeginDoc`, then the drawing methods (`TextOut`, `Line`, `Rectangle`,
    `FillRect`, `Circle`, `Pset`, `Draw`, `StretchDraw`, `CopyRect`),
    `NewPage`, then `EndDoc` or `Abort`.
  - Properties: `Font` (a QFONT, or `Printer.Font.Size = …`),
    `TextWidth` / `TextHeight`, `PageWidth` / `PageHeight` (A4 at
    300 dpi, swapped by `Orientation`), `PageNumber`, `Printing`,
    `Aborted`, `Copies`, `Title`, `Printers(i)` / `PrintersCount` /
    `PrinterIndex`, `Capabilities.*`.
  - The document becomes a PDF: text in Helvetica (the PDF standard font,
    nothing embedded; widths from its metrics), shapes as vectors,
    bitmaps as images.
  - **Where it goes:** on the desktop, `EndDoc` sends it to the chosen
    printer with CUPS `lp`. With no printer it saves a PDF in the current
    directory. `RAPIDR_PRINT_TO=file-or-directory` saves it instead of
    printing. On the web it opens in the browser for printing.
  - The conformance runner and `tools/corpus_compare.mjs` always set
    `RAPIDR_PRINT_TO`, so tests never print on paper.
- **`PLAYWAV file|resource, options`** on desktop and web.
  - A WAV file or a `$RESOURCE` handle; `SND_SYNC` waits, `SND_ASYNC`
    plays in the background, `SND_LOOP` (8, or the manual's 3) repeats.
  - A new sound replaces the one playing; `PLAYWAV ""` stops it.
  - The sound device is opened once and kept open.
  - `SND_SYNC`, `SND_ASYNC` and `SND_LOOP` are among the built-in
    RAPIDQ.INC constants.

### Tests
- New conformance case `printer` (it aborts, so it never prints);
  102/102 pass on both backends. New model unit test for pages and the
  PDF.
- The corpus's Printer and PLAYWAV programs that build run the same
  natively and interpreted.

## [2.44.0] — 2026-09-28

### Added
- **QFILELISTBOX, on desktop and web**: a list box of a directory's files,
  from the shared list model (`rapidr_value::objects::filelist`).
  - `Directory` starts at the current directory; setting it fires
    OnChange.
  - `Mask` (`*.*`; several separated by `;`, case-insensitive as on
    Windows), `AddFileTypes` / `DelFileTypes` (ftDirectory lists `[..]`
    and `[dir]` entries, ftHidden adds dot files), `Update`, `FileName`
    and `Drive`.
  - The list box's own properties and events work as on a QLISTBOX.
- **QDIRTREE, on desktop and web**: a directory tree from the filesystem
  root, shown as indented rows (`rapidr_value::objects::dirtree`).
  - `InitialDir` / `Directory` select a directory, open the ones above it
    and scroll it into view.
  - A click selects a directory (OnChange); a double click opens or closes
    it.
  - `FullCollapse`, `FullExpand` (up to 5,000 rows), `Reload`.
  - Directories are read only when they're opened.
- The web has no filesystem, so there both show nothing, or just the root.

### Fixed
- **`MID$(s, i)` without a length** returns the rest of the string, as in
  QBasic; it returned "".

### Tooling
- `tools/corpus_compare.mjs` removes each program's own native build
  outputs after running it and builds without incremental caches. A full
  run had grown `tests/conformance/.work` to 165 GB and filled the disk.
  Window captures are written next to each program's folder, not in it,
  so programs that list their directory see the same files in both runs.
- Full comparison: the programs that differ from one run to the next do so
  from timing and focus (windows on a live desktop), not from the
  backends; reruns match.

### Tests
- New conformance case `file_list_box`, and two-argument `MID$` in
  `strings`; 100/100 pass on both backends.
- New GUI fixture `file_browser.bas` (a QDirTree driving a QFileListBox)
  in `tests/native_gui_events.mjs`.
- Model unit tests for wildcards, listing and the tree.

## [2.43.0] — 2026-09-28

### Added
- **QSTRINGGRID range selection (goRangeSelect)**, on desktop and web.
  - Dragging over cells, shift-clicking or shift+arrow keys select a block
    of cells.
  - The block is highlighted, and OnDrawCell's `State` has gdSelected for
    each of its cells (gdFocused for the current one).
  - It is on by default, as in Delphi, and off with goEditing, as the
    manual says.
- **gcsList columns** (`ColumnStyle(i) = gcsList`, `ColumnList(i)`). The
  selected cell shows a drop-down button listing the column's items.
  Picking one stores it like an edit: the cell, then OnSetEditText, then
  OnChange.
- **goColSizing / goRowSizing**: dragging a header's border resizes the
  column or row, and `ColWidths` / `RowHeights` follow.

### Fixed
- **Selecting the current cell again clears a range selection.** OnDrawCell
  also fires again when the selection changes, with each cell's new
  `State`, as Delphi redraws selected cells.

### Tests
- New fixture `grid_range_list.bas`, in `tests/native_gui_events.mjs` and
  `tests/web_ide_grid_draw.mjs`. The browser test drags a range, picks
  from a drop-down and resizes a column. New model unit test.
- The corpus's grid and list-view programs (9) run the same natively and
  interpreted.

## [2.42.0] — 2026-09-27

### Added
- **QSTRINGGRID `OnDrawCell(Col, Row, State, Rect, Sender)`, on desktop
  and web from the shared grid model.**
  - After the grid's content, sizes, selection or options change, or on
    `Repaint` / `Refresh`, the event fires once for every cell (up to
    20,000).
  - `State` uses Delphi's bits (gdSelected = 1, gdFocused = 2,
    gdFixed = 4). `Rect` is a QRECT in the grid's coordinates.
  - What the handler draws on the grid is kept on its cell and drawn over
    it: `Line`, `Rectangle`, `FillRect`, `Circle`, `Pset`,
    `TextOut(x, y, text, color, background)` and `Draw(x, y, bitmap)`.
  - Reading the grid in the handler doesn't fire the event again.
  - The corpus's `chkgrid.bas` draws its checkbox column, identically
    natively and interpreted.

### Changed
- **Every event passes its component last, as `Sender`**, as in RapidQ
  (`SUB DrawCell (…, Rect AS QRECT, Sender AS QSTRINGGRID)`). Before, only
  events without arguments passed it. Handlers that declare fewer
  parameters are unaffected.
- **Grid cell text is drawn at Left + 2, Top + 2**, as Delphi's grid draws
  it, on desktop and web. It was vertically centered.

### Tests
- `tests/fixtures/grid_draw_cell.bas` in `tests/native_gui_events.mjs`
  (native and interpreted) and in the new `tests/web_ide_grid_draw.mjs`;
  new model unit test.

## [2.41.0] — 2026-09-27

### Added
- **QCOMBOBOX edit box, as in RapidQ.** The default style, csDropDown,
  lets the user type as well as pick, on desktop and web.
  - Typing sets `Text`, and `ItemIndex` becomes the matching item or -1,
    before OnChange fires.
  - `Style = csDropDownList` (2) keeps a pick-only list.
  - RapidR's own examples that use combos as pick lists now set it.

### Fixed
- **QLISTBOX with `MultiSelect` on the desktop** now shows every selected
  item, and the user can select several (click, shift-click). The
  selection goes into `Selected(i)` / `SelCount`, with `ItemIndex` as the
  clicked item. Before, only one item was ever shown selected.

### Tests
- `tests/web_ide_lists.mjs` types into the combo; new unit test for the
  user's multi-selection.

## [2.40.0] — 2026-09-27

### Added
- **`$RESOURCE`: files built into the program, on both backends.**
  - `$RESOURCE NAME AS "file"` makes `NAME` the resource's handle.
    Several directives may share a line, separated by `:`.
  - The files are built into the program: interpreted builds carry them
    in a new optional bytecode section; native builds use `include_bytes!`.
  - A missing file is a compile error, as in RapidQ.
  - New: `RESOURCE(n)`, `RESOURCECOUNT`, `EXTRACTRESOURCE handle, file`
    and `Stream.ExtractRes(handle)`.
  - `BMPHandle = NAME` and `ImageList.AddBMPHandle NAME` show a resource
    bitmap.
- **QIMAGE shows and draws pictures, the same on desktop and web.**
  - It is backed by the shared bitmap model (`rapidr_value::objects`),
    like QSTRINGGRID and QLISTBOX.
  - Supported: `BMP`, `BMPHandle`, `AutoSize`, `Stretch`, `Center`,
    `Transparent` (the bottom-left pixel's color, as in Delphi's TImage),
    `Pixel`, and the QBITMAP drawing methods.
  - Drawing on an image without a picture first gives it one the size of
    the control.
  - PNG and JPEG files still load as before.
  - The RapidQ Othello game now shows its board and chips, identically
    both ways.
- **QIMAGE mouse events** in RapidQ's order, on desktop and web:
  OnMouseDown / OnMouseUp (Button, X, Y, Shift), OnMouseMove (X, Y,
  Shift), OnClick, OnDblClick.
- **`MOUSEX` / `MOUSEY`**: the mouse position relative to the active
  form's client area.

### Fixed
- **Native builds: a typed main-program variable read inside a CREATE
  block** (`Width = bx`) read an empty value (since v2.38.0). The
  Sokoban level editor's board was invisible because of it.
- **`Circle(x1, y1, x2, y2, c, fill)`** fills with the color `fill`, as
  in RapidQ; it was treated as a yes/no flag and filled with `c`.
- **Interpreter: `True` / `False` without RAPIDQ.INC** are -1 / 0, as in
  native builds; they were empty.
- **Native builds: reading `Img.Center`** reads the property; it called
  the Center method.
- **Native builds: a component DIMmed inside a block** (`IF … DIM Dlg AS
  QOPENDIALOG`) was never created, so RapidQ's rqb2html skipped its file
  dialog. Components are now created by their DIM at any depth, as in the
  interpreter.

### Tests
- **Corpus comparison** (`tools/corpus_compare.mjs`): 98 of the 101
  programs that build run the same natively and interpreted. The other 3
  use random numbers or the clock. `graphics/rotate/rotate.bas` no longer
  builds, because its `$RESOURCE` file isn't in the corpus.
- New conformance cases `resources` and `nested_component_dim`;
  `typed_globals` covers CREATE blocks; 98/98 pass on both backends.
- `tests/fixtures/picture_resource.bas` in `tests/native_gui_events.mjs`,
  natively and interpreted.
- New `tests/web_ide_picture.mjs` for the browser.

## [2.39.0] — 2026-09-27

### Added
- **QMEMORYSTREAM / QFILESTREAM `SaveArray` and `LoadArray`**, on both
  backends from one shared rewrite (`rapidr_ast::stream_arrays`).
  - `Stream.SaveArray(A(i), n)` writes `n` elements starting at `A(i)`;
    `LoadArray` reads them back.
  - Each element takes its declared type's bytes, as in RapidQ: BYTE 1,
    SHORT 2, LONG 4, SINGLE 4, DOUBLE 8, …
  - For a multi-dimensional array the last index steps, so
    `LoadArray(vertex(i, j, 0), 3)` fills `vertex(i, j, 0..2)`.
  - Loading or saving stops at the array's last element. RapidQ wrote past
    it; the corpus's own `arrins.bas` does this.
  - The corpus programs `arrins.bas` and `sieve.bas` now run, and run the
    same both ways.

### Fixed
- **`Stream.Read(x)` and `Stream.Write(x)` use `x`'s declared size.**
  Before, BYTE, SHORT, WORD, DWORD and SINGLE variables always took 4 or
  8 bytes.

### Tests
- New conformance case `stream_arrays`; 94/94 pass on both backends.

## [2.38.0] — 2026-09-27

### Added
- **The RapidQ corpus now runs the same natively and interpreted.**
  `tools/corpus_compare.mjs` builds every program that compiles, both
  ways, runs each (no input, a time limit, window captures), and compares
  what they print and show.
  - 99 of the 102 programs it runs look and print the same. The other 3
    use random numbers or the clock, so their differences are expected.
  - 14 network programs are skipped, so the tool never contacts real hosts.
  - The run found and led to the fixes listed below.
- **Native builds: typed main-program variables and parameters.**
  - A numeric variable of the main program (`DIM total AS LONG`, a FOR
    counter, …) is now a Rust number in an atomic static rather than a
    boxed `Value` slot. SUBs that don't declare their own variable of that
    name use it too.
  - A BYVAL numeric parameter is a Rust number inside its SUB/FUNCTION.
  - The same exclusions as typed locals apply: nothing that could store
    something else into the variable (BYREF, `@`, array or object use,
    fractional STEP, inline Rust).
  - Benchmark, 5M iterations of a main-program loop: 0.19 s → 0.01 s
    native; 0.56 s interpreted.
- **Run-time errors in the interpreter name the file and line:** `…
  (at QFlatButton.inc line 12)`. The bytecode carries a small source map,
  with file names only (never full paths), as an optional section that
  older readers ignore.

### Fixed
- **Desktop, both backends: a crash when a form was resized while being
  shown** (the window manager adjusting it). The resize callback ran while
  the runtime held its widget table ("RefCell already mutably borrowed").
  Introduced in v2.35.0.
- **Native: `Font = Font` inside a CREATE block was dropped** when a QFONT
  variable had that name. It now sets the created control's font from the
  variable, as in RapidQ and the interpreter.
- **Interpreter: `IF Form.ShowModal THEN` / `IF OpenDialog.Execute THEN`**
  (the method called without parentheses for its result) read a property
  instead of showing the form or dialog. Both backends now share the list
  (`rapidr_ast::VALUE_METHODS`).
- **Both backends: `DIM r AS QRegistry` (a RapidQ object RapidR doesn't
  implement) inside a SUB, FUNCTION or TYPE method.** The interpreter
  stopped with "not an object" while native builds went on. Now the local
  refers to an object of its name, as in the main program: its methods
  warn and the program continues (`rapidr_ast::routine_objects`).
- **Security: bytecode decoding checks every count** against the bytes
  left, so a corrupt `.rrbc` can't make the reader allocate gigabytes.
  Slice bounds are computed with checked arithmetic (a 32-bit overflow on
  WebAssembly could bypass the check), and a function's local count is
  limited to the 65,536 slots the instructions can address.

### Tests
- Conformance case `typed_globals`: typed main-program variables used in
  SUBs and shadowed by locals, typed BYVAL parameters (reassigned), BYTE
  wrap, GOTO / GOSUB with a typed FOR counter, a global passed BYREF,
  DIM reset, and INPUT into a typed variable. 92/92 on both backends.
- Unit tests: bytecode source map round trip (names only), corrupt counts
  rejected, and code-generation tests for typed globals.

## [2.37.0] — 2026-09-27

### Changed
- **A form's Width / Height are its whole window, and ClientWidth /
  ClientHeight its inside**, as in RapidQ (Delphi), and the same on the
  desktop and the web (`rapidr_value::layout::form_client_size`).
  - The frame is a 29px caption and a 1px border on every side, plus the
    main menu. A 400 × 300 form has a 398 × 269 client area without a menu.
  - `BorderStyle = bsNone` (0) removes the frame: the window has no
    decorations on the desktop and no title bar or border on the web.
  - Setting `ClientWidth` / `ClientHeight` sets the size that gives that
    inside.
  - Before, a desktop form's Height was its inside (31px taller than in
    RapidQ). A web form's Width left out its border, and `BorderStyle = 2`
    (bsSizeable, the default) gave the web form a dashed border.
  - Other controls' BorderStyle on the web is now none (0) or a solid line.

### Added
- **QSPLITTER can be dragged**, on the desktop and the web, as Delphi's
  TSplitter (`rapidr_value::layout::splitter_drag`):
  - it resizes the control just outside its anchored edge (left of an
    alLeft splitter, above an alTop one, …);
  - it never goes below `MinSize` (30) or leaves less than `MinSize` for
    the rest of the client area;
  - the other aligned controls re-flow as it moves, and `OnMoved` fires
    when the drag ends;
  - the cursor shows the direction.
- Test hook `RAPIDR_TEST_SPLIT=splitter:delta` drags a splitter as the
  mouse would. `RAPIDR_TEST_RESIZE` now takes the form's Width,Height.

### Tests
- `align_layout` (native and interpreted agree) and `tests/web_ide_align.mjs`
  now check the client area of a framed form and a splitter drag with
  OnMoved. The browser test drags with a real mouse. Both runtimes give
  the same numbers for the same program.
- Unit tests for the frame sizes and for splitter drags (neighbour found,
  MinSize, room left, alBottom direction, nothing to resize).

## [2.36.0] — 2026-09-27

### Fixed
- **QLISTBOX and QCOMBOBOX work like RapidQ's.** Both runtimes now draw
  one shared model, `rapidr_value::objects::list`. The RapidQ corpus uses
  `Item` 294 times, `AddItems` 143 and `ItemCount` 90.
  - **Desktop:**
    - `AddItems a, b, c` kept only the last item;
    - `ItemCount` and `Item(i)` weren't implemented;
    - a list box selection didn't fire `OnDblClick`.
  - **Both runtimes:**
    - `Item(i)` read / `Item(i) = s`, `ItemCount`, `ItemIndex` (−1 for
      none), `AddItems`, `InsertItem i, s`, `DelItems i, …` (indexes as
      before any deletion), `Clear`;
    - `Sorted` (case-insensitive; the selected item stays selected),
      `MultiSelect` with `Selected(i)` and `SelCount` (setting `Selected`
      keeps ItemIndex, as in Delphi), `TopIndex`;
    - `Text`: a combo box's current text, which follows ItemIndex and
      selects the matching item when set; a list box's items as
      CRLF-terminated lines;
    - `LoadFromFile` / `SaveToFile`, one item per line.
  - The user's pick is stored before the program's `OnClick` / `OnChange`
    runs, so handlers read the new ItemIndex / Text.
  - **Items are shown exactly as written:** FLTK's `@` formatting codes
    and a menu's `/`, `&` and `\` are escaped on the desktop, and the web
    uses plain-text options.
  - The web list box shows rows rather than a drop-down; its options are
    redrawn once per batch of changes.
- RapidR's older names still work on the same model: `AddItem`,
  `DeleteItem` / `RemoveItem`, `Items`, `Count` / `ListCount`,
  `ListIndex`, `Find`.

### Tests
- `tests/fixtures/list_items.bas`, checked natively and interpreted
  (`tests/native_gui_events.mjs`, both agree) and in the browser
  (`tests/web_ide_lists.mjs`, including picking items with the mouse).
- Unit tests for the list model: every item kept, index tracking through
  inserts and deletes, sorting, multi-select, combo text, legacy names,
  out-of-range indexes.

## [2.35.0] — 2026-09-27

### Added
- **`Align` works on the desktop and the web**: alTop, alBottom, alLeft,
  alRight and alClient, laid out as RapidQ's Delphi VCL controls are.
  52 of the corpus's example programs use it.
  - The placement is one shared function, `rapidr_value::layout`, with
    Delphi's `AlignControls` rules:
    - order alTop, alBottom, alLeft, alRight, then alClient;
    - each aligned control keeps its height (top/bottom) or width
      (left/right);
    - controls with the same Align are ordered by position, and the one
      whose Align, size or visibility just changed goes first (so a
      splitter created before its tree ends up to the right of it);
    - invisible controls are skipped.
  - Layout works on the stored Left / Top / Width / Height, so a program
    reads the aligned size immediately (`Grid.Align = alClient : PRINT
    Grid.Width`), before or after the form is shown.
  - A container is laid out again when:
    - an aligned child changes Align, position, size or visibility;
    - the container itself is resized;
    - a form gets its main menu, which takes the top of the client area.
  - QSTATUSBAR is alBottom and QSPLITTER alLeft by default, as in RapidQ.
    The status bar now takes its space from the client area instead of
    covering the controls behind it (web) or being drawn over them
    (desktop).
- **Desktop: live geometry.** Setting Left / Top / Width / Height on a
  control that is already shown moves or resizes it; before, only the
  stored value changed. `Form.Left` / `Form.Top` move the window, and
  `Center` records where the form went.
- **Desktop: resizing a form by hand** updates Width / Height, lays out its
  aligned controls and fires `OnResize`. Other controls keep their places,
  as in RapidQ; before, FLTK scaled every control in proportion.
- **Web:** maximizing or restoring a form lays it out again and fires
  `OnResize`; dragging a form updates its Left / Top.
- `ClientHeight` is the form's height less its in-window main menu (on the
  desktop, except on macOS where the menu is the system menu bar).

### Fixed
- **Panels show their Caption, centered.** On the web, setting a panel's
  Caption no longer deletes the controls inside it.
- **Web geometry reads:** Left / Top read what the program or the layout
  set; Width / Height fall back to the stored value while an element isn't
  rendered. Before, a control on a form not yet shown read 0.
- **Web defaults match the desktop:** the splitter is 5px wide and the
  status bar 24px high.
- **Test hooks:** captures (`RAPIDR_CAPTURE`) draw what event handlers just
  changed; interpreted programs' captures could show the previous state.

### Tests
- `tests/fixtures/align_layout.bas` in `tests/native_gui_events.mjs`: every
  Align, the splitter's default, hiding a control, and a form resized as by
  the user. The new `RAPIDR_TEST_RESIZE=w,h` hook drives the resize.
  Native and interpreted builds agree.
- `tests/web_ide_align.mjs`: the same program in the browser, including
  element positions, maximize with OnResize, and a panel's caption keeping
  its children.
- Unit tests for the layout function (edges, ordering, invisible
  controls, running out of space).

## [2.34.0] — 2026-09-26

### Added
- **QSTRINGGRID works like RapidQ's, on the desktop and the web.** Both
  runtimes draw one shared model (`rapidr_value::objects::grid`), so a
  grid behaves the same natively, interpreted, and in the browser.
  - `Cell(col, row)` / `Cells` read and write, `ColCount` / `RowCount`
    (5 each by default), `FixedCols` / `FixedRows` (1 each),
    `DefaultColWidth` (64) / `DefaultRowHeight` (24), `ColWidths(i)`,
    `RowHeights(i)`, `Col`, `Row`, `TopRow`, `LeftCol`, `GridWidth`,
    `GridHeight`, `EditorMode`.
  - `InsertRow`, `DeleteRow`, `InsertCol`, `DeleteCol`, `SwapRows`,
    `SwapCols`, `AddOptions`, `DelOptions`, `ColumnStyle(i)`,
    `ColumnList(i)`.
  - `SaveToFile` / `LoadFromFile` / `SaveToStream` / `LoadFromStream`
    `(file or stream, RowOffset, ColOffset, MaxRows)`: rows of cells joined
    by `Separator`.
  - **Desktop:** a real table widget (FLTK `Table`), not rows of text
    boxes. It has a shaded fixed header row and column, per-column widths
    and per-row heights, scrolling, and a highlighted selected cell.
  - **Web:** a table redrawn once per batch of changes.
  - **Interaction, both runtimes:**
    - clicking or the arrow keys select a cell (fixed cells can't be
      selected) and fire `OnSelectCell(Col, Row, CanSelect)` and `OnClick`;
    - with `goEditing` (`AddOptions 10`), a double-click, Enter, F2 or
      typing edits the cell in place (a single click with
      `goAlwaysShowEditor`). Enter or leaving the cell stores it and fires
      `OnSetEditText(Col, Row, Value$)` and `OnChange`; Escape cancels;
    - an ellipsis column (`ColumnStyle(i) = gcsEllipsis`) shows a button
      that fires `OnEllipsisClick(Col, Row)`.
  - Sizes are capped (at most 4 million cells), and cell text is always
    drawn as plain text.
- RapidR's own grid API is kept on the same model: `AddRow a, b, …`
  (widens the grid if needed), `SetCell` / `GetCell`, `Clear` (no rows),
  `SetRowCount` / `SetColCount`, `Cols` / `Rows` / `ColWidth`,
  `SelectedRow` / `SelectedCol`, and a "..." cell showing an ellipsis
  button.

### Changed
- **`SetCell` / `GetCell` take `(col, row)` in both runtimes**, the same
  order as RapidQ's `Cell`. The desktop runtime used `(row, col)` and the
  web runtime `(col, row)`.
- A grid starts at RapidQ's 5 × 5 with one fixed row and column, and cells
  can be edited only with `goEditing`. RapidR's IDE programs
  (`examples/ide.rr` and others) now set `RowCount = 0` before filling
  grids with `AddRow`, and `AddOptions 10, 13` where cells are edited.
- `RDataFrame.ToGrid` fills the grid through the same model on the
  desktop.

### Tests
- `tests/web_ide_grid.mjs`: cells, InsertRow/SwapRows, widths, fixed
  shading, plain-text cells, selection by click and arrow keys, in-place
  editing with `OnSetEditText`, the ellipsis button, `AddRow` / `SetCell` /
  `GetCell`.
- `tests/fixtures/string_grid.bas` in `tests/native_gui_events.mjs`: rows
  and columns, `Separator`-based streams and selection; native and
  interpreted builds agree.
- Unit tests for the grid model: defaults, row/column edits, the text
  round trip, out-of-range indexes and huge sizes.

## [2.33.0] — 2026-09-26

### Changed
- **Every RapidQ example the interpreter accepts now compiles natively.**
  Of the 116 corpus programs that compile to bytecode, 116 pass `cargo
  check` as native builds, up from 49. The native compiler now resolves
  what the interpreter already did, the same way:
  - methods, properties and indexed properties of objects that aren't
    component variables: `Application.Terminate`, `Screen.Cursors(i) = h`,
    `Printer.Printers(i)`, `RichEdit.SelAttributes.Color = c`,
    `This.Grid.Cell(x, y) = s`, `DXTimer.OnTimer = Handler`;
  - these objects are resolved by name in the main program, and by the
    component id a SUB's own parameter or local holds (the VM's rule);
  - `Cell(1, 0) = s` / `ColWidths(0) = w` inside a CREATE block set an
    indexed property of the object being created;
  - WITH blocks use the shared `rapidr_ast::resolve_with_body`, so
    `.Member` works on every kind of object, in statements and expressions;
  - variables named like builtins (`RGB$ = …`), arrays declared with a type
    suffix (`DEFSTR Rider$(1 TO 29)`), FUNCTIONs named with a suffix
    (`QSystem.OSName$`), and dotted names with a suffix in the middle;
  - a SUB's name used as a value is empty, and an event handler the program
    never defines binds nothing;
  - `BIND ptr TO Prototype` with no such routine only gives the pointer a
    signature;
  - a call with more arguments than the routine has parameters drops the
    extra ones (all are still evaluated, in order); missing ones are empty;
  - a routine nobody defines, which only unreached library code can call,
    is a run-time error, as in the VM;
  - program files whose names aren't valid Cargo package names
    (`Cancel Form Close.bas`, `3dview.bas`) build; the executable keeps the
    file's name.

### Fixed
- **Both backends.**
  - A number written with a leading dot (`SetRGBA(.1, 1, .1, .7)`) was
    read as a WITH member.
  - A TYPE method named `Init` (or `Ctor`, `Ev0`) replaced the routine that
    sets up the TYPE's instances. The generated routines are now
    `Type___init`, `Type___ctor` and `Type___ev<i>`.
  - SUBs and FUNCTIONs written inside another SUB are hoisted to the top
    level (`rapidr_ast::hoist_routines`); native builds couldn't bind them
    as event handlers.
  - `CURDIR$`, `RND` and `DIR$` work without parentheses, unless a variable
    has that name.
- **Interpreter.**
  - `REDIM a(…)` inside a SUB made a new local array instead of resizing
    the module-level one. It now resizes it, unless the SUB declares its
    own `a`.
  - `Screen.Cursors(i) = h` and `Grid.Cell(x, y) = s` on objects that
    aren't components failed or did nothing; they call the object's
    method, as native builds do.

### Tests
- Conformance case `native_parity`: leading-dot numbers, a builtin-named
  variable, suffixed arrays, REDIM of a module array in a SUB, nested SUBs,
  a TYPE method named `Init`, argument fitting, and a suffixed FUNCTION
  called without its suffix.
- `tools/corpus_native.sh <corpus.json>`: the native corpus check (was a
  scratch script).

## [2.32.0] — 2026-09-26

### Changed
- **Native builds keep numeric locals as Rust numbers.** A SUB/FUNCTION's
  local declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE or DOUBLE is
  an `i64` / `f64` instead of a boxed `Value`. Arithmetic, comparisons,
  AND/OR/XOR/NOT, IF/WHILE/DO conditions and FOR loops over such variables
  compile to plain Rust, with exactly `Value`'s semantics:
  - wrapping integer `+ - *`;
  - `/`, `\` and MOD by zero give 0;
  - comparisons are done as floats, with NaN treated as equal;
  - FOR evaluates end and step once.

  A local stays a `Value` when something else could store into it: it's
  passed to a user SUB/FUNCTION (a possible BYREF), its address is taken,
  it's used as an array or object, an integer counter has a fractional
  STEP, or the routine uses GOTO/GOSUB or inline Rust.
  - Benchmark (5M iterations of `MOD`, `/`, `AND` and IF in a SUB): 0.02 s
    typed, 0.13 s as `Value`s, 0.62 s interpreted; all three print the same
    result.
  - On the RapidQ example corpus, the same 49 programs `cargo check`
    natively before and after, so typed locals introduced no failures.

### Fixed
- A FOR counter's start value converts to the counter's declared type in
  both backends (`FOR i = 1.5 TO 4` with `i AS INTEGER` starts at 2).

### Tests
- Conformance case `typed_locals`: every operator on typed LONG / DOUBLE /
  SHORT / BYTE locals, including division and MOD by zero, NaN comparisons,
  overflow, negative and fractional STEPs, DIM re-run in a loop, WHILE/DO,
  a BYREF-passed local, and a typed FUNCTION.
- Unit tests pin `rapidr_value::numeric`'s typed helpers to `Value`'s
  operators on edge values.

## [2.31.0] — 2026-09-26

### Changed
- **Declared numeric types are enforced, in both backends** (RapidQ manual,
  Appendix C: a type "cannot be changed once bound"). Storing into
  something declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE or
  DOUBLE now converts the value; before, `DIM n AS INTEGER : n = 2.5` kept
  2.5 and a BYTE held 300.
  - Integers round half to even, the rounding RapidQ documents for ROUND:
    2.5 → 2, 3.5 → 4, -2.5 → -2.
  - Integers wrap to their width: BYTE 0..255, WORD 0..65535, SHORT 16-bit,
    INTEGER/LONG 32-bit, DWORD 32-bit unsigned.
  - SINGLE and DOUBLE hold floating-point numbers; a string converts like
    VAL.
  - This applies to assignments, `INPUT`, array elements, TYPE fields
    (including array fields), BYVAL parameters on entry, and a typed
    FUNCTION's result (`f = …`, `RESULT = …`, `RETURN …`).
  - One shared pass (`rapidr_ast::numeric`) inserts the conversions; the
    rules live in `rapidr_value::numeric`. The VM runs each conversion as a
    single opcode (`ToNum`); native builds call it directly.
  - Not yet: FOR counters' own increments, and type suffixes (`n%`, `n&`).

### Fixed
- **Native builds:** a SUB/FUNCTION's own `DIM x` or parameter `x` now
  shadows a module-level `x`, as in the interpreter. Native code used to
  read and write the global instead (`DIM n AS STRING` in a SUB overwrote
  the program's `n`).

### Tests
- Conformance case `numeric_types` (both backends): rounding, wrapping of
  every type, strings, arrays, TYPE fields, parameters, FUNCTION results,
  globals changed in a SUB, local and parameter shadowing, and `INPUT`.

## [2.30.0] — 2026-09-26

### Security
- **The interpreter is sound when events fire while it runs** (SEC-08).
  Both VM hosts used to reach the VM through a raw pointer from event
  callbacks. An event fired while the VM was inside a host call (an
  `OnClose` fired by `Form.Close`, a button click during `ShowModal`, a
  timer) created a second `&mut Vm` alongside the running one. That is
  undefined behaviour in Rust, and an `OnClose` handler that opened a
  dialog could leave the rest of the click handler running on the wrong
  frames. Now:
  - the runtimes only **queue** the handlers they fire; the VM runs them
    itself at safe points, right after the operation that fired them, each
    to completion before the next (`Host::take_events`);
  - the desktop's `ShowModal` and the program's windows are a wait the VM
    serves (`Host::wait_started` / `Host::pump`): it pumps UI events and
    runs their handlers between them, so nested modal forms work;
  - the browser keeps the program in a `RefCell` session, and JavaScript
    callbacks run queued handlers only while the VM is idle;
  - an event handler runs on top of the code it interrupted, with one
    frame stack. A handler that waits for a dialog, or pauses in the
    debugger, finishes on resume, and then that code continues;
  - `rapidr-vm`, `rapidr-vm-host-native` and `rapidr-vm-host-web` now
    `#![forbid(unsafe_code)]`.

  Speed is unchanged: 3M builtin calls take 1.44 s, against 1.42 s before.

### Changed
- `END` inside an event handler ends the program, including the code the
  handler interrupted (as in a native build).
- A run-time error in a handler fired during a statement stops the
  program, as it does natively. Handlers of UI events that fail are still
  reported and the program goes on.
- Event handlers fired inside another handler (e.g. `OnClose` from
  `Form.Close`) can now use the in-page dialogs; they used to fall back to
  the browser's `alert`/`confirm`.

### Fixed
- The web debugger's variables and stack trace JSON escapes names.
- Each IDE run replaces the previous program's session instead of leaking
  it.

### Tests
- VM unit tests for queued events, their order, a handler that waits for a
  dialog, a host-driven wait, and `END` inside a handler.
- `tests/fixtures/nested_modal.bas`: timers during `ShowModal`, plus a
  modal form opened and closed by a timer inside a handler, checked
  natively and interpreted.
- `tests/web_ide_reentrant_events.mjs`: `OnClose` fired inside a click
  handler shows a dialog, then both handlers finish in order. This test
  fails on v2.29.0.

## [2.29.0] — 2026-09-26

### Added
- **QSTATUSBAR panels.** `AddPanels "Ready", "Line 1"`, `Panel(i).Caption`,
  `Panel(i).Width` (100 by default; the last panel takes the rest),
  `PanelCount`, `SimplePanel` and `SimpleText` are drawn on the desktop and
  in the web runtime, and update when the program changes them.
- **QLISTVIEW columns, items and sub-items** (RapidQ manual, Appendix A),
  one data model shared by both runtimes (`rapidr_value::objects::listview`):
  - `AddColumns`, `ClearColumns`, `Column(i).Caption` / `.Width`,
    `ColumnsCount`;
  - `AddItems`, `InsertItem`, `DelItems`, `Clear`, `SwapItem`,
    `Item(i).Caption` / `.Checked` / `.Selected` / `.ImageIndex` / `.Index`,
    `ItemCount`, `ItemIndex`, `SelCount`;
  - `AddSubItem`, `InsertSubItem`, `DelSubItem`, `SubItem(i, j)`;
  - clicking a row sets `ItemIndex` and fires `OnClick` / `OnDblClick`;
    clicking a header fires `OnColumnClick(Column%)`.
  Captions are always shown as plain text (no FLTK `@` codes or HTML
  markup), and out-of-range indexes are harmless.
- `ClientWidth` / `ClientHeight` on forms and containers (RapidQ's ZIP
  viewer example sized its list view with them and showed nothing).

### Changed
- **`rapidr build --interp` makes optimized executables by default.** It
  used the debug interpreter unless `--release` was given: 328 MB and far
  slower. The runner now comes from a stripped `runner` profile, so the
  ZIP viewer is 44 MB. `--debug` still selects the debug runner. The runner
  is built once; later builds take about 0.2 s.

### Fixed
- `rapidr build --interp` (and native builds) now work from any directory:
  the CLI finds the RapidR workspace from `RAPIDR_HOME`, the current
  directory, its own executable, or where it was compiled. It used to fail
  with "could not find `Cargo.toml`" outside the repository.

## [2.28.1] — 2026-09-26

### Removed
- **Old object code in both compilers (about 800 lines).** Since v2.27.0
  every TYPE goes through the shared object front end
  (`rapidr_ast::objects`) before either backend sees it, so the interpreter
  compiler's own TYPE handling (instance setup, method lookup, PROPERTY SET
  setters, EVENT trampolines, `This` aliases) and the native code
  generator's UDT-struct path could no longer run. Both are gone; the
  compilers keep only their component paths (`Sender AS QBUTTON`, arrays of
  components, indexed sub-objects). No behaviour change: conformance 84/84
  on both backends, the RapidQ example corpus stays at 116/386, 44/44 native
  examples, GUI and web IDE suites pass.

## [2.28.0] — 2026-09-26

### Security
- **Hostile values no longer crash the runtime.** Integer `\`, `MOD` and
  unary minus wrap instead of panicking (`MIN \ -1` took the whole program
  down), and `INV` computes in 128 bits. The fix is in the shared value
  layer, so both backends have it.
- **Memory can't be exhausted by one call:**
  - strings are capped at 256 Mi characters; `SPACE$(1E12)`, `STRING$` and
    a string doubled in a loop stop with a clear run-time error instead of
    hanging while allocating;
  - the interpreter stops runaway recursion at 100,000 nested calls with a
    "stack overflow" error instead of growing without bound (native builds
    already stop safely with Rust's stack-overflow abort).
- **Fuzzing:**
  - every builtin was called with edge arguments in the interpreter (4,617
    calls; the one crash found, `INV`, is fixed);
  - 1,344 mutated RapidQ programs went through the compiler and the native
    code generator with no crashes.
- **Known issue (roadmap):** the web interpreter host reaches the VM
  through raw pointers, and an event fired from inside a running
  statement re-enters it. This works on single-threaded wasm but isn't
  sound Rust; it needs an event queue or a restructured host. (Fixed in
  2.30.0.)

### Changed
- **Faster module-level variables, in both backends.** They live in slots
  instead of a string-keyed map. The interpreter indexes by the name's
  string-table entry; native code gets `gv(3)` instead of `gv("total")`, so
  every write no longer allocates a key.
- **Faster interpreter core:**
  - array accesses don't allocate an index vector;
  - SUB/FUNCTION calls reuse their locals' storage;
  - the opcode decoders are inlined.
- **Benchmarks** (plain loop: arithmetic, a FUNCTION, an array, 5 million
  iterations): the interpreter went from 2.05 s to 0.97 s, native from
  1.22 s to 0.48 s. The object benchmark (10 million method and field
  operations) now takes 1.21 s interpreted (14.5 s in v2.26.0) and 0.66 s
  native (13.0 s in v2.26.0).

### Fixed
- **`STRING$(n, code)`** repeats the character with that code
  (`STRING$(3, 65)` = "AAA"); it took the first digit. `SPACE$` and
  `STRING$` are shared now (`rapidr_value::strings`).
- **Tests:** conformance case `edge_values`.

## [2.27.0] — 2026-09-26

### Changed
- **Objects are real values with direct field access, in both backends.**
  An instance of a TYPE is now `Value::Object`: shared by reference, with
  its fields in slots fixed at compile time, the ancestors' fields first.
  Before, fields were looked up by name in the runtime's component
  registry.
  - Native builds compile a field access to a direct vector index
    (`obj_field(&c, 1)`); the interpreter has matching `GetField`/`SetField`
    opcodes.
  - On an object benchmark (a method call plus field reads and writes, 10
    million times): the interpreter went from 14.5 s to 2.6 s (5.6×), and
    native builds from 13.0 s to 1.4 s (9.2×).
- **One object front end for both backends: `rapidr_ast::objects`.** The
  bytecode compiler and the Rust generator run the same lowering pass;
  objects become plain routines plus a few builtins that each backend
  implements. The native compiler's own object pass is gone; the
  interpreter's old object code in its compiler no longer runs and will be
  removed in a later cleanup. The two can't drift apart any more.
- **Every TYPE is an object type now.** Plain UDTs (fields only) used to be
  by-value Rust structs in native builds but shared references in the
  interpreter; both use the interpreter's reference semantics now.
- **A method's code is reachable only when it's called** (or its address
  is taken). Before, declaring an instance made every method of its TYPE
  reachable, so an unused method's DLL call could stop an interpreted
  build.

### Added
- **Arrays of TYPE objects:** `DIM a(1 TO 3) AS TType` creates and sets up
  one instance per element (ids `a(1)`, …), constructors included. The
  same holds for array fields of objects, e.g. `Parts(2) AS TPart`.
- **Self-referencing fields start empty.** A field whose TYPE leads back to
  its owner (`Link AS TItem`) starts as Nothing and is assigned by the
  program, which allows linked structures; it no longer recurses forever.
- **EVENT handlers bind to each instance at run time.** This covers array
  elements and locals too. Both runtimes gained instance-bound handlers:
  `rp_bind_event_indirect_this` for the interpreter, closures for native
  code.
- **Method pointers:** `CODEPTR(obj.Method)` and `BIND p TO TType.Method`,
  called as `CALLFUNC(p, obj, args…)`, on both backends.
- **RapidQ patterns inside TYPEs:**
  - a TYPE extending a component reaches that component's property objects
    (`Font.Size`, `Qlistviewex.font.size`, `obj.Canvas.Font.AddStyles`) and
    indexed sub-objects (`QlistviewEx.column(i).caption`);
  - `Application`, `Screen` and the other RapidQ global objects are never
    taken for component members;
  - fields of RapidQ object types RapidR doesn't implement yet (e.g.
    `QD3DVECTOR`) are property-bag objects, as before.
- **Debugger:** the IDE shows objects with their type, id and field names.
- 116 of the 386 RapidQ examples compile (was 113).

### Fixed
- **Web runtime events:** a handler with 2–5 parameters bound to an event
  without arguments was never called; handlers may now also bind or fire
  events themselves. Both runtimes use one `fire` path.
- **Native:** assigning a whole array to a module-level array variable
  compiled wrongly.
- **Interpreter:** the compiler's own helper calls inside a CREATE block
  were taken for methods of the created component.

## [2.26.0] — 2026-09-26

### Added
- **Arrays of components**, in both backends. `DIM lbl(1 TO 3) AS QLABEL`
  creates one real component per element, with ids `lbl(1)`, `lbl(2)`, …;
  more dimensions work too (`grid(0,1)`), and so do local arrays.
  - Elements take properties, reads, method calls and event handlers:
    `lbl(i).Caption = …`, `lbl(i).OnClick = Handler`, where Sender says
    which element fired.
  - The id array is built in `rapidr_value::objects::object_ids`, and both
    runtimes create the components (`rp_component_array`).
- **Indexed sub-objects of components**, in both backends.
  `SB.Panel(0).Width = 100` works, and inside `CREATE SB … END CREATE` so
  does the bare `Panel(0).Width = 100`. It becomes the method
  `panel.width=`, and the runtimes keep these as the component's
  properties.

  Status-bar panels and list-view columns don't render them yet. The
  CREATE-block rule is shared: `rapidr_ast::qualify_create_body`.
- **QFILESTREAM on the shared stream code**, the same as QMEMORYSTREAM on
  every platform:
  - `Open` with fmCreate/fmOpenRead/fmOpenWrite/fmOpenReadWrite; all the
    Read*/Write*/ReadNum/WriteNum methods, Seek, LineCount, Size and
    Position; `CopyFrom` between file and memory streams in both
    directions;
  - on the desktop, writes go straight to the file, so nothing is lost
    without `Close`; on the web, `Open` reads the page's own files;
  - writing to a file opened with fmOpenRead is refused with a warning.

  Before, desktop file streams only read and wrote whole lines.
- **`Stream.Read(var)`** (both backends): reads as many bytes as the
  variable holds (a string: its length), per the manual's "Generic Read".
  It's compiled as `var = Stream.__read(var)` (`rapidr_ast::stream_read_assignment`).
- **Tests:**
  - conformance cases `component_arrays`, `indexed_subobjects` and
    `file_streams`;
  - `tests/native_gui_events.mjs` now builds each GUI fixture both
    natively and with `--interp` and requires identical results; its new
    fixture is an array of buttons sharing one OnClick handler.

### Fixed
- **`File.EOF` is True (-1) at the end, not 1.** With RapidQ's bitwise
  `NOT`, `WHILE NOT File.EOF` never ended.
- **`rapidr build --interp` honours `CARGO_TARGET_DIR`** when it builds and
  embeds the runner.
- **Native CREATE blocks** no longer treat the compiler's own helpers as
  methods of the created component.
- 113 of the 386 RapidQ examples compile now (was 101).

## [2.25.0] — 2026-09-26

### Added
- **Native builds compile object-oriented TYPEs** (RapidQ manual ch. 10) as
  real Rust. `crates/rapidr-codegen-rust/src/objects.rs` lowers objects to
  plain routines with the interpreter's model, so both backends behave the
  same:
  - methods (SUB/FUNCTION) using `This`, `Me`, the type's own name, bare
    field names and a leading `.`;
  - CONSTRUCTORs (base TYPE first) and inheritance (`EXTENDS`);
  - `PROPERTY SET`, `RESULT`, and `obj.Func` without parentheses;
  - composition (fields of TYPE or component type are their own objects);
  - EVENT blocks bound to each instance (`EVENT OnClick`,
    `EVENT Panel.OnClick`);
  - `CREATE x AS TType … END CREATE`, array fields of objects, and objects
    passed to and declared in SUBs.

  This works on the desktop and the web target. Every conformance case now
  passes on both backends: 76 of 76, none marked as a known gap.
- **SUB parameters typed as components** (`Sender AS QBUTTON`) work in
  native builds: `Sender.Caption = …` used to fail to compile.
- **Desktop test hooks:** `RAPIDR_TEST_EVENTS=b1.onclick,…` fires events
  and `RAPIDR_TEST_DUMP=b1.caption,…` prints properties, together with
  `RAPIDR_CAPTURE`. New test `tests/native_gui_events.mjs` builds an OOP
  GUI program natively and checks each instance's clicks and Sender.

### Fixed
- **Compiled event handlers receive the Sender.** A handler with a
  parameter now gets the firing component, as in the interpreter; it used
  to get Null. This applies to both runtimes.
- **Interpreter: a bare method call inside `CREATE x AS TType`** (e.g.
  `Describe`) calls the TYPE's method. It went to the component and
  warned.
- **Tests:** new conformance case `oop_create_arrays`; `oop_types`,
  `oop_property_set` and `oop_composition` pass natively.

## [2.24.1] — 2026-09-26

### Changed
- **Native is native, interpreted is interpreted.** `rapidr build` no
  longer switches to the embedded interpreter for object-oriented
  programs. Until the Rust backend compiles OOP TYPEs, a native build of
  such a program stops with a clear error that points to the interpreter
  (`rapidr build-bc` / `run-bc`, or `--interp`). `RAPIDR_STRICT_CODEGEN` is
  gone.

## [2.24.0] — 2026-09-25

### Added
- **Function pointers in native builds:** `BIND p TO Proc`,
  `CODEPTR(Proc)` and `CALLFUNC(p, args…)`. Each SUB/FUNCTION gets an id,
  and `CALLFUNC` dispatches through a generated table (BYREF parameters
  included). An unset or wrong pointer stops with the same run-time error
  as the interpreter.
- **`rapidr build` handles object-oriented programs.** When a program uses
  TYPEs with methods, CONSTRUCTOR, EVENT, EXTENDS or PROPERTY SET, which the
  Rust backend doesn't compile yet, `rapidr build` still produces a native
  executable (or web bundle) and says so. It uses the embedded bytecode
  interpreter, exactly as `--interp` does, so the program behaves
  identically. `RAPIDR_STRICT_CODEGEN=1` turns this off; the conformance
  suite uses it so it keeps testing the Rust backend itself.

### Fixed
- **`rapidr build --interp` never embeds a stale interpreter.** It used
  whatever `rapidrintr-runner` was already in `target/`, even one older
  than the CLI, so new features were silently missing. It now always has
  cargo bring the runner up to date (a quick no-op when nothing changed).
- **Tests:** `function_pointers` now passes on both backends.

## [2.23.0] — 2026-09-25

### Added
- **Native builds (`rapidr build`) run line labels, `GOTO`, `GOSUB` and
  `RETURN`.** A routine that uses them is emitted as a state machine
  (`crates/rapidr-codegen-rust/src/jumps.rs`). This covers the main
  program, SUBs and FUNCTIONs:
  - backwards and forwards jumps, line numbers, nested GOSUB;
  - labels inside FOR/WHILE/DO/IF bodies;
  - GOSUB inside IF and loops, and `EXIT` out of such loops.

  Only statements that contain a jump target or a GOSUB are flattened;
  everything else stays ordinary Rust.
- **`STATIC` variables in native builds:** each becomes a program-wide slot
  for its routine, created once, so every call and recursion shares it.
- **Native builds report the same compile errors as the interpreter**, with
  line and column, e.g. an unknown SUB or a missing label. `rapidr build`
  runs the bytecode compiler's checks first. Errors about what only the
  interpreter lacks (DLL calls, raw memory, Windows APIs) don't stop a
  native build (`rapidr_bcgen::error_applies_to_native_builds`).

### Fixed
- **Native `SELECT CASE` inside a loop** no longer fails to compile ("value
  moved"): the tested value is copied.
- **Tests:** `gosub_goto`, `gosub_advanced`, `label_not_found`,
  `unknown_sub_error` and `static_vars` now pass on both backends (no
  longer marked xfail for codegen). New case `goto_structured`.

## [2.22.0] — 2026-09-25

### Added
- **In-page dialogs on the web, with every button.** When the bytecode
  interpreter runs a program (the IDE preview and exported bundles),
  `MESSAGEBOX`, `MESSAGEDLG`, `SHOWMESSAGE` and `INPUT` open a modal
  dialog in the page (`crates/rapidr-runtime-web/src/dialog_web.rs`):
  - it has a title and all the buttons, e.g. Yes · No · Cancel and
    Abort · Retry · Ignore;
  - the first button is the default, and Escape means "closed", as on the
    desktop.

  The program waits for the answer, in the main program or in an event
  handler. The VM suspends (`VmError::Suspended`, `Host::suspend_requested`)
  and `Vm::resume_with(answer)` continues it with its state intact, the way
  the debugger's pause already worked. Where waiting isn't possible (the
  Rust-compiled web build, or a handler fired from inside another VM call),
  the browser's dialogs are still used.
- **INPUT on the web:** an in-page text field showing the prompt; the
  typed line is echoed in the output. Console programs in exported bundles
  can now ask questions.

### Fixed
- **`INPUT` follows the RapidQ manual (ch. 6.4) on both backends:**
  - the prompt is printed (it was dropped);
  - a whole line is read;
  - the line is stored as text or a number according to the variable: its
    DIM type, else its suffix (`$`, `%`, `#`, …), else a number when the
    line is one. Before, `INPUT age` stored the string "41", so `age + 1`
    gave "411".

  The rule is shared in `rapidr_value::input_value`; the Rust backend
  lowers INPUT through `rapidr_ast::input_assignment`, which also fixes
  INPUT into global array elements there.
- **Tests:** the conformance runner feeds an optional `name.input` file to
  the program; new case `input_line`. `tests/web_ide_dialogs.mjs` now
  checks the in-page dialogs, including one opened from a button handler.
  `tests/web_bundle_console.mjs` checks INPUT in a bundle. A VM unit test
  covers suspending inside a function.

## [2.21.0] — 2026-09-25

### Added
- **Exported web bundles show `PRINT` output on the page.** This covers
  both `rapidr bundle-bc` and the IDE's Build. The console
  (`web-ide/bundle_console.js`) uses the same ANSI screen as the IDE's
  Output panel, so `CLS`, `COLOR` and `LOCATE` work:
  - it appears on the first PRINT;
  - it fills the page for a console program, and docks at the bottom when
    the program also shows forms;
  - it works under the IDE bundle's Content-Security-Policy.

  Before, bundles only wrote to the browser's developer console. The web
  runtime now also passes each printed text, exactly as printed, to
  `window.__rapidr_print` when a page defines it.
- **Tests:** `tests/web_bundle_console.mjs` (a console program and a form
  program built with the CLI). `tests/web_ide_e2e_build.mjs` now also
  checks the console in an IDE-built bundle.

## [2.20.0] — 2026-09-25

### Added
- **RapidQ's non-visual objects `QFONT`, `QMEMORYSTREAM`, `QBITMAP` and
  `QIMAGELIST`** (manual, Appendix A), no longer empty placeholders. They are
  written once, in `rapidr_value::objects`, and the desktop and web runtimes
  both use that code, so the two behave the same:
  - `QFONT`: Name, Size, Color, Bold/Italic/Underline/StrikeOut, `AddStyles`,
    `DelStyles`. `Label.Font = Font` applies the font to the component, on
    the desktop as well as on the web (desktop widgets didn't apply fonts
    before).
  - `QMEMORYSTREAM`: Position, Size, LineCount; `WriteStr`/`ReadStr`,
    `WriteLine`/`ReadLine`, `WriteNum`/`ReadNum` (the `Num_*` types),
    generic `Write`, `Seek`, `CopyFrom` (from another memory stream or a
    QFILESTREAM), `Close`.
  - `QBITMAP`: Width/Height, `Pixel(x, y)` (read and assign), `PSet`,
    `Line`, `Rectangle`, `FillRect`, `Circle` (outline or filled),
    `RoundRect`, `Paint` (flood fill), `Draw`, `CopyRect`, `StretchDraw`,
    Transparent/TransparentColor, `LoadFromFile`/`SaveToFile` and
    `LoadFromStream`/`SaveToStream` (uncompressed BMP, read and written by
    RapidR itself, with no new dependencies). `.BMP` gives the image as a
    `data:` URL, so `Image.BMP = Bitmap.BMP` works anywhere.
  - `QIMAGELIST`: Width/Height/Count/Masked, `AddBMPFile`/`AddBMPHandle`/
    `Insert…` (strips wider than the list are split into images, and the
    mask color becomes transparent), `GetBMP`, `Draw` onto a bitmap,
    `Delete`, `Clear`.
- **`Canvas.Draw(x, y, Bitmap)`** on the desktop and the web, keeping the
  bitmap's transparent color. On the web, `Bitmap.LoadFromFile` reads files
  shipped with the page, and `SaveToFile` keeps the file for the session.
- **Assigning to an indexed property**, e.g. `Bitmap.Pixel(x, y) = c`, calls
  the component's method with the value as its last argument (both
  backends).
- **Built-in RAPIDQ.INC:** `soFrom*`, `Num_*`, `fs*` font styles, `clNone`,
  `clDefault`, `pf*`.
- **Checking desktop rendering (for tests):** with `RAPIDR_CAPTURE=<prefix>`
  set, a desktop program saves each open window (forms and dialogs) as
  `<prefix>-N.bmp` and exits. This needs no screen-recording permission.
- **Tests:** conformance case `rapidq_objects`; IDE suite
  `tests/web_ide_objects.mjs`; unit tests for the BMP codec, drawing, streams
  and image lists.

### Fixed
- **Desktop canvas drawing is relative to the canvas**, as in RapidQ, and
  clipped to it. It used window coordinates, so on a canvas at Left = 100,
  `Line 0, 0, …` started at the window's corner.
- **Desktop `MESSAGEBOX`/`MESSAGEDLG`:** Enter now chooses the first button
  (Yes/OK), as in RapidQ. FLTK's stock dialog defaulted to the middle one,
  which was "No" on Yes/No/Cancel.
- **Desktop labels are left-aligned** by default (RapidQ's taLeftJustify),
  and honour `Alignment` 1 (right) and 2 (center). They were centered.
- **Web labels:** Underline and StrikeOut can be on together.

## [2.19.0] — 2026-09-25

### Added
- **`MESSAGEBOX(text, title, flags)` and `MESSAGEDLG(text, mtType, mbButtons,
  0)`** (RapidQ manual), with real buttons and the documented return values
  (IDOK/IDYES/…, mrOk/mrNo/…). The button logic lives in
  `rapidr_value::dialogs`:
  - desktop builds show an FLTK dialog with up to three buttons and a title;
  - the web uses the browser's blocking dialogs, labelled when OK/Cancel stand
    for other answers ("OK = Yes, Cancel = No"). Browsers offer at most two
    buttons, so a third (Yes/No/*Cancel*) isn't available there.
- **Built-in RAPIDQ.INC:** the `mt*` and `mb*` MessageDlg constants.
- **Tests:** IDE suite `tests/web_ide_dialogs.mjs`; bitwise operator checks
  in `operators_rapidq`.

### Fixed
- **`AND`, `OR`, `XOR` and `NOT` are bitwise, as in RapidQ.** The manual
  gives `5 OR 3 = 7` and `NOT -1 = 0`. They were logical, so
  `MB_YESNO OR MB_ICONQUESTION`, or any combined flags, gave -1. Comparisons
  are -1/0, so conditions behave as before. One change, in the shared value
  layer, fixes both backends.

## [2.18.0] — 2026-09-25

### Added
- **`REDIM a(n) AS T`** (RapidQ manual) on both backends:
  - the array is resized in place, so every variable holding it sees the new
    size;
  - each element whose index still fits is kept;
  - without an earlier DIM, it creates the array;
  - VB's `REDIM PRESERVE` is accepted;
  - one implementation, `rapidr_value::redim`, shared by both backends.
- **`a INV m`:** the modular inverse (`3 INV 26` = 9, 0 when there is none),
  at MOD's precedence.
- **Tests:** conformance case `redim_inv`.

## [2.17.0] — 2026-09-25

**Portable console, and honest Windows errors.** RapidQ's console statements
work on every platform and in the IDE. Calls into Windows itself say plainly
that RapidR doesn't emulate Windows, and what to use instead.

### Added
- **Console statements** (RapidQ appendix C) on both backends:
  - `CLS`, `COLOR [fg][, bg]` (QBasic colors 0-15), `LOCATE [row][, col]`,
    `CSRLIN` and `POS(0)`;
  - they emit standard ANSI/VT sequences (macOS, Linux and Windows 10+
    terminals), from one implementation in `rapidr_value::console`;
  - PRINT zones and the cursor ignore escape sequences.
- **The IDE Output panel renders them** (`web-ide/ansi_screen.js`): colors,
  cursor positioning and clearing, with printed text only ever inserted as
  text. Plain PRINT output looks and copies exactly as before.
- **Windows API errors name the portable alternative.** A `DECLARE … LIB`
  into a Windows system DLL (user32, gdi32, kernel32, winmm, odbc32, ws2_32,
  …) now reads: "'ShellExecute' is a Windows API function; RapidR runs on
  every platform and doesn't emulate Windows. Instead, run programs and open
  files with SHELL / SHELLWAIT". Hints cover ODBC → RSQLITE/RMYSQL, GDI →
  RCANVAS, window management → component properties, MCI → PLAYSOUND,
  Winsock → RSOCKET, WinINet → RHTTP and more. A program's own DLLs keep the
  native-build message.
- **Omitted arguments:** `INSTR(, a, b)`, `COLOR , 1`, `LOCATE , 5`.
- **Tests:** conformance case `console_ansi`, IDE suite
  `tests/web_ide_console.mjs`, and a unit test for the Windows API messages.

### Fixed
- **Bare `TIMER` in the interpreter.** It (and `TIME$`, `DATE$`, `COMMAND$`,
  `PI`) evaluated to Null when written without parentheses; native builds
  already called them. Both backends now share one list of such built-ins.

## [2.16.1] — 2026-09-25

**Open-source credits.** RapidR ships 478 open-source Rust libraries plus
native libraries such as FLTK and FreeType. All of them are now credited,
and the list can't drift out of date.

### Added
- **`THIRD_PARTY_NOTICES.md`:** every library in the shipped dependency
  graph, with version, license and upstream link. It's generated by
  `tools/third_party_notices.py` from `cargo metadata`, and CI fails when it's
  stale (`--check`). License clarifications come from `deny.toml`, so the
  notices and `cargo deny` agree.
- **`LICENSES.md` §7** credits the native C/C++ code that `-sys` crates build
  or link (FLTK, FreeType with its required notice, zstd, LZ4, zlib, OpenSSL,
  fontconfig, ALSA), which `cargo deny` can't see.
- **Web bundles** (`rapidr bundle-bc` and the IDE's Build) now include
  `LICENSE-RapidR.txt`, `THIRD_PARTY_NOTICES.md` and `LICENSES.md`, since they
  redistribute the runtime.
- **The IDE's About dialog** has an "Open-source credits" button, and the
  README has a credits paragraph.
- **ROADMAP policy:** no Windows-only compatibility; RapidQ features map to
  portable Rust; new dependencies must be permissive open source and be
  credited.

### Fixed
- **`web_ide_bugfixes`** expected an old About-dialog wording; it now checks
  the current credits text and the credits dialog.

## [2.16.0] — 2026-09-25

**RAPIDQ2.INC compiles.** RapidQ's official extension library (about 9,000
lines, 287 routines) now compiles and runs in the interpreter. It uses
composition, function pointers, SUBI/FUNCTIONI methods and more. Corpus: 96/386
programs compile, stricter than before: typos in a program's own code are
reported even in routines nothing calls.

### Added
- **Code the program never runs doesn't block it.** An error only counts in
  code the program can reach. Reachability follows every name mentioned,
  including handlers and BIND. This applies to:
  - native-only features (DLL calls, VARPTR, `@` to a DLL), anywhere;
  - unknown names and unsupported features, but only inside `$INCLUDE`d
    libraries. The program's own code is always fully checked.
- **Clearer messages for RapidQ built-ins RapidR lacks:** `LOCATE (a RapidQ
  built-in) isn't supported yet` instead of "Unknown SUB". The list comes from
  RapidQ's KEYWORD.LST and manual.
- **Composition** (manual 10.5), in the interpreter:
  - `Panel AS QPanel` fields become each instance's own component;
  - TYPE fields of TYPE type become objects;
  - `EVENT Panel.OnClick` handlers;
  - nested access at any depth (`A.Engine.Power = 5`, `b64.src.Close`,
    `A.Engine.Describe`);
  - arrays of objects (`image(1000) AS QBITMAP`);
  - a component's property objects (`.Canvas.Font.AddStyles`);
  - indexed sub-objects (`.column(i).caption`).
- **Function pointers:** `BIND ptr TO Proc` (or a TYPE method), `CODEPTR`/
  `CALLBACK`, and `CALLFUNC(ptr, …)`, with the new opcode `CallIndirect`
  (0x78). `BIND ptr TO Prototype` only gives the pointer a signature, as in
  RAPIDQ2.INC.
- **`SUBI`/`FUNCTIONI`** (manual ch. 9): variable arguments read through
  `ParamStr$(i)`, `ParamVal(i)`, `ParamStrCount` and `ParamValCount`. They
  work as TYPE methods and with `DECLARE`, on both backends.
- **`DATA`/`READ`/`RESTORE [label]`** on both backends: unquoted items are
  text unless numeric, and the table is shared by all DATA lines of the
  program.
- **`$ESCAPECHARS ON`:** `\n \t \" \\ \a \b \f \r \v`, `\65` and `\x41`.
- **`SWAP a, b`, and `SHL`/`SHR`** (32-bit, at `*` precedence).
- **Syntax:**
  - dotted names: `DECLARE SUB SLEEP.ms LIB …` and `FUNCTION Screen.MousePresent`
    (called as `Screen.MousePresent`);
  - names starting with digits (`SUB 01click`);
  - keywords as parameter names (`type AS LONG`);
  - `FOR i = 1 TO 3: PRINT i: NEXT` on one line;
  - `FOR THIS.x = …`;
  - `CASE = x`;
  - `< =` / `> =` / `< >` written with spaces;
  - `_ ' comment` continuations;
  - `WITH TypeName` around a TYPE's members;
  - `EXIT SUBI` / `EXIT FUNCTIONI` / `EXIT EVENT` / `EXIT PROPERTY`;
  - a SUB closed by `END FUNCTION` (and vice versa);
  - `SIZEOF(SINGLE)`;
  - array fields with bounds (`Colors(1 TO 16)`).
- **RapidQ objects RapidR has no component for yet** (QFONT, QBITMAP,
  QIMAGELIST, QMEMORYSTREAM, …) are accepted as types: generic objects whose
  missing methods warn at run time.
- **Tests:**
  - conformance cases `subi_functioni`, `data_read`, `oop_composition` and
    `function_pointers`;
  - `rapidq_syntax` and `operators_rapidq` extended;
  - reachability unit tests.

### Fixed
- **`BIND ptr TO Proc` did nothing.** The interpreter only printed a warning
  and native builds wrote a comment, so the statement was silently skipped.
  The interpreter now implements it; native builds report it clearly.
- **Labels only used by `RESTORE`** no longer stop native builds.

## [2.15.0] — 2026-09-25

**Real RapidQ programs.** The original RapidQ distribution (386 example
programs, 126 include files and the manual) is now the compatibility corpus.
`tools/rapidq_corpus.py` compiles every program and ranks what blocks them.
46/386 compiled at the start of this release and 82/386 at the end. Semantics
come from the RapidQ manual, not guesses.

### Added
- **Includes like RapidQ on Windows:**
  - `RAPIDR_INCLUDE_PATH` names RapidQ's `include\` directory.
  - `\` separators and case-insensitive names work.
  - Absolute paths (`c:\rapidq\include\windows.inc`) resolve by trying shorter
    endings of the path.
  - Windows-1252 source files are decoded.
  - `$DEFINE`s and `$MACRO`s from an include reach the program that includes it.
  - `WIN32` is predefined, as in RapidQ.
- **Errors in included code point at the include file and line.** Before, they
  showed a shifted line in the main file.
- **VB conditional compilation:** `#If … Then`, `#ElseIf`, `#Else`, `#End If`
  and `#Const`.
- **The full RapidQ `DIM` grammar:**
  - `DIM a AS INTEGER, s AS STRING`;
  - `DIM (a, b, c)(5) AS LONG`;
  - `DIM x AS INTEGER = 5`;
  - `AS STRING * 20`.
- **`DEFINT`, `DEFSTR`, `DEFLNG`, `DEFDBL` and the rest of the DEFxxx family,**
  with initializers: `DEFINT a(1 TO 3) = {1, 2, 3}` is filled in memory order,
  as the manual describes.
- **`STATIC` variables** in SUBs and FUNCTIONs, in the interpreter. The value
  survives between calls and is shared by recursive calls, exactly as in the
  manual's example.
- **Operators and syntax:**
  - `i++`, `i--`, `x += y`, `-=`, `*=`, `/=`, `&=`;
  - string index `s$[i]`;
  - string subtraction `"jello" - "l"` gives `"jeo"`;
  - `a NOT= b`;
  - literal type suffixes `0&`, `1.5!`, `2#`;
  - `CASE 1: stmt` and `CASE ELSE : stmt`;
  - `Foo : Bar` calls;
  - VB `Public`/`Private`/`Global` modifiers.
- **`@var` passes a variable by reference** (manual 3.5, `StrCat(@A$, "!")`),
  on both backends. For a DLL call, native builds pass its address.
- **Array parameters:** `SUB Fill (list() AS STRING)`.
- **SUBs named like file keywords** (`SUB Close`, `DECLARE SUB Open`), which
  RapidQ doesn't reserve.
- **`RESULT = value`** in FUNCTIONs.
- **Accented identifiers** such as `Précédent`.
- **String continuation:** a string can continue onto the next line with `_`.
- **RapidQ OOP from manual chapter 10 (interpreter):**
  - `Field AS LONG PROPERTY SET Setter` with `PROPERTY SET Setter (v) … END PROPERTY`;
  - the type's name standing for the instance (`TForm.Focus`, `WITH TForm`);
  - `EXTENDS QObject`;
  - `TYPE X AS QFORM`;
  - `PUBLIC:`/`PRIVATE:`/`PROTECTED:` sections;
  - `DECLARE` lines and `AS EVENT(…)` fields inside a TYPE;
  - `obj.Func` calls a FUNCTION method without parentheses.
- **`True`/`False` in the built-in RAPIDQ.INC**, 1 and 0 as in the real one.
- **AST walkers** `rapidr_ast::walk`, `walk_expressions_mut` and
  `ref_argument_positions`.
- **Tests:**
  - conformance cases `dim_forms`, `rapidq_syntax`, `static_vars`,
    `operators_rapidq`, `with_result` and `oop_property_set`;
  - preprocessor tests for includes and `#If`.

### Changed
- **Comparisons print as `-1`/`0`** (RapidQ has no boolean type), not
  `True`/`False`.
- **Precedence follows the manual.** `NOT` binds looser than comparisons, so
  `IF NOT x = 5` means `NOT (x = 5)`. `MOD` binds looser than `*` and `/`.
- **`DIM a, b AS LONG` makes `a` a VARIANT.** The manual says only `b` is
  LONG. Before, both were LONG.
- **Unterminated strings end at the end of the line,** as RapidQ does,
  instead of being an error.
- **Native builds reject `STATIC` inside a SUB/FUNCTION** with a clear
  message, like OOP TYPEs.

### Fixed
- **Typed variables start at their type's default in the interpreter.** An
  uninitialised `DIM n AS INTEGER` printed as empty; it now prints `0`.
- **`WITH obj … END WITH` works in the interpreter.** Every `.Member = v`
  inside it silently did nothing.
- **Bare FUNCTION names are called** (`y = Five + 1`) on both backends.
  Before, they read an empty variable.
- **Every compile error has a line number.** Errors raised while lowering a
  statement (e.g. unsupported nested member assignment) used to stop
  compilation without a position; they're now reported with their line and
  compilation continues.
- **The lexer no longer crashes** on non-ASCII text right after a line break.

## [2.14.0] — 2026-09-25

**RapidQ object-oriented TYPEs** in the bytecode interpreter (IDE preview and web
bundles). The ROADMAP's `rq3.bas` (`TYPE TMyForm EXTENDS QFORM` with an `EVENT OnClick`
block and a `CONSTRUCTOR`) now runs as written. Checked in the browser: the form opens
at 200 px with caption "Custom", and each click updates the caption with the
instance's own counter.

### Added
- **`TYPE … EXTENDS <component>`** creates a real component. The fields, methods,
  events and constructor all belong to the type, and every instance gets its own
  `This`.
- **`EVENT Name … END EVENT`** blocks inside a TYPE, also written `EVENT Name(params)`
  and `EVENT(Name)`. They are wired to the instance when it's created. A missing
  `END EVENT` is an error.
- **Methods** (`SUB`/`FUNCTION` inside a TYPE), called as `obj.Method args` or
  `x = obj.Func(...)`.
- **`CONSTRUCTOR … END CONSTRUCTOR`**. Constructors run when the instance is created,
  base type first, so derived types can override defaults.
- **Inheritance between user types** (`TYPE TLoud EXTENDS TCounter`). Methods,
  fields and constructors are inherited, and circular `EXTENDS` is guarded.
- **Implicit members:** inside TYPE code, a bare field or component property name
  (`Caption = "x"`, `Count = Count + 1`) means `This.<name>`. Locals, globals and
  functions still win.
- **`Sender` in event handlers:** handlers assigned with `OnClick = MySub` receive
  the firing component. `SUB MySub (Sender AS QBUTTON)` can then set
  `Sender.Caption`.
- **Objects as values:** a TYPE or component instance can be passed to a SUB
  (`SUB UseIt (obj AS TCounter)`) and used through the parameter: fields, methods,
  and component properties and methods.
- **Array fields** in TYPEs (`Names(2) AS STRING`), read and written as `obj.Names(i)`.
- New opcodes for this: `GetPropDyn` (0x75), `SetPropDyn` (0x76) and
  `CallMethodDyn` (0x77). They access members of an object whose identity is only
  known at run time (`This`, `Sender`, parameters).
- Tests:
  - conformance case `oop_types`;
  - parser tests for EVENT, methods and constructors;
  - `tests/web_bundle_oop.mjs`, a browser test covering per-instance event state
    and `Sender`.

### Changed
- Unknown lines inside a TYPE are now errors ("Unexpected 'X' inside TYPE T").
  Before, they were silently skipped. The same applies to `PROPERTY` declarations,
  which aren't supported yet.
- Native builds (`rapidr build`) reject TYPEs that use `EXTENDS`, methods, EVENTs or a
  CONSTRUCTOR with one clear `compile_error!`. It says to use the bytecode
  interpreter instead of producing confusing rustc errors.

### Fixed
- Assignments inside a `CREATE … END CREATE` body (`Caption = "x"`) no longer register
  a global variable with the property's name.
- The VM ignores surplus arguments to a SUB instead of writing past its locals.

## [2.13.0] — 2026-09-24

**RapidQ source compatibility:** classic RapidQ programs compile and run unchanged.
Checked with a RapidQ GUI program (`$INCLUDE "RAPIDQ.INC"`, `CREATE Form AS QFORM`, a
nested `QBUTTON`, `Center`, `OnClick`, `ShowMessage`) running in the browser: the form
is centered, the button's handler runs, and the caption updates.

### Added
- **RapidQ component names:** `QFORM`, `QBUTTON`, `QSTRINGLIST`, … (any `Qxxx` whose
  `Rxxx` exists, plus `QGAUGE` → `RPROGRESSBAR`) are accepted in `DIM`, `CREATE`,
  `TYPE … EXTENDS` and parameter types. Before, `QFORM` silently created nothing.
  The component list now lives in one place (`rapidr_ast::COMPONENT_TYPES`) instead of
  a copy per backend.
- **Built-in `RAPIDQ.INC`:** `$INCLUDE "RAPIDQ.INC"` works without the file. It
  provides the RapidQ/Delphi constants: colors (BGR, as RapidR's runtimes expect),
  `mr*`, `MB_*`/`ID*`, `bs*`, `ws*`, `al*`, `mb*`, `fm*`, `VK_*`. It expands to one line,
  so error line numbers after it stay correct. A real `RAPIDQ.INC` next to the program
  still takes precedence.
- **`?` as shorthand for `PRINT`**.

### Fixed
- Native builds: a forward declaration (`DECLARE SUB Foo (…)` for a SUB defined later)
  generated a duplicate function and failed to compile.

### Verification
- New conformance cases `question_mark_print` and `rapidq_program`; 38 of 42 backend
  runs pass (the 4 known failures are codegen GOTO/GOSUB and its unknown-SUB message).
- Parser tests for Q-name mapping; a preprocessor test for the built-in RAPIDQ.INC.
- RapidQ GUI program run in Chromium as a web bundle. 42/44 examples compile; all IDE
  suites pass.

### Known issues
- `$INCLUDE` of a real file splices its lines in, so errors reported after it point at
  the wrong line (source maps needed).

## [2.12.0] — 2026-09-24

Classic BASIC control flow: line labels, `GOTO`, `GOSUB`/`RETURN`, and `END`. The bytecode
interpreter (IDE, `build-bc`/`run-bc`, bundles, `--interp`) now passes every conformance
case.

### Added
- **Line labels, `GOTO` and `GOSUB` / `RETURN`** in the bytecode interpreter.
  - Labels can be `Name:` or line numbers (`100 PRINT …`), anywhere in the main program
    or inside a SUB/FUNCTION. Jumps stay within their routine.
  - GOSUB can nest; `RETURN` goes back to the latest GOSUB, and otherwise returns
    normally.
  - `Name:` stays a call when `Name` is a SUB or builtin (`DoEvents: x = 1`).
  - A missing or duplicate label is a compile error with its location.
  - New opcodes `Gosub` and `GosubRet`.
- Native builds (Rust codegen) refuse labels/GOTO/GOSUB with a clear compile error
  pointing to the interpreter, instead of generating code that runs wrongly.
  State-machine lowering for codegen is planned.

### Fixed
- **A bare `END` statement was silently ignored**, so programs ran on into the code
  after it (typically their GOSUB subroutines). It now ends the program. In the
  interpreter it also stops execution in the browser, where the END builtin only
  logged a message.

### Verification
- Conformance: 34 of 38 backend runs pass. The VM passes all 19 cases; the 4 known
  failures are codegen's GOTO/GOSUB refusal (3 cases) and its unknown-SUB message.
  New cases: `gosub_advanced` (nested GOSUB, GOSUB in a SUB, backwards GOTO, line
  numbers, `DoEvents:`, END) and `label_not_found`.
- GOSUB and END checked in the browser IDE. Parser tests for labels, GOTO/GOSUB and
  bare END. 42/44 examples compile; all IDE suites pass.

## [2.11.0] — 2026-09-24

Real arrays, correct PRINT output, and more RapidQ compatibility, now identical on both
backends. The conformance suite passes 31 of 34 backend runs; only GOTO/GOSUB and
codegen's unknown-SUB message remain.

### Added
- **Real BASIC arrays** (`Value::Array`), shared by the bytecode VM and the Rust codegen:
  - any number of dimensions and real bounds (`DIM a(10)` is 0 to 10;
    `DIM b(1 TO 5, 3)`), with each element set to the type's default;
  - `LBOUND(a [, dim])` / `UBOUND(a [, dim])`;
  - arrays passed to a SUB are shared, as in BASIC;
  - an out-of-range index is a run-time error ("Subscript out of range"), not a silent
    `null`.

  Before this, interpreter arrays were comma-separated strings: a string element
  containing a comma corrupted the array, numbers came back as strings, `DIM` didn't
  allocate, and 2-D arrays didn't work. Codegen arrays were 1-D only, and `LBOUND`/
  `UBOUND` always returned 0.
- **PRINT separators and print zones.** `;` joins items, `,` moves to the next 14-column
  zone (as in QBasic and VB), and only a trailing `;` or `,` keeps the cursor on the line.
  Before, any `;` suppressed the newline, codegen put spaces between items, and the VM
  put none.
- **`INC x [, n]` / `DEC x [, n]`** (RapidQ), on both backends; works for variables and
  array elements.
- **`REPLACESUBSTR$(s, find, replacement)`** (RapidQ), on both backends.
- Arrays appear in the debugger's variable view as lists.

### Fixed
- **String functions count characters, not bytes** (`LEN`, `MID$`, `LEFT$`, `RIGHT$`,
  `INSTR`, `RINSTR`, `INSERT$`, `DELETE$`), so text such as `"héllo"` or `"ñandú"` works.
  Before, it gave wrong results or crashed the program. `ASC` returns the character code
  (`ASC("é")` = 233, matching `CHR$`). These functions now live in one shared module
  (`rapidr_value::strings`) instead of a copy per runtime.
- **IDE Output panel:** every PRINT used to add a blank line, and `PRINT "a";` broke the
  line. Browser output is now line-buffered, and a partial line is flushed when the
  program or event handler finishes.

### Changed
- Bytecode: `NewArray`, `AGet` and `ASet` take a dimension/index count, and `ASet`
  updates in place; new `PrintZone` opcode. Bytecode from older compilers must be
  recompiled; the IDE and bundles always compile fresh.
- Codegen: arrays are ordinary variables holding `Value::Array` (`rp_get`/`rp_set`);
  the separate `GARRS` store is gone.

### Verification
- `cargo test --workspace`, including new tests for the shared string module, and
  conformance: 31 pass / 3 known failures / 0 failed.
- The array-using examples `test_array`, `hang_test` and `ide` (2,300 lines) build with
  codegen, and `test_array` prints the same values on both backends. 42/44 examples
  compile to bytecode (the other 2 are native-only).
- All IDE suites pass. IDE Output checked for `;`, `,` and trailing partial lines.

### Open questions (need real RapidQ to confirm)
- The 14-column print zone width, `INSERT$` argument order, and how non-whole numbers
  are formatted (`0.1 + 0.2`).

## [2.10.1] — 2026-09-24

Correctness fixes found by the conformance suite (now 20 of 32 backend runs pass,
up from 11).

### Fixed
- **`CASE 2, 3` (several values in one CASE) restarted the program in an infinite loop**
  in the bytecode interpreter, because of an unpatched jump to address 0. `SELECT CASE`
  is rewritten: any item matches, then the next CASE, then CASE ELSE.
- **`CASE 1 TO 5` and `CASE IS > 10`** were misparsed (`TO 5` became a stray statement;
  `IS` was treated as a variable), so both backends picked the wrong branch. They're now
  proper case items in the parser, VM and codegen. Codegen uses BASIC comparisons, so
  `2` matches `2.0`.
- **`FOR … STEP -n` never ran** in the bytecode interpreter. The loop now tests the
  step's sign.
- **FUNCTIONs returning by name** (`Fact = n * Fact(n - 1)`) returned nothing in the
  bytecode interpreter. The function's name is now its result variable, returned at
  `END FUNCTION`, `EXIT FUNCTION` and a bare `RETURN`.
- **`BYREF` parameters** were ignored by the interpreter, and the codegen output didn't
  compile. Both now write back to the caller's variable (copy-in/copy-out, as VB does);
  the VM uses a new `LoadArgOut` opcode.
- **`EXIT SUB` / `EXIT FUNCTION`** inside a loop only left the loop, and outside one
  they were ignored. **`EXIT FOR`** inside a nested `WHILE` left the `WHILE` instead of
  the `FOR` (interpreter and codegen). Each EXIT now leaves the right construct, and a
  misplaced EXIT is a compile error.
- Whole-number results print without a decimal point: `2 ^ 10` shows `1024`, not
  `1024.0`.
- Codegen: BYVAL parameters can be assigned to (they're `mut`), as BASIC allows.

### Added
- Conformance case `exit_statements`.

## [2.10.0] — 2026-09-24

**Nothing is silently skipped anymore.** Lines the compiler couldn't parse, and calls
to routines that don't exist, used to compile anyway and quietly do nothing in the
IDE and the bytecode interpreter. They're now compile errors with a line and column,
underlined in the editor. Programs that "ran" while ignoring broken lines will now
show those errors; that's intended.

### Added
- **Errors underlined in the IDE editor.** Every compile error is shown as a red
  squiggle in the right form or module, listed in the Errors panel with
  `Form1 (line 6, col 1): …` (click to jump to the line), and re-checked live while
  you type. Run, Debug and Build show all errors instead of one raw message.
- **Parser error reporting.** Every unparseable line is reported, not just the first.
  Messages explain the problem: `'STEP' is a reserved word and can't be used as a
  name`, `GOSUB is not supported yet`, `Unexpected 'y' after the end of the statement`.
  This includes statements inside single-line `IF … THEN`.
- **Unknown SUB/FUNCTION check** in the bytecode compiler, against a shared builtin
  registry (`rapidr-bytecode::builtins`). A test keeps the registry identical to both
  VM hosts' dispatch tables. Specific hints for `INC`/`DEC`, DLL functions
  (`DECLARE … LIB`), `VARPTR`, and line labels.
- The bytecode interpreter now runs `OPEN`, `CLOSE`, `PRINT #`, `WRITE #` and `SEEK`
  (native). These statements were silently skipped before. In the browser, `PRINT #`/
  `WRITE #` report a clear error.
- `SOUND` and `PLAYSOUND` in the bytecode interpreter (both hosts).
- `tests/web_ide_diagnostics.mjs` (14 checks) and new parser unit tests.

### Fixed
- **Identifiers are case-insensitive in the bytecode interpreter**, as in BASIC.
  `total`/`Total`/`TOTAL` were three different variables, and calling `mysub` for
  `SUB MySub` silently did nothing. The debugger still shows names as written.
- **Calls inside `CREATE` blocks** (`Center`, `AddItems(…)`) now call the object being
  created, as in RapidQ and the Rust backend. The interpreter used to drop them, so
  forms were never centered in the IDE.
- A call to an unknown builtin at run time is now an error instead of `null`.
- `examples/web_datascience.rr` used `step` (a reserved word) as a variable. The
  lines were silently skipped, so its trig table computed garbage. Renamed to
  `stepSize`.
- The parser's examples test only looked at `.rp` files, so it tested nothing. It now
  checks all 44 `.rr` examples.

### Changed
- `rapidr_parser::parse_tokens` / `parse_file` return `Result<Program, ParseError>`,
  with every diagnostic. `parse_tokens_recovering` gives a best-effort tree plus
  diagnostics for tools.
- `rapidr build-bc`, `bundle-bc` and `--interp` report errors as `file:line:col`.
- Conformance suite: 11 pass / 19 known failures (was 7 / 23). `case_insensitive`,
  `syntax_error` and `unknown_sub_error` now pass.

### Verification
- `cargo test --workspace` passes, including new parser tests and the builtin-registry
  sync tests.
- 42 of the 44 examples compile with the stricter compiler. The other 2 use native-only
  features (DLL calls, `VARPTR`) and now get a clear error.
- Passing: `web_ide_diagnostics`, `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`, `web_ide_preview_isolation`, `web_ide_undo`, `web_ide_round3`,
  `web_ide_round4`, `web_ide_assets`, `web_ide_tree_validation`. (`web_ide_bugfixes` and
  `web_ide_phaseF`: known pre-existing failures only.)

### Added
- `SECURITY.md`: how to report vulnerabilities privately through GitHub's
  "Report a vulnerability", plus supported versions and scope.
- `.claude/launch.json`: a `web-ide` config that serves the repo on
  `http://localhost:8765` (open `/web-ide/index.html`). The IDE must be served over
  HTTP; opening it as a `file://` page breaks module and wasm loading.

- **Conformance suite** (`tests/conformance/`): small BASIC programs with the output
  correct BASIC must produce, run on both the bytecode VM and the Rust codegen backend,
  and part of CI. Known bugs are marked per backend and reported as known failures;
  a case that starts passing fails the run so its marker gets removed. There are 15
  seed cases. It already found about a dozen silent-correctness bugs, listed in
  ROADMAP.md; for example, identifiers are case-sensitive in the VM, and
  `CASE 2, 3` loops forever in the VM.

### Changed
- The whole `tests/` folder is now tracked. Only generated output is ignored:
  `node_modules`, `.matrix`, `.ide-matrix`, `screenshots`, `web-screenshots`, `results`.
- Ten test scripts wrote screenshots to a hard-coded personal path. They now write to
  `tests/screenshots/`, which can be overridden with `RAPIDR_SHOT_DIR`.

## [2.9.0] — 2026-09-24

### Added
- **Undo / Redo in the web IDE.** It was a stub before ("not yet implemented").
  - Works from the Edit menu, the toolbar, and **Ctrl/Cmd+Z**, **Ctrl/Cmd+Shift+Z**
    and **Ctrl/Cmd+Y**.
  - Covers every kind of project edit: adding, moving, resizing and deleting widgets,
    property-grid changes, adding/removing forms and modules, and code.
  - Each user action is one step. A whole drag or a burst of typing counts as a single
    step, grouped after a short pause.
  - History holds up to 100 steps. A new edit clears Redo. New/Open Project and loading
    an example start a fresh history.
  - Inside the code editor, Ctrl/Cmd+Z stays the editor's own fine-grained text undo.
    The toolbar and menu Undo also revert code edits.
  - Undo/Redo grey out when there's nothing to undo or redo.
  - Undo is the safety net the planned AI agent needs before it edits projects.
- `tests/web_ide_undo.mjs` (26 checks). It drives real interactions: toolbox clicks,
  a mouse drag, the property grid, the keyboard, typing in the code editor, and New
  Project. It fails on 2.8.4.

### How it works
- Snapshot-based: after an interaction settles, the project is serialized and compared
  with the last recorded state, so no individual edit site needs changing. Asset data
  (large data URLs) is shared by reference between snapshots rather than copied.

### Verification
- `web_ide_undo` passes (26/26); on the 2.8.4 IDE it fails as expected.
- Checked by hand in the browser: adding a button enables Undo, and Undo removes it
  from the designer and the project tree.
- Passing: `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`, `web_ide_preview_isolation`, `web_ide_round3`, `web_ide_round4`,
  `web_ide_assets`, `web_ide_tree_validation`. `web_ide_bugfixes` and `web_ide_phaseF` still
  fail only on their known pre-existing assertions.

## [2.8.4] — 2026-09-24

Supply-chain security release (ROADMAP SEC-09) and toolchain fix.

### Security
- Updated dependencies with published vulnerabilities (all compatible patch releases):
  - `rustls` 0.23.37 → 0.23.45: RUSTSEC-2026-0285, TLS 1.3 handshake messages accepted
    across encryption levels.
  - `rustls-webpki` 0.103.10 → 0.103.15: RUSTSEC-2026-0098 and -0099 (certificate name
    constraints) and RUSTSEC-2026-0104 (panic in CRL parsing).
  - `crossbeam-epoch` 0.9.18 → 0.9.21: RUSTSEC-2026-0204.
  - `rand` 0.9.2 → 0.9.5: RUSTSEC-2026-0097 (unsound).
  - `spin` 0.9.8 → 0.9.9 (yanked version).

  `rustls`/`rustls-webpki` are the TLS stack behind native `RHttp` (via `ureq`).
- Added `deny.toml` and **`cargo deny check`** (advisories, licenses, bans, sources). It
  now passes cleanly. The two remaining unmaintained-crate notices are documented
  exceptions: `proc-macro-error2` is compile-time only, via `mysql`, and `ttf-parser` is an
  optional `fltk` dependency that isn't enabled.
- Added CI (`.github/workflows/ci.yml`): `cargo-deny`, `cargo test --workspace`, and
  a clippy check that fails the build if `js_sys::eval` is used in the web runtime.

### Fixed
- **The native runtime and CLI failed to build on current Rust (1.98).** `ethnum` 1.5.2,
  used by polars, transmuted `()` into `TryFromIntError`, which gained a field. Updated
  to 1.5.3.
- `rapidr version` printed a hard-coded `0.1.0`; it now prints the real version.
- `rapidr-compiler-wasm` and `rapidr-webbundle` were missing the workspace MIT license.

### Changed
- All workspace crates are marked `publish = false` (their internal path dependencies
  already made them unpublishable; this makes it explicit and prevents accidental
  publishing).

### Verification
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
- `cargo test --workspace`: all 97 tests pass. The CLI rebuilds and compiles and runs
  bytecode programs.
- Lint confirmed both ways: passes on current code, fails when `js_sys::eval` is added.
- Web artifacts rebuilt. Passing: `web_ide_smoke`, `web_ide_e2e` (6/6),
  `web_ide_designer`, `web_ide_debugger_test`, `web_ide_preview_isolation`,
  `web_ide_round3`, `web_ide_round4`.
- The CI workflow hasn't run on GitHub yet; the Linux `test` job's system packages
  are untested until the first push.

## [2.8.3] — 2026-09-24

Security release: programs in the IDE preview are now isolated from the IDE.

### Security
- **Preview sandbox isolation (SEC-02, high).** The preview iframe used
  `sandbox="allow-scripts allow-same-origin"`, which cancels the sandbox: any program run
  in the IDE could read the IDE's `localStorage` (and future AI API keys), read and modify
  the IDE's DOM, and impersonate the IDE. The iframe now runs **without
  `allow-same-origin`**, in an opaque origin. The IDE sends the runtime into the frame at
  boot, because an opaque-origin page can't load same-server modules without CORS.
- **Private message channel (SEC-03, high).** IDE ↔ preview traffic used to go through
  `window.postMessage` with no sender checks, so any window could fake log, status or
  debugger events. The IDE now hands the preview a private `MessageChannel` port in a
  one-time handshake (checked with `e.source`; stale runs are dropped by a generation
  counter), and all traffic uses that port. The IDE no longer listens for preview messages
  on `window`, and the IDE window no longer imports `rapidr_run_bc`.

### Fixed
- `PRINT` output from the preview appeared **twice** in the Output panel. It is now
  delivered once.
- Downloads from programs running in the preview (`SaveToFile`, `RFileStream`) were silently
  blocked by the sandbox; `allow-downloads` is now granted.
- A forged debugger message from another window could throw an error in the IDE; such
  messages are now ignored.

### Changed
- `RWebStorage` / `localStorage` in the IDE preview now goes through a stand-in that
  saves data per project under `rapidr-app-storage:<project>` in the IDE, capped at
  1 MB. `sessionStorage` lasts for one run. Built bundles use the real browser storage,
  unchanged.
- Known limitation, IDE preview only: `RHttp` requests to relative URLs on the IDE's own
  server now need that server to send CORS headers (the `.htaccess` in `DEPLOY.md` does;
  `python3 -m http.server` doesn't). Browser APIs that need a real origin, such as
  notifications, may not work in the preview. Built bundles are not affected.

### Verification
- New `tests/web_ide_preview_isolation.mjs` (15 checks). It runs a hostile program that
  tries to read a planted fake API key from IDE `localStorage` and the IDE DOM, and to
  forge IDE messages, and then forges messages from the outside. All checks pass on 2.8.3.
  **On 2.8.2 the same test fails 11 checks**: the program read the fake key, read the
  DOM, forged the status bar, and a forged debug-pause threw an error in the IDE.
- Preview dialogs (`ShowMessage`) and downloads confirmed working in Chromium.
- Passing: `web_ide_smoke`, `web_ide_e2e` (6/6, same preview output sizes as before),
  `web_ide_designer`, `web_ide_debugger_test` (pause and step over the port),
  `web_ide_round3`, `web_ide_round4`, `web_ide_assets`, `web_ide_tree_validation`.
  Three tests (`round4`, `bugfixes`, `phaseF`) now inspect the preview through Playwright's
  frame API. `web_ide_bugfixes` and `web_ide_phaseF` still fail on assertions that fail
  identically on 2.8.2 (About-dialog credits, project restore), unrelated to this change.

## [2.8.2] — 2026-09-24

Security hardening release (Roadmap Phase 0, Sprint 1).

### Security
- **Build server no longer reachable from the network or from websites (SEC-01, critical).**
  `rapidr-buildserver` compiles arbitrary code, including `RUSTSTART` blocks built with cargo.
  It previously listened on `0.0.0.0` with fully permissive CORS. It now:
  - binds to `127.0.0.1` by default (`RAPIDR_BUILDSERVER_HOST` overrides, with a warning);
  - rejects requests whose `Host` or `Origin` is not loopback, blocking cross-site
    requests and DNS rebinding;
  - only answers CORS preflights from loopback origins.
- **Build server path traversal fixed (SEC-11).** `/preview/<id>/../..` could read files
  outside the build directory; paths with `..` or absolute components are now rejected.
- **JavaScript injection in `RStringList.SaveToFile` fixed (SEC-04).** List contents were
  spliced into `eval`'d JavaScript with incomplete escaping. Downloads now go through a
  Blob URL with the content passed as data.
- **Captions and `PRINT` no longer interpret HTML (SEC-05).** `Caption`/`Text` on buttons
  and other elements used `innerHTML`, and `PRINT` appended to `#rr-console` as HTML, so
  displaying database, HTTP or other untrusted data could inject scripts. Both now use
  plain text. Use `RDOM.InnerHTML` or `RWebView.HTML` when markup is intended.
- **Removed string-built `eval` from the web runtime (SEC-06).** `RHttp.Get/Post`,
  `BEEP` and `SOUND` now call `XMLHttpRequest` and Web Audio through `web_sys`.
  `RJavaScript.Eval` is the only remaining `eval`, by design.
- Added `clippy.toml` banning `js_sys::eval` so it can't be reintroduced accidentally.

### Fixed
- `RHttp`, `BEEP`, `SOUND` and `RStringList.SaveToFile` now work in bundles built by the
  IDE, whose Content-Security-Policy blocks `eval`.
- `RHttp` no longer mis-parses response bodies (the hand-rolled JSON round-trip was removed).
- `BEEP`/`SOUND` reuse one `AudioContext` instead of creating one per call (browsers cap them).

### Added
- `ROADMAP.md` — the plan toward full RapidQ/VB compatibility, the debugger, AI features
  and the security audit.
- `CHANGELOG.md` (this file).

### Verification
- Build server: unit tests for loopback host/origin and path checks; live requests confirmed
  local requests succeed while cross-site, DNS-rebinding and `null`-origin requests get 403.
- Web runtime: a hostile test program (HTML payloads in captions and `PRINT`, an injection
  payload in `SaveToFile`, `RHttp` GET/POST, `BEEP`) run in Chromium both without a CSP and
  under the IDE bundle's CSP — no script execution, no CSP violations, all features working.
- Existing Playwright suites pass: `web_ide_smoke`, `web_ide_e2e` (6/6), `web_ide_designer`,
  `web_ide_debugger_test`.

## [2.8.1] — 2026-06-01

### Fixed
- Dynamic `RDOM` tag swapping to fix background CSS leakage.

## [2.8.0]

### Added
- Visual widget outline, Assets Manager explorer, and module/form identifier
  sanitization suggestions.
