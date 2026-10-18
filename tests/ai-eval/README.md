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
