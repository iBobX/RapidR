#!/bin/bash
# vmexec.sh [-S] [-i IN] LINE — runs LINE with the Windows command interpreter in the VM
# ($RAPIDR_WIN_VM, default "Windows 11 Pro"): `prlctl exec VM --current-user cmd /c "LINE"`, as
# the logged-in user, stdin from IN (default /dev/null: prlctl needs one), retried when Parallels
# answers "Invalid argument". Parallels drops the quoting of the outer command except inside
# `cmd /c "…"`, which is why LINE is passed whole.
#
# -S: without --current-user, so as SYSTEM in Windows' session 0. A call in the user's own session
# opens a console window on their desktop every time (a flash; a script that polls flashes it
# all day) — SYSTEM has no desktop to show one on. SYSTEM sees the user's files by their full path
# (%USERPROFILE% is SYSTEM's own there) and reads \\Mac\Home too, but is not the user: nothing of
# the user's registry (HKEY_USERS\<sid> instead of HKCU) or settings, no programs of theirs to
# start. tools/release/windows-vm.sh works this way.
vm="${RAPIDR_WIN_VM:-Windows 11 Pro}"; in=/dev/null; user=(--current-user)
while :; do
  case "$1" in
    -S) user=(); shift ;;
    -i) in="$2"; shift 2 ;;
    *) break ;;
  esac
done
for try in 1 2 3 4 5; do
  out=$(prlctl exec "$vm" ${user[@]+"${user[@]}"} cmd /c "$1" < "$in" 2>&1); code=$?
  if ! grep -q "PrlJob_Get.*Invalid argument" <<<"$out"; then printf '%s\n' "$out"; exit $code; fi
  sleep 3
done
printf '%s\n' "$out"; exit $code
