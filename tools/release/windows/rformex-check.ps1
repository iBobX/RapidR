# RFormEx on Windows (docs/rformex-design.md), with the build `windows-vm.sh dllmem` made
# (%USERPROFILE%\rapidr-dllmem\src\rapidr.exe): the GUI cases headless (a form written QFormEx
# hears OnMinimise before the minimise here), then real windows: a QFormEx's OnMessage hears a
# posted WM_USER + 5 but not WM_SYSCOMMAND, its SC_MINIMIZE waits for OnMinimise (wsNormal in
# it); an RFormEx is layered (alpha 200, its Color the colour key) and topmost, captured, and
# hears OnMinimise once minimised (wsMinimized in it). Its own folder, removed at the end; the
# capture in %USERPROFILE%\rapidr-release\out. Nothing prints or touches the user's settings.
# (This file is read as ANSI by Windows PowerShell: no character outside ASCII in its code.)
param([string]$Prep = "")
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$src = "$env:USERPROFILE\rapidr-dllmem\src"
$exe = "$src\rapidr.exe"
$W = "$env:USERPROFILE\rformex-check"
$out = "$env:USERPROFILE\rapidr-release\out"
if (-not (Test-Path $exe)) { Write-Host "no $exe: run windows-vm.sh dllmem first"; exit 1 }
cmd /c "rmdir /s /q `"$W`" 2>nul" | Out-Null
New-Item -ItemType Directory -Force $W, "$W\prints", "$W\config", $out | Out-Null
$env:RAPIDR_PRINT_TO = "$W\prints"; $env:RAPIDR_REGISTRY = "$W\registry.reg"; $env:RAPIDR_AI_KEYSTORE = "memory"
$env:RAPIDR_CONFIG_DIR = "$W\config"; $env:RAPIDR_LANG = "en"
if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
$fail = 0

Write-Host "== the GUI cases, headless (rapidr run)"
$env:RAPIDR_THEME = "classic"; $env:RAPIDR_TEST_DROP = "a.txt;b.txt"
$cases = @(
    @("formex", "main.__wheel_20_30_-1,main.__drop_10_20,zone.onclick,main.__drop_10_20,main.__drop_150_10,main.__tray_517,main.__tray_515,main.__tray_514,main.__activate,main.__deactivate,main.__activate,other.__tray_513,main.__minimise", "lbl.caption,lbl2.caption,lbl3.caption",
      "lbl.caption=wheel1,20,30,0 drag:a.txt@10,20 drag:b.txt@10,20 drag:a.txt@150,10 drag:b.txt@150,10 click up1 dbl up0 min1 "),
    @("formex_classic", "main.__wheel_20_30_-1,main.__drop_10_20,zone.onclick,main.__drop_10_20,main.__drop_150_10,main.__minimise", "lbl.caption,lbl2.caption",
      "lbl.caption=wheel1,21,60,0 drag:a.txt@150,10 drag:b.txt@150,10 min0 ")
)
Push-Location "$src\tests\fixtures"
foreach ($c in $cases) {
    $env:RAPIDR_CAPTURE = "$W\cap-$($c[0])"; $env:RAPIDR_TEST_EVENTS = $c[1]; $env:RAPIDR_TEST_DUMP = $c[2]
    $text = (& $exe run "$($c[0]).bas" 2>&1 | Out-String)
    $text -split "`n" | Where-Object { $_ -match "=" } | ForEach-Object { "   $($c[0]): $($_.TrimEnd())" } | Write-Host
    if ($text.Contains($c[3])) { Write-Host "   ok: $($c[3])" } else { Write-Host "   MISSING: $($c[3])"; $fail = 1 }
}
Pop-Location
Remove-Item Env:\RAPIDR_CAPTURE, Env:\RAPIDR_TEST_EVENTS, Env:\RAPIDR_TEST_DUMP, Env:\RAPIDR_TEST_DROP, Env:\RAPIDR_THEME -ErrorAction SilentlyContinue

Write-Host "== real windows"
Copy-Item "$src\tests\fixtures\QFormEx.inc" "$W\QFormEx.inc"
$prog = @'
$INCLUDE "QFormEx.inc"
DECLARE SUB Mini
DECLARE SUB RMini
DECLARE SUB Heard (H AS LONG, M AS LONG, W AS LONG, L AS LONG)
DECLARE SUB Note (S AS STRING)
CREATE Main AS QFORMEX
    Caption = "rformex wincheck classic"
    Left = 80: Top = 80: Width = 320: Height = 200
    OnMinimise = Mini
    OnMessage = Heard
END CREATE
CREATE Other AS RFormEx
    Caption = "rformex wincheck rapidr"
    Left = 440: Top = 80: Width = 360: Height = 240
    Color = &HFF00FF
    TransparentClient = True
    Transparent = 200
    FormStyle = fsStayOnTop
    OnMinimise = RMini
    CREATE Card AS RPanel
        Left = 20: Top = 20: Width = 160: Height = 90
        Caption = "A panel stays"
        Color = &HFFFFFF
    END CREATE
