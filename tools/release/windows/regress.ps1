# The conformance suite (the bytecode VM) and the examples (`rapidr run`) with an INSTALLED (or staged)
# rapidr on Windows: a copy of the exe beside the tests, its home named (RAPIDR_HOME), the tests and
# examples copied from a source folder into a temporary one (the harness writes beside them).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File regress.ps1 [-Prefix <install or stage folder>] [-Source <folder with tests\ and examples\>]
#   tools/release/windows-vm.sh regress [Prefix=<folder in the VM>] [Source=<folder>]
#
# Defaults: the Prefix is the build's staged ARM64 SDK (%USERPROFILE%\rapidr-release\stage\sdk-aarch64),
# the Source the release's own (%USERPROFILE%\rapidr-release\src). smoke.ps1 -Regress runs it on the install
# it has made. Every failure is printed with its detail. Exit status 0 = none failed. Needs node.exe
# (%USERPROFILE%\node), and for a release build the test-mode token (%USERPROFILE%\.rapidr\test-mode.token).
# Nothing here prints, touches the user's registry store or settings; the temporary folder is removed.
# (This file is read as ANSI by Windows PowerShell: no character outside ASCII in its code.)
param(
    [string]$Prefix = "$env:USERPROFILE\rapidr-release\stage\sdk-aarch64",
    [string]$Source = "$env:USERPROFILE\rapidr-release\src"
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$node = "$env:USERPROFILE\node\node.exe"
if (-not (Test-Path $node)) { $nc = Get-Command node.exe -ErrorAction SilentlyContinue; if ($nc) { $node = $nc.Source } }
if (-not (Test-Path $node)) { Write-Host "no node.exe: skipped"; exit 0 }
if (-not (Test-Path "$Source\tests\conformance\run.mjs")) { Write-Host "no tests in ${Source}: skipped"; exit 0 }
$exe = "$Prefix\bin\rapidr.exe"
if (-not (Test-Path $exe)) { Write-Host "no rapidr.exe in ${Prefix}\bin"; exit 1 }
$T = Join-Path $env:TEMP ("rapidr-regress-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force "$T\prints" | Out-Null
$fail = 0
try {
    # (robocopy: the tests' and examples' files, without a work folder, node_modules or old results)
    & robocopy "$Source\tests" "$T\tests" /E /XD .work node_modules results screenshots /NFL /NDL /NJH /NJS /NP | Out-Null
    & robocopy "$Source\examples" "$T\examples" /E /NFL /NDL /NJH /NJS /NP | Out-Null
    Copy-Item $exe "$T\rapidr.exe"
    $env:RAPIDR_HOME = "$Prefix\lib\rapidr"
    $env:RAPIDR_PRINT_TO = "$T\prints"; $env:RAPIDR_REGISTRY = "$T\registry.reg"; $env:RAPIDR_AI_KEYSTORE = "memory"
    $env:RAPIDR_BUILD_CACHE = "$T\build-cache"; $env:LANGUAGE = "en_US.UTF-8"; $env:RAPIDR_LANG = "en"
    if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
    Write-Host "== $((& $exe version | Out-String).Trim()) from $Prefix; tests from $Source"
    Push-Location $T
    try { $oe = [Console]::OutputEncoding; [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch { $oe = $null }
    Write-Host "== conformance (the bytecode VM)"
    & $node tests\conformance\run.mjs --backend vm *> "$T\conformance.txt"
    $c = Get-Content "$T\conformance.txt"
    $c | Where-Object { $_ -match "^FAIL|^XPASS|passed," } | Select-Object -First 60 | Write-Host
    if (-not (($c -join "`n") -match "\d+ passed, \d+ known failures \(xfail\), 0 failed")) { $fail = 1 }
    Write-Host "== examples (rapidr run)"
    & $node tests\examples_run.mjs --runtimes run *> "$T\examples.txt"
    if ($oe) { try { [Console]::OutputEncoding = $oe } catch { } }
    # (a failed example: U+2717 and its name, then its detail on indented lines)
    $show = $false
    Get-Content "$T\examples.txt" | ForEach-Object {
        if ($_.Length -gt 0 -and [int]$_[0] -eq 0x2717) { $show = $true; $_ | Write-Host }
        elseif ($_ -match "^Examples:") { $_ | Write-Host }
        elseif ($show -and $_ -match "^\s{4}") { $_ | Write-Host }
        else { $show = $false }
    }
    if (-not ((Get-Content -Raw "$T\examples.txt") -match "Examples: \d+ checks passed, 0 failed")) { $fail = 1 }
    Pop-Location
}
finally {
    Set-Location $env:SystemRoot
    cmd /c "rmdir /s /q `"$T`" 2>nul" | Out-Null
}
if ($fail -eq 0) { Write-Host "regression: all passed" } else { Write-Host "regression: FAILED" }
exit $fail
