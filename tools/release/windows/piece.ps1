# A file in pieces of base64 (each with its SHA-256), for tools/release/windows-vm.sh
# fetch: prlctl exec loses output over ~1 MB.
#   -Count: how many pieces; -Index n: piece n and "sha256 <hex>"; -Sum: the file's SHA-256
param([Parameter(Mandatory = $true)][string]$File, [int]$Index = -1, [switch]$Count, [switch]$Sum)
$size = 400000
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
[Convert]::ToBase64String($buf, [Base64FormattingOptions]::InsertLineBreaks)
$sha = [Security.Cryptography.SHA256]::Create()
"sha256 " + (($sha.ComputeHash($buf) | ForEach-Object { $_.ToString("x2") }) -join "")
exit 0
