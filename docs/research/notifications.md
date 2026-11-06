# Research: notifications, toasts and alerts (lane NOTIFY, 2026-11-06)

What people love and hate about the small messages an application shows them, gathered before RapidR's own design ([the design guide's Notifications section](../design-guidelines.md#notifications)). No product is named: this is what the patterns do well and badly, and what the accessibility standards ask. The sources are the standards (WCAG 2.2, WAI-ARIA 1.2) and years of common complaints about developer tools, operating-system notification centres and everyday apps; no site was fetched for it.

## What people hate

| Complaint | What goes wrong |
|---|---|
| **It is huge.** | A card as big as a dialog for "5 files were updated". It pushes the editor down, covers what the person is reading, and its buttons are as large as a dialog's. The trigger for this lane. |
| **It vanishes before it is read.** | A fixed two seconds whatever the text says; the person looks up and it is gone. Worst for errors and for anything with a button. |
| **It steals the focus.** | A message that takes the keyboard while the person types loses what they typed or makes them press Esc. |
| **It stacks without end.** | Ten toasts for ten files, one on top of the next, covering the screen. People close them one by one, or stop reading them. |
| **The same message again and again.** | "Saved" every second; the same warning on every keystroke. |
| **It covers something that matters.** | The status bar, the Save button, the line the person is typing on. |
| **No way back.** | A toast missed by a second is lost for good: what did it say? |
| **It talks only in colour.** | A red box and nothing else. Colour-blind people, screen reader users and high-contrast users get nothing. |
| **A dialog for news.** | A modal "OK" for something that needs no answer. |
| **It shouts.** | Red for something that isn't wrong; sounds nobody asked for; blinking. |
| **Screen readers.** | Not announced at all, or announced as an interruption in the middle of a sentence, or announced twice. |
| **Jargon and blame.** | "Error 0x80004005", "Invalid input", "Operation failed". |

## What people love

- **Small and calm.** One or two lines, an icon, a quiet link. It says what happened and gets out of the way.
- **It waits while you look.** The time runs only while the mouse is not over it and the keyboard is not on it; it comes back with a fresh moment after you leave.
- **Undo instead of "Are you sure?"**. A toast with Undo lets the person act first and reconsider after, which is faster than asking every time and safer than not asking. (Use only when there really is an undo.)
- **Grouping with names.** "5 files were reloaded: main.rr, form1.rr, utils.inc +2" beats five toasts, and beats "5 files were reloaded" with no names: the person knows at once whether their file is among them.
- **A history.** A bell with a count. Missing a toast costs nothing, because the list is one click away, with the action still working.
- **A predictable place.** Always the same corner, never over the controls the person is using.
- **A decision where it belongs.** A question about one file is asked on that file, one line tall, not in the middle of the whole window.
- **It respects the system.** Reduced motion, text size, high contrast and the person's language are all followed without a setting to find.

## Timing: how long should it stay?

WCAG 2.2 *2.2.1 Timing Adjustable* (level A) wants a time limit to be off, adjustable or extendable by the person, unless it is essential; *2.2.2 Pause, Stop, Hide* (A) wants moving or self-updating information the person can pause. A toast that hides by itself is a time limit. The design answers the criterion three ways (this is the design's intent, not a claim of conformance): it **pauses** while the mouse is over it or the keyboard is on it, a **history** keeps everything it said (the message is never really gone), and a **setting** makes toasts stay longer or until closed.

How long is "long enough"? As a working assumption (reading speeds vary widely), adults read about 240 words a minute; children, second-language readers and people with dyslexia, about 130 to 180; and a person has to notice the toast first, then read, then reach for its button. The formula the guide uses, in milliseconds:

```
stay = 3000 + 80 x (characters) [+ 2000 when it has an action]
       at least 5000, at most 20000
       a warning stays one and a half times as long; an error stays until closed
```

A 55-character message with an action stays about 9 seconds; a two-word "Saved." stays the minimum 5. 80 ms per character is a slow reader's pace (about 150 words a minute), on purpose.

## Where, and how many

- **Where:** a corner of the area it speaks about (the editor area for the editor's news), 12 pixels from the edges; bottom-right keeps it away from the toolbar, the tabs and the first lines of text, and out of the way of a person typing near the top. Never over the status bar (that is where the history's bell lives) and never over a dialog.
- **How many:** three at most. A fourth pushes the oldest one out (it stays in the history). Messages with the same *key* replace each other, so "5 files" becomes "7 files" instead of a second toast.
- **When it arrives:** it slides in (12 pixels and a fade, 160 ms); with reduced motion it simply appears. Nothing blinks (WCAG 2.3.1: three flashes a second at most).

## Three levels, because one level is wrong for most messages

1. **Information** (something happened; nothing to decide): a toast.
2. **A decision** (the person must choose before work goes on): not a toast, which can vanish, but a strip that stays on the thing it is about until answered. A modal dialog is for consequences (consent), not for "the file changed".
3. **History** (what was said, a while ago): a list, not a log file.

## Screen readers and the keyboard

- WCAG 2.2 *4.1.3 Status Messages* (AA): a message that appears without taking the focus must be available to assistive technology through a role or property. In WAI-ARIA that is a **live region**: `role="status"` (polite: said when the reader has finished its sentence) for news, `role="alert"` (assertive: said at once) only for what cannot wait. A toast is polite; the decision strip is assertive, **once**, when it appears (not again when it is redrawn or when the editor's text changes).
- The toast's text and its action must be *in the tree* (a status with a button), so a screen reader user can reach the button, not only hear the words.
- *2.1.1 Keyboard* and *2.4.3 Focus Order*: the action and the close button are reachable without a mouse. A toast never takes the focus by itself (RapidR Studio's one deliberate exception is the new program the assistant just made: its toast has the keyboard on its first button, so Enter runs it, and Escape gives the keyboard back); a key (the history's key) and the pane-cycling key reach it; Esc closes the focused toast and returns the focus to where it was.
- *1.4.1 Use of Color* (A): the icon and the words say what kind it is; the colour agrees. *1.4.3 / 1.4.11*: text 4.5:1, icons and marks 3:1; high contrast frames the card in the text colour. *2.5.8 Target Size* (AA): the buttons are at least 24 x 24.
- The text can be **copied** (an error's words are what a person pastes into a question), and nothing in a notification shows a secret: a key or a token is scrubbed before it is shown or kept, exactly as the Output pane does.

## Words

- Say what happened, in the past tense, with the names: "main.rr was reloaded", not "Success!" or "File operation completed".
- Name up to three things, then "+N". Never a bare count when names exist.
- The action is a verb of one or two words: *Show*, *Undo*, *Open*, *Compare*.
- No "Error:", "Warning:" or "Info:" in front: the icon and the kind say it. No exclamation marks, no blame, no jargon.
- Titles are not needed: one sentence.

## What this means for RapidR (decisions)

1. Three levels: an **information toast**, a **decision strip** on the editor it is about, and a **history** behind a bell in the status bar.
2. The toast: a compact card, bottom-right of its area, three at most, merged by key, stays 3 s + 80 ms per character (+2 s with an action; at least 5, at most 20), paused by the mouse and the keyboard, never takes the focus, polite for screen readers, reduced motion respected, an optional second action and a close button.
3. The decision strip: one line tall, the warning colour's 4-pixel mark, compact buttons, on that editor only, assertive once, reachable by the pane-cycling key; two lines when the editor is too narrow for one.
4. The history: the bell's count of unseen messages, a list with the time, the text and the action again; the last fifty are kept; **Clear**.
5. One component, **`RNotify`**, for every RapidR program (RapidR Studio uses it first), drawn by the UI kernel in the program's own look on the desktop and the web, with the same members for people, RAI and AI apps: what is visible can be read and its action pressed.
6. Document tabs keep their names ([the Tabs section](../design-guidelines.md#tabs)): a minimum width, the middle of a long name replaced by an ellipsis so the extension stays, scroll arrows at the ends, and a list of all open tabs.
