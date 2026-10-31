# Demo mode and the video pipeline

RapidR Studio's narrated tutorial videos are made from **scripts**: a JSON
file says what is said and what is done, and two parts turn it into a
video:

1. **Demo mode** (`crates/rapidr-ui-app/src/demo/`, the kernel's overlay
   `crates/rapidr-ui-kernel/src/demo_overlay.rs`, Studio's side
   `ide/demo.inc`): the script played inside Studio's own windows,
   headless, and recorded frame by frame at 30 fps. A pointer glides to a
   control found **by name**, clicks with a soft ripple, drags, scrolls and
   types letter by letter, all through the kernel's own input routing; a
   highlight ring or spotlight, a caption strip and title cards are drawn
   over the windows.
2. **The pipeline** (`tools/video/`, Node, run on the author's Mac): the
   narration (one clip a sentence from a local text-to-speech model),
   then a take (Studio in demo mode, the lines timed by their clips), then
   the video (ffmpeg: frames + voice + optional music bed, captions burned
   in and as an `.srt`, a poster frame).

Demo mode is **not public** in Preview 5: it is a development tool, gated
as a test hook (below). Its parts are built to become the public **RTutor**
(guided tours in users' apps) and **RCaptions** components in Preview 6
(see *Toward RTutor and RCaptions*).

## Quick start

```sh
# a release build that runs Studio (or any build of rapidr-cli)
CARGO_TARGET_DIR=... cargo build --release -p rapidr-cli
export RAPIDR=$CARGO_TARGET_DIR/release/rapidr

# a silent dry take (no voice: lines timed from their words)
node tools/video/video.mjs tools/video/scripts/first-app.json --lang en --silent

# the narrated videos, English and Spanish (+ a WebM for rapidr.dev)
node tools/video/video.mjs tools/video/scripts/first-app.json --lang en,es --webm

# the video again from the last take (after a change to mux.mjs, say)
node tools/video/video.mjs tools/video/scripts/first-app.json --lang en --mux-only
```

Output, all in the main checkout's git-ignored `dist/video/` (audio and
video never go in Git):

| File | What |
|---|---|
| `<id>-<lang>.mp4` | the video: 1920 x 1080, 30 fps, H.264 (CRF 20) + AAC 160k, `+faststart`; the upload master for video sites |
| `<id>-<lang>.webm` | with `--webm`: the same in VP9 + Opus (royalty-free), for rapidr.dev's own pages |
| `<id>-<lang>.srt` | the captions (also burned into the frames) |
| `<id>-<lang>.jpg` | the poster: the frame at the script's `poster` mark |
| `<id>-<lang>.description.txt` | title, description, the AI-voice disclosure, made-with, support line |
| `<id>-<lang>.timeline.json` | when each line was said (clip file names only) |
| `takes/`, `clips/`, `voices/` | working files: the frames, the cached narration clips, the narrators' copies |

Options: `--lang en,es` (default: every language the title has),
`--silent`, `--mux-only`, `--music FILE` (a bed ducked 12 dB under the
voice; none until Robert chooses a track licensed for commercial use, and
it is credited in the description), `--webm`, `--out DIR`.

## Requirements

- **Demo mode's gate (SEC-24, docs/security/test-mode.md).** Demo mode is
  input automation, so `RAPIDR_DEMO`, `RAPIDR_DEMO_OUT` and
  `RAPIDR_DEMO_LANG` are test hooks: a release build reads them only with a
  valid test-mode token (the pipeline passes `~/.rapidr/test-mode.token`
  when it is there); a development build (debug, or `--features
  dev-hooks`) needs none. Without either, the variables are ignored and one
  line on stderr says so; the pipeline then reports that the take never
  finished. `tests/security/test_mode_release.mjs` checks it.
- **ffmpeg** (`/opt/homebrew/bin/ffmpeg`, or `FFMPEG`/`FFPROBE`): a tool on
  the Mac, only run, never linked or shipped.
- **The narration server**: the local Qwen3-TTS model served at
  `http://127.0.0.1:11234` by MLX-Serve, which Robert starts himself. The
  pipeline never starts or stops it; if it isn't running, or answers 503
  (short of memory), the pipeline says so and stops (clips made so far are
  kept). It only talks to this computer: an `MLX_SERVE_URL` that isn't
  127.0.0.1, localhost or ::1 is refused. Its key is read at run time from
  the macOS Keychain (`security find-generic-password -a "$USER" -s
  mlx-serve-key -w`), kept in memory, never printed or written. At the end
  the model is unloaded (`POST /v1/unload-model`).

## A take: what runs, and what never does

`tools/video/lib/record.mjs` starts `rapidr run ide/studio.rr --fresh
--maximized` with `--do demo:poll` on a **profile of its own**: a fresh
temporary home (links to the checkout's `ide/` and `docs/`, a copy of
`examples/`), its own registry, Documents, Studio data, AI chats, audit
log, prints and build cache, AI keys in memory only, and only the
environment it needs (an allowlist: `PATH`, `TMPDIR`, the locale; `HOME` is
the profile). A saved version is by "Ana" (`RAPIDR_VERSIONS_NAME`), with no
Git identity of this machine. Projects go to `./projects` in that home, so
no path of this computer shows. Still watch each video whole before
posting: the clock is real (a version's time shows in the timeline).

The host is **headless**: no window shows on the screen, nothing of the
desktop, Dock or other apps is in a frame, and the real mouse and keyboard
are never touched. Each frame is composed from the kernel's own drawing of
Studio's windows (framed, the theme's shadow), the program Studio runs as
layers over them, the overlay on top.

**Time is the video's.** Each turn of the window loop is one frame (1/30 s
on the kernel's clock, `tick::set_test_now`), never ahead of real time: a
slow frame makes the recording slower, never the video choppy. Identical
frames are written once (`frames/frames.json` lists every frame's time and
file). A narration line lasts its clip's length (else an estimate from its
words).

