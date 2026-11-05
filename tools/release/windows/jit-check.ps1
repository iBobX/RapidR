# The JIT on Windows (docs/jit-design.md): the branch's sources (prepare: a `git archive` in
# -Prep's src.tar) built in the VM, then the JIT's own tests (every edge value through compiled
# code), the conformance suite with the interpreter alone and with the JIT compiling all it can
# (RAPIDR_JIT=always), and the speed of three programs both ways.
#
#   tools/release/windows-vm.sh jit [Target=x86_64-pc-windows-msvc]
#
# Target: a Rust target to build for (default: the VM's own, ARM64); an x64 build runs under
# Windows' own emulation. Its own folder (%USERPROFILE%\rapidr-jit\<target>), kept for the next
# run's incremental build. Nothing here prints, touches the user's registry store or settings.
# (This file is read as ANSI by Windows PowerShell: no character outside ASCII in its code.)
param(
    [Parameter(Mandatory = $true)][string]$Prep,
    [string]$Target = ""
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$node = "$env:USERPROFILE\node\node.exe"
$name = if ($Target) { $Target } else { "host" }
$W = "$env:USERPROFILE\rapidr-jit\$name"
$src = "$W\src"
cmd /c "rmdir /s /q `"$src`" 2>nul" | Out-Null
New-Item -ItemType Directory -Force $src, "$W\prints" | Out-Null
& tar.exe -m -xf "$Prep\src.tar" -C $src
if ($LASTEXITCODE -ne 0) { Write-Host "extracting the sources failed"; exit 1 }
$env:CARGO_TARGET_DIR = "$W\target"
$env:RAPIDR_PRINT_TO = "$W\prints"; $env:RAPIDR_REGISTRY = "$W\registry.reg"; $env:RAPIDR_AI_KEYSTORE = "memory"
$env:RAPIDR_BUILD_CACHE = "$W\build-cache"; $env:LANGUAGE = "en_US.UTF-8"; $env:RAPIDR_LANG = "en"
if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
Push-Location $src
$fail = 0
$targs = @()
$out = "$W\target\release"
if ($Target) { $targs = @("--target", $Target); $out = "$W\target\$Target\release" }
Write-Host "== $(& rustc -vV | Select-String host), building rapidr for $name"
& cargo build --release -q -p rapidr-cli @targs 2>&1 | Select-String -Pattern "^error" -Context 0, 6 | Select-Object -First 20 | Write-Host
if (-not (Test-Path "$out\rapidr.exe")) { Write-Host "the build failed"; Pop-Location; exit 1 }
Copy-Item -Force "$out\rapidr.exe" "$src\rapidr.exe"
Write-Host "== the JIT's tests (every edge value)"
& cargo test --release -q -p rapidr-jit @targs 2>&1 | Select-String -Pattern "test result|panicked|^error" | Select-Object -First 20 | Write-Host
if ($LASTEXITCODE -ne 0) { $fail = 1 }
Write-Host "== rapidr run --verbose"
foreach ($p in @("loop10m", "fcalls", "dsp_scalars", "calls")) {
    $t0 = Get-Date
    & "$src\rapidr.exe" run --verbose "tools\bench\vm\$p.rr" 2>&1 | Write-Host
    $t1 = Get-Date
    $env:RAPIDR_JIT = "off"
    & "$src\rapidr.exe" run "tools\bench\vm\$p.rr" 2>&1 | Out-Null
    $t2 = Get-Date
    Remove-Item Env:\RAPIDR_JIT
    Write-Host ("   {0}: {1:N0} ms with the JIT, {2:N0} ms without (wall)" -f $p, ($t1 - $t0).TotalMilliseconds, ($t2 - $t1).TotalMilliseconds)
}
if (Test-Path $node) {
    Write-Host "== conformance (the interpreter, and RAPIDR_JIT=always)"
    try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch { }
    # (each run its own output file: a run left behind by a VM that was paused mid-way holds
    # the last one's — windows-vm.sh stop ends it. The work folder is the sources' own
    # tests\conformance\.work: the cases write their files there)
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    New-Item -ItemType Directory -Force "$src\tests\conformance\.work" | Out-Null
    & $node tests\conformance\run.mjs --backend vm,vm-jit *> "$W\conformance-$stamp.txt"
    $c = Get-Content "$W\conformance-$stamp.txt"
    $c | Where-Object { $_ -match "^FAIL|^XPASS|passed," } | Select-Object -First 60 | Write-Host
    $c | Where-Object { $_ -match "^\s+\|" } | Select-Object -First 40 | Write-Host
    if (-not (($c -join "`n") -match "\d+ passed, \d+ known failures \(xfail\), 0 failed")) { $fail = 1 }
} else { Write-Host "no node.exe: conformance skipped" }
Pop-Location
if ($fail -eq 0) { Write-Host "jit-check: all passed" } else { Write-Host "jit-check: FAILED" }
exit $fail
