# RAI-TRAIN: teaching small local models RapidR

An environment and a corpus to fine-tune (LoRA) small models, Qwen3.5-2B first, so that RAI works
with them on a modest computer. **Nothing here ships in a release**: no model is published, no
training is part of a build. This folder is the groundwork (2026-10-29): the scripts, the data
generator, the guards, and a smoke run that proves the pipeline end to end.

Why: the eval of 2026-10-29 (`dist/ai-eval-2026-10-29/report.md`) showed small models failing in
the same ways — never looking at an example, writing other languages' syntax (`SET`, `+=`, `%`,
`for … in`, braces), repeating the same error, wrong paths. The corpus teaches exactly those:
look first, write RapidR, read the compiler's real message, do not repeat.

Everything big lives on the external SSD (`/Volumes/RapidRBuild/rai-train`; another place with
`RAI_TRAIN_ROOT`); the internal disk keeps these scripts only. Every script refuses to run when the
internal disk has under 20 GB free.

```
tools/rai_train/
  setup.sh             the environment (idempotent: says what it did and what was already there)
  build_corpus.py      the corpus: kinds b, c, e (build_corpus_more.py: a and d)
  validate_corpus.py   the guard: schema, compile, replay, hold-out, sources, hygiene (--selftest)
  train_lora.sh        a LoRA run with MLX (through lora_train.py: the same options, a loss that fits in memory)
  export_gguf.sh       adapter -> merged weights -> GGUF (llama.cpp) -> quantized -> Ollama
  eval_lora.sh         the eval harness (tools/ai_eval) on an Ollama model
  measure_fit.py       "Fits on a small computer?": memory and speed in Ollama
  measure_quants.sh    the size of the model at each quantization
  measure_prompt.py    how much of a small window RAI's fixed prompt takes
  check_lora_train.py  shows lora_train.py changes the loss and the gradients by bfloat16 noise only
  list_licenses.py     the licences of everything installed, for the lawyer check
  requirements-*.txt   what pip installs; *.lock.txt the exact versions it resolved
  CORPUS.md            what the last build made (counts, tokens, languages, what was left out)
  rai_corpus.py programs.py mutations.py corpus_report.py slice_data.py merge_lora.py common.sh   helpers
crates/rapidr-studio/examples/rai_check.rs   RAI's and Studio's MCP tools, replayed with no model
```

## The steps, with the commands and the times measured on this Mac (M4 Max, 48 GB)

```
tools/rai_train/setup.sh                       # 184 s the first time (4.6 GB of weights, two Python environments, the Rust builds); 17 s the next
$RAI/venv/bin/python tools/rai_train/build_corpus.py        # ~2 min: 24 s to make 3,600 conversations (7,900 tool calls run cold), the rest counts tokens
python3 tools/rai_train/validate_corpus.py --report         # 13 s (every tool answer is asked again, no cache)
tools/rai_train/train_lora.sh smoke-2b --rows 200 --iters 50 --batch 1 --max-seq 3500   # 28 min (1,682 s)
tools/rai_train/export_gguf.sh smoke-2b rapidr-smoke-2b     # 32 s merge + 43 s convert + ~20 s quantize and register
tools/rai_train/eval_lora.sh rapidr-smoke-2b --driver A --apps tip-calculator --num-ctx 8192   # 6 s for one app
```

