# AI assistant replays

Conversations with RapidR Studio's assistant, one user turn a line, for
`crates/rapidr-studio/examples/ai_eval.rs`: the same RAI and project tools
Studio uses, against a real model (a local OpenAI-compatible server, or a
cloud provider with a test key), headless. After each turn the proposals are
applied as if the user pressed Apply; the run prints, per turn, the tools
called, each request's prompt and reply tokens, whether the answer claims a
change no tool made, and the status Studio would show; at the end every file
is checked with check_code.

    RAPIDR_AI_EVAL_MODEL=gemma-4-e4b-it-4bit \
      cargo run --release -p rapidr-studio --example ai_eval -- \
      tests/ai-eval/tip-calculator.txt --out scratch/ai-eval/tip

Never part of the test suites: it needs a model, and a model's answers vary.

`deepseek-evening.txt` replays, in miniature, what Robert's evening with
DeepSeek (2026-10-20) showed (docs/preview4-plan.md R10, R11): a two-line fix
must come as a targeted edit (`+1 −1`, never the whole file), and the answers
follow the language of the latest message (Spanish, then English again). The
same paths run against the mock in tests/studio_flows.mjs (`ai-r3-*`,
`ai-r10-*`, `ai-r11-*`), with no model and no cost.
`RAPIDR_AI_EVAL_URL` (default `http://127.0.0.1:11234/v1`),
`RAPIDR_AI_EVAL_PROVIDER` (`compatible` or `anthropic`), `RAPIDR_AI_EVAL_KEY`,
`RAPIDR_AI_EVAL_WORKSPACE`, `RAPIDR_AI_EVAL_MAXTOKENS` (default 65535).

## The eval apps and their click tests (`apps/`)

Ten requests for a small app (docs/small-models-plan.md §5), one script each.
A script's `#!` lines are its click test: after the conversation, if the
program compiles (the compiler's own errors), it is run headless with the
test hooks (`RAPIDR_TEST_EVENTS` / `RAPIDR_TEST_DUMP`) and used as a person
would. A model names its components as it likes, so they are found by what
they show: a button by its caption, an edit by its name or the label beside
it. **Score = it compiles and every test passes**: the run prints
`== click test: …` and `== SCORE 1` (or 0).

| Line | What it does |
|---|---|
| `#! test NAME` | a new test (one run of the program, plus a baseline run) |
| `#! click 2` / `#! click /start\|go/` | a button (or anything with a caption) by its caption, exactly or by a regex; `click?`: only if there is one |
| `#! type /bill\|amount/ 100` | Ctrl+A, then the text typed into the edit whose name or label matches (`#1`: the first edit); `type?`: only if there is one |
| `#! pick #list 0` | item 0 of the first list (or one matching) |
| `#! drag #canvas 30 30 140 110` | the mouse pressed, moved and released on the first canvas (else the form) |
| `#! wait 30` | 30 event turns of 50 ms (timers tick) |
| `#! click-all` | every button once, in order (not Quit/Close/Reset/Again) |
| `#! baseline` | the steps above it are the baseline run's too (what the expectations compare with) |
| `#! file-dialog people.csv` | the Open dialog answers that file (written beside the program) |
| `#! expect /regex/` | a caption or text (not one typed into) matches |
| `#! expect-changed [/regex/]` | a caption or text differs from the baseline run (and matches); without a regex the window's picture may differ instead |
| `#! expect-count +1` / `changed` | a list's ItemCount (a grid's RowCount) is the baseline's plus one / differs |
| `#! expect-picture-changed` | the window's picture differs from the baseline run's |

Each test passes on its recipe (`examples/recipes/`):

    cargo run --release -p rapidr-studio --example ai_eval -- \
      --score tests/ai-eval/apps/scientific-calculator.txt \
      --project examples/recipes --main calculator.rr --out scratch/score

`held-out/` has three more (a BMI calculator, a dice roller, a countdown
timer) that no recipe is: what the assistant does beyond its recipes.

`--score` scores any program folder the same way (an earlier run's
`final/`). `RAPIDR_AI_EVAL_RAPIDR` is the `rapidr` that runs it (default
`target/release/rapidr`).

## An outside AI app, and the matrix (`tools/ai_eval/`)

The replays above are Studio's own assistant (RAI). Any other AI app — a coding agent,
a chat app — reaches Studio through its MCP server (`ide/mcp.inc`, docs/manual/ai-apps.md),
and sees only what the server tells it: the tool list, each tool's description, the server's
instructions. `tools/ai_eval/studio_mcp_agent.mjs` is such an app, to measure what a model
makes of those words alone:

    # Studio's tools as a model is given them
    node tools/ai_eval/studio_mcp_agent.mjs --list-tools
    # one app, a local model (Ollama, LM Studio, mlx-serve … — any OpenAI-compatible server)
    node tools/ai_eval/studio_mcp_agent.mjs --app tests/ai-eval/apps/tip-calculator.txt \
         --driver openai --model bonsai-2:2b --base-url http://localhost:11434/v1 --out scratch/run
    # the same, Anthropic's Messages API (RAPIDR_AI_EVAL_KEY, RAPIDR_AI_EVAL_WORKSPACE;
    # --keychain on macOS: the maintainer's rapidr-test-anthropic entries, in memory only)
    node tools/ai_eval/studio_mcp_agent.mjs --app … --driver anthropic --model claude-haiku-5-5 \
         --keychain --ledger scratch/cost.json --cost-cap 30 --out scratch/run
    # the guard that the harness itself still works: the tests' mock, no model, no cost
    node tools/ai_eval/studio_mcp_agent.mjs --dry --app tests/ai-eval/apps/tip-calculator.txt --out scratch/dry

