# RapidR on older Windows (7, 8 and XP)

RapidR runs on 32-bit Windows too: Windows 7, 8 and 8.1 with the **Windows 7 download** (an alpha),
and plain Windows XP SP3 with the **Windows XP test download** (unofficial: it is tested, but not
supported). This page says what works there and what doesn't. Everything else in this manual applies.

## What you need

- **Windows 7, 8 or 8.1** (32 or 64-bit): the `win7-x86` zip. Unzip it anywhere and start
  `RapidR-Studio.bat`.
- **Windows XP SP3** (32-bit): the `winxp-x86-test` zip, the same way. XP needs a processor with SSE2
  (a Pentium 4, an Athlon 64 or newer). Nothing has to be installed on XP: not the One-Core-API, not a
  newer runtime.

## What works

On Windows 7 and on plain Windows XP alike:

- **RapidR Studio**: the designer, the code editor, the Project pane, Help (F1), Settings, the
  right-click menus (right click, Shift+F10 and the Menu key) and the keyboard shortcuts (Ctrl+S,
  Ctrl+O, Ctrl+Shift+B ...).
- **Running a program** with F5, its output in Studio's Output pane.
- **Build**: on 32-bit Windows, Run > **Build (Interpreted App)** makes the app at once. A native build
  needs Rust, which 32-bit Windows doesn't have, so Studio doesn't offer it there and never asks; the
  interpreted app runs the same program and needs nothing on the computer it is copied to. (The
  32-bit downloads are 32-bit programs on a 64-bit Windows 7 too; the 64-bit Windows download has
  Build and Build Native App as everywhere.)
- **The programs you build**, and `rapidr run` in a Command Prompt: forms, the console, files and
  folders, big numbers (`INT64`, `UINT64`, `UINT32`), DLL calls (`DECLARE ... LIB`) and callbacks
  (`WndProc`), `RFormEx` (on top, see-through, the hole, the tray icon), right-click menus, MIDI through
  Windows' own MIDI devices (`RMIDIPort`; a computer with no MIDI device lists none and says so).
- **Open and Save dialogs**: on Windows XP they are XP's own Open, Save and Browse for Folder
  dialogs, with the same filters, starting folder and file name.
- **Languages**: English, Español and Português (Brasil) on both; 中文 (Simplified Chinese) on Windows 7
  and later. On Windows XP, Studio in Chinese shows empty boxes and says so in its Output pane: plain XP
  has no Chinese font, and RapidR doesn't use the fonts installed in XP yet (below), so installing XP's
  East Asian support doesn't help there yet. RapidR never downloads or ships a Windows font.
- **Symbols** such as ✓ ✗ ⚠ come from Windows' own fonts; on a Windows with no fonts RapidR can use
  (plain XP), RapidR draws them with its own small symbol font.

## What doesn't (yet)

| On | What | Why, and what to do |
|---|---|---|
| 32-bit Windows | Native builds | No Rust for 32-bit Windows: Build makes the interpreted app (above). |
| 32-bit Windows | The JIT (machine code while a program runs) | Not built for 32-bit x86 yet: programs run in the interpreter, with the same answers. `RAPIDR_JIT=always` and `rapidr run --verbose` say "JIT: not on this kind of computer". |
| Windows XP | A screen reader | XP has no UI Automation, which RapidR's accessibility speaks. Windows 7 and later have it. |
| Windows XP | AI providers (the assistant, `RAI`) | XP's network security is too old for them (TLS 1.0, no SNI): they refuse the connection. An AI on the computer itself over `http://127.0.0.1` works. |
| Windows XP | Fonts installed in Windows, Chinese text | RapidR draws with its own fonts there (they cover Latin, Greek and Cyrillic, and the symbols above); a program that names a Windows font gets RapidR's nearest one. Chinese (and Japanese, Korean) text needs a font RapidR doesn't bundle: on XP it shows as boxes for now. |

## Checking which Windows a program runs on

`RSystemInfo.OS` is "Windows", and `RSystemInfo.OSVersion` says which one: "5.1 (2600)" on XP,
"6.1 (7601)" on Windows 7, "11 24H2 (26100)" on Windows 11. The example
`examples/system_info.rr` lists everything `RSystemInfo` knows.
