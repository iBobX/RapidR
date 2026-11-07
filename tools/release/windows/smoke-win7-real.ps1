# smoke-win7-real.ps1 - a test script for RapidR's own builds (RapidR's release checks, MIT licence).
#
# WHAT IT DOES
#   Takes the Windows 7 / 8 / 8.1 alpha zip of RapidR (tools/win7/alpha.sh: RapidR-<ver>-win7-x86-alpha<N>.zip),
#   unpacks it into a temporary folder of its own and uses it as a tester would: rapidr.exe runs a console and a
#   window program, Build (interpreted) makes two programs on a PC with no Rust, the programs it made are started,
#   and RapidR Studio starts. Then it removes everything it made.
#   tools/release/win7-vm.sh runs it in the UTM VM "Windows 7 x86" (a REAL Windows 7); tools/release/build-matrix.sh
#   reads its log as the row "Windows 7 (real)". Its sister smoke-win7.ps1 does the same in the Windows 11 VM,
#   where the 32-bit program runs by emulation and Windows 7 itself is not checked; this one checks what only a real
#   Windows 7 can say: that its loader accepts the programs (rapidr.exe and the two runner programs: no
#   "api-ms-win-core-...dll is missing", no "entry point not found"), that the six functions Windows 7 lacks are
#   answered by RapidR's stand-ins, and that the programs Build made run and show a window.
#
# WHAT IT NEVER DOES (it is a test script, not an installer)
#   - no network access and no download; nothing is encoded, obfuscated or built at run time (no Add-Type with
#     C# source, no Invoke-Expression, no reflection tricks, no P/Invoke); it only calls PowerShell and .NET;
#   - no change to the machine: no registry, service, scheduled task, firewall, antivirus or security setting, and no
#     execution-policy change (the policy is not touched here; see "HOW IT IS STARTED");
#   - no hidden window: the one window it opens is the test program's own, on purpose (see the picture step);
#   - it starts only programs of the zip and programs RapidR's Build made from two tiny test programs, stops only
#     those (by process id) and deletes only its own temporary folder t-<8 hex digits> inside -Work.
#
# HOW IT IS STARTED
#   tools/release/win7-vm.sh runs it as the user who is logged in at the VM's desktop, through a one-off scheduled
#   task that it deletes afterwards (the VM's guest agent itself runs in session 0, which has no desktop, so a window
#   would never show). That command line is
#     powershell -NoProfile -ExecutionPolicy Bypass -File smoke.ps1 -Zip <zip> [-Sha256 <hex>] [-Expect <n>] [-Token <file>] [-Work <folder>] [-Shots <folder>]
#   The policy is relaxed for that one process only (a stock Windows 7 refuses scripts that are not signed); nothing in
#   this script changes it. PowerShell 2.0 (what Windows 7 has) is the target: no Expand-Archive, no Get-FileHash.
#
# THE UNPACKING
#   Windows' own "Compressed (zipped) Folders" does it (Shell.Application: what Explorer's "Extract All" uses), so the
#   zip is opened the way a person opens it. It copies in the background, so the folder is watched until every file
#   is there. -Expect is the number of files the zip holds (win7-vm.sh counts them on the Mac); without it the script
#   waits until the folder stops growing.
#
# Checks (the cells of build-matrix.sh):
#   run      rapidr.exe version, __win7_check, a console program and a GUI program run (headless capture)
#   interp   Build (interpreted) of a console and of a GUI program on a PC with no Rust (PATH and CARGO_HOME point nowhere)
#   exe      the programs that build made start, print, and the GUI one shows a window (a picture of the desktop is kept)
#   other    the OS is Windows 7 (6.1); the zip is whole; the loader accepts every program of the zip; Studio starts headless
#   native   not part of this package (no Rust for Windows 7): the cell stays "-"
# The test hooks of a release build (captures, dumps) act only with a signed test-mode token (SEC-24,
# docs/security/test-mode.md): -Token <file>. Without it the GUI checks cannot run and say so.
# Everything made (the unpacked folder, the programs, the captures) is removed at the end, even when a check
# failed; the pictures are first copied to -Shots. The last line before the verdict,
# "##cells run=ok interp=ok ...", is what the build matrix reads.
param(
    [string]$Zip = "",
    [string]$Sha256 = "",
    [int]$Expect = 0,
    [string]$Token = "",
    [string]$Work = "C:\rapidr-matrix",
    [string]$Shots = ""
)
$ErrorActionPreference = "Continue"
$T = Join-Path $Work ("t-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force "$T\work", "$T\prints" | Out-Null
if (-not $Shots) { $Shots = "$Work\shots" }
# (every program run here has its own printer folder, registry file and settings: nothing of the user's is touched)
$env:RAPIDR_PRINT_TO = "$T\prints"; $env:RAPIDR_REGISTRY = "$T\registry.reg"; $env:RAPIDR_CONFIG_DIR = "$T\config"
$env:RAPIDR_BUILD_CACHE = "$T\build-cache"; $env:RAPIDR_AI_KEYSTORE = "memory"
$hooks = $false
if ($Token -and (Test-Path $Token)) { $env:RAPIDR_TEST_MODE = $Token; $hooks = $true }
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
# Runs a program with a time limit (a GUI program stuck behind a loader error box is stopped, by its process
# id, and says so); its output (both streams) and exit code come back.
function Run-Timed([string]$exe, [string]$argLine = "", [int]$sec = 120) {
    $so = "$T\o-" + [guid]::NewGuid().ToString("N").Substring(0, 6) + ".txt"
    $se = "$so.err"
    $p = @{ FilePath = $exe; RedirectStandardOutput = $so; RedirectStandardError = $se; PassThru = $true }
    if ($argLine) { $p.ArgumentList = $argLine }
    $timedOut = $false; $code = -1; $text = ""
    try {
        $proc = Start-Process @p
        $null = $proc.Handle
        if (-not $proc.WaitForExit($sec * 1000)) { $timedOut = $true; Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; $null = $proc.WaitForExit(5000) }
        $code = $proc.ExitCode
    } catch { $text = "could not start $exe : $_`n" }
    foreach ($f in @($so, $se)) { if (Test-Path $f) { $text += [IO.File]::ReadAllText($f) } }
    New-Object PSObject -Property @{ Out = $text; Code = $code; TimedOut = $timedOut }
}
# The line of a build's answer that says what went wrong (what a person reads first).
function First-Error($text) {
    $lines = @($text -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $line = $lines | Where-Object { $_ -match "error|could not|cannot|no runner|failed" -and $_ -notmatch "^warning" } | Select-Object -First 1
    if (-not $line) { $line = $lines | Select-Object -First 1 }
    "$line"
}
# 2: a windowed program, 3: a console program (the "subsystem" field of the exe's PE header)
function Subsystem($exe) {
    $b = [IO.File]::ReadAllBytes($exe); $pe = [BitConverter]::ToInt32($b, 0x3C)
    [BitConverter]::ToUInt16($b, $pe + 0x5C)
}
# STATUS_DLL_NOT_FOUND, STATUS_ENTRYPOINT_NOT_FOUND, STATUS_INVALID_IMAGE_FORMAT, STATUS_DLL_INIT_FAILED: the loader refused the program
function Loader-Refused($r) { $r.TimedOut -or (@(-1073741515, -1073741511, -1073741701, -1073741502) -contains $r.Code) -or ($r.Out -match "could not start") }

# Unpacks a .zip the way a person does in Explorer ("Extract All"): Windows' own Compressed Folders, through
# Shell.Application. The shell copies in the background, so the folder is watched until $expect files are there (or,
# when $expect is 0, until it stops growing) and until the last file has stopped changing size. Returns the number
# of files. Windows 7's PowerShell 2.0 has no Expand-Archive, and .NET 4.5's ZipFile is not on a stock Windows 7.
function Expand-ZipLikeExplorer([string]$zipFile, [string]$folder, [int]$expect) {
    [void](New-Item -ItemType Directory -Force $folder)
    $shell = New-Object -ComObject Shell.Application
    $source = $shell.NameSpace((Resolve-Path $zipFile).ProviderPath)
    $target = $shell.NameSpace((Resolve-Path $folder).ProviderPath)
    if (-not $source -or -not $target) { throw "Windows' Compressed Folders could not open $zipFile" }
    # flags: 4 no progress box, 16 yes to all, 512 no "create folder?" question, 1024 no error box
    $target.CopyHere($source.Items(), 4 + 16 + 512 + 1024)
    $deadline = (Get-Date).AddMinutes(15)
    $count = 0; $bytes = -1; $same = 0; $started = Get-Date
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 2
        $files = @(Get-ChildItem -Recurse -Force $folder -ErrorAction SilentlyContinue | Where-Object { -not $_.PSIsContainer })
        $now = 0; foreach ($f in $files) { $now += $f.Length }
        $same = if (($files.Count -eq $count) -and ($now -eq $bytes)) { $same + 1 } else { 0 }
        $count = $files.Count; $bytes = $now
        # (all there and nothing changing for 3 looks: done; no number given: nothing changing for 8 looks)
        if ($expect -gt 0 -and $count -ge $expect -and $same -ge 3) { break }
        if ($expect -le 0 -and $count -gt 0 -and $same -ge 8) { break }
        if ($count -eq 0 -and ((Get-Date) - $started).TotalSeconds -gt 90) { throw "nothing was unpacked in 90 s" }
        # (stuck: 90 s without a new file or a new byte; the caller's check says how many files are missing)
        if ($same -ge 45) { break }
    }
    $count
}

# A picture of the whole desktop (PNG) and, when the program's window can be found, a crop around it. Plain .NET
# from PowerShell: System.Drawing takes the picture (Graphics.CopyFromScreen) and Windows' accessibility
# interface (UI Automation) says where the program's windows are. Returns a line that says what was found.
function Save-Pictures([int]$procId, [string]$desktopFile, [string]$windowFile) {
    Add-Type -AssemblyName System.Drawing
    Add-Type -AssemblyName System.Windows.Forms
    $area = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $desktop = New-Object System.Drawing.Bitmap($area.Width, $area.Height)
    $info = "$($area.Width)x$($area.Height)"
    try {
        $canvas = [System.Drawing.Graphics]::FromImage($desktop)
        $canvas.CopyFromScreen($area.Left, $area.Top, 0, 0, $area.Size)
        $canvas.Dispose()
        $desktop.Save($desktopFile, [System.Drawing.Imaging.ImageFormat]::Png)
        try {
            Add-Type -AssemblyName UIAutomationClient
            Add-Type -AssemblyName UIAutomationTypes
            $element = [System.Windows.Automation.AutomationElement]
            $mine = New-Object System.Windows.Automation.PropertyCondition($element::ProcessIdProperty, $procId)
            $best = $null
            foreach ($w in $element::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, $mine)) {
                $r = $w.Current.BoundingRectangle
                if ($w.Current.IsOffscreen -or $r.IsEmpty) { continue }
                $info += " window '" + $w.Current.Name + "' " + [int]$r.Width + "x" + [int]$r.Height + " at " + [int]$r.X + "," + [int]$r.Y
                if (-not $best -or ($r.Width * $r.Height) -gt ($best.Width * $best.Height)) { $best = $r }
            }
            if ($best -and $windowFile) {
                # (the rectangle is the window's inside: 48 pixels around it take in the title bar and the frame, with some desktop)
                $x = [Math]::Max(0, [int]$best.X - 48); $y = [Math]::Max(0, [int]$best.Y - 48)
                $w2 = [Math]::Min($area.Width - $x, [int]$best.Width + 96); $h2 = [Math]::Min($area.Height - $y, [int]$best.Height + 96)
                if ($w2 -gt 0 -and $h2 -gt 0) {
                    $crop = $desktop.Clone((New-Object System.Drawing.Rectangle($x, $y, $w2, $h2)), $desktop.PixelFormat)
                    $crop.Save($windowFile, [System.Drawing.Imaging.ImageFormat]::Png)
                    $crop.Dispose()
                }
            }
            if (-not $best) { $info += " (no window of the program was found to crop)" }
        } catch { $info += " (the window's place is not available: $($_.Exception.Message))" }
    } finally { $desktop.Dispose() }
    $info
}

