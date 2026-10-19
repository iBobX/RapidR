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
