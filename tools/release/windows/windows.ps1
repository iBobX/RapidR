# RapidR's Windows installers, made in the Windows VM (or any Windows 11):
# the SDK and the Runtime for x64 and ARM64 (Inno Setup; per user, no admin).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File windows.ps1 -Prep <dist\<ver>\prep>
#       [-Arch x86_64,aarch64] [-Toolchain gnullvm|msvc] [-Sign <certificate subject>]
#       [-Work %USERPROFILE%\rapidr-release] [-Publish <folder to copy the installers to>]
#
#   -Prep       prepare.sh's output (the Mac's dist\<ver>\prep, e.g. through \\Mac\Home: only read)
#   -Toolchain  gnullvm (default, recommended): LLVM-MinGW — open source, its output carries no
#               obligations; llvm-mingw's bin\ on PATH (https://github.com/mstorsjo/llvm-mingw).
#               msvc: Visual Studio Build Tools (Microsoft's licence) with the x64 and ARM64 tools.
#   -Sign       a code-signing certificate in the user's store (signtool, Windows SDK). Without
#               it the installers are unsigned: SmartScreen warns ("More info > Run anyway").
#
# Needs: Rust (rustup), Python 3.11+ (python.org; the release scripts), Inno Setup 6 (ISCC.exe),
# tar.exe (Windows' own). Builds in -Work (~8 GB: two targets); the installers go to -Work\out.
param(
    [Parameter(Mandatory = $true)][string]$Prep,
    [string[]]$Arch = @("x86_64", "aarch64"),
    [ValidateSet("gnullvm", "msvc")][string]$Toolchain = "gnullvm",
    [string]$Sign = "",
    [string]$Work = "$env:USERPROFILE\rapidr-release",
    [string]$Publish = ""
)
# (Continue: Windows PowerShell turns a native tool's stderr into errors when the output is
# redirected; failures are checked by exit code and thrown, cmdlets that matter say -ErrorAction Stop)
$ErrorActionPreference = "Continue"
function Step($m) { Write-Host "== $m" }
function Run($exe, [string[]]$a) {
    & $exe @a
    if ($LASTEXITCODE -ne 0) { throw "$exe $($a -join ' ') failed ($LASTEXITCODE)" }
}

# (Windows PowerShell 5.1: no `??`)
$python = Get-Command py -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command python -ErrorAction SilentlyContinue }
if (-not $python) { throw "Python 3.11+ is needed (https://www.python.org/downloads/windows/)" }
$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 is needed (https://jrsoftware.org/isdl.php)" }

$Src = "$Work\src"; $Out = "$Work\out"; $Stage = "$Work\stage"
Remove-Item -Recurse -Force $Src, $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Src, $Out, $Stage | Out-Null

Step "the release's source"
# (-m: the files' times are now, so cargo rebuilds what changed)
Run tar.exe @("-m", "-x", "-C", $Src, "-f", "$Prep\src.tar")
Set-Location $Src
$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$env:CARGO_TARGET_DIR = "$Work\target"
$env:RAPIDR_PRINT_TO = "$Work\prints"; $env:RAPIDR_REGISTRY = "$Work\registry.reg"

Step "the home: the runtime's sources, their crates vendored"
Run $python.Source @("tools\release\home.py", "--os", "windows", "--src", $Src, "--out", "$Work\home")

$triples = @{}
foreach ($a in $Arch) {
    $t = "$a-pc-windows-$Toolchain"
    $triples[$a] = $t
    Step "build $t"
    Run rustup @("target", "add", $t)
    if ($Toolchain -eq "gnullvm") {
        # llvm-mingw's compiler drivers link, and compile the C the crates carry (SQLite)
        $cc = "$a-w64-mingw32-clang"
        if (-not (Get-Command $cc -ErrorAction SilentlyContinue)) { throw "$cc not on PATH: put llvm-mingw's bin\ on PATH" }
        $up = $t.ToUpper().Replace("-", "_")
        Set-Item "env:CARGO_TARGET_${up}_LINKER" $cc
        Set-Item "env:CC_$($t.Replace('-', '_'))" $cc
        Set-Item "env:AR_$($t.Replace('-', '_'))" "llvm-ar"
    } else {
        # Rust links the MSVC runtime in (no VC++ redistributable needed by users)
        $env:RUSTFLAGS = "-C target-feature=+crt-static"
    }
    Run cargo @("build", "-q", "--locked", "--release", "--target", $t, "-p", "rapidr-cli", "-p", "rapidr-launcher")
    Run cargo @("build", "-q", "--locked", "--profile", "runner", "--target", $t, "-p", "rapidr-runner-stub")
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
    Run $python.Source (@("tools\release\stage.py", "--kind", "sdk", "--os", "windows", "--out", "$Stage\sdk-$a", "--bin", $bin,
        "--home", "$Work\home", "--web", "$Prep\web-runtime", "--ide", "$Prep\rapidr-ide.rrbc") + $runners)
    Run $python.Source @("tools\release\stage.py", "--kind", "runtime", "--os", "windows", "--out", "$Stage\runtime-$a",
        "--bin", $bin, "--version", $Version, "--rust", $rust)
    foreach ($kind in "sdk", "runtime") {
        Run $iscc @("/Q", "/DVersion=$Version", "/DKind=$kind", "/DArch=$iarch", "/DStage=$Stage\$kind-$a", "/DOutDir=$Out",
            "tools\release\windows\rapidr.iss")
    }
}
if ($Sign) {
    Run signtool @(@("sign", "/n", $Sign, "/fd", "sha256", "/tr", "http://timestamp.digicert.com", "/td", "sha256") + (Get-ChildItem "$Out\*-setup.exe").FullName)
}
Remove-Item -Recurse -Force $Stage, "$Work\home"
Get-ChildItem $Out | ForEach-Object { "{0}  {1:N0} MB" -f $_.Name, ($_.Length / 1MB) }
if ($Publish) {
    Copy-Item "$Out\*-setup.exe" $Publish
    Write-Host "copied to $Publish"
}
exit 0
