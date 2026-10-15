# Prints the user's profile folder (windows-vm.sh asks once, to name files in it from a call
# that runs as SYSTEM: user.ps1), then who runs this and in which session (a job started by
# detach.ps1 says the user, session 1; a call from windows-vm.sh says SYSTEM, session 0).
. "$PSScriptRoot\user.ps1"
$UserProfile
"as $([Security.Principal.WindowsIdentity]::GetCurrent().Name), session $((Get-Process -Id $PID).SessionId)"
exit 0
