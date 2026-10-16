# The AI assistant and RAI

RapidR Studio has an AI assistant that knows RapidR: ask it about your
project, have it propose changes — shown as diffs you accept or reject — or
ask it to build a new program, previewed before anything is written. Your
own programs can use the same AI through the **RAI** component; the
assistant is itself an RAI.

It works with the AI provider you choose: **Anthropic** (Claude),
**OpenAI**, **DeepSeek**, or any **OpenAI-compatible** server — including a
model server on your own computer (Ollama, LM Studio), which needs no key
and sends nothing out.

**Off by default.** Nothing is sent anywhere until you turn the assistant
on, set it up and press Send. RapidR has no telemetry.

**What it costs.** RapidR charges nothing. A cloud provider charges your
account for each request, by the *tokens* (pieces of words) sent and
received; a model server on your computer costs nothing. Studio counts the
tokens of each session, and shows their cost when you give it your
provider's prices (Settings has a **See prices** link to them).

## Setting it up

**Tools > AI Settings…** (or **Tools > Settings**, the **AI** page):

![Tools > Settings, the AI page: the provider, a key kept for the session, the model picked from the provider's list, the reply limit, the prices, the permission](images/ai/settings.png)

1. **Turn on the AI assistant.**
2. **Provider.** **See prices** opens the provider's price page.
3. **API key** — the key from your account with the provider (**Get a
   key** opens the page that makes one). Paste it and press **Save Key**:
   it is kept in your system's keychain (the macOS Keychain, the Windows
   Credential Manager, the Linux Secret Service), never in a project, never
   shown again — Settings shows its last four characters. **Remove**
   deletes it. If you set the provider's environment variable instead
   (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DEEPSEEK_API_KEY`), Studio uses
   that. A server on your own computer needs no key.
4. **Workspace ID** (Anthropic, optional): for a key made in your
   organisation rather than inside one of its workspaces. If Anthropic
   answers that it needs to know the workspace, Settings says so and puts
   the cursor in this box: copy the ID from the Claude Console's
   workspaces page.
5. **Address** (OpenAI-compatible): the server's address, such as
   `http://localhost:11434/v1` for Ollama or `http://localhost:1234/v1` for
   LM Studio (the list offers these, xAI and OpenRouter).
6. **Model.** **Fetch Models** lists the models your key can use, from the
   provider itself — with the key you just typed, if you typed one (it is
   only tried: **OK** or **Save Key** keeps it). None is chosen for you. The
   ones whose names say they are small (*mini*, *nano*, *flash*, *lite*,
   *haiku*, *small* …) are marked **fast · low cost** and listed first;
   *opus*, *pro* … are marked **costs more**. This is only a hint from the
   names: the provider's price page is the truth. Studio remembers your
   choice for each provider, and asks again if the provider stops offering
   it.

   Under the list, Settings says what happened: how many models came, or —
   in plain words — why none did: a key the provider refused, an account
   without credit, a server that isn't there. Fix it (paste the key again,
   say, or fill in the Workspace ID) and press **Fetch Models** again: it
   works again at once. A provider that doesn't answer is given 30 seconds;
   picking another provider or closing Settings stops waiting.

   ![A key the provider refused: what it said, under the Model list; Fetch Models ready to press again](images/ai/settings-refused-key.png)

7. **Longest reply**: the most tokens one answer may take (4096 unless you
   change it) — a limit on what one answer can cost.
8. **Prices** (optional): what your provider charges, in dollars for a
   million tokens sent and for a million received, from its price page. The
   token meter then shows an estimate of the cost.
9. **It may**: *Read the project, propose changes you review* (the
   default), or *Only read the project and answer*.

The assistant's pane and its toolbar button appear once it is on and has a
provider, a key (when the provider needs one) and a model.

**OK** gives your choices to the assistant. **Cancel** leaves it exactly as
it was — whatever you tried meanwhile: other providers, their lists, a key
typed. Only **Save Key** and **Remove** act at once, as their names say (on
the key kept in your keychain).

