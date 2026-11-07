# Build your app for every system

Write your program once, and RapidR makes it into an app for Windows, macOS,
Linux and the web, all from the computer you are on. Each app is a zip (a
`.tar.gz` for Linux) that you can give to people: they unpack it and open
the app. You don't need the other computers, a compiler or anyone's SDK.

```sh
rapidr build tips.rr --targets windows-x64,macos,linux-x64,web
rapidr build Tips.rrproj --all            # every system RapidR builds for
```

The output folder (the project's `build` folder, or the program's own
folder) then holds:

```text
Tip-Calculator-1.2.0-windows-x64.zip
Tip-Calculator-1.2.0-macos-universal.zip
Tip-Calculator-1.2.0-linux-x64.tar.gz
Tip-Calculator-1.2.0-web.zip
SHA256SUMS
```

`SHA256SUMS` lists each file's SHA-256, so people can check that what they
downloaded is what you built (`sha256sum -c SHA256SUMS` on Linux,
`shasum -a 256 -c SHA256SUMS` on a Mac). Building again for one system
updates its line and keeps the others.

## The systems

| `--targets` | What it makes | Runs on |
|---|---|---|
| `windows-x64` | `Name.exe`, with its icon and version | 64-bit Windows 10 and 11, most PCs |
| `windows-arm64` | the same | Windows PCs with an ARM processor (Snapdragon and others) |
| `windows-x86` | the same, with the fast interpreter (no JIT) | 32-bit and 64-bit Windows 7, 8, 8.1, 10 and 11 |
| `macos` (`macos-universal`) | `Name.app`, one app for both kinds of Mac | Apple silicon (macOS 11 and later) and Intel Macs (macOS 10.13 and later) |
| `linux-x64` | `Name/`: the app, its menu entry and icons, `install.sh` | 64-bit Linux on PCs |
| `linux-arm64` | the same | 64-bit Linux on ARM, Raspberry Pi 4 and 5 too |
| `web` | the page, the runtime and your program | any static web host, every modern browser |

`--all` builds the first seven. `macos-arm64` and `macos-x64` make a smaller
Mac app for one kind of Mac only.

`--targets` makes interpreted apps: each system's runner, the program that
runs RapidR's bytecode (with the JIT on 64-bit systems), and your program.
That is why one computer can build for all of them.

## What each zip holds

- **Windows:** `Name.exe`, your program's data files beside it,
  `THIRD-PARTY-NOTICES.txt` and `README.txt`.
- **macOS:** `Name.app` (your data files and the notices are inside it) and
  `README.txt`.
- **Linux:** one folder, `Name-1.0.0-linux-x64/`, with `AppRun` (run it to
  start the app), `install.sh` (puts the app in the applications menu, for
  you alone, no root needed; `./install.sh --uninstall` takes it out
  again), the app, its desktop entry, its icons from 16 to 512 pixels, the
  notices and `README.txt`. The `.tar.gz` keeps the "executable" mark, which
  a zip would lose.
- **The web:** the page and everything it needs, plus `manifest.webmanifest`
  and `icons/`, so browsers that install web apps can install it.

`README.txt` tells the person who gets the app how to open it the first time
on their system. It is in English, or in Spanish with `--language es` (or
`language = "es"` in the project).

## Your app's details

Every system shows the same details, from the project (`.rrproj`, `[build]`)
or the command line:

| Option | Project setting | What it is | Default |
|---|---|---|---|
| `--name` | `app_name` | The name in the title, the Dock, the Start menu | the project's name |
| `--app-version` | `version` | Up to four numbers: `1.2.0` | `1.0.0` |
| `--company` | `company` | Who makes it | none |
| `--copyright` | `copyright` | The copyright line | `© <company>` |
| `--description` | `description` | One line about the app (Windows' description, the Linux menu's comment, the web page's) | none |
| `--website` | `website` | Its web site, `https://…` (in the README) | none |
| `--icon` | `icon` | One picture: a PNG of 1024 × 1024 or an SVG; every size is made from it | RapidR's |
| `--bundle-id` | `bundle_id` | The app's id, `com.example.tips` | `dev.rapidr.app.<name>` |
| `--category` | `category` | utilities, productivity, education, games, developer-tools, graphics, music, video, business, finance or science | utilities |
| `--kind` | `kind` | `window` (an app with windows) or `console` | what the program is |
| `--theme-color` | `theme_color` | The web app's colour, `#RRGGBB` | the page's |
| `--include-source` | | Your program's source in the zips, under `source/` | not included |

```toml
[build]
app_name = "Tip Calculator"
version = "1.2.0"
company = "Example Ltd"
description = "Splits the bill"
website = "https://example.com/tips"
icon = "art/tips.svg"
category = "finance"
```

RapidR warns when the icon is small (under 512 pixels: large sizes look
blurry) or not square (it is centred on a transparent square).

## Safe by design

- Only the data files your program names go in, never a file it writes
  (your own copy may hold your data), and never your source unless you ask
  with `--include-source`.
- A data file's path can't leave the app's folder (`..`, absolute paths,
  drives), and links are never followed.
- A runner for the wrong system is refused, so a mixed-up file can't make a
  broken app.
- With `SOURCE_DATE_EPOCH` set (the files' time, in seconds since 1970),
  the same program and details make the same zip, byte for byte; without
  it, the files carry the time of the build.

## Opening it the first time

Apps you build are not signed with a certificate, so each system asks once:

- **Windows:** "Windows protected your PC". Click **More info**, then
  **Run anyway**.
- **macOS:** the app is signed ad hoc, which Apple silicon needs, but not
  with an Apple Developer ID. RapidR makes that signature itself, on
  whichever system builds the app: Windows, Linux or a Mac. macOS says
  “Name” Not Opened (*Apple could not verify “Name” is free of malware…*).
  Click **Done**, then in **System Settings > Privacy & Security** click
  **Open Anyway** and confirm with your password or Touch ID.
- **Linux:** nothing to confirm. If the files lost their "executable" mark,
  `chmod +x AppRun install.sh`.

The README in each zip says this in plain words. Signing apps with your own
certificate, so that nothing is asked, is up to you (`signtool` on Windows;
`codesign` with your Developer ID and notarization on a Mac).
