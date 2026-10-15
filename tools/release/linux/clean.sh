#!/bin/bash
# Removes what RapidR's release and smoke scripts leave in the Linux VM (linux-vm.sh clean):
# the release work folder (builds, cargo target, packages, logs), the build cache, the smoke
# tests' temporary folders and the test apps they built. A DRY RUN (lists, with sizes) unless
# --delete.
#
#   bash clean.sh [delete] [keep-out]        (also --delete, --keep-out; the VM's exec drops
#                                             dash options, which is why linux-vm.sh says delete)
#
# Kept, always: an installed rapidr package, rapidr-tools, the user's own Rust (~/.cargo,
# ~/.rustup: setup-tools.sh), their files and apps. keep-out keeps the finished packages
# (~/rapidr-release/dist/out). The release job and smoke test still running there are stopped
# first (only processes working inside RapidR's own folders); nothing else is closed.
# The user's other RapidR folders (rapidr, bpkg, rr-font … earlier lanes' scratch) are only
# listed, never removed: someone may still be using them.
# Runs as the user or as root (linux-vm.sh: root, which works while nobody is logged in or the screen is locked at the
# VM's desktop): root cleans the home of the VM's first normal user.
delete=0; keep_out=0
for a in "$@"; do
    case "$a" in delete|--delete) delete=1 ;; keep-out|--keep-out) keep_out=1 ;; *) echo "usage: $0 [delete] [keep-out]"; exit 2 ;; esac
done
if [ "$(id -u)" = 0 ]; then
    HOME="$(getent passwd | awk -F: '$3 >= 1000 && $3 < 60000 && $6 ~ /^\/home\// { print $6; exit }')"
    [ -d "$HOME" ] || { echo "no user's home found"; exit 1; }
fi
B="$HOME/rapidr-release"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/rapidr"
free() { df -BM --output=avail / | tail -1 | tr -dc 0-9; }
echo "free on /: $(( $(free) / 1024 )) GB"

# What is ours (a path per line). Names are matched only in the folders named here.
list=()
if [ -d "$B" ]; then
    for p in "$B"/* "$B"/.[!.]*; do
        [ -e "$p" ] || continue
        if [ "$keep_out" = 1 ] && [ "$p" = "$B/dist" ]; then
            # (dist/out kept, the rest of dist — the prepared source, the web interpreter — goes)
            for q in "$p"/* "$p"/.[!.]*; do [ -e "$q" ] && [ "$q" != "$p/out" ] && list+=("$q"); done
            continue
        fi
        list+=("$p")
    done
fi
[ -d "$CACHE/build" ] && list+=("$CACHE/build")
# (not this script's own copy, nor /tmp/rapidr-vm.sh: ubuntu-vm.sh's transport, which removes both)
for p in /tmp/rapidr-smoke.* /tmp/rapidr-smoke-* /tmp/rapidr-release-* /tmp/rapidr-fetch.* /var/tmp/rapidr-*; do
    [ -e "$p" ] && [ "$p" != "$0" ] && list+=("$p")
done
# (test apps the smoke tests built where RapidR is installed — Documents/RapidR/Builds/<name>:
# only the names they use; a user's own app there is never matched)
for n in hello hello_form gui native list_items arithmetic arrays control_flow functions gosub_goto data_read newer downloaded script; do
    [ -e "$HOME/Documents/RapidR/Builds/$n" ] && list+=("$HOME/Documents/RapidR/Builds/$n")
done
total=0; lines=""
for p in "${list[@]}"; do
    mb="$(du -sm -- "$p" 2>/dev/null | cut -f1)"; mb="${mb:-0}"
    total=$((total + mb))
    lines+="$(printf '%10s MB  %s' "$mb" "$p")"$'\n'
done
printf '%s' "$lines" | sort -rn
printf '%10s MB  total (%s items)\n' "$total" "${#list[@]}"

if [ "$delete" = 0 ]; then
    echo "-- other RapidR folders in $HOME (earlier lanes' scratch; not removed here)"
    for p in "$HOME"/rapidr* "$HOME"/rr-* "$HOME"/rq* "$HOME"/csys* "$HOME"/cdx* "$HOME"/bpkg* "$HOME"/studio-* "$HOME"/corpus-*; do
        [ -e "$p" ] || continue
        case "$p" in "$HOME/rapidr-tools"|"$B") continue ;; esac
        printf '%10s MB  %s  %s\n' "$(du -sm "$p" 2>/dev/null | cut -f1)" "$(date -r "$p" +%Y-%m-%d)" "$p"
    done | sort -rn
    echo "(a dry run: delete removes the items above)"
    exit 0
fi

# Stop what is still running in these folders (the release job, the smoke test and the tools
# they started): by where the process works, not by name. Our own process chain is spared.
spare=" $$ $PPID "
p=$PPID; while [ "$p" -gt 1 ] 2>/dev/null; do p="$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ')"; spare="$spare$p "; done
for d in /proc/[0-9]*; do
    pid="${d#/proc/}"
    case "$spare" in *" $pid "*) continue ;; esac
    cwd="$(readlink "$d/cwd" 2>/dev/null)"; cmd="$(tr '\0' ' ' < "$d/cmdline" 2>/dev/null)"
    case "$cwd/" in "$B"/*|/tmp/rapidr-smoke.*|/tmp/rapidr-smoke-*) kill "$pid" 2>/dev/null && echo "  stopped $pid ${cmd:0:80}"; continue ;; esac
    case "$cmd" in *"$B/"*|*/tmp/rapidr-smoke.*) kill "$pid" 2>/dev/null && echo "  stopped $pid ${cmd:0:80}" ;; esac
done
sleep 2
for p in "${list[@]}"; do
    rm -rf -- "$p"
    if [ -e "$p" ]; then echo "  could not remove: $p"; else echo "  removed: $p"; fi
done
# (the work folder itself, once nothing is left in it)
rmdir "$B/dist/out" "$B/dist" "$B" 2>/dev/null
[ -d "$CACHE" ] && rmdir "$CACHE/build" "$CACHE" 2>/dev/null
echo "free on /: $(( $(free) / 1024 )) GB"
exit 0
