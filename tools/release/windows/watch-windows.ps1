# Logs every top-level window that appears on the user's desktop while it runs
# (windows-vm.sh flashtest: proof that the VM's calls open no console window there).
#
#   watch-windows.ps1 -Seconds 60 -Out <file>      (run in the user's session: a detached job)
#
# Polls every ~10 ms; a window that exists for less than that is missed, a console window
# lasts far longer. One line per event: the time, milliseconds since the start, the process,
# the window's class, title and size. An event is a window that was not visible a moment ago
# (new, or shown again), or a console window whose title changed (Windows Terminal reuses
# its window for the next command). Console windows are marked CONSOLE: class
# ConsoleWindowClass (conhost), CASCADIA_HOSTING_WINDOW_CLASS (Windows Terminal) or
# PseudoConsoleWindow, or a command interpreter / PowerShell / script host as the process.
param([int]$Seconds = 60, [Parameter(Mandatory = $true)][string]$Out)
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class RrWin {
    delegate bool EnumProc(IntPtr h, IntPtr p);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr p);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    struct RECT { public int L, T, R, B; }
    // The visible top-level windows: "hwnd|pid|class|title|WxH".
    public static List<string> Visible() {
        var list = new List<string>();
        EnumWindows((h, p) => {
            if (!IsWindowVisible(h)) return true;
            int pid; GetWindowThreadProcessId(h, out pid);
            var c = new StringBuilder(256); GetClassName(h, c, 256);
            var t = new StringBuilder(256); GetWindowText(h, t, 256);
            RECT r; GetWindowRect(h, out r);
            list.Add(h.ToInt64() + "|" + pid + "|" + c + "|" + t + "|" + (r.R - r.L) + "x" + (r.B - r.T));
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
"@
# hwnd -> title, for the windows visible at the previous poll
$known = @{}
foreach ($w in [RrWin]::Visible()) { $f = $w -split "\|", 5; $known[$f[0]] = $f[3] }   # (already there: not news)
$names = @{}
$consoleNames = "cmd", "powershell", "pwsh", "conhost", "OpenConsole", "WindowsTerminal", "wscript", "cscript"
$lines = New-Object System.Collections.ArrayList
[void]$lines.Add("$(Get-Date -Format HH:mm:ss.fff)  watching for $Seconds s; $($known.Count) windows were already visible")
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $Seconds) {
    $now = @{}
    foreach ($w in [RrWin]::Visible()) {
        $f = $w -split "\|", 5
        $now[$f[0]] = $f[3]
        $procId = [int]$f[1]
        if (-not $names.ContainsKey($procId)) { $names[$procId] = (Get-Process -Id $procId -ErrorAction SilentlyContinue).ProcessName }
        $n = $names[$procId]
        $console = ($f[2] -in "ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS", "PseudoConsoleWindow") -or ($n -in $consoleNames)
        $isNew = -not $known.ContainsKey($f[0])
        $retitled = $console -and -not $isNew -and $known[$f[0]] -ne $f[3]
        if (-not ($isNew -or $retitled)) { continue }
        # (whose it is: the command line of a command interpreter or script host, while it still runs)
        $cmd = ""
        if ($console -and $n -ne "WindowsTerminal") { $cmd = "  cmd=" + ("" + (Get-CimInstance Win32_Process -Filter "ProcessId = $procId" -ErrorAction SilentlyContinue).CommandLine) }
        [void]$lines.Add(("{0}  {1,6} ms  {2}{3}pid {4} {5}  class={6} title='{7}' size={8}{9}" -f (Get-Date -Format HH:mm:ss.fff), $sw.ElapsedMilliseconds, $(if ($console) { "CONSOLE " } else { "" }), $(if ($retitled) { "(retitled) " } else { "" }), $procId, $n, $f[2], $f[3], $f[4], $cmd))
    }
    $known = $now
    Start-Sleep -Milliseconds 10
}
[void]$lines.Add("done")
Set-Content -Path $Out -Value $lines
exit 0
