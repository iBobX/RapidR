# How the assistant should feel: apply, show, undo (plan, 2026-10-18)

Robert's walk on 2026-10-18 (gemma-4 E4B, "Build a scientific calculator with graphics"): the program it made was good, but **the experience was confusing**.
- **The chat contradicted itself.** First "I have created a new project", which wasn't true, then Studio's note, then "creation failed", then "I tried again".
- **The result hid behind a small Preview… button,** with the same status shown twice, then a modal dialog with a folder field and Create and Open.
- **Easy to miss.** A child or someone with low vision may never notice that something was made and is waiting for them.

Robert asked: what's the best approach for RAI to be easy, intuitive, accessible, powerful and fast, true to RapidQ's spirit (draw, write, Run)?

## The principle: show the result, make undo trivial, ask only when it matters

Today the flow is *propose → preview → approve → create*. That's safe, but it puts three gates between the user and the magic moment. Modern assistants converge on something better: **apply the change where you work, show clearly what changed, and make undo one click.** You review the result *in place*, not in a separate dialog. Safety comes from three things:
- **where** the change lands;
- **nothing runs** without you;
- **every step can be undone**;

not from making people approve a diff they may not be able to read.

## 1. A new program: straight into the IDE

- "Build me a calculator" → Studio **creates the project at once** in Documents/RapidR/Projects (the `ProjectsFolder` rule), with a folder name made from the title. No folder question, no modal; you can rename or move it later.
  - It's zero risk: a new folder that touches nothing else.
- It **opens in the IDE** with the form in the designer, so you *see* your program immediately.
- **A big, friendly banner** sits at the top of the editor, and focus moves to it:

  > ✨ **The assistant made this program for you.** ▶ **Run it** · ↶ Undo · What did it do?

- **Run stays a deliberate press,** never automatic, because a program can read files or use the network. For a kid, pressing the green ▶ *is* the magic moment.

## 2. Changes to your program: tracked changes in the editor, not a dialog

- The assistant's edits go **into the editor's open buffers, not to disk**. They show like tracked changes:
  - added lines tinted, with a `+` in the gutter;
  - removed lines struck through, with a `−`;
  - colour is never the only signal.
- **The bar at the top of the editor:** "The assistant changed 2 files (14 lines). **Keep** · **Undo** · Next change ⌄". Per change there's Keep / Undo, and, in Pair mode, a *why* line (see [learn-with-rai-plan.md](learn-with-rai-plan.md)).
- **Nothing reaches the disk until you Keep or Save.** The unsaved buffer *is* the approval step. Ctrl+Z undoes it like any edit, and closing without saving discards it.
- **Checkpoints:** each assistant turn is a checkpoint ("before: add a graph"), so "Undo everything since…" is one click. Later it becomes the gentle entry to versions and Git (Learn with RAI §9).
- **Review first:** a Settings option, "Ask before the assistant changes files", which teachers and cautious pros can choose. It shows the same tracked changes, but unapplied until you press Keep. It's also the default when the project is RapidQ-compatible and opened from someone else's folder.

## 3. The chat tells the truth, in cards

- **Studio writes the facts:** each tool action is a compact card:
  - "📄 Created *Calculator* · main.rr · ✓ compiles · [Open] [Undo]";
  - "✏️ Changed main.rr · +12 −3 · [Show] [Undo]";
  - "🔍 Used the recipe *Calculator*";
  - "⚠️ 2 errors fixed".
- **The model writes the explanation:** one or two sentences in its own words, after the cards. The instructions ask for exactly that.
- **A reply that claimed something untrue** before Studio's nudge collapses to "*An earlier draft said this was done before it was; Studio asked it to actually do it.*" (expandable). The final answer shows in full.
- **One status, in one place.** No duplicate lines.
- **Progress while it works,** as a live checklist: *Finding an example… ✓ · Writing main.rr… ✓ · Checking… ✓ · Fixing 2 errors… ✓*. A small local model can take a minute; watching it work makes the wait fine.

## 4. Accessible from the first second

- **Focus moves** to the result banner and the Keep/Undo bar. Every card, banner and step is announced through AccessKit live regions ("The assistant made Calculator. Run it, Undo, or What did it do?").
- **Optional spoken summaries** through RSpeech (Phase 3D).
- **Large targets and high contrast** for the banner and bar; diff markers that don't rely on colour; the screen reader reads "added line" / "removed line".
- **Keyboard:** F5 runs; Ctrl+Enter keeps; Ctrl+Z undoes; Alt+↓ goes to the next change. All of these are listed in the banner's tooltip and the manual.
- **Plain words** everywhere: "Keep" and "Undo", not "Apply proposal".

## 5. What stays strict (security)

- The assistant writes **only inside the open project**, or creates a new project folder. Never anything else.
- It **never runs a program** or anything else; only you press Run.
- Edits stay **in memory until Keep/Save**, with checkpoints and undo.
- Files and tool results stay *data*, never instructions (unchanged).
- **Read-only** permission still exists: the assistant then only explains.

## Order of work

