# Unzips the Windows 7 / 8 / 8.1 alpha package (tools/win7/alpha.sh: RapidR-<ver>-win7-x86-alpha<N>.zip)
# into a temporary folder and checks it as a tester would use it, then removes everything.
# It runs in the Windows 11 VM, where the 32-bit program runs by x86 emulation: the result says
# what the PACKAGE does (the files are right, Build works, the built program starts), not that
# Windows 7 itself accepts it (the VM has no Windows 7: tools/win7/README.md tests that by hand;
# the build matrix adds a static check of the import tables for what Windows 7 lacks).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File smoke-win7.ps1 -Zip <...-win7-x86-alpha1.zip> [-Shots <folder>]
#
# Checks (the cells of tools/release/build-matrix.sh):
#   run      rapidr.exe version, __win7_check, a console program and a GUI program run (headless captures)
#   interp   Build (interpreted) of a console and of a GUI program, from a Command Prompt that has no
#            Rust (PATH and CARGO_HOME point nowhere: a Windows 7 PC) and, the way a person does it,
#            with nothing set but what the package's own launcher sets
#   exe      the programs that build made start and print / show what they should
#   other    Studio starts headless (the installed layout: bin\ and lib\rapidr\, or the flat alpha 1
#            layout with RAPIDR_HOME set the way its .bat files do)
#   native   not part of this package (no Rust for Windows 7): the cell stays "-"
# The test hooks of a release build act only with a signed test-mode token (SEC-24,
# docs/security/test-mode.md): RAPIDR_TEST_MODE, or %USERPROFILE%\.rapidr\test-mode.token.
# Everything made (the unzipped folder, the programs, the captures) is removed at the end, even when a
# check failed; -Shots <folder> first copies the pictures the programs captured there.
# The last line before the verdict, "##cells run=ok interp=ok ...", is what the build matrix reads.
param(
    [Parameter(Mandatory = $true)][string]$Zip,
    [string]$Shots = ""
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$T = Join-Path $env:TEMP ("rapidr-smoke-win7-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force "$T\work", "$T\prints" | Out-Null
$env:RAPIDR_PRINT_TO = "$T\prints"; $env:RAPIDR_REGISTRY = "$T\registry.reg"; $env:RAPIDR_CONFIG_DIR = "$T\config"
$env:RAPIDR_BUILD_CACHE = "$T\build-cache"; $env:RAPIDR_AI_KEYSTORE = "memory"
if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
$script:fail = 0
$script:cell = "other"; $script:cellsBad = @(); $script:cellsSeen = @()
function Cells-Line {
    "##cells " + ((@("run", "interp", "exe", "native", "nexe", "other") | ForEach-Object {
        "$_=" + $(if ($script:cellsBad -contains $_) { "FAIL" } elseif ($script:cellsSeen -contains $_) { "ok" } else { "-" })
    }) -join " ")
}
trap { Write-Host "  FAIL  unexpected error: $_"; $script:fail = 1; $script:cellsBad += $script:cell; $script:cellsSeen += $script:cell; continue }
function Check($what, [scriptblock]$test) {
    $ok = $false
    try { $ok = [bool](& $test) } catch { $ok = $false }
    $script:cellsSeen += $script:cell
    if ($ok) { Write-Host "  ok    $what" } else { Write-Host "  FAIL  $what"; $script:fail = 1; $script:cellsBad += $script:cell }
}
function Out-Of($exe, [string[]]$a) { (& $exe @a 2>&1 | Out-String) }
# The line of a build's answer that says what went wrong (what a person reads first): an error, not
# the warning before it.
function First-Error($text) {
    $lines = @($text -split "`r?`n" | ForEach-Object { ($_ -replace "^\S+\.exe : ", "").Trim() } | Where-Object { $_ -and $_ -notmatch "^(At line|\+|CategoryInfo|FullyQualifiedErrorId)" })
    $line = $lines | Where-Object { $_ -match "error|could not|cannot|no runner|failed" -and $_ -notmatch "^warning" } | Select-Object -First 1
    if (-not $line) { $line = $lines | Select-Object -First 1 }
    "$line"
}
function Subsystem($exe) {
    $b = [IO.File]::ReadAllBytes($exe); $pe = [BitConverter]::ToInt32($b, 0x3C)
    [BitConverter]::ToUInt16($b, $pe + 0x5C)    # 2: windowed, 3: console
}

try {
    Write-Host "== unzip $(Split-Path -Leaf $Zip) into $T\z"
    $sumFile = "$Zip.sha256"
    if (Test-Path $sumFile) {
        $want = ((Get-Content $sumFile | Select-Object -First 1) -split "\s+")[0].ToLower()
        Check "the zip matches its .sha256" { (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLower() -eq $want }
    }
    Expand-Archive -Path $Zip -DestinationPath "$T\z" -Force
    $root = (Get-ChildItem "$T\z" -Directory | Select-Object -First 1).FullName
    # The installed layout (bin\rapidr.exe + lib\rapidr\release.toml: the home is found with nothing set)
    # or alpha 1's flat one (rapidr.exe beside Studio's files; its .bat files set RAPIDR_HOME).
    $installed = Test-Path "$root\bin\rapidr.exe"
    $R = if ($installed) { "$root\bin\rapidr.exe" } else { "$root\rapidr.exe" }
    Check "rapidr.exe is in the package" { Test-Path $R }
    Write-Host ("  layout: " + $(if ($installed) { "installed (bin\, lib\rapidr\)" } else { "flat (alpha 1: RAPIDR_HOME set by its .bat files)" }))
    if ($installed) {
        Check "the home is complete: release.toml, the IDE, the examples, a runner for windows-x86" {
            (Test-Path "$root\lib\rapidr\release.toml") -and (Test-Path "$root\lib\rapidr\ide\rapidr-ide.rrbc") -and (Test-Path "$root\lib\rapidr\examples") -and
            (Test-Path "$root\lib\rapidr\runners\windows-x86\rapidrintr-runner.exe") -and (Test-Path "$root\lib\rapidr\runners\windows-x86\rapidrintr-runnerw.exe")
        }
    } else {
        $env:RAPIDR_HOME = $root
    }
    Check "licence files and the notices" { (Test-Path "$root\LICENSE") -and (Test-Path "$root\LEGAL.md") -and (Test-Path "$root\THIRD-PARTY-NOTICES.txt") }
    Set-Location "$T\work"

    Write-Host "== run"
    $script:cell = "run"
    $ver = Out-Of $R @("version")
    Write-Host "  $($ver.Trim())"
    Check "rapidr.exe version" { $ver -match "RapidR \d+\.\d+\.\d+" }
    Write-Host (Out-Of $R @("__win7_check"))
    Check "__win7_check runs (the six functions only newer Windows has)" { (Out-Of $R @("__win7_check")) -match "RoGetActivationFactory" }
    Set-Content hello.bas "`$APPTYPE CONSOLE`r`nPRINT `"hello `"; COMMAND`$`r`n"
    Check "rapidr run hello.bas a b" { (Out-Of $R @("run", "hello.bas", "a", "b")) -match "hello a b" }
    Set-Content gui.bas "CREATE Form AS QFORM`r`n  Caption = `"smoke`"`r`n  CREATE Lbl AS QLABEL`r`n    Caption = `"ready`"`r`n  END CREATE`r`nEND CREATE`r`nForm.ShowModal`r`n"
    $env:RAPIDR_CAPTURE = "$T\work\cap"; $env:RAPIDR_CAPTURE_DELAY = "0.5"; $env:RAPIDR_TEST_DUMP = "lbl.caption"
    Check "GUI program runs (capture)" { (Out-Of $R @("run", "gui.bas")) -match "lbl.caption=ready" }
    Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP

    Write-Host "== Build (interpreted), on a PC with no Rust (PATH and CARGO_HOME point nowhere)"
    $script:cell = "interp"
    $pathBefore = $env:PATH
    $env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"; $env:CARGO_HOME = "$T\no-cargo"; $env:RUSTUP_HOME = "$T\no-rustup"
    $b1 = Out-Of $R @("build", "hello.bas", "--interp")
    Check "console program: rapidr build --interp makes hello.exe" { Test-Path "$T\work\hello.exe" }
    if (-not (Test-Path "$T\work\hello.exe")) { Write-Host "        $(First-Error $b1)" }
    Check "... a console executable" { (Test-Path "$T\work\hello.exe") -and ((Subsystem "$T\work\hello.exe") -eq 3) }
    $b2 = Out-Of $R @("build", "gui.bas", "--interp")
    Check "GUI program: rapidr build --interp makes gui.exe" { Test-Path "$T\work\gui.exe" }
    if (-not (Test-Path "$T\work\gui.exe")) { Write-Host "        $(First-Error $b2)" }
    Check "... a windowed executable (no console window)" { (Test-Path "$T\work\gui.exe") -and ((Subsystem "$T\work\gui.exe") -eq 2) }
    Check "... with its notices beside it" { Test-Path "$T\work\THIRD-PARTY-NOTICES.txt" }
    # (what Studio's Welcome page does with an example: Build Interpreted App on a program of the package's own)
    $example = "$root\lib\rapidr\examples\gui\hello_form.rr"
    if (Test-Path $example) {
        $b5 = Out-Of $R @("build", $example, "$T\work\ex", "--interp")
        Check "an example of the package builds: hello_form.exe" { Test-Path "$T\work\ex\hello_form.exe" }
        if (-not (Test-Path "$T\work\ex\hello_form.exe")) { Write-Host "        $(First-Error $b5)" }
    }
    # (a Windows 7 PC in a Command Prompt of its own: nothing set by the package's launcher)
    if (-not $installed) {
        $savedHome = $env:RAPIDR_HOME; Remove-Item env:RAPIDR_HOME
        New-Item -ItemType Directory -Force "$T\work\plain" | Out-Null
        $b3 = Out-Of $R @("build", "hello.bas", "$T\work\plain", "--interp")
        Check "from a plain Command Prompt too (RAPIDR_HOME not set)" { Test-Path "$T\work\plain\hello.exe" }
        if (-not (Test-Path "$T\work\plain\hello.exe")) { Write-Host "        $(First-Error $b3)" }
        $env:RAPIDR_HOME = $savedHome
    }
    $env:PATH = $pathBefore; Remove-Item env:CARGO_HOME, env:RUSTUP_HOME

    Write-Host "== the programs it built run (32-bit, by emulation here)"
    $script:cell = "exe"
    Check "hello.exe prints" { (Out-Of "$T\work\hello.exe" @("q")) -match "hello" }
    $env:RAPIDR_CAPTURE = "$T\work\built"; $env:RAPIDR_CAPTURE_DELAY = "0.5"; $env:RAPIDR_TEST_DUMP = "lbl.caption"
    Check "gui.exe shows its window (capture)" { (Test-Path "$T\work\gui.exe") -and ((Out-Of "$T\work\gui.exe" @()) -match "lbl.caption=ready") }
    if (Test-Path $example) {
        $env:RAPIDR_CAPTURE = "$T\work\example"
        Check "hello_form.exe shows its window (capture)" { (Test-Path "$T\work\ex\hello_form.exe") -and ((Out-Of "$T\work\ex\hello_form.exe" @()) -match "captured window") }
    }
    Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP

    Write-Host "== Studio, headless"
    $script:cell = "other"
    $env:RAPIDR_CAPTURE = "$T\work\ide"; $env:RAPIDR_CAPTURE_DELAY = "1"; $env:RAPIDR_TEST_DUMP = "Studio.caption"
    Check "the IDE starts" { (Out-Of $R @("ide")) -match "Studio.caption=RapidR Studio" }
    Check "the IDE opens a file" { (Out-Of $R @("ide", "$T\work\hello.bas")) -match "Studio.caption=hello - RapidR Studio" }
    Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP
    Write-Host "  (Build Native: not part of the Windows 7 package: no Rust for Windows 7)"
} finally {
    Set-Location $env:SystemRoot
    if ($Shots) {
        # (a name, not a path: below %USERPROFILE%\rapidr-release\out, where windows-vm.sh fetch reads)
        if (-not [IO.Path]::IsPathRooted($Shots)) { $Shots = "$env:USERPROFILE\rapidr-release\out\$Shots" }
        New-Item -ItemType Directory -Force $Shots | Out-Null
        Get-ChildItem "$T\work" -Filter "*.bmp" -ErrorAction SilentlyContinue | ForEach-Object { Copy-Item $_.FullName "$Shots\win7-$($_.Name)" -Force }
    }
    Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -like "$T\*" } | Stop-Process -Force -ErrorAction SilentlyContinue
    # (a program just stopped may still hold its folder for a moment)
    for ($try = 0; $try -lt 8 -and (Test-Path $T); $try++) {
        cmd /c "rmdir /s /q `"$T`" 2>nul" | Out-Null
        if (Test-Path $T) { Start-Sleep -Seconds 2 }
    }
}
Check "the temporary folder is gone" { -not (Test-Path $T) }
Write-Host (Cells-Line)
if ($script:fail) { Write-Host "== smoke test FAILED"; exit 1 } else { Write-Host "== smoke test passed"; exit 0 }
