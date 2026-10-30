# Local AI made easy (Preview 5)

Robert, 2026-10-29: *"I am not sure we should recommend models. Just test what they've downloaded and recommend what's the best match for the resources they have."*

So RapidR never names, ranks or suggests a model the user doesn't have. It looks at the AI models already on the computer, checks which ones fit, tests them on RapidR's own small tasks, and says which one is the best match **for this computer**. Our knowledge lives in the tests, not in a list of model names: lists go stale, tests don't, and recommending third-party models (each with its own licence) is a risk we don't take.

## 1. `RSystemInfo` (a component for every program)
What this computer has, read locally, never sent anywhere: total and free memory, CPU (cores, architecture), the GPU and its memory (unified memory on Apple silicon), free disk space, operating system, on battery or plugged in. A `Class` property in plain words (`small`, `medium`, `large`) for programs that adapt. Desktop and web (the web gives less; it says what it can't know).

## 2. Find the models already there
Only when the user opens **Set Up RAI › On this computer** (it says *"Looking for AI apps on this computer…"*): ask the local AI servers on `127.0.0.1` that speak the OpenAI-compatible API, at their usual ports, plus any address the user types. Never another computer. For each model, read what the server reports: file size, parameter count, quantization, context length, whether it supports tools, its architecture (layers, attention heads) and its licence text.

## 3. Fit check (instant, nothing runs)
Memory a model needs = its weights + the conversation's cache for the chosen context (from layers, key/value heads and head size) + the runtime's overhead. Compared with the memory free, keeping room for Studio and the system:

- **Fits** (green chip): comfortable.
- **Fits if you close other apps** (amber).
- **Too big for this computer** (grey): shown, never picked.

Studio picks the context size by itself (8K by default, 16K when there's room, never below 4K) and RAI uses it. A model without tool support: *"Can chat, but can't build apps with RAI."*

## 4. Check my models (a test the user starts)
A button, optional, with an estimate (*"about 2 minutes per model"*), Cancel at any time, one model at a time, each unloaded after. Three probes from the eval harness (tools/ai_eval), built in:

1. **Speed:** how long RAI's real first answer takes to start, and words per second after.
2. **Tools:** does a tool call arrive properly (the format problems seen in the eval).
3. **A tiny RapidR task** through RAI's real loop: *"a window with a button that says Hello"*: does it compile and does the click test pass.

Results are kept per model (by its digest) and per Studio version; a model is tested again only when it or RAI's prompts change.

## 5. The best match, in words
Rank: passes the task › tools work reliably › fast enough (an answer starts within ~10 s on this computer) › memory to spare. One line each, for a student:

- *"Best match for this computer: builds apps, answers in about 4 s."*
- *"Works, but slowly here (about 40 s per answer)."*
- *"Can chat, but can't build apps."*
- *"Too big for this computer."*

The user picks; Studio remembers. Only models found on this computer appear. When none is found: what a local AI server is, and a link to the manual page that explains how to add one (no product pushed).

## 6. RAI adapts to the model
A small model gets RAI's **small-model mode** (from the eval report): a shorter prompt, recipe first, fewer tools, the chosen context, the fallback to a recipe after repeated errors. Chosen from the probe results, not from the model's name.

## 7. Safety, privacy, legal
- Everything stays on the computer: system facts, probes and results are never sent.
- Only `127.0.0.1`; probing happens only when the user opens the setup or presses Check.
- Each model's licence is shown as its server reports it, with a note: *"Building apps to sell? Check this model's licence."*
- RapidR downloads and distributes no model. A built-in runner and a model download manager (with licence acceptance) are Preview 6, after the lawyer's review; the same tests then rank our own trained model too.
- Lawyer review of this feature before merge (localhost probing, showing licences, the wording).

## 8. Fast, easy, accessible, beautiful
The fit check is instant; the test is optional and cancellable; rows with status chips from the design guide, every result said in words for screen readers; works with the keyboard; in the four languages.

## 9. Web Studio
A browser can't look for local servers by itself: the user types the address (the server must allow the page's origin; the manual says how). The fit check uses what the browser can tell. A runner in the browser (WebGPU) is later.
