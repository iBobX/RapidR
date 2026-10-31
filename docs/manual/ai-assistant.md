# The AI assistant and RAI

RapidR Studio has an AI assistant that knows RapidR: ask it about your
project, have it propose changes — shown as diffs you accept or reject — or
ask it to build a new program, previewed before anything is written. Your
own programs can use the same AI through the **RAI** component; the
assistant is itself an RAI.

In a classroom, a teacher can keep the assistant off behind a password:
[For teachers: School Lock](for-teachers.md).

It works with the AI provider you choose: **Anthropic** (Claude),
**OpenAI**, **DeepSeek**, or any **OpenAI-compatible** server — including a
local AI app on your own computer, which needs no key: Studio sends what you
ask only to that app ([AI on this computer](local-ai.md)).

**Off by default.** Nothing is sent anywhere until you turn the assistant
on, set it up and press Send. RapidR has no telemetry. While your program runs, RAI can read what its windows show and click and type in them; that goes to your provider like the rest of the chat.

**What it costs.** RapidR charges nothing. A cloud provider charges your
account for each request, by the *tokens* (pieces of words) sent and
received; a model server on your computer costs nothing. Studio counts the
tokens of each session, and shows their cost when you give it your
provider's prices (Settings has a **See prices** link to them).

## Setting it up

### Set up RAI, step by step

**Tools > Set Up RAI…** (also **Set Up RAI…** on the AI page of Settings,
and **Ctrl+I** before the assistant is set up) asks one thing at a time:

1. **Where should RAI's answers come from?**
   - **Free, on this computer.** A model runs on your own computer, in a
     local AI app (a free program you install that keeps AI models on your
     computer). It costs nothing, needs a fast computer with plenty of
     memory, and answers are slower and simpler than a cloud model's. Studio
     asks this computer which models it already has and says what it found
     — *✓ Found 3 models in a local AI app on this computer* — or that none
     is running. It never starts one for you. [AI on this computer](local-ai.md)
     has the rest: which models fit, Check My Models, the best match.
   - **A cloud service, with your own key**: Anthropic (Claude), OpenAI or
     DeepSeek — usually the best answers. You pay the provider for what you
     use; the page says what a question costs and what is sent.

   ![Set up RAI: free on this computer (3 models found in a local AI app), or a cloud service with your key](images/ai/setup.png)

