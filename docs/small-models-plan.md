# Small local models that build real apps (plan, 2026-10-18)

Robert's goal (2026-10-18): help the "small" open models that run on a modest computer, like gemma-4 E4B (about 4B parameters, 4-bit, 5 GB) or MiMo 9B, to **actually build** working programs in RapidR Studio. Then say so publicly, with real numbers: *even a small open model on your laptop can help you build apps.*

## Where we are (2026-10-18)

- After the small-model fixes (window-sized replies, context budget, the nudge, honest statuses, edit misses that show the real lines), gemma E4B now makes real proposals: a project, a form, files.
- **But the programs don't work.** In Robert's scientific-calculator test, the model wrote a skeleton with placeholder logic and made-up names (`ButtonPressed`). It also wrote habits from other languages (`==`, `AS BOOLEAN AS BOOLEAN`), forgot `Form.ShowModal`, and did the pixel math badly (gaps, clipped buttons).
- A bug: once the model proposes a new project, it can't edit it until the user presses Create ("no project open"). Then it gets lost.

## The principle: Studio is the brain, the model is the hands

A 4B model is bad at *inventing* a program and good at *adapting* one. It's bad at pixel math and at remembering rules, but good at filling a small, clear gap. So we move the intelligence **out of the model and into Studio**:
1. Studio **retrieves**: the model never guesses.
2. Studio **builds the hard parts**: layout and structure come from tools, not from the model's arithmetic.
3. Studio **constrains** what the model must write: small blanks, not blank pages.
4. Studio **verifies and repairs**: compile, hints, then run with scripted clicks.
5. Studio **drives small steps**: each request fits an 8K window.

Big models keep today's free agent mode. Small-model mode switches on by itself when the window is small, or by choice in Settings.

## 1. Retrieval: a working example first

- **The recipe library** (`examples/recipes/`): about 20 short, idiomatic programs that compile and cover the common app shapes:
  - a calculator (one handler for many buttons, using `Sender`);
  - a form with a list and Start/Stop;
  - a settings dialog;
  - a data grid from CSV;
  - a timer/stopwatch;
  - a canvas drawing;
  - a simple game loop;
  - a menu, a file open/save, a chart;
  - …

  Each one has a header (what it shows, keywords). Every recipe is in the regression suite: it compiles, and a scripted click test passes.
- **Studio pushes the recipe; the model doesn't have to ask.** Small models don't call optional tools reliably. On "build me X", Studio itself puts the closest recipe into the first request: "Start from this working program and change it." There's also a `find_example(query)` tool for later steps.
- **The search, local and private:**
  - **BM25 keyword search** over recipes, examples and manual snippets. It needs no model and no download; we start here.
  - **Optional embeddings:** EmbeddingGemma (about 300M parameters) through the local server's embeddings endpoint (mlx-serve lists embedding support), or any OpenAI-compatible `/v1/embeddings`. Vectors for the recipes are computed once and cached.
  - **Legal:** Gemma weights are under Google's Gemma terms, not an OSI licence. We never ship them; the user downloads them in their own server.
- **Automatic facts on errors:** when check_code reports an unknown component, property or method, Studio attaches `rapidr_describe`'s entry to the error itself, so the model doesn't have to think of asking.

## 2. Studio builds the hard parts

- **`layout` tool:** "a 5×4 grid of buttons with these captions, below Display, filling the form" → correct CREATE blocks with exact coordinates that fit the client area. It uses `Align` (alTop, alClient…) where that's better.
  - Later: RStackPanel, a grid panel (on the ROADMAP), so layouts don't need coordinates at all.
- **`check_layout`:** overlaps, controls outside the client area, uneven gaps, buttons too small for their caption. It's reported like a compiler error.
- **Handler skeletons:** given a control and an event, it returns the exact `DECLARE SUB` and `SUB … END SUB` with the right parameters. The model fills only the body.

## 3. Constrain what the model writes

