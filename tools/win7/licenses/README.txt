Licence texts for the pieces of the C/C++ toolchain inside rapidr.exe
=====================================================================

rapidr.exe for Windows 7 is built with the LLVM-MinGW toolchain (version
20261020, "msvcrt" flavour, which uses the C runtime Windows 7 has:
msvcrt.dll). The linker copies a few small pieces of the toolchain's own
runtime into the program. This folder has their licence texts. RapidR's own
licence (MIT) and the Rust libraries inside it are in LICENSE, NOTICE,
LEGAL.md, THIRD_PARTY_NOTICES.md and LICENSES.md, one folder up.

mingw-w64\   The mingw-w64 C runtime (start-up code, a few math and string
             functions, the delay-loading helper). Source of these files:
             the mingw-w64 project's own source repository,
             https://sourceforge.net/p/mingw-w64/mingw-w64/ , at commit
             0d6dfbc348dfad714761c0a7151615767f96bcb5 (the commit LLVM-MinGW
             20261020 is built from; GitHub mirrors the same commit).
               COPYING                          the Zope Public License 2.1 that
                                                covers the runtime
               COPYING.MinGW-w64.txt            the project's licence summary
               COPYING.MinGW-w64-runtime.txt    the notices for what is linked
                                                into programs (public domain, ZPL,
                                                David M. Gay's gdtoa permission
                                                notice, and others for parts this
                                                program does not contain)
               DISCLAIMER, DISCLAIMER.PD        the disclaimers the files refer to
             Most of the linked files are public domain or ZPL-2.1. The
             floating-point text conversion (gdtoa) is David M. Gay's, under
             the permission notice in COPYING.MinGW-w64-runtime.txt.

llvm\        LLVM's licence (Apache License 2.0 with LLVM Exceptions), which
             covers LLVM's libunwind (stack unwinding for errors and panics)
             that is linked in. The exception means no notice is needed for
             code embedded in a program by compiling; it is here anyway.

Not linked: nothing under the GPL or the LGPL, no libgcc, no winpthreads, no
compiler-rt builtins (Rust's own compiler_builtins provides those). The
Windows system DLLs the program uses (kernel32, user32, msvcrt, ...) are part
of Windows and are not in this download.
