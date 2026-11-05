# Prints the SHA-256 of a file in the VM, or "missing" (tools/release/windows-vm.sh hash).
#   filehash.ps1 -File <path>
param([Parameter(Mandatory = $true)][string]$File)
if (Test-Path -LiteralPath $File) { (Get-FileHash -Algorithm SHA256 -LiteralPath $File).Hash.ToLower() } else { "missing" }
exit 0
