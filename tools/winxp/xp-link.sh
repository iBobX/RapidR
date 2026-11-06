#!/bin/bash
# The Windows XP flavour's linker driver (lane WINXP). rustc calls it in place of the LLVM-MinGW clang
# driver; it puts the XP stand-in objects ($XP_SHIM_OBJS) FIRST in the link, then calls the real clang.
#
# Why first: rustc appends a `-C link-arg` object at the very end, after every `-lkernel32`. The import
# library then defines the Windows-Vista-and-later thunks (e.g. _ReleaseSRWLockExclusive@4) to satisfy
# std's references before the stand-in object is read, and the stand-in's own definitions collide
# ("duplicate symbol"). Read first, the stand-in defines those symbols before any `-lkernel32` is
# scanned, so the import-library members are never pulled, the functions never enter the import table
# (which is what lets plain XP load the program), and there is no collision. Isolated to this build:
# no other target uses this linker.
#
# XP_SHIM_OBJS  the stand-in objects, space-separated (set by tools/winxp/build.sh)
# XP_REAL_CC    the real clang driver (default the LLVM-MinGW i686 one)
set -euo pipefail

REAL="${XP_REAL_CC:-/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt/bin/i686-w64-mingw32-clang}"
# The first argument is the output or an early flag; keep clang's own argument order and simply put the
# stand-in objects at the front of the inputs. clang hands objects to the linker in the order given, so
# the stand-ins are the linker's first inputs.
exec "$REAL" ${XP_SHIM_OBJS:-} "$@"
