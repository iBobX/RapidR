# DirectX, media and devices

The DirectX, media and device objects run on every platform and in the
browser. They are RapidR's own implementations of what RapidQ programs do
with Windows' DirectX, MCI, WinINet and serial APIs: the same members and
behaviour, checked against RapidQ (RC.EXE, and Windows' MCI in a Windows
VM) where it still runs. Each has a RapidQ name too (`QDXSCREEN` for
`RDXScreen`, and so on), accepted everywhere.

## DirectX 2D

| Object | |
|---|---|
| `RDXScreen` | an off-screen surface shown on `Flip`: `Init`, `AutoSize`, `AllowStretch`, `Fill` (DirectDraw colours), `Pixel`, text (`TextOut`, `TextRect`), `Draw` / `StretchDraw` / `CopyRect`, `Rotate`, `View.*`; `OnInitialize` / `OnInitializeSurface` when its form first shows; `FullScreen` covers the screen with the surface scaled to fit |
| `RDXImageList` | DelphiX image libraries (`.DXG`) with their transparency and patterns |
| `RDXTimer` | a frame timer: `FrameRate`; `Interval = 0` fires once a frame; `ActiveOnly` only while the program is active |
| `RDXSound` | a WAV played at its `Frequency`, `Volume` and `Pan` (DirectSound's decibels), looped or once; `Playing`, `Position`, `Size`, `Play`, `Stop` |
| `RDXJoystick` | RapidQ's `Update`, `IsLeft` / `IsRight` / `IsUp` / `IsDown`, `Button(n)`; RapidR adds `Index`, `Connected`, `Name`, the axes `X Y Z R U V`, `Buttons`, `POV` and the events `OnButtonDown`, `OnButtonUp`, `OnMove` |

Joysticks: gamepads through gilrs (Windows, macOS), evdev (Linux) and the
browser's Gamepad API, all reporting the same layout (an Xbox controller as
Windows' winmm showed it). With no joystick, every member reads "not
connected" instead of RapidQ's error.

## Direct3D (retained mode)

`RD3DFrame`, `RD3DMeshBuilder`, `RD3DMesh`, `RD3DFace`, `RD3DLight`,
`RD3DTexture`, `RD3DVisual`, `RD3DWrap`, `RD3DVector` and RDXScreen's 3D
methods (`CreateFrame`, `CreateMeshBuilder`, `CreateLightRGB`,
`SetCameraPosition`, `CameraLookAt`, `Render`, …): frames, faces, `.X`
models (text and binary — every model of RapidQ's examples loads), lights
(ambient, directional, point, spot), the camera, textures, wraps,
shadows and blended transparency, lit flat or Gouraud.

RapidR draws 3D with its own software rasterizer: the same pixels on every
system and in the browser, sharp at any display scale, and fast enough for
RapidQ's programs (a 29,000-face model renders in about 4 ms a frame). Fog
is accepted and has no effect (it never had one in RapidQ either). RapidQ's 3D can no longer be compared with RapidQ itself (Windows
dropped Direct3D Retained Mode after XP), so where its documentation is
silent RapidR follows Direct3D's.

## Sound and music

- `PLAYWAV file [, options]` and `PLAYSOUND` play WAV, Ogg Vorbis, FLAC and
  MP3 files (or `$RESOURCE`s); `BEEP` and `SOUND freq, duration`.
- **RMIDI** (RapidQ's `QMIDI`, from `QMidi.inc`) plays standard MIDI files with MCI's timing on
  the system's MIDI output; where there is none (macOS, Linux, browsers
  without Web MIDI) RapidR's own General MIDI synthesizer plays them.
- **RWave** (`QWAVE`, from `QWave.inc`) plays WAV files, records from the default input
  (`New`, `Record`), saves and deletes.
- **RVideo** (`QVIDEO`, from `QVideo.inc`) plays AVI files with RapidR's own decoders:
  uncompressed, Microsoft RLE, Microsoft Video 1, Cinepak and Motion JPEG,
  with their PCM sound — on a form or in a window of its own.
- **RCDAudio** (`QCDAUDIO`, from `Qcdaudio.inc`) answers as a computer with no CD drive.

The media objects share RapidQ's members: `Open`, `Close`, `Play`, `Stop`,
`Pause`, `State`, `FileOpen`, `Lenght` (RapidQ's spelling), `Error` (MCI's
own texts), `Volume`, their `Timer` and `OnChange`. A program that
`$INCLUDE`s RapidQ's library file gets RapidR's object.

## Devices and the network

| Object | |
|---|---|
| `RComPort` | a serial port (RapidQ's `COMPORT`, from RAPIDQ2.INC): `Port`, `BaudRate`, `DataBits`, `Parity`, `StopBits`, `Open`, `Close`, `ReadString`, `WriteString`, `OnRxChar`, `OnComError`, … — serial2 on the desktop (nothing to install on Linux), Web Serial in the browser; RapidR's extras for IoT boards (ListPorts, DTR / RTS, ReadLine, OnLine, OnPortsChanged) in [Serial ports and IoT boards](serial-ports.md) |
| `RSocket` (RapidQ's `QSOCKET`), `RServerSocket` | TCP client and server (the browser: WebSocket client only) |
| `RHttp` | HTTP(S) `Get` and `Post` |
| `RDownload` | RapidQ's `QDOWNLOAD` (`Qdownload.inc`) and its `LeechFile`: a file from a web server into a variable or a file, with progress; port 443 is HTTPS, redirects are followed, and the program's windows keep working while it waits |
| `RCGI` | RapidQ's `QCGI` (`qcgi.inc`) CGI object: the CGI variables as properties, `Parse`, `Get(Name, Value)` — for `$APPTYPE CGI` programs behind a web server |

HTTPS uses the operating system's TLS (macOS' Security framework,
Windows' SChannel, Linux's OpenSSL 3) and its certificates: RapidR compiles
no encryption code into your program.

## The system tray

RapidQ programs put an icon in Windows' notification area with
`RNotifyIconData` and `Shell_NotifyIcon`, and hear its clicks in their
form's `WndProc`. RapidR keeps that one Windows call working everywhere:
the icon shows in the macOS menu bar, the Windows notification area, the
Linux tray (StatusNotifierItem) and a strip at the bottom right of a web
page, and clicks reach `WndProc (hWnd, uMsg, wParam, lParam)` as Windows'
mouse messages. `NI.hIcon = Application.Icon` shows the program's icon.

## The registry

`RRegistry` is Windows' own registry on Windows, as RapidQ's was. On macOS,
Linux and the web it is a per-user store that answers the same way
(`~/Library/Application Support/RapidR/registry.reg`,
`~/.config/rapidr/registry.reg`). `RAPIDR_REGISTRY=<file>` puts the keys
in that file on every system, so tests never touch the real registry.
