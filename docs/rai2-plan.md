# RAI 2: the best coding agent, inside RapidR Studio (plan, 2026-10-24)

Robert, 2026-10-24: *"Make sure we have everything to make RAI the best coding agent in the world, in an IDE, and it's for RapidR."* RAI 2 is the heart of Preview 5, "Pair with AI" (master plan, wave 1). This page gathers everything decided so far, including what Robert found by building real apps with RAI (`dist/rai-app-prompts.md`), so nothing is lost. Each section becomes lanes; Robert approves mockups before any UI is built.

## 1. An agent that finishes the job
What broke on 2026-10-24: an answer was cut at a fixed 8,192 tokens while RAI was writing the program, the chat was summarised at a fixed 40,000, and the turn simply ended; Robert waited minutes, not knowing it had stopped.
- **Limits from the model, not fixed numbers:** the answer length and the context window come from the provider's model information (or a maintained table), up to the model's maximum; Settings shows them with a slider. Cost safety moves to a **budget** the user sets (per turn, per day), shown live.
- **Never stop half-way:** when an answer reaches its length limit, RAI continues the same task by itself; when the conversation needs compacting, it compacts and carries on. A turn ends only when the work is done, the user stops it, or RAI needs a decision, and it says which.
- **Plans for big jobs:** RAI writes a short plan (steps with checkboxes the user sees), works through it, saves after each step, and can resume after a restart.
- **More than 25 tool calls when the job needs them,** within the budget; parallel reads where safe.
- **It never loses work:** every change is a checkpoint (Versions), so a crash, a freeze or a bad edit is one click from undone. The web Studio writes real files and restores unsaved work after a crash (the data loss of 2026-10-24 must be impossible).

## 2. It sees: vision
- **The running program:** RAI runs it (with permission, in a safe test run), takes screenshots, clicks and types in it, reads what it shows, and fixes what's wrong ("the Save button is cut off", "the list is empty").
- **The designer and the editor:** RAI looks at the form it made and checks it against the request: sizes, alignment, readable text, light and dark.
- **The user's pictures:** paste a screenshot, a sketch or a mockup, and RAI builds the form from it, or explains an error shown in it.
- **Accessibility checks** of what it builds: contrast, keyboard order, names for screen readers.

## 3. It knows RapidR deeply
- The language registry (every component, member, builtin, with examples), the manual, recipes and the examples, searched as RAI needs them; a compact language card in every request.
- **The right target:** an app that needs files or the system is a desktop app; RAI sets the project's targets and says so (2026-10-24: a file renamer ran on the web).
- **Modern defaults:** text at the system's size, `$THEME rapidr`, RapidR names, plain words; never RapidQ comparisons.
- **Project memory:** a project notes file RAI keeps (what the app is, decisions, conventions), read at the start of every chat.

## 4. It works the way people can follow: Follow the AI
- **Live status with a gentle animation:** planning, reading `x.rr`, writing `main.rr`, designing the form, checking, running, fixing, with live token and cost counters. Never a silent counter.
- **The work happens in the IDE:** edits appear in the editor with a review (keep or undo each change), forms in the designer, runs in Output and the debugger, so students see real work in real places (the same idea as Database Studio's redesign).
- **A slim timeline** of steps (click to replay), and the plan's checklist.
- **A smart chat:** RapidR code in answers highlighted like the editor; hover a keyword, component or builtin for its quick help, click to open it in the Help pane; code blocks with Insert and Apply.
- **The redesigned panel** (memory rai-chat-redesign): SVG icons, a history drawer, a small model and effort picker, the privacy note and meters out of the way.

## 5. Safe by design
- **Permissions in plain words:** Ask each time / Allow for this session / Auto for safe actions; file changes reviewable; nothing outside the project without asking; never a password or key sent; secrets in RSecrets.
- **Safe test runs:** a sandbox for running programs RAI wrote (no network, a scratch folder, time and memory limits) unless the user allows more.
- **An audit log** of every action, and a clear "what was sent" for every request.
- **Age and cloud rules:** cloud keys for adults; for schools, local models (§6).

## 6. Any model, including free ones on your own computer
- Multi-provider (Anthropic, OpenAI, DeepSeek, compatible servers) with effort levels, and per-model limits (§1).
- **The local model manager (no server):** `RSystemInfo` reads the computer (RAM, graphics memory, disk); a catalogue of models with a fit check (runs well / slowly / too big, so a 200 GB model is never downloaded on a 16 GB computer); safe formats only; each model's licence shown and accepted by the user; the model runs inside Studio (memory: rapidr-teach-pairing-and-local-models).
- Small-model mode: shorter instructions, smaller steps, more checking.

## 7. It teaches: learn to pair with an AI
- Build / Pair / Coach / Quiz modes (memory: learn-with-rai): from "build it for me" to "guide me while I write it", with explanations of every change, predict-then-run, and the infinite-loop catcher.
- Lessons that use the real IDE, and "Save as a lesson" of any session.

## 8. Open: MCP both ways
- Studio's own MCP server (other AI apps can work in Studio, visibly, as in Database Studio), and RAI as an MCP client (tools the user adds), with the same permissions.

## 9. Measured: an evaluation suite
- **The app prompts are the benchmark:** RAI builds the 9 apps of `dist/rai-app-prompts.md` (and more), automatically, with each supported model; we measure success (it compiles, runs, passes a scripted walk), turns, tokens, cost and time, before and after every RAI change. Runs on the API credits, never the plan.
- Regressions block a release.

## 10. Order (lanes, after the reset)
1. §1 limits, continuation, compaction without stopping, checkpoints, the web data-loss fix (top).
2. §4 live status and the smart chat (mockups first), then Follow the AI.
3. §2 vision of the running program and the designer.
4. §3 targets, defaults, project memory.
5. §9 the evaluation suite (so every later lane is measured).
6. §6 the local model manager and `RSystemInfo`; §7 teaching modes; §8 MCP both ways.
Design pass with mockups for Robert first (Fable only with his OK, else Opus); lawyer review for model licences and downloads.
