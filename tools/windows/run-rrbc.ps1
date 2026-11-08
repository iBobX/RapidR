# run-rrbc.ps1: runs a RapidR bytecode file (.rrbc, compiled on the Mac) in
# the Windows VM with a rapidr.exe already there, and prints what it printed.
# Through tools/release/windows-vm.sh (as SYSTEM: no window on the desktop):
#
#   rapidr build-bc examples/windows/rwin32_system_info.rr -o tests/fixtures/rwin32/out/info.rrbc
#   tools/release/windows-vm.sh sync tools/windows/run-rrbc.ps1 -File tests\fixtures\rwin32\out\info.rrbc
#
# -File is a path below the repository (read through the Mac's home share,
# only read). The rapidr.exe is the newest of the VM's own builds
# (%USERPROFILE%\rapidr-dllmem, rapidr-jit, rapidr-release\stage) or its
# install; -Exe names another.
param(
    [Parameter(Mandatory = $true)][string]$File,
    [string]$Exe = ""
)
$ErrorActionPreference = "Stop"
. "$PSScriptRoot\..\release\windows\user.ps1"
$repo = (Resolve-Path "$PSScriptRoot\..\..").Path
if (-not $Exe) {
    $candidates = @(
        "$UserProfile\rapidr-dllmem\target\release\rapidr.exe",
        "$UserProfile\rapidr-jit\target\release\rapidr.exe",
        "$UserProfile\rapidr-release\stage\sdk-aarch64\bin\rapidr.exe",
        "$UserProfile\AppData\Local\Programs\RapidR\bin\rapidr.exe",
        "$UserProfile\AppData\Local\Programs\RapidR\rapidr.exe"
    ) | Where-Object { Test-Path $_ }
    if (-not $candidates) { Write-Host "no rapidr.exe in the VM"; exit 1 }
    $Exe = $candidates | Sort-Object { (Get-Item $_).LastWriteTime } -Descending | Select-Object -First 1
}
$work = "$UserProfile\rapidr-rrbc"
New-Item -ItemType Directory -Force $work | Out-Null
$name = Split-Path $File -Leaf
Copy-Item (Join-Path $repo $File) "$work\$name" -Force
# (the run's own registry, printer and settings: never the user's)
$env:RAPIDR_REGISTRY = "$work\registry.reg"
$env:RAPIDR_PRINT_TO = "$work\prints"
$env:RAPIDR_CONFIG_DIR = "$work\config"
$env:RAPIDR_AI_KEYSTORE = "memory"
Write-Host "== $((& $Exe version | Out-String).Trim()) ($Exe)"
Write-Host "== Windows $([Environment]::OSVersion.Version), $env:PROCESSOR_ARCHITECTURE, 64-bit process: $([Environment]::Is64BitProcess)"
& $Exe run-bc "$work\$name" 2>&1 | ForEach-Object { "$_" }
Write-Host "== exit $LASTEXITCODE"
