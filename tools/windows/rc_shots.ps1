# RapidQ's look, for RapidR's visual gallery (tests/visual/README.md): every
# .bas in -Programs compiled by RapidQ's RC.EXE and run, each of its windows
# (a form, a message box …) captured, then the program ended. Runs in the
# Windows VM; tools/visual/rq_shots.sh runs it from the Mac and gets the
# PNGs back (base64 on standard output: nothing is written to the share).
#
#   powershell -ExecutionPolicy Bypass -File rc_shots.ps1 -Programs <dir>
#     [-RapidQ <dir>] [-Work <dir>] [-Sub vis] [-Only name,…] [-Wait 1500]
#
# Each window is captured twice:
#   1x  the program as Windows runs it (RapidQ's programs aren't DPI-aware:
#       they draw at 96 dpi, which PrintWindow returns as drawn);
#   2x  the program run again with Windows' GDI scaling (the compatibility
#       layer GDIDPISCALING DPIUNAWARE, per process: no system setting is
#       touched) on the VM's 200 % screen — GDI draws its lines and text at
#       the screen's resolution, as RapidR draws at 2× — read off the screen
#       by this (then per-monitor DPI-aware) script.
# Each window's client area (what RapidR's captures hold): a line
# "-- shot <name> <scale> <n> <w> <h> <window class>", then
# "-- png64 <base64>"; "-- skip <name> <why>" for a program
# not run.
# Safety: a program that could print (LPRINT, QPRINTER / Printer, a print
# dialog, an LPT / PRN device) or touch the registry is never run (the VM
# shares the Mac's real printer); every program is ended after its capture.
param(
  [Parameter(Mandatory = $true)][string]$Programs,
  [string]$RapidQ = "\\Mac\Home\Downloads\Rapidq",
  [string]$Work = "$env:USERPROFILE\rq",
  [string]$Sub = "vis",
  [string]$Only = "",
  [int]$Wait = 1500,
  [string]$Scales = "1,2"
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int size);
  public static List<IntPtr> Of(uint pid) {
    var found = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      RECT r;
      if (p == pid && IsWindowVisible(h) && GetWindowRect(h, out r) && r.R - r.L > 2 && r.B - r.T > 2) found.Add(h);
      return true;
    }, IntPtr.Zero);
    return found;
  }
  public static string Class(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc f, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Ansi)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, int m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr h, IntPtr r, IntPtr rgn, uint flags);
  // (child windows whose text is t)
  public static List<IntPtr> Children(IntPtr parent, string t) {
    var found = new List<IntPtr>();
    EnumChildWindows(parent, (h, l) => {
      var s = new StringBuilder(512); GetWindowText(h, s, 512);
      if (s.ToString() == t) found.Add(h);
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@

$here = "$Work\$Sub"
New-Item -ItemType Directory -Force $here | Out-Null
Copy-Item -Force "$RapidQ\RC.EXE" $Work
Copy-Item -Force -Recurse "$RapidQ\Lib" $Work
Copy-Item -Force -Recurse "$RapidQ\include" $Work
Remove-Item -Force -Recurse "$here\*" -ErrorAction SilentlyContinue
Copy-Item -Force -Recurse "$Programs\*" $here
# (RapidQ reads CRLF lines; the repository keeps LF)
foreach ($f in Get-ChildItem "$here\*.bas") { $t = Get-Content -LiteralPath $f.FullName; Set-Content -LiteralPath $f.FullName -Value $t -Encoding Default }
# (<name>.rapidq.bas, where there is one, is <name>.bas as RapidQ writes it)
$progs = Get-ChildItem "$here\*.bas" | Where-Object { $_.Name -notlike "*.rapidq.bas" } | Sort-Object Name
if ($Only) { $want = $Only.Split(","); $progs = $progs | Where-Object { $want -contains $_.BaseName } }

$printing = '(?im)\blprint\b|\blflush\b|qprinter|printer\.|printdialog|"\s*(lpt\d?|prn)\s*:?\s*"'
$registry = '(?i)qregistry|advapi32|\bReg(Open|Create|Set|Delete|Query)\w*'

function Png64($bmp) {
  $ms = New-Object IO.MemoryStream
  $bmp.Save($ms, [Drawing.Imaging.ImageFormat]::Png)
  [Convert]::ToBase64String($ms.ToArray())
}

# (the windows of process $p, top-level and visible, in creation order)
function Windows-Of($p) {
  $all = [W]::Of([uint32]$p.Id)
  # (Delphi's hidden application window is 0 × 0; EnumWindows lists the
  # top of the z-order first: the newest window — reversed, the form first)
  [array]::Reverse($all)
  $all | Where-Object { [W]::Class($_) -ne "TApplication" }
}

foreach ($bas in $progs) {
  $name = $bas.BaseName
  $variant = Join-Path $here "$name.rapidq.bas"
  if (Test-Path -LiteralPath $variant) { $bas = Get-Item -LiteralPath $variant }
  $src = Get-Content -Raw -LiteralPath $bas.FullName
  if ($src -match $printing -or $src -match $registry) { Write-Output "-- skip $name printing or registry"; continue }
  Push-Location -LiteralPath $here
  $rc = Start-Process -FilePath "$Work\RC.EXE" -ArgumentList "-I$Work\include", "-L$Work\Lib", "`"$($bas.Name)`"" `
    -NoNewWindow -PassThru -Wait -RedirectStandardOutput "$Work\$Sub.rc.out" -RedirectStandardError "$Work\$Sub.rc.err"
  $exe = Join-Path $here "$($bas.BaseName).exe"
  if (-not (Test-Path $exe)) { Write-Output "-- skip $name compile: $((Get-Content "$Work\$Sub.rc.out") -join ' | ')"; Pop-Location; continue }
  foreach ($scale in $Scales.Split(",")) {
    if ($scale -eq "2") { $env:__COMPAT_LAYER = "GDIDPISCALING DPIUNAWARE" } else { Remove-Item Env:__COMPAT_LAYER -ErrorAction SilentlyContinue }
    $p = Start-Process -FilePath $exe -WorkingDirectory $here -PassThru
    Remove-Item Env:__COMPAT_LAYER -ErrorAction SilentlyContinue
    $wins = @()
    for ($i = 0; $i -lt 40 -and $wins.Count -eq 0; $i++) { Start-Sleep -Milliseconds 200; $wins = @(Windows-Of $p) }
    Start-Sleep -Milliseconds $Wait
    $wins = @(Windows-Of $p)
    foreach ($h in $wins) {
      # (focus rectangles and & underlines shown, as after a key: RapidR
      # always shows them — WM_CHANGEUISTATE, UIS_CLEAR, HIDEFOCUS | HIDEACCEL)
      [W]::SendMessage($h, 0x127, [IntPtr](2 -bor (3 -shl 16)), [IntPtr]::Zero) | Out-Null
      # (' press: <caption>' in the source: that button held down, BM_SETSTATE)
      foreach ($m in [regex]::Matches($src, "(?im)^'\s*press:\s*(.+?)\s*$")) {
        foreach ($b in [W]::Children($h, $m.Groups[1].Value)) { [W]::SendMessage($b, 0xF3, [IntPtr]1, [IntPtr]::Zero) | Out-Null }
      }
      # (RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW)
      [W]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x185) | Out-Null
    }
    Start-Sleep -Milliseconds 300
    $n = 0
    foreach ($h in $wins) {
      $n++
      $c = New-Object W+RECT; [W]::GetClientRect($h, [ref]$c) | Out-Null
      $w = $c.R * [int]$scale; $hh = $c.B * [int]$scale
      if ($w -le 0 -or $hh -le 0) { continue }
      $bmp = New-Object Drawing.Bitmap $w, $hh
      $g = [Drawing.Graphics]::FromImage($bmp)
      if ($scale -eq "2") {
        # (the screen's pixels, as a per-monitor-aware thread sees them)
        # (the screen's pixels, as a per-monitor-aware thread sees them —
        # PrintWindow gives the 96-dpi surface — the window put on top of
        # everything first: HWND_TOPMOST, no move, size or activation)
        [W]::SetWindowPos($h, [IntPtr]-1, 0, 0, 0, 0, 0x13) | Out-Null
        [W]::SetForegroundWindow($h) | Out-Null
        Start-Sleep -Milliseconds 600
        [W]::SetThreadDpiAwarenessContext([IntPtr]-4) | Out-Null
        $o = New-Object W+POINT; [W]::ClientToScreen($h, [ref]$o) | Out-Null
        $g.CopyFromScreen($o.X, $o.Y, 0, 0, (New-Object Drawing.Size $w, $hh))
        [W]::SetThreadDpiAwarenessContext([IntPtr]-1) | Out-Null
      } else {
        # (the client area as drawn, from the window's own surface:
        # PW_CLIENTONLY | PW_RENDERFULLCONTENT)
        $dc = $g.GetHdc(); [W]::PrintWindow($h, $dc, 3) | Out-Null; $g.ReleaseHdc($dc)
      }
      $g.Dispose()
      Write-Output "-- shot $name $scale $n $w $hh $([W]::Class($h))"
      Write-Output ("-- png64 " + (Png64 $bmp))
      $bmp.Dispose()
    }
    if ($wins.Count -eq 0) { Write-Output "-- skip $name no window ($scale)" }
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    $p.WaitForExit(5000) | Out-Null
  }
  Pop-Location
}
Write-Output "-- end"