2. **On this computer**: the server (the ones running first) and one of its
   models. **In the cloud**: the provider, your key (**Get a key** opens the
   provider's page that makes one; a key Studio already finds — in the
   keychain, or the provider's environment variable — can be used as it is),
   **Test the Key**, then a model — the fast, low-cost ones are listed
   first. A key the provider refuses is said in plain words, right there;
   type another and test again.

   ![Set up RAI with a cloud key: the key tested, the models listed, what it costs and what is sent](images/ai/setup-cloud.png)

**Finish** turns the assistant on, keeps a key you typed in your system's
keychain (never in a project), and shows the Assistant pane with the
provider's models already listed — and what it says of them, such as each
model's context window, without pressing ↻. **Cancel**
changes nothing. Everything can be changed later in Settings.

### Every setting: Tools > AI Settings

**Tools > AI Settings…** (or **Tools > Settings**, the **AI** page):

![Tools > Settings, the AI page: the provider, a key kept for the session, the model picked from the provider's list, the reply limit, the prices, the permission](images/ai/settings.png)

On a short screen (a 1280 × 720 laptop, a small browser window) Settings is
never taller than the screen: its page scrolls under the tabs, and OK and
Cancel stay at the bottom.

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

7. **Longest answer**: the most tokens one *part* of an answer may take —
   32,000 unless you change it, and never more than the model allows (a model
   that allows less says so, and Studio asks again within its limit). An
   answer that reaches it doesn't stop: it goes on by itself, in up to 5
   parts (see [A long answer goes on by itself](#a-long-answer-goes-on-by-itself)).
   Under the box Settings says, in words, when the *chat* is summarised too:
   your request kept word for word, when the chat holds 70% of the model's
   window.
8. **Prices** (optional): what your provider charges, in dollars for a
   million tokens sent and for a million received, from its price page. The
   token meter then shows an estimate of the cost; when the line is longer
   than the pane is wide, the meter takes a second line, so the cost is
   never cut off.
9. **It may**: *Read the project, propose changes you review* (the
   default), or *Only read the project and answer*.
10. **RAI answers in**: *Studio's language* (the default), or *The language
    I write in*, which keeps the language of your latest message (and the
    one before it, when a message is too short to tell). See *Your
    language*, under Asking.

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
written; **Stop** stops it.

**Typing while it works.** You don't have to wait. Type your next message
and press **Enter** (or **Send**) while the assistant is still answering:
nothing is interrupted. The message waits in a line just above the box —
*Queued: and add a Clear button* — and goes by itself as soon as the answer
ends. **×** on that line takes it back. **Stop** is a button of its own: it
ends the answer, and a queued message goes back into the box, unsent, for
you to send or change.

![A message typed while the assistant works: queued above the box, with × to take it back](images/ai/queued.png)

**Your language.** The assistant answers in **Studio's language**
(Tools > Settings > General > Language), using RapidR's words in it, and
reads the manual's and the help's pages in that language (a page not
translated yet is read in English, and it says so). If you clearly write in
another language, it answers in yours: write in English in a Spanish Studio
and the answer is in English — even when your files, earlier messages or
the summary of a long chat are in another language. A message too short to
tell (*ok*, a line of code) is answered in Studio's language. To have it
keep the language you write in, choose **The language I write in** under
**RAI answers in** (Tools > Settings > AI).

**A dropped connection.** When a question can't reach the provider — its
name not found, no connection for a moment —, Studio tries once more, two
seconds later, by itself; you only see an error if that fails too. A
refusal (a wrong key, no credit left) is never tried again.

**What it is doing now.** While an answer works, a strip right under the
conversation — where the answer will appear — shows its work, calmly, each
thing once:

- **what it does now**, in words, on one row: *Thinking…*, *Reading the help
  of RListBox…*, *Looking at the form…*, *Writing main.rr (+318 lines)…*
  (the lines counted as the model writes them), *Checking (compiling)…*,
  *Fixing 2 problems…*, *Continuing (part 2)…*, *Summarising the chat to
  keep it short…* — with three small **dots** before it, the theme's
  accent at a gentle strength, swelling one after the other in a slow wave
  (1.2 s a cycle; they stand still when your system asks for reduced
  motion) — and the **seconds** the answer has taken at the right, muted;
- above that row, the **steps done**, ✓ each, one a line and wrapped (never
  cut off): the newest four, the older ones counted (*3 earlier steps*); a
  small pane keeps its room for the conversation and shows fewer;
- the **tokens** of this answer so far, in the token meter's line under the
  buttons — *This answer: 12,340 in (11,900 cached) · 1,204 out* — moving as
  they stream in; when the answer is done the line goes back to the
  session's totals. (The status line at the bottom is hidden while the strip
  shows: it said the same.)

**There is never a silent wait.** If the provider has sent nothing new for 10
seconds, the strip says what it is waiting for: *Waiting for Claude's answer…
24 s*. (A model on your own computer can be slow while it writes a long
change: wait, or **Stop**.)

![The strip under the conversation while the assistant works: the steps done above, then the dots, what it does now and its seconds; the tokens so far in the meter's line](images/ai/live-strip.png)

**For a screen reader** the strip is a polite announcement: each new step is
said once ("Writing main.rr", not each line it grows by; "Waiting for the
provider" once), never the seconds — they are read only when you ask for them
— and the dots are decoration, skipped. When the answer is done the status
line is back and says what it left (*Answered.*, *Waiting for you — a change
to review*), and if the model sends nothing for 30 seconds it says *No answer
from the model for 30 s: wait, or Stop.*

![The status line while the assistant works: what it does now (Reading hello_form.rr), the seconds beside Stop](images/ai/status.png)

When the answer is done, the strip goes, and the steps it took are one line
under its name — **✓ 6 steps · show** — that opens the list (and **hide**
closes it); the chat keeps them.

### A long answer goes on by itself

A model writes a long program in parts. When an answer reaches its length
limit, RAI doesn't stop: it asks the model to go on in smaller parts, and the
chat says so in one small muted line, in its place between the parts:

> *Answer was long: continuing (part 2 of up to 5)*

(A code block the cut left open is closed, so what follows is never shown as
code, and a change that was cut off in the middle is never applied: the model
writes it again in smaller pieces.) After the fifth part RAI stops, says why
in plain words — *the answer kept reaching its length limit (32,000 tokens),
5 parts in a row* — and offers **Continue in smaller steps**: a link that,
clicked (or followed with the keyboard, **Enter**), sends that request:
several changes of at most about 100 lines each, with short explanations.

![A long answer going on by itself: the muted line between its parts, then the link that asks for smaller steps](images/ai/live-continue.png)

### A smart chat: RapidR words answer

RapidR code in an answer looks like it does in the editor — the same
colours in every look. That holds for **code blocks** (a block with no
language that looks like RapidR's is coloured too) and for `inline code`.
And the words are alive:

- **Hover** a keyword, a component, a builtin or a member — `PRINT`,
  `RListBox`, `AddItems` — and a tooltip gives its one-line help:
  *PRINT — Statement: Writes text to the console.*; the pointer is a hand.
- **Click** it to open its entry in the **Help pane**.
- Every code block has **Copy** under it, and RapidR's code has **Insert**:
  the code is typed into the open editor at the caret, as one change you can
  undo with **Ctrl+Z**. With no editor open, the status line asks you to
  open a file first.

![An answer with RapidR words: coloured inline and in a block, the tooltip of a word under the mouse](images/ai/smart-chat.png)

Your own variables and SUBs have no entry in the language's help, so they
show none; the words the registry knows — every component, builtin,
statement, keyword and member — do. (The same help is one **F1** away in the
editor; the chat's words are a mouse feature, as a tooltip is.)

**The model, and ↻.** The list at the top of the pane is the provider's
models; pick another for the next question. **↻** asks the provider for its
models again (a new model, or one you just downloaded to your computer). The
provider itself is chosen in Settings, or with Set up RAI.

Under the box: **Send** and **Stop** at the left, **Compact** at the right
(and *What was sent* between them when the pane is wide enough); the context
meter on its own line, its words shorter when the pane is narrow (the whole
numbers in its tooltip); the tokens used; and one line that says who Send
shares the chat with — **Details** opens the whole sentence (what is shared,
and nothing else), **Less** closes it again.

![The Assistant pane: a question and its answer, the status line, the context meter, the token meter, what Send shares](images/ai/panel.png)

### Cards: what really happened

What the assistant *did* is written by Studio, never by the model, as small
bold **cards** in the chat:

- ✓ **Created** *Scientific Calculator*  main.rr · compiles  Open · Undo
- ✓ **Proposed a change to** main.rr  **+12** **−3**  Review (the counts in the
  diff's green and red, the signs saying it too; a screen reader hears
  "Proposed a change to main.rr, 12 lines added, 3 lines removed, Review")
- ✓ **Started from the recipe** *Calculator*
- ✓ **Fixed** 2 errors
- × **Refused** sites.rr: it is cut off at the end (a change Studio didn't
  pass on to you — see [Small, exact changes](#small-exact-changes))

The model's own words come after the cards: one or two sentences on what it
made and how to use it. A card's links work like the buttons they name — by
the mouse, and by the keyboard: from the box where you type, **Shift+Tab**
goes back to the chat and stops on each link in turn (the last card's **Undo**
first, then its **Open**), with a ring round the link and the chat scrolled to
it; **Tab** goes the other way and, past the last link, back to the box.
**Enter** (or **Space**) follows the link. A screen reader hears each one as
a link named *Open*, *Undo* or *Review*.

Small models sometimes say *"I have created your project!"* before they have
done anything. Studio notices, asks the model to really do it, and shows that
early draft as one grey line — *An earlier draft said this was done before it
was — Studio asked it to actually do it* — instead of the claim itself. The
cards always say what is true.

![The chat after "Build a scientific calculator": the program created, with Open and Undo, and the short reply](images/ai/chat-cards.png)

At the top, **Chats** lists this project's chats (see [Your chats are
kept](#your-chats-are-kept)), and **New Chat** starts another — the one you
leave is kept. A long chat costs more, as everything said so far is sent with
each question: **Compact** makes it smaller (see [The context meter and
Compact](#the-context-meter-and-compact)).

The assistant reads only what it needs, when it needs it: the project's
files, the components on your forms and their properties, RapidR's own
description of every component, property, method, event and function, and
what the compiler says about the code. It can't read anything outside the
project's folder, and never your key.

## Your chats are kept

Each project keeps its own chats. Close Studio, open the project again
tomorrow: its latest chat is there, as you left it. **New Chat** starts
another and keeps the one you leave.

**Chats** (at the top of the pane; **Tools > AI Chats**; `/chats`) shows the
project's chats in the chat's place, the newest first, the one open marked
*(open)*:

![Chats: this project's chats, the newest first; Open, Rename, Delete, Back; where they are kept](images/ai/chats.png)

- **Search** (the box on top) finds a chat by its title or by anything said
  in it; the line that has it is shown beside the title.
- **Open** (or a double-click) shows that chat; the one you leave is kept.
- **Rename** gives a chat your own name (Enter keeps it, Escape doesn't);
  until you do, a chat is named after its first question.
- **Delete** removes a chat from your computer at once; **Undo**, beside the
  list, puts it back.
- **Back** (or Escape in the search box) returns to the chat.

**Where they are kept.** On your computer, in Studio's own folder — never in
the project's folder, so a chat is never shared, committed or shipped with
your program by accident: `~/Library/Application Support/RapidR/chats` on a
Mac, `%APPDATA%\RapidR\chats` on Windows, `~/.config/rapidr/chats` on Linux —
a folder for each project, a file for each chat. In Studio on the web, in the
browser's storage for Studio's page. API keys and tokens you paste into a
question are replaced by *[key removed]* before anything is written. **Delete**
removes a chat's file; deleting the `chats` folder removes them all.

### Editing a question, Retry

Beside each of your questions, **Edit** puts it back in the box to change.
A bar above the box says what Send will do — *Send replaces it: its answer
and the 2 questions after it move to a saved copy of this chat* — and
**Cancel** (or Escape) leaves everything as it was. Send asks the changed
question; what followed it is kept, nothing lost, as a copy named *… (before
the edit)* in Chats.

![Editing a sent question: the bar says the replies below it move to a saved copy](images/ai/edit.png)

**Retry**, beside the last answer, asks the last question again — handy
after changing the model or the effort; the answer it replaces is kept as
*… (before Retry)*.

Questions from before the chat was compacted can't be edited (the model has
only their summary): ask again as a new question.

## Thinking harder: the effort

The **Effort** list beside the model asks the model to think more before it
answers: **Low**, **Medium** or **High** (**Auto**: as the model usually
does). Thinking more helps with hard problems — a tricky bug, a bigger
change — and takes longer and costs more: at High, an answer may use up to
16,384 more tokens for its thinking. `/effort high` does the same from the
box.

When the model thinks, its answer starts with **Thought for 12 s** — a link
that shows the reasoning the provider shares, and hides it again (the
link goes to the top of the chat, the reasoning opens under it). Adaptive thinking
is the model's choice: at High it thinks when the question needs it — an
easy question may still be answered straight away, with no link:

![Effort High: the answer starts with "Thought for 1 s · hide", the model's reasoning under it](images/ai/thought.png)

Anthropic's Claude models think adaptively at the effort you choose (an
older model, with a thinking budget); OpenAI and servers on your computer
are asked with their own setting, where the model has one; DeepSeek's
models choose for themselves, so the list is off for them. A model that
takes no effort says so once and answers as it usually does.

## Coach mode

RAI's pane has a dial under the model list: **Build** and **Coach**.

- **Build** (the default): RAI writes the change and you decide — every
  change is a diff you review and apply ([Changes, as diffs](#changes-as-diffs)).
- **Coach**: RAI teaches. It never writes the answer into your code. You
  ask, it gives one hint at a time, and you write the code yourself:
  writing the code yourself helps it stick.

![The dial in RAI's pane, Build and Coach, with Coach chosen; the empty chat says how Coach works](images/ai/coach-dial.png)

Click **Coach**, or press Tab until the dial has the keys and then Enter or
Space. Studio remembers your choice, and you can switch at any time;
switching starts the hints over.

### The hint ladder

Every problem has four hints, from a little help to a lot. RAI gives the
next one only when you ask for it:

| Hint | What RAI gives |
|---|---|
| 1. A nudge | A question that points the way. No code. |
| 2. The idea | The idea in plain words, with a tiny example about something else — not your program. |
| 3. The shape of the code | The steps, or your code's skeleton with blanks (`___`) for you to fill. |
| 4. The code | Only when you ask a fourth time: the code that solves it, shown as text. You type it, or copy it, into your program yourself. |

The strip under the chat says which hint you're on — **Hint 2 of 4** — with
two buttons:

- **Next Hint** asks for the next one (after the third it says **Show the
  Code**).
- **I Got It** ends the problem: your next question starts again at hint 1.

A question you type about the same problem stays on the same hint: asking
in other words doesn't move you to the next hint. Studio keeps the count,
and RAI is told which hint to write.

![Hint 1 of 4: RAI asks what should happen each time the button is clicked; Next Hint and I Got It under the chat](images/ai/coach-hint-1.png)

![Hint 3 of 4: the shape of the code, with blanks to fill](images/ai/coach-hint-3.png)

![Hint 4 of 4: the code shown as text, with only a Copy link and no Next Hint](images/ai/coach-hint-4.png)

### Help with an error

When your program doesn't compile, or stops with a run-time error, the
strip shows **Help With This Error**. RAI says what the message means, why
it happens on that line, and gives you a hint to fix it yourself. The fixed
line only comes after the hints, as hint 4.

![Help With This Error: RAI explains it doesn't know a name on line 24, and hints to compare it with its DIM](images/ai/coach-error.png)

### How Coach works

What Studio enforces, in its own code:

- **RAI can't change your files in Coach.** The tools that propose changes
  or make programs aren't offered to the model at all, and if a model tries
  one anyway, Studio refuses it before anything happens. There's nothing to
  review or apply, and code blocks have **Copy**, never **Insert**.
- **The ladder lives in Studio**, not in the model: Studio keeps the count
  and tells the model which hint to write each time. That's why Coach works
  with small models on your own computer too ([AI on this computer](local-ai.md)).
  When a first hint shows code anyway, Studio asks RAI once to write it
  again without, and the code isn't shown.

What RAI is asked to do (a model may still say more than asked):

- Give one hint at a time — no code in the first, a small example about
  something else in the second, blanks in the third.
- Read your project, check your code with the compiler, look at your
  running program (after you allow it for that run), and search the manual
  and RapidR's examples to pick the right idea.
- Write hints in the language you write in (or Studio's, when it can't
  tell).

What is sent and kept: the same kinds of things as in Build (your question,
the chat, what RAI reads), plus Studio's own Coach instructions; and the
same records on your computer — your chat, the audit log, and which mode
you chose.

A teacher can keep RAI in Coach mode: [For teachers](for-teachers.md#keep-rai-in-coach-mode).

## Changes, as diffs

The assistant never changes one of your files itself. It **proposes**: when
an answer has changes, a card in the chat and the **Review the Changes…**
button under it show them:

![Reviewing a proposed change: the file as it is on the left, as proposed on the right, Accept or Reject on each change](images/ai/review.png)

Each change has **Accept** and **Reject** (F7 goes to the next one; Enter
accepts it, Backspace rejects it); **Accept All**, **Reject All**, then
**Apply**. What you applied is one step in each file's undo: **Edit >
Undo** takes it back. **Tools > Review the AI's Changes…** opens the review
again.

The chat says each decision once, as it happened: *You applied the change
to server.rr*, *You rejected the change to main.rr*, or — when you edited the
file after the assistant read it — *The change to main.rr wasn't applied:
main.rr changed after the assistant read it*. A review opened again shows
only what still waits.

**Apply is all or nothing.** Before it changes anything, Studio checks every
change: a new file can be written where it goes (its folder is made when it
isn't there yet: `icons/folder.svg`), nothing is there already, and each
changed file is still as the assistant read it. Only then are the new files
written — exactly as their text is, in UTF-8 like every file Studio saves
(`lang/zh-Hans.po` keeps its 太阳系) — and the changes made in their editors.
If anything can't be done, nothing is: the status line and the Output say
why (*Nothing was changed: hello_form.rr/blocked.txt: its folder can't be
made*), and the changes still wait, to review again.

### Small, exact changes

The assistant changes an existing file by sending only the lines that
change, each with the few lines it replaces. A two-line fix is a two-line
change — **+2 −2** on its card, however long the file. Studio refuses a
change, and tells the model how to send it properly, when it is:

- **a whole file sent back for a few lines** — a file of more than 40 lines
  that the change mostly keeps;
- **cut off before its end** — a reply that ran out of room stops in the
  middle of a line, like `END FUNCTI`, or with a SUB still open;
- **a syntax error** — a line RapidR can't read.

The last two are refused in every file, a module included. A refused change
never reaches you; if the model doesn't send it again, properly, a **×
Refused** card says what happened.

![A change cut off at its end, refused: the card says so, there is nothing to review](images/ai/refused.png)

### Changes wait for you

You don't have to review a change before you ask the next question. The
changes you haven't applied or rejected yet wait, and the assistant works
with them: it reads those files as it proposed them, and the compiler checks
your project with every waiting change in place — so a change to main.rr
that calls a SUB a waiting change adds to a module compiles. **Review the
Changes…** shows them all together.

- A newer change to the same file builds on the one that waits, and the
  chat says *The change to server.rr proposed before was replaced by a
  newer proposal*.
- If you edit a file yourself while a change to it waits, that change is
  set aside at your next question, and the chat says so: it is never
  applied over your own edits.
- **The waiting changes are kept with the chat.** Close Studio, open the
  project again tomorrow: its chat comes back with **Review the Changes…**
  ready, and Apply works as before. Opening another project keeps them with
  their own project's chat, never offered to apply in this one.

## A new program: "Build me…"

Ask for a program — *"Build me a tip calculator"*, *"Make a program that
lists the files of a folder"*. The assistant writes it and checks it with
RapidR's compiler; when its answer ends, the program **opens in Studio**,
ready: its form in the designer, its code a click away. No dialog, no folder
to choose — it goes in your projects folder (**Documents/RapidR/Projects**),
in a folder named after it: *Scientific Calculator* becomes
`ScientificCalculator` (`ScientificCalculator2` if that name is taken). Move
or rename it later if you like.

A banner across the top says what happened, and the keyboard is already on
its first button:

![A new program from the assistant, open in Studio: the banner with Run It, Undo and What Did It Do?](images/ai/banner.png)

- **▶ Run It** (or **Enter**, or **F5**) runs the program. Nothing runs until
  you press it.
- **↺ Undo** closes the program and removes exactly the files the assistant
  wrote — nothing else. If you changed or added files in its folder since,
  Studio asks first, and the files you added stay. The project you had open
  before comes back.
- **What Did It Do?** shows the Assistant pane at the program's card.
- **×** closes the banner.

If the program still has errors after the assistant's tries, it opens all the
same and the banner says so: **▶ Run Anyway**, **Ask It to Fix** (the
assistant gets another go), or **↺ Undo**.

![The same in the dark look](images/ai/banner-dark.png)

### Review first

Prefer to look before anything is created? Turn on **Tools > Settings > AI >
Ask before the assistant creates or changes files**. Then a new program waits
behind **Preview the New Program…**: its files, the code, and the form as it
will look; nothing is written until you choose a folder and press **Create
and Open**.

![A new program from the assistant, previewed: its file, its code and its form](images/ai/new-program.png)

Until you press **Create and Open**, the new program is still the
assistant's to fix: if the compiler finds errors, it reads and corrects its
own files, and the preview always shows the latest version. Ask for a change
before creating it (*"make the buttons bigger"*) and the preview follows.

### Recipes and friendly hints

When you ask for a new program, Studio first looks for the closest of its
**recipes** — short, working programs of the shapes people ask for most: a
calculator, a tip calculator, a stopwatch, a to-do list, a unit converter, a
quiz, a drawing pad, a password generator, a CSV viewer, a list with Start /
Stop buttons ([examples/recipes](../../examples/recipes/)) — and, for programs
that handle a machine's insides, ten systems recipes: memory, bits and flags,
a tiny CPU, the stack, a hex dump, a disassembler, a binary file, video
memory, a keyboard, cycles per frame ([Recipes for systems
programs](recipes-systems.md)). It sends that
recipe with your request, so the assistant adapts a program that already
works instead of inventing one. The search runs on your computer, in
English, Spanish and Portuguese; *What was sent* shows the recipe that went.

Each time the assistant checks its code, Studio adds **friendly hints**
under the compiler's messages: the exact fix for habits from other languages
(`!=` is `<>` in BASIC; there is no `BOOLEAN`, use `INTEGER`; one `AS` per
name; no `[ ]` lists), a name that is never given a value (with the closest
one the program has), a classic constant used without `$INCLUDE "RAPIDQ.INC"`
in a `.bas` program (a `.rr` program has them built in),
a window that is never shown (`Form.ShowModal` missing), and a handler that
does nothing yet or only says what "a real implementation would" do. Once
it compiles, Studio also looks at the window as you would: a control cut off
by the edge of its form, two controls on top of each other, a caption longer
than its button. For a keypad or a toolbar the assistant can ask Studio to
lay out the buttons (exact positions, one shared handler) instead of doing
the arithmetic itself. The compiler itself is unchanged: your own programs
compile exactly as before.
These two help small local models most — see below.

### The language card, the outline and the manual

Every model, even a small one, gets RapidR's **language card** with each
question: about 1,300 tokens of the rules models actually get wrong — DIM a
variable before its first use, declare a global once in the whole program,
call only SUBs that exist, `FOR i AS INTEGER`, `True` is -1 while `Checked`
and `FILEEXISTS` give 1, arrays, event SUBs and their parameters, RapidR's
names. It isn't written by hand: it comes from the language registry and
the compiler's rules, and every example on it is compiled and run by a
check, so it can't teach a wrong signature. See it with
`rapidr lang export --card`.

Two more things the assistant can ask Studio for:

- **The project's outline** — its files in the order the compiler reads
  them (which file `$INCLUDE`s which), and in each file its forms and their
  components, its SUBs and FUNCTIONs with their parameters, its globals with
  their types, its CONSTs and TYPEs, each with its line. The assistant looks
  at it before changing a project of several files, so it doesn't declare a
  name twice or call a helper no file has.
- **The manual** — the three sections of this manual that answer a
  question best ("how do I set a breakpoint", "serial port"), searched on
  your computer.

The mistakes these answer, and what RapidR does about each (a clearer
compiler message, a line on the card, or a small extension like
`FOR i AS INTEGER`), are listed in the
[mistake ledger](../rai-mistake-ledger.md).

## What was sent, and the audit log

**Tools > What Was Sent to the AI…** shows the last request exactly as it
was sent — with your key replaced by *(your API key, not shown)*.

![What was sent: the request, its headers without the key, RapidR's instructions first, the project's facts, the tools](images/ai/what-was-sent.png)

Every request, every file the assistant read, every change it proposed and
what you decided is written to an **audit log** on your computer, one JSON
object a line (its last 10,000 lines): `~/Library/Application
Support/RapidR/ai-audit.jsonl` on a Mac, `%APPDATA%\RapidR\ai-audit.jsonl` on
Windows, `~/.config/rapidr/ai-audit.jsonl` on Linux; on the web, the
browser's storage for the page (its last 500). It never holds your key:
keys and tokens are taken out of every line before it is written. To clear
it, delete that file (on the web, clear the site's data in the browser);
RapidR starts a new one with the next request.

## Privacy and safety

- **Cloud services are for adults.** If you're under 18, ask a parent or
  teacher to make the key. RapidR needs no account, and a model on this
  computer is free.
- **Send shares** your message, the chat so far, RapidR's instructions (the
  same for every project), what the assistant reads of the open project and,
  while your program runs and you allowed it (a card in the chat asks once a
  run), what the program's windows show — nothing outside it, nothing before
  you press Send. See [A program's eyes and hands](app-eyes-and-hands.md#in-rapidr-studio-ai-apps-and-rai-look-at-your-running-program).
- **Text in your files is data.** A comment or a data file that says
  "ignore your instructions and …" is something the assistant reports, not
  something it obeys; and whatever it is persuaded of, it can only propose
  changes you review, or make a new program in a new folder of its own
  (or only answer, with *Only read the project*).
- **Nothing runs by itself.** A program the assistant made runs only when
  you press Run.
- **At most 25 tool uses** for one question; Stop works at any time.
- **Keys** are never written into a project, a build, a log, the audit log
  or a screenshot of RapidR's. A key someone wrote into a file of the
  project is replaced by *(an API key, not shown)* before the assistant
  reads the file.
- **A model on your own computer**, in a local AI app: Studio sends what you ask only to that app, so it stays on the computer unless the app is set up to pass it on.
- **Your chats stay on your computer**, in Studio's folder, never in the
  project (see [Your chats are kept](#your-chats-are-kept)); keys and tokens
  in them are removed before they are written, and **Delete** removes a
  chat for good.

## Tokens: what makes a question cheap

Studio keeps each question small:

- RapidR's instructions come first and are the same every time, so the
  providers' caches serve them again at a lower price — the meter says how
  many tokens came *from the cache*.
- The project isn't sent whole: the assistant reads the parts it needs.
- What the assistant read for an earlier question is shortened when you
  ask the next one; a long chat is summarized past a size.
- **Longest answer** limits each part of an answer.

Measured on 2026-10-15 with each provider's fast, low-cost model, *"Build me
a hello-form program"* (a working form program, made and checked) took from
about 7,600 to 22,000 tokens sent — between a third and two thirds of them
from the cache — and 400 to 2,000 received: a fraction of a cent.

### The context meter and Compact

Under the buttons, the **context meter** shows how much the next question
will send — *Context: 12,400 of 700,000 (2%)*. At the right end (70% of the
model's window: 700,000 tokens for claude-opus-5-5 or claude-sonnet-5-5,
whose window is 1,000,000 — RapidR knows Claude's windows before the
provider's model list is fetched; 40,000 when a model's window isn't known,
or the lower point you set) the chat is compacted by itself:
summarised in one short request, and the next questions start from the
summary. **Compact** (or `/compact`) does it now — the chat says *The chat
was compacted: what each question sends of it went from about 12,400 tokens
to about 900*. Nothing you see goes: the chat still shows every question and
answer; only what is sent becomes smaller. (A chat of one question has
nothing to compact yet, and says so.)

### Commands: type /

Type **/** at the start of the box: the commands are listed above it, and
narrow as you type (`/c` → `/compact`, `/chats`). **Up** and **Down**
choose one, **Tab** completes it, **Enter** runs it, **Escape** closes the
list. A command is Studio's: it is never sent to the model.

| Command | What it does |
|---|---|
| `/compact` | Summarise this chat now: each question then sends less |
| `/new` | Start a new chat (this one is kept in Chats) |
| `/chats` | This project's chats: switch, rename, delete, search |
| `/model` | Choose the model; `/model name` picks it |
| `/effort` | How hard it thinks: `/effort low`, `medium`, `high` or `auto` |
| `/undo` | Undo the assistant's last change: a new program it made (as the banner's Undo), else the files the last review changed, while each is as the review left it |
| `/help` | The commands |

![Typing / lists the commands; /c narrows them](images/ai/slash.png)

### Small models on your own computer

A local model (Ollama, LM Studio, mlx-serve) takes only so many tokens at
once — its *context window*, often 8,000 to 32,000. Studio asks the server
for it before a question and keeps everything inside it: each reply gets at
most a quarter of the window (and at most 4,096 tokens on this computer,
whatever **Longest reply** says), and when the chat nears 70% of the window
the files the assistant read or wrote earlier are replaced by short notes —
it reads them again when it needs them. The status line stays honest: when
an answer says it changed something but no change reached you, the
assistant is asked once to really make it, and if it still doesn't, the
status says *"The assistant didn't make the change you asked for"*; a
change that doesn't compile says so too. The assistant can't run your
program, but it sees the last lines your last run printed. A model of 4 to 9
billion parameters can build small programs this way; for bigger ones a
larger model does better.

**A small window gets the lite set.** RapidR's full instructions — how the
assistant works, the language card, an example program, the names of every
component — and its tools are large: Claude counted about 11,000 tokens for
a first question, the project's facts included. A model whose window is
under 16,384 tokens (many small local models run with 8,192) would have no
room left, so it gets the **lite set** instead: a few lines on how to work,
the language card, and the eight tools a small model needs (read, check,
search the manual, change, create), about a third of the full set. It can
start, and answer, in an 8,192-token window.

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
- **ReplyLanguage**: the language the answers are in. `""` (the default)
  says nothing about it — the model chooses, as a program that translates
  needs. `"app"` answers in the program's language
  (`Application.Language`), unless the user's latest message is clearly in
  another: that one, as Studio's assistant does by default. `"auto"`
  follows the language of the user's latest message (**UserLanguage** says
  which it read; kept while a message is too short to tell): every request
  ends by saying it, and a long chat's summary is written in it, so the
  model doesn't drift into another language. A language's name
  (`AI.ReplyLanguage = "Spanish"`) always answers in that one.
- A request that never reached the provider (no network for a moment) is
  tried once more by itself, two seconds later; a refusal never is.
- **MaxTokens** (the longest answer; 1024 by default), **System** (your
  instructions), **SummaryAt**, **Usage** (*"1,000 tokens sent, 20
  received"*), **InputTokens**, **OutputTokens**, **CachedTokens**, **Cost**
  (with **PriceIn** / **PriceOut** set), **Audit** (the audit log above).
- **PricingURL** and **KeysURL**: the provider's price page and the page
  that makes keys, for your own links.
- **A chat that lasts**: **ChatState** is the conversation as text (JSON) to
  keep in a file or a database and set back later — API keys and tokens in
  it are removed. **Rewind(N)** takes question *N* and everything after it
  out and gives back its text, to edit and ask again; **QuestionCount** and
  **QuestionText(N)** number them; **Editable** adds *Edit* and *Retry*
  links to **Transcript** for an RMarkdownView.
- **What it is doing**: **Activity** (*"Reading main.rr"*, *"Writing the
  answer"*), **Elapsed** (seconds since the answer began) and **Quiet**
  (seconds since the provider last sent anything), for a status line.
- **Context**: **ContextTokens** against **ContextLimit** (where it
  compacts by itself), **Compact** now.
- **Effort** (`""`, `"low"`, `"medium"`, `"high"`): the model thinks more
  before it answers, where the provider has it (**EffortSupported**); the
  reasoning shows in **Transcript** as *Thought for N s*, and
  **ToggleThought(N)** shows or hides it.

Every property, method and event: [RAI in the reference](reference/members.md#rai).

### In the designer

Drop an RAI on a form and pick its **Provider** in the inspector. Type the
key in the **APIKey** row: it is kept **in your keychain**, under the name
`RapidR/<project>/<component>`, and the code gets only where it is —
`APIKeySource = "keychain:RapidR/<project>/AI|ask"` — never the key. Then
the **Model** row lists the provider's own models, fetched in the
background (*↻ Refresh the list* fetches them again). The key is kept for
**that program**: run from Studio, the program finds it; a built app is
another program and asks its user once (`|ask`, then **RememberKey**):

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
- **Each program has its own keys.** What a program keeps with
  `keychain:NAME` is kept for that program only (in the keychain, under
  the program's own name: `rapidr-app:<program>:RAI`). A program never
  reads RapidR Studio's own keys (`RapidR Studio/anthropic` …) or another
  program's, so a program you download can't take your key and send it
  somewhere. A program that names Studio's key finds none, and **Error**
  says so: *"… is RapidR Studio's own key, and a program can't read
  Studio's keys"*. Give the program its own key instead: type it in the
  designer's **APIKey** row, or let the program ask (`"keychain:MyApp/Key|ask"`,
  **OnKeyNeeded**, then **RememberKey**). Keys a program kept before Preview
  4 are asked for once again.
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
