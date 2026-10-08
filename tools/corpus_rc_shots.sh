#!/bin/bash
# RapidQ's windows of its own example programs, beside RapidR's
# (tools/corpus_run.py): each GUI program built by RC.EXE in the Parallels
# Windows VM and run there, its windows captured at 1× by
# tools/windows/rc_shots.ps1, the PNGs saved as
# tests/conformance/.work/corpus-run/rc-shots/<name>-<n>.png, the programs
# run listed in its programs.txt (the corpus and
# what it shows stay on this machine: never committed).
#
#   tools/corpus_rc_shots.sh [filter …]     (parts of the programs' paths)
#
# Programs come from the last sweep's report (tests/conformance/.work/
# corpus-run/report.json, or CORPUS_RUN_JSON): the GUI ones that ran, not the
# ones that use the network, print or start other programs (rc_shots.ps1
# also never runs one that prints or touches the registry). Needs the VM
# (RAPIDR_VM, default "Windows 11 Pro") and RapidQ's folder (RAPIDQ_DIR,
# default ~/Downloads/Rapidq); the repository must be under your home folder.
set -u
root=$(cd "$(dirname "$0")/.." && pwd -P)
vm=${RAPIDR_VM:-Windows 11 Pro}
rapidq=$(cd "${RAPIDQ_DIR:-$HOME/Downloads/Rapidq}" && pwd -P)
home=$(cd "$HOME" && pwd -P)
json=${CORPUS_RUN_JSON:-$root/tests/conformance/.work/corpus-run/report.json}
work="$root/tests/conformance/.work/corpus-run/rc-stage"
out="$root/tests/conformance/.work/corpus-run/rc-shots"
mkdir -p "$out"
unc() { printf '\\\\Mac\\Home\\%s' "$(printf '%s' "${1#"$home"/}" | tr '/' '\\')"; }
state=$(prlctl list -a -o status,name | awk -v vm="$vm" '{ s = $1; $1 = ""; sub(/^ /, ""); if ($0 == vm) print s }')
case "$state" in
  paused | suspended) prlctl resume "$vm" > /dev/null || exit 1 ;;
  running) ;;
  *) echo "VM \"$vm\": ${state:-not found}" >&2; exit 1 ;;
esac
python3 - "$json" "$rapidq/examples" "$work" "$@" <<'PY' > "$work.list" || exit 1
import json, os, re, shutil, sys
report, examples, work, filters = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
shutil.rmtree(work, ignore_errors=True)
for e in json.load(open(report)):
    if e.get("skipped") or e.get("kind") != "gui":
        continue
    rel = e["program"]
    if filters and not any(f.lower() in rel.lower() for f in filters):
        continue
    name = re.sub(r"[^A-Za-z0-9]+", "_", os.path.splitext(rel)[0]).strip("_")
    src = os.path.join(examples, rel)
    d = os.path.join(work, name)
    shutil.copytree(os.path.dirname(src), d, ignore=shutil.ignore_patterns("*.exe", "*.EXE", "*.zip", "*.ZIP", "*.rar"))
    # (one program per folder, named as the report names it)
    os.replace(os.path.join(d, os.path.basename(src)), os.path.join(d, name + ".bas"))
    for f in os.listdir(d):
        if f.lower().endswith(".bas") and f != name + ".bas" and not f.lower().endswith(".inc"):
            # (the folder's other programs: not built here — but kept when
            # this one includes them)
            text = open(os.path.join(d, name + ".bas"), encoding="latin-1").read()
            if f.lower() not in text.lower():
                os.remove(os.path.join(d, f))
    print(name)
PY
while IFS= read -r name; do
  log=$(mktemp)
  prlctl exec "$vm" --current-user powershell -NoProfile -ExecutionPolicy Bypass -File "$(unc "$root/tools/windows/rc_shots.ps1")" \
    -Programs "$(unc "$work/$name")" -RapidQ "$(unc "$rapidq")" -Sub corpus -Only "$name" -Scales 1 -Wait 2000 < /dev/null | tr -d '\r' > "$log"
  python3 - "$log" "$out" "$name" <<'PY'
import base64, sys
log, out, prog = sys.argv[1], sys.argv[2], sys.argv[3]
shot, saved = None, 0
for line in open(log, encoding="utf-8", errors="replace"):
    line = line.rstrip("\n")
    if line.startswith("-- shot "):
        f = line.split()
        shot = f"{f[2]}-{f[4]}.png"
    elif line.startswith("-- png64 ") and shot:
        open(f"{out}/{shot}", "wb").write(base64.b64decode(line[9:]))
        saved += 1
        shot = None
    elif line.startswith("-- skip"):
        print(prog, line)
print(f"{prog}: {saved} window(s)")
PY
  rm -f "$log"
  echo "$name" >> "$out/programs.txt"
done < "$work.list"
# (the programs RC.EXE ran here: tools/corpus_run.py compares their windows)
sort -u -o "$out/programs.txt" "$out/programs.txt"