The program Studio runs (its own process) plays along through files in
`<out>/app` (`demo/app.rs`): its windows come over as layers, and the
player's requests (`find`, `click`, `type`, `key` on `app:Button1`) are
played through its kernel and answered. Studio's own commands (a step's
`do`) go through `<out>/inbox` to `ide/demo.inc`, one a macro tick.

## The script format

JSON, with `//` comment lines. Any text can be one language or several:
`{"en": "…", "es": "…"}` (else English, else the first). Narration lines
can use `{shown|spoken}`: the caption shows the first part, the voice says
the second (`{RAI|Rye}`).

```jsonc
{
  "id": "first-app",                       // the files' names
  "title": { "en": "Your first app", "es": "Tu primera app" },
  "description": { "en": "…", "es": "…" }, // for the .description.txt
  "screen": [1440, 810], "scale": 1.3333333, // logical frame x scale = 1920 x 1080
  "fps": 30,
  "window": "studio",                      // the frame's base window
  "pointer": { "speed": 0.9, "size": 1.3 },
  "typing": 1,                             // 1: about 11 characters a second
  "captions": { "size": 24 },              // caption text height (0: none drawn)
  "studio": { "theme": "rapidr-light", "setup": "…", "open": "examples/…", "user": "Ana" },
  "steps": [ … ]
}
```

A **step** is narration and the actions played meanwhile; it ends when
both are done, then `gap` seconds (0.45 by default).

```jsonc
{ "say": [ "One sentence a line.",
           { "text": "And this one waits for a cue.", "cue": "clicked", "captions": true } ],
  "do": [ { "click": "app:Button1", "cue": "clicked", "pause": 1.4 } ],
  "card": { "title": "…", "lines": ["…"], "small": ["credits"], "background": "x.svg",
            "logo": "logo.svg", "logoHeight": 64, "lightText": true },
  "gap": 0.8 }
```

**Actions** (each an object with one verb; `"do"` may also be a single
Studio command, and a step may hold one action directly):

