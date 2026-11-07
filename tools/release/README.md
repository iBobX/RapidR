# Building a release

Everything here is run by hand, on the Mac and in the Parallels VMs. Nothing
pushes, uploads or signs with a key that isn't yours. The long version (what
is in each package, the licences, the VM setup) is
[docs/release-packaging.md](../../docs/release-packaging.md); this page is the
flow.

## One entry script per system

| System | Run | Where | Makes (in `dist/<ver>/out/`) |
|---|---|---|---|
| (all) | `prepare.sh` | Mac | the commit as `prep/src.tar`, the web interpreter, the IDE's bytecode; the web bundle, the VS Code extension, the SBOM, `RELEASE_NOTES.md` |
| macOS | `macos.sh` | Mac | `RapidR-<ver>-macos-universal.dmg` (RapidR Studio.app + RapidR Runtime.app, an Applications alias, the licences) and `RapidR-Runtime-<ver>-macos-universal.dmg` |
| Windows | `windows-vm.sh build` | Windows 11 VM | `RapidR-<ver>-windows-<x64\|arm64>-setup.exe` and `RapidR-Runtime-…` (Inno Setup, per user) |
| Linux | `linux-vm.sh` | Ubuntu VM | `rapidr_<ver>_<amd64\|arm64>.deb`, `rapidr-<ver>-linux-<arch>.tar.gz` and the `rapidr-runtime…` pair |
| (all) | `finish.sh` | Mac | the licence files checked in every package; `SHA256SUMS` |
| (all) | `build-matrix.sh <ver>` | Mac, drives both VMs | not a package: **the gate before publishing**. Installs every artifact the way a user does, presses Build (interpreted) and Build Native, runs what they made, and prints ONE table ([docs/build-matrix.md](../../docs/build-matrix.md)) |

The other files are the parts those scripts share:

