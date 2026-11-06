# RMIDIPort: design

MIDI input and output for a program, with System Exclusive (SysEx) messages,
on every system: Windows, macOS, Linux and the browser. One component,
`RMIDIPort` (RapidR's own, non-visual; no RapidQ name), one shared model
(`rapidr_value::objects::midiport`) and, per runtime, only the device: the
desktop's through midir, the browser's through Web MIDI.

Why: today a program that wants MIDI *input* or SysEx has to call Windows'
winmm itself (`examples/windows/midi_in.rr`, `midi_sysex_out.rr`): headers,
pointers, buffers the driver fills on its own thread, `midiInAddBuffer`
again after each one. That is Windows only, easy to get wrong, and the
headers' layout differs between 32- and 64-bit programs. RMIDIPort does all
of it inside RapidR. RMIDI (playing a MIDI file) stays as it is.

## 1. What people asked for, and what they trip over

From a quick look at how MIDI libraries and the people who use them work
(private notes in the lane's report; no product named here):

- **Open by name, not only by number.** Numbers change when a device is
  plugged in or out; names don't. Duplicate names (two identical keyboards)
  must still be told apart.
- **SysEx that just works.** The usual traps: a SysEx longer than the
  driver's buffer arrives in pieces; real-time bytes (clock, active sensing)
  may arrive in the middle of a SysEx; a SysEx sent while another is still
  going out is cut. A program wants one whole `F0 … F7` message.
- **No flood.** A keyboard or a sequencer sends clock (`F8`, 24 a beat) and
  active sensing (`FE`, 3 a second) all the time: a monitor full of them is
  useless, so they are ignored unless asked for.
- **Running status** (a status byte left out between messages of the same
  kind) must be understood.
- **Hot-plug**: a list that knows when a device came or went.
- **Callbacks on the driver's thread** are the classic crash: a program's
  code must never run there.
- **The browser** asks the user before a page may use MIDI, and separately
  for SysEx: the program must be told plainly when it was refused.
- **Easy for a beginner**: "a note came in" should be one event with the
  channel, the note and the velocity, not bit masks.

## 2. The component

`DIM Midi AS RMIDIPort` (or in a CREATE block). One RMIDIPort holds at most
one input and one output open at a time; a program that listens to two
keyboards makes two RMIDIPorts.

### Devices

| Member | What it is |
|---|---|
| `InputCount` (method, read like a property) | How many MIDI inputs the system has now. Reads the system's list again. |
| `OutputCount` (same) | How many MIDI outputs. |
| `InputDevice(Index)` | Input number `Index`'s name (0 is the first; the list `InputCount` read). "" past the end. |
| `OutputDevice(Index)` | The same for outputs. |
| `FillInputs(List)`, `FillOutputs(List)` | Fill an RListBox or RComboBox with the names (cleared first; the open one selected). Returns how many. |

Names are the system's (CoreMIDI's display name, ALSA's `client:port`,
winmm's product name, Web MIDI's `name`). When two have the same name the
second is `Name (2)`, the third `Name (3)`, so a name always means one
device. `InputCount` and `OutputCount` are methods with `value = true` in
the registry so that, in the browser, the first one can wait for the user's
answer to the permission question (§5).

### Opening and closing

| Member | What it does |
|---|---|
| `OpenInput(Device)` | Opens an input: `Device` is its number in the list, or its name — the exact name first, then a part of it, any case ("keyboard" finds "USB Keyboard"). Returns True when it opened; else False and `Error` says why. An input already open is closed first. |
| `OpenOutput(Device)` | The same for an output. |
| `CloseInput`, `CloseOutput`, `Close` | Close one, the other, or both. Closing what isn't open does nothing. |
| `IsInputOpen`, `IsOutputOpen` (read) | True while one is open. |
| `InputName`, `OutputName` (read) | The name of the one open ("" when none). |

The program ending, or the RMIDIPort being freed, closes both (notes left
on are the program's business; `SendAllNotesOff` is a later idea).

### Receiving

| Member | What it is |
|---|---|
| `OnMessage(Status, Data1, Data2)` | Every short message: notes, controllers, program changes, pitch bend, aftertouch, and system messages (song position, start, stop, clock if not ignored …). `Status` is the full status byte (`&H90` + channel − 1 for a note on); `Data1`, `Data2` are 0 when the message has fewer bytes. Running status is already expanded: the status is always there. |
| `OnNoteOn(Channel, Note, Velocity)` | A note on, `Channel` 1 to 16. A note on with velocity 0 is a note off (the MIDI rule) and fires OnNoteOff instead. |
| `OnNoteOff(Channel, Note, Velocity)` | A note off. |
| `OnControl(Channel, Controller, Value)` | A control change (`&HB0`): a knob, a slider, the sustain pedal (64). |
| `OnSysEx(Data AS STRING)` | (with `IgnoreSysEx = False`) A whole SysEx message, `F0` to `F7` included, as a binary string (each character one byte, `ASC` 0–255). Pieces are joined; real-time bytes in the middle are taken out (and given to OnMessage); a SysEx cut by another status byte is dropped and OnError says so. |
| `OnDevicesChanged` | A device was plugged in or out (or the browser's list changed). Read the lists again. |
| `OnError(Message AS STRING)` | Something went wrong while nobody was calling: the device went away, messages were dropped, a SysEx was too long or cut, the browser refused. The same text is in `Error`. |
| `Timestamp` (read, DOUBLE) | When the message being handled arrived, in ms since the input opened (fractions of a ms kept). For recording. |
| `IgnoreSysEx` (True) | SysEx messages that arrive are dropped until the program sets it to False: SysEx can change what is stored inside a device, so a program asks for it (and in a browser that is a separate permission, asked only then: §5). |
| `IgnoreClock` (True) | Drops timing clock (`F8`) and MIDI time code quarter frames (`F1`). |
| `IgnoreActiveSensing` (True) | Drops active sensing (`FE`). |
| `Queued` (read) | Messages that arrived and wait for their events. |
| `Dropped` (read) | Messages lost because they came faster than the program handled them (§4). 0 is the goal. |

OnMessage fires for every short message, notes too; OnNoteOn / OnNoteOff /
OnControl fire after it for theirs. A program uses whichever it likes.

### Sending

| Member | What it does |
|---|---|
| `Send(Status, [Data1], [Data2])` | Sends one short message. Its length comes from the status (`&HC0` program change and `&HD0` channel pressure take one data byte; `&HF8` clock none). Data bytes are 0 to 127. Returns True when it went out. |
| `SendNoteOn(Channel, Note, Velocity)` | A note on, channel 1 to 16. |
| `SendNoteOff(Channel, Note, [Velocity])` | A note off (velocity 0 by default). |
| `SendControl(Channel, Controller, Value)` | A control change. |
| `SendProgram(Channel, Program)` | A program (instrument) change, 0 to 127. |
| `SendSysEx(Data AS STRING)` | Sends a whole SysEx message: a binary string starting with `F0` (240) and ending with `F7` (247), every byte between 0 to 127. Any length (a 4 KB patch dump is fine). In a browser the first one asks for the SysEx permission. |
| `SendHex(Text AS STRING)` | Sends bytes written in hex: `"F0 7E 7F 06 01 F7"`, `"90 3C 64"`. Spaces, commas, `&H` and `0x` are allowed. Each message in the text must be whole. |
| `Hex(Data AS STRING) AS STRING` | The bytes of `Data` in hex, `"F0 7E 7F 06 01 F7"`: what a monitor shows. |
| `Error` (read) | Why the last call didn't work, in plain words ("" when it did): no such device, the device is in use, nothing open, a data byte over 127, a SysEx without its `F7`, the browser refused. |

Validation is the same everywhere (the shared model checks the bytes before
any device sees them), so a mistake gives the same message on every runtime.

## 3. The shared model

`crates/rapidr-value/src/objects/midiport.rs`:

- **The parser** (`Parser`): a MIDI byte stream in, whole messages out.
  Running status; real-time bytes (`F8`–`FF`) anywhere, even inside a SysEx
  or between a status and its data, delivered at once without breaking what
  they interrupted; SysEx joined across any number of pieces until `F7`; a
  status byte inside a SysEx ends it (dropped, said); stray data bytes with
  no status ignored; `F4`, `F5`, `F9`, `FD` (undefined) ignored. The
  ignore flags are applied here, so every platform ignores the same way.
  Unit-tested byte by byte.
- **The inbox** (`Inbox`, `Arc<Mutex<…>>`): what the driver's thread hands
  over. The driver's callback only parses and pushes (no allocation beyond
  the message, no program code), then wakes the program's loop. Bounded:
  32 768 messages and 16 MB of SysEx waiting at most; past that a message is
  counted in `Dropped` instead (never blocks the driver), and OnError says
  once how many were lost. A SysEx longer than 1 MB is dropped (said).
- **The system** (`trait MidiSystem`): the runtime's devices — `inputs()`,
  `outputs()`, `open_input(name, inbox)`, `open_output(name)`; an open
  output is `trait MidiOut { fn send(&mut self, bytes) }`, an open input is
  kept as a `Box<dyn Any>` the model drops to close it. Installed once per
  runtime (`set_system`), like QCOMPORT's ports.
- **The tests' system** (`TestMidi`): `RAPIDR_TEST_MIDIPORT` (or the page's
  variable in the browser) lists scripted devices, `;`-separated:
  `in:NAME` (an input), `out:NAME` (an output that takes anything),
  `loop:NAME` (an output and an input of that name: what is sent comes
  back, SysEx in 1 KB pieces as winmm gives them), `synth:NAME` (an output
  and an input: a synthesizer that answers an Identity Request with a
  scripted Identity Reply, maker `7D`), `burst:NAME=N` (an input that plays
  N notes at once), `in:NAME=HEX` (an input
  that plays those bytes when opened), `busy:NAME` (an input and output in
  use). Never a real device. A program's `ENVIRON "RAPIDR_TEST_MIDIPORT=…"`
  plugs one in or out (OnDevicesChanged).
- **Delivery** (`deliver`): between the program's turns the runtime asks the
  model for the next message of each open input and fires its events. One
  message is in flight at a time: the next goes once the last one's
  handlers ran (`rapidr_value::events::fire_then`), so `Timestamp` is
  always the handled message's, the program's events never pile up in the
  interpreter's queue, and what waits stays in the bounded inbox. A native
  build runs handlers at once, so it takes up to 512 messages (or 15 ms) a
  turn and then lets the window paint.

## 4. Threading and timing

- **Desktop**: midir calls back on its own thread (CoreMIDI's, ALSA's
  thread midir starts, winmm's callback). That thread parses into the
  inbox and calls `rp_wake` (the window loop's waker, as RHttpServer's
  threads do). The window loop's turn (`ui/kernel.rs`) and `DOEVENTS` call
  `midi_port::poll()`, which fires the events on the program's thread. A
  console program hears its messages in `DOEVENTS`.
  - Latency: the time for the loop to wake (well under a millisecond on an
    idle machine), not a polling interval.
  - The device list is looked at about once a second while an
    OnDevicesChanged handler is set (midir has no plug notification).
- **Browser**: Web MIDI calls `onmidimessage` on the page's thread between
  the program's turns; the bytes go through the same parser into the same
  inbox and are delivered the same way. `onstatechange` gives
  OnDevicesChanged.
- **Bursts**: the inbox takes 32 768 messages, about 10 seconds of a fully
  busy MIDI cable (3 125 bytes a second), so a program that falls behind for
  a moment loses nothing; `Dropped` and OnError say when one falls behind
  for longer.

## 5. Platforms

| | Inputs/outputs | SysEx | Plug in/out | Notes |
|---|---|---|---|---|
| Windows | winmm through midir | winmm splits long SysEx into 1 KB buffers: joined by the parser | looked at each second | Names as winmm gives them (32 characters at most). The Microsoft GS Wavetable Synth is an output. A device another program has open can't be opened (winmm's rule): Error says "in use". |
| macOS | CoreMIDI through midir | packets joined | looked at each second | IAC buses and network sessions appear when the user turned them on; RapidR never turns anything on. |
| Linux | ALSA sequencer through midir (libasound, the system's) | joined | looked at each second | "Midi Through" is always there (a loop). |
| Browser | the browser's MIDI interface (the Web MIDI API), through RapidR's own code (`media_web.rs`, as RMIDI's output), never midir's web backend | a separate permission, asked only when the program asks for SysEx | `statechange` | See below. |

**The browser** (docs/legal/review-2026-11-05-midi.md, C2 and C3):

- Nothing is asked when the page loads. The first RMIDIPort call that needs
  the devices (`InputCount`, `OutputCount`, `InputDevice`, `FillInputs`,
  `OpenInput` …) first shows RapidR's own one-line explanation ("This
  program wants to use your MIDI devices … RapidR does not send the list of
  your devices anywhere." — Continue / Not now), then the browser's prompt
  (`requestMIDIAccess()` **without** SysEx). The program waits for the
  answer (the page goes on); a native web build, which can't wait, gets the
  devices a moment later and OnDevicesChanged.
- SysEx is asked for separately, only when the program wants it: `IgnoreSysEx
  = False` or a `SendSysEx`/`SendHex` with SysEx, at that call, after a
  second explanation ("… SysEx … can change settings stored inside a MIDI
  device and, on some devices, even replace its firmware. Allow it only for
  programs you trust."). Refusing SysEx leaves notes working.
- A refusal is a plain error, never a crash: "MIDI access was not allowed in
  this browser" / "SysEx messages were not allowed in this browser"; "Not
  now" says "MIDI wasn't turned on: the program can ask again". RapidR
  remembers nothing about the answer beyond the browser's own memory.
- Pages on HTTPS or localhost only (the browser's rule); a page in a frame
  needs `allow="midi"`; Safari and iOS have no MIDI in the browser: Error
  says "This browser has no MIDI support".
- Device names stay in the page: the runtime sends them nowhere.

Interpreter, native and web share the model, so the members, the
validation, the parser and the messages are the same; only the device
differs. No pointers, no headers, no `$UNSAFEMEM`.

## 6. Errors, in plain words

- "No MIDI input is called "X" (there are: A, B)" / "There is no MIDI input
  number 3 (there are 2: 0 to 1)"
- "MIDI input "X" is in use by another program"
- "No MIDI output is open: call OpenOutput first"
- "Data1 is 0 to 127 (it was 200)"
- "A SysEx message starts with F0 (240) and ends with F7 (247)"
- "Byte 5 of the SysEx is 200: bytes between F0 and F7 are 0 to 127"
- "MIDI input "X" was unplugged" (OnError; the input is closed)
- "37 MIDI messages were dropped: they came faster than the program
  handled them" (OnError, once per run of drops)
- "A SysEx message longer than 1 MB was dropped"
- "A SysEx message was cut by another message and dropped"
- "The browser didn't allow MIDI for this page" / "… SysEx messages …"
- "This browser has no Web MIDI"
- "This program was built without MIDI" (a lean build without the device)

## 7. Tests (no hardware)

- **Model** (Rust unit tests): the parser byte by byte (running status,
  real-time inside SysEx and between data bytes, SysEx in many pieces, a
  4 KB SysEx, a cut SysEx, the ignore flags), the inbox bounds and Dropped,
  validation and its messages, the test system's script, duplicate names.
- **Desktop loopback** (macOS and Linux, a real system path): a test opens
  a midir *virtual output* in the same process and RMIDIPort opens it as an
  input; notes, controllers, a 4 KB SysEx round trip and a burst of 2 000
  messages arrive whole and in order. The virtual port exists only while the
  test runs; nothing of the user's MIDI setup is touched. Windows has no
  virtual ports in winmm: there the test lists the devices and sends a GS
  Reset to the Microsoft GS Wavetable Synth.
- **Runtimes** (interpreted, native, web): conformance cases on the test
  system's `loop:` device (sends, receives, SysEx in pieces, errors), and
  a GUI fixture with events.
- **Web**: Playwright with a mocked `navigator.requestMIDIAccess` (inputs,
  outputs, `onmidimessage`, `send`, a refused SysEx permission, a
  `statechange`), the real Web MIDI code path.
- **Examples**: `examples/media/midi_monitor.rr` and
  `examples/media/midi_sysex_port.rr` in the examples suite on the test
  system, every runtime.

## 8. Legal (docs/legal/review-2026-11-05-midi.md: OK with conditions)

- midir (MIT) is already a dependency of runtime-core (RMIDI's output): no new
  crate. Bumped to 0.11.1 (its Windows SysEx-buffer fixes), no features
  (`jack`, `winrt` stay off; `jack-sys` banned in deny.toml), native only:
  midir stays out of the web build. A `midi` feature of runtime-core (part
  of `audio`) holds it. Notices regenerated (C7).
- The browser side is RapidR's own code on the browser's API (C2), with the
  explanations of §5 (C3).
- Linux: the system's ALSA library (`libasound2`, LGPL-2.1+) is linked
  dynamically; LEGAL.md tells users to list it as a package dependency and
  never copy it into their package (C4).
- Names: "MIDI" names the standard the component speaks; no MIDI logo, no
  certification or approval wording, "MIDI" never in a product name; the
  browser's interface is "the browser's MIDI interface (the Web MIDI API)",
  never a RapidR feature name (C5). LEGAL.md's Trademarks gets the line.
- SysEx: RapidR defines no SysEx format of its own; the examples use
  Universal SysEx (the Identity Request `F0 7E 7F 06 01 F7`, GM System On)
  or a device maker's message typed by the user. Help says custom formats in
  a released program need a manufacturer ID from the MIDI Association
  (`0x7D` is for unreleased, non-commercial work) and warns that SysEx can
  change a device's stored settings or firmware (C6).
- AI and MCP: listing and opening are read-only; sending (SysEx above all)
  is an action the user confirms; the tools' descriptions say port names go
  to the chosen AI provider. Crash reports and logs never carry port names,
  ids or message bytes (C8).

## 9. Later

- Virtual ports of a program's own on macOS and Linux (another app plays
  into a RapidR program), timestamps given to Send (send later),
  SendAllNotesOff, a "learn" helper for controls.
- MIDI 2.0 (UMP, MIDI-CI) is out of scope: a separate review first.
