# AI on this computer

RAI, RapidR Studio's AI assistant, can work with a model that runs **on your own computer**: free, with no account and no key, and what you ask stays on the computer, as long as the app itself isn't set up to pass it on to a service elsewhere. The model runs in a *local AI app*: a free program you install that keeps AI models on your computer and answers RAI on it.

RapidR Studio doesn't download, run or recommend any model. It looks at the models **already on your computer**, tells you which ones fit it, can try them on a few small tasks, and says which one is the best match for this computer. Only the models you have appear: RapidR never names or suggests a model you don't have.

## Set it up

1. **Tools > Set Up RAI...** (or Ctrl+I before RAI is set up).
2. Choose **Free, on this computer**. Studio looks for AI apps on this computer at once and says what it found.

   ![Set Up RAI's first page: Free, on this computer, with "Found 3 models in a local AI app on this computer"; or a cloud service with your own key](images/local-ai/setup-found.png)

3. **Next**. Each model of your local AI app is listed, the best match for this computer first and already chosen:

   ![The models on this computer: example-small fits and is the best match by its size, example-chat's app says it takes no tools, example-huge is too big for this computer, example-embed can't chat; the chosen model's size, parameters, the context RAI gets and its licence as its local AI app reports it](images/local-ai/setup-models.png)

4. Optional: **Check My Models** tries each model that fits now, and each whose size isn't known, one at a time, on this computer (below). You can choose without it.
5. Choose a model and **Finish**. RAI is on, with that model, and its pane opens.

Everything is done with the keyboard too: Tab moves between the list, the buttons and the address; the arrow keys choose a model; Enter is Finish. A screen reader reads each row as one sentence ("example-small. Fits. Best match for this computer by its size: Check My Models to see how it does.").

## What Studio looks for, and where

Only when you open this page (or press **Look Again** or **Look**), Studio asks the local AI apps **on this computer** for their models:

- at `127.0.0.1` on the ports local AI apps usually use (11434, 1234, 8080, 8000, 1337, 5001, 4891 and 8081),
- and at an address you type under **Another address** — for an app that uses another port.

It never asks another computer: an address that isn't this computer's (`127.0.0.1`, `[::1]` or `localhost`) is refused before anything is sent. A port where nothing answers is skipped at once. Studio asks each app for its list of models (the OpenAI-compatible API many local AI apps offer: `GET /v1/models`) and, when the app describes its models, for each one's size, parameters, quantization, context, whether it takes tools, its shape (layers, attention heads) and its licence. It starts nothing and loads nothing to look.

RapidR Studio sends nothing about your computer or your models anywhere: it only talks to this computer. What a local AI app does with what you ask is up to the app: some can pass questions on to a service elsewhere, and Studio marks a model whose app says it runs elsewhere (*Runs elsewhere*): it is never the best match, never checked and can't be chosen here.

On a computer shared with other people (a school lab, a remote desktop), the app that answers may belong to someone else, and Check My Models would use its memory and processor. Use Check My Models only on an app that is yours.

## Does it fit? (the chips)

Each model has a chip that says whether it fits **this** computer. Studio works it out at once, without running the model:

> memory the model needs = its weights (the model file) + the conversation's memory for the context RAI gets (from its layers, attention heads and head size) + what the AI app needs to run it

and compares it with the memory free now (keeping about 1.5 GB for RapidR Studio and the system) and the most a model could have with other apps closed (on a Mac with Apple silicon, the part of the shared memory the graphics may use; elsewhere, all but what the system keeps).

| Chip | What it means |
|---|---|
| **Fits** | By Studio's estimate, it fits in the memory free now, comfortably. |
| **Fits if you close other apps** | It would fit with other apps closed: close some (a browser with many tabs, a game), then Look Again. |
| **Too big for this computer** | Not even then. It is listed, but it can't be chosen. |
| **Size not known** | Its app doesn't say its size (or a browser can't say the memory): Check My Models shows how it does. |
| **Runs elsewhere** | Its app says it runs the model on a service elsewhere: what you ask would be sent there. It can't be chosen here. |

Studio also chooses the **context** RAI gets (how much of the conversation the model keeps in mind): 16K tokens when there is room, 8K by default, never below 4K, and never more than the model was made for. The line under the list says it ("RAI gets a 16K context").

## Check My Models

**Check My Models** loads each model that fits now, and each whose size isn't known, one at a time, on this computer, and tries it. The note beside it says about how long it takes (a minute or two a model; more for a large one). It uses the computer's memory and processor meanwhile, and loading a model may make the app close another one it had loaded. A model that says *Fits if you close other apps* is not checked until it fits: close other apps, Look Again, then Check My Models. For each model Studio:

1. **times an answer**: how long RAI's real first answer — its instructions and tools — takes to start, and how many words a second follow;
2. **tries a tool call**: whether the model calls one of RAI's tools the way RAI needs (some models write the call as text instead: RAI can't use those to build apps);
3. **builds a tiny app** through RAI's real loop and Studio's real tools: *a window with a button that says Hello*. It passes when the program it writes compiles and has that button. Nothing the model writes is run or saved.