- `common.sh` (sourced): the version (from `Cargo.toml`), the `dist/<ver>/{prep,work,out}` folders.
- `home.py`: the runtime's sources with their crates vendored (what native builds compile against, offline).
- `stage.py`: lays out an installed RapidR (`bin/`, `lib/rapidr/`, `share/doc/rapidr/`) from built pieces; every package is made from that tree.
- `smoke.sh` (macOS, Linux) and `windows/smoke.ps1` (Windows): install a package into a temporary place, check it, uninstall it, and remove everything they made (temporary folder, build cache, cargo target) even when a check fails. Both end with a line `##cells run=ok interp=ok exe=ok native=ok nexe=ok other=ok`: which of the checks that matter for "everybody builds" passed (what `build-matrix.sh` reads).
- `windows/smoke-win7.ps1`: the same idea for the Windows 7 alpha zip (`tools/win7/alpha.sh`), unzipped and used in the Windows 11 VM (`windows-vm.sh smoke-win7`).
- `win7-vm.sh` and `windows/smoke-win7-real.ps1`: the same zip on a **real Windows 7** (Robert's UTM VM "Windows 7 x86", driven through `tools/win7/vm.sh`): `win7-vm.sh smoke <zip>` sends the zip and the script, runs it as the user at the VM's desktop (PowerShell 2.0; the zip is unpacked with Windows' own Compressed Folders and counted against the Mac's count of its files), takes a picture of the built GUI program on that desktop, and removes what it made (`win7-vm.sh clean` does only that). The script is plain PowerShell on purpose (no inline C#, no archive reader of its own): the antivirus programs take such code for malware, see [docs/build-matrix.md](../../docs/build-matrix.md), "Test scripts and antivirus programs".
- `build-matrix.sh`: runs all of the above on every artifact of `dist/<ver>/out/` and prints one table (`dist/<ver>/build-matrix.txt`; logs and pictures in `dist/<ver>/build-matrix/`).
- `p5check.sh` and `windows/p5check.ps1`: Preview 5's features in the installed Studio, headless (Versions: save, change, save, Go Back; the Coach dial; Set Up RAI's empty "On this computer"; Studio in Spanish; School Lock's dialog). `smoke.sh` and `windows/smoke.ps1` run them; `linux-vm.sh smoke` sends `p5check.sh` beside `smoke.sh`.
- **The test-mode token (SEC-24).** A release build's test hooks (the captures and dumps every smoke test reads) act only with a signed token: `RAPIDR_TEST_MODE=<token>`, or the harnesses' `~/.rapidr/test-mode.token` (`%USERPROFILE%\.rapidr\test-mode.token` on Windows), which `smoke.sh`, `smoke.ps1` and `p5check` pick up. On the Mac it is already there; for a VM, copy the token file to that user's `.rapidr` folder first (it is signed, expires, and is not a key: [docs/security/test-mode.md](../../docs/security/test-mode.md)). Without it every GUI check fails with "test hooks are off".
- `linux.sh`, `linux/` (install.sh, uninstall.sh, the MIME types and desktop entries, setup-tools.sh, clean.sh): the Linux build, run inside the VM by `linux-vm.sh`; `ubuntu-vm.sh` is the VM driver (send, run, root, fetch).
- **x86-64 Linux in the arm64 VM** (`linux/setup-tools.sh x86rust`, `linux/x86-env.sh`, `linux/jit-check.sh`): rustup's own x86_64 Rust host in `~/rust-x86_64`, run by Rosetta for Linux beside the VM's arm64 Rust, so the x86_64 packages' *Build Native* and the JIT's x86-64 code generator are tested as x86_64 (`ubuntu-vm.sh run tools/release/linux/jit-check.sh <src.tar.gz> x86_64`). Rosetta needs `ROSETTA_DISABLE_AOT=1` for rustc (it crashes without), and the programs are linked by Zig; `x86-env.sh` has the reasons. Without the x86_64 Rust the matrix shows `n/a` for those rows, with the reason.
- `windows/` (windows.ps1, rapidr.iss, setup-tools.ps1, smoke.ps1, clean.ps1, the job helpers detach/job/stop/piece/user/whoami, watch-windows.ps1): the Windows build, run inside the VM by `windows-vm.sh`. Its calls run as SYSTEM and its jobs in a headless console, so nothing opens a window on the VM's desktop (`windows-vm.sh flashtest control` checks).
- `macos/`: the two Info.plists and the entitlements.
- `web.sh`, `vscode.sh`, `sbom.py`: the web bundle, the VS Code extension, the SBOM (called by `prepare.sh`).
- `windows/link_audit.sh`, `windows/member_licences.py`: the licence audit of what the Windows builds link from LLVM-MinGW.

Every script starts with a comment saying what it needs, what it makes and
how to run it.

## The flow

1. **The commit**: version in `Cargo.toml`, `CHANGELOG.md`, the ROADMAP ticks,
   `tools/regress.sh` green, the tree clean. A release is built from a commit.
2. **`tools/release/prepare.sh`** on the Mac. Edit `RELEASE_NOTES.md`.
3. **`tools/release/macos.sh`** on the Mac. Check:
   `tools/release/smoke.sh dist/<ver>/out/RapidR-<ver>-macos-universal.dmg`.
4. **`tools/release/linux-vm.sh`**, then `tools/release/linux-vm.sh smoke`.
5. **`tools/release/windows-vm.sh build`** (about two hours on the first run),
   then for each installer `windows-vm.sh smoke <installer> Native` (add `Regress` for the conformance suite and the examples with the installed rapidr) and
   `-Associations`, then `windows-vm.sh fetch <installer> dist/<ver>/out/<installer>`.
6. **`tools/release/finish.sh`**.
7. **`tools/release/build-matrix.sh <ver>`: the gate.** Before publish it must be
   **all green, or every red explained** in the release notes (an explained red is
   a known gap, never a surprise: [docs/build-matrix.md](../../docs/build-matrix.md)
   lists the gaps). It exits non-zero on any red, so a script can stop on it.
   About an hour with the native builds (`--quick`: minutes, interpreted only);
   it needs both Parallels VMs running (it never starts one: ask Robert) and what steps 4
   and 5 needed (the test-mode token in each VM, the Windows VM's `rapidr-release\src`).
   The Linux VM need not have anybody logged in at its desktop (`ubuntu-vm.sh` then works as root and hands the
   scripts to the VM's user). The Windows 7 alpha zip (`tools/win7/alpha.sh`, made apart from the release) is
   checked too when it is in `out/`: in the Windows 11 VM (`win7`, emulation) and on the real Windows 7
   (`win7real`: the UTM VM started by Robert, somebody logged in at its desktop, the guest agent working).
   A VM job that never reports ends the wait with a `TIMEOUT` and a red row after `MATRIX_TIMEOUT_MIN` minutes (default 90).
8. **Look at `dist/<ver>/out/`**, do the fresh-user journey below, then publish
   by hand (`gh release create`, see the packaging doc).
9. **Clean up** (the last step, always: a release leaves ~10 GB in each VM and
   the Windows VM's disk once filled up): `rm -rf dist/<ver>/work dist/<ver>/prep`,
   then `linux-vm.sh clean` and `windows-vm.sh clean`. Each removes the release
   work folder, the build cache, the smoke tests' temporary folders and the test
   apps, and keeps the tools, an installed RapidR and the user's files;
   `--dry-run` lists what would go, with sizes, `--keep-out` keeps the packages.
   (The real Windows 7 VM removes its own folder, `C:\rapidr-matrix`, at the end of each run.)

## Signing (yours to turn on)

Unsigned by default; every script works without a certificate. The hooks:

- macOS: `macos.sh --sign "Developer ID Application: Name (TEAMID)" --notarize <keychain profile>`.
- Windows: `windows.ps1 -Sign "<certificate subject>"` (through `windows-vm.sh build Sign=…`).
- Linux: the packages aren't signed.

## The fresh-user journey (release gate 1)

On each system, from a clean account, with the installer from `out/`:

1. Install it (macOS: drag **RapidR Studio** to Applications; Windows: run the
   setup; Linux: `sudo apt install ./rapidr_<ver>_<arch>.deb`).
2. Open **RapidR Studio** (Dock/Launchpad, Start menu, applications menu).
3. Open an example (File > Open, the `examples` folder) and run it.
4. Build an app from it (Run > Build) and open the app.
5. Double-click a `.rrbc` and open a `.rr` and a `.rrproj`.
6. Uninstall (macOS: move the app to the Trash; Windows: Settings > Apps;
   Linux: `sudo apt remove rapidr`). The machine is as before: no files, no
   file types, no PATH entry left.

The screenshots of these steps are in
[docs/manual/images/install/](../../docs/manual/images/install/).
