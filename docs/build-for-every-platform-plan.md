# Build for every platform: one click, every system, from any Studio (plan, 2026-11-02)

Robert, 2026-11-02: "when you click Build, it should give us a popup where we specify if we want mac, windows, linux, or the web, also x86 or x64 ... the options that we will pack, including custom icon, descriptions, all the meta-data, and then we compile our super fast bytecode with the platform-specific runtime, zip, and WOW! We give the user the zip with the selected platform executable. ... We can extend in the future, for mobile."

This moves "Export anywhere" ([lean-builds-and-export-plan.md](lean-builds-and-export-plan.md) §3, planned for 1.0) **into the speed release**. It ships together with the fast runtime, so every app it builds is a fast one. The decisions in that plan still hold, and this page turns them into the feature.

**The WOW in one line:** *Write it once. Click Build. Get your app for Windows, macOS, Linux and the web, from any computer, even from a browser.*

## 1. What exists today (checked 2026-11-02)
- `rapidr build --interp --target <os>-<arch>` already makes a standalone app from **bytecode + a prebuilt runner**, with `--icon`, `--name`, `--bundle-id`, `--app-version` and `--company`. A GUI program becomes `Name.app` on macOS, an `.exe` with its icon and version on Windows, and `Name.AppDir` on Linux.
- **Each install ships only its own system's runner** (the Mac app has `runners/macos` only, 68 MB). Building for another system says "no runner for … in this install". So cross-building needs the other runners, fetched on demand.
- The web build exists (`bundle-bc`, a zip for any static host).
- The web Studio can't build desktop apps at all.
- Studio has a Run > Build command with no dialog.

So the core already works. What's missing is the **dialog**, the **runners for every target, fetched and verified on demand**, **packaging done in-process** (so the browser can do it too), **macOS signing off a Mac**, and the **32-bit Windows runner** in the set.

## 2. The Build dialog (Studio, desktop and web: one design)
Mockup in Pen first (design guide, light/dark/high contrast, EN/ES). A dialog with a left list of steps, or one scrolling page on small screens:

**1. Where should it run?** Target cards with icons (our own SVG set). Each card can be ticked and shows the choices that really exist:

| Card | CPU choices | Notes shown on the card |
|---|---|---|
| Windows | 64-bit (x64), ARM64, 32-bit (x86: Windows 7, 8, 8.1 and 10/11 32-bit) | "32-bit: no JIT; the fast interpreter" |
| macOS | one **Universal** app (Apple silicon + Intel) | Mac has no 32-bit; one app runs on both |
| Linux | x64, ARM64 (also Raspberry Pi 4/5 64-bit) | `.tar.gz`; an AppImage later |
| Web | — | a zip for any web host, or "run it here" |
| *Coming later* (greyed) | Android, iOS, boards | so users see the direction |

- "All desktop" and "Everything" quick choices.
- Plain words: "x64 = most PCs since 2005", "ARM64 = Apple silicon, Snapdragon PCs, Raspberry Pi".

