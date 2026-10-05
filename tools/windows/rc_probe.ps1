# RapidQ's own compiler (RC.EXE, 2006) as the ground truth for RapidR's
# behaviour (docs/rapidq-ground-truth.md): every .bas in -Programs compiled
# by RC.EXE, and the console ones run, printing what RC and each program
# say. Runs on Windows (x86 or ARM64: RC.EXE is a 32-bit x86 program,
# emulated on ARM). tools/rc_probe.sh runs it in the Parallels VM from the
# Mac.
#
#   powershell -ExecutionPolicy Bypass -File rc_probe.ps1 -Programs <dir> [-RapidQ <dir>] [-Work <dir>] [-Timeout 10]
#
# -RapidQ: RapidQ's folder (RC.EXE, Lib\, include\); copied to -Work first.
# A program whose name starts with g_ is only compiled (a GUI program would
# wait for its window to close). A program still running after -Timeout
# seconds is ended (RapidQ shows an exception in a message box, which waits
# for a click) and reported as TIMEOUT.
param(
  [Parameter(Mandatory = $true)][string]$Programs,
  [string]$RapidQ = "\\Mac\Home\Downloads\Rapidq",
  [string]$Work = "$env:USERPROFILE\rq",
  [int]$Timeout = 10
)

New-Item -ItemType Directory -Force "$Work\t" | Out-Null
Copy-Item -Force "$RapidQ\RC.EXE" $Work
Copy-Item -Force -Recurse "$RapidQ\Lib" $Work
Copy-Item -Force -Recurse "$RapidQ\include" $Work
Remove-Item -Force "$Work\t\*" -ErrorAction SilentlyContinue
Copy-Item -Force "$Programs\*.bas" "$Work\t"

Push-Location "$Work\t"
foreach ($bas in Get-ChildItem *.bas | Sort-Object Name) {
  $name = $bas.BaseName
  Write-Output "== $($bas.Name)"
  # (RC prints its banner and a compile summary; errors as
  # "Line N: ERROR: ..." with the line and a caret)
  & "$Work\RC.EXE" "-I$Work\include" "-L$Work\Lib" $bas.Name 2>&1 |
    Where-Object { $_ -and $_ -notmatch '^(Rapid-Q Compiler|Compiling .* to |Number of statements|Numeric Variables|String Variables|Resource data|Bytes global data|Bytes sub data)' } |
    ForEach-Object { Write-Output $_ }
  if (-not (Test-Path "$name.exe")) { continue }
  if ($name -like "g_*") { Write-Output "-- compiled"; continue }
  Write-Output "-- run"
  $p = Start-Process -FilePath ".\$name.exe" -NoNewWindow -PassThru -RedirectStandardOutput "$name.out" -RedirectStandardError "$name.err"
  if (-not $p.WaitForExit($Timeout * 1000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Write-Output "TIMEOUT after $Timeout s (an exception's message box?)"
  }
  Get-Content "$name.out", "$name.err" -ErrorAction SilentlyContinue | ForEach-Object { Write-Output $_ }
}
Pop-Location
