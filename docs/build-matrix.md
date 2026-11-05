# The build matrix: "everybody builds, no issues"

Lane BUILD-MATRIX, 2026-11-05. One command shows, before a release is published,
whether a person on **every system RapidR ships** can install the package, press
**Build (interpreted)** and **Build Native**, and get a program that runs.

```
tools/release/build-matrix.sh <ver> [--quick] [--only mac,linux,windows,win7] [--shots] [--serial]
```

It uses the finished files in `dist/<ver>/out/` (checked against `SHA256SUMS` first, in
the Linux VM and the Windows VM too) and the smoke tests the release already had
(`tools/release/smoke.sh`, `windows/smoke.ps1`), plus one for the Windows 7 zip
(`windows/smoke-win7.ps1`). It prints one table, writes it to `dist/<ver>/build-matrix.txt`
(a run of some lanes: `build-matrix.partial.txt`), and keeps every log in
`dist/<ver>/build-matrix/logs/`. **Exit status 0 only when every cell is green**; the release
flow ([tools/release/README.md](../tools/release/README.md), step 7) says it must be all
green, or every red explained.

## The columns

| Column | What it means (and which checks fill it) |
|---|---|
| system | where the package ran: this Mac, the Ubuntu VM (aarch64 natively; x86_64 by Rosetta), the Windows 11 ARM VM (arm64 natively; x64 and the 32-bit Windows 7 zip by emulation) |
| package | the artifact's file name |
| **run** | `rapidr run` of a console program and of a GUI program (a headless capture of its window) works from the installed package |
| **Build (interp)** | `rapidr build --interp` makes a standalone executable **with no Rust reachable** (PATH narrowed, `CARGO_HOME` pointing nowhere). This is what Studio's *Run > Build Interpreted App* does. Also: an installed example built, a data example built with its data file. Runtime-only packages show `n/a (Runtime)`: they refuse to build by design (and the smoke test checks that they say so) |
| **Build Native** | `rapidr setup --yes` installs RapidR's own Rust toolchain beside the user's (the user's own is checked unchanged), then offline native builds from the **shipped** sources: a console program, a program with forms (release), one that uses RArchive (every archive crate; 2.122.0 missed a module), and on Windows the conformance cases and a form's WndProc. `skipped` with `--quick`; `n/a` where it cannot be checked (below) |
| **built exe runs** | the programs those builds made are started and say what they should: `interp ok / native ok`. An interpreted GUI program is started the way the desktop starts it (macOS: from `/`, Linux: from the home folder, Windows: in its own folder) and must read its data file |
| other | everything else the smoke test checks: install, file types, Studio starts headless, Preview 5's features, the downloaded-file question, uninstall leaves nothing. A red here is as much a red as the others |
| time | wall time of that row (install to uninstall), as `m:ss` |

A cell is `ok`, `FAIL`, `n/a` (with the reason under the table), or `skipped`. A cell for which **no
check ran** is a `FAIL`, not a blank: a smoke test that stopped early cannot look green. Under the table
every red row has its first FAIL lines, with the line after each (the detail the smoke test printed:
the tail of the build's log, the expected text and what was there).

The smoke tests tag their checks with the cell they belong to and end with one line,
`##cells run=ok interp=ok exe=ok native=ok nexe=ok other=ok` (`-` = no check of that kind ran).
That line is the only thing `build-matrix.sh` reads, besides the `  FAIL` lines. To add a check, set
`CELL=<cell>` (`$script:cell` in PowerShell) before it; a new cell needs a column here and in `printrow`.

## How to read a failure

1. The table says which row and which cell. Open `dist/<ver>/build-matrix/logs/<lane>-<package>.log`
   (e.g. `windows-RapidR-2.122.1-windows-x64-setup.exe.log`): the whole smoke test, `== sections` and
   `ok` / `FAIL` lines. The first `FAIL` is almost always the cause; the rest follow from it.
2. **Build (interp) FAIL** with "can't run cargo" / "could not build or find rapidrintr-runner…": the
   package has no runner for that system (`lib/rapidr/runners/<os>-<arch>/`). This was the Windows 7 alpha.
3. **Build Native FAIL** on the `setup` check: the toolchain download failed or `rapidr setup` changed
   the user's defaults (a real bug). On a later check: the build itself (the log tail is printed).
   `Access is denied (os error 5)` right after cargo says *Finished*: an antivirus ate the new program
   (the smoke test names the antivirus on the machine). `os error 4551` / "An Application Control
   policy has blocked this file": Smart App Control (below).
4. **built exe runs FAIL** with the build ok: the executable started and printed something else; run it by hand
   from the log's folder.
5. A row with `the smoke test did not finish (no ##cells line)`: the VM dropped, a command hung, or the script
   died; the log's last line says where.

A flaky red (a timing check on a loaded machine) is rerun alone: `--only <lane>`; `P5_DELAY=12` gives
Preview 5's headless Studio checks more time on a busy Mac.

## What each lane does

| Lane | Runs | Where | Needs |
|---|---|---|---|
| `mac` | `smoke.sh` on both dmgs (native build included) | this Mac; scratch in `MATRIX_TMP` (default `/Volumes/RapidRBuild/build-matrix-tmp`) | Rust installed (the user's, only read) for the native build |
| `linux` | `smoke.sh` on every `.tar.gz` and `.deb` of both architectures, in one detached job in the Ubuntu VM | the VM's `~/rapidr-release/dist/out` (sent by base64 if missing or different: slow) | the VM running; `test-mode.token`; tools from `linux/setup-tools.sh` |
| `windows` | `smoke.ps1` on the x64 and arm64 SDK installers (with `Native`) and Runtime installers | the Windows 11 VM, as the user, headless (`windows-vm.sh smoke`); an installer is copied into `…\rapidr-release\out` from the home share unless the VM has it with the same SHA-256 | the VM; `rapidr-release\src` (the release's conformance cases: run the matrix **before** `windows-vm.sh clean`, which removes it); the token |
| `win7` | `smoke-win7.ps1` on the Windows 7 alpha zip | the same VM (x86 emulation) | the zip in `dist/<ver>/out/` or `MATRIX_WIN7_ZIP=<zip under your home>` |

The three machines work at the same time (`--serial` for one after the other). The script never
starts, stops or reboots a VM (it resumes a paused one); a stopped VM is a red lane with the reason.
Everything a program writes goes to a temporary registry file, printer folder and settings folder, and
`RAPIDR_AI_KEYSTORE=memory`: nothing touches the user's keychain or settings. `windows-vm.sh clean` and
`linux-vm.sh clean` remove what the runs leave (`--dry-run` first).

## The gaps (what this matrix cannot see)

These are known, so a green matrix is not read as more than it is:

- **Windows 7, 8, 8.1 and XP themselves.** No Windows 7 or XP VM can be driven from the Mac (Robert's UTM
  machines have no guest agent: `tools/win7/README.md`). The Windows 7 zip is used for real in the
  Windows 11 VM (x86 emulation), which proves the package: the files are right, Build works, the
  built program starts. It does not prove Windows 7 accepts the program. Instead the matrix reads the
  **import tables** of `rapidr.exe` and of both runner programs on the Mac (`tools/win7/imports.py`) and
  calls any function Windows 7 lacks a red `other`: a program Windows 7 refuses to start is not a
  program that runs. That check would have caught the six WinRT/COM functions of the runner stubs (below).
  The last word is still starting the zip in the Windows 7 VM by hand (`tools/win7/alpha/VM-STEPS.txt`).
  XP is not shipped (the XP *os error 123* is a separate lane).
- **Build Native on Linux x86_64.** The packages run by Rosetta in an aarch64 VM; there is no x86_64 Rust host
  there, and a native build through Rosetta would quietly build for the VM's own architecture. Needs a
  real x86_64 PC (or the user adding an x86_64 Ubuntu VM): the aarch64 rows cover the code path.
- **macOS on Intel** and **Windows on real x64 hardware**: the universal macOS apps are checked statically
  (both slices present), the x64 installers run emulated on ARM.
- **Smart App Control and other application-control policies** (below): the matrix runs on a Windows
  that allows unsigned programs, so the native-build block users saw (os error 4551) is not reproduced yet.
- **Antivirus products.** A product that quarantines new unsigned programs fails a native build with
  "Access is denied"; the Windows smoke test prints which products are installed, but the VM runs
  Microsoft Defender only.
- Real hardware (graphics drivers: the VM draws with the CPU renderer), a slow disk, a 3 GB RAM PC.

## The Windows 7 package, and what was wrong with it

`RapidR-2.122.1-win7-x86-alpha1.zip` was `rapidr.exe` beside `ide\studio.rr` and `examples\`, with
`RAPIDR_HOME` set by its `.bat` files. A person pressing **Build interpreted** got
*"can't run cargo (program not found); … could not build or find rapidrintr-runnerw.exe (cargo build -p
rapidr-runner-stub)"*: `rapidr build --interp` copies a **runner program** into the user's
executable, an install ships those in `lib\rapidr\runners\<os>-<arch>\`, and a source checkout builds
them with cargo. The zip was neither. (`README-ALPHA.txt` said Build was "not in this alpha"; the
Build commands in Studio were there anyway.)

The fix is in `tools/win7/`, not in the runtime:

- `tools/win7/runners.sh i686` builds the two runner programs for `i686-win7-windows-gnu` (only them; with
  the dependencies cached, two minutes).
- Both runner programs need the same six stand-ins for functions Windows 7 lacks that `rapidr.exe`
  has in `crates/rapidr-cli/src/win7.rs`; the runner stub is another crate without that module, and
  built as it is, **Windows 7 would refuse to start every program Build made** ("api-ms-win-core-winrt-l1-1-0.dll
  is missing"; invisible in the Windows 11 VM, where the functions exist). `tools/win7/shim/win7_stand_ins.c` is
  the same six stand-ins in C (our own code, no header, no new library), linked into the two runner programs
  only; `tools/win7/shim/test_stand_ins.c` checks it (the real functions, and the answers of a Windows that
  lacks them). It also has `cbrt` / `cbrtf`: `tools/win7/link_audit.sh <mingw-w64 src> <work> runner` found the
  runner linking mingw-w64's **Cephes-derived** cube roots (106 toolchain objects, 2 flagged), which the project
  never links (`docs/licensing.md` 3.3; `rapidr.exe` has its own in `cbrt.rs`). With ours: 103 objects, 0 flagged. The permanent version is `win7.rs` included by the stub crate (a 4-line change in
  `interpreter/rapidr-runner-stub`, left for whoever owns that crate).
- `tools/win7/alpha.sh` now lays the zip out as an install: `bin\rapidr.exe`, `lib\rapidr\release.toml`,
  `ide\rapidr-ide.rrbc`, `examples\`, `runners\windows-x86\`, `notices\windows-x86.txt`. `rapidr.exe` finds
  that home by itself, from any Command Prompt, with no environment variable. The `.bat` files and
  `README-ALPHA.txt` follow.

## Smart App Control: how to reproduce the block (research; nothing applied)

**The symptom.** Windows 11 with Smart App Control (SAC) on blocks unsigned, unknown programs, including the
build-script programs `cargo` compiles and runs from a user folder. `rapidr build` (native) ends with
`failed to run custom build command for …` / `could not execute process …build-script-build (never
executed)` and Rust's *"An Application Control policy has blocked this file. (os error 4551)"*
(Win32 4551 = 0x800711C7). The CodeIntegrity log (`Microsoft-Windows-CodeIntegrity/Operational`) has
event 3077 (blocked) or 3033. No Mark of the Web is needed: locally compiled output is blocked too.
The **interpreted** build is not affected in the same way (it copies a runner RapidR ships; whether the
copied, still unsigned `.exe` may start is for SAC's reputation check to say, and is worth testing on a
real SAC machine). The fix (a prebuilt, signed runtime so native builds do not run unsigned build
scripts) is a later lane.

**Facts** (Microsoft Learn and Support, read 2026-11-05; the URLs are at the end):

- SAC is built on WDAC / App Control for Business. It asks a Microsoft cloud reputation service about a
  binary; if unsure, the binary runs only if signed by a CA in Microsoft's Trusted Root Program. States:
  evaluation (blocks nothing), on, off. Read it: registry `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy`,
  value `VerifiedAndReputablePolicyState` (0 off, 1 on, 2 evaluation); `CiTool -lp` shows the policy
  `VerifiedAndReputableDesktop` (on) or `…Evaluation`; the Windows Security app shows it too.
- **"SAC can't be turned back on once off" is out of date.** Microsoft's FAQ says recent Windows updates
  let SAC be re-enabled without a clean install (one third-party source ties it to an April 2026 update;
  check the VM's build and Settings first: *Windows Security > App & browser control > Smart App Control*).
  Microsoft also documents forcing SAC *on* for testing by editing the registry offline from WinRE
  ("Test your app with Smart App Control"): disable BitLocker protectors, boot WinRE, load the SYSTEM hive,
  set `VerifiedAndReputablePolicyState` and `VerifiedAndReputablePolicyStateMinValueSeen`. Testing only,
  on a snapshot.
- Not found in any source: SAC inside Parallels or Hyper-V guests, and how SAC treats x64 programs run by
  emulation on ARM64. SAC is enforced on physical Windows on ARM PCs (Snapdragon X).

**Option 1, the real thing (preferred if the VM allows it).** Parallels snapshot first. In the VM, see whether
Settings offers *Smart App Control: On*. If it does, turn it on and run
`build-matrix.sh <ver> --only windows`; the native cells go red with os error 4551, which is the picture
the matrix should show. If it is off and cannot be turned on, use Microsoft's offline-registry test method
above, or a fresh Windows 11 VM created for it (SAC starts in evaluation mode on a clean install and may switch itself on).

**Option 2, simulate it with an App Control for Business policy** (SAC is itself such a policy). Untested here.
Never on the working VM without a snapshot: a bad policy can lock the VM (the shell and Parallels Tools can
be blocked; recovery is the boot menu's *Disable Driver Signature Enforcement*, or deleting the policy file
from WinRE).

1. Snapshot the VM (Parallels: *Actions > Take Snapshot*, or `prlctl snapshot "Windows 11 Pro" -n before-sac`).
2. Start from Microsoft's own SAC example, `%windir%\schemas\CodeIntegrity\ExamplePolicies\SmartAppControl.xml`
   (not on every build), or the `SmartAppControlAuditNoISG` audit policies from <https://aka.ms/sacauditpolicies>.
   Remove the cloud-reputation option (14, *Intelligent Security Graph*) so the block is deterministic, keep
   the SAC-derived base (an `AllowMicrosoft` base would also block Parallels' own tools), reset the PolicyID so
   it cannot collide with the built-in one, add rule options 9 (F8 menu), 10 (boot audit on failure) and 11
   (disable script enforcement, so PowerShell stays in full language mode for the rollback scripts).
3. **Audit first** (rule option 3): `ConvertFrom-CIPolicy -XmlFilePath policy.xml -BinaryFilePath {PolicyID}.cip`,
   then `CiTool --update-policy {PolicyID}.cip` (Windows 11 22H2 and later; no restart for an unsigned policy on
   24H2 and later). Run a native build; event **3076** means "would have been blocked". Check with
   `Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational';Id=3076,3077,3089}`.
4. **Enforce**: `Set-RuleOption -FilePath policy.xml -Option 3 -Delete`, convert again, `CiTool --update-policy`
   again. Under this policy `cargo.exe` and `rustc.exe` are blocked as well; to get "cargo runs, its build
   scripts are blocked" add hash rules for the toolchain (`New-CIPolicy -ScanPath <toolchain> -Level Hash -UserPEs`).
   That last step is a suggestion, not verified.
5. **Roll back**: `CiTool --remove-policy "{PolicyID}"` (alias `-rp`), or delete
   `C:\Windows\System32\CodeIntegrity\CiPolicies\Active\{PolicyID}.cip` and restart. Schedule a SYSTEM task that
   runs `CiTool -rp` after N minutes *before* enforcing (CiTool is Microsoft-signed, so it still runs), and
   keep the snapshot until the matrix has been run.

The lead decides with Robert whether to do this, and when (it needs the snapshot and an hour of the VM).
Workarounds users reported, to document with the symptom: turn SAC off; build with an older toolchain
(one report: a newer `cargo.exe` was blocked and an older one was not, so reputation moves); signing with a
Trusted Root Program certificate. Moving `CARGO_TARGET_DIR`, Developer Mode, self-signed certificates and
Defender exclusions did **not** help in the reports found.

Sources: Microsoft Support, *What is Smart App Control*; Microsoft Learn, *Smart App Control overview*,
*Test your app with Smart App Control*, *App Control for Business* (overview, example base policies, policy
rule options, CiTool commands, deploy policies by script, remove policies, event IDs, Intelligent Security
Graph), *Rust setup on Windows*; GitHub issues rust-lang/rust#160163 and storytold/photocraft#1585 for the
Cargo symptom text. A scout read them on 2026-11-05; nothing was run.

## Results: 2.122.1, the first runs (2026-11-05)

Kept in `dist/2.122.1/`: `build-matrix-before-win7-fix.txt` (the first full run, on the artifacts as shipped),
`build-matrix.txt` (the second run, with the fixed Windows 7 zip), the logs in `build-matrix/before-win7-fix/` and
`build-matrix/logs/`, and the pictures in `build-matrix/shots/` (below).

**Before the Windows 7 fix** (the shipped `RapidR-2.122.1-win7-x86-alpha1.zip`; the rest of that run, as read then):

```
Windows 7 (x86 alpha; run in the Win 11 VM) | RapidR-2.122.1-win7-x86-alpha1.zip | ok | FAIL | n/a | interp FAIL | ok | 0:43
  FAIL  console program: rapidr build --interp makes hello.exe
        could not build or find rapidrintr-runner.exe (cargo build -p rapidr-runner-stub)
```

**After** (the same zip made by the fixed `tools/win7/alpha.sh`: runner programs, installed layout, cube roots of our own):

```
Windows 7 (x86 alpha; run in the Win 11 VM) | RapidR-2.122.1-win7-x86-alpha1.zip | ok | ok | n/a | interp ok | ok | 1:05
```

Its checks: the home is complete (release.toml, the IDE's bytecode, the examples, both runners); `rapidr run`; Build
(interpreted) of a console program, a GUI program and the package's own `hello_form` example with PATH and `CARGO_HOME`
pointing nowhere; every program it made started (`hello_form.exe`'s window is
`build-matrix/shots/win7-example-1.png`); Studio headless; and `rapidr.exe`, `rapidrintr-runner.exe` and
`rapidrintr-runnerw.exe` import nothing Windows 7 lacks.

**The whole second run** (Linux: not run, its VM was stopped; Robert is asked to start it, then `--only linux`):

| system | package | run | Build (interp) | Build Native | built exe runs | other | time |
|---|---|---|---|---|---|---|---|
| macOS (this Mac, arm64) | RapidR-2.122.1-macos-universal.dmg | ok | ok | ok | interp ok / native ok | ok | 4:16 |
| macOS (this Mac, arm64) | RapidR-Runtime-2.122.1-macos-universal.dmg | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:04 |
| Linux (aarch64 and x86_64) | the 8 Linux packages | not run | not run | not run | not run | not run | VM stopped |
| Windows x64 (emulated) | RapidR-2.122.1-windows-x64-setup.exe | ok | ok | ok | interp ok / native ok | FAIL | 41:23 |
| Windows ARM64 | RapidR-2.122.1-windows-arm64-setup.exe | ok | ok | ok | interp ok / native ok | ok | 24:11 |
| Windows x64 (emulated) | RapidR-Runtime-2.122.1-windows-x64-setup.exe | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:48 |
| Windows ARM64 | RapidR-Runtime-2.122.1-windows-arm64-setup.exe | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:49 |
| Windows 7 (x86 alpha, in the Win 11 VM) | RapidR-2.122.1-win7-x86-alpha1.zip (fixed) | ok | ok | n/a | interp ok | ok | 1:05 |

So on every system the matrix could reach, **a person can build interpreted and build native from the packaged files and
the programs run** (Windows 7: interpreted only, there is no Rust for it). The reds that were not product failures:

- **Windows x64 SDK, `other`: "Coach is the mode"** (Preview 5's headless Studio check): the check reads Studio's
  window 5 seconds after start, and the x64 build running emulated on a busy VM had not handled the click yet
  (`aitools.mode=build`). The first run of the same installer passed it. The Windows scripts now take
  `P5Delay` (the matrix passes 10 seconds, `P5_DELAY` overrides), as `p5check.sh` always could; a rerun of that installer with it (the install, interpreted builds and Studio checks; no native part, 12:52) passed:
  `##cells run=ok interp=ok exe=ok other=ok`.
- **macOS, first run, `other`: "no local AI app found"**: the same kind of timing check on a Mac that was busy
  with both VMs (`Looking for AI apps…` was still on screen at 5 s). Passed in the second run (`P5_DELAY` 10).
- **All four Windows rows, first run, `other`: ".bas is not RapidR's by default (unticked)"**: the VM user's registry had
  `.bas`, `.rqw`, `.rqb` and `.rq` pointing at file types that no longer exist (left by an install that ticked "open .bas
  files with RapidR"; where they came from is not known: the clean's dry run before the first run did not list them, and
  `windows-vm.sh clean --dry-run` after it did, as "names a file type that is gone". The matrix's installs pass `/TASKS=`, which selects none). Removing them
  made the check pass, and a Runtime install and uninstall left none behind (the SDK installers were run again only in the second run, which had no such failure on ARM64). `smoke.ps1` now compares with the state before the install, so a
  leftover cannot fail it (and says so).

Real walk, in `dist/2.122.1/build-matrix/shots/`: `win7-example-1.png` and `win7-built-1.png` (programs built by the
Windows 7 package, running by x86 emulation), `RapidR-2.122.1-windows-x64-gui-1.png` (a natively built program, running),
`RapidR-2.122.1-windows-x64-lk-cap-1.png` (an interpreted build that reads its data file) and
`RapidR-2.122.1-windows-arm64-ide-1.png` (Studio).

## What this lane found, for the lead

- The Windows 7 runner programs, built as they were, **linked mingw-w64's Cephes-derived `cbrt` and `cbrtf`** (the project
  links none; found by `link_audit.sh ... runner`): fixed in `tools/win7/shim/` (own code), audit now 103 objects, 0 flagged.
  Worth the lawyer's note, with the stand-ins file as new (our own) code.
- They also lacked the six Windows 8-only stand-ins `rapidr.exe` has, so **Windows 7 would have refused to start every
  program Build made**, invisibly in the Windows 11 VM. The permanent fix is 4 lines in `interpreter/rapidr-runner-stub`
  (include `win7.rs` and `cbrt.rs`); left alone because this lane touches no runtime crate.
- A native build in the Windows 7 package is refused with a cargo error text (there is no Rust there); a clearer
  "not in this package" message is runtime code, not done.
- The zip grew from 34 MB to 84 MB: the two runner programs are 54 MB each (stripped), and every program Build makes is about
  that size.
- `windows-vm.sh clean` must come after the matrix: it removes the release source the Windows native check reads.