When its app lets Studio ask, a model the check loaded is let go after it (one that was already in use stays); some apps keep it until their own timer. **Stop Checking** stops as soon as the model's current answer ends (the model being checked stays unchecked). The results are kept on this computer, for each model and this version of Studio: a model is checked again only when it, RAI's instructions or Studio change. They are in `local-ai-checks.json` in Studio's own folder (where it keeps its settings and the audit log: [the assistant's page](ai-assistant.md) says where): timings and yes/no results, and the model's digest — or its name and address when it has none. Nothing in it is sent anywhere; delete the file to forget them.

![After Check My Models: example-small builds apps and is the best match for this computer; example-chat can chat, but its tool calls didn't work](images/local-ai/setup-checked.png)

### The best match, in words

This says how a model did on Studio's one small test on this computer today. It is not a verdict on the model: another computer or another task can differ. The models are ordered by what matters to RAI on this computer: it **built the test app** › its **tool calls work** › it is **fast enough** (an answer starts within about 10 seconds) › it leaves the most **memory to spare**. Each row says it in one line:

| The line | What it means |
|---|---|
| *Best match for this computer: it built Studio's test app, answers in about 4 s.* | The best of your models for this computer, as Studio's check sees it. You choose. |
| *It built Studio's test app, answers in about 6 s.* | Did well too. |
| *Works, but slowly here (about 40 s per answer).* | It built the test app, but you'll wait. |
| *Uses RAI's tools, but couldn't build the test app here.* | RAI can still help: it starts its programs from RapidR's recipes. |
| *Can chat, but can't build apps with RAI: its tool calls didn't work here.* | It answers questions, but can't use RAI's tools. |
| *Its local AI app says it takes no tools, so RAI can't build apps with it.* | Not checked: its app says so. |
| *Best match for this computer by its size: Check My Models to see how it does.* | Not checked yet: the best guess from its size and what its app says. |

## RAI with a small model

A small model gets RAI's **small-model mode**: shorter instructions, fewer tools, a working recipe sent first with a request for a new program, and the context chosen above. And if a request for a new program keeps going wrong — three tries that are refused or don't compile — Studio starts the program from that recipe, unchanged, and says so in the chat (*Started from the recipe …, unchanged: the model's own 3 tries didn't work*). You get a working program to change step by step instead of nothing.

The mode is chosen from the model's check, never from its name: a model that built the test app, quickly, with working tool calls and a 16K context gets RAI's full instructions; any other the small-model mode. A model not checked gets the mode its context gives (below 16K: small). Programs can choose it too: [RAI's `SmallModel` and `ContextSize`](reference/members.md).

## Licences

Every model comes with its own licence, from whoever made it — some are very open, others limit commercial use, what the model may be used for, or what you may do with what it writes, and some ask you to credit it. Studio shows each model's licence **as its local AI app reports it** (its name, and **Read the Licence...** for the whole text). Studio doesn't check it: it comes from the model's file, and can be wrong or out of date. With it, a reminder:

> **Building apps to sell? Check this model's licence:** it may limit commercial use, or what you may do with what the model writes.

When the app doesn't report one, Studio says so: check the licence where you got the model. RapidR doesn't download or distribute any model, doesn't check what a licence says, and its own licence (MIT) says nothing about a model's or about what a model writes. Whether you may sell programs a model helped write, and who owns them, depends on that model's licence and on your country's law; ask the model's makers if you are unsure.

![A model's licence, as its local AI app reports it, with the reminder for apps to sell](images/local-ai/setup-licence.png)

## No local AI app yet

When nothing answers, the page says what a local AI app is and how to add one:

![No AI model answered on this computer: what a local AI app is, how to add one, and another address](images/local-ai/setup-none.png)

1. Install a local AI app: a free program that keeps AI models on your computer and offers them through the **OpenAI-compatible API** (many do; check its licence, and that it runs on your system).
2. In it, add a model that fits your computer, and read its licence. As a rule of thumb, a model file of 2–5 GB suits a computer with 16 GB of memory; this page's chips estimate it once it is there. Models that take tools work best with RAI.
3. Start the app (Studio never starts it for you), then **Back** and **Look Again** — or type its address under **Another address** if it uses a port of its own (`127.0.0.1:PORT`).

## In the web Studio

A browser can't look around your computer by itself. On the page **RAI on this computer**, type your local AI app's address (`http://127.0.0.1:11434`, for example) and press **Look**: an address that isn't this computer's is refused before anything is sent. The app must allow the web Studio's page to ask it: in its settings, allow requests from the address the page is served from (its *origin*; a local AI app usually calls this its allowed origins or CORS setting). Allow only that address, never every site: an app that answers any page lets any website you visit use your models. The browser can't tell how much memory is free, so the chips say *Size not known*; checking models runs in Studio on the desktop.

## School Lock

While [School Lock](for-teachers.md) turns RAI off, Set Up RAI stays on its first page, read-only: nothing is looked for and no model is checked.

## RSystemInfo: what this computer has

The fit check reads the same facts any program can read with **RSystemInfo**: the system and its version, the processor and its cores, all the memory and the memory a program can have now, the graphics and their memory (shared on Apple silicon), the disk and what is free, the battery — read on this computer; RapidR never sends them anywhere. `Class` says the computer's size in plain words (`small`, `medium`, `large`), so a program can adapt; `Summary` says it all in one sentence. Sizes are in megabytes.

```basic
DIM Info AS RSystemInfo
PRINT Info.Summary
IF Info.Class = "small" THEN PRINT "Using the light settings."
IF Info.FreeMemory < 2048 THEN PRINT "Less than 2 GB free: close some apps first."
```

![The system_info example: macOS on an Apple M4 Max, its memory, graphics, disk and battery, and the size of the computer](images/local-ai/system-info.png)

**If your program sends these facts anywhere** (your server, a log, a crash report), they can be personal data: together they pick out one computer, and in the EU reading them to track people needs consent. Tell your users, ask first where the law says so, send only what you need (`Class` is usually enough), and never send them from a program for children. On the web the browser gives less on purpose (no graphics name, no battery, memory rounded to 8 GB at most): don't try to get round that.

The example [system_info.rr](../../examples/system_info.rr) shows every fact. On a web page the browser tells only a little (its cores, roughly its memory, the system): `Unknown` names what can't be known there, and those properties read 0 or "". Every member is in [the reference](reference/members.md).

## When something is off

| What you see | What to do |
|---|---|
| *No local AI app found running on this computer* | Start your local AI app, then Look Again; if it uses another port, type its address. |
| *A local AI app answers on this computer, but it has no model yet* | Add a model in the app, then Look Again. |
| Every model is *Too big for this computer* | Close other apps and Look Again, or add a smaller model (a smaller file, or a stronger quantization such as Q4). |
| *Its check couldn't finish* | The app stopped answering (it may have run out of memory): close other apps, then Check My Models again. |
| RAI forgets the start of a long chat | The model runs with a small context: RAI keeps within it. A model that fits with room to spare gets 16K. |