(`$RAI` is `/Volumes/RapidRBuild/rai-train`.) `setup.sh` builds `rapidr`, `ai_eval` and `rai_check` from
this checkout into `$RAI/target` (the corpus is checked against the compiler of this very commit),
makes the two Python environments, downloads the weights and fetches llama.cpp at a pinned tag. Run
it again after a pull: a changed compiler changes the tool answers, and `build_corpus.py` makes the
corpus again against it (its cache is keyed with the compiler's version).

### 1. Environment (`setup.sh`)

| What | Version | Licence | Where from |
|---|---|---|---|
| mlx | 0.32.4 | MIT | PyPI |
| mlx-lm | 0.32.2 | MIT | PyPI |
| mlx-metal | 0.32.4 | MIT | PyPI (a dependency of mlx) |
| transformers / tokenizers / safetensors / huggingface_hub | 5.22.0 / 0.23.3 / 0.8.0 / 2.4.0 | Apache-2.0 | PyPI (mlx-lm's dependencies) |
| numpy, jinja2, PyYAML, rich, typer, protobuf, regex … | see `requirements-train.lock.txt` | BSD / MIT / Apache-2.0; `tqdm` MPL-2.0 (file-level, unmodified, never shipped) | PyPI |
| llama.cpp (converter and `llama-quantize` only) | tag `b11981` | MIT | github.com/ggml-org/llama.cpp |
| torch 2.11.0 (CPU, converter only), transformers 4.57.6, gguf 0.19.0, sentencepiece 0.2.2 | pinned in `requirements-convert.txt` | BSD-3 / Apache-2.0 / MIT / Apache-2.0 | PyPI |
| **Qwen/Qwen3.5-2B** weights | revision `15852e8c1636…` (4.6 GB, bf16) | **Apache-2.0** (the repo's `LICENSE` file, checked by `setup.sh`) | huggingface.co/Qwen/Qwen3.5-2B |
| Qwen/Qwen3.5-4B weights (downloaded, not trained) | revision `851bf6e806ef…` (8.7 GB, bf16) | **Apache-2.0** (checked the same way) | huggingface.co/Qwen/Qwen3.5-4B |

The official repo has no `-Instruct` suffix: `Qwen3.5-2B` is the post-trained model (a hybrid with
linear attention, 24 layers, 248K-token vocabulary); the `-Base` repos are the pre-trained ones. The
4B (`Qwen/Qwen3.5-4B`) is fetched only with `setup.sh --with-4b`. No other model is downloaded.
`list_licenses.py` prints every package's licence from its own metadata (both environments are
permissive; the only copyleft-flavoured one is MPL-2.0 for `tqdm` and `certifi`, as unmodified
libraries of a build tool, never shipped). Nothing from here is bundled in RapidR.

### 2. The corpus (`build_corpus.py`, `validate_corpus.py`)

Our own material only: `examples/` (not `examples/rapidq/`), the recipes that are not an eval
solution, `docs/manual/` (through the real `search_manual`), the registry (`rapidr lang export`:
components, members, builtins, with their docs), RapidR's own compiler messages, and templates written
for this corpus (`programs.py`, `mutations.py`, the how-to table in `build_corpus_more.py`; every
caption in English, Spanish and Portuguese). No RapidQ corpus, no `.reference/`, no cloud-model output
(the eval transcripts included), no project outside this repository, no model-paraphrased text
(everything is deterministic and seeded: `--seed`; `paraphrased_by` is `null` in every provenance line).

What makes it different from text a model wrote:

- **Every tool answer is the real one.** `rai_check` (a small example of `rapidr-studio`) runs RAI's own
  tools — `check_code`, `find_example`, `search_manual`, `rapidr_describe`, `read_file`, `write_file`,
  `propose_edit`, `create_project` — on in-memory projects, and Studio's MCP server's wording for the
  rest (`studio_status`, the "Applied in Studio's editor" prefix). The compiler's error for a foreign-syntax
  slip is whatever the compiler says today.
- **Every program a model is trained to write compiles** (no error line from `check_code`); the
  generated component demos and builtin snippets must compile with no warning at all, and the
  builtin snippets are also run, so the output the answer quotes is real.
- **A slip shown for context is never learned.** In the "error, then fix" and "do not repeat"
  conversations the model's first try is in the prompt but outside the loss. mlx-lm's `--mask-prompt`
  learns only the last message of a row, so each learned assistant turn is its own row (the same
  conversation, cut after that turn): 3,600 conversations are about 13,000 rows.
- **Three set-ups**, as a model meets them: `lite` (RAI's instructions and 8 tools for a window under 16K
  tokens: the one a 2B-4B model uses), `full` (RAI's complete set) and `mcp` (an outside AI app: the
  server's own instructions and 26 tools, `studio_status` first).

The five kinds (counts and token sizes: `CORPUS.md`):

| Kind | What | Teaches |
|---|---|---|
| a | "How do I … with RTimer?", "What does MID$ do?" → `rapidr_describe` → answer + a program that compiles | use the registry, small correct programs, the language of the question |
| b | a request in EN/ES/PT → the example Studio sends (or `find_example`) → write → check → one sentence | start from the example; whole programs |
| c | a slip the compiler refuses (`SET x = 1`, `%`, `for … in range`, braces, `ELSE IF`, a missing `END IF` or `NEXT` inside a block, `!=`, `//`, `int n;`, `v[i]`, a trailing `;`, `THEN` left out) → the compiler's real message → the fix by `propose_edit` (or by writing the program again) | read the message, change only the line |
| d | how-to questions → `search_manual` first; change requests → `read_file` first → `propose_edit` | look before writing |
| e | the same slip twice → "let me look at how it is written" → `search_manual` / `find_example` → fix | do not repeat the same error |

**The hold-out.** `validate_corpus.py` compares every prompt and every program in the corpus (the model's
code, the answers, the tool results) with `tests/ai-eval/{apps,held-out,school}` — prompts and click tests —,
their reference solutions (`dist/ai-eval-2026-10-29/reference` of the main checkout; `--holdout-ref`) and the
recipes that are the eval's own solutions, by word set, sequence ratio, 6-word shingles, exact text and the
program's shape with names blurred. Two decisions to know about:

1. The ten recipes that implement the `apps/` tasks (calculator, tip calculator, stopwatch, to-do list, unit
   converter, quiz, drawing pad, password generator, CSV viewer, server manager) are **not** in the corpus: they
   are the reference solutions of the hold-out apps, and training on them with any phrasing of the eval's
   requests would teach the answers. The recipe search still finds them at run time (RAI sends the example
   with the request), which is the thing a model should learn to use. Likewise the four CPU recipes
   (`cpu_step`, `cpu_stack`, `cpu_timing`, `disassembler`), next to the school CPU-emulator task. Six systems
   recipes and 33 other programs of `examples/` are in. If you want the recipes in later (after the base
   model is measured), `rai_corpus.EVAL_RECIPES` says which, and the validator will then fail until the
   hold-out list is changed on purpose.
2. Requests whose recipe search would hand a model one of those recipes are not used for `lite`/`full` (the
   sample would show an example that does not fit the answer).

`validate_corpus.py` (exit 1 on any failure; `--no-replay` skips the slow check; `--selftest` proves the guards fire
on a hold-out prompt, a verbatim and a renamed eval recipe, a forbidden source and a program that does not
compile) is **not** a quick check and not in `tools/regress.sh`; run it by hand, or as an extra step of a local
regress when the corpus or the compiler changed:

```
python3 tools/rai_train/validate_corpus.py --selftest && python3 tools/rai_train/validate_corpus.py
```

### 3. Training (`train_lora.sh`) and the GGUF export (`export_gguf.sh`)

`mlx_lm lora` through `lora_train.py`: rank 8, scale 20, the last 16 of 24 layers, `--mask-prompt`, gradient
checkpointing, **batch 1** and rows up to 3,200-3,500 tokens (the `lite` prompt alone is 2.5K). `lora_train.py` changes two things
in mlx-lm for Qwen3.5 and nothing else (`check_lora_train.py` shows the loss and the gradients stay the same): the
cross-entropy over the 248K-token vocabulary runs 512 positions at a time, and the gated-delta loop of the linear-attention layers
runs in checkpointed segments (findings below: the unchanged mlx-lm took 42 GB for a batch of two and crashed). Training on
`--rows N` takes a seeded slice (`slice_data.py`: even over the kinds of turn); without it, all of `data/train.jsonl`.

`mlx_lm fuse` saves MLX's own tensor layout, which llama.cpp's converter does not read, so
`merge_lora.py` adds `scale · (A·B)ᵀ` to the Hugging Face weights themselves (same names, same
layout) and checks the logits against base+adapter. Then llama.cpp's `convert_hf_to_gguf.py` makes an f16 GGUF and
`ollama create -q q4_K_M` quantizes it with the base model's own chat template, tool-call parser and sampling
(`RENDERER qwen3.5`, `PARSER qwen3.5`). The base's Apache-2.0 `LICENSE` goes into the model.

### 4. Evaluating (`eval_lora.sh`)

The eval harness as it is (`tools/ai_eval`), one model as a cell: `A:ollama/NAME` (an outside AI app through
Studio's MCP server) and/or `B:ollama/NAME` (Studio's own RAI). Compare with the base on the same apps
(`qwen3.5:2b-q4_K_M`), repeat (`--rep 3`: a cell is one sample, not a mean), and never judge on the training
corpus: only the apps, held-out and school sets count. The model is unloaded at the end.

### The smoke run (2026-10-29), what it proves and what it does not

200 rows of the `lite` set-up (every kind of turn: 29 of each), 50 steps of batch 1 (rows up to 3,200 tokens), learning rate
1e-5, on Qwen3.5-2B, then `rapidr-smoke-2b` in Ollama (Q4_K_M, 1.3 GB), then the tip-calculator app through driver A.

| Step | Result |
|---|---|
| train | 1,682 s, 34 s a step, peak GPU memory about 20 GB (19.7 GB measured at 3,500 tokens, 8.5 GB at 1,024); validation loss 0.715 at step 1, 0.144 at step 25, 0.349 at step 50 (20 rows, noisy); 5.6M trainable parameters (0.3%), adapter 22 MB |
| merge | 124 modules merged in 32 s; logits of base+adapter and of the merged model differ by 0.375 of 22.4 (bfloat16 rounding) |
| convert | f16 GGUF 3.6 GB in 43 s (text model only: the vision tower is not exported); Q4_K_M 1.2 GB |
| Ollama | `rapidr-smoke-2b` registered, `RENDERER qwen3.5`, `PARSER qwen3.5` |
| eval | `A:ollama/rapidr-smoke-2b`, num_ctx 8192, tip-calculator: **0/1**, in 6 s: the run ended `model_stopped` after 4 requests. |

The score is not the point (50 steps on 200 rows); the run proves the pipeline end to end. It is also informative:
the model called `find_example("tip calculator …")` first and `read_file` before `propose_edit` (look first: learned),
but then changed only the comment line instead of writing the recipe it was given (copy the found example: not learned).
Only 56 conversations of the corpus have a recipe to copy (see Findings).


## Fits on a small computer?

Students and poor families have little RAM: the model has to be small, quick on a CPU and short in context.

Measured on this Mac (M4 Max, 48 GB) with Ollama 0.40.13 for the smoke model, and for the base `qwen3.5:2b-q4_K_M` beside it.
A laptop's CPU is slower than this one: scale the CPU rows down by 2-4x.

**What the model holds.** `measure_fit.py rapidr-smoke-2b` (peak resident memory of Ollama's runner while it answers; a 2,540-token prompt, then 200 generated tokens):

| Model | Download | Context | GPU: memory held / tokens per s (prompt, generation) | No GPU (`num_gpu 0`): memory held / tokens per s |
|---|---:|---:|---|---|
| rapidr-smoke-2b, Q4_K_M | **1.3 GB** | 8,192 | 1.5 GB (peak 1.7) / 3,696 and 184 | 1.9 GB (peak 2.5) / 559 and 69 |
| rapidr-smoke-2b, Q4_K_M | | 16,384 | 1.6 GB (peak 1.8) / 3,634 and 183 | 2.1 GB (peak 2.6) / 528 and 60 |
| qwen3.5:2b-q4_K_M (base) | 1.9 GB (with its vision part) | 8,192 | 1.8 GB (peak 2.3) / 3,156 and 177 | 2.2 GB (peak 3.2) / 502 and 45 |
| qwen3.5:2b-q4_K_M (base) | | 16,384 | 1.9 GB (peak 2.5) / 3,644 and 176 | 2.3 GB (peak 3.3) / 505 and 51 |

So the 2B at Q4_K_M runs in **under 3 GB of RAM, with no GPU, at about 60 tokens a second to write and 500 to read**; the context
is cheap (16K costs 0.1-0.2 GB more than 8K, because only 6 of its 24 layers keep a KV cache). A 4B would be about twice as large
(weights downloaded, not trained: `setup.sh --with-4b`).

**Sizes by quantization** (`measure_quants.sh smoke-2b rapidr-smoke-2b`; llama.cpp's `llama-quantize`, sizes only, quality not measured):

| f16 | Q8_0 | Q6_K | Q5_K_M | **Q4_K_M** | Q4_K_S | IQ4_XS | IQ4_NL | Q3_K_L | Q3_K_M | Q3_K_S | IQ3_M | IQ3_S | Q2_K | IQ3_XS |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 3.6G | 1.9G | 1.5G | 1.4G | **1.2G** | 1.2G | 1.2G | 1.2G | 1.1G | 1.1G | 1.0G | 1.0G | 1.0G | 952M | needs an importance matrix |

Below Q4 almost nothing is saved: a quarter of the 1.9 billion parameters are the 248K-token vocabulary's embedding, which
the small types barely shrink. Q4_K_M (or IQ4_XS) is the floor worth having; Q3 and Q2 would cost quality for 0.2 GB. (Ollama's own
`-q` cannot quantize a GGUF import, only safetensors; `export_gguf.sh` quantizes with `llama-quantize` first.)

**The prompt is most of a small window** (`measure_prompt.py`, Qwen3.5 tokens, a window of 8,192):

| Set-up | System prompt | + tool schemas | = before the user speaks | + median recipe (1,431) | + largest recipe (2,204) | left for the conversation |
|---|---:|---:|---:|---:|---:|---:|
| lite (RAI under 16K) | 1,599 | 932 | **2,542 (31%)** | 3,973 (48%) | 4,746 (58%) | 3,446 |
| mcp (an outside AI app) | 146 | 3,401 | 3,558 (43%) | 4,989 (61%) | 5,762 (70%) | 2,430 |
| full (RAI at 16K and over) | 4,886 | 2,262 | 7,159 (87%) | 8,590 (105%) | 9,363 (114%) | none |

`full` does not fit 8K at all (RAI already switches to `lite` under 16K), and the corpus keeps its rows under 8,192 tokens, so
the `full` conversations in it are the short ones (810 rows). The corpus' typical `lite` row is 3,400 tokens, 75% of that the
fixed prompt. Reading it costs the CPU about 5 s at 530 tokens a second, once per conversation (the prefix is cached).
A fine-tuned model does not need the language card in its prompt: a `tiny` set-up (300 tokens of instructions and the eight tools
by name) would free about 2,000 tokens of the 8K and cut training time, and is the next thing to try (idea 3 below).

## Findings and ideas

**Findings in RapidR itself** (not fixed here: out of scope; found while making the corpus, which only keeps slips the compiler refuses):

1. `IF x = 3 && x = 3 THEN` compiles ("No problems") and is **false** at run time (`||` is refused, `&&` is not): silently wrong.
2. `console.log("hello")` compiles and only warns at run time (`[WARN] console.log() not implemented for type`).
3. A `FOR` with no `NEXT` at the top level of a program compiles (inside a SUB or an IF it is refused): the loop's body is the rest of the program.
4. `print(x)`, `x == y`, `x += 1` and `x++` are valid RapidR (they are not slips; the eval report's list of "invented syntax" should not count them).
5. check_code gives `Syntax error in SET statement` and `Unexpected character: %` with no hint line: top-10 fix 4 (compiler hints for foreign syntax) has not landed. The corpus teaches the messages of today; rebuild it (`setup.sh`, then `build_corpus.py`: the cache is keyed with the compiler's version) when it does.

**Findings about training Qwen3.5 with MLX** (mlx-lm 0.32.2; this is why `lora_train.py` exists, and `check_lora_train.py` shows it changes nothing but memory):

1. Out of the box a batch of two 3,500-token rows took **42 GB** and ended in "Insufficient Memory" (and, with two Parallels VMs holding RAM, in 20 GB of swap and the internal disk down from 27 to 18 GB free). Two causes: the 248K-token vocabulary (the logits of one row are 3.5 GB in float32: now computed 512 tokens at a time), and the gated-delta layers, which mlx-lm runs as one Python step per token while training (its Metal kernel has no gradient) and whose states autograd keeps (now in checkpointed segments of 128 tokens). Now 8.5 GB at 1,024 tokens, 19.7 GB at 3,500.
2. It is slow: **34 s a step for one 3,000-token row**. The whole corpus (12,500 rows) would be about 5 days an epoch. The levers, in the order I would try them: (a) a `tiny` set-up (rows of about 1,000 tokens: 4x less memory and time); (b) a slice (2,000 rows, one epoch: about 19 h now, 5 h with (a)); (c) one pass per conversation with a loss mask on every learned turn (3.6x fewer tokens; needs our own trainer, mlx-lm masks only the last message of a row); (d) a Metal kernel with a backward pass for the delta rule (mlx), or a CUDA box with flash-linear-attention for a few hours; (e) a Qwen3 dense model (no linear attention) as the trainable base.
3. A long run should not start while the Parallels VMs are up: they hold about 17 GB resident and 29 GB compressed, the machine sat at 47 GB used, and a training that outgrows the rest goes to swap, which is the internal disk. Keep the peak under 20 GB (rows under 3,500 tokens) or stop the VMs first.

**What the corpus lacks (and ideas):**

1. **Copy the example you were given** — the skill the eval's apps need most — has 56 conversations (the six recipes allowed), because the ten recipes that are eval solutions and the four CPU ones are withheld and the recipe search must return the right one for a phrasing. Options: more recipes of our own (the recipes-for-small-models plan: every recipe added to `REAL_RECIPES` in `programs.py` gives its conversations in three languages and three set-ups, checked by the real search); or, after the base is measured, a decision on training with the eval recipes and judging on `held-out/` and `school/` only (`validate_corpus.py` will then need the hold-out list changed on purpose).
2. Slips the compiler accepts cannot make error-fix pairs (`print(x)`, `==`, `+=`): the corpus covers 14 that it refuses (SET 473, array `v[i]` 86, braces 84, `for … in range` 84, a missing `END IF` 84, `THEN` left out 83, a trailing `;` 80, `int n;` 79, `ELSE IF` 64, `%` 61, a missing `NEXT` 60, `//` 57, `&&` 48, `!=` 41).
3. A `tiny` set-up and more weight on `lite`; paraphrased requests by a local Apache model (marked `paraphrased-by:`); the same corpus for the 4B (weights fetched, not trained: `setup.sh --with-4b`); an eval of base against trained on `held-out/` and `school/` with `--rep 3` before anything else.
4. The lawyer check before any trained model is shared (the list above is complete for what this lane downloaded; the corpus is RapidR's own material, and the base's Apache-2.0 `LICENSE` is in the Modelfile).