# Removes the temporary folder of this run, and nothing else: it must be t-<8 hex digits> inside -Work. A program
# just stopped may still hold its folder for a moment, so it asks again a few times.
function Remove-TestFolder([string]$folder) {
    if (($folder -notmatch '\\t-[0-9a-f]{8}$') -or (-not $folder.StartsWith($Work))) { return }
    for ($try = 0; $try -lt 8 -and (Test-Path $folder); $try++) {
        Remove-Item -Recurse -Force $folder -ErrorAction SilentlyContinue
        if (Test-Path $folder) { Start-Sleep -Seconds 2 }
    }
}

try {
    $os = Get-WmiObject Win32_OperatingSystem
    Write-Host "== the machine: $($os.Caption) $($os.CSDVersion) version $($os.Version) ($($os.OSArchitecture)), PowerShell $($PSVersionTable.PSVersion), user $env:USERNAME"
    $script:cell = "other"
    Check "this is a real Windows 7 / Server 2008 R2 (version 6.1)" { ([Environment]::OSVersion.Version.Major -eq 6) -and ([Environment]::OSVersion.Version.Minor -eq 1) }
    Check "the zip is there" { $Zip -and (Test-Path $Zip) }
    if ($Sha256) {
        Check "the zip matches its checksum" {
            $sha = New-Object Security.Cryptography.SHA256Managed
            $fs = [IO.File]::OpenRead($Zip); $hash = ([BitConverter]::ToString($sha.ComputeHash($fs)) -replace "-", "").ToLower(); $fs.Close()
            $hash -eq $Sha256.ToLower()
        }
    }
    Write-Host "== unzip $(Split-Path -Leaf $Zip) into $T\z (Windows' own Compressed Folders, as Explorer does it)"
    $t0 = Get-Date
    $n = @(Expand-ZipLikeExplorer $Zip "$T\z" $Expect)[-1]    # (the last thing it says is the number of files)
    Write-Host ("  {0} files in {1:N0} s" -f $n, ((Get-Date) - $t0).TotalSeconds)
    if ($Expect -gt 0) { Check "the zip is whole: all $Expect files were unpacked" { $n -eq $Expect } }
    $root = (Get-ChildItem "$T\z" | Where-Object { $_.PSIsContainer } | Select-Object -First 1).FullName
    $installed = Test-Path "$root\bin\rapidr.exe"
    $R = if ($installed) { "$root\bin\rapidr.exe" } else { "$root\rapidr.exe" }
    Check "rapidr.exe is in the package" { Test-Path $R }
    if ($installed) {
        Check "the home is complete: release.toml, a runner for windows-x86 (console and windowed)" {
            (Test-Path "$root\lib\rapidr\release.toml") -and (Test-Path "$root\lib\rapidr\runners\windows-x86\rapidrintr-runner.exe") -and (Test-Path "$root\lib\rapidr\runners\windows-x86\rapidrintr-runnerw.exe")
        }
    } else { $env:RAPIDR_HOME = $root }
    Check "licence files and the notices" { (Test-Path "$root\LICENSE") -and (Test-Path "$root\LEGAL.md") -and (Test-Path "$root\THIRD-PARTY-NOTICES.txt") }
    Set-Location "$T\work"

    Write-Host "== the loader of this Windows accepts the programs of the zip (no missing DLL, no missing entry point)"
    $script:cell = "other"
    $loaderTargets = @($R)
    if ($installed) { $loaderTargets += @("$root\lib\rapidr\runners\windows-x86\rapidrintr-runner.exe", "$root\lib\rapidr\runners\windows-x86\rapidrintr-runnerw.exe") }
    foreach ($exe in $loaderTargets) {
        # (the runner programs without a program inside answer with an error of their own; the loader's errors are 0xC000xxxx)
        # (not $r: PowerShell names are case-insensitive and $R is rapidr.exe)
        $lr = Run-Timed $exe "version" 60
        $short = $exe.Replace("$T\z\", "")
        Write-Host ("  {0}: exit {1}{2}" -f $short, $lr.Code, $(if ($lr.TimedOut) { " (stopped after the time limit: a message box?)" } else { "" }))
        Check "Windows 7 starts $short" { -not (Loader-Refused $lr) }
    }

    Write-Host "== run"
    $script:cell = "run"
    $ver = Run-Timed $R "version" 60
    Write-Host "  $($ver.Out.Trim())"
    Check "rapidr.exe version" { $ver.Out -match "RapidR \d+\.\d+\.\d+" }
    $w7 = Run-Timed $R "__win7_check" 60
    Write-Host ($w7.Out.TrimEnd())
    Check "__win7_check: RoGetActivationFactory is not on this Windows and its stand-in answers (so this really is Windows 7)" { $w7.Out -match "RoGetActivationFactory: not on this Windows" -and $w7.Out -match "stand-in RoGetActivationFactory" }
    Check "__win7_check: all six functions are accounted for" { ($w7.Out -split "`n" | Where-Object { $_ -match "^(RoGetActivationFactory|RoOriginateErrorW|PathCchStripPrefix|CoIncrementMTAUsage|CoCreateFreeThreadedMarshaler|CoTaskMemFree):" }).Count -eq 6 }
    Set-Content hello.bas "`$APPTYPE CONSOLE`r`nPRINT `"hello `"; COMMAND`$`r`n"
    $rr = Run-Timed $R "run hello.bas a b" 120
    Check "rapidr run hello.bas a b" { $rr.Out -match "hello a b" }
    if (-not ($rr.Out -match "hello a b")) { Write-Host "        $(First-Error $rr.Out)" }
    # a window with some words in it: the picture below is of this program, built
    Set-Content gui.bas "CREATE Form AS QFORM`r`n  Caption = `"RapidR on Windows 7`"`r`n  Left = 560`r`n  Top = 280`r`n  Width = 360`r`n  Height = 150`r`n  CREATE Lbl AS QLABEL`r`n    Left = 24`r`n    Top = 24`r`n    Caption = `"ready`"`r`n  END CREATE`r`nEND CREATE`r`nForm.ShowModal`r`n"
    if ($hooks) {
        $env:RAPIDR_CAPTURE = "$T\work\cap"; $env:RAPIDR_CAPTURE_DELAY = "0.5"; $env:RAPIDR_TEST_DUMP = "lbl.caption"
        $rg = Run-Timed $R "run gui.bas" 180
        Check "GUI program runs (capture)" { $rg.Out -match "lbl.caption=ready" }
        if (-not ($rg.Out -match "lbl.caption=ready")) { Write-Host "        $(First-Error $rg.Out)" }
        Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP
    } else {
        Check "GUI program runs (capture): needs the test-mode token (-Token), which was not given" { $false }
    }

    Write-Host "== Build (interpreted), on a PC with no Rust (PATH and CARGO_HOME point nowhere)"
    $script:cell = "interp"
    $pathBefore = $env:PATH
    $env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"; $env:CARGO_HOME = "$T\no-cargo"; $env:RUSTUP_HOME = "$T\no-rustup"
    $t1 = Get-Date
    $b1 = Run-Timed $R "build hello.bas --interp" 300
    Write-Host ("  hello.bas: {0:N0} s" -f ((Get-Date) - $t1).TotalSeconds)
    Check "console program: rapidr build --interp makes hello.exe" { Test-Path "$T\work\hello.exe" }
    if (-not (Test-Path "$T\work\hello.exe")) { Write-Host "        $(First-Error $b1.Out)" }
    Check "... a console executable" { (Test-Path "$T\work\hello.exe") -and ((Subsystem "$T\work\hello.exe") -eq 3) }
    $b2 = Run-Timed $R "build gui.bas --interp" 300
    Check "GUI program: rapidr build --interp makes gui.exe" { Test-Path "$T\work\gui.exe" }
    if (-not (Test-Path "$T\work\gui.exe")) { Write-Host "        $(First-Error $b2.Out)" }
    Check "... a windowed executable (no console window)" { (Test-Path "$T\work\gui.exe") -and ((Subsystem "$T\work\gui.exe") -eq 2) }
    Check "... with its notices beside it" { Test-Path "$T\work\THIRD-PARTY-NOTICES.txt" }
    $env:PATH = $pathBefore; Remove-Item env:CARGO_HOME, env:RUSTUP_HOME

    Write-Host "== the programs it built run on this Windows 7"
    $script:cell = "exe"
    $h = Run-Timed "$T\work\hello.exe" "q" 60
    Write-Host "  hello.exe: exit $($h.Code), said: $($h.Out.Trim())"
    Check "hello.exe starts (the loader accepts it) and prints" { (-not (Loader-Refused $h)) -and ($h.Out -match "hello") }
    if ($hooks) {
        $env:RAPIDR_CAPTURE = "$T\work\built"; $env:RAPIDR_CAPTURE_DELAY = "0.5"; $env:RAPIDR_TEST_DUMP = "lbl.caption"
        $g = Run-Timed "$T\work\gui.exe" "" 180
        Check "gui.exe shows its window (capture)" { (Test-Path "$T\work\gui.exe") -and ($g.Out -match "lbl.caption=ready") }
        if (-not ($g.Out -match "lbl.caption=ready")) { Write-Host "        exit $($g.Code): $(First-Error $g.Out)" }
        Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP
    }
    # The window on the real desktop: started with no test hooks (nothing captures it), a picture taken, the program stopped by its process id.
    Remove-Item env:RAPIDR_TEST_MODE -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $Shots | Out-Null
    $gp = Start-Process -FilePath "$T\work\gui.exe" -PassThru
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt 60 -and -not $gp.HasExited; $i++) {
        Start-Sleep -Milliseconds 500
        $gp.Refresh()
        if ($gp.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $gp.MainWindowHandle; break }
    }
    Check "gui.exe is running with a window on the desktop (not stuck behind an error box)" { (-not $gp.HasExited) -and ($hwnd -ne [IntPtr]::Zero) }
    if ($hwnd -ne [IntPtr]::Zero) {
        Start-Sleep -Seconds 3   # (the first paint)
        $info = Save-Pictures $gp.Id "$Shots\win7-desktop.png" "$Shots\win7-gui-hello.png"
        Write-Host "  picture: $info"
    } else {
        try { [void](Save-Pictures $gp.Id "$Shots\win7-desktop.png" "") } catch { }
    }
    if (-not $gp.HasExited) { Stop-Process -Id $gp.Id -Force -ErrorAction SilentlyContinue }
    if ($hooks) { $env:RAPIDR_TEST_MODE = $Token }

    Write-Host "== Studio, headless"
    $script:cell = "other"
    if ($hooks) {
        $env:RAPIDR_CAPTURE = "$T\work\ide"; $env:RAPIDR_CAPTURE_DELAY = "1"; $env:RAPIDR_TEST_DUMP = "Studio.caption"
        $s1 = Run-Timed $R "ide" 240
        Check "the IDE starts" { $s1.Out -match "Studio.caption=RapidR Studio" }
        if (-not ($s1.Out -match "Studio.caption=RapidR Studio")) { Write-Host "        exit $($s1.Code): $(First-Error $s1.Out)" }
        Remove-Item env:RAPIDR_CAPTURE, env:RAPIDR_CAPTURE_DELAY, env:RAPIDR_TEST_DUMP
    } else {
        Check "the IDE starts (headless): needs the test-mode token (-Token), which was not given" { $false }
    }
    Write-Host "  (Build Native: not part of the Windows 7 package: no Rust for Windows 7)"
} finally {
    Set-Location $env:SystemRoot
    # Programs of ours still running (a window left open by a failed check): stopped by process id, only those under $T
    Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -like "$T\*" } | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
    Remove-TestFolder $T
}
$script:cell = "other"
Check "the temporary folder is gone" { -not (Test-Path $T) }
Write-Host (Cells-Line)
if ($script:fail) { Write-Host "== smoke test FAILED"; exit 1 } else { Write-Host "== smoke test passed"; exit 0 }