| Verb | What it does |
|---|---|
| `move` / `hover` | the pointer glides to the target and rests there |
| `click` / `press`, `double`, `right` | glide, then a click (a ripple), a double-click, a right-click |
| `drag` + `to` | press on the target, glide to `to`, let go (Toolbox to the designer) |
| `type` (+ `into`) | typed letter by letter, a natural, repeatable rhythm |
| `key` (+ `into`) | keys: `Enter`, `Ctrl+A`, `Tab Tab`, `Escape` |
| `scroll` + `by` | wheel notches over the target |
| `wait` | seconds |
| `waitFor` (+ `timeout`) | `"window:NAME"`, `"gone:NAME"`, `"shows:TARGET"`, `{"app": true}` (the program Studio runs shows a window; `false`: none any more), `{"prop": "comp.Prop", "is": …}` / `"has"` |
| `highlight` / `spotlight` (+ `for`) | a ring around the target (a spotlight dims everything else); `null` clears it |
| `do` | Studio's own commands, comma-separated (`file.saveAll`, `run.stop`, `lang:es`, `newproject:gui|folder|Name`) |
| `open` | a file opened in Studio |
| `mark` | a named moment (`poster`: the poster frame) |
| `pointer` | `"show"` or `"hide"` |

Every action can also have `cue` (fired when the pointer arrives, or when
the action starts: the line waiting for that cue starts then), `afterLine`
(wait until the step's line n has been said), `pause` (seconds after it,
0.25 by default) and `glide` (seconds the pointer takes, else from the
distance).

