#!/bin/bash
# Serves one folder (read-only, plain HTTP GET) to the Windows 7 test VM over
# the VM's HOST-ONLY network, and nowhere else.
#
#   tools/win7/serve.sh [FOLDER] [PORT]       (default /Volumes/RapidRBuild/win7-share, 8099)
#
# It binds to ONE address: the Mac's address on UTM's host-only bridge
# (found with ifconfig: a `bridgeNNN` that has a `vmenetN` member). Not
# 0.0.0.0, never en0 (Wi-Fi/Ethernet), so nothing on the LAN or the internet
# can reach it. No upload, no listing outside the folder. Ctrl+C stops it;
# nothing is installed and no System Settings switch is touched.
#
# In the VM (Internet Explorer is fine): http://<printed address>:<port>/
# or PowerShell 2.0:  (New-Object Net.WebClient).DownloadFile('http://ADDR:PORT/rapidr/rapidr.exe','C:\rapidr\rapidr.exe')
set -euo pipefail

DIR="${1:-/Volumes/RapidRBuild/win7-share}"
PORT="${2:-8099}"
[ -d "$DIR" ] || { echo "no folder $DIR" >&2; exit 1; }

# the host-only bridge: has a vmenet member and a private 192.168.x.x address
# (Parallels' own bridges are 10.x; Wi-Fi/Ethernet is never a bridge100+)
ADDR=""
for br in $(ifconfig -l | tr ' ' '\n' | grep -E '^bridge1[0-9][0-9]$'); do
    a="$(ifconfig "$br" | awk '/inet /{print $2; exit}')"
    case "$a" in 192.168.*) ADDR="$a"; BR="$br"; break ;; esac
done
[ -n "$ADDR" ] || { echo "No host-only bridge with a 192.168.x.x address: start the Windows 7 VM first (its network card must be Host Only)." >&2; exit 1; }
echo "Serving $DIR on http://$ADDR:$PORT/ (host-only bridge $BR). Ctrl+C to stop."
exec python3 -I -m http.server "$PORT" --bind "$ADDR" --directory "$DIR"
