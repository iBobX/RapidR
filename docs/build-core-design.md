# The build core: one packer for every target (design, 2026-11-07)

Lane B1 of [build-for-every-platform-plan.md](build-for-every-platform-plan.md)
(sections 3.1, 3.3, 3.4 and the CLI). The plan is the spec; this page says how
it is built.

## 1. The rule: bytes in, bytes out

`crates/rapidr-package` is the packer, and it is **pure**: no file system, no
processes, no clock, no network. Everything comes in as bytes and goes out as
bytes, so the same code runs in the CLI, in the desktop Studio (which runs the
CLI) and in the web Studio (compiled to `wasm32-unknown-unknown`).

```
program bytecode (.rrbc) ─┐
data files (path, bytes) ─┤
App info + icon (bytes) ──┼──> pack(target) ──> Files (the app's tree) ──> archive() ──> zip / tar.gz bytes
the runner's bytes ───────┤                                                   └──> sha256sums()
the runner's notices ─────┘
```

The callers do the I/O:

| Caller | Reads | Writes |
|---|---|---|
| CLI `rapidr build --interp` (one target, a folder) | the source, the data files, the icon, `runner_bytes(target)` | the tree into the output folder (as today: `Name.app`, `Name.exe`, `Name.AppDir`) |
| CLI `rapidr build --interp --targets …` / `--all` | the same, once; a runner per target | `Name-1.0.0-<target>.zip` (`.tar.gz` for Linux) per target, and `SHA256SUMS` |
| Desktop Studio | runs the CLI (unchanged) | |
| Web Studio (later lane, B5) | the browser's project store, runners fetched into Cache Storage | the browser's downloads |

**One implementation.** The old writers that touched the disk inside
rapidr-package are deleted: `macos::write`, `linux::write`, `linux::install`
(moved to the CLI's `install-app`), `Icon::load` (the CLI reads the file and
calls `Icon::from_bytes`). A folder build is the archive's tree written to
disk; nothing builds an app another way. The native build uses the same
`pack` with its own executable in place of a runner, and the same fat-Mach-O
writer in place of `lipo`.

## 2. Targets

