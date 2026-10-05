# Installs a RapidR Windows installer into a temporary folder and checks it
# works as a user's install would (tools/release/smoke.sh's checks, for
# Windows), then uninstalls it.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File smoke.ps1 -Installer <...-setup.exe>
#       [-Associations] [-Native]
#
# By default nothing is registered: the file types and PATH are tasks the
# test leaves out (/TASKS=""). -Associations installs the file types too —
# into this user's HKCU\Software\Classes, for the test's length — checks
# them, and checks the uninstaller removes them: use it in a test VM.
# -Native also builds a program natively (Rust and the linker needed).
param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [switch]$Associations,
    [switch]$Native
)
$ErrorActionPreference = "Stop"
$T = Join-Path $env:TEMP ("rapidr-smoke-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force "$T\work", "$T\prints" | Out-Null
$env:RAPIDR_PRINT_TO = "$T\prints"; $env:RAPIDR_REGISTRY = "$T\registry.reg"; $env:RAPIDR_CONFIG_DIR = "$T\config"
$script:fail = 0
function Check($what, [scriptblock]$test) {
    $ok = $false
    try { $ok = [bool](& $test) } catch { $ok = $false }
    if ($ok) { Write-Host "  ok    $what" } else { Write-Host "  FAIL  $what"; $script:fail = 1 }
}
function Out-Of($exe, [string[]]$a) { (& $exe @a 2>&1 | Out-String) }

Write-Host "== install $(Split-Path -Leaf $Installer) into $T\app"
$pathBefore = [Environment]::GetEnvironmentVariable("Path", "User")
$tasks = if ($Associations) { "/TASKS=associate" } else { "/TASKS=" }
Start-Process -Wait -FilePath $Installer -ArgumentList @("/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CURRENTUSER", "/DIR=$T\app", "/LOG=$T\install.log", $tasks)
$R = "$T\app\bin\rapidr.exe"; $RW = "$T\app\bin\rapidrw.exe"
Check "installed: rapidr.exe, rapidrw.exe, lib\rapidr" { (Test-Path $R) -and (Test-Path $RW) -and (Test-Path "$T\app\lib\rapidr\release.toml") }
Check "licence files" { (Test-Path "$T\app\share\doc\rapidr\LICENSE") -and (Test-Path "$T\app\share\doc\rapidr\THIRD_PARTY_NOTICES.md") }
Check "PATH untouched without the task" { [Environment]::GetEnvironmentVariable("Path", "User") -eq $pathBefore }
$kind = if ((Get-Content "$T\app\lib\rapidr\release.toml") -match 'kind = "sdk"') { "sdk" } else { "runtime" }
Write-Host "== $kind`: $(Out-Of $R @('version'))"
Set-Location "$T\work"

Write-Host "== interpreted programs"
Set-Content hello.bas "`$APPTYPE CONSOLE`r`nPRINT `"hello `"; COMMAND`$`r`nPRINT Application.ExeName`r`n"
Check "rapidr run hello.bas a b" { (Out-Of $R @("run", "hello.bas", "a", "b")) -match "hello a b" }
& $R build-bc hello.bas -o hello.rrbc | Out-Null
Check "rapidr hello.rrbc" { (Out-Of $R @("hello.rrbc", "z")) -match "hello z" }
Check "rapidr info: console" { (Out-Of $R @("info", "hello.rrbc")) -match "apptype: console" }
$b = [IO.File]::ReadAllBytes("$T\work\hello.rrbc"); $b[10] = 99; $b[11] = 0
[IO.File]::WriteAllBytes("$T\work\newer.rrbc", $b)
Check "a program for a newer runtime says which" { (Out-Of $R @("run", "newer.rrbc")) -match "needs RapidR Runtime 99" }

Write-Host "== a GUI program, headless"
Set-Content gui.bas "CREATE Form AS QFORM`r`n  Caption = `"smoke`"`r`n  CREATE Lbl AS QLABEL`r`n    Caption = `"ready`"`r`n  END CREATE`r`nEND CREATE`r`nForm.ShowModal`r`n"
$env:RAPIDR_CAPTURE = "$T\work\cap"; $env:RAPIDR_CAPTURE_DELAY = "0.3"; $env:RAPIDR_TEST_DUMP = "lbl.caption"
Check "GUI program runs (capture)" { (Out-Of $R @("run", "gui.bas")) -match "lbl.caption=ready" }
Check "rapidrw.exe runs a windowed program" { $p = Start-Process -Wait -PassThru -FilePath $RW -ArgumentList "gui.bas"; $p.ExitCode -eq 0 }
Remove-Item env:RAPIDR_TEST_DUMP

Write-Host "== a downloaded file asks once (Mark of the Web)"
Copy-Item hello.bas downloaded.bas
Set-Content -Path "downloaded.bas" -Stream Zone.Identifier -Value "[ZoneTransfer]`r`nZoneId=3`r`nHostUrl=https://example.com/downloaded.bas"
Check "rapidr info: downloaded" { (Out-Of $R @("info", "downloaded.bas")) -match "downloaded: yes" }
$env:RAPIDR_CAPTURE_DELAY = "0.2"
$env:RAPIDR_TEST_MESSAGE_DIALOG = "Yes"
Check "asked, answered Yes: runs" { ("" | & $R run downloaded.bas first 2>&1 | Out-String) -match "hello first" }
Check "the answer is kept" { (Get-Content "$T\config\trusted-files.txt") -match " run " }
$env:RAPIDR_TEST_MESSAGE_DIALOG = "No"
Check "not asked again" { ("" | & $R run downloaded.bas second 2>&1 | Out-String) -match "hello second" }
Remove-Item env:RAPIDR_TEST_MESSAGE_DIALOG, env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY

function Subsystem($exe) {
    $b = [IO.File]::ReadAllBytes($exe); $pe = [BitConverter]::ToInt32($b, 0x3C)
    [BitConverter]::ToUInt16($b, $pe + 0x5C)    # 2: windowed, 3: console
}
if ($kind -eq "sdk") {
    Write-Host "== standalone interpreted executables"
    & $R build hello.bas --interp | Out-Null
    Check "console program: a console executable" { (Subsystem "$T\work\hello.exe") -eq 3 }
    Check "it runs" { (Out-Of "$T\work\hello.exe" @("q")) -match "hello q" }
    & $R build gui.bas --interp | Out-Null
    Check "GUI program: a windowed executable (no console window)" { (Subsystem "$T\work\gui.exe") -eq 2 }
    $other = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "windows-x86_64" } else { "windows-aarch64" }
    if (Test-Path "$T\app\lib\rapidr\runners\$other") {
        New-Item -ItemType Directory -Force "$T\work\other" | Out-Null
        & $R build hello.bas "$T\work\other" --interp --target $other | Out-Null
        Check "--target $other" { Test-Path "$T\work\other\hello.exe" }
        if ($other -eq "windows-x86_64") { Check "the x64 executable runs (emulated)" { (Out-Of "$T\work\other\hello.exe" @("x")) -match "hello x" } }
    }
    $env:RAPIDR_CAPTURE = "$T\work\ide"; $env:RAPIDR_CAPTURE_DELAY = "0.5"; $env:RAPIDR_TEST_DUMP = "statusbar.caption"
    Check "the IDE starts (headless)" { (Out-Of $R @("ide")) -match "statusbar.caption=Ready" }
    Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP
    if ($Native -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "== a native build (offline, the shipped sources, an empty cargo home)"
        Set-Content native.bas "`$APPTYPE CONSOLE`r`nPRINT `"native `"; 6 * 7`r`n"
        $env:CARGO_HOME = "$T\cargo-home"; $env:CARGO_TARGET_DIR = "$T\native-target"
        & $R build native.bas *> native.log
        Check "rapidr build (native)" { (Out-Of "$T\work\native.exe" @()) -match "native 42" }
        Remove-Item env:CARGO_HOME, env:CARGO_TARGET_DIR
    }
} else {
    Check "no executables: says it's the runtime" { (Out-Of $R @("build", "hello.bas", "--interp")) -match "This is the RapidR Runtime" }
}

if ($Associations) {
    Write-Host "== file types (HKCU\Software\Classes)"
    Check ".rrbc is a RapidR program" { (Get-ItemProperty "HKCU:\Software\Classes\.rrbc")."(default)" -eq "RapidR.Program" }
    Check "it runs with rapidrw.exe" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Program\shell\open\command")."(default)" -like "*rapidrw.exe*%1*" }
    Check ".rr / .bas: a source with a Run action" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Source\shell\run\command")."(default)" -like "*rapidrw.exe*" }
    if ($kind -eq "sdk") { Check "... opened in the IDE" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Source\shell\open\command")."(default)" -like "*--ide*" } }
}

Write-Host "== uninstall"
Set-Location $env:TEMP
Start-Process -Wait -FilePath "$T\app\unins000.exe" -ArgumentList @("/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART")
Start-Sleep -Seconds 2
Check "files removed" { -not (Test-Path "$T\app\bin\rapidr.exe") -and -not (Test-Path "$T\app\lib") }
if ($Associations) {
    Check "file types removed" { -not (Test-Path "HKCU:\Software\Classes\RapidR.Program") -and -not (Test-Path "HKCU:\Software\Classes\RapidR.Source") }
}
Check "PATH as before" { [Environment]::GetEnvironmentVariable("Path", "User") -eq $pathBefore }
Remove-Item -Recurse -Force $T -ErrorAction SilentlyContinue
if ($script:fail) { Write-Host "== smoke test FAILED"; exit 1 } else { Write-Host "== smoke test passed" }
