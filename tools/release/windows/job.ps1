# Runs a script with its parameters, its output in a log, then its exit code in
# <log>.done (tools/release/windows-vm.sh starts this detached and polls).
#   job.ps1 -Script <x.ps1> -Log <file> [-Pass "Name=value;Switch;…"]
param(
    [Parameter(Mandatory = $true)][string]$Script,
    [Parameter(Mandatory = $true)][string]$Log,
    [string]$Pass = ""
)
# (the script's own parameters, by name: Name=value, or Switch alone)
$named = @{}
foreach ($p in ($Pass -split ";" | Where-Object { $_ })) {
    $kv = $p -split "=", 2
    if ($kv.Count -eq 2) {
        $named[$kv[0]] = if ($kv[1] -match ",") { $kv[1] -split "," } else { $kv[1] }
    } else { $named[$kv[0]] = $true }
}
$code = 1
try {
    # (the scripts end with `exit <code>`)
    $global:LASTEXITCODE = 0
    & $Script @named *> $Log
    $code = $LASTEXITCODE
} catch {
    $_ | Out-String | Add-Content $Log
    $code = 1
}
Set-Content "$Log.done" $code
