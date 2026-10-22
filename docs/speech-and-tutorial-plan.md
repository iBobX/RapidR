# RSpeech and Studio's guided lessons: plan (2026-10-16, after Preview 2)

Research on 2026-10-16, with licences read from the source files. Robert asked for speech for accessibility and an in-Studio tutorial in the spirit of guided tours and interactive playgrounds.

## RSpeech: text to speech for every program

### Design: system voices by default, nothing GPL

**API**
- `Voices`: id, name, language (BCP 47), gender if known, quality (default / enhanced / premium / neural), local or not.
- Settings: `Voice`, `Rate`, `Pitch`, `Volume`.
- Methods: `Speak`, `Stop`, `Pause`, `Resume`, `IsSpeaking`.
- Events: `OnStart`, `OnWord(index, length)` (for highlighting the text being read), `OnEnd`, `OnError`.
- Capability flags such as `CanWordBoundary`, so programs know what a platform can do.

**Backends** (our own thin ones; permissive crates only)

| Platform | API | Crate | Word events |
|---|---|---|---|
| macOS | AVSpeechSynthesizer | `objc2-avf-audio` | yes |
| Windows | WinRT `Windows.Media.SpeechSynthesis` | `windows` | through metadata tracks; to verify on ARM64 |
| Linux | speech-dispatcher's socket protocol | `ssip-client-async` (MIT/Apache) | SSML marks per word |
| Web | Web Speech `speechSynthesis` | `web-sys` | Chrome's network voices give none, so prefer local voices; split long text by sentence |

On Linux, never link `libspeechd`, which is LGPL. Don't depend on the `tts` crate: it has no word events and its last release is from 2024. Use it only as MIT reference code.

**Optional neural voices** (a download, off by default, self-hosted)
- **Kokoro-82M:** code and weights are Apache-2.0. Run it through `ort`, which is MIT.
- **Languages:**
  - **English:** excellent.
  - **Mandarin:** labelled "basic"; its maker rates it D. It uses a pure-Rust G2P (`jieba-rs`, `pinyin`).
  - **Spanish and Portuguese:** system voices until we write our own non-GPL pronunciation rules and have listened to the result. Kokoro's reference uses espeak-ng, which is GPL-3.
- **Attribution:** add the CC BY credits to THIRD_PARTY_NOTICES.
- **Open question:** part of Kokoro's training audio is synthetic output from closed TTS services. Robert decides whether to offer the download.

### Excluded (legal)

| Excluded | Why |
|---|---|
| Piper (current repo), piper-rs, espeak-ng, espeak-rs | GPL-3 |
| sherpa-onnx TTS builds | link an espeak-ng fork |
| Coqui XTTS (CPML), F5-TTS, Fish Speech, Spark-TTS, Voxtral | non-commercial |
| MARS5 | AGPL |
| Supertonic-3 (OpenRAIL-M) | restrictions flow down to users |
| Piper voices `en_US ryan`, `zh_CN xiao_ya` | non-commercial |
| Voice cloning | consent and likeness law not researched |
| Saving system voices to WAV | the vendors' terms are not checked yet |

### Effort (rough)
- RSpeech on 4 hosts, with Studio integration, the manual and real-voice checks on each OS: 2–3 weeks.
- The Kokoro pack for English and Mandarin: 1–2 weeks.
- Our own Spanish/Portuguese pronunciation rules: 2–4 weeks, uncertain.

## Guided lessons in Studio

### What we learn from others
- **Guided tours in IDEs:** they can't be resumed once stopped.
- **Walkthroughs in editors:** steps complete on events.
- **Block-coding sites:** lessons are written in Markdown.
- **Playground apps:** instant results while typing, challenges.
- **Office assistants of the 1990s:** failed by interrupting people, uninvited. So we never start a lesson on our own.

### The lesson format
One Markdown file per lesson: front matter plus one block per step. It's easy to translate, review, and write by hand or with the AI.

Each step has:
- `say`: the text, always shown as a caption; spoken with RSpeech only if the user turns it on.
- `target`: a control's role and accessible name, so the spotlight and the screen reader point at the same thing.
- `demo`: Studio's named commands, the same ones `--do` runs.
- `check`: a condition on the project, e.g. a component exists, a property equals a value, the source contains text, the program printed something, the debugger stopped at a line.
- `hint`.

### Three modes per step
- **Read:** text only.
- **Watch:** the demo plays with a ghost cursor. The user can pause it, skip it or take over.
- **Try:** the user does it and the check turns green. "Show me" plays the demo.

The demo runs the real commands, so keyboard-only and screen-reader users get the same lesson.

### Accessibility (WCAG 2.2)
- Never auto-start. Lessons are reached from Welcome or Help > Learn.
- Captions always. No speech on top of a running screen reader.
- Pause, stop, replay, previous and next at any time, and the lesson resumes later.
- Esc exits, with no focus trap. The spotlight never covers the focused control.
- No flashing. Reduce-motion turns the cursor movement off.
- Short sentences.
- No streaks or loss mechanics, and no animated mascot in v1.
- A first-run choice of mouse, keyboard only or screen reader.
- "Skip to practice" for power users.

### Every lesson is a test
The lessons replay headlessly in the regression suites, so a UI change that breaks a lesson fails the build.

### First lessons
1. Welcome tour.
2. Hello, button.
3. Input and variables.
4. Find a bug: breakpoints and stepping.
5. Components and properties.
6. Your Studio: themes, high contrast, zoom, shortcuts.
7. The AI assistant.
8. Data in 10 minutes: CSV → RDataFrame → chart.

Later lessons: micro:bit (ROADMAP Phase 3C), and a screen-reader / keyboard-only workflow.

### Effort (rough)
- The engine: 3–4 weeks.
- Each lesson, with captures, a manual page and 4 languages: 1–3 days.
- The engine plus 6 lessons: about 5–6 weeks.

## Not yet verified
- macOS speech callbacks from a non-bundled CLI process.
- WinRT speech on Windows ARM64 and its word-boundary option.
- CPU speeds for Kokoro.
- Whether sherpa-onnx can be built without espeak-ng.
