# Building apps and their icons

`rapidr build` makes a program into a proper app for its system, with an
icon: the one you give it, or RapidR's own for compiled programs. RapidR
Studio's **Run > Build** does the same. A build is a release build —
optimized, what you ship (`--debug` makes a quick one for debugging) — and
its output folder gets only the app: the Rust a native build generates and
cargo's files stay in RapidR's build cache ([The CLI](cli-and-runtime.md#native-builds)).

| System | A program with windows | A console program |
|---|---|---|
| macOS | `Name.app`: the program in `Contents/MacOS`, its icon (`.icns`, every size from 16 to 1024 pixels), `Info.plist` (name, identifier, version, minimum macOS, high-resolution), signed ad hoc so it opens on Apple silicon | `prog`, a plain executable (`--bundle` makes an app of it too) |
| Windows | `prog.exe` with its icon (16 to 256 pixels, every size Explorer shows) and its version information (product name, version, company, description) | the same |
| Linux | `Name.AppDir`: the program, `AppRun`, a desktop entry and the icon from 16 to 512 pixels (the layout AppImage packs) | `prog`, a plain executable (`--bundle` makes an AppDir of it too) |

A program "has windows" when it says `$APPTYPE GUI`, or says no `$APPTYPE`
and creates forms or other components (RapidQ's rule). Native builds and
interpreted builds (`--interp`) make the same apps. Cross builds do too:
`rapidr build prog.bas --interp --target windows-x86_64` on a Mac writes the
`.exe`'s icon and version information; only signing a macOS app needs a Mac.

```sh
rapidr build notepad.bas --interp          # macOS: notepad.app
rapidr build notepad.bas --interp --icon art/notepad.png --name Notepad --app-version 2.1
rapidr build Notepad.rrproj                # the project's main file, with its settings: build/Notepad.app
rapidr build notepad.bas --no-bundle       # just the executable, no icon
rapidr build notepad.bas dist --keep-rust  # dist/notepad.app and dist/notepad-rust-source/
```

## Do I need Rust?

Not to build an app. There are two kinds of build, and the first needs
nothing installed beyond RapidR:

| | Interpreted | Native |
|---|---|---|
| Needs Rust | no | yes (free; `rapidr setup` installs it) |
| What the app is | the RapidR runner with your program's bytecode | your program compiled to machine code |
| Build time | seconds | minutes the first time |
| Speed | fine for most programs | faster, for heavy loops and games |

Both make the same app, with the same icon and information. Start with
interpreted; choose native when a program needs the speed.

```sh
rapidr build notepad.bas --interp   # no Rust needed
rapidr build notepad.bas            # native: needs Rust
rapidr setup                        # installs Rust (asks first); rapidr setup --check only looks
```

A native build on a computer without Rust stops at once and says so: "Native
builds need Rust (it is free). Run `rapidr setup` to install it, or build
without it: add --interp". It changes nothing and leaves nothing half built.

## The app's details

| Option | Project setting (`.rrproj`, `[build]`) | What it is | Default |
|---|---|---|---|
| `--icon <file>` | `icon` | The icon: `.icns`, `.ico`, `.png` or `.svg` | the program's `$OPTION ICON`, else RapidR's |
| `--name <name>` | `app_name` | The name Finder, Explorer and the applications menu show (and the `.app`'s) | the project's name, else the source file's |
| `--bundle-id <id>` | `bundle_id` | A reverse-DNS identifier, `com.example.notepad` (macOS' bundle ID, the Linux desktop entry's name) | `dev.rapidr.app.<name>` |
| `--app-version <v>` | `version` | Up to four numbers with dots: `1.0`, `2.3.1` | `1.0` |
| `--company <name>` | `company` | Who makes it: Windows' company name and the copyright line | none |
| `--bundle` / `--no-bundle` | | An app even for a console program / only the executable | |
| `--project <file.rrproj>` | | The project whose settings to use | the `.rrproj` beside the source that names it as its main file |
| `[output folder]`, `--output <folder>` | `output` | Where the app goes (a folder of the project's) | the project's `build`; without a project the source's folder |
| `--keep-rust` / `--no-keep-rust` | `keep_rust` | A native build also leaves its generated Rust in `<output>/<program>-rust-source` (with a README saying what it is) | deleted |

The command line wins over the project, and the project over the source.
A project file carries them like this:

```toml
[build]
icon = "art/notepad.svg"
app_name = "Notepad"
bundle_id = "com.example.notepad"
version = "2.1"
company = "Example Ltd"
targets = ["bytecode"]      # Studio's Build: interpreted ("native" compiles with Rust)
output = "dist"             # the output folder ("build" when it isn't said)
keep_rust = true            # the generated Rust beside the app (not kept when it isn't said)
```

### The app's name

The name Finder, Explorer and the applications menu show (and, on a Mac, the
`.app`'s file name) is the source file's (`notepad.bas` makes `notepad.app`),
unless the project or `--name` gives another one:

```sh
rapidr build 3dcube.bas --interp                 # 3dcube.app (macOS)
rapidr build 3dcube.bas --interp --name "3D Cube"    # "3D Cube.app"
```

- A name can be anything that can be a file name, **including one that
  starts with a digit** (`3dcube`, `2048`, `8ball`). Only `/`, `\` and `:`
  are refused, because on a Mac and in a Linux AppDir the name is also a
  folder's.
- The bundle ID (macOS) and the Linux desktop entry's name come from the
  name: everything but letters and digits becomes a hyphen, in lower case,
  so "3D Cube" is `dev.rapidr.app.3d-cube`. A name with no letters or digits
  at all (Japanese, for example) gets `dev.rapidr.app.program`: give it a
  `--bundle-id` of your own (`com.example.cube`).
- On a Mac, when you check a signature by hand with `codesign --verify`,
  write `./3dcube.app` (or the full path) for a name that starts with a
  digit: `codesign` takes `3dcube.app` for a process number and answers
  "No such process". Older versions of `rapidr build` stopped with that
  message for such a name; now `rapidr build` hands `codesign` the full path
  and signs and checks the app.

## Icons

Where the icon comes from, the first one there wins:

1. `--icon` on the command line;
2. the project's `icon`;
3. `$OPTION ICON "file"` in the program — RapidQ's own way, which also gives
   the program's windows that icon;
4. `-g<file.ico>`, RapidQ's compiler option (`RC -gfile.ico prog.bas`):
   RapidR takes it too, below `$OPTION ICON` as RC.EXE does;
5. RapidR's icon for compiled programs.

Any of these formats works, and RapidR draws every size each system needs
from it:

| Format | Best as |
|---|---|
| `.svg` | Sharp at every size. Text in the SVG must be converted to outlines (RapidR draws shapes, not fonts, in icons) |
| `.png` | 1024 × 1024 pixels, square (another shape is centred on a transparent square) |
| `.icns` | macOS' own: its PNG pictures are used |
| `.ico` | Windows' own: every picture in it, PNG or BMP (RapidQ's 766-byte 32 × 32 icons too) |

A picture smaller than 512 pixels works, but the larger sizes are scaled up
and look soft: the build says so. A file that isn't a picture RapidR can
read stops the build with the reason, before anything is compiled.

RapidR's own icon for programs is the Runtime's dark tile with an app window
and the amber Run triangle (`design/brand/icons`: `RapidR-App.icns`,
`rapidr-app.ico`, `apps/rapidr-app` in the hicolor tree).

### `$OPTION ICON`, as RapidQ has it

`$OPTION ICON "path\file.ico"` works as in RapidQ's compiler: the name may
be quoted or not; a file that isn't there is a compile error
(`ICON file nowhere.ico does not exist.`); with several, the last one wins.
RapidQ looks for the file in the current folder; RapidR also looks beside
the source. RapidQ only takes a 766-byte (32 × 32, sixteen-colour) icon;
RapidR takes any of the formats above.

## macOS

- The app is signed ad hoc (`codesign --sign -`): it opens on the Mac that
  built it, and Apple silicon Macs run it. To give it to others, sign it with
  your Developer ID and notarize it (`codesign`, `xcrun notarytool`); a
  downloaded unsigned app is allowed once in System Settings > Privacy &
  Security > Open Anyway (see [Install on macOS](getting-started.md#install-on-macos)).
- An interpreted app keeps the program's bytecode in
  `Contents/Resources/<name>.rrbc` (data after a signed executable would
  break its signature).
- `THIRD-PARTY-NOTICES.txt` is in `Contents/Resources`.
- `LSMinimumSystemVersion` is 10.13: macOS High Sierra on Intel, macOS 11
  on Apple silicon.

## Windows

The icon and version information are written into the `.exe` after it is
built (pure Rust: no resource compiler, no Visual Studio), so Explorer, the
taskbar and Properties > Details show them. Code signing with a certificate
is up to you (`signtool`).

## Linux

The AppDir runs where it is (`Notepad.AppDir/AppRun`). To put the app in
your applications menu with its icon:

```sh
rapidr install-app Notepad.AppDir
```

writes its desktop entry and icons under `~/.local/share` (the menu starts
it from where the AppDir is: install it again after moving it). Tools such
as appimagetool pack an AppDir into one AppImage file.

## Build a web app

A web app is your program and the RapidR web runtime in one `.zip`. Unzip it
on any web host (GitHub Pages, a shared host, an `nginx` folder) and the
program runs in the visitor's browser, with nothing to install. It is a
static site: no server program is needed.

```sh
rapidr build hello_form.rr --web --interp    # hello_form-web.zip
rapidr bundle-bc hello_form.rr -o site.zip   # the same, named as you like
```

**In RapidR Studio**, choose **Run > Build Web App** (on the web page:
**Run > Build Web App (.zip)**, Ctrl+Shift+B). The program is saved, checked
and compiled; the Output panel's **Build** page lists what goes in, and ends
with "✓ Built hello_form-web.zip — unzip on any web host". On the desktop the
zip is written to the project's output folder (`build` beside the project
file) and the last line has **Reveal in Finder** (File Explorer, Files); on
the web page the browser downloads it. The zip is named after the project
(`Notes-web.zip` for `Notes.rrproj`), else after the program.

![RapidR Studio on the web: the Run menu lists Run, Run Without Debugging, Stop, Restart, Run in Browser and Build Web App (.zip), with Ctrl+Shift+B](images/build-web/run-menu.png)

![The Output panel's Build page after Build Web App: the files that go in, "Saved Notes-web.zip", and the green line "Built Notes-web.zip — unzip on any web host"](images/build-web/build-page.png)

What is in the zip:

| File | What it is |
|---|---|
| `index.html` | the page: it names the program and carries its Content-Security-Policy, which allows only what the program uses (a program that never uses `RHTTP` can't reach a server) |
| `<program>.rrbc` | your program, compiled |
| `rapidrintr.js`, `rapidrintr_bg.wasm`, `loader.js` | the web runtime and what starts it |
| `rapidr-assets.js` | the project's own files (a CSV the program loads, pictures), when there are some |
| `fonts/` | the fallback fonts, loaded only for characters the built-in fonts lack |
| `THIRD-PARTY-NOTICES.txt` | the open-source notices that travel with the runtime |
| `_headers`, `.htaccess` | the same safe headers, for hosts that read them |

To try it before you upload, unzip it and serve the folder:
`python3 -m http.server -d hello_form-web 8000`, then open
`http://127.0.0.1:8000/`. (Opening `index.html` from the disk doesn't work:
browsers don't let a page load WebAssembly from `file://`.)

## In RapidR Studio

- **Run > Build** (Ctrl+Shift+B, and the Build button on the tool bar)
  builds the project's app the way **Project > Project Options** says
  (**Build**: Compiled or Interpreted). **Run > Build Native App** and
  **Run > Build Interpreted App** build it that way once, whatever the
  project says. All are release builds
  for the system Studio runs on, into the project's output folder
  (`build` beside the project file, or the one Project Options names). What
  `rapidr build` prints goes to the Output panel's **Build** page as it
  comes: the app's path and the kept Rust source are links, and the last
  line, "✓ Built Notepad.app (interpreted) in 12 s", has **Reveal in Finder** (File
  Explorer, Files) — a click shows the app selected. A build that fails says
  so in red, after cargo's errors. Run (F5) is the one for debugging: it runs
  the program under Studio's debugger, no build needed.
- **Without Rust**, Build Native App asks first: "Native builds need Rust
  (it is free)." **Build Interpreted Instead** (Enter) makes the
  interpreted app now; **Install Rust...** runs `rapidr setup` and shows what
  it says on the Build page (when it ends, native builds work); **Cancel**
  does nothing. A new project made on a computer without Rust is set to
  build interpreted, and Project Options says "(Rust not installed)" beside
  the Compiled choice.
- **Run > Reveal in Finder** shows the last app built.
- **Project > Project Options** sets the app's name, bundle ID, version,
  company and icon (with a preview, as the app will have it); whether
  Build compiles it natively or makes it interpreted (no Rust needed); the
  output folder; and **Keep the generated Rust source** (off by default:
  the Rust is deleted after the build; on, it is left in
  `<output>/<program>-rust-source` to read). They are saved in the project
  file.
- **Run > Build Web App** makes the program a web app (see
  [Build a web app](#build-a-web-app)). On the web page this is the Build
  command: **Run > Build Web App (.zip)**. The web page doesn't make apps for
  a computer (Build Native App, Build Interpreted App, Reveal): those are in
  RapidR Studio on the desktop.

### RapidR Studio as an app, from a source checkout

```sh
tools/studio_app.sh            # target/RapidR Studio.app
open "target/RapidR Studio.app"
```

makes RapidR Studio an app with its icon, running this checkout's Studio
(the release apps come from `tools/release/macos.sh`).
