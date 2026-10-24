# Installs the QEMU guest agent in the Windows 7 test VM, so the Mac can run
# commands in it and copy files (utmctl exec / utmctl file push / pull).
# PowerShell 2.0 (Windows 7's own). Run ONCE, inside the VM, from an
# Administrator Command Prompt, with tools/win7/serve.sh running on the Mac:
#
#   powershell -NoProfile -ExecutionPolicy Bypass -Command "iex (New-Object Net.WebClient).DownloadString('http://ADDR:8099/install-agent.ps1')"
#
# (tools/win7/vm.sh stage puts this file, with ADDR filled in, on the share.)
# It downloads virtio-win 0.1.173's VirtIO serial driver for Windows 7 x86 and
# the guest agent MSI from the share (the files are in tools/win7/vm/ ->
# the share's guest/ folder), installs the driver (pnputil), installs the
# agent as a service, and writes everything it did to C:\agent\install.log.
$base = 'http://@ADDR@:8099/guest'
$dir = 'C:\agent'
New-Item -ItemType Directory -Force $dir | Out-Null
$log = "$dir\install.log"
function Say($m) { $line = (Get-Date -Format 'HH:mm:ss') + ' ' + $m; $line; Add-Content $log $line }
Say "start: $([Environment]::OSVersion.VersionString)"
$wc = New-Object Net.WebClient
foreach ($f in 'WdfCoInstaller01009.dll', 'vioser.cat', 'vioser.inf', 'vioser.sys') {
    $wc.DownloadFile("$base/vioserial/w7/x86/$f", "$dir\$f"); Say "downloaded $f"
}
$wc.DownloadFile("$base/guest-agent/qemu-ga-i386.msi", "$dir\qemu-ga-i386.msi"); Say 'downloaded qemu-ga-i386.msi'
Say 'pnputil: the VirtIO serial driver'
pnputil -i -a "$dir\vioser.inf" 2>&1 | ForEach-Object { Say $_ }
Say 'msiexec: the guest agent'
$p = Start-Process msiexec -ArgumentList "/i `"$dir\qemu-ga-i386.msi`" /qn /l*v `"$dir\msi.log`"" -Wait -PassThru
Say "msiexec exit code $($p.ExitCode)"
Start-Sleep 3
Get-Service | Where-Object { $_.Name -like 'QEMU*' } | ForEach-Object { Say "service $($_.Name): $($_.Status)" }
Say 'done'
