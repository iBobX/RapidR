# A file in pieces of base64 (each with its SHA-256), for tools/release/windows-vm.sh fetch.
#   -Count: how many pieces; -Index n: piece n, then a line "sha256 <hex>"; -Sum: the file's SHA-256
# Pieces of 6 MB (8 MB of text): a call as SYSTEM (windows-vm.sh makes none in the user's session)
# returns 16 MB intact, measured; the old pieces of 400 KB were for the user's session, which
# lost output over ~1 MB. Each piece is checked by the caller, which asks again when it differs.
param([Parameter(Mandatory = $true)][string]$File, [int]$Index = -1, [switch]$Count, [switch]$Sum)
$size = 6000000
$len = (Get-Item $File).Length
if ($Count) { [math]::Ceiling($len / $size); exit 0 }
if ($Sum) { (Get-FileHash -Algorithm SHA256 $File).Hash.ToLower(); exit 0 }
$fs = [IO.File]::OpenRead($File)
try {
    $fs.Position = [long]$Index * $size
    $buf = New-Object byte[] ([math]::Min($size, $len - $fs.Position))
    $read = 0
    while ($read -lt $buf.Length) { $read += $fs.Read($buf, $read, $buf.Length - $read) }
} finally { $fs.Close() }
$sha = [Security.Cryptography.SHA256]::Create()
$hash = ($sha.ComputeHash($buf) | ForEach-Object { $_.ToString("x2") }) -join ""
# (one write: the base64 on a line of its own, then the checksum's)
[Console]::Out.Write([Convert]::ToBase64String($buf) + "`nsha256 " + $hash + "`n")
exit 0
