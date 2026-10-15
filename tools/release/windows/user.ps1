# Dot-sourced by the scripts that may run as SYSTEM: windows-vm.sh runs its calls without
# `--current-user` (as SYSTEM, in Windows' session 0), because a call in the user's own
# session opens a console window on the desktop every time. Sets
#   $UserName     DOMAIN\name  of the person at the computer
#   $UserProfile  C:\Users\name
#   $UserSid      S-1-5-21-…   (HKEY_USERS\<sid> is that user's HKCU while they are logged on)
# When the script runs as the user (the detached jobs do), that is the user itself.
$me = [Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $me.IsSystem) {
    $UserName = $me.Name
    $UserProfile = $env:USERPROFILE
} else {
    $UserName = (Get-CimInstance Win32_ComputerSystem).UserName
    if (-not $UserName) {
        # (nobody at the console: the profile whose registry file was written last)
        $last = Get-ChildItem "C:\Users" -Directory -Force -ErrorAction SilentlyContinue |
            Where-Object { Test-Path "$($_.FullName)\NTUSER.DAT" } | Sort-Object { (Get-Item "$($_.FullName)\NTUSER.DAT" -Force).LastWriteTime } -Descending | Select-Object -First 1
        if (-not $last) { throw "no user profile found" }
        $UserName = "$env:COMPUTERNAME\$($last.Name)"
    }
    $UserSid = (New-Object Security.Principal.NTAccount($UserName)).Translate([Security.Principal.SecurityIdentifier]).Value
    $UserProfile = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$UserSid").ProfileImagePath
}
if (-not $UserSid) { $UserSid = $me.User.Value }
