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

The other files are the parts those scripts share:

- `common.sh` (sourced): the version (from `Cargo.toml`), the `dist/<ver>/{prep,work,out}` folders.
- `home.py`: the runtime's sources with their crates vendored (what native builds compile against, offline).
- `stage.py`: lays out an installed RapidR (`bin/`, `lib/rapidr/`, `share/doc/rapidr/`) from built pieces; every package is made from that tree.
- `smoke.sh` (macOS, Linux) and `windows/smoke.ps1` (Windows): install a package into a temporary place, check it, uninstall it.
- `linux.sh`, `linux/` (install.sh, uninstall.sh, the MIME types and desktop entries, setup-tools.sh): the Linux build, run inside the VM by `linux-vm.sh`; `ubuntu-vm.sh` is the VM driver (send, run, fetch).
- `windows/` (windows.ps1, rapidr.iss, setup-tools.ps1, smoke.ps1, the job helpers): the Windows build, run inside the VM by `windows-vm.sh`.
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
   then for each installer `windows-vm.sh smoke <installer> -Native` and
   `-Associations`, then `windows-vm.sh fetch <installer> dist/<ver>/out/<installer>`.
6. **`tools/release/finish.sh`**.
7. **Look at `dist/<ver>/out/`**, do the fresh-user journey below, then publish
   by hand (`gh release create`, see the packaging doc).
8. **Clean up**: `rm -rf dist/<ver>/work dist/<ver>/prep`,
   `linux-vm.sh --clean`, `windows-vm.sh clean`.

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
