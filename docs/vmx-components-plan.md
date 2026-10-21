# R3D, audio, synth, app MCP and studio components: for Visualmusicator X and everyone (plan, 2026-10-21)

Robert (2026-10-21): Visualmusicator X (VMX) is the successor of his realtime, music-reactive visuals app. It will be written **entirely in RapidR**, as a flagship RapidR product. Everything VMX needs becomes a **general-purpose RapidR component** that any RapidR program can use.

- **The brief:** `~/Herd/visualmusicator-x/docs/rapidr/REQUEST-001-components.md`.
- **When:** this work starts **after Preview 4** ([preview4-plan.md](preview4-plan.md)).

## How the two chats work together
1. The VMX chat never edits RapidR. It writes requests (`REQUEST-00N`).
2. Robert brings each request here.
3. This chat plans, builds, verifies and ships it as RapidR components (registry, MCP, manual, conformance, web parity).
4. Robert tells the VMX chat that the components are ready.
5. VMX builds on them and reports what's missing in the next request.

VMX's own shows and scenes stay in VMX's repo. RapidR receives only general components, examples and tests.

## Answers to REQUEST-001 §4
1. **Names.** The `R` prefix is right, and these are new RapidR components (origin rapidr, no Q aliases):
   - the R3D family: `R3DView` (the component) with `R3DScene`, `R3DCamera`, `R3DObject`, `R3DShaderPass`, `R3DRenderTarget`, `R3DPost`;
   - `RAudioAnalyzer`;
   - `RSynth`, which grows from the GM synth that already plays QMIDI (`rapidr-value/src/objects/synth.rs`). GM stays one preset bank;
   - `RMcpServer`;
   - `RNodeGraph`, `RTimeline`, `RPianoRoll`, `RCurveEdit`, `RGradientEdit`, `RKnob`.

   Final names are confirmed when each component enters the registry.
2. **Native or interpreted.** Both:
   - Running in Studio uses the bytecode VM.
   - `rapidr build` compiles the program to native code through Rust, and native builds never embed the VM.
   - On the web, the program runs on the VM compiled to WebAssembly.

   So keep the heavy per-frame work in shaders and compute, and let BASIC orchestrate. That is fast everywhere.
3. **WebGPU on the web, with WebGL2 as the fallback: yes.**
   - R3D presents directly and reads back asynchronously, so it doesn't share RDXScreen's reason for WebGL2.
   - RDXScreen keeps its own path for RapidQ compatibility.
   - Both use wgpu 30, with the same adapter policy.
4. **The tiny profile (M7): yes, in scope, last.** It shares goals with the board work in master-plan M3: size budgets, a minimal runtime, only the components a program uses.

## Our standards, always (Robert, 2026-10-21: "always careful about them")
No wave is done until every point below holds. Speed and licences are never traded for features.
- **Licences first.**
  - Every dependency is permissive and passes `cargo deny`; THIRD_PARTY_NOTICES is regenerated.
  - Every ported algorithm, shader or asset has a known, permissive origin.
  - MPL, or anything unclear, waits for Robert's approval.
- **Speed.** The fps targets in REQUEST-001 are measured, not assumed, on the M4 Max and on a mid-range laptop. The sound-to-kick latency stays ≤ 20 ms.
- **Easy to use.**
  - A first program runs in a few lines.
  - Defaults look good out of the box.
  - Shader errors come in plain words, with line and column.
- **Accessible.**
  - Every new component is in the accessibility tree (AccessKit) and works with the keyboard.
  - Editors such as RTimeline and RNodeGraph have keyboard equivalents for every drag.
  - Visuals offer a reduced-flash mode (photosensitivity: no more than 3 flashes a second, the WCAG 2.3.1 limit) as a documented option. Shows for big audiences need it.
- **Secure.**
  - WGSL from AI or users is validated (naga) before it reaches the GPU.
  - RMcpServer follows the IDE's MCP security.
  - Audio input asks for the system's permission and is never recorded without being shown.
- **Parity, the RapidR way.** Desktop and web, interpreted and native, all behave the same, and the conformance cases prove it.
- **Seen and walked.** There are captures and a demo for each wave, and it is walked like a user.

## Decided conservatively (legal zero risk)
- **Audio decoding is already permissive:**
  - WAV (hound), Ogg Vorbis (lewton), FLAC (claxon) and MP3 (nanomp3);
  - symphonia (MPL-2.0) is not needed.
- **M4A/AAC** is still covered by patent pools. RapidR does not ship an AAC decoder. M4A plays through the system's own decoder where one exists: macOS AudioToolbox, Windows Media Foundation, and the browser's `decodeAudioData`. On Linux it is reported as unsupported.
- **Shader origins:**
  - Every function in the WGSL include library (noise, SDF, palettes) must be our own, or come from a permissive source with attribution, such as Ashima/stegu's MIT simplex noise.
  - **Nothing comes from Shadertoy unless its author's licence is permissive.** Shadertoy's default is CC BY-NC-SA, which is non-commercial.
  - VMX's `glsl.js` is checked function by function before it is translated.
- **Patterns:** the pattern notation is our own design. No Strudel/Tidal code (AGPL/GPL).
- **Mesh decoding:**
  - meshoptimizer's format and code are MIT;
  - DEFLATE through miniz_oxide (MIT/Apache/Zlib).
- **Protected web shows (AES-GCM):** they protect against casual copying. They are not real DRM, because the key ships with the page. The manual says so plainly.
- **`RMcpServer`** uses exactly the IDE's MCP security:
  - loopback only;
  - a token;
  - Origin/Host checks;
  - permission tiers;
  - an audit log.

  It goes through the Preview 4 security audit's checklist.

## Waves (each one a release with a WOW video)
| Wave | REQUEST-001 milestones | Why first | Models |
|---|---|---|---|
| W1 | **M1 R3D core** (R3DView, shader passes, HDR targets, post stack, async Capture) + **M3 RAudioAnalyzer** (bands, onsets, BPM PLL, drops, low-latency input, loopback devices) | "Visuals that react" is the first VMX demo and a huge RapidR WOW | Opus L × 2; a Fable design review of the R3D backend trait only with Robert's OK |
| W2 | **M2** scene graph, geometry, materials, instancing, GPU compute, reflections, generated textures | Ports the 3D and particle scenes | Opus L |
| W3 | **M4 RSynth** (patch voices, FX, sample-accurate sequencer, our own notation, AudioWorklet, offline render) | Self-scored shows; music made in BASIC | Opus L |
| W4 | **M5 RMcpServer** (shares the IDE's MCP code) + **M6** studio components (RNodeGraph, RTimeline, RPianoRoll, editors, multi-window, theme tokens) | The VMX editor; the docking builds on RDockManager | Opus L + Sonnet M |
| W5 | **M7** single-file export, the tiny profile, direct backends | Demoscene sizes | Opus L |

Each wave:
- passes the acceptance demo in REQUEST-001;
- runs on macOS, Windows, Linux and the web (interpreted and native);
- gets registry and MCP entries, a manual chapter with screenshots, and conformance and web-parity cases.

Where the waves sit in the master plan (against M1 RAI 2, M2 layout and M3 boards) is decided with Robert when Preview 4 ships.