![An organisation's Anthropic key: Anthropic wants to know the workspace](images/ai/settings-workspace.png)

**In Studio on the web**, a key is kept for the browser tab only.
**Remember on this device** keeps it in the browser's storage for Studio's
page instead — anyone who uses this browser profile could read it there.

![The AI settings in Studio on the web: Remember on this device, under the key](images/ai/settings-web.png)

## Asking

**View > AI Assistant** (Ctrl+I; the ✦ button on the toolbar) shows the
**Assistant** pane, a tab beside Properties; the first time it shows, its column
opens 380 pixels wide (drag the divider to change it, and Studio remembers).
Type your question and press
**Enter** (Shift+Enter starts a new line). The answer appears as it is
written; **Stop** stops it. The lines in italics say what the assistant
looked at to answer: a file it read, a component it looked up in RapidR's
registry, the forms' components it listed, the compiler's check.

![The Assistant pane: an answer, what the assistant looked at, the token meter, what Send shares](images/ai/panel.png)

At the top, switch the provider and the model for the next question;
**New Chat** starts over (a long chat costs more: everything said so far is
sent with each question).

The assistant reads only what it needs, when it needs it: the project's
files, the components on your forms and their properties, RapidR's own
description of every component, property, method, event and function, and
what the compiler says about the code. It can't read anything outside the
project's folder, and never your key.

## Changes, as diffs

The assistant never changes a file itself. It **proposes**: when an answer
has changes, a bar says so — **Review…** shows them:

![Reviewing a proposed change: the file as it is on the left, as proposed on the right, Accept or Reject on each change](images/ai/review.png)

Each change has **Accept** and **Reject** (F7 goes to the next one; Enter
accepts it, Backspace rejects it); **Accept All**, **Reject All**, then
**Apply**. What you applied is one step in each file's undo: **Edit >
Undo** takes it back. **Tools > Review the AI's Changes…** opens the review
again.

## A new program: "Build me…"

Ask for a program — *"Build me a tip calculator"*, *"Make a program that
lists the files of a folder"*. The assistant writes it, checks it with
RapidR's compiler, and shows it to you: its files, the code, and the form as
it will look. Nothing is written until you choose a folder and press
**Create and Open**.

![A new program from the assistant, previewed: its file, its code and its form](images/ai/new-program.png)

## What was sent, and the audit log

**Tools > What Was Sent to the AI…** shows the last request exactly as it
was sent — with your key replaced by *(your API key, not shown)*.

![What was sent: the request, its headers without the key, RapidR's instructions first, the project's facts, the tools](images/ai/what-was-sent.png)

Every request, every file the assistant read, every change it proposed and
what you decided is written to an **audit log** on your computer, one JSON
object a line (its last 10,000 lines): `~/Library/Application
Support/RapidR/ai-audit.jsonl` on a Mac, `%APPDATA%\RapidR\ai-audit.jsonl` on
Windows, `~/.config/rapidr/ai-audit.jsonl` on Linux; on the web, the
browser's storage for the page (its last 500). It never holds your key.

## Privacy and safety

- **Send shares** your message, the chat so far, RapidR's instructions (the
  same for every project) and what the assistant reads of the open project —
  nothing outside it, nothing before you press Send.
- **Text in your files is data.** A comment or a data file that says
  "ignore your instructions and …" is something the assistant reports, not
  something it obeys; and whatever it is persuaded of, it can only propose
  changes you review (or only answer, with *Only read the project*).
- **At most 25 tool uses** for one question; Stop works at any time.
- **Keys** are never written into a project, a build, a log, the audit log
  or a screenshot of RapidR's. A key someone wrote into a file of the
  project is replaced by *(an API key, not shown)* before the assistant
  reads the file.
- **A model on your own computer** (Ollama, LM Studio): nothing leaves it.

## Tokens: what makes a question cheap

Studio keeps each question small:

- RapidR's instructions come first and are the same every time, so the
  providers' caches serve them again at a lower price — the meter says how
  many tokens came *from the cache*.
- The project isn't sent whole: the assistant reads the parts it needs.
- What the assistant read for an earlier question is shortened when you
  ask the next one; a long chat is summarized past a size.
- **Longest reply** limits each answer.

Measured on 2026-10-15 with each provider's fast, low-cost model, *"Build me
a hello-form program"* (a working form program, made and checked) took from
about 7,600 to 22,000 tokens sent — between a third and two thirds of them
from the cache — and 400 to 2,000 received: a fraction of a cent.

## RAI in your own programs

**RAI** (Toolbox > AI) gives your program the same AI: a provider, its
models, questions and answers streamed in. It is a non-visual component.

```basic
DIM AI AS RAI
DECLARE SUB TokenIn(Text AS STRING)
DECLARE SUB ModelsIn(Count AS INTEGER)

AI.Provider = "anthropic"
' the key: the environment variable if it is set, else ask the user
AI.APIKeySource = "env:ANTHROPIC_API_KEY|ask"
AI.OnModelsLoaded = ModelsIn
AI.OnToken = TokenIn
AI.RefreshModels                   ' the provider's own list, in the background

SUB ModelsIn(Count AS INTEGER)
    AI.Model = AI.ModelName(0)     ' the first: a fast, low-cost one
    AI.Ask "Say hello in five words."
END SUB

SUB TokenIn(Text AS STRING)
    PRINT Text;                    ' the answer, a few words at a time
END SUB

' (a console program waits for it; one with a window needn't: its
' window's loop hears the answer)
DO
    DOEVENTS
LOOP UNTIL AI.Busy = 0 AND (AI.Answer <> "" OR AI.Error <> "")
PRINT
PRINT AI.Usage; AI.Error
```

