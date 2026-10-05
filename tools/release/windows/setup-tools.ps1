# The Windows release machine's tools, from their official sources, checked:
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File setup-tools.ps1 [-LlvmMingw <release tag>]
#
#   - Python 3.13 and Inno Setup 6: winget (its manifests carry each installer's SHA-256, which
#     winget checks; the files come from python.org and jrsoftware.org), per user
#   - LLVM-MinGW (UCRT) for both hosts, from github.com/mstorsjo/llvm-mingw's releases, checked
#     against the SHA-256 GitHub publishes for each asset: %USERPROFILE%\rapidr-tools\llvm-mingw-*\
#     (this machine's, for building) and the zips (what the installers ship)
#   - Rust's gnullvm toolchain for this machine and the x64 / ARM64 gnullvm targets (rustup)
#
# Idempotent: what is there is kept. Prints where everything is.
param([string]$LlvmMingw = "")
# (Continue: Windows PowerShell turns a native tool's stderr into errors when the output is
# redirected; failures are checked by exit code and thrown, cmdlets that matter say -ErrorAction Stop)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
function Step($m) { Write-Host "== $m" }
$Tools = "$env:USERPROFILE\rapidr-tools"
New-Item -ItemType Directory -Force $Tools | Out-Null

Step "Python 3.13, Inno Setup 6 (winget)"
foreach ($id in "Python.Python.3.13", "JRSoftware.InnoSetup") {
    $listed = winget list --exact --id $id --accept-source-agreements 2>$null | Out-String
    if ($listed -match [regex]::Escape($id)) { Write-Host "  ${id}: installed" ; continue }
    winget install --exact --id $id --scope user --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
    if ($LASTEXITCODE -ne 0) { throw "winget install $id failed ($LASTEXITCODE)" }
}

Step "LLVM-MinGW (github.com/mstorsjo/llvm-mingw)"
$api = if ($LlvmMingw) { "https://api.github.com/repos/mstorsjo/llvm-mingw/releases/tags/$LlvmMingw" } else { "https://api.github.com/repos/mstorsjo/llvm-mingw/releases/latest" }
$rel = Invoke-RestMethod -ErrorAction Stop -Uri $api -Headers @{ "User-Agent" = "rapidr-release" }
Write-Host "  release $($rel.tag_name)"
foreach ($arch in "aarch64", "x86_64") {
    $asset = $rel.assets | Where-Object { $_.name -like "llvm-mingw-*-ucrt-$arch.zip" } | Select-Object -First 1
    if (-not $asset) { throw "no ucrt-$arch zip in $($rel.tag_name)" }
    $zip = "$Tools\$($asset.name)"
    if (-not (Test-Path $zip)) {
        Invoke-WebRequest -ErrorAction Stop -UseBasicParsing -Uri $asset.browser_download_url -OutFile "$zip.part"
        Move-Item "$zip.part" $zip
    }
    $want = ($asset.digest -replace "^sha256:", "").ToLower()
    if (-not $want) { throw "GitHub publishes no digest for $($asset.name)" }
    $have = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
    if ($have -ne $want) { Remove-Item $zip; throw "$($asset.name): SHA-256 $have, GitHub says $want" }
    Write-Host "  $($asset.name): SHA-256 ok ($have)"
}
$hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { "x86_64" }
$hostZip = Get-ChildItem "$Tools\llvm-mingw-$($rel.tag_name)-ucrt-$hostArch.zip"
$hostDir = "$Tools\$($hostZip.BaseName)"
if (-not (Test-Path "$hostDir\bin\clang.exe")) { Expand-Archive -ErrorAction Stop -Path $hostZip.FullName -DestinationPath $Tools -Force }
Write-Host "  this machine's: $hostDir"

Step "Rust: the gnullvm toolchain"
$ver = (rustc --version) -split " " | Select-Object -Index 1
$tc = "$ver-$hostArch-pc-windows-gnullvm"
rustup toolchain install $tc --profile minimal --no-self-update
if ($LASTEXITCODE -ne 0) { throw "rustup toolchain install $tc failed" }
rustup target add --toolchain $tc x86_64-pc-windows-gnullvm aarch64-pc-windows-gnullvm
if ($LASTEXITCODE -ne 0) { throw "rustup target add failed" }

Step "where"
$py = Get-ChildItem "$env:LOCALAPPDATA\Programs\Python\Python313*\python.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
Write-Host "python: $($py.FullName)"
Write-Host "iscc: $iscc"
Write-Host "llvm-mingw: $hostDir ($($rel.tag_name))"
Write-Host "rust: $tc"
exit 0
