#!/usr/bin/env python3
"""The Mac's side of the Windows XP test helper (tools/winxp/helper/START-HERE.vbs).

    python3 -I tools/winxp/xphost.py [DIR]       (default /Volumes/RapidRBuild/winxp-share)

Binds to ONE address: the Mac's address on UTM's host-only bridge (a
bridge1NN with a 192.168.x.x address), never 0.0.0.0, Wi-Fi or Ethernet, so
only the VMs on that network reach it. Python's standard library only.

DIR/files/    what the VM may fetch (read-only, GET /files/<name>)
DIR/queue/    jobs written by tools/winxp/vm.sh (<id>.job), handed out in order
DIR/results/  what the VM sends back (<id>.txt or <id>.bin)

Nothing in the VM can add a job: jobs come only from files on the Mac. A
result is written under a number the Mac handed out, nowhere else.
"""
import http.server
import os
import re
import subprocess
import sys
import time
import urllib.parse

PORT = 8097
MAX_RESULT = 512 * 1024 * 1024


def host_only_address():
    names = subprocess.run(["ifconfig", "-l"], capture_output=True, text=True).stdout.split()
    for br in names:
        if not re.fullmatch(r"bridge1\d\d", br):
            continue
        out = subprocess.run(["ifconfig", br], capture_output=True, text=True).stdout
        m = re.search(r"inet (192\.168\.\d+\.\d+)", out)
        if m:
            return m.group(1), br
    return None, None


ROOT = sys.argv[1] if len(sys.argv) > 1 else "/Volumes/RapidRBuild/winxp-share"
FILES = os.path.join(ROOT, "files")
QUEUE = os.path.join(ROOT, "queue")
RESULTS = os.path.join(ROOT, "results")
for d in (FILES, QUEUE, RESULTS):
    os.makedirs(d, exist_ok=True)
HANDED = set()
LAST_SEEN = os.path.join(ROOT, "last-seen.txt")


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        if not self.path.startswith("/next"):
            sys.stderr.write("%s %s\n" % (time.strftime("%H:%M:%S"), fmt % args))

    def reply(self, code, body=b"", kind="text/plain; charset=utf-8"):
        self.send_response(code)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        if url.path == "/next":
            with open(LAST_SEEN, "w") as f:
                f.write(time.strftime("%Y-%m-%d %H:%M:%S") + " " + url.query + "\n")
            jobs = sorted((n for n in os.listdir(QUEUE) if re.fullmatch(r"\d+\.job", n)), key=lambda n: int(n[:-4]))
            for name in jobs:
                if name in HANDED:
                    continue
                HANDED.add(name)
                with open(os.path.join(QUEUE, name), "rb") as f:
                    body = f.read()
                os.rename(os.path.join(QUEUE, name), os.path.join(QUEUE, name + ".sent"))
                return self.reply(200, body)
            return self.reply(200, b"")
        if url.path.startswith("/files/"):
            rel = urllib.parse.unquote(url.path[len("/files/"):])
            full = os.path.realpath(os.path.join(FILES, rel))
            if not full.startswith(os.path.realpath(FILES) + os.sep) or not os.path.isfile(full):
                return self.reply(404, b"not here")
            with open(full, "rb") as f:
                return self.reply(200, f.read(), "application/octet-stream")
        return self.reply(404, b"not here")

    def do_POST(self):
        url = urllib.parse.urlparse(self.path)
        m = re.fullmatch(r"/done/(\d+)", url.path)
        if not m or m.group(1) + ".job" not in HANDED:
            return self.reply(404, b"no such job")
        n = int(self.headers.get("Content-Length") or 0)
        if n > MAX_RESULT:
            return self.reply(413, b"too big")
        data = self.rfile.read(n)
        kind = urllib.parse.parse_qs(url.query).get("kind", ["text"])[0]
        ext = "bin" if kind == "bin" else "txt"
        tmp = os.path.join(RESULTS, m.group(1) + "." + ext + ".part")
        with open(tmp, "wb") as f:
            f.write(data)
        os.rename(tmp, os.path.join(RESULTS, m.group(1) + "." + ext))
        return self.reply(200, b"ok")


def main():
    addr, br = host_only_address()
    if not addr:
        sys.exit("No host-only bridge with a 192.168.x.x address: the XP VM's network card must be Host Only, and the VM running.")
    print("XP helper host on http://%s:%d/ (bridge %s), folder %s" % (addr, PORT, br, ROOT), flush=True)
    http.server.ThreadingHTTPServer((addr, PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
