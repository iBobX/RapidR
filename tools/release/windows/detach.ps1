# Starts one of these scripts (job.ps1: its log, its exit code) outside this session's
# process tree: Parallels' `prlctl exec` ends what it started when it returns. A one-off
# scheduled task runs it in the user's own session (WMI's Win32_Process.Create put it in
# its provider host's job object, whose quotas ended long builds); job.ps1 removes the
# task when it ends. tools/release/windows-vm.sh polls the log.
#
#   detach.ps1 -Script <name.ps1> -Log <file> [-Prep <share path>] [-Pass "Name=value;Switch;…"]
#
# The job reads no share (it may run in another logon session than this one), so what it
# needs is copied here first: these scripts, and -Prep (prepare.sh's
# output) when given, into %USERPROFILE%\rapidr-release\{scripts,prep}.
param(
    [Parameter(Mandatory = $true)][string]$Script,
    [Parameter(Mandatory = $true)][string]$Log,
    [string]$Prep = "",
    [string]$Pass = ""
)
$ErrorActionPreference = "Stop"
$base = "$env:USERPROFILE\rapidr-release"
New-Item -ItemType Directory -Force "$base\scripts" | Out-Null
Copy-Item -Force "$PSScriptRoot\*" "$base\scripts\"
if ($Prep) {
    robocopy $Prep "$base\prep" /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { Write-Host "copying $Prep failed (robocopy $LASTEXITCODE)"; exit 1 }
    $Pass = (@("Prep=$base\prep") + @($Pass | Where-Object { $_ })) -join ";"
}
$task = "rapidr-release-" + [IO.Path]::GetFileNameWithoutExtension($Log)
$argline = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$base\scripts\job.ps1`" -Script `"$base\scripts\$Script`" -Log `"$Log`" -Pass `"$Pass`" -Task $task"
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument $argline -WorkingDirectory $env:USERPROFILE
$principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $task -Action $action -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName $task
Write-Host "started ($task)"
exit 0
