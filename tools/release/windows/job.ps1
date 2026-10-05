# Runs a script with its arguments, its output in a log, then its exit code in
# <log>.done (tools/release/windows-vm.sh starts this detached and polls).
param([Parameter(Mandatory = $true)][string]$Script, [Parameter(Mandatory = $true)][string]$Log)
$code = 1
try {
    # (the scripts end with `exit <code>`)
    $global:LASTEXITCODE = 0
    & $Script @args *> $Log
    $code = $LASTEXITCODE
} catch {
    $_ | Out-String | Add-Content $Log
    $code = 1
}
Set-Content "$Log.done" $code
