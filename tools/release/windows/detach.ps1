# Starts job.ps1 (a script, its log, its exit code) outside this session's process
# tree: Parallels' `prlctl exec` ends what it started when it returns, so a long
# build is started by WMI instead, and tools/release/windows-vm.sh polls its log.
param([Parameter(Mandatory = $true)][string]$Script, [Parameter(Mandatory = $true)][string]$Log)
$job = Join-Path $PSScriptRoot "job.ps1"
$rest = ($args | ForEach-Object { if ("$_" -match '\s') { "`"$_`"" } else { "$_" } }) -join " "
$line = "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$job`" -Script `"$Script`" -Log `"$Log`" $rest"
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = $line; CurrentDirectory = $env:USERPROFILE }
if ($r.ReturnValue -ne 0) { Write-Host "Win32_Process.Create failed ($($r.ReturnValue))"; exit 1 }
Write-Host "started (pid $($r.ProcessId))"
exit 0