END CREATE
SUB Note (S AS STRING)
    OPEN "wincheck.log" FOR APPEND AS #9
    PRINT #9, S
    CLOSE #9
END SUB
SUB Mini
    Note "classic OnMinimise WindowState=" + STR$(Main.WindowState)
END SUB
SUB RMini
    Note "rapidr OnMinimise WindowState=" + STR$(Other.WindowState)
END SUB
SUB Heard (H AS LONG, M AS LONG, W AS LONG, L AS LONG)
    IF M = &H405 OR M = &H112 THEN Note "classic OnMessage " + HEX$(M) + " " + STR$(W)
END SUB
Other.Show
Main.ShowModal
'@
Set-Content -Path "$W\wincheck.rr" -Value $prog -Encoding ASCII
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class RfxUser32 {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr h, int i);
    [DllImport("user32.dll")] public static extern bool GetLayeredWindowAttributes(IntPtr h, out uint key, out byte alpha, out uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
}
"@
Push-Location $W
$p = Start-Process -FilePath $exe -ArgumentList "run", "wincheck.rr" -WorkingDirectory $W -PassThru -RedirectStandardError "$W\wincheck-err.txt"
Start-Sleep -Seconds 6
$c = [RfxUser32]::FindWindowW($null, "rformex wincheck classic")
$r = [RfxUser32]::FindWindowW($null, "rformex wincheck rapidr")
Write-Host "   windows: classic $c, rapidr $r"
if ($r -ne [IntPtr]::Zero) {
    $ex = [RfxUser32]::GetWindowLongW($r, -20)
    $key = 0; $alpha = 0; $flags = 0
    $got = [RfxUser32]::GetLayeredWindowAttributes($r, [ref]$key, [ref]$alpha, [ref]$flags)
    Write-Host ("   RFormEx: layered {0}, topmost {1}, layered attributes {2}: key {3:X6} alpha {4} flags {5}" -f (($ex -band 0x80000) -ne 0), (($ex -band 8) -ne 0), $got, $key, $alpha, $flags)
    if (($ex -band 0x80000) -eq 0 -or ($ex -band 8) -eq 0 -or $alpha -ne 200 -or ($flags -band 3) -ne 3) { $fail = 1 }
    try {
        Add-Type -AssemblyName System.Drawing
        $rect = New-Object RfxUser32+RECT
        [RfxUser32]::GetWindowRect($r, [ref]$rect) | Out-Null
        $bmp = New-Object System.Drawing.Bitmap ($rect.R - $rect.L + 40), ($rect.B - $rect.T + 40)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($rect.L - 20, $rect.T - 20, 0, 0, $bmp.Size)
        $bmp.Save("$out\rformex-win-look.png", [System.Drawing.Imaging.ImageFormat]::Png)
        Write-Host "   captured: $out\rformex-win-look.png"
    } catch { Write-Host "   no capture: $($_.Exception.Message)" }
}
if ($c -ne [IntPtr]::Zero) {
    [RfxUser32]::PostMessageW($c, 0x405, [IntPtr]7, [IntPtr]9) | Out-Null
    Start-Sleep -Milliseconds 800
    [RfxUser32]::PostMessageW($c, 0x112, [IntPtr]0xF020, [IntPtr]0) | Out-Null
    Start-Sleep -Seconds 2
    Write-Host "   QFormEx minimised after SC_MINIMIZE: $([RfxUser32]::IsIconic($c))"
    if (-not [RfxUser32]::IsIconic($c)) { $fail = 1 }
}
if ($r -ne [IntPtr]::Zero) {
    [RfxUser32]::PostMessageW($r, 0x112, [IntPtr]0xF020, [IntPtr]0) | Out-Null
    Start-Sleep -Seconds 2
    Write-Host "   RFormEx minimised after SC_MINIMIZE: $([RfxUser32]::IsIconic($r))"
}
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Pop-Location
$log = Get-Content "$W\wincheck.log" -ErrorAction SilentlyContinue
$log | ForEach-Object { "   log: $_" } | Write-Host
Get-Content "$W\wincheck-err.txt" -ErrorAction SilentlyContinue | Select-Object -First 10 | ForEach-Object { "   stderr: $_" } | Write-Host
$joined = ($log -join "`n")
foreach ($want in @("classic OnMessage 405 7", "classic OnMinimise WindowState=0", "rapidr OnMinimise WindowState=1")) {
    if ($joined.Contains($want)) { Write-Host "   ok: $want" } else { Write-Host "   MISSING: $want"; $fail = 1 }
}
if ($joined.Contains("OnMessage 112")) { Write-Host "   WRONG: OnMessage heard WM_SYSCOMMAND"; $fail = 1 }
cmd /c "rmdir /s /q `"$W`" 2>nul" | Out-Null
Write-Host "== rformex-check: $(if ($fail) { 'FAILED' } else { 'passed' })"
exit $fail
