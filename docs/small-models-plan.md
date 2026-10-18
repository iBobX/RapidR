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
  - **Legal (checked 2026-10-18 on Hugging Face):**
    - **Gemma 4** (E4B and the rest) is **Apache-2.0** per Google's model card. The mlx-community 4-bit copy still carries an old "gemma" tag, so we link to Google's card.
    - **EmbeddingGemma** is still under the older, gated Gemma Terms of Use, so it can only be the user's own choice, never required or recommended by default.
    - **MiMo-V2.6-Distill-Qwen-9B** is MIT (Xiaomi).
    - We never ship weights: Studio talks to whatever OpenAI-compatible server the user runs.
    - "Gemma" and "Qwen" are trademarks: name them plainly, with no logos and no implied endorsement.
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
`mlx-community/gemma-4-e4b-it-4bit` on mlx-serve on this Mac, and
`claude-haiku-5-5` (Anthropic). The local window is what the server gave
when it loaded the model (it sizes it by the memory free): 16,384 before;
after, 47,104 for the first three apps and 33,792 for the rest.

| App | gemma E4B before | gemma E4B after | Haiku before | Haiku after |
|---|---|---|---|---|
| Tip calculator | ✓ | ✓ | ✓ | ✓ |
| Scientific calculator (Robert's two turns) | ✗ compiles, no + key, empty handlers | ✓ 2 + 3 = 5 | ✓ | ✓ |
| Stopwatch | ✗ `AS BOOLEAN` | ✓ | ✓ | ✓ |
| To-do list | ✗ syntax, after "no project open" | ✓ | ✓ | ✓ |
| Unit converter | ✗ `OnChange = ()` | ✓ | ✗ shows "6 miles" | ✓ |
| Quiz game | ✗ `[ … ]` lists | ✓ | ✓ | ✓ |
| Drawing pad | ✗ `BOOLEAN`, `MOUSEEVENT` | ✓ | ✓ | ✓ |
| Password generator | ✗ compiles, Generate shows nothing | ✓ | ✓ | ✓ |
| CSV viewer | ✗ compiles, the grid stays empty | ✓ | ✓ | ✓ |
| Web-server manager | ✓ | ✓ | ✓ | ✓ |
| **Score** | **2 / 10** | **10 / 10** | **9 / 10** | **10 / 10** |
| *Held out: BMI calculator, dice roller, countdown timer* | *not measured* | *3 / 3* | — | — |

- **Read with care.** The ten eval apps are the shapes the ten recipes
  cover, and gemma's ten passing programs *are* the pushed recipe, as it is
  (the same lines, but for a caption or the last line break): Studio
  delivered them, the model passed them on — which is the plan's principle,
  but it is not the model writing an app. The **held-out** apps
  (`tests/ai-eval/held-out/`: no recipe is them) show the adapting: gemma
  made a working BMI calculator (70 kg, 175 cm → "BMI: 22.9", from the tip
  calculator's shape), a dice roller (6 + 1 → "Total: 7") and a countdown
  timer (9 s → "0:06.6" after the clicks), 3 of 3. Three apps are a small
  sample: measure more held-out apps before a post says "builds apps".
- **False claims** (an answer saying it changed something no tool changed):
  0 in every run (73 conversations).
- **"No project open"**: gemma before hit it in 7 of 10 conversations (it
  tried to fix its new program before Create); after, in none.
- **A second "before" run** at the 33,792 window (the after one) agreed:
  tip calculator and unit converter passed, the scientific calculator,
  stopwatch, to-do list and quiz failed (2 of the 6 measured); the rest and
  the held-out apps weren't measured, because of the server (next point).
- **The local server stops answering after a very long generation.** Three
  times an answer took over five minutes (the eval gives up then); each
  time mlx-serve's memory grew past 100 GB and afterwards it refused every
  prompt — *"does not fit in GPU memory … only ~19 MB is available"* — until
  it was restarted. A client can't prevent it; Studio says the server's own
  words. Worth reporting to mlx-serve, and worth knowing before a live demo.
- Every recipe passes its eval app's click test (`ai_eval --score`), so a
  failing app is the model's program, not the test.