**2. Your app** (saved in the project, `.rrproj`, so it's filled in once):
- **Name** (shown in the title, Dock, Start menu), **version** (1.0.0), **company/author**, **copyright** (prefilled "(c) 2026 <author>"), **description** (one line; Windows' file description, macOS' Get Info, Linux's .desktop Comment, web page description), **website**.
- **Icon:** one picture (PNG 1024×1024 or SVG), with a live preview of every size. We generate `.ico` (16–256), `.icns` (16–1024 @1x/@2x), the Linux PNGs (48–512) and the web favicon + manifest icons. A warning appears if the picture is small or not square.
- **App id** (`com.company.app`, derived from the company and name, editable; macOS bundle id, Linux app id, web manifest id).
- **Category** (Productivity, Education, Games, Developer tools…; macOS LSApplicationCategoryType, Linux Categories).
- **Kind:** windowed app or console app (detected from the program, can be changed).
- *Advanced (folded):* file types it opens (later), minimum system version, include the source? (default no), the language list.

**3. What goes in:** the program's data files (`data_files_of`), each with a tick box, and the files left out with the reason (the #14/#16 fixes make this list right). It shows each target's runner flavour and size: "Windows x64: 21 MB · macOS: 38 MB …" (lean flavours in phase 2).

**4. Build:** one progress list per target ("Windows x64: runner verified ✓ · packed ✓ · zipped ✓"), then:
- **Desktop Studio:** "Show in folder" plus one zip per target (`MyApp-1.0.0-windows-x64.zip` …), or "All in one zip".
- **Web Studio:** the browser downloads each zip (or one zip), and **nothing leaves the user's computer**: the packing runs in the page.
- A `SHA256SUMS` beside them, and a `README.txt` per zip in the app's language: how to open it the first time on that system (SmartScreen "More info > Run anyway"; macOS "Open Anyway"; Linux `chmod +x`).
- "Build again" uses the saved choices (Ctrl+Shift+B), with no dialog.

**Native (compiled) builds:** an option only in the desktop Studio, folded under Advanced: "Native code (needs Rust; this computer's system only)". It stays as today until the JIT is measured against native, then Robert decides ([[old-pcs-plan]] open question). On 32-bit Windows there's no native option.

## 3. How it works inside
One packer, written once in Rust, used by the CLI, the desktop Studio and the web Studio (compiled to wasm):
1. **Compile once:** the program becomes `.rrbc` bytecode plus resources. This is the same for every target, so it's done once.
2. **Get the runner** for each target:
   - from the install, if it's there;
   - otherwise download it from **rapidr.dev/runners/<version>/<target>.zst** (CORS on; not GitHub release links, which have no CORS);
   - verify the **SHA-256 from a manifest signed with our minisign key**, which is shipped inside Studio (desktop and web), before using it;
   - cache it (`~/Library/Caches/RapidR/runners/<version>/`; in the browser, IndexedDB/Cache Storage);
   - the runner's version always equals Studio's: a 2.123 Studio uses 2.123 runners, never older or newer.
3. **Pack per target:**
   - **Windows:** write the icon, version info (name, version, company, copyright, description) and manifest into the runner's resources with **our own PE resource writer** (or a permissive crate after the lawyer's review), then append the program (bytecode + data) to the `.exe`. Runners are not signed (legal decision 7c), so appending breaks no signature. The result is **one `.exe`**, plus a folder only if the user ticked loose data files.
   - **macOS:** build `Name.app` (Info.plist with name, id, version, category and copyright; `.icns`; the program inside the bundle's Resources, since macOS moves downloaded apps to a read-only random place). Then **sign it ad-hoc with our own small signer** (a code directory of SHA-256 page hashes, no certificates, written from public descriptions; legal review 2026-10-22 7b). **This is required:** an Apple-silicon Mac refuses to run an unsigned arm64 binary at all. It works the same from Windows, Linux or a browser. A Developer ID signature, for a "no warning" app, is made later on a Mac with the user's own account (the manual explains).
   - **Linux:** `name` (runner + appended program) + `name.desktop` + icons + `install.sh` (to `~/.local`, like our own), in a `.tar.gz` that keeps the exec bit (the zip loses it; Linux gets tar.gz).
   - **Web:** today's bundle, plus a web manifest (name, icons, theme colour), so it installs as an app (PWA) where browsers allow.
4. **Zip** in-process (our RArchive code) and write `SHA256SUMS`.

Every build includes `THIRD-PARTY-NOTICES.txt` for that runner.

## 4. Speed (the release's theme)
- The runners carry the fast interpreter, and the JIT on x64 and ARM64. On 32-bit x86 Windows, the fast interpreter only: no Cranelift backend.
- **Lean flavours** (console / gui / gui+data / full) cut size and start time. Phase 2, if the release fills up. Phase 1 uses the full runner.
- Build time targets: one target in under 2 s on the Mac, all six in under 10 s (runners cached); in the web Studio, under 5 s per target after the first download.
- First download: about 15–40 MB per runner, zstd-compressed, with a progress bar. Cached after that.

## 5. Security and legal
- Runners come only from our host and are verified against the minisign-signed manifest; a mismatch stops the build with a clear message. No user code goes to any server, and the web Studio packs in the page.
- **Never sign users' apps with Ruta's identity** (7c). Export runners stay unsigned on Windows and Linux, ad-hoc on macOS. Users sign with their own certificates; the manual explains how, later.
- **Lawyer agent review before merging:**
  - our PE resource writer and Mach-O ad-hoc signer (clean, from public format descriptions), or any crate used instead;
  - the per-runner notices;
  - Apple: no Apple SDK is used, and ad-hoc signing needs no Apple software or licence;
  - the wording of the first-launch README texts.
- The packer rejects data paths with `..`, absolute paths and links (zip-slip in reverse).

## 6. Tests (seen working, test like a user)
- **The matrix:** the same 5 example apps (hello form, tip calculator, CSV explorer, a game with sound, a console tool), built for every target **from the Mac Studio, from the web Studio, and from the Windows VM's Studio**.
- **Each built app is opened by hand** on: macOS (Apple silicon, from a downloaded zip, so quarantine is on), Windows 11 ARM VM (ARM64 and x64), the Windows 7 32-bit VM (x86), Ubuntu VM (ARM64; x64 under emulation), and Chrome/Safari/Firefox for the web.
- Checked in each: the icon in the Dock/taskbar/Finder/Explorer, the name and version in Get Info and Properties, the description and copyright, that the data loads, and that the first-launch README steps are right.
- **Automated:** the packer's unit tests per format; a golden check of the PE resources, Info.plist and code directory; a corrupted runner refused; runner/Studio version mismatch refused; the size and time budgets.
- Accessibility: the dialog fully usable with the keyboard and VoiceOver/Narrator.

## 7. Definition of done
- A manual page "Build your app for every system" (screenshots, first-launch help per OS), in EN + ES.
- Help for the dialog and the CLI (`rapidr build --interp --target …` plus `--all`).
- An AI facet: an MCP tool `build_app(targets, metadata)` so RAI and agents can build too ("build it for Windows and the web").
- `examples/`: a sample project with a full App info filled in.
- A video: "One click, six systems", made from a real run.

## 8. Lanes (after the Thu 11-05 reset; models per [[agent-model-policy]])
| # | Lane | Model, size |
|---|---|---|
| B1 | The packer crate: per-target packing (PE resources, .app + Info.plist + icns, Linux tar.gz + .desktop, web manifest), icon generation, zip; CLI `--all`, metadata from `.rrproj` | Opus L |
| B2 | The ad-hoc Mach-O signer (own code), tested on Apple silicon from a non-Mac build | Opus M |
| B3 | Runners for every target in the release chain (adding windows-x86), the signed manifest, rapidr.dev hosting with CORS; download + verify + cache in the CLI and Studio | Sonnet M |
| B4 | The Build dialog in Studio (Pen mockup first, design guide, EN/ES, a11y), the saved choices, Build again | Opus M |
| B5 | The web Studio: the packer in wasm, runner fetch into Cache Storage, the downloads | Opus M |
| B6 | The test matrix in the VMs + the manual + the example + the video | Sonnet M + coordinator |
| L | The lawyer review (B1/B2 formats, notices, Apple, README texts) | Sonnet S |

Order: B1 and B3 first (the core, in the CLI); then B2 and B4; then B5; then B6. Phase 2 (if room): lean runner flavours, AppImage, single-file web build.

## 9. Later (shown greyed in the dialog)
- **Android:** an APK runner (our UI kernel already draws everything). Signing with a debug key, or the user's own.
- **iOS:** needs a Mac, Apple's tools and the user's own Apple account, by Apple's rules. Desktop Studio on a Mac only.
- **Boards** (ESP32, micro:bit): through Preview 6/7's board work.