**Targets** are found the way RAppHands finds them: a component's name
(`NewOk`, `designdoc(0)`, `TplCard(0)`), `caption:Run`, `label:Name`, an
item inside a component by what it shows (`Toolbox > RButton`, `Inspector >
Caption`, `studio > File`: the accessibility tree's names), and `app:` for
the program Studio runs (`app:Button1`). An object adds where in it
(`{"target": "Inspector > Caption", "x": 0.78}`, 0 to 1 across and down) or
names a point (`{"window": "studio", "at": [x, y]}`).

When a target isn't found, a `waitFor` times out or Studio never runs a
`do`, demo mode stops with its reason in `<out>/error.txt` (the step and the
video's time) and the pipeline says it.

## Narration

`tools/video/lib/narration.mjs`: one clip a sentence, `POST
/v1/audio/speech` with model `mlx-community/Qwen3-TTS-12Hz-1.7B-Base-bf16`,
the line's spoken text and the language's narrator clip as `ref_audio`
(base64), so each language keeps one narrator. Clips are cached by
(model, narrator, tempo, words): a script recorded again only speaks its
new lines. Each clip is resampled to 44.1 kHz mono; Spanish is played 1.2
times as fast with the pitch kept (`atempo=1.2`, Robert's choice). The mix
is loudness-normalised (EBU R128, I=-16, TP=-1.5, LRA=11).

The narrators (`tools/video/lib/config.mjs` `VOICES`; the clips in the
main checkout's git-ignored `dist/voices/narrators/`, back them up before
cleaning `dist/`):

| Language | Clip | Provenance |
|---|---|---|
| English | `en-79.wav` (voice-79) | made up by the model with no reference recording; picked by Robert from 112 candidates (2026-10-27) |
| Spanish | `es-13.wav`, at 1.2x | made up by the model with no reference recording; Robert's pick (2026-10-30) |
| Portuguese (Brazil) | `pt-25.wav` | made up by the model with no reference recording; Robert's pick (2026-10-30) |
| Chinese | `zh-21.wav` | made up by the model with no reference recording; Robert's pick (2026-10-30); still needs a native speaker's check |

*Narration notice.* The voice is made with Qwen3-TTS
(`Qwen/Qwen3-TTS-12Hz-1.7B-Base`, by the Qwen team; MLX conversion
`mlx-community/Qwen3-TTS-12Hz-1.7B-Base-bf16`), licensed under the Apache
License 2.0 (https://www.apache.org/licenses/LICENSE-2.0), run on the
author's Mac by MLX-Serve (MIT). Neither is part of RapidR: RapidR does not
ship, link or download them; `tools/video` only calls the local server at
127.0.0.1. The audio is the model's output, not a copy of the model:
Apache-2.0 asks nothing of it, and the credit in the videos is courtesy and
transparency. "Qwen" belongs to its owners and is used only to say which
model made the voice; RapidR is not affiliated with or endorsed by them.
The narrators' reference clips are voices the model made up with no
reference recording. (Licences as given by the model cards and the
server's repository; Robert re-checks them once and dates it here.)

## Credits on every video

(docs/legal/review-2026-10-30-demo-mode.md)

- The **title card** says "AI-generated narration" / "Narración generada
  con IA".
- The **end card**: "Narrated by an AI-generated voice (Qwen3-TTS)" /
  "Narrado por una voz generada con IA (Qwen3-TTS)", then "Support RapidR:
  GitHub Sponsors · Buy Me a Coffee" / "Apoya RapidR: …".
- The **description** (`CREDITS` in `config.mjs`, per language): the
  disclosure right under the description, then "Made with RapidR Studio:
  https://rapidr.dev", the music's credit when there is a bed, and the
  support line with both links.
- The file's tags: the title, the disclosure as the comment, "Copyright
  2026 Ruta Internet SRL".

At each upload: answer the video site's "altered or synthetic content"
question **Yes** (realistic AI speech); set the audience honestly (a
beginner tutorial for any age is not "made for kids" unless it is directed
at children); on rapidr.dev self-host the WebM or link to it (no embedded
third-party player); on mailing lists, a link rather than an attachment.

## The sample and the launch videos

- `tools/video/scripts/first-app.json`: **Your first app in RapidR
  Studio** (about a minute, English and Spanish): a new project, a button
  dragged from the Toolbox, its caption set in Properties, a double-click
  to its OnClick, one line typed, Run, the running app's button clicked,
  Stop, and a version saved.
- `tools/video/scripts/launch/`: the six Preview 5 launch videos as
  **skeletons** for review (steps plus English and Spanish narration; not
  recorded yet; `// TODO` marks a target to check when recorded):
  `1-pair-with-ai.json`, `2-coach-mode.json`, `3-versions.json`,
  `4-spanish-and-school-lock.json`, `5-new-components.json`,
  `6-8bit-showcase.json`. Only RapidR's own programs and examples appear.

## Tests

- `cargo test -p rapidr-ui-app --lib demo`: the script parser (languages,
  `{shown|spoken}`, targets, conditions, errors), the easing and glide,
  typing rhythm, the recorder's helpers; `cargo test -p rapidr-ui-kernel
  --lib demo_overlay`: the overlay's drawing; `cargo test -p
  rapidr-testmode`: the demo's variables are hooks.
- `node tests/studio_flows.mjs demo-mode-tiny` (desktop): a tiny script
  played headless: a new project, a button dragged from the Toolbox, its
  caption typed; the saved project has exactly one button with that
  caption, frames and a finished timeline were written, and Studio ended
  by itself.
- `RAPIDR_RELEASE=<release rapidr> node tests/security/test_mode_release.mjs`:
  in a release build without a valid token the demo's variables are
  ignored (ENVIRON$ empty, nothing written; `--gui` adds a windowed run).

## Toward RTutor and RCaptions (Preview 6)

The pieces map onto two components; the script format is their data:

- **RTutor** (a guided tour in a user's app): `Target` (a control by name,
  caption, label or item: what RAppHands already finds), the pointer's
  motion (`demo::motion`: minimum-jerk glide along a slight arc, a
  repeatable typing rhythm), the input played through the kernel
  (`demo::input`), the highlight ring and spotlight, `waitFor` conditions,
  steps with cues. Methods would be `Start`, `Pause`, `Stop`, `Next`,
  events `OnStep`, `OnDone`.
- **RCaptions** (timed captions over any window): the caption strip
  (`demo_overlay::Caption`: a card with the accent mark, Inter, wrapped to
  a few lines), cues, `{shown|spoken}`, `.srt` in and out.

Before they go public (the legal review's checklist): only the app's own
windows, never the system's mouse or keyboard; a pointer you can always see
and a visible "tour playing" sign, reduced motion honoured; the user stays
in charge (Pause, Stop, Esc, their own click ends it; never auto-start); an
in-process API with an allowlist of actions, not the test hook's files;
captions always, keyboard and screen-reader friendly, no flashing; nothing
recorded or sent; a name search, a legal review, a manual page, an example
and the AI facet.