1. A new program created and opened directly, with the result banner; Run is primary; no folder dialog. Folder names made from titles ("Scientific Calculator" → `ScientificCalculator`).
2. Chat cards, the collapsed "earlier draft", one status, and the live progress checklist.
3. Tracked changes in the editor buffers, the Keep/Undo bar, and checkpoints. The current diff dialog becomes the "Review first" option.
4. Accessibility pass: focus, live regions, keyboard, contrast, a screen-reader walk, and a captioned GIF for the manual.

## 6. The review, made beautiful (Robert, 2026-10-19)

After 2.120.0, Robert asked for two things:
- **The diff should be prettier, and docked as a tab in the IDE**, not a separate window.
- **The banner and chat buttons should match Studio's streamlined design**: today they look foreign.

**What the best diff views do (the model to follow):**
- **A side-by-side design:** a comparison, with a soft coloured *ribbon* connecting each change between the two sides, and calm tints rather than loud red and green.
  - It has a minimap of changes in the scrollbar.
  - Each change is a rounded block with its own discard control.
  - Its typography is identical to the editor's.
- **A second design:** the same connectors between the sides.
  - Per-change ≫ / × arrows in the gutter, to take or drop a change.
  - Word-level highlights inside changed lines.
  - "Next / previous difference" always visible.
- **An inline design:** the change stays *in the editor* (inline, unified).
  - A small floating Keep / Undo pill sits on each change.
  - A bar shows "1 of 5 changes".
  - Word-level highlighting is standard.
- **Web and Git clients:** a unified/split toggle; folded unchanged lines ("⋯ 24 lines"); a file list with +/− counts.

**For RapidR Studio:**
- **A "Review" tab docked in the editor area**, next to the file's own tab.
- **Split / unified toggle,** remembered.
- **Calm tints from the theme's tokens:**
  - in light, dark and high contrast;
  - with `+` / `−` markers and screen-reader names, so colour is never the only signal.
- **Word-level highlights** inside changed lines.
- **Connecting ribbons** between the sides in split view.
- **Unchanged runs folded,** with a "⋯ N lines" button.
- **A change minimap** in the scrollbar.
- **Per-change controls:** a small Keep ✓ / Undo ↺ pill on each change. In Pair mode, the change's one-line *why* goes under it (Learn with RAI).
- **A slim header bar** in the editor's own style: "main.rr · 5 changes · +496 −70 · Keep All · Undo All · ↑ ↓". Keyboard: F7 / Shift+F7 move between changes, Enter keeps, Backspace undoes.
- **The same visual language** for the new-program banner, the chat cards and their buttons: Studio's toolbar buttons and tokens, not default push buttons. It also leads into step 3 (tracked changes in the editor itself): the same pills, inline.

## 7. RAI that can see and test (Robert, 2026-10-19)

From the demo takes: Haiku builds apps that compile but sometimes don't *work* (a calculator whose `=` was missing, functions that broke the display). RAI answered honestly: *"I can't reproduce a failure from the code alone, and I can't run the program."* That's the gap to close. RAI should **see and try** what it built, like a person would:
- **Screenshots, with permission:** RAI can ask to see the designer or the running program ("May I look at the window? It uses more tokens"). The image goes to the model only after the user says yes. Off by default; the model must support images (cloud models do; some local ones too).
- **Run and drive the program, with permission:** a `run_program` tool that starts the app headless with scripted events (Studio already has `RAPIDR_TEST_EVENTS` / `RAPIDR_TEST_DUMP`, which the recipes' click tests use). It returns what happened, for example "pressed 9, *, 5, =; Display.Caption = 45", plus errors and, if allowed, a screenshot. Then RAI can test its own app ("2 + 3 = gives 5? sin(x) plots?") and fix it before the user ever sees it.
- **A permission mode, one dropdown,** like the best coding tools:
  - **Ask every time** (the default for new users and kids): each change, each look, each run asks;
  - **Accept edits:** changes apply as tracked changes (Keep/Undo), looks and runs still ask;
  - **Plan only:** it explains and proposes, it changes nothing;
  - **Trusted:** it edits, looks and test-runs on its own, inside the project; the run is sandboxed, with no network unless the program asks and the user allows.
  - Teachers can lock the mode (Learn with RAI).
- **Safety:** screenshots show only Studio's own windows and the program it runs, never the rest of the screen. Running stays inside the project and its data files. The same audit log records every look and run.

**The banner, refined (Robert, 2026-10-19, with a screenshot):** today's banner is a tall, full-width strip with big push buttons. It's clear, but heavy: *"it hurts the eyes"*. The next version is **slim and native to Studio**:
- **One line, about the height of the tab bar,** with a soft accent tint and a small ✨ icon.
- **Text buttons in the toolbar's style** (▶ Run · ↺ Undo · What did it do?), not big framed buttons.
- **It sits where the editor's tabs are, or as an info bar inside the editor area,** like an editor's "file changed on disk" bar, not across the whole window.
- **It folds itself away** after the first Run, or after a few minutes. It's announced once to screen readers. Its height doesn't jump the layout.
- **The manual for RAI:** a `search_manual` tool. The same BM25 search as the recipes, run over `docs/manual/` split by section, returns the 2–3 most relevant sections, shortened. The registry tells RAI *what exists*; the manual tells it *how to use it well* (events, DataSource, plotting a CSV). It's local, cheap, and needs nothing to download.