| Target (`--targets`) | Also accepted | Runner (install's `runners/<dir>/`) | Archive |
|---|---|---|---|
| `windows-x64` | `windows-x86_64`, `win-x64` | `windows-x86_64` | `.zip` |
| `windows-arm64` | `windows-aarch64` | `windows-aarch64` | `.zip` |
| `windows-x86` | `windows-i686`, `win32` | `windows-x86` (no JIT: the fast interpreter) | `.zip` |
| `macos-universal` | `macos` | `macos` (one universal runner) | `.zip` |
| `linux-x64` | `linux-x86_64` | `linux-x86_64` | `.tar.gz` |
| `linux-arm64` | `linux-aarch64` | `linux-aarch64` | `.tar.gz` |
| `web` | | `web` (the web runtime) | `.zip` |
| `macos-arm64`, `macos-x64` | `macos-x86_64` | `macos`, one slice taken out | `.zip` (not in `--all`) |

`--all` is the first seven. **macOS universal, what was found:** RapidR's
release already ships *one universal runner* (`runners/macos/rapidrintr-runner`
is a fat Mach-O, arm64 + x86_64), so nothing has to be joined for it. The core
still has `macho::universal(&[slices])` (a fat Mach-O from thin ones, slices
aligned to 16 KiB as Apple's tools do), so a runner delivered as two thin files
works too, and the native build uses it instead of running `lipo`.

## 3. One runner per target: `runner_bytes(target)`

The CLI finds runners through ONE pair of functions
(`crates/rapidr-cli/src/runners.rs`); the RUNNERS lane adds the cache and the
verified download behind them, without changing the callers:

```rust
pub fn runner_bytes(target: Target) -> Result<Vec<u8>, String>;
pub fn runner_notices(target: Target) -> Result<String, String>;
```

- Today: the install's `runners/<dir>/rapidrintr-runner[.exe]` and
  `notices/<kind>.txt`; in a source checkout, this machine's runner built by
  cargo (as `locate_or_build_stub` did) and the notices made by `notices.rs`.
- **One file per target.** Windows' windowed runner (`rapidrintr-runnerw.exe`)
  is the console runner with one field changed: the PE optional header's
  `Subsystem` (3 console, 2 GUI). Checked on the 2.123.0 Windows 7 runners:
  the two files differ in exactly 4 bytes, the build time stamp, the checksum
  and the subsystem. So the core makes the windowed one
  (`windows::set_subsystem`), and a download is one file, not two.
- The web's "runner" is the web runtime: a plain tar (ustar, read by
  rapidr-archive) holding `rapidrintr.js`, `rapidrintr_bg.wasm` and `fonts/…`.
- The error, when there is none: the target's name in plain words and where it
  was looked for ("no runner for Windows (64-bit, x64) in this install …").

## 4. The App info

`AppInfo` (rapidr-package) and `.rrproj`'s `[build]` (rapidr-project), the
plan's section 2 "Your app":

| Field | `[build]` key | CLI | Used for |
|---|---|---|---|
| name | `app_name` | `--name` | title, Dock, Start menu, `.desktop` Name, web manifest name |
| version (default `1.0.0`) | `version` | `--app-version` | Info.plist, VERSIONINFO, `.desktop`, file names |
| company | `company` | `--company` | Windows CompanyName, the copyright default |
| copyright | `copyright` | `--copyright` | NSHumanReadableCopyright, LegalCopyright (default `© <company>`) |
| description | `description` | `--description` | FileDescription, `.desktop` Comment, web page and manifest |
| website | `website` | `--website` | README.txt, Windows `Comments` |
| icon | `icon` | `--icon` | every size (ico 16–256, icns 16–1024, PNG 16–512, web 32/180/192/512) |
| app id | `bundle_id` | `--bundle-id` | CFBundleIdentifier, the Linux desktop id, the web manifest `id` |
| category | `category` | `--category` | LSApplicationCategoryType, `.desktop` Categories |
| kind | `kind` (`window`/`console`, empty = from the program) | `--kind` | Windows subsystem, `Terminal=`, a console tool is a plain executable on macOS and Linux |
| theme colour | `theme_color` | `--theme-color` | the web manifest and the page's `theme-color` |

Warnings (never errors): an icon smaller than 512 px, an icon that is not
square (it is centred on a transparent square).

## 5. What each archive holds

Names: `<Name>-<version>-<target>.zip` (`.tar.gz` for Linux), `Name` made a
file name (`exe_name_for`). No top folder in a zip (Windows' Extract All and
macOS' Archive Utility make one); a top folder in the tar.gz.

| Target | Inside |
|---|---|
| Windows | `Name.exe` (runner + icon + version + an `asInvoker` manifest when it has none + the program appended), the data files beside it, `THIRD-PARTY-NOTICES.txt`, `README.txt` |
| macOS | `Name.app` (Info.plist, PkgInfo, `MacOS/<exe>` 0755, `Resources/AppIcon.icns`, `Resources/<exe>.rrbc`, the data files and `THIRD-PARTY-NOTICES.txt` in Resources, signed ad hoc), `README.txt` |
| Linux | `Name-1.0.0-linux-x64/`: the AppDir's contents (`AppRun` 0755, `usr/bin/<exe>` 0755, the desktop entry, hicolor icons 16–512 and the SVG), `install.sh` 0755, `THIRD-PARTY-NOTICES.txt`, `README.txt` |
| Web | today's bundle (`index.html` … `fonts/`), plus `manifest.webmanifest`, `icons/` (32, 180, 192, 512 px), `README.txt` |

A folder build writes the same tree without `README.txt` (a single file is
built beside its program: a README there could be the user's own).

Exec bits: rapidr-archive's writer gains `add_executable` (zip: Unix mode
0755 in the external attributes; tar: mode 0755). Archive Utility and `tar`
keep it, so the unpacked app runs.

Determinism: the same inputs make the same bytes (entries sorted, the caller's
`mtime`, no clock inside), so `SHA256SUMS` is reproducible.

## 6. macOS signing: the call site

An Apple-silicon Mac runs no unsigned arm64 code, so every macOS app is signed
ad hoc. The MACHO-SIGN lane wrote the signer (`macho_sign.rs`, pure,
[docs/macho-signer-design.md](macho-signer-design.md)); its one call site is
`macos::sign_ad_hoc(files, app_root, info)` inside `pack`, so folder builds
and archives, on every system and in the browser, come out signed. The
`codesign` bridge the CLI had until then is deleted (one implementation).

## 7. Security

- A data path is refused with a clear message when it is absolute, has `..`,
  a drive, a device name or a control character (rapidr-archive's
  `safe_path`), or collides with the app's own files.
- Links are never followed: the CLI reads data files with
  `symlink_metadata` first and refuses a link.
- The program's source never goes in: data files with a source extension
  (`.rr`, `.bas`, `.inc`, `.rqb`, `.rqw`, `.rrproj`) are left out with a note,
  unless the source is asked for (`--include-source`: the files under
  `source/` in the archive, never in a folder build).
- Every name the user gives is escaped where it lands: XML (Info.plist), the
  desktop entry's rules, the shell (`install.sh` single-quotes every value),
  JSON (the web manifest), HTML (the page's head, through rapidr-webbundle's
  escaper).
- A runner for the wrong machine is refused (PE machine, ELF e_machine,
  Mach-O CPU types), so a mixed-up download can't make a broken app.

## 8. The AI facet

`build_app(targets, metadata)` in the shared tool set (rapidr-studio's
toolset.rs): handled by Studio (it saves the App info into the project, then
runs the CLI with `--targets`), offered to RAI and AI apps alike, permission
class `run`.

## 9. Tests and budgets

- Per format: Info.plist (golden), PE resources (icon, VERSIONINFO, manifest,
  subsystem), ELF/Mach-O/PE machine checks, the AppDir and desktop entry,
  `install.sh` quoting, the web manifest, zip and tar modes, `SHA256SUMS`.
- Determinism (same input, same bytes), bad data paths refused, the wrong
  runner refused.
- `cargo build --target wasm32-unknown-unknown -p rapidr-package`.
- Timing: one target under 2 s, all seven under 10 s with runners local.
