#!/bin/bash
# Runs the Windows release scripts in the Windows VM (Parallels; RAPIDR_WIN_VM
# names it), which reads this repository and prepare.sh's output through the
# Mac's home share (\\Mac\Home) — only reads:
#
#   tools/release/windows-vm.sh build [windows.ps1 options…]   the installers, in the VM's
#                                                              %USERPROFILE%\rapidr-release\out
#   tools/release/windows-vm.sh smoke <installer file name> [-Associations] [-Native]
#
# The installers come back to the Mac by `-Publish <\\Mac\Home\…\dist\<ver>\out>` (it writes
# through the share) or by copying them yourself. tools/windows/vmexec.sh does the
# prlctl part (the VM may need `prlctl resume` when it paused itself).
set -euo pipefail
source "$(dirname "$0")/common.sh"
REL="${ROOT#"$HOME"/}"
[ "$REL" = "$ROOT" ] && die "the repository must be under your home folder (the VM sees it as \\\\Mac\\Home)"
SHARE="\\\\Mac\\Home\\${REL//\//\\}"
PS="powershell -NoProfile -ExecutionPolicy Bypass -File"
case "${1:-}" in
    build)
        shift
        exec "$ROOT/tools/windows/vmexec.sh" "$PS $SHARE\\tools\\release\\windows\\windows.ps1 -Prep $SHARE\\dist\\$VERSION\\prep $*"
        ;;
    smoke)
        installer="$2"; shift 2
        exec "$ROOT/tools/windows/vmexec.sh" "$PS $SHARE\\tools\\release\\windows\\smoke.ps1 -Installer %USERPROFILE%\\rapidr-release\\out\\$installer $*"
        ;;
    *) die "usage: $0 build|smoke …" ;;
esac
