# RapidR master plan: the road to 1.0 and beyond (2026-10-19)

How we get from Preview 3 (2.120, the RAI release) to RapidR 1.0, and what comes after. It's planned so that each piece is a well-briefed lane for the right model, in the right order, within Robert's weekly budget.

- **ROADMAP.md** is the detailed checklist: every box, ticked as work lands.
- **This file** is the *sequence*: what comes first, why, who does it, and what "done" means.
- **The plans in docs/** are the designs, linked from each milestone.

## 1. Principles (unchanged, and they decide every trade-off)

- **Test like a user:** done means used end to end, with real clicks, on screen, on every OS and on the web. A green suite alone isn't done.
- **WOW, and seen:** every release shows something that makes people smile, with screenshots and video.
- **Legal zero risk:**
  - permissive dependencies only;
  - nothing copied;
  - the conservative choice when in doubt;
  - the six legal traps checked before any accounts, uploads, analytics or payments.
- **RapidR first:** RapidQ is a small compatibility footnote, and RapidQ-exact behaviour is never broken (extensions are additive).
- **Native is native, one implementation, no fallbacks** (except the deliberate drawn fallback of `$UI native`).
- **A manual entry for every user-visible change,** with reproducible screenshots.
- **Close gaps before new work.**

## 2. Who does what: the models

| Model | Its jobs | Typical size |
|---|---|---|
| **Fable** | The hardest calls: architecture reviews of new foundations (the native UI contract, the on-board VM), **security audits**, deep bug hunts that defeated Opus. **Robert approves each Fable lane before it starts** (it's the most expensive). | rare, high value |
| **Opus** | Complex implementation: compiler, runtimes, UI kernel, Studio features, the RAI loop, new targets (ESP32, micro:bit), native backends; plus the coordinator (briefs, what-if, merges, releases). | L / XL lanes |
| **Sonnet** | Well-defined features and fixes, examples and recipes, docs sweeps, translations, tests, small UI polish. | S / M lanes |
| **Haiku** | Lookups and inventories, licence checks, link checks, research summaries, mechanical edits. | XS |

**The rules of the house:**
- every lane gets the full brief (~/.claude/rapidr-brief-template.md);
- visual lanes go one at a time, under the screen lock;
- runs are watched by a watchdog;
- the coordinator merges, runs the regression and pushes;
- hardware is flashed only with Robert's OK for that board.

**Sizes and budget** (weekly limit; keep ~20% for Robert's other projects unless he says otherwise):

| Size | What | Rough cost |
|---|---|---|
| XS | Haiku lookup | < 0.2% |
| S | Sonnet fix | ~0.5% |
| M | Sonnet feature or small Opus fix | ~1% |
| L | Opus feature | ~2–3% |
| XL | Opus foundation | ~4–6%, split into L steps |

**A week's rhythm** (the limit resets Thursday 07:00):
- **Thu–Sat:** the heavy Opus lanes.
- **Weekend:** Robert's live walks, the release, the posts.
- **Mon–Wed:** Sonnet and Haiku polish, the backlog, and planning the next week's briefs.

## 3. The road to 1.0, milestone by milestone

Each milestone ends with a **Preview release** and a WOW post: a video or GIF, in EN/ES/PT.

### M0: Clean-up (right after the reset, small)
Plans: preview2-fix-queue (memory), ROADMAP.
| Item | Model, size |
|---|---|
| Constants built into `.rr` programs (no `$INCLUDE "RAPIDQ.INC"`); recipes, hints and instructions updated | Opus M |
| `OnMessage` accepted on RForm (same as WndProc) | Sonnet S |
| Playing sound must never ask for the microphone (macOS) | Opus S |
| RapidR-first leftovers: generated reference headers, Studio hint strings, internal docs, two install screenshots | Sonnet S |
| Studio's macOS accessibility window (the computer-use tool found no AX window): check VoiceOver | Opus S |
| Web parity notes for menus, themes, focus_order | Sonnet S |
| Decision: Apple Developer ID signing and notarization ($99/yr, Robert) and Windows signing | Robert |

### M1: RAI 2, the assistant that sees and tests
Plans: [ai-ux-plan.md](ai-ux-plan.md), [small-models-plan.md](small-models-plan.md).
| Item | Model, size |
|---|---|
| A slim, native banner; buttons in Studio's style | Sonnet M |
| The docked, beautiful review: split/unified view, ribbons, word-level highlights, Keep/Undo pills | Opus L |
| Tracked changes in the editor buffers and checkpoints | Opus L |
| `run_program` (scripted clicks, results) and screenshots **with permission**; the permission-mode dropdown | Opus L (design review: Fable, if Robert agrees) |
| `search_manual`; a game-loop recipe; layout and handler-skeleton tools; spec-first for small models | Sonnet M + Opus M |
| **RChat** component | Opus L |

### M2: Layout, binding and the native look
Plans: ROADMAP Phase 3B, [native-ui-plan.md](native-ui-plan.md) (v1 part).
| Item | Model, size |
|---|---|
| `RStackPanel`, a grid panel | Opus M |
| `BIND()`: properties that follow variables | Opus L |
| `$THEME native`: drawn Windows 11, macOS and GNOME looks, light and dark | Opus L + Sonnet M (theme tokens, screenshots) |

### M3: RapidR on boards: ESP32, micro:bit, M5Stack (**v1 scope**, Robert 2026-10-19)
Plan: ROADMAP Phase 3C (to be expanded into docs/hardware-plan.md as M3's first lane).
- **Two ways in, both in v1:**
  1. **Link mode:** a program on the computer drives the board over USB, using small RapidR firmware we provide. It's the fastest way to motors, sensors and screens, and great for classrooms.
  2. **On-board mode:** RapidR BASIC running *on* the board (`no_std` runtime; ESP32 via esp-hal, micro:bit v2 via embassy-nrf / microbit-v2, all MIT/Apache).
- **Components** (R names, our own docs):
  - **Pins:** `RPin` (digital), `RAnalogIn` (ADC with calibration and smoothing), `RPWM`, `RI2C` / `RSPI` / `RUART`.
  - **Motion:** `RServo`, `RMotor` (DC through an H-bridge), `RStepper`.
  - **Power:** `RRelay` (AC/DC loads, see safety below).
  - **Sensors:** temperature and humidity, distance, IMU (micro:bit's built-in too), light.
  - **Sound:** buzzer and speaker.
  - **Displays:** OLED (SSD1306), TFT (ST7789), the micro:bit LED matrix, the M5Stack screen.
  - **Radio:** Wi-Fi and BLE on ESP32, micro:bit radio.
  - **Camera:** ESP32-CAM.
- **Studio:**
  - a board picker;
  - Run = build + flash (espflash; micro:bit by copying the .hex);
  - a serial monitor;
  - a **live board view** (pins and values while running);
  - a micro:bit simulator panel;
  - examples and lessons.
- **Safety, decided conservatively:**
  - examples and lessons are low-voltage only;
  - mains (AC) only through certified relay modules, with clear warnings in the manual and in Studio;
  - nothing that tells kids to touch mains.
- **Legal:** micro:bit, ESP32 and M5Stack names only to describe compatibility, no logos.
- **Robert's hardware:** micro:bit, ESP32s (including the CAM), M5Stack and sensors. Each board is flashed only with his OK.

| Lane | Model, size |
|---|---|
| Hardware design doc and spike (blink, display "Hello", Wi-Fi on Robert's ESP32; binary size, build time) | Opus M (on-board VM design review: Fable, if Robert agrees) |
| Link firmware + components (link mode), ESP32 and micro:bit | Opus L ×2 |
| On-board runtime: ESP32 | Opus XL (as L steps) |
| On-board runtime: micro:bit | Opus XL (as L steps) |
| Displays and M5Stack GUI | Opus L |
| Studio board panel, simulator, examples, lessons | Opus L + Sonnet M |

### M4: Learn with RAI, speech and lessons
Plans: [learn-with-rai-plan.md](learn-with-rai-plan.md), [speech-and-tutorial-plan.md](speech-and-tutorial-plan.md).
| Item | Model, size |
|---|---|
| Pair mode (a *why* per change); friendly errors; the tip budget | Sonnet M + Opus M |
| Coach and Quiz modes, the infinite-loop catcher, Predict-then-Run | Opus L |
| `RSpeech`; the guided-lesson format and the first lessons (desktop and boards) | Opus L + Sonnet M |
| Teach-the-AI; the local skill map (nothing collected) | Opus L |

### M5: Four languages
Plan: ROADMAP "Languages".
| Item | Model, size |
|---|---|
| Studio's UI, compiler messages and the manual in Spanish, Portuguese and Chinese; the website | Sonnet L (translations) + Opus M (the i18n plumbing) |

### M6: Data, the last mile
Plan: [data-science-plan.md](data-science-plan.md).
| Item | Model, size |
|---|---|
| Excel, Parquet and PostgreSQL as linked components; any remaining speed and ease gaps | Opus L |

### M7: Hardening for 1.0
| Item | Model, size |
|---|---|
| **Security audit** (desktop, web, RAI, the board link) | **Fable** (Robert approves) → fixes on Opus |
| Accessibility audit: VoiceOver, Narrator, NVDA, Orca, TalkBack, on real machines | Opus L |
| USE-AUDIT: every feature walked like a user on every OS and the web; performance pass | Opus L ×2 |
| Signing and notarization; installers; store readiness notes | Opus M |
| The manual complete with screenshots; release notes; the 1.0 video | Sonnet M + coordinator |
| The six legal traps, if rapidr.dev adds accounts, newsletters or uploads | 6× Sonnet (the CLAUDE.md rule) |

**→ RapidR 1.0.**

## 4. After 1.0

- **Phase 8: truly native UI** (`$UI native`), plan: [native-ui-plan.md](native-ui-plan.md).
  - The contract, then AppKit, then Win32 → WinUI 3, then UIKit (iOS/iPadOS), then Android.
  - The web as an opt-in DOM backend.
  - Published as a standalone MIT crate.
  - **Linux: decided.** It keeps our drawn kernel with the GNOME look; no GTK (LGPL), consistent with the zero-risk rule. Revisit only if many users ask, and then only system-linked, never bundled, with Robert's approval.
- **Phase 7: mobile apps,** which merges into Phase 8 (UIKit, Android) plus device components (camera, location, sensors, notifications).
- **RapidR UI as a standalone crate** (the drawn kernel, accessible, web and desktop).
- **RapidP: Python export** (`.rr` → `.py`).
- **Calls into native libraries** on macOS and Linux (like DLL calls on Windows), and `$UNSAFEMEM`.

## 5. How a milestone runs

1. **Plan:** the coordinator expands the milestone's plan doc. A what-if pass covers risks, legal, budget and the screen.
2. **Brief:** one full brief per lane, with the model picked by size. Fable only with Robert's OK.
3. **Execute:** the lanes run in worktrees. Visual ones run one at a time, with watchdogs.
4. **Merge and verify:** the full regression, plus a walk like a user. Robert's live walk for anything he'll show.
5. **Release:** the release chain, Robert signs, publish.
6. **Tell the world:** a video or GIF, and posts in EN/ES/PT on YouTube, X, Discussions and groups.io, RapidR-first, ending with the support line.
7. **Remember:** memory and ROADMAP updated, leftovers queued for the next milestone.
