# Installs a RapidR Windows installer into a temporary folder and checks it
# works as a user's install would (tools/release/smoke.sh's checks, for
# Windows), then uninstalls it.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File smoke.ps1 -Installer <...-setup.exe>
#       [-Associations] [-Native -Cases <the release's source>]
#
# The .rrbc / .rr file types are always registered (HKCU\Software\Classes, this
# user's) and the test checks the uninstaller removes them: run it in a test VM.
# PATH is left alone (/TASKS=""); -Associations also ticks "Open .bas files with
# RapidR by default". -Native runs `rapidr setup --yes` (Rust's gnullvm toolchain,
# as a user would) and builds programs natively with the LLVM-MinGW the SDK ships:
# conformance cases from -Cases (their output checked) and a GUI fixture.
param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [switch]$Associations,
    [switch]$Native,
    [string]$Cases = "$env:USERPROFILE\rapidr-release\src"
)
# (Continue: Windows PowerShell turns a native tool's stderr into errors when the output is
# redirected; failures are checked by exit code and thrown, cmdlets that matter say -ErrorAction Stop)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
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
$tasks = if ($Associations) { "/TASKS=basdefault" } else { "/TASKS=" }
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
    if ($Native) {
        Write-Host "== native builds: rapidr setup, then the shipped LLVM-MinGW (offline, an empty cargo home)"
        Write-Host (Out-Of $R @("setup", "--yes"))
        Check "setup: the shipped linker" { (Out-Of $R @("setup", "--check")) -match "linker: LLVM-MinGW, shipped" }
        $env:CARGO_HOME = "$T\cargo-home"; $env:CARGO_TARGET_DIR = "$T\native-target"
        $readobj = "$T\app\lib\rapidr\toolchain\bin\llvm-readobj.exe"
        foreach ($case in "arithmetic", "arrays", "control_flow", "functions", "gosub_goto", "data_read") {
            Copy-Item "$Cases\tests\conformance\cases\$case.bas" "$T\work\$case.bas"
            $log = Out-Of $R @("build", "$case.bas")
            $want = (Get-Content -Raw "$Cases\tests\conformance\cases\$case.expected").Replace("`r", "").TrimEnd()
            $got = if (Test-Path "$T\work\$case.exe") { (Out-Of "$T\work\$case.exe" @()).Replace("`r", "").TrimEnd() } else { $log }
            $lines = { param($x) ($x -split "`n" | ForEach-Object { $_.TrimEnd() }) -join "`n" }
            Check "native $case`: output as expected" { (& $lines $got) -eq (& $lines $want) }
            if (-not (Test-Path "$T\work\$case.exe")) { Write-Host (($log -split "`n" | Select-Object -Last 25) -join "`n") }
        }
        if (Test-Path $readobj) {
            $dlls = & $readobj --coff-imports "$T\work\arithmetic.exe" | Select-String "Name: (.*\.dll)" | ForEach-Object { $_.Matches[0].Groups[1].Value.ToLower() } | Sort-Object -Unique
            Write-Host "  arithmetic.exe imports: $($dlls -join ', ')"
            Check "native executables need no MinGW / LLVM DLL" { -not ($dlls | Where-Object { $_ -match "unwind|c\+\+|winpthread|gcc|stdc" }) }
        }
        Copy-Item "$Cases\tests\fixtures\list_items.bas" "$T\work\list_items.bas"
        Out-Of $R @("build", "list_items.bas") | Out-Null
        $env:RAPIDR_CAPTURE = "$T\work\gui"; $env:RAPIDR_CAPTURE_DELAY = "0.5"
        Check "native GUI fixture runs (capture)" { (Test-Path "$T\work\list_items.exe") -and ((Out-Of "$T\work\list_items.exe" @()) -match "captured window") }
        Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:CARGO_HOME, env:CARGO_TARGET_DIR
        Remove-Item -Recurse -Force "$T\native-target", "$T\cargo-home" -ErrorAction SilentlyContinue
    }
} else {
    Check "no executables: says it's the runtime" { (Out-Of $R @("build", "hello.bas", "--interp")) -match "This is the RapidR Runtime" }
}

Write-Host "== file types (HKCU\Software\Classes)"
Check ".rrbc is a RapidR program" { (Get-ItemProperty "HKCU:\Software\Classes\.rrbc")."(default)" -eq "RapidR.Program" }
Check "it runs with rapidrw.exe" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Program\shell\open\command")."(default)" -like "*rapidrw.exe*%1*" }
Check ".rr is RapidR source" { (Get-ItemProperty "HKCU:\Software\Classes\.rr")."(default)" -eq "RapidR.Source" }
Check ".bas lists RapidR under Open with" { $null -ne (Get-ItemProperty "HKCU:\Software\Classes\.bas\OpenWithProgids")."RapidR.Source" }
if ($Associations) {
    Check ".bas is RapidR's by default (ticked)" { (Get-ItemProperty "HKCU:\Software\Classes\.bas")."(default)" -eq "RapidR.Source" }
} else {
    Check ".bas is not RapidR's by default (unticked)" { (Get-ItemProperty "HKCU:\Software\Classes\.bas" -ErrorAction SilentlyContinue)."(default)" -ne "RapidR.Source" }
}
Check "source: a Run action" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Source\shell\run\command")."(default)" -like "*rapidrw.exe*" }
if ($kind -eq "sdk") { Check "... opened in the IDE" { (Get-ItemProperty "HKCU:\Software\Classes\RapidR.Source\shell\open\command")."(default)" -like "*--ide*" } }
Check "the icons" { Test-Path "$T\app\share\icons\rapidr-doc.ico" }

Write-Host "== uninstall"
Set-Location $env:TEMP
Start-Process -Wait -FilePath "$T\app\unins000.exe" -ArgumentList @("/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART")
Start-Sleep -Seconds 2
Check "files removed" { -not (Test-Path "$T\app\bin\rapidr.exe") -and -not (Test-Path "$T\app\lib") }
Check "file types removed" { -not (Test-Path "HKCU:\Software\Classes\RapidR.Program") -and -not (Test-Path "HKCU:\Software\Classes\RapidR.Source") -and ((Get-ItemProperty "HKCU:\Software\Classes\.rrbc" -ErrorAction SilentlyContinue)."(default)" -ne "RapidR.Program") }
Check "PATH as before" { [Environment]::GetEnvironmentVariable("Path", "User") -eq $pathBefore }
Remove-Item -Recurse -Force $T -ErrorAction SilentlyContinue
if ($script:fail) { Write-Host "== smoke test FAILED"; exit 1 } else { Write-Host "== smoke test passed"; exit 0 }
