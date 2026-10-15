# Removes what RapidR's release and smoke scripts leave in this VM (windows-vm.sh clean):
# the release work folder (builds, cargo target, logs), the build cache, the smoke tests'
# temporary folders and what an interrupted smoke test left registered for the user (its
# file types, Uninstall entry, Start menu shortcut), the test apps they built.
# A DRY RUN (lists, with sizes) unless -Delete.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File clean.ps1 [-Delete] [-KeepOut]
#
# Kept, always: the user's installed RapidR (only a registration pointing into a
# rapidr-smoke-* folder is removed), rapidr-tools (the toolchain: setup-tools.ps1), the
# user's own Rust, files and apps. Running programs are never closed: a file in use is
# reported and skipped. -KeepOut keeps rapidr-release\out (the finished installers).
# The profile's other RapidR folders (rapidr, rapidr-ui, csys2-*, rq …: earlier lanes'
# scratch) are only listed, never removed: someone may still be using them.
param(
    [switch]$Delete,
    [switch]$KeepOut
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
. "$PSScriptRoot\user.ps1"
$local = "$UserProfile\AppData\Local"
$hive = "Registry::HKEY_USERS\$UserSid"
$temps = @("$local\Temp", "$env:SystemRoot\Temp") | Select-Object -Unique
function Free-GB { (Get-PSDrive ($env:SystemDrive.TrimEnd(":"))).Free / 1GB }

# A folder's size in bytes (cmd's dir is far faster than Get-ChildItem on a cargo target folder).
function Size-Of($path) {
    if (Test-Path -LiteralPath $path -PathType Leaf) { return (Get-Item -LiteralPath $path -Force).Length }
    $sum = 0
    foreach ($l in (cmd /c "dir /s /a-d /-c `"$path`" 2>nul" | Select-Object -Last 3)) { if ($l -match "^\s+\d+ File\(s\)\s+(\d+) bytes") { $sum = [int64]$Matches[1] } }
    return $sum
}

# What is ours. Names are only ever matched inside the folders named here.
$found = New-Object System.Collections.ArrayList
function Add-Path($path, $why) { if (Test-Path -LiteralPath $path) { [void]$found.Add([pscustomobject]@{ Path = $path; Why = $why }) } }

$rel = "$UserProfile\rapidr-release"
if (Test-Path -LiteralPath $rel) {
    foreach ($item in Get-ChildItem -LiteralPath $rel -Force) {
        if ($KeepOut -and $item.Name -eq "out") { continue }
        if ($item.Name -eq "scripts") { continue }          # (the job helpers, tiny: removed last, below)
        Add-Path $item.FullName "release work"
    }
}
Add-Path "$local\RapidR\Cache\build" "build cache (safe to delete)"
foreach ($t in $temps) {
    foreach ($pat in "rapidr-*", "rapidr_*", "rapidr.*", "smoke*", "rr-*.log*") {
        Get-ChildItem -LiteralPath $t -Force -Filter $pat -ErrorAction SilentlyContinue | ForEach-Object { Add-Path $_.FullName "temp leftover" }
    }
}
# Test apps the smoke tests built where RapidR is installed (Documents\RapidR\Builds\<name>):
# only the names the smoke tests use. A user's own app there is never matched.
$smokeApps = "hello", "hello_form", "gui", "native", "list_items", "arithmetic", "arrays", "control_flow", "functions", "gosub_goto", "data_read", "newer", "downloaded", "script"
foreach ($n in $smokeApps) { Add-Path "$UserProfile\Documents\RapidR\Builds\$n" "smoke-test app" }

# What an interrupted smoke test left registered for the user: only what points into a
# rapidr-smoke-* folder (a real install, in its own folder, is never touched).
$smoke = "\rapidr-smoke-"
$progIds = "RapidR.Program", "RapidR.Source", "RapidR.Project", "RapidR.BasicSource", "RapidR.RapidQSource"
$regLeft = New-Object System.Collections.ArrayList
if (Test-Path $hive) {
    Get-ChildItem "$hive\Software\Microsoft\Windows\CurrentVersion\Uninstall" -ErrorAction SilentlyContinue | ForEach-Object {
        $p = Get-ItemProperty $_.PSPath
        if ("$($p.InstallLocation)$($p.UninstallString)" -like "*$smoke*") { [void]$regLeft.Add("$hive\Software\Microsoft\Windows\CurrentVersion\Uninstall\$($_.PSChildName)") }
    }
    foreach ($id in $progIds) {
        $k = "$hive\Software\Classes\$id"
        $cmd = foreach ($sub in "shell\open\command", "shell\run\command") { if (Test-Path "$k\$sub") { (Get-ItemProperty "$k\$sub")."(default)" } }
        if ("$cmd" -like "*$smoke*") { [void]$regLeft.Add($k) }
    }
}
# (the extensions that name one of those file types, or one already gone: what a first clean's
# registry removal left behind must not stay; a file type in a real install exists and is its own)
$exts = ".rrbc", ".rr", ".rrproj", ".bas", ".rqw", ".rqb", ".rq"
$smokeIds = @($regLeft | Where-Object { $_ -like "*\Classes\*" } | ForEach-Object { Split-Path $_ -Leaf })
$goneIds = @($progIds | Where-Object { $_ -in $smokeIds -or -not (Test-Path "$hive\Software\Classes\$_") })
$extLeft = @()
if (Test-Path $hive) {
    foreach ($ext in $exts) {
        $k = "$hive\Software\Classes\$ext"
        if (-not (Test-Path $k)) { continue }
        $owp = if (Test-Path "$k\OpenWithProgids") { @((Get-Item "$k\OpenWithProgids").GetValueNames()) } else { @() }
        if (((Get-ItemProperty $k)."(default)" -in $goneIds) -or ($owp | Where-Object { $_ -in $goneIds })) { $extLeft += $k }
    }
}
$lnk = "$UserProfile\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\RapidR Studio.lnk"
$lnkLeft = (Test-Path -LiteralPath $lnk) -and ((New-Object -ComObject WScript.Shell).CreateShortcut($lnk).TargetPath -like "*$smoke*")

Write-Host ("free on {0}: {1:N1} GB" -f $env:SystemDrive, (Free-GB))
$total = 0
$rows = foreach ($f in $found) {
    $b = Size-Of $f.Path
    $total += $b
    [pscustomobject]@{ MB = [math]::Round($b / 1MB, 1); Path = $f.Path; Why = $f.Why }
}
$rows | Sort-Object MB -Descending | ForEach-Object { "{0,10:N1} MB  {1}  ({2})" -f $_.MB, $_.Path, $_.Why }
foreach ($r in $regLeft) { "{0,13}  {1}  (left registered by a smoke test)" -f "", $r }
foreach ($r in $extLeft) { "{0,13}  {1}  (names a file type that is gone)" -f "", $r }
if ($lnkLeft) { "{0,13}  {1}  (left by a smoke test)" -f "", $lnk }
"{0,10:N1} MB  total ({1} items)" -f ($total / 1MB), (@($found).Count + @($regLeft).Count + @($extLeft).Count + [int]$lnkLeft)

if (-not $Delete) {
    # (listed only: never removed by this script)
    "-- other RapidR folders in $UserProfile (earlier lanes' scratch; not removed here)"
    Get-ChildItem $UserProfile -Force -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match "^(rapidr|rr-|rq|csys|cdx|bpkg|studio-)" -and $_.Name -notin "rapidr-tools", "rapidr-release" } |
        ForEach-Object { [pscustomobject]@{ GB = [math]::Round((Size-Of $_.FullName) / 1GB, 2); Date = $_.LastWriteTime.ToString("yyyy-MM-dd"); Name = $_.Name } } |
        Sort-Object GB -Descending | ForEach-Object { "{0,10:N2} GB  {1}  {2}" -f $_.GB, $_.Date, $_.Name }
    "(a dry run: -Delete removes the items above)"
    exit 0
}

function Remove-Tree($path) {
    if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue; return }
    cmd /c "rmdir /s /q `"$path`" 2>nul" | Out-Null
    if (Test-Path -LiteralPath $path) {
        # (paths past 260 characters: mirroring an empty folder over it empties it)
        $empty = Join-Path $env:SystemRoot ("Temp\rapidr-empty-" + [guid]::NewGuid().ToString("N"))
        New-Item -ItemType Directory -Force $empty | Out-Null
        robocopy $empty $path /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
        Remove-Item -LiteralPath $empty -Force -ErrorAction SilentlyContinue
        cmd /c "rmdir /s /q `"$path`" 2>nul" | Out-Null
    }
}
# The registrations first: the extensions' pointers to those file types, then the types themselves.
foreach ($k in $extLeft) {
    if ((Get-ItemProperty $k)."(default)" -in $goneIds) { reg delete ($k -replace "^Registry::HKEY_USERS", "HKU") /ve /f | Out-Null }
    foreach ($id in $goneIds) { Remove-ItemProperty "$k\OpenWithProgids" -Name $id -ErrorAction SilentlyContinue }
    # (the extension's key, once nothing else is in it: a .bas the user set up otherwise stays)
    $item = Get-Item $k
    $subs = @($item.GetSubKeyNames())
    $onlyEmptyOpenWith = $subs.Count -eq 1 -and $subs[0] -eq "OpenWithProgids" -and (Get-Item "$k\OpenWithProgids").ValueCount -eq 0
    if ($item.ValueCount -eq 0 -and ($subs.Count -eq 0 -or $onlyEmptyOpenWith)) { Remove-Item $k -Recurse -ErrorAction SilentlyContinue; "  unregistered: $k" }
}
foreach ($r in $regLeft) { Remove-Item -LiteralPath $r -Recurse -Force -ErrorAction SilentlyContinue; "  unregistered: $r" }
if ($lnkLeft) { Remove-Item -LiteralPath $lnk -Force; "  removed: $lnk" }
foreach ($f in $found) {
    Remove-Tree $f.Path
    if (Test-Path -LiteralPath $f.Path) { "  in use, left: $($f.Path)" } else { "  removed: $($f.Path)" }
}
# (the job helpers, and the work folder itself once nothing is left in it)
if (Test-Path -LiteralPath "$rel\scripts") { Remove-Tree "$rel\scripts" }
if ((Test-Path -LiteralPath $rel) -and -not (Get-ChildItem -LiteralPath $rel -Force)) { Remove-Item -LiteralPath $rel -Force }
Write-Host ("free on {0}: {1:N1} GB" -f $env:SystemDrive, (Free-GB))
exit 0
