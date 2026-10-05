# QREGISTRY on Windows' own registry, checked on a Windows machine. Runs
# registry_check.bas with RAPIDR_REGISTRY (the per-user store, as on macOS,
# Linux and the web), then without it (Windows' registry) three ways:
# interpreted (rapidr run-bc), an interpreted build (rapidr build --interp)
# and a native build (rapidr build). Each registry run must print what the
# store run printed up to its "machine" line, and leave what reg query
# expects under HKEY_CURRENT_USER\Software\RapidR-Test - the only key the
# program writes to, deleted at the end. Refuses to run elevated (writes to
# HKEY_LOCAL_MACHINE must be refused).
#
#   powershell -ExecutionPolicy Bypass -File tools\windows\registry_check.ps1 [-Rapidr rapidr.exe] [-SkipBuilds]
#
# From the repo root, after building rapidr.exe (cargo build --release -p
# rapidr-cli). The builds use CARGO_TARGET_DIR when it is set. Programs
# print to %TEMP% (RAPIDR_PRINT_TO), never on paper.
param([string]$Rapidr = ".\rapidr.exe", [switch]$SkipBuilds)

$id = [Security.Principal.WindowsIdentity]::GetCurrent()
if ((New-Object Security.Principal.WindowsPrincipal($id)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  Write-Output "elevated: run it as a normal user (writes to HKEY_LOCAL_MACHINE must be refused)"
  exit 2
}
$Rapidr = (Resolve-Path $Rapidr).Path
$Bas = Join-Path $PSScriptRoot "registry_check.bas"
$Work = Join-Path $env:TEMP "rapidr-registry-check"
Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$Work\prints", "$Work\interp", "$Work\native" | Out-Null
$env:RAPIDR_PRINT_TO = "$Work\prints"
$Key = "HKCU\Software\RapidR-Test"
$script:failed = 0

function Fail([string]$why) { Write-Host "FAIL $why"; $script:failed++ }
function Test-Key { cmd /c "reg query $Key >nul 2>&1"; return ($LASTEXITCODE -eq 0) }
function Clear-Key { if (Test-Key) { reg delete $Key /f | Out-Null } }
# (kinds of value QREGISTRY can't write, as another program would put them)
function Set-Seed {
  Clear-Key
  reg add "$Key\Seed" /v Multi /t REG_MULTI_SZ /d 'a\0b' /f | Out-Null
  reg add "$Key\Seed" /v Qword /t REG_QWORD /d 1 /f | Out-Null
  reg add "$Key\Seed" /v Path /t REG_EXPAND_SZ /d '%TMP%' /f | Out-Null
}
# A program's output (trailing spaces off), its exit code checked.
function Invoke-Program([string]$name, [string]$exe, [string[]]$argList) {
  $out = "$Work\$name.txt"
  if ($argList) {
    $quoted = $argList | ForEach-Object { '"' + $_ + '"' }
    $p = Start-Process -FilePath $exe -ArgumentList $quoted -NoNewWindow -Wait -PassThru -RedirectStandardOutput $out -RedirectStandardError "$out.err"
  } else {
    $p = Start-Process -FilePath $exe -NoNewWindow -Wait -PassThru -RedirectStandardOutput $out -RedirectStandardError "$out.err"
  }
  if ($p.ExitCode -ne 0) { Fail "$name exited with $($p.ExitCode): $(Get-Content -Raw "$out.err")" }
  return ((Get-Content $out) | ForEach-Object { $_.TrimEnd() })
}
function Get-Head([string[]]$lines) {
  $i = [Array]::IndexOf($lines, "machine")
  if ($i -lt 1) { return $lines }
  return $lines[0..($i - 1)]
}
# What the program leaves, as reg query shows it.
function Test-Left([string]$name) {
  $q = (reg query $Key /s) -join "`n"
  $want = @(
    "RapidR-Test\\Final`n", "\(Default\)\s+REG_SZ\s+the default", "Empty\s+REG_SZ", "Int\s+REG_DWORD\s+0xfffffffb",
    "Float\s+REG_BINARY\s+0000000000000A40", "Bin\s+REG_BINARY\s+0700FF09", "FromText\s+REG_BINARY\s+616263",
    "Text\s+REG_SZ\s+x=1;y=2", "Number\s+REG_DWORD\s+0x12345678", "Final\\A2", "Final\\c\\deep",
    "Final\\Seed`n\s+Multi\s+REG_MULTI_SZ\s+a\\0b`n\s+Qword\s+REG_QWORD\s+0x1`n\s+Path\s+REG_EXPAND_SZ\s+%TMP%")
  foreach ($w in $want) { if ($q -notmatch $w) { Fail "$name, reg query lacks $w" } }
  foreach ($gone in @("RapidR-Test\\Work", "RapidR-Test\\Seed", "RapidR-Test\\Elsewhere", "Renamed")) {
    if ($q -match $gone) { Fail "$name, reg query still has $gone" }
  }
}

# The store (the registry key left by a run stopped half-way: gone first)
Clear-Key
$env:RAPIDR_REGISTRY = "$Work\store.reg"
Set-Content -Encoding Ascii $env:RAPIDR_REGISTRY @(
  "Windows Registry Editor Version 5.00", "",
  "[HKEY_CURRENT_USER\Software\RapidR-Test\Seed]",
  '"Multi"=hex(7):61,00,00,00,62,00,00,00,00,00',
  '"Qword"=hex(b):01,00,00,00,00,00,00,00',
  '"Path"=hex(2):25,00,54,00,4d,00,50,00,25,00,00,00')
& $Rapidr build-bc $Bas -o "$Work\check.rrbc" | Out-Null
if ($LASTEXITCODE -ne 0) { Fail "build-bc"; exit 1 }
$store = Invoke-Program "store" $Rapidr @("run-bc", "$Work\check.rrbc")
if ((Test-Key)) { Fail "the store run wrote to the registry" }
$store | ForEach-Object { Write-Output "  $_" }

# Windows' registry
Remove-Item Env:\RAPIDR_REGISTRY
$runs = @(@{ name = "run-bc"; exe = $Rapidr; args = @("run-bc", "$Work\check.rrbc") })
if (-not $SkipBuilds) {
  Copy-Item $Bas "$Work\interp\check.bas"
  Copy-Item $Bas "$Work\native\check.bas"
  & $Rapidr build "$Work\interp\check.bas" --interp *> "$Work\build-interp.log"
  if ($LASTEXITCODE -ne 0) { Fail "the interpreted build (build-interp.log)" } else { $runs += @{ name = "interpreted build"; exe = "$Work\interp\check.exe"; args = $null } }
  & $Rapidr build "$Work\native\check.bas" *> "$Work\build-native.log"
  if ($LASTEXITCODE -ne 0) { Fail "the native build (build-native.log)" } else { $runs += @{ name = "native build"; exe = "$Work\native\check.exe"; args = $null } }
}
foreach ($run in $runs) {
  Set-Seed
  $out = Invoke-Program ($run.name -replace " ", "-") $run.exe $run.args
  $diff = Compare-Object (Get-Head $store) (Get-Head $out) -SyncWindow 0
  if ($diff) { Fail "$($run.name) printed otherwise than the store:"; $diff | Format-Table -AutoSize | Out-String | Write-Output }
  $i = [Array]::IndexOf($out, "machine")
  if ($i -lt 0 -or $out[$i + 1] -match "none") { Fail "$($run.name) didn't reach the machine's keys" }
  Write-Output "$($run.name): the same as the store; then"
  $out[$i..($out.Count - 1)] | ForEach-Object { Write-Output "  $_" }
  Test-Left $run.name
}
cmd /c "reg query HKLM\SOFTWARE\RapidR-Test >nul 2>&1"
if ($LASTEXITCODE -eq 0) { Fail "HKEY_LOCAL_MACHINE\SOFTWARE\RapidR-Test was made" }
Clear-Key
if (Test-Key) { Fail "$Key is still there" } else { Write-Output "$Key deleted" }
if ($script:failed) { Write-Output "$($script:failed) failed"; exit 1 }
Write-Output "all passed"