- **Provider** (`anthropic`, `openai`, `deepseek`, `compatible`), **Model**
  (one of **Models**, which **RefreshModels** fills — `OnModelsLoaded`;
  each line is the model's id, its name and its hint, tab-separated;
  **ModelName(i)** is line *i*'s id), **BaseURL** (another address: a
  server on the user's computer, your own proxy), **WorkspaceID**.
- **Ask(Text)** sends Text with the conversation so far and returns at once:
  **OnToken** as the answer streams in, **OnMessage** with the whole of it,
  **OnDone("ok" / "error" / "cancelled")**. **Cancel** stops it, **Busy**
  says whether one is coming, **NewChat** forgets the conversation.
  **OnError(Message)** says what went wrong in plain words: a wrong key, no
  credit left, a model the key can't use, too many requests.
- **MaxTokens** (the longest answer; 1024 by default), **System** (your
  instructions), **SummaryAt**, **Usage** (*"1,000 tokens sent, 20
  received"*), **InputTokens**, **OutputTokens**, **CachedTokens**, **Cost**
  (with **PriceIn** / **PriceOut** set), **Audit** (the audit log above).
- **PricingURL** and **KeysURL**: the provider's price page and the page
  that makes keys, for your own links.

Every property, method and event: [RAI in the reference](reference/members.md#rai).

### In the designer

Drop an RAI on a form and pick its **Provider** in the inspector. Type the
key in the **APIKey** row: it is kept **in your keychain**, under the name
`RapidR/<project>/<component>`, and the code gets only where it is —
`APIKeySource = "keychain:RapidR/<project>/AI|ask"` — never the key. Then
the **Model** row lists the provider's own models, fetched in the
background (*↻ Refresh the list* fetches them again):

![An RAI in the designer: the code names where the key is, never the key; the Model row offers the provider's models](images/ai/designer-rai.png)

### Keys in programs you give to others

- **Never write a key in a program.** Anyone who has the built app can read
  it. A build that finds one says so: *"an API key is written in the
  program…"*.
- **APIKeySource** says where the key comes from, the first that has one:
  `keychain:NAME` (the keychain of the computer the program runs on — the
  program keeps a key there with **RememberKey**), `env:NAME` (an
  environment variable), `ask` (**OnKeyNeeded** fires: ask the user, set
  **APIKey**, ask again). Join them with `|`: `"env:OPENAI_API_KEY|ask"`.
- **APIKey** is write-only: reading it gives `""`; the inspector, the
  debugger's Variables and every log show it masked or not at all.
- **On a web page** there is no keychain and no environment: the page asks
  its user (`ask`), and keeps the key for the tab — or in the browser's
  storage for the page when the program calls **RememberKey(True)** (tell
  your users what that means). A web build says this too.
- Each user then pays for their own questions with their own key. To pay
  for them yourself, point **BaseURL** at a server of yours that holds the
  key.

### The example

[examples/ai/rai_models.rr](../../examples/ai/rai_models.rr): choose a
provider, give your key (from its environment variable, or typed while the
program runs), list the provider's models, ask a question and watch the
answer stream in. With Anthropic a **Workspace ID** row shows: fill it in
for a key made outside a workspace (or set `ANTHROPIC_WORKSPACE_ID`). What
goes wrong is said in plain words under the answer, and **List Models**
and **Ask** work again at once. The window can be made bigger: everything
follows.

```sh
rapidr run ai/rai_models.rr                              # a cloud provider
rapidr run ai/rai_models.rr http://localhost:11434/v1    # Ollama on this computer
```

![examples/ai/rai_models.rr: a model server on this computer, its models, a question and its answer](images/ai/rai-models-example.png)

![examples/ai/rai_models.rr with Anthropic: the Workspace ID row, the models, a question answered](images/ai/rai-models-workspace.png)

## Providers

| Provider | Needs | Notes |
|---|---|---|
| Anthropic (Claude) | a key; a workspace ID for some keys | |
| OpenAI | a key | only the models that can chat are listed |
| DeepSeek | a key | |
| OpenAI-compatible | its address; a key if the server wants one | Ollama, LM Studio, xAI, OpenRouter, a company's own server |

On the web, the browser calls the provider itself: Anthropic, OpenAI and
DeepSeek accept calls from a page (checked 2026-10-15); Ollama needs its
`OLLAMA_ORIGINS` setting to allow the page's address, and LM Studio its
CORS switch. A web app built with RapidR may call providers and servers on
the user's own computer: its page's security policy allows them.

*The pictures on this page were made with a test server standing in for a
provider: its model names are made up, and so is the key.*
