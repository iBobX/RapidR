# The Windows 7 / 8 / 8.1 alpha zip (tools/win7/alpha.sh: RapidR-<ver>-win7-x86-alpha<N>.zip) on a REAL
# Windows 7: unzipped into a temporary folder and used as a tester would use it, then removed.
# tools/release/win7-vm.sh runs it in the UTM VM "Windows 7 x86" (as the logged-in user, through a
# scheduled task, so the programs get a real desktop); tools/release/build-matrix.sh reads its log as the
# row "Windows 7 (real)". The sister script smoke-win7.ps1 does the same in the Windows 11 VM, where the
# 32-bit program runs by emulation and Windows 7 itself is not checked; this one is written for
# PowerShell 2.0 (what Windows 7 has: no Expand-Archive, no Get-FileHash) and checks what only a real
# Windows 7 can say: that its loader accepts the programs (rapidr.exe and the two runner programs: no
# "api-ms-win-core-...dll is missing", no "entry point not found"), that the six functions Windows 7
# lacks are answered by RapidR's stand-ins, and that the programs Build made run and show a window.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File smoke-win7-real.ps1 -Zip <zip> [-Sha256 <hex>] [-Token <token file>] [-Work <folder>] [-Shots <folder>]
#
# Checks (the cells of build-matrix.sh):
#   run      rapidr.exe version, __win7_check, a console program and a GUI program run (headless capture)
#   interp   Build (interpreted) of a console and of a GUI program on a PC with no Rust (PATH and CARGO_HOME point nowhere)
#   exe      the programs that build made start, print, and the GUI one shows a window (a picture of the desktop is kept)
#   other    the OS is Windows 7 (6.1); the loader accepts every program of the zip; Studio starts headless
#   native   not part of this package (no Rust for Windows 7): the cell stays "-"
# The test hooks of a release build (captures, dumps) act only with a signed test-mode token (SEC-24,
# docs/security/test-mode.md): -Token <file>. Without it the GUI checks cannot run and say so.
# Everything made (the unzipped folder, the programs, the captures) is removed at the end, even when a check
# failed; the pictures are first copied to -Shots. The last line before the verdict,
# "##cells run=ok interp=ok ...", is what the build matrix reads.
param(
    [string]$Zip = "",
    [string]$Sha256 = "",
    [string]$Token = "",
    [string]$Work = "C:\rapidr-matrix",
    [string]$Shots = ""
)
$ErrorActionPreference = "Continue"
$T = Join-Path $Work ("t-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force "$T\work", "$T\prints" | Out-Null
if (-not $Shots) { $Shots = "$Work\shots" }
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
function Subsystem($exe) {
    $b = [IO.File]::ReadAllBytes($exe); $pe = [BitConverter]::ToInt32($b, 0x3C)
    [BitConverter]::ToUInt16($b, $pe + 0x5C)    # 2: windowed, 3: console
}
# STATUS_DLL_NOT_FOUND, STATUS_ENTRYPOINT_NOT_FOUND, STATUS_INVALID_IMAGE_FORMAT, STATUS_DLL_INIT_FAILED: the loader refused the program
function Loader-Refused($r) { $r.TimedOut -or (@(-1073741515, -1073741511, -1073741701, -1073741502) -contains $r.Code) -or ($r.Out -match "could not start") }

# A zip reader of our own (Windows 7 has no Expand-Archive; Shell.Application's zip folder is slow and asynchronous):
# the central directory, stored and deflated entries, nothing else.
$unzipSrc = @'
using System; using System.IO; using System.IO.Compression;
public static class RrUnzip {
    public static int Extract(string zip, string dest) {
        byte[] b = File.ReadAllBytes(zip);
        int e = b.Length - 22;
        while (e >= 0 && !(b[e] == 0x50 && b[e+1] == 0x4b && b[e+2] == 5 && b[e+3] == 6)) e--;
        if (e < 0) throw new Exception("not a zip file (no end record)");
        int n = BitConverter.ToUInt16(b, e + 10);
        int cd = (int)BitConverter.ToUInt32(b, e + 16);
        string root = Path.GetFullPath(dest);
        int count = 0;
        for (int i = 0; i < n; i++) {
            if (BitConverter.ToUInt32(b, cd) != 0x02014b50) throw new Exception("bad central directory");
            int method = BitConverter.ToUInt16(b, cd + 10);
            long csize = BitConverter.ToUInt32(b, cd + 20);
            int nlen = BitConverter.ToUInt16(b, cd + 28), xlen = BitConverter.ToUInt16(b, cd + 30), clen = BitConverter.ToUInt16(b, cd + 32);
            int lho = (int)BitConverter.ToUInt32(b, cd + 42);
            string name = System.Text.Encoding.UTF8.GetString(b, cd + 46, nlen);
            cd += 46 + nlen + xlen + clen;
            string path = Path.GetFullPath(Path.Combine(root, name.Replace('/', '\\')));
            if (!path.StartsWith(root)) throw new Exception("entry outside the folder: " + name);
            if (name.EndsWith("/")) { Directory.CreateDirectory(path); continue; }
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            int lnlen = BitConverter.ToUInt16(b, lho + 26), lxlen = BitConverter.ToUInt16(b, lho + 28);
            int data = lho + 30 + lnlen + lxlen;
            using (FileStream o = File.Create(path)) {
                if (method == 0) { o.Write(b, data, (int)csize); }
                else if (method == 8) {
                    using (MemoryStream ms = new MemoryStream(b, data, (int)csize))
                    using (DeflateStream d = new DeflateStream(ms, CompressionMode.Decompress)) {
                        byte[] buf = new byte[65536]; int r;
                        while ((r = d.Read(buf, 0, buf.Length)) > 0) o.Write(buf, 0, r);
                    }
                } else throw new Exception("unsupported compression method " + method + " for " + name);
            }
            count++;
        }
        return count;
    }
}
'@
# A picture of the desktop, and of the window of a process, as PNG files.
$shotSrc = @'
using System; using System.Drawing; using System.Drawing.Imaging; using System.Runtime.InteropServices;
public static class RrShot {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int i);
    public static string Grab(string file, IntPtr hwnd, string cropFile) {
        int w = GetSystemMetrics(0), h = GetSystemMetrics(1);
        string info = w + "x" + h;
        using (Bitmap bmp = new Bitmap(w, h)) {
            using (Graphics g = Graphics.FromImage(bmp)) { g.CopyFromScreen(0, 0, 0, 0, new Size(w, h)); }
            bmp.Save(file, ImageFormat.Png);
            RECT r;
            if (hwnd != IntPtr.Zero && GetWindowRect(hwnd, out r)) {
                int x = Math.Max(0, r.Left - 24), y = Math.Max(0, r.Top - 24);
                int cw = Math.Min(w - x, r.Right - r.Left + 48), ch = Math.Min(h - y, r.Bottom - r.Top + 48);
                info += " window " + (r.Right - r.Left) + "x" + (r.Bottom - r.Top) + " at " + r.Left + "," + r.Top;
                if (cw > 0 && ch > 0) { using (Bitmap c = bmp.Clone(new Rectangle(x, y, cw, ch), bmp.PixelFormat)) { c.Save(cropFile, ImageFormat.Png); } }
            }
        }
        return info;
    }
}
'@

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
    Write-Host "== unzip $(Split-Path -Leaf $Zip) into $T\z"
    $t0 = Get-Date
    Add-Type -TypeDefinition $unzipSrc
    $n = [RrUnzip]::Extract($Zip, "$T\z")
    Write-Host ("  {0} files in {1:N0} s" -f $n, ((Get-Date) - $t0).TotalSeconds)
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
    Add-Type -TypeDefinition $shotSrc -ReferencedAssemblies System.Drawing
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
        $info = [RrShot]::Grab("$Shots\win7-desktop.png", $hwnd, "$Shots\win7-gui-hello.png")
        Write-Host "  picture: $info"
    } else {
        try { [void][RrShot]::Grab("$Shots\win7-desktop.png", [IntPtr]::Zero, "") } catch { }
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
    for ($try = 0; $try -lt 8 -and (Test-Path $T); $try++) {
        cmd /c "rmdir /s /q `"$T`" 2>nul" | Out-Null
        if (Test-Path $T) { Start-Sleep -Seconds 2 }
    }
}
$script:cell = "other"
Check "the temporary folder is gone" { -not (Test-Path $T) }
Write-Host (Cells-Line)
if ($script:fail) { Write-Host "== smoke test FAILED"; exit 1 } else { Write-Host "== smoke test passed"; exit 0 }
