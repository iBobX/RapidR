# The build matrix: "everybody builds, no issues"

Lane BUILD-MATRIX, 2026-11-05. One command shows, before a release is published,
whether a person on **every system RapidR ships** can install the package, press
**Build (interpreted)** and **Build Native**, and get a program that runs.

```
tools/release/build-matrix.sh <ver> [--quick] [--only mac,linux,windows,win7,win7real] [--shots] [--serial]
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
| system | where the package ran: this Mac, the Ubuntu VM (aarch64 natively; x86_64 by Rosetta), the Windows 11 ARM VM (arm64 natively; x64 and the 32-bit Windows 7 zip by emulation), the real Windows 7 (Robert's UTM VM: a real Windows 7 on an emulated PC) |
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
| `linux` | `smoke.sh` on every `.tar.gz` and `.deb` of both architectures, in one detached job in the Ubuntu VM | the VM's `~/rapidr-release/dist/out` (sent by base64 if missing or different: slow) | the VM running (nobody needs to be logged in at its desktop: `ubuntu-vm.sh` then works as root and hands the scripts to the VM's user); `test-mode.token`; tools from `linux/setup-tools.sh`. `P5_DELAY` (10 s by default) reaches the VM job: Preview 5's headless checks of the x86_64 packages, run by Rosetta, need it. The x86_64 packages' **Build Native** needs the VM's x86_64 Rust (`linux/setup-tools.sh x86rust`: rustup's own host in `~/rust-x86_64`, run by Rosetta; `linux/x86-env.sh` sets up its linking); without it those rows say `n/a` and why |
| `windows` | `smoke.ps1` on the x64 and arm64 SDK installers (with `Native`) and Runtime installers | the Windows 11 VM, as the user, headless (`windows-vm.sh smoke`); an installer is copied into `…\rapidr-release\out` from the home share unless the VM has it with the same SHA-256 | the VM; `rapidr-release\src` (the release's conformance cases: run the matrix **before** `windows-vm.sh clean`, which removes it); the token |
| `win7` | `smoke-win7.ps1` on the Windows 7 alpha zip | the same VM (x86 emulation) | the zip in `dist/<ver>/out/` or `MATRIX_WIN7_ZIP=<zip under your home>` |
| `win7real` | `smoke-win7-real.ps1` on the same zip, through `tools/release/win7-vm.sh smoke` | Robert's UTM VM "Windows 7 x86": a **real Windows 7** (build 7600), run as the user at its desktop by a scheduled task; everything it makes is in `C:\rapidr-matrix` and removed at the end | the VM started by Robert with somebody logged in at its desktop; the guest agent (`tools/win7/vm.sh exec ver`); the zip as for `win7`; the test-mode token (copied into `C:\rapidr-matrix` only). About 6 minutes: most of it is sending the 84 MB zip |

The four machines (this Mac, the Ubuntu VM, the Windows 11 VM, the Windows 7 VM) work at the same time (`--serial`
for one after the other). The script never starts, stops or reboots a VM (it resumes a paused Parallels one); a stopped
VM is a red lane with the reason. The default run includes `win7real`, so a release gate needs Robert's Windows 7 VM up too.
Everything a program writes goes to a temporary registry file, printer folder and settings folder, and
`RAPIDR_AI_KEYSTORE=memory`: nothing touches the user's keychain or settings. `windows-vm.sh clean` and
`linux-vm.sh clean` remove what the runs leave (`--dry-run` first).

## The gaps (what this matrix cannot see)

These are known, so a green matrix is not read as more than it is:

- **Windows 7 on real hardware; Windows 7 SP1 and updated; Windows 8, 8.1 and XP.** The `win7real` row runs the
  zip on a real Windows 7 (build 7600, RTM, no updates) in UTM: its own loader, system DLLs and window manager, so a
  missing function or a program Windows 7 refuses to start is seen for real (the same row on the zip as shipped
  failed with the same Build error users saw). The PC around it is emulated (QEMU i386 on an Apple Silicon Mac: slow,
  3 GB, no GPU driver, software drawing), so graphics drivers and speed are not tested. Windows 7 SP1 / updated,
  8 and 8.1 (the same binary) have no VM; XP is not shipped (the XP *os error 123* is a separate lane). The `win7`
  row keeps the **import-table check** on the Mac (`tools/win7/imports.py`: any function Windows 7 lacks is a red
  `other`, and it names the function); the real row confirms it, and the Windows 11 row still proves the package
  where Robert's Windows 7 VM is not running.
- **Build Native on Linux x86_64 is tested through Rosetta, not on an x86 CPU.** (Since the x86-64 lane, 2026-11-05
  night; before it the cell was `n/a`: the VM's own Rust is arm64, and a native build with it would quietly build
  arm64 programs.) The x86_64 packages' native build is done by an **x86_64 Rust host that the arm64 VM runs by
  Rosetta for Linux** (`~/rust-x86_64`, `linux/setup-tools.sh x86rust`), so what is built is x86_64 (the smoke test
  checks it with `file`) and runs by Rosetta. What it still cannot see: a real x86 CPU (timing, CPU features: Rosetta
  advertises AVX2 here), a real x86_64 distribution with its own gcc (the programs are linked by Zig against the
  amd64 libraries, because the VM has no x86_64 gcc and apt is not used for this). Two Rosetta facts: rustc needs
  `ROSETTA_DISABLE_AOT=1` (below), and without the x86_64 Rust installed the rows say `n/a` with that reason, never
  a blank.
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

## Results: 2.122.1, part 2: the Linux rows and the real Windows 7 (2026-11-05, evening)

`build-matrix.sh 2.122.1 --only linux,win7real` (with `MATRIX_WIN7_ZIP` the fixed zip, `P5_DELAY=15`), appended to
`dist/2.122.1/build-matrix.txt`; the run itself is `build-matrix.partial.txt`, the logs `build-matrix/logs/`, and the
real Windows 7 run on the zip *as shipped* (the "before") is `build-matrix/before-win7-fix/win7real-…log`. **ALL GREEN.**

| system | package | run | Build (interp) | Build Native | built exe runs | other | time |
|---|---|---|---|---|---|---|---|
| Linux aarch64 | rapidr-2.122.1-linux-aarch64.tar.gz | ok | ok | ok | interp ok / native ok | ok | 4:38 |
| Linux x86_64 (Rosetta) | rapidr-2.122.1-linux-x86_64.tar.gz | ok | ok | n/a | interp ok | ok | 1:39 |
| Linux aarch64 | rapidr-runtime-2.122.1-linux-aarch64.tar.gz | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:01 |
| Linux x86_64 (Rosetta) | rapidr-runtime-2.122.1-linux-x86_64.tar.gz | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:02 |
| Linux x86_64 (Rosetta) | rapidr-runtime_2.122.1_amd64.deb | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:02 |
| Linux aarch64 | rapidr-runtime_2.122.1_arm64.deb | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:01 |
| Linux x86_64 (Rosetta) | rapidr_2.122.1_amd64.deb | ok | ok | n/a | interp ok | ok | 1:38 |
| Linux aarch64 | rapidr_2.122.1_arm64.deb | ok | ok | ok | interp ok / native ok | ok | 4:41 |
| Windows 7 (real, UTM VM) | RapidR-2.122.1-win7-x86-alpha1.zip (fixed) | ok | ok | n/a | interp ok | ok | 6:00 |
| Windows 7 (real, UTM VM) | the zip as shipped | ok | **FAIL** | n/a | interp FAIL | ok | 3:03 |

The shipped zip fails on the real Windows 7 with the first error users met: `could not build or find
rapidrintr-runner.exe (cargo build -p rapidr-runner-stub)`. Build Native is `n/a` for the x86_64 rows (no x86_64 Rust host
behind Rosetta) and for Windows 7 (no Rust for it): the aarch64 rows cover the native code path.

**What was real and what was emulated**

| Row | Ran on | Real or emulated |
|---|---|---|
| macOS (part 1) | this Mac, Apple Silicon | real hardware, native arm64; the universal x86_64 slice was not run |
| Linux aarch64 | Ubuntu 24.04.3 ARM64 in Parallels | virtualised, not emulated: a real arm64 Linux on the Mac's CPU; native builds are real aarch64 builds |
| Linux x86_64 | the same VM, Rosetta for Linux | translated: a real Linux kernel, but Apple's Rosetta runs the x86_64 programs, no x86 CPU; so run, Build (interpreted) and the built programs are real, no native build |
| Windows ARM64 / x64 (part 1) | Windows 11 Pro ARM64 in Parallels | arm64 virtualised; x64 by Windows 11's own emulation |
| Windows 7, `win7` (part 1) | the same Windows 11 VM | x86 emulation by Windows 11: proves the package, Windows 7 not involved |
| **Windows 7, `win7real`** | Robert's UTM VM, **Windows 7 Ultimate x86 build 7600** (RTM, no updates, PowerShell 2.0, .NET 3.5) | **a real Windows 7** (its loader, DLLs, window manager) on an **emulated PC** (QEMU i386: slow, no GPU) |

**What the real Windows 7 said** (log `build-matrix/logs/win7real-RapidR-2.122.1-win7-x86-alpha1.zip.log`):

- Its loader starts `rapidr.exe`, `rapidrintr-runner.exe` and `rapidrintr-runnerw.exe`: no missing DLL, no missing entry
  point. The static import check of the Mac (the `win7` row) is confirmed by the real loader.
- `rapidr.exe __win7_check`: `RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix` and `CoIncrementMTAUsage`
  **are not on this Windows** and RapidR's stand-ins answer (`0x80040154`, `0`, `0x80004001`, `0x80004001`);
  `CoCreateFreeThreadedMarshaler` and `CoTaskMemFree` are found in `ole32.dll`.
- `rapidr run` of a console and a GUI program; **Build (interpreted)** of both with PATH and `CARGO_HOME` pointing nowhere
  (a 54 MB program in 2 s); `hello.exe` prints; `gui.exe` shows its window (headless capture) and, started on the
  desktop, stays up with a window; Studio starts headless.
- The picture: `build-matrix/shots/win7-gui-hello.png` is the built program's window on the real desktop (Aero frame,
  "RapidR on Windows 7") and `win7-desktop.png` the whole desktop (1728x1004, taskbar, the VM's clock).

**What the Linux lane had wrong** (it had never run; commit `c14c6216`):

- Nobody was logged in at the Ubuntu VM's desktop (it had just been started), and `prlctl exec --current-user` then answers
  *"Unable to authenticate the user"* to every call. `tools/release/ubuntu-vm.sh` probes the user's own session three times and
  otherwise works as root and hands the script to the VM's user (`runuser`). That needed three more things: Ubuntu's
  `fs.protected_regular=2` (root cannot write a user-owned file in `/tmp`, the user cannot write a root-owned one: the root
  calls have names of their own, and files for the user go straight into its home), root's `~` being `/`, and root's umask 077.
- The x86_64 package's Preview 5 *Versions* check read its picture too early under Rosetta (`vertimeline.itemcount=2`, wanted 3)
  at the 5 s default, once, in a run with both machines busy: `P5_DELAY` is now passed into the VM job (15 s: green).
- A time Parallels dropped was recorded as `0:00`; it is asked again.

## Results: 2.122.1, part 3: x86-64 (X86-64 lane, 2026-11-05, night)

Robert allowed one download: Rust's own x86-64 pieces from rust-lang's servers (rustup). With them the two things
that were `n/a` or "not run" are run, and the JIT is tested on x86-64 in both VMs. Logs: `dist/2.122.1/build-matrix/x86-lane/`
(the Linux rows' logs are in `build-matrix/logs/`; the earlier ones are kept in `build-matrix/before-x86/`).

**Linux x86_64, Build Native** (`build-matrix.sh 2.122.1 --only linux`, `P5_DELAY=15`; the whole run 44:57; **ALL GREEN**):

| system | package | run | Build (interp) | Build Native | built exe runs | other | time |
|---|---|---|---|---|---|---|---|
| Linux aarch64 | rapidr-2.122.1-linux-aarch64.tar.gz | ok | ok | ok | interp ok / native ok | ok | 5:58 |
| Linux x86_64 (Rosetta) | rapidr-2.122.1-linux-x86_64.tar.gz | ok | ok | **ok** | interp ok / **native ok** | ok | 16:43 |
| Linux aarch64 | rapidr-runtime-2.122.1-linux-aarch64.tar.gz | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:02 |
| Linux x86_64 (Rosetta) | rapidr-runtime-2.122.1-linux-x86_64.tar.gz | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:06 |
| Linux x86_64 (Rosetta) | rapidr-runtime_2.122.1_amd64.deb | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:02 |
| Linux aarch64 | rapidr-runtime_2.122.1_arm64.deb | ok | n/a (Runtime) | n/a (Runtime) | n/a | ok | 0:02 |
| Linux x86_64 (Rosetta) | rapidr_2.122.1_amd64.deb | ok | ok | **ok** | interp ok / **native ok** | ok | 16:05 |
| Linux aarch64 | rapidr_2.122.1_arm64.deb | ok | ok | ok | interp ok / native ok | ok | 5:01 |

For each x86_64 SDK package the native part is: `rapidr setup --yes` installs RapidR's Rust 1.98.1 **x86_64** toolchain into
the throwaway rustup home (the defaults unchanged); a console program, a program with forms (release) and an RArchive
program build offline from the shipped sources; the console and RArchive programs run (by Rosetta); `file` says the native
executable is x86-64; the user's own x86_64 Rust is unchanged afterwards. 16 minutes instead of the aarch64 row's 5: rustc
is translated at run time (below). The informational line of those two logs that prints the Rust version is wrong
(`error: rustup could not choose a version…`: it ran before the toolchain existed); fixed afterwards in `smoke.sh`, no check
depends on it.

**The JIT on x86-64** (docs/jit-design.md §12.6):

| | Linux x86-64 (Ubuntu VM, Rosetta) | Windows x64 (Windows 11 ARM64 VM, Prism emulation) |
|---|---|---|
| built | in the VM, x86_64 rustc 1.98.1 by Rosetta, linked by Zig | in the VM, `x86_64-pc-windows-msvc` cross-built by the VM's ARM64 Rust and MSVC's `HostARM64\x64` linker (27 min) |
| the JIT's tests (every edge value) | 1, 4, 7, 17 passed, 0 failed (the 2 ignored are the same two as on arm64) | 1, 4, 7, 17 passed, 0 failed, the same two ignored (the 7-test binary took 589 s emulated) |
| conformance, interpreter and `RAPIDR_JIT=always` | **668 passed, 0 failed** | **660 runs: clean in 2 of 5 tries of both backends (3 of 5 for each backend alone); the others lost 1-2 cases to a 30 s stall** (below) |
| speed, wall ms with / without the JIT | `loop10m` 164 / 529, `fcalls` 169 / 474, `dsp_scalars` 174 / 334, `calls` 140 / 140 | `loop10m` 187-539 / 746-755, `fcalls` 169-347 / 728-1523, `dsp_scalars` 211-228 / 627-681, `calls` 170-235 / 211-276 (3 runs each, quiet machine) |

The speed rows are what a translator makes of the JIT, not an x86 CPU's. The JIT's own start-up is the part that suffers: the
code generator is a big program that Rosetta or Prism has to translate before it first runs, and compiling the same few
routines took 60-70 ms under Rosetta against 0.6-1.7 ms in the VM's arm64 build, which eats the gain of a short program (`calls`
on Linux: 140 / 140). The programs' own `ms` lines on Windows, JIT on / off: `fcalls` 15-42 / 546-838, `dsp_scalars` 34-35 / 413-462.

**Windows x64 conformance: intermittent 30 s stalls, with the JIT off as well (open).** Every try of the whole suite (330 cases per
backend; `vm` = the interpreter alone, `vm-jit` = `RAPIDR_JIT=always`; logs `x86-lane/windows-x64-*`):

| try | machine | `vm` (JIT off) | `vm-jit` (JIT always) |
|---|---|---|---|
| first run (with the build) | busy: the Linux matrix's native builds ran at the same time | 1 failed: `canvas_rotate_get_put` (30 s stall) | 2 failed: `gosub_advanced` (30 s stall), `glass_frame_errors` (the compile-error case lost its expected diagnostics; no timeout printed) |
| second whole run | quiet (9 % CPU) | all passed | 2 failed: `rapidq_form_first_show` (30 s stall in the compile step), `stream_bytes` (30 s stall after its `AB,` line) |
| rounds 1, 2 | quiet | all passed | all passed |
| round 3 | quiet | 1 failed: `canvas_rotate_get_put` (30 s stall again, after its `rot …` line) | all passed |

What this says, and does not: the stalls are not the JIT's alone (two of them are in the JIT-off backend, the same case twice), and
they do not repeat on demand: the five cases again 12 times each in both backends, alone, all passed (3-7 s for a round of six
runs). A hang that the JIT caused with `RAPIDR_JIT=always` would be expected to come back on the same case; the four `vm-jit` casualties
were four different cases, each once, and three whole `vm-jit` passes are 330 of 330. But four one-off failures (in two tries) cannot be told
from the same thing the JIT-off side shows, so **this is not proof that the JIT is innocent on Windows x64**, only that nothing points at it.
Two of the cases that stalled build a form (`canvas_rotate_get_put`, `rapidq_form_first_show`). There is one native baseline for the
same VM: the JIT lane's last Windows ARM64 run passed 660 of 660 (its earlier tries failed on a shared work folder, fixed), so how often
this happens natively is not known. Not investigated further (no code was to change): it needs a look at what a
form case does when the process is started by the runner under Windows' x64 emulation (and with Bitdefender, which the VM runs).
The first run's `rapidr run` timings (`loop10m` 16 s with the JIT) came from the busy moment, on the first run of a fresh binary, and are
not to be quoted; the table above has the quiet ones.

**Two things the lane found** (both fixed in `tools/release/`, on the lane's branch):

- **x86_64 `rustc` crashes under Rosetta for Linux unless `ROSETTA_DISABLE_AOT=1`.** Every program that loads rustc's 150 MB
  `librustc_driver.so` (rustc, rustdoc) died with SIGSEGV inside Rosetta's own runtime (fault address `0x8000…`, right after it
  mapped its translation cache from `prlrosettad`; `strace` shows it); `cargo`, `rustup` and the programs Rust builds are fine.
  Not the cache directory (a fresh `HOME` crashes too); `ROSETTA_ADVERTISE_AVX=1` does not help. Rosetta's own list of switches
  (`strings /media/psf/RosettaLinux/rosetta`) has `DISABLE_AOT`: with `ROSETTA_DISABLE_AOT=1` rustc reports `host:
  x86_64-unknown-linux-gnu` and builds. `linux/x86-env.sh` sets it for these builds only. Rosetta then translates at run time:
  slower builds (16 minutes for an x86_64 row's native part instead of 5 for the arm64 one), the programs built are unchanged.
- **`windows/detach.ps1` stopped every detached job** because one script of `tools/release/windows/` could not be copied
  into the VM's `…\rapidr-release\scripts` folder: `smoke-win7-real.ps1` (added that day) answers *Access is denied* there, at that
  one path only (any other name there, and that name in every other folder, work; there is no such file in the folder, so
  something holds a stale entry; the VM also runs Bitdefender). A `windows-vm.sh jit` or `build` or `smoke` died silently
  at that line. `detach.ps1` now copies file by file, warns, and fails only when one of the three scripts the job runs
  (`job.ps1`, `user.ps1`, the job's own) is not this checkout's. The stale entry itself was left alone (why it is there is
  not known).

**What is new in the tools** (branch `x86-64-lane`): `linux/setup-tools.sh x86rust` (rustup-init from static.rust-lang.org,
SHA-256 checked, into `~/rust-x86_64`, same Rust version as the VM's arm64 one, nothing of the arm64 Rust touched),
`linux/x86-env.sh` (Rosetta switch, Zig as `cc`/linker, the amd64 pkg-config), `linux/jit-check.sh … x86_64`, and
`smoke.sh` / `build-matrix.sh` / `linux-vm.sh smoke` using that Rust for the x86_64 packages. A VM without the x86_64 Rust still
gets `n/a` for those cells, with the reason in the table.

**What was real and what was emulated, now:**

| Row | Ran on | Real or emulated |
|---|---|---|
| Linux x86_64, run / Build (interp) / Build Native / built programs | the Ubuntu VM, Rosetta for Linux | translated: a real Linux kernel and Apple's Rosetta, no x86 CPU; **the Rust compiler, its output and the programs are real x86_64**; the linker is Zig, not gcc |
| JIT, Linux x86-64 | the same | the code generator's x86-64 backend and the System V x86-64 calling convention of its entry forms and helpers run as x86-64 code (translated) |
| JIT, Windows x64 | Windows 11 ARM64 VM | translated by Windows' own x64 emulation: the Windows x64 calling convention of the entry forms and helpers (the one thing the Mac's Rosetta run could not cover) runs as x64 code |
| everything above | | **not run on an x86 CPU**: timings, CPU feature detection and a real x86_64 gcc/glibc are what only a real PC shows |

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
- The Ubuntu VM is shared with other lanes (the JIT's `~/rapidr-jit` and `/tmp/rapidr-jit-*` were there): `linux-vm.sh clean`
  touches only `~/rapidr-release`, the build cache and the smoke tests' `/tmp` files, and lists the other folders without removing them.
- The permanent fix of the shipped Windows 7 zip is `tools/win7/alpha.sh` (the stubs and the layout); the zip in `dist/2.122.1/out/` is
  still the broken alpha1 (the matrix's `win7` and `win7real` rows on it are red until it is replaced: `MATRIX_WIN7_ZIP` points at the fixed one).
