# RapidR's Windows installers, made in the Windows VM (or any Windows 11):
# the SDK and the Runtime for x64 and ARM64 (Inno Setup; per user, no admin).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File windows.ps1 -Prep <prepare.sh's prep>
#       [-Arch x86_64,aarch64] [-Toolchain gnullvm|msvc] [-Sign <certificate subject>]
#       [-Work %USERPROFILE%\rapidr-release]
#
#   -Toolchain  gnullvm (default): Rust's *-pc-windows-gnullvm, linked by LLVM-MinGW — open
#               source, its output carries no obligations beyond the mingw-w64 runtime's notice
#               (LICENSES.md §7). Each SDK ships a trimmed LLVM-MinGW for its architecture
#               (lib\rapidr\toolchain): users' native builds need no Visual Studio.
#               msvc: Visual Studio Build Tools (Microsoft's licence) with the x64 and ARM64 tools;
#               the SDK then ships no toolchain.
#   -Sign       a code-signing certificate in the user's store (signtool, Windows SDK). Without
#               it the installers are unsigned: SmartScreen warns ("More info > Run anyway").
#
# Needs what setup-tools.ps1 installs: Python 3.11+, Inno Setup 6, LLVM-MinGW (both hosts' zips,
# this machine's unpacked) in %USERPROFILE%\rapidr-tools, Rust's gnullvm toolchain. Builds in
# -Work (~10 GB: two targets); the installers go to -Work\out.
param(
    [Parameter(Mandatory = $true)][string]$Prep,
    [string[]]$Arch = @("x86_64", "aarch64"),
    [ValidateSet("gnullvm", "msvc")][string]$Toolchain = "gnullvm",
    [string]$Sign = "",
    [string]$Work = "$env:USERPROFILE\rapidr-release"
)
# (Continue: Windows PowerShell turns a native tool's stderr into errors when the output is
# redirected; failures are checked by exit code and thrown, cmdlets that matter say -ErrorAction Stop)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
function Step($m) { Write-Host "== $m" }
function Run($exe, [string[]]$a) {
    & $exe @a
    if ($LASTEXITCODE -ne 0) { throw "$exe $($a -join ' ') failed ($LASTEXITCODE)" }
}
if ($Arch.Count -eq 1 -and $Arch[0] -match ",") { $Arch = $Arch[0] -split "," }

$Tools = "$env:USERPROFILE\rapidr-tools"
$python = Get-ChildItem "$env:LOCALAPPDATA\Programs\Python\Python3*\python.exe" -ErrorAction SilentlyContinue | Select-Object -Last 1
if (-not $python) { throw "Python 3.11+ is needed (setup-tools.ps1)" }
$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 is needed (setup-tools.ps1)" }
$hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { "x86_64" }

$Src = "$Work\src"; $Out = "$Work\out"; $Stage = "$Work\stage"
Remove-Item -Recurse -Force $Src, $Stage, "$Work\home", "$Work\toolchains" -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Src, $Out, $Stage | Out-Null

Step "the release's source"
# (Python's tarfile: Windows' tar fails on the archive's symlinks, which need a privilege to
# make and nothing here uses; the files' times are now, so cargo rebuilds what changed)
$untar = "import os, sys, tarfile`nwith tarfile.open(sys.argv[1]) as t:`n    m = [x for x in t if not (x.issym() or x.islnk())]`n    t.extractall(sys.argv[2], members=m, filter='data')`nfor d, _, fs in os.walk(sys.argv[2]):`n    for f in fs: os.utime(os.path.join(d, f))"
Run $python.FullName @("-c", $untar, "$Prep\src.tar", $Src)
Set-Location $Src
$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$env:CARGO_TARGET_DIR = "$Work\target"
$env:RAPIDR_PRINT_TO = "$Work\prints"; $env:RAPIDR_REGISTRY = "$Work\registry.reg"

Step "the home: the runtime's sources, their crates vendored"
Run $python.FullName @("tools\release\home.py", "--os", "windows", "--src", $Src, "--out", "$Work\home")

