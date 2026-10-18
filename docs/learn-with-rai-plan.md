# Learn with RAI: the assistant that teaches (plan, 2026-10-18)

Robert's idea (2026-10-18): RapidR is for students and people who don't program yet, and programming must stay *fun*. So Studio's AI must not only build programs for us: it must help us **learn**. It explains, tests us, gives reasons, and drops a tip at the right moment, so that people and AI work together and people keep growing.

This builds on Phase 3D ([speech-and-tutorial-plan.md](speech-and-tutorial-plan.md)). Guided lessons are the scripted path; this plan is the AI path that goes with them.

## The problem we solve

An AI that writes everything gives you a program, but you learn nothing ("illusion of competence"). A tutor that only lectures kills the fun. Learning research points the same way again and again:
- **Retrieval practice:** you remember what you *recall*, not what you re-read.
- **Spacing:** come back to an idea days later.
- **Worked examples, then fading:** see one done, then do more and more of it yourself.
- **Productive struggle with hints:** a hint, not the answer.
- **Predict, then check:** guess what code will do, then run it (PRIMM: Predict, Run, Investigate, Modify, Make).
- **Learning by teaching** (the protégé effect): explaining to someone else is one of the strongest ways to learn.

RapidR has what most tutors don't: a real debugger, a live designer, a compiler that understands every line, and the registry. **The tutor can see and *show* what the code really does.**

## 1. One dial: how much the AI does

A dial in the Assistant panel, always visible, changeable any time:

| Mode | The AI… | For |
|---|---|---|
| **Build** | writes the change and you apply it (today) | getting things done |
| **Pair** | writes the change, and every hunk has a one-line *why*; after you apply, one optional question ("what would happen if Tip were 0?") | learning while building (**the default for new users**) |
| **Coach** | never writes the answer. A hint ladder: nudge → concept → the shape of the code → shown only if you ask a fourth time | learners, homework |
| **Quiz me** | asks about *your own* code: predict the output, find the bug, put the lines in order | review, exams |

Teachers can lock the dial (for example to Coach) for a class or an exam.

## 2. Moments that teach (right on time, never spam)

Tips come from **what just happened in your code**, not from a random list. The compiler and the debugger already know these moments:

- **The infinite-loop catcher (WOW):** a program that's been busy for a few seconds without handling events → Studio pauses it *on the loop line*, highlights the loop, and RAI explains: "This `DO WHILE` never changes `x`, so it never ends. Here's how to give it a way out." It offers a **Show me** with a tiny demo, run in a sandbox with a time limit.
- **Friendly errors:** every compiler error gets "what it means", "why it happens" and a hint to fix it yourself. A **Fix it** button only appears after the hint.
- **First times:** your first event handler, first array, first file read → one short card ("Events: your SUB runs when the user clicks. Try adding a second button.").
- **A tip budget:** at most one unrequested tip every few minutes, "Don't show this again" per kind, and none while you're typing fast or debugging. Never uninvited in Build mode.

## 3. Predict, then Run

An optional **Predict** button next to Run: "What will this print?" You type a guess, it runs, and Studio shows guess vs result. When they differ, the debugger offers to step through, with RAI narrating each variable as it changes. The debugger becomes the "notional machine": you *see* how the computer thinks.

## 4. Your code, the AI's code

- The gutter marks which lines the AI wrote and which you wrote. The mark fades once you edit a line yourself.
- **Make it mine:** a challenge to retype or rewrite an AI-written part from memory, with hints.
- Learning credit (below) counts **only lines you wrote**. This is the new paradigm: the AI helps, but you always know what's yours.

## 5. Teach the AI (protégé mode, WOW)

RAI plays a curious beginner: "I don't get why this `FOR` starts at 1. Can you explain it to me?" You explain in your own words, and RAI asks a follow-up or gently points out a mistake. Explaining makes you learn deeply, and it's fun: you're the teacher.

## 6. A skill map that grows

The concepts (variables, IF, loops, arrays, SUB and FUNCTION, events, forms, files, data, debugging) are a constellation. A star lights up when **you** use a concept correctly in your own code, not when you read about it. Spaced review: "Three days ago you used `STEP -1`. Quick: what does it do?" (one tap; skip any time). Achievements only for real things: "Fixed your own bug", "First program someone else ran".

## 7. Challenges made from your own project

- **Parsons puzzles:** the lines of one of your SUBs, shuffled; drag them into order.
- **Bug hunt:** RAI hides one bug in a *copy* of your project; find it with the debugger.
- **Modify:** "Make the tip calculator round up to the next peso."
- **Learning paths:** tip calculator → quiz game → paint → data app. Each is a RapidR project and a guided lesson (Phase 3D), with RAI as coach.

## Why this works with small local models

Explaining code, asking about it and giving hints is much easier than building a program. A small local model like gemma-4-e4b can do it well, and RapidR feeds it the facts: the compiler's diagnostics, the registry, the debugger's values. **Free, offline, private**: nothing leaves the computer. That matters for schools.

## Safety and legal (decided conservatively)

- **Children:** no accounts and no data collection for learning features. Progress lives **only on the device** (Studio's settings folder) and can be exported as a file. If accounts or class sync ever come, the 6 legal checks run first, with COPPA above all (age gate; under 13 needs verifiable parental consent).
- **Cloud AI providers** have their own age terms (many require 18+). For students, Studio recommends **local models** first, and the teacher, not the child, configures any cloud key.
- Tone: encouraging, never shaming, and it never says "this is easy". Accessible: screen reader, keyboard, and spoken explanations through RSpeech.

## Order of work (after the small-model fixes land)

1. **Pair mode:** a *why* per hunk, plus friendly compiler errors with a hint before the fix. (It uses today's tools; the biggest win for the least work.)
2. **Coach mode** with the hint ladder, and the tip budget with first-time cards.
3. **Infinite-loop catcher** and **Predict, then Run**, with debugger narration.
4. **Your code / AI code** marks, and the skill map with spaced review (local only).
5. **Teach the AI**, Parsons puzzles, bug hunts, learning paths, and the teacher lock.
