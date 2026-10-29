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
Studio's RAI through ai_eval; `--rep 2` for a repeat), and `tools/ai_eval/analyse.mjs` turns
the folders into the score table and the failure categories. A cell is one run of a
model that varies: repeat it and read the pass rate. Keys are never printed or written
(transcripts are redacted); Haiku's cost is counted per request in a ledger shared by every
run, and nothing starts past `--cost-cap`. Model outputs are test artefacts, not shipped.