if ($Toolchain -eq "gnullvm") {
    # This machine's LLVM-MinGW links every target; Rust's gnullvm toolchain builds with it,
    # build scripts included — no MSVC anywhere.
    $llvm = Get-ChildItem $Tools -Directory | Where-Object Name -like "llvm-mingw-*-ucrt-$hostArch" | Select-Object -Last 1
    if (-not $llvm) { throw "no LLVM-MinGW in $Tools (setup-tools.ps1)" }
    $env:PATH = "$($llvm.FullName)\bin;$env:PATH"
    # (the pinned one setup-tools.ps1 installs — the Rust the release is tested with — over `stable`)
    $gnullvm = (rustup toolchain list) | ForEach-Object { ($_ -split " ")[0] } | Where-Object { $_ -like "*-$hostArch-pc-windows-gnullvm" }
    $rustTc = @($gnullvm | Where-Object { $_ -match "^\d" }) + @($gnullvm) | Select-Object -First 1
    if (-not $rustTc) { throw "no gnullvm Rust toolchain (setup-tools.ps1)" }
    $env:RUSTUP_TOOLCHAIN = $rustTc
    foreach ($a in "x86_64", "aarch64") {
        $t = "$a-pc-windows-gnullvm"; $up = $t.ToUpper().Replace("-", "_"); $lo = $t.Replace("-", "_")
        $cc = "$($llvm.FullName)\bin\$a-w64-mingw32-clang.exe"
        Set-Item "env:CARGO_TARGET_${up}_LINKER" $cc
        # (libunwind and the mingw-w64 runtime linked in: executables that need no DLL beside them)
        Set-Item "env:CARGO_TARGET_${up}_RUSTFLAGS" "-C target-feature=+crt-static"
        Set-Item "env:CC_$lo" $cc
        Set-Item "env:AR_$lo" "$($llvm.FullName)\bin\llvm-ar.exe"
    }
    Write-Host "rust $rustTc, LLVM-MinGW $($llvm.Name)"
} else {
    # Rust links the MSVC runtime in (no VC++ redistributable needed by users)
    $env:RUSTFLAGS = "-C target-feature=+crt-static"
}

$triples = @{}
foreach ($a in $Arch) {
    $t = "$a-pc-windows-$Toolchain"
    $triples[$a] = $t
    Step "build $t"
    Run rustup @("target", "add", $t)
    Run cargo @("build", "-q", "--locked", "--release", "--target", $t, "-p", "rapidr-cli", "-p", "rapidr-launcher")
    Run cargo @("build", "-q", "--locked", "--profile", "runner", "--target", $t, "-p", "rapidr-runner-stub")
    # (what they import: Windows' own DLLs only)
    $dumper = "$($llvm.FullName)\bin\llvm-readobj.exe"
    if ($Toolchain -eq "gnullvm" -and (Test-Path $dumper)) {
        $dlls = & $dumper --coff-imports "$env:CARGO_TARGET_DIR\$t\release\rapidr.exe" | Select-String "Name: (.*\.dll)" | ForEach-Object { $_.Matches[0].Groups[1].Value.ToLower() } | Sort-Object -Unique
        Write-Host "  rapidr.exe imports: $($dlls -join ', ')"
        $bad = $dlls | Where-Object { $_ -match "unwind|c\+\+|winpthread|gcc|stdc" }
        if ($bad) { throw "rapidr.exe needs $($bad -join ', ')" }
    }
}

# A trimmed LLVM-MinGW for an SDK of architecture $a: the compiler and linker, the
# runtime and headers for that architecture, the licences (no debugger, Python,
# other targets' libraries).
function Trim-Toolchain($a) {
    $zip = Get-ChildItem "$Tools\llvm-mingw-*-ucrt-$a.zip" | Select-Object -Last 1
    if (-not $zip) { throw "no llvm-mingw ucrt-$a zip in $Tools" }
    $dir = "$Work\toolchains\$a"
    New-Item -ItemType Directory -Force "$Work\toolchains" | Out-Null
    Expand-Archive -ErrorAction Stop -Path $zip.FullName -DestinationPath "$Work\toolchains\unz-$a" -Force
    $full = Get-ChildItem "$Work\toolchains\unz-$a" -Directory | Select-Object -First 1
    New-Item -ItemType Directory -Force "$dir\bin", "$dir\lib\clang" | Out-Null
    $keep = "^(clang|clang\+\+|clang-\d+|clang-target-wrapper|$a-w64-mingw32-(clang|clang\+\+|ar)|ld\.lld|lld|llvm-ar|llvm-ranlib|llvm-rc|llvm-windres)\.exe$|^lib(LLVM-\d+|clang-cpp|c\+\+|unwind|winpthread-1)\.dll$"
    Get-ChildItem "$($full.FullName)\bin" -File | Where-Object { $_.Name -match $keep } | Copy-Item -Destination "$dir\bin"
    Copy-Item -Recurse "$($full.FullName)\include" "$dir\include"
    Copy-Item -Recurse "$($full.FullName)\$a-w64-mingw32" "$dir\$a-w64-mingw32"
    $res = Get-ChildItem "$($full.FullName)\lib\clang" -Directory | Select-Object -First 1
    New-Item -ItemType Directory -Force "$dir\lib\clang\$($res.Name)\lib\windows" | Out-Null
    Copy-Item -Recurse "$($res.FullName)\include" "$dir\lib\clang\$($res.Name)\include"
    $rtArch = if ($a -eq "x86_64") { "x86_64" } else { "aarch64" }
    Get-ChildItem "$($res.FullName)\lib\windows" -File | Where-Object { $_.Name -match "-$rtArch\." } | Copy-Item -Destination "$dir\lib\clang\$($res.Name)\lib\windows"
    Copy-Item "$($full.FullName)\LICENSE.TXT" "$dir\LICENSE.TXT"
    Set-Content "$dir\README.txt" @"
LLVM-MinGW $($zip.Name -replace '^llvm-mingw-(\d+)-.*$', '$1'), trimmed for $a by RapidR (tools/release/windows/windows.ps1)
from https://github.com/mstorsjo/llvm-mingw/releases ($($zip.Name)).
Native builds (rapidr build) link with it, through Rust's $a-pc-windows-gnullvm toolchain.
Licences: LLVM (clang, lld, compiler-rt, libunwind, libc++) - LICENSE.TXT (Apache-2.0 WITH LLVM-exception);
mingw-w64 (runtime, headers) - $a-w64-mingw32\share\mingw32\COPYING*. A program built with it carries
the mingw-w64 runtime: ship COPYING.MinGW-w64-runtime.txt's notices with it (RapidR's LICENSES.md section 7).
"@
    Remove-Item -Recurse -Force "$Work\toolchains\unz-$a"
    $mb = (Get-ChildItem $dir -Recurse -File | Measure-Object Length -Sum).Sum / 1MB
    Write-Host ("  toolchain for {0}: {1:N0} MB" -f $a, $mb)
    return $dir
}

