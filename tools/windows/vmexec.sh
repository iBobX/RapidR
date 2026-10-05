#!/bin/bash
# vmexec.sh [-i IN] LINE — runs LINE with the Windows command interpreter in the VM
# ($RAPIDR_WIN_VM, default "Windows 11 Pro") as the logged-in user:
# `prlctl exec VM --current-user cmd /c "LINE"`, stdin from IN (default /dev/null: prlctl needs
# one), retried when Parallels answers "Invalid argument". Parallels drops the quoting of the outer
# command except inside `cmd /c "…"`, which is why LINE is passed whole.
vm="${RAPIDR_WIN_VM:-Windows 11 Pro}"; in=/dev/null
if [ "$1" = "-i" ]; then in="$2"; shift 2; fi
for try in 1 2 3 4 5; do
  out=$(prlctl exec "$vm" --current-user cmd /c "$1" < "$in" 2>&1); code=$?
  if ! grep -q "PrlJob_Get.*Invalid argument" <<<"$out"; then printf '%s\n' "$out"; exit $code; fi
  sleep 3
done
printf '%s\n' "$out"; exit $code
