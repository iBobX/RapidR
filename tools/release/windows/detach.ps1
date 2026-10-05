# Starts one of these scripts (job.ps1: its log, its exit code) outside this session's
# process tree: Parallels' `prlctl exec` ends what it started when it returns, so a long
# build is started by WMI instead, and tools/release/windows-vm.sh polls its log.
#
# A process WMI starts can't read the Mac's share (\\Mac\Home: another logon session's),
# so what the job needs is copied here first: these scripts, and -Prep (prepare.sh's
# output) when given, into %USERPROFILE%\rapidr-release\{scripts,prep}.
param(
    [Parameter(Mandatory = $true)][string]$Script,
    [Parameter(Mandatory = $true)][string]$Log,
    [string]$Prep = ""
)
$ErrorActionPreference = "Stop"
$base = "$env:USERPROFILE\rapidr-release"
New-Item -ItemType Directory -Force "$base\scripts" | Out-Null
Copy-Item -Force "$PSScriptRoot\*" "$base\scripts\"
$rest = @($args | ForEach-Object { if ("$_" -match '\s') { "`"$_`"" } else { "$_" } })
if ($Prep) {
    robocopy $Prep "$base\prep" /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { Write-Host "copying $Prep failed (robocopy $LASTEXITCODE)"; exit 1 }
    $rest = @("-Prep", "$base\prep") + $rest
}
$line = "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$base\scripts\job.ps1`" -Script `"$base\scripts\$Script`" -Log `"$Log`" $($rest -join ' ')"
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = $line; CurrentDirectory = $env:USERPROFILE }
if ($r.ReturnValue -ne 0) { Write-Host "Win32_Process.Create failed ($($r.ReturnValue))"; exit 1 }
Write-Host "started (pid $($r.ProcessId))"
exit 0
