# Stops the release jobs running in this VM (windows-vm.sh stop): their scheduled tasks
# and the processes they started (cargo, rustc, clang, python, ISCC, the smoke's rapidr).
# Only RapidR's own release jobs: nothing else of the user's, and no other lane's work.
$ErrorActionPreference = "Continue"
. "$PSScriptRoot\user.ps1"
Get-ScheduledTask -TaskName "rapidr-release-*" -ErrorAction SilentlyContinue | ForEach-Object {
    Stop-ScheduledTask -TaskName $_.TaskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $_.TaskName -Confirm:$false -ErrorAction SilentlyContinue
    "stopped $($_.TaskName)"
}
$base = "$UserProfile\rapidr-release"
$jobs = Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "powershell.exe" -and $_.CommandLine -like "*$base\scripts\job.ps1*" }
function Tree($id) { Get-CimInstance Win32_Process -Filter "ParentProcessId = $id" | ForEach-Object { Tree $_.ProcessId; $_ } }
foreach ($j in $jobs) {
    # (the headless console host the task started it through goes with it)
    $all = @(Tree $j.ProcessId) + @($j)
    foreach ($p in $all) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue; "killed $($p.ProcessId) $($p.Name)" }
    $host_ = Get-CimInstance Win32_Process -Filter "ProcessId = $($j.ParentProcessId)" -ErrorAction SilentlyContinue
    if ($host_ -and $host_.Name -eq "conhost.exe" -and $host_.CommandLine -like "*--headless*") { Stop-Process -Id $host_.ProcessId -Force -ErrorAction SilentlyContinue; "killed $($host_.ProcessId) conhost.exe" }
}
# (what a stopped task leaves running: its tools, by what they work on)
Get-CimInstance Win32_Process | Where-Object { $_.Name -ne "powershell.exe" -and $_.ProcessId -ne $PID -and ("$($_.CommandLine)" -like "*rapidr-release*" -or "$($_.CommandLine)" -like "*rapidr-smoke-*") -and "$($_.CommandLine)" -notlike "*windows\clean.ps1*" -and "$($_.CommandLine)" -notlike "*windows\stop.ps1*" -and $_.Name -ne "cmd.exe" } | ForEach-Object {
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; "killed $($_.ProcessId) $($_.Name)"
}
exit 0