- **Spec first:** the model fills a small spec (controls, events, what each does) as JSON, with the local server's `json_schema` constrained decoding so it *can't* be malformed. Studio generates the form code deterministically from the spec.
- **One blank at a time:** then the model writes each handler body, one per request, seeing only:
  - the spec;
  - the names it may use (the form's controls, its variables);
  - the recipe's matching handler.

  Ten small, sure steps instead of one big guess.

## 4. Verify and repair (Studio's loop)

1. **check_code with friendly hints.** A table of other-language habits:
   - `==` and `!=` (BASIC uses `=` and `<>`); `&&` and `||`; `{}` and `;`;
   - `AS X AS X`; `+=`;
   - `print(` with brackets;
   - an undefined name, with the closest real one;
   - no `ShowModal`.

   Each comes with the exact fix. Up to 3 retries, then an honest status. The same table is used for people in Learn with RAI's friendly errors.
2. **Run it with scripted clicks.** Studio runs the program headless and drives it like a user: press 2, +, 3, = and expect `Display.Text = "5"`. A check the model writes from the request ("what should happen when…") is a later step; we start with the recipes' own tests and the eval suite.
3. **A diff guard:** a "fix" that deletes half the file is refused.

## 5. Measure, then brag

- **An eval suite** in the replay harness (`crates/rapidr-studio/examples/ai_eval.rs`, `tests/ai-eval/`):
  - 10 app requests: tip calculator, scientific calculator, stopwatch, to-do list, CSV viewer, quiz game, unit converter, drawing pad, password generator, the web-server manager's GUI;
  - models: gemma E4B, MiMo 9B, Haiku;
  - **score** = it compiles, and its click test passes.
- The results go in a table in this file, before and after each step.
- **The post waits for real numbers.** Something like: "a 4B open model, running locally on a laptop, built 8 of 10 working apps in RapidR Studio. Free, offline, private." Honest, measured, and RapidR-first. It pairs naturally with Learn with RAI: small local models are good teachers too.

## Order of work

1. Fix: a proposed new project stays editable before Create; the Preview follows.
2. Friendly hints for other-language habits in check_code, plus `rapidr_describe` attached to unknown-name errors.
3. The recipe library and Studio pushing the closest recipe (BM25).
4. The eval suite with click tests, and the baseline table.
5. The `layout` and `check_layout` tools, and handler skeletons.
6. Small-model mode: spec first (json_schema), one handler at a time.
7. Optional embeddings (EmbeddingGemma or any `/v1/embeddings`).
8. Measure again, then the post.

## Results

Measured on 2026-10-18 with `crates/rapidr-studio/examples/ai_eval.rs` and
the click tests of `tests/ai-eval/apps/` (one conversation per app — Robert's
own two turns for the scientific calculator; **score = the compiler takes it
and every click test passes**). *Before*: `development` at 441cf32b. *After*:
steps 1–5 of the order of work (the pending project, friendly hints, the
recipes pushed by BM25, the `layout` tool and layout check). Models:
`mlx-community/gemma-4-e4b-it-4bit` on mlx-serve on this Mac (window 16,384
before, 47,104 after — the server sizes it by the memory free when it loads
the model), and `claude-haiku-5-5` (Anthropic).

| App | gemma E4B before | gemma E4B after | Haiku before | Haiku after |
|---|---|---|---|---|
| Tip calculator | ✓ | ✓ | ✓ | ✓ |
| Scientific calculator (Robert's two turns) | ✗ compiles, no + key, empty handlers | ✓ 2 + 3 = 5 | ✓ | ✓ |
| Stopwatch | ✗ `AS BOOLEAN` | ✓ | ✓ | ✓ |
| To-do list | ✗ syntax, after "no project open" | — | ✓ | ✓ |
| Unit converter | ✗ `OnChange = ()` | — | ✗ shows "6 miles" | ✓ |
| Quiz game | ✗ `[ … ]` lists | — | ✓ | ✓ |
| Drawing pad | ✗ `BOOLEAN`, `MOUSEEVENT` | — | ✓ | ✓ |
| Password generator | ✗ compiles, Generate shows nothing | — | ✓ | ✓ |
| CSV viewer | ✗ compiles, the grid stays empty | — | ✓ | ✓ |
| Web-server manager | ✓ | — | ✓ | ✓ |
| **Score** | **2 / 10** | **3 / 3 measured** | **9 / 10** | **10 / 10** |

- **False claims** (an answer saying it changed something no tool changed):
  0 in every run.
- **"No project open"**: gemma before hit it in 7 of 10 conversations (it
  tried to fix its new program before Create); after, in none.
- **gemma after, apps 4–10: not measured.** Twice the local server stopped
  answering: during a long generation (the to-do list's first answer, over
  five minutes) mlx-serve's memory grew past 100 GB, and once the request was
  given up every prompt was refused — *"does not fit in GPU memory … only
  ~19 MB is available"* — until mlx-serve was restarted. The lane doesn't
  restart Robert's server: the rest is measured with
  `tests/ai-eval/apps/*.txt` (and the held-out `tests/ai-eval/held-out/`) once
  it answers again.
- **Read with care:** the ten eval apps are the shapes the ten recipes
  cover, so "after" measures *a request that has a recipe*. The held-out
  apps (a BMI calculator, a dice roller, a countdown timer — no recipe is
  them) measure the rest; their numbers come with the rest of gemma's.
- Every recipe passes its eval app's click test (`ai_eval --score`), so a
  failing app is the model's program, not the test.
