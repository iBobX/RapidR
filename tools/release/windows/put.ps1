# Copies one file from the Mac's home share (\\Mac\Home, only read) into the user's
# %USERPROFILE%\rapidr-release\out in this VM, and prints its SHA-256
# (tools/release/windows-vm.sh put). Skips the copy when the file is already there with the same
# SHA-256 (-Sum): an installer is 200 MB.
#
#   put.ps1 -From <\\Mac\Home\...\file> -Name <file name in out\> [-Sum <sha256 it should have>]
param(
    [Parameter(Mandatory = $true)][string]$From,
    [Parameter(Mandatory = $true)][string]$Name,
    [string]$Sum = ""
)
$ErrorActionPreference = "Stop"
. "$PSScriptRoot\user.ps1"
$out = "$UserProfile\rapidr-release\out"
New-Item -ItemType Directory -Force $out | Out-Null
$to = "$out\$Name"
if ($Sum -and (Test-Path $to) -and ((Get-FileHash -Algorithm SHA256 $to).Hash.ToLower() -eq $Sum.ToLower())) {
    Write-Host "already there: $to"
} else {
    Copy-Item -Force -LiteralPath $From -Destination $to
    Write-Host "copied: $to"
}
Write-Host ("sha256 " + (Get-FileHash -Algorithm SHA256 $to).Hash.ToLower())
exit 0
