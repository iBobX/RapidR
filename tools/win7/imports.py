#!/usr/bin/env python3
"""What a Windows .exe imports, and what of it Windows 7 does not have.

    tools/win7/imports.py path/to/rapidr.exe [llvm-readobj] [--against older.exe]

Prints the DLLs it needs at start-up (static imports), the ones it loads on
first use (delay imports: win7.rs answers for the missing ones), and flags
what Windows 7 / 8 lack. The flag list is a name check of the functions that
came up in RapidR's crates, from Microsoft's "minimum supported client"
column: it can miss one. The proof is starting the .exe on Windows 7.
Exit status 1 if a static import is flagged.

--against older.exe (an .exe that is known to start on Windows 7): also lists
every function this one imports and the older one did not, by name: the new
ones are the only places a new problem can come from.
"""
import re
import subprocess
import sys

READOBJ = "/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt/bin/llvm-readobj"

# DLLs Windows 7 does not have (Windows 8 and later)
NO_DLL = ("combase.dll", "api-ms-win-core-winrt-", "api-ms-win-core-path-", "api-ms-win-core-synch-l1-2", "api-ms-win-core-libraryloader-l1-2", "api-ms-win-core-processthreads-l1-1-3", "bcryptprimitives.dll", "ucrtbase.dll", "api-ms-win-crt-", "vcruntime", "msvcp", "d3d12.dll", "shcore.dll")
# functions Windows 7 does not have, in DLLs it does have
NO_FN = {
    "GetSystemTimePreciseAsFileTime", "WaitOnAddress", "WakeByAddressSingle", "WakeByAddressAll",
    "SetThreadDescription", "GetThreadDescription", "CreatePseudoConsole", "ClosePseudoConsole",
    "ResizePseudoConsole", "GetDpiForWindow", "GetDpiForSystem", "SetProcessDpiAwarenessContext",
    "GetSystemMetricsForDpi", "AdjustWindowRectExForDpi", "EnableNonClientDpiScaling",
    "GetOverlappedResultEx", "CoIncrementMTAUsage", "PathCchStripPrefix", "PathCchCombine",
    "RoGetActivationFactory", "RoOriginateErrorW", "GetCurrentPackageFullName", "SetProcessMitigationPolicy",
    "VirtualAlloc2", "MapViewOfFile3", "GetSystemCpuSetInformation", "SetThreadInformation",
    "QueryInterruptTime", "WaitForSingleObjectEx2", "InitializeSynchronizationBarrier",
    "GetMachineTypeAttributes",
}


def read_table(exe, readobj):
    text = subprocess.run([readobj, "--coff-imports", exe], capture_output=True, text=True, check=True).stdout
    kind = cur = None
    table = {}
    for line in text.splitlines():
        m = re.match(r"^(Import|DelayImport) \{", line)
        if m:
            kind = m.group(1)
            continue
        m = re.match(r"\s+Name: (.+\.dll)\s*$", line, re.I)
        if m:
            cur = (kind, m.group(1).lower())
            table.setdefault(cur, set())
            continue
        m = re.match(r"\s+Symbol: (.+?) \(\d+\)\s*$", line)
        if m and cur:
            table[cur].add(m.group(1))
    return table


def main():
    args = sys.argv[1:]
    against = None
    if "--against" in args:
        i = args.index("--against")
        against = args[i + 1]
        del args[i : i + 2]
    exe = args[0]
    readobj = args[1] if len(args) > 1 else READOBJ
    table = read_table(exe, readobj)
    bad_static = 0
    for label, kind in (("static (needed to start)", "Import"), ("delay-loaded (on first use)", "DelayImport")):
        print(f"== {label}")
        for (k, dll), fns in sorted(table.items()):
            if k != kind:
                continue
            flags = []
            if dll.startswith(NO_DLL):
                flags.append("NOT ON WINDOWS 7")
            bad = sorted(f for f in fns if f in NO_FN)
            if bad:
                flags.append("missing on Windows 7: " + ", ".join(bad))
            print(f"  {dll:42} {len(fns):4} functions  {'; '.join(flags)}")
            if flags and kind == "Import":
                bad_static += 1
    if against:
        old = read_table(against, readobj)
        print(f"== new since {against}")
        new = 0
        for (k, dll), fns in sorted(table.items()):
            added = sorted(fns - old.get((k, dll), set()))
            if added:
                new += len(added)
                print(f"  {k.lower()} {dll}: {', '.join(added)}")
        print(f"  {new} new functions" if new else "  nothing new")
    sys.exit(1 if bad_static else 0)


main()
