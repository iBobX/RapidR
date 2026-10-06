# The web

The same program runs in a browser. Its windows are drawn by the same UI
kernel as on the desktop, into a canvas on the page, so they look and
behave as they do on the desktop — windows you can move, resize, minimize
and maximize, the same fonts and pixels, high-DPI screens, keyboard and
screen-reader support (an ARIA mirror of the kernel's accessibility tree).
The page needs nothing but static web hosting: no server-side code, and
nothing contacts a third party unless the program does.

## Two ways to build for the web

| | `rapidr bundle-bc prog.bas` (or `build --web --interp`) | `rapidr build prog.bas --web` |
|---|---|---|
| What | the shared web interpreter + your program's bytecode | your program compiled to WebAssembly |
| Needs | nothing (the SDK ships the interpreter) | Rust, `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version 0.2.129` |
| Output | `prog-web.zip`: `index.html`, `rapidrintr.js` / `rapidrintr_bg.wasm`, `prog.rrbc`, `THIRD-PARTY-NOTICES.txt`, the fallback fonts | `prog_web/`: `index.html`, `prog.js`, `prog_bg.wasm`, `THIRD-PARTY-NOTICES.txt`, the fallback fonts |

```sh
rapidr bundle-bc form.bas -o form-web.zip
unzip form-web.zip -d form-web
python3 -m http.server -d form-web 8080      # or any static host
```

Serve the folder over HTTP(S) — browsers don't load WebAssembly from
`file://` pages. The web interpreter is about 10 MB (2.7 MB compressed with
brotli, which most hosts do for you).

## Files and assets

A browser page has no file system. RapidR gives a program one of its own:

- **Project files**: when you build, the files beside the program with the
  extensions `.csv`, `.db`, `.sqlite`, `.png`, `.jpg`, `.jpeg`, `.gif`,
  `.bmp`, `.txt`, `.wav` and `.mp3` are built into `index.html`, so the
  program reads them as on the desktop (`OPEN`, streams, pictures,
  `RDataFrame.LoadFromCSV`, `RSQLite.Connect`). Keep a web program in a
  folder of its own so that only its files go in. `$RESOURCE` files are
  built in too.
- **Files the program writes** live for the page's session: written, read
  back, listed by `FILEEXISTS`, deleted by `KILL`.
- **Other names** are fetched from the page's own server.
- Open / Save dialogs use the browser's file picker; printing opens the
  browser's print dialog.

## What runs differently in a browser

- A program's busy loop (a game loop without `DOEVENTS`, a long
  computation) doesn't freeze the page: the interpreter runs in time slices
  and what it draws shows as it runs. Events still run only when the
  program waits, as on the desktop.
- `SLEEP`, `ShowModal`, `INPUT`, message boxes and dialogs wait as they do
  on the desktop; `INPUT` asks in an input box when windows are shown, and
  `PRINT` output appears in a console on the page.
- **RSQLITE** is SQLite itself, compiled to WebAssembly; `Connect "x.db"`
  opens the project's `x.db` in memory for the session (changes aren't
  saved to the browser's storage yet).
- **QMYSQL** and **RSERVERSOCKET** don't work in a browser (no raw TCP);
  call a server of yours with RHTTP instead. **QSOCKET / RSOCKET** connect
  over WebSocket. **RHTTP** uses the browser's fetch, so the browser's CORS
  rules apply.
- **QCOMPORT** uses Web Serial, **QDXJOYSTICK** the Gamepad API, **QMIDI**
  Web MIDI (or RapidR's own synthesizer through Web Audio), sound Web Audio.
- **QREGISTRY** keeps its keys in a per-user store, as on macOS and Linux.
- `DECLARE … LIB` (native libraries) and `RUSTSTART` blocks aren't
  available in the interpreter.
- Text RapidR's built-in fonts can't draw (symbols, Chinese, Japanese,
  Korean, emoji) uses Noto fonts shipped with the build, loaded as the page
  needs them.

## Web-only components

These are the page's own HTML elements, placed by the kernel where their
components are (clipped to their parents, hidden with them):

| Component | |
|---|---|
| `RWEBVIEW` | an embedded web page or HTML (an iframe): `URL`, `HTML`, `Sandbox` |
| `RDOM` | an HTML element of your own: `InnerHTML`, `CssClass`, `CssStyle`, `SetAttribute`, `AddClass`, `QuerySelector`, … |
| `RJAVASCRIPT` | run JavaScript: `Eval(code)`, `Call(function, args…)` |
| `RWEBSTORAGE` | `localStorage` / `sessionStorage`: `Set`, `Get`, `Remove`, `Clear`, `Keys`, `HasKey` |
| `RWEBAUDIO`, `RWEBVIDEO` | HTML5 audio and video: `Src`, `Volume`, `Loop`, `Play`, `Pause`, `Stop`, `Seek` |
| `RWEBNOTIFICATION` | browser notifications: `RequestPermission`, `Show` |
| `RWEBGEOLOCATION` | the device's position: `GetPosition`, `Latitude`, `Longitude`, `Accuracy` |
| `RROUTER` | hash routes for single-page apps: `Navigate`, `Back`, `Forward`, `OnRouteChange` |

They exist only in browsers: a desktop program that uses them builds and
runs, but their methods do nothing there and print a warning.

## The web IDE

`rapidr-web-2.117.0.zip` is the web IDE: a folder for any static host
(open its `index.html` through a web server). It has a form designer, a
code editor (Monaco), Run (the program runs in a sandboxed frame of the
page), Build (downloads a bundle like `bundle-bc`'s) and a debugger. It
compiles in the browser; nothing is sent anywhere.