(`RAPIDR` is the `rapidr` to run, `RAPIDR_AI_EVAL_BIN` the built `ai_eval` example that
scores: `cargo build --release -p rapidr-cli -p rapidr-studio --example ai_eval`.)

It starts Studio headless on a copy of `tests/fixtures/ai_eval_empty` with the MCP server on
(mode *Allow everything*; a registry, keychain, printer, documents folder and "home" of its
own under `--out`, never the user's), hands the model the tool list, sends the app's request
(one user turn a line, as ai_eval does) with a one-line system prompt plus the server's own
instructions, and makes the model's tool calls over MCP until it stops without one or a cap
(40 requests, 25 minutes, the cost cap, five identical calls in a row) ends the run. A project
the AI app makes (`create_project`) opens in Studio's default mode, *Ask before changes*; a
scripted "user" presses *Allow for This Session* whenever something waits in the AI Apps pane.
Then Studio is asked what it has open (what the AI app made lives in the editor, "unsaved":
the files are read back with `read_file`, as the user would see them) and the click test of
`ai_eval --score` scores that, exactly as for RAI. `--out` gets `transcript.jsonl` (each
request: tokens, seconds, text, calls; each tool call: tool, argument size, ok or the error
text, result size), `summary.json` (steps, tokens, seconds, where it stopped, the score),
`final/` (the files), `window.png` (Studio at the end), `studio.log`, `score.log`.
`node tools/ai_eval/show_transcript.mjs RUN_DIR` reads a transcript as lines.

`tools/ai_eval/run_matrix.mjs` runs the same apps through both drivers and any models, one
folder per cell (`--cells A:bonsai,B:bonsai,A:haiku,B:haiku`, `A` = this outside app, `B` =
Studio's RAI through ai_eval; `--rep 2` for a repeat; `--set apps,held-out,school`), and
`tools/ai_eval/analyse.mjs` turns the folders into the score table, the failure categories
and, per model, the pass rate, the seconds an app takes and how its tool calls arrive. A cell
is one run of a model that varies: repeat it and read the pass rate. Keys are never printed or
written (transcripts are redacted); Haiku's cost is counted per request in a ledger shared by
every run, and nothing starts past `--cost-cap`. Model outputs are test artefacts, not shipped.

**The models are a parameter.** A cell's model is `bonsai` (bonsai-2:2b on Ollama), `haiku`
(claude-haiku-5-5), `mock`, `ollama/<any model Ollama serves>` (`A:ollama/qwen3.5:4b-q4_K_M`:
free, local; folders are named with `/` and `:` as `_`), or a name in a `--models FILE.json`
(`{"name": {"id": "…", "url": "http://host:port/v1", "anthropic": false, "paid": false}}`).
`ollama/…` cells use Ollama's own chat API for driver A (`--driver ollama`): the context window
asked for is explicit (`--num-ctx`, default 32768: RAI's prompt and the tool schemas are large)
and thinking is off (`--think on` turns it on), which Ollama's `/v1` can't say; driver B is
RAI's own compatible provider on `/v1` and gets the server's default. Per run the harness
counts how each call arrived — in the API's `tool_calls`, as text in the reply that a parser
had to find (`<tool_call>…`), or not at all (a reply with no finish, no text and no call: an
Ollama template parser dropped a malformed call: `Ollama … tool call parsing failed` in its
log). Local models run one at a time (`ollama stop MODEL` after a model: `--unload`).

**The school set** (`school/`) is ten harder requests of the kind a school would give: a
periodic table explorer, a molar mass calculator (type H2O, Ca(OH)2 …), a chemical equation
balancer, a projectile motion simulator, an animated pendulum, planets orbiting the sun, a
predator–prey (Lotka–Volterra) chart, Conway's Game of Life, a logic gate simulator and a tiny
8-bit CPU emulator. Their click tests are looser (it compiles, runs, the key output appears,
the picture changes) but each was shown achievable: `molar-mass`, `projectile-motion`,
`game-of-life`, `logic-gates` and `pendulum` pass on a reference solution written for it
(kept out of the repository: they are not recipes). The click tests can type `+ = > < ( ) , / *`
since 2026-10-29.

**Tools for the next problem.** `studio_mcp_agent.mjs --calls calls.json` makes a list of tool
calls as an AI app would, no model, each with its own wait, to reproduce what a run showed (a
call that never answers says so after its wait); `rescore.mjs RUNS_DIR --app NAME` judges
finished runs again with the click test as it is now, from the files they left (a test that
was wrong — `/^and$/` missed the caption `&AND` — is fixed and the runs it judged are judged
again); `show_transcript.mjs` reads a transcript.