if ($Sign) {
    Step "sign the executables ($Sign)"
    $files = foreach ($t in $triples.Values) {
        "$env:CARGO_TARGET_DIR\$t\release\rapidr.exe", "$env:CARGO_TARGET_DIR\$t\release\rapidrw.exe",
        "$env:CARGO_TARGET_DIR\$t\runner\rapidrintr-runner.exe", "$env:CARGO_TARGET_DIR\$t\runner\rapidrintr-runnerw.exe"
    }
    Run signtool @(@("sign", "/n", $Sign, "/fd", "sha256", "/tr", "http://timestamp.digicert.com", "/td", "sha256") + $files)
}

# Every SDK ships both architectures' runners (`rapidr build --interp --target windows-x86_64`).
$runners = foreach ($a in $Arch) { "--runner"; "windows-$a=$env:CARGO_TARGET_DIR\$($triples[$a])\runner" }
$rust = (Select-String -Path "$Work\home\release.toml" -Pattern '^rust = "(.*)"').Matches[0].Groups[1].Value
foreach ($a in $Arch) {
    $t = $triples[$a]; $bin = "$env:CARGO_TARGET_DIR\$t\release"
    $iarch = if ($a -eq "aarch64") { "arm64" } else { "x64" }
    Step "stage and package windows-$a"
    $tc = @()
    if ($Toolchain -eq "gnullvm") { $tc = @("--toolchain", (Trim-Toolchain $a)) }
    Run $python.FullName (@("tools\release\stage.py", "--kind", "sdk", "--os", "windows", "--out", "$Stage\sdk-$a", "--bin", $bin,
        "--home", "$Work\home", "--web", "$Prep\web-runtime", "--ide", "$Prep\rapidr-ide.rrbc") + $runners + $tc)
    Run $python.FullName @("tools\release\stage.py", "--kind", "runtime", "--os", "windows", "--out", "$Stage\runtime-$a",
        "--bin", $bin, "--version", $Version, "--rust", $rust)
    foreach ($kind in "sdk", "runtime") {
        Run $iscc @("/Q", "/DVersion=$Version", "/DKind=$kind", "/DArch=$iarch", "/DStage=$Stage\$kind-$a", "/DOutDir=$Out",
            "tools\release\windows\rapidr.iss")
    }
    Remove-Item -Recurse -Force "$Stage\sdk-$a", "$Stage\runtime-$a", "$Work\toolchains" -ErrorAction SilentlyContinue
}
if ($Sign) {
    Run signtool @(@("sign", "/n", $Sign, "/fd", "sha256", "/tr", "http://timestamp.digicert.com", "/td", "sha256") + (Get-ChildItem "$Out\*-setup.exe").FullName)
}
Remove-Item -Recurse -Force $Stage, "$Work\home"
Get-ChildItem "$Out\*-setup.exe" | ForEach-Object { "{0}  {1:N1} MB  sha256 {2}" -f $_.Name, ($_.Length / 1MB), (Get-FileHash -Algorithm SHA256 $_.FullName).Hash.ToLower() }
exit 0
