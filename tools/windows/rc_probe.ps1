# RapidQ's own compiler (RC.EXE, 2006) as the ground truth for RapidR's
# behaviour (docs/rapidq-ground-truth.md): every .bas in -Programs compiled
# by RC.EXE, and the console ones run, printing what RC and each program
# say. Runs on Windows (x86 or ARM64: RC.EXE is a 32-bit x86 program,
# emulated on ARM). tools/rc_probe.sh runs it in the Parallels VM from the
# Mac.
#
#   powershell -ExecutionPolicy Bypass -File rc_probe.ps1 -Programs <dir>
#     [-RapidQ <dir>] [-Work <dir>] [-Sub t] [-List <file>] [-Timeout 10] [-Encode]
#
# -RapidQ: RapidQ's folder (RC.EXE, Lib\, include\); copied to -Work first.
# -Sub: the folder under -Work the programs are copied to and run in (one per
#   user of the VM, so two probes don't clear each other's files).
# -List: a file naming the programs to compile and run, one path relative to
#   -Programs per line; the whole -Programs tree is copied (a program's own
#   includes and data files next to it) and each runs in its own folder.
#   Without it, every *.bas directly in -Programs.
# -Encode: each program's output as one base64 line ("-- out64 <bytes>"), so
#   the Mac gets RapidQ's exact bytes (tools/rapidq_truth.py reads it).
# A program reads <name>.input (next to it) as its standard input, or an
# empty file — never the console, where it would wait.
# A program whose name starts with g_, and one RC built as a GUI program
# (the .exe's PE subsystem), is only compiled (it would wait for its window
# to close). A program still running after -Timeout seconds is ended
# (RapidQ shows an exception in a message box, which waits for a click) and
# reported as TIMEOUT.
# Safety: a program that could print (LPRINT, QPRINTER / Printer, a print
# dialog, an LPT / PRN device) or touch the registry (QREGISTRY, the
# registry API) is never run here — the VM shares the Mac's real printer.
# tools/rapidq_truth.py filters them out before; this is the second check.
# Everything the run prints also goes to <Work>\<Sub>.report.txt, to read
# back when the Mac's side of prlctl dropped during a long batch.
param(
  [Parameter(Mandatory = $true)][string]$Programs,
  [string]$RapidQ = "\\Mac\Home\Downloads\Rapidq",
  [string]$Work = "$env:USERPROFILE\rq",
  [string]$Sub = "t",
  [string]$List = "",
  [int]$Timeout = 10,
  [switch]$Encode
)

$here = "$Work\$Sub"
New-Item -ItemType Directory -Force $here | Out-Null
Copy-Item -Force "$RapidQ\RC.EXE" $Work
Copy-Item -Force -Recurse "$RapidQ\Lib" $Work
Copy-Item -Force -Recurse "$RapidQ\include" $Work
Remove-Item -Force -Recurse "$here\*" -ErrorAction SilentlyContinue
if ($List) {
  Copy-Item -Force -Recurse "$Programs\*" $here
  $progs = Get-Content $List | Where-Object { $_.Trim() } | ForEach-Object { Get-Item -LiteralPath (Join-Path $here $_.Trim()) }
} else {
  Copy-Item -Force "$Programs\*.bas" $here
  Copy-Item -Force "$Programs\*.input" $here -ErrorAction SilentlyContinue
  $progs = Get-ChildItem "$here\*.bas" | Sort-Object Name
}
$empty = "$Work\$Sub.empty"
Set-Content -LiteralPath $empty -Value $null -NoNewline

# (the .exe's PE subsystem: 2 GUI, 3 console)
function Get-Subsystem($exe) {
  $b = [IO.File]::ReadAllBytes($exe)
  $pe = [BitConverter]::ToInt32($b, 0x3C)
  return [BitConverter]::ToUInt16($b, $pe + 0x5C)
}

$printing = '(?im)\blprint\b|\blflush\b|printer|printdialog|"\s*(lpt\d?|prn)\s*:?\s*"'
$registry = '(?i)qregistry|advapi32|\bReg(Open|Create|Set|Delete|Query)\w*'

& {
foreach ($bas in $progs) {
  $name = $bas.BaseName
  $rel = if ($List) { $bas.FullName.Substring($here.Length + 1) } else { $bas.Name }
  Write-Output "== $rel"
  Push-Location -LiteralPath $bas.DirectoryName
  # (RC prints its banner and a compile summary; errors as
  # "Line N: ERROR: ..." with the line and a caret)
  $rc = Start-Process -FilePath "$Work\RC.EXE" -ArgumentList "-I$Work\include", "-L$Work\Lib", "`"$($bas.Name)`"" `
    -NoNewWindow -PassThru -RedirectStandardOutput "$Work\$Sub.rc.out" -RedirectStandardError "$Work\$Sub.rc.err" -RedirectStandardInput $empty
  if (-not $rc.WaitForExit(60000)) { Stop-Process -Id $rc.Id -Force -ErrorAction SilentlyContinue; Write-Output "-- RC TIMEOUT" }
  Get-Content "$Work\$Sub.rc.out", "$Work\$Sub.rc.err" -ErrorAction SilentlyContinue |
    Where-Object { $_ -and $_ -notmatch '^(Rapid-Q Compiler|Compiling .* to |Number of statements|Numeric Variables|String Variables|Resource data|Bytes global data|Bytes sub data)' } |
    ForEach-Object { Write-Output $_ }
  $exe = Join-Path $bas.DirectoryName "$name.exe"
  if (-not (Test-Path -LiteralPath $exe)) { Pop-Location; continue }
  $src = Get-Content -Raw -LiteralPath $bas.FullName
  if ($src -match $printing) { Write-Output "-- compiled (not run: printing)"; Pop-Location; continue }
  if ($src -match $registry) { Write-Output "-- compiled (not run: registry)"; Pop-Location; continue }
  if ($name -like "g_*") { Write-Output "-- compiled"; Pop-Location; continue }
  if ((Get-Subsystem $exe) -ne 3) { Write-Output "-- compiled (GUI program)"; Pop-Location; continue }
  Write-Output "-- run"
  $in = Join-Path $bas.DirectoryName "$name.input"
  if (-not (Test-Path -LiteralPath $in)) { $in = $empty }
  $out = Join-Path $bas.DirectoryName "$name.out"
  $err = Join-Path $bas.DirectoryName "$name.err"
  $p = Start-Process -FilePath $exe -NoNewWindow -PassThru -RedirectStandardOutput $out -RedirectStandardError $err -RedirectStandardInput $in
  if (-not $p.WaitForExit($Timeout * 1000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
    Write-Output "TIMEOUT after $Timeout s (an exception's message box?)"
  }
  if ($Encode) {
    $bytes = if (Test-Path -LiteralPath $out) { [IO.File]::ReadAllBytes($out) } else { @() }
    Write-Output "-- out64 $([Convert]::ToBase64String($bytes))"
    Get-Content -LiteralPath $err -ErrorAction SilentlyContinue | ForEach-Object { Write-Output "-- err $_" }
  } else {
    Get-Content -LiteralPath $out, $err -ErrorAction SilentlyContinue | ForEach-Object { Write-Output $_ }
  }
  Pop-Location
}
Write-Output "== end"
} | Tee-Object -FilePath "$Work\$Sub.report.txt"
