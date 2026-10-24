# Does the antivirus in the Windows VM (Bitdefender, real-time on) let the
# Windows 7 alpha zip be unzipped and rapidr.exe be run? Run as SYSTEM through
#   tools/release/windows-vm.sh sync tools/win7/av_check.ps1 -Zip '\\Mac\Home\...\RapidR-...-alpha1.zip'
# (never while a release job is running in the VM: it checks and refuses).
#
# The zip is copied to C:\rapidr-win7-av (NOT under a Temp folder: those have
# an exception), unzipped there, `rapidr.exe version` and the two console
# tests are run (the 32-bit program runs under x86 emulation on Windows ARM),
# and Bitdefender's quarantine and pending-actions files are compared before
# and after. Bitdefender's settings are never touched; the folder is removed
# at the end. Prints plain text.
param(
    [Parameter(Mandatory = $true)][string]$Zip,
    [string]$Dir = 'C:\rapidr-win7-av',
    [switch]$Keep
)
$ErrorActionPreference = 'Continue'
$bd = 'C:\ProgramData\Bitdefender\Desktop\Quarantine'

if ($Dir -ne 'C:\rapidr-win7-av') { throw "refusing: -Dir must be C:\rapidr-win7-av (only that folder is ever emptied)" }
# (a release build or smoke test: tools/release/windows-vm.sh starts them as scripts\job.ps1; or a build is running)
$busy = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -in 'cargo.exe', 'rustc.exe' -or $_.CommandLine -like '*rapidr-release\scripts\job.ps1*' })
if ($busy.Count) { "A release job is running in the VM ($(($busy | Select-Object -Expand Name -Unique) -join ', ')): not now."; exit 3 }

function Snapshot {
    $items = Get-ChildItem $bd -File -ErrorAction SilentlyContinue | ForEach-Object { "{0}|{1}|{2}" -f $_.Name, $_.Length, $_.LastWriteTimeUtc.Ticks }
    $pend = Join-Path $bd 'pending_actions.json'
    $ph = if (Test-Path $pend) { (Get-FileHash $pend -Algorithm SHA256).Hash } else { '' }
    [pscustomobject]@{ Items = @($items); PendingHash = $ph }
}

"== $(Get-Date)  on $env:COMPUTERNAME, $((Get-CimInstance Win32_OperatingSystem).Caption)"
"AV: " + ((Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntivirusProduct | ForEach-Object { $_.displayName }) -join ', ')
$before = Snapshot
"quarantine before: $($before.Items.Count) files; pending_actions.json $($before.PendingHash.Substring(0,[Math]::Min(12,$before.PendingHash.Length)))"

if (Test-Path $Dir) { Remove-Item -Recurse -Force $Dir }
New-Item -ItemType Directory $Dir | Out-Null
$local = Join-Path $Dir (Split-Path $Zip -Leaf)
Copy-Item $Zip $local -ErrorAction Stop
$want = ''
$shaFile = "$Zip.sha256"
if (Test-Path $shaFile) { $want = ((Get-Content $shaFile -TotalCount 1) -split '\s+')[0].ToLower() }
$got = (Get-FileHash $local -Algorithm SHA256).Hash.ToLower()
"zip: $((Get-Item $local).Length) bytes, sha256 $got " + $(if ($want -eq $got) { '(matches the .sha256 file)' } elseif ($want) { "(DIFFERS from $want)" } else { '(no .sha256 file beside it)' })

Add-Type -AssemblyName System.IO.Compression.FileSystem
$x = Join-Path $Dir 'unz'
$t = Get-Date
[IO.Compression.ZipFile]::ExtractToDirectory($local, $x)
"unzipped in $([int]((Get-Date) - $t).TotalSeconds) s"
Start-Sleep -Seconds 8   # (the on-access scan of the new files finishes)
$root = Get-ChildItem $x -Directory | Select-Object -First 1
$exe = Join-Path $root.FullName 'rapidr.exe'
$n = (Get-ChildItem $root.FullName -Recurse -File).Count
$entries = [IO.Compression.ZipFile]::OpenRead($local); $inZip = ($entries.Entries | Where-Object { $_.Name }).Count; $entries.Dispose()
"files: $n unzipped of $inZip in the zip" + $(if ($n -ne $inZip) { '   <-- SOME ARE MISSING (quarantined?)' } else { '' })
if (-not (Test-Path $exe)) { "rapidr.exe IS MISSING after unzipping: the antivirus took it" } else {
    $vi = (Get-Item $exe).VersionInfo
    "rapidr.exe: $((Get-Item $exe).Length) bytes; CompanyName='$($vi.CompanyName)' ProductName='$($vi.ProductName)' FileVersion='$($vi.FileVersion)' ProductVersion='$($vi.ProductVersion)' FileDescription='$($vi.FileDescription)'"
}

function Run([string]$what, [string[]]$arguments, [int]$seconds = 60) {
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = ($arguments -join ' ')
    $psi.WorkingDirectory = $root.FullName
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.EnvironmentVariables['RAPIDR_HOME'] = $root.FullName
    $psi.EnvironmentVariables['RAPIDR_REGISTRY'] = Join-Path $Dir 'registry'
    $psi.EnvironmentVariables['RAPIDR_PRINT_TO'] = Join-Path $Dir 'print'
    $psi.EnvironmentVariables['RAPIDR_AI_KEYSTORE'] = 'memory'
    "-- $what"
    try {
        $p = [Diagnostics.Process]::Start($psi)
        $o = $p.StandardOutput.ReadToEndAsync(); $e = $p.StandardError.ReadToEndAsync()
        if (-not $p.WaitForExit($seconds * 1000)) { $p.Kill(); "   TIMED OUT after $seconds s" }
        else { "   exit code $($p.ExitCode)" }
        $out = ($o.Result + $e.Result).Trim()
        if ($out.Length -gt 600) { $out = $out.Substring(0, 600) + ' …' }
        ($out -split "`r?`n" | ForEach-Object { "   | $_" })
    } catch {
        "   COULD NOT START: $($_.Exception.Message)   <-- the antivirus holds or blocks it"
    }
}
if (Test-Path $exe) {
    Run 'rapidr.exe version' @('version')
    Run 'the six newer-Windows functions (rapidr.exe __win7_check)' @('__win7_check')
    Run 'console program: hello.rr' @('run', 'tests\hello.rr')
    Run 'language tour: language.rr (first lines)' @('run', 'tests\language.rr')
}

Start-Sleep -Seconds 5
$after = Snapshot
$new = $after.Items | Where-Object { $before.Items -notcontains $_ }
"quarantine after: $($after.Items.Count) files; " + $(if ($new) { "NEW OR CHANGED: " + (($new | ForEach-Object { ($_ -split '\|')[0] }) -join ', ') } else { 'nothing new or changed' })
"pending_actions.json: " + $(if ($after.PendingHash -eq $before.PendingHash) { 'unchanged' } else { 'CHANGED' })
foreach ($name in ($new | ForEach-Object { ($_ -split '\|')[0] } | Where-Object { $_ -like '*.dat' })) {
    "   $name :"
    (Get-Content (Join-Path $bd $name) -Raw -ErrorAction SilentlyContinue) -replace '[^\x20-\x7e\r\n]', '.' | Select-Object -First 1 | ForEach-Object { if ($_.Length -gt 500) { $_.Substring(0, 500) } else { $_ } }
}

if (-not $Keep) {
    Remove-Item -Recurse -Force $Dir -ErrorAction SilentlyContinue
    "cleaned up $Dir" + $(if (Test-Path $Dir) { ' (SOME REMAINS)' } else { '' })
}
