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
