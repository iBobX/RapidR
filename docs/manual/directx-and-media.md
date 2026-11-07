# DirectX, media and devices

The DirectX, media and device objects run on every platform and in the
browser. They are RapidR's own implementations of what programs do
with Windows' DirectX, MCI, WinINet and serial APIs, with the members and
behaviour that older programs rely on. Each has a classic name too
(`QDXSCREEN` for `RDXScreen`, and so on), accepted everywhere.

## DirectX 2D

| Object | |
|---|---|
| `RDXScreen` | an off-screen surface shown on `Flip`: `Init`, `AutoSize`, `AllowStretch`, `Fill` (DirectDraw colours), `Pixel`, text (`TextOut`, `TextRect`), `Draw` / `StretchDraw` / `CopyRect`, `Rotate`, `View.*`; `OnInitialize` / `OnInitializeSurface` when its form first shows; `FullScreen` covers the screen with the surface scaled to fit |
| `RDXImageList` | DelphiX image libraries (`.DXG`) with their transparency and patterns |
| `RDXTimer` | a frame timer: `FrameRate`; `Interval = 0` fires once a frame; `ActiveOnly` only while the program is active |
| `RDXSound` | a WAV played at its `Frequency`, `Volume` and `Pan` (DirectSound's decibels), looped or once; `Playing`, `Position`, `Size`, `Play`, `Stop` |
| `RDXJoystick` | The classic `Update`, `IsLeft` / `IsRight` / `IsUp` / `IsDown`, `Button(n)`; RapidR adds `Index`, `Connected`, `Name`, the axes `X Y Z R U V`, `Buttons`, `POV` and the events `OnButtonDown`, `OnButtonUp`, `OnMove` |

Joysticks: gamepads through gilrs (Windows, macOS), evdev (Linux) and the
browser's Gamepad API, all reporting the same layout (an Xbox controller as
Windows' winmm showed it). With no joystick, every member reads "not
connected" instead of an error.

## Direct3D (retained mode)

`RD3DFrame`, `RD3DMeshBuilder`, `RD3DMesh`, `RD3DFace`, `RD3DLight`,
`RD3DTexture`, `RD3DVisual`, `RD3DWrap`, `RD3DVector` and RDXScreen's 3D
methods (`CreateFrame`, `CreateMeshBuilder`, `CreateLightRGB`,
`SetCameraPosition`, `CameraLookAt`, `Render`, …): frames, faces, `.X`
models (text and binary — every model of the compatibility suite loads), lights
(ambient, directional, point, spot), the camera, textures, wraps,
shadows and blended transparency, lit flat or Gouraud.

RapidR draws 3D on the GPU through wgpu — Metal on macOS, Direct3D 12 on
Windows (its WARP software adapter where there's no GPU), Vulkan or OpenGL
on Linux, WebGL 2 in the browser — sharp at any display scale and fast
(the compatibility suite's largest model, 16,000 triangles, about 4 ms a frame), then hands
the picture to the screen's back buffer, so text drawn after `Render`,
`Pixel` and `Flip` work as before. Fog is accepted and has no effect (it
never had one). RapidR's 3D matches the compatibility suite's examples, and
follows Direct3D's documentation where those are silent.

## Sound and music

- `PLAYWAV file [, options]` and `PLAYSOUND` play WAV, Ogg Vorbis, FLAC and
  MP3 files (or `$RESOURCE`s); `BEEP` and `SOUND freq, duration`.
- **RMIDI** (older name `QMIDI`, from `QMidi.inc`) plays standard MIDI files with MCI's timing on
  a system MIDI output that makes sound; where there is none (macOS, Linux,
  browsers without Web MIDI) RapidR's own General MIDI synthesizer plays them.
  An output counts when its name says it is a synthesizer (Windows' *GS
  Wavetable Synth*, FluidSynth, TiMidity …) or, on Windows, when it is the
  first one that isn't a loop or a bus. A Mac's own MIDI outputs (an IAC bus,
  a network session, a virtual port another program made) and Linux's *Midi
  Through* only pass messages on to other programs and make no sound, so a
  song is never sent to one of them by itself: RapidR's synthesizer plays it.
  To play on a keyboard or a sound module you have plugged in, name it before
  the program starts (a part of its name, in any case); `builtin` always
  plays on RapidR's synthesizer. A name no output has is reported on the
  error output and RapidR's synthesizer plays.

  ```
  RAPIDR_MIDI_OUT="Roland" rapidr run song.rr     (macOS, Linux)
  set RAPIDR_MIDI_OUT=builtin & rapidr run song.rr  (Windows)
  ```

  Example: [`examples/media/midi_compose.rr`](../../examples/media/midi_compose.rr)
  writes a song in code (chords, a bass line, drums on channel 10) and plays
  it. In a browser the page variable `RAPIDR_MIDI_OUT` does the same.
- **RWave** (`QWAVE`, from `QWave.inc`) plays WAV files, records from the default input
  (`New`, `Record`), saves and deletes.
- **RVideo** (`QVIDEO`, from `QVideo.inc`) plays AVI files with RapidR's own decoders:
  uncompressed, Microsoft RLE, Microsoft Video 1, Cinepak and Motion JPEG,
  with their PCM sound — on a form or in a window of its own.
- **RCDAudio** (`QCDAUDIO`, from `Qcdaudio.inc`) answers as a computer with no CD drive.

The media objects share these members: `Open`, `Close`, `Play`, `Stop`,
`Pause`, `State`, `FileOpen`, `Lenght` (the classic spelling), `Error` (MCI's
own texts), `Volume`, their `Timer` and `OnChange`. A program that
`$INCLUDE`s the classic library file gets RapidR's object.

## Sound made sample by sample: RAudioOut

**RAudioOut** plays sound your program makes as it goes, one number
(a *sample*) at a time: an emulator's beeper or sound chip, a synthesizer,
a tone for a physics lesson, a music game. The program writes samples into
the stream's queue; the sound device plays them on a thread of its own and
never waits for the program.

![The tone generator playing A4: wave buttons, frequency and volume sliders and a Stop button on the left; on the right an oscilloscope screen with three cycles of a sine wave and the readout "440.0 Hz sine, playing: 60 ms queued, 0 gaps"](images/media/tone-generator.png)

```basic
DIM Speaker AS RAudioOut
DIM phase AS DOUBLE

SUB Feed (Count AS INTEGER)                 ' the stream asks for Count samples
    DIM buf(Count - 1) AS INTEGER, i AS INTEGER
    FOR i = 0 TO Count - 1
        buf(i) = SIN(phase) * 6000          ' a quiet 440 Hz tone
        phase = phase + 2 * 3.14159265 * 440 / Speaker.SampleRate
    NEXT
    Speaker.Write buf, Count
END SUB

Speaker.OnNeedData = Feed
Speaker.Start
```

- **Samples.** By default a sample is a 16-bit number from -32768 to 32767,
  44,100 of them a second (`SampleRate`, 8000 to 96000). `Write` takes an
  INTEGER (or BYTE, LONG …) array, a binary string (two bytes a sample, low
  byte first), an `RMemoryStream` (from its `Position`) or one number, with
  an optional `Count` and `Offset`; `WriteFloat` takes -1 to 1 from a SINGLE
  or DOUBLE array; `WriteSample v` writes one sample, the easiest way to
  learn. `Bits = 8` reads bytes 0 to 255 (128 is silence) as 8-bit WAV files
  have them. With `Channels = 2` samples are interleaved: left, right, left …
- **Keeping it fed.** Keep about `BufferMs` (60 ms by default) of sound
  queued. Either write in `OnNeedData(Count)`, which the stream raises on
  your program's own thread when the queue drops under `BufferMs` (Count
  fills it again), or, for a program that works a frame at a time, check
  `Queued` (ms waiting) from an `RTimer` and write until it is about
  `BufferMs`. The sound you hear is about `BufferMs` late: smaller answers
  sooner, larger survives a busy moment.
- **When the program is late** the queue runs dry: the stream plays silence
  after a 2 ms fade (no click), counts the gap in `Underruns`, and fades the
  next sound in. `Stop` fades out the same way; `Volume` (0–100, by the ear:
  50 sounds half as loud) glides over 10 ms, so a slider never clicks.
  `Enabled = False` pauses; `Clear` drops what waits. `Start` gives the
  sound device 40 ms of silence before your samples, so a headset that
  takes a burst as it begins (Bluetooth ones do) never cuts the start.
- **When something goes wrong,** `OnError(Code, Message)` says it, on your
  program's thread a moment after it happened: `aeNoDevice` (1: no sound device,
  or it refused the stream; the stream plays silently, in time), `aeDeviceLost`
  (2: the device stopped taking sound while it played, unplugged or a headset
  that went away; said once), `aeBadSetting` (3: a property set out of its range,
  the nearest value is used), `aeUnderrun` (4: the queue ran dry, a gap you hear;
  at most four a second, the message says how many times — a sound that simply
  ends counts once too) and `aeNeedsClick` (5: a browser plays sound only after
  a click or a key). `Message` says it in words, and so does `Error` until the
  problem is over. A program without an `OnError` changes nothing: `Error` and
  `Underruns` still tell. [audio_errors.rr](../../examples/media/audio_errors.rr)
  shows each one.
- **Everywhere.** The same on Windows, macOS and Linux, interpreted and
  native, and in the browser. The device converts your `SampleRate` to its
  own. Without a sound device (or in a program built without sound) the
  stream plays silently, in time, and `Error` says why: "No sound output is
  available on this computer", or on Windows XP, whose sound system RapidR
  can't use yet, "Sound output needs Windows Vista or newer for now". A
  program is never stopped by its sound: `PLAYWAV`, `PLAYSOUND`, `BEEP`,
  `SOUND`, RWave's Play and Record and RMIDI's built-in synthesizer are
  silent in the same cases (a MIDI song still plays on Windows' own GS
  Wavetable synthesizer, XP's too, and RMIDIPort works there).

### Emulators: sound from a level timeline

A computer's simplest sound is one bit wired to a speaker, flipped by the
program at exact moments. An emulator plays it a frame at a time
(examples/media/beeper.rr, and the recipe examples/recipes/frame_audio.rr):

1. run the emulated machine for one frame (at 50 frames a second, 1/50 of
   its cycles), noting the cycle of every write to the speaker;
2. for each of the frame's samples (44100 / 50 = 882), how much of its
   time the bit was high: -1 to +1 by that share. Averaging over the
   sample instead of reading the bit once keeps the sound clean (no
   aliasing hiss);
3. a DC blocker (`out = x - prevX + 0.995 * prevOut`): a real speaker can't
   hold a push, so a bit left high drifts back to the middle;
4. `Write` the 882 samples.

An `RTimer` checks often and runs frames until about 60 ms is queued, so a
tick that comes late catches up and the sound never gaps. Use
`Interval = 10`: an `RTimer`'s ticks come on the system timer's 15.6 ms
steps (`Interval = 20` ticks only every 31 ms, and a RapidR program's timer
ticks one Interval after its handler has *ended*, which leaves a 60 ms queue
little room for a late one). Or set `FixedRate = 1` with `Interval = 20`: a
tick exactly every 20 ms by the clock, one frame each, a late one made up at
once ([A picture every 20 milliseconds](graphics.md#a-picture-every-20-milliseconds)).
RapidR's interpreter makes such a frame in 0.2 to 0.6 ms, a small part of the
20 ms it has.

![The one-bit beeper: the speaker bit as a square wave over one frame at the top, the frame's 882 samples below it, and the readout: the note, the frame made in well under a millisecond, 75 ms queued, 0 gaps](images/media/beeper.png)

### In the browser

A browser plays sound only after the person clicked or pressed a key on the
page: call `Start` from a button's `OnClick`. Until the page may play,
`Error` says so; the sound starts (and `Error` clears) as soon as it may.
Keep the queue a little longer there (`BufferMs = 80` or more) if the page
is busy: the browser plays the stream between the page's own work.

The examples: [tone_generator.rr](../../examples/media/tone_generator.rr)
(a sine, square, sawtooth or noise and an oscilloscope of the samples
played), [beeper.rr](../../examples/media/beeper.rr) (the one-bit speaker),
[chiptune.rr](../../examples/media/chiptune.rr) (three square-wave voices
playing RapidR's own tune) and the recipe
[frame_audio.rr](../../examples/recipes/frame_audio.rr).

## MIDI in and out: RMIDIPort

**RMIDIPort** connects your program to MIDI keyboards, controllers, drum
pads and synthesizers: it lists them, opens one input and one output, tells
your program what arrives with events, and sends notes, controls and
System Exclusive (SysEx) messages. There are no pointers, buffers or system
calls to get right, and it is the same on Windows, macOS, Linux and in the
browser.

![The MIDI monitor: an input and an output picked at the top, the messages received listed with their time, their bytes in hex and what they mean (a SysEx, a note on and a note off), and the Identity Request ready to send at the bottom](images/media/midi-monitor.png)

```basic
DIM Midi AS RMIDIPort

SUB NoteOn (Channel AS INTEGER, Note AS INTEGER, Velocity AS INTEGER)
    PRINT "note "; Note; " on channel "; Channel; ", velocity "; Velocity
END SUB

Midi.OnNoteOn = NoteOn
IF Midi.InputCount > 0 THEN
    Midi.OpenInput 0                          ' the first input
ELSE
    PRINT "No MIDI input: "; Midi.Error
END IF
```

- **Devices.** `InputCount` and `OutputCount` read the system's lists;
  `InputDevice(i)` and `OutputDevice(i)` give the names (0 is the first);
  `FillInputs List` and `FillOutputs List` put them in an `RListBox` or
  `RComboBox`. `OpenInput` and `OpenOutput` take a number or a name: the
  exact name, or the first one that contains it (`Midi.OpenInput "keys"`
  opens "USB Keys"). Two devices with the same name are told apart: the
  second is "Name (2)". `OnDevicesChanged` runs when one is plugged in or
  out. One `RMIDIPort` opens one input and one output; make one per
  keyboard you want to listen to.
- **Receiving.** `OnMessage(Status, Data1, Data2)` gets every short
  message: notes, controls, program changes, pitch bend. For the common
  ones there are easier events: `OnNoteOn` and `OnNoteOff` (channel 1 to
  16, note, velocity; a note on with velocity 0 counts as a note off) and
  `OnControl` (a knob, a slider, the sustain pedal). Events run on your
  program's own thread, in the order the messages came; `Timestamp` says
  when the message arrived (ms since the input opened). Timing clock and
  active sensing, which some devices send all the time, are dropped until
  you set `IgnoreClock = False` or `IgnoreActiveSensing = False`.
- **Sending.** `SendNoteOn channel, note, velocity`, `SendNoteOff`,
  `SendControl`, `SendProgram`, or `Send status, data1, data2` for any short
  message. `SendHex "90 3C 64"` sends bytes written in hex. Each returns
  True when the message went out; otherwise `Error` says why in plain
  words (a value over 127, no output open, the device was unplugged).
- **Bursts.** Messages wait in a queue until your program takes them, so a
  burst is never lost; if the program falls far behind (more than 32,768
  waiting), `Dropped` counts what was lost and `OnError` says so.

### SysEx: System Exclusive messages

A SysEx message is a whole block of bytes from `F0` to `F7`: patch dumps,
device settings, the Identity Request a synthesizer answers with its maker
and model. **SysEx is off until your program asks for it: set
`IgnoreSysEx = False` to receive SysEx.** Then `OnSysEx(Data)` gets each
message whole, as a binary string (`ASC` of each character is a byte),
even when it came in pieces; `Hex(Data)` shows it as `F0 7E 7F 06 02 …`.
`SendSysEx Data` sends a binary string, and `SendHex` the same written in
hex:

```basic
Midi.IgnoreSysEx = False                      ' receive SysEx
Midi.OnSysEx = Answer                         ' SUB Answer (Data AS STRING)
Midi.SendHex "F0 7E 7F 06 01 F7"              ' Identity Request: "who are you?"
```

![SysEx with a synthesizer: the devices on the left, the Universal messages and the device picked on the right, the bytes F0 7E 7F 06 01 F7, and below the log: the Identity Request sent, the reply received and decoded (maker, family, model, version)](images/media/midi-sysex.png)

- **Be careful with SysEx.** A SysEx message can change settings stored
  inside a device and, on some devices, even replace its firmware. Send
  only messages you know (from the device's manual), and run programs that
  send SysEx only if you trust them.
- **Manufacturer IDs.** The second byte of a SysEx message says whose
  format it is. The MIDI standard's *Universal* messages (`7E`, `7F`), like
  the Identity Request and GM System On, work with any General MIDI
  device; the examples use only those. If your released program defines a
  SysEx format of its own, it needs a manufacturer ID from the MIDI
  Association (check their current terms); ID `7D` is only for tests,
  schools and work that is not released.

### In the browser

The first time the program lists or opens a device, a short note explains
what it wants, then the browser asks the person to allow MIDI. The program
waits for the answer. SysEx is a separate question, asked only when the
program sets `IgnoreSysEx = False` or sends a SysEx; saying no to it leaves
notes and controls working. A "no" is a plain message in `Error` ("MIDI
access was not allowed in this browser"), never a crash; after "Not now",
`OpenInput` or `OpenOutput` asks again.

- MIDI works in a browser only on a page served over HTTPS or from your own
  computer; a page shown inside a frame needs `allow="midi"` on that frame.
  Some browsers have no MIDI support at all: then the lists are empty and
  `Error` says so.
- RapidR sends the names of the person's devices nowhere. What your own
  program does with them is up to you: device names can identify a person
  ("Ana's keyboard"), so don't send them to a server without asking.

On Linux, MIDI (like sound) uses the system's ALSA library (`libasound2`):
if you package your program, list it as a dependency and don't copy it into
the package.

The examples: [midi_monitor.rr](../../examples/media/midi_monitor.rr)
(everything a device sends, in hex and in words, and a message typed in hex
sent back) and [midi_sysex_port.rr](../../examples/media/midi_sysex_port.rr)
(Universal SysEx messages sent to a synthesizer, its Identity Reply
decoded, a note and a chord to check the cable).

## Devices and the network

| Object | |
|---|---|
| `RComPort` | a serial port (older name `COMPORT`): `Port`, `BaudRate`, `DataBits`, `Parity`, `StopBits`, `Open`, `Close`, `ReadString`, `WriteString`, `OnRxChar`, `OnComError`, … — serial2 on the desktop (nothing to install on Linux), Web Serial in the browser; RapidR's extras for IoT boards (ListPorts, DTR / RTS, ReadLine, OnLine, OnPortsChanged) in [Serial ports and IoT boards](serial-ports.md) |
| `RSocket` (older name `QSOCKET`), `RServerSocket` | TCP client and server (the browser: WebSocket client only); servers in [Networking](network.md): this computer only (`127.0.0.1`), by lines, bytes or whole HTTP requests |
| `RHttpServer` | a web server in your program, on `127.0.0.1`: a local API, a dashboard, an MCP server ([Networking](network.md)) |
| `RHttp` | HTTP(S) `Get` and `Post` |
| `RDownload` | older name `QDOWNLOAD` (`Qdownload.inc`), with `LeechFile`: a file from a web server into a variable or a file, with progress; port 443 is HTTPS, redirects are followed, and the program's windows keep working while it waits |
| `RCGI` | older name `QCGI` (`qcgi.inc`), the CGI object: the CGI variables as properties, `Parse`, `Get(Name, Value)` — for `$APPTYPE CGI` programs behind a web server |

HTTPS uses the operating system's TLS (macOS' Security framework,
Windows' SChannel, Linux's OpenSSL 3) and its certificates: RapidR compiles
no TLS library into your program.

## The system tray

Programs put an icon in Windows' notification area with
`RNotifyIconData` and `Shell_NotifyIcon`, and hear its clicks in their
form's `WndProc`. RapidR keeps that one Windows call working everywhere:
the icon shows in the macOS menu bar, the Windows notification area, the
Linux tray (StatusNotifierItem) and a strip at the bottom right of a web
page, and clicks reach `WndProc (hWnd, uMsg, wParam, lParam)` as Windows'
mouse messages. `NI.hIcon = Application.Icon` shows the program's icon. On
Windows the form's `WndProc` also hears every other message of its window
(see [Windows messages](components.md#windows-messages-a-forms-wndproc)); on
the other systems the tray's are the only ones.

## The registry

`RRegistry` is Windows' own registry on Windows, as it always was. On macOS,
Linux and the web it is a per-user store that answers the same way
(`~/Library/Application Support/RapidR/registry.reg`,
`~/.config/rapidr/registry.reg`). `RAPIDR_REGISTRY=<file>` puts the keys
in that file on every system, so tests never touch the real registry.
