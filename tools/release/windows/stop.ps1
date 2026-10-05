# Stops the release jobs running in this VM (windows-vm.sh stop): their scheduled tasks
# and the processes they started (cargo, rustc, clang, python, ISCC, the smoke's rapidr).
$ErrorActionPreference = "Continue"
Get-ScheduledTask -TaskName "rapidr-release-*" -ErrorAction SilentlyContinue | ForEach-Object {
    Stop-ScheduledTask -TaskName $_.TaskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $_.TaskName -Confirm:$false -ErrorAction SilentlyContinue
    "stopped $($_.TaskName)"
}
$base = "$env:USERPROFILE\rapidr-release"
$jobs = Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "powershell.exe" -and $_.CommandLine -like "*$base\scripts\job.ps1*" }
function Tree($id) { Get-CimInstance Win32_Process -Filter "ParentProcessId = $id" | ForEach-Object { Tree $_.ProcessId; $_ } }
foreach ($j in $jobs) {
    $all = @(Tree $j.ProcessId) + @($j)
    foreach ($p in $all) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue; "killed $($p.ProcessId) $($p.Name)" }
}
# (what a stopped task leaves running: its tools, by what they work on)
Get-CimInstance Win32_Process | Where-Object { $_.Name -ne "powershell.exe" -and ("$($_.CommandLine)" -like "*rapidr-release*" -or "$($_.CommandLine)" -like "*rapidr-smoke-*") } | ForEach-Object {
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; "killed $($_.ProcessId) $($_.Name)"
}
exit 0
