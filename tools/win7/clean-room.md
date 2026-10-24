# Windows 7 build: the Windows 8-only functions (clean-room record)

Lane WIN7-ALPHA-2, 2026-10-23. Why: the lawyer review
(`docs/legal/review-2026-10-23-win7-alpha.md`, addendum, item 1) found that
mingw-w64's delay-load helper (`delayimp.c`, with `delay-f.c` and `delay-n.c`)
says it is derived from Microsoft's `delayhlp.cpp`, and that Microsoft's side
of that claim could not be checked. The rule here is to replace what is not
clearly safe. So the Windows 7 build no longer delay-loads anything and no
delay-load helper, from mingw-w64 or anyone else, is linked.

## What was written

`crates/rapidr-cli/src/win7.rs`, by this lane's agent, in one pass:

- six small functions of RapidR's own, one for each function Windows 7 lacks
  (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`,
  `CoIncrementMTAUsage`, and `CoCreateFreeThreadedMarshaler` / `CoTaskMemFree`,
  which Windows 7 has in `ole32.dll` and the `windows` crates import from
  `combase.dll`);
- six `__imp_<Function>` pointer slots (the symbols the compiled callers load
  their target from), defined in RapidR's object so the linker never needs the
  import library's entry, and the DLL never appears in the program's import table;
- on the first call each function looks the real one up with `GetModuleHandleW`
  / `LoadLibraryW` (Windows' own folder only) and `GetProcAddress`, remembers the
  answer, and calls it; where Windows does not have it, the function answers
  "not available" in its own kind of answer (the stand-ins that were already
  RapidR's own in the first version of `win7.rs`).

There is no parsing of the delay-load tables, no thunk, no `__delayLoadHelper2`:
the delay-load mechanism is not used at all.

## Sources consulted

| Source | For |
|---|---|
| Microsoft Learn, the documentation pages of `GetModuleHandleW`, `LoadLibraryW`, `GetProcAddress`, `GetSystemDirectoryW` | their signatures and behaviour (public API documentation) |
| Microsoft Learn, the documentation pages of the six functions | their signatures and which DLL has them |
| Microsoft's PE/COFF specification, the import library naming (`__imp_` symbols) | what the callers reference |
| RapidR's own code (`crates/rapidr-cli/src/win7.rs` of the first alpha, `build.rs`) and the output of `llvm-nm` / `llvm-readobj --coff-imports` on RapidR's own program | the symbol names the callers use (`__imp_RoGetActivationFactory`, undecorated, from the `windows-link` crate's `import_name_type = "undecorated"`) |

**Not read, not opened, not searched, not copied from:** mingw-w64's
`delayimp.c`, `delay-f.c`, `delay-n.c` or `misc/delayimp.*`; Microsoft's
`delayhlp.cpp` or `delayimp.h`; any other delay-load helper implementation
(in the mingw-w64 source tree at `/Volumes/RapidRBuild/wt-win7-alpha/mingw-src`
or anywhere on the internet). From the mingw-w64 tree only `AUTHORS` was read
and copied (into `licenses\mingw-w64\`), as the lawyer review asked. The linker
map of `tools/win7/link_audit.sh` reads the headers of the files that ARE
linked; after this change none of the three delay-load files is linked.

## Checked after

- `tools/win7/link_audit.sh`: the toolchain objects in `rapidr.exe` went from
  106 to 103 (`libmingwex` 20 to 17: `delayimp`, `delay-f`, `delay-n` are gone);
  0 flagged.
- `tools/win7/imports.py rapidr.exe --against <the .exe that ran on Windows 7 RTM>`:
  no delay-loaded section; no `combase.dll` and no `api-ms-win-core-*` anywhere;
  the only new import is `ole32.dll`'s nine plain functions (`CoCreateGuid`,
  `CoCreateInstance`, `CoInitializeEx`, `CoUninitialize`, `OleInitialize`,
  `PropVariantClear`, `PropVariantCopy`, `RegisterDragDrop`, `RevokeDragDrop`),
  which were delay-loaded before and are in Windows 7 RTM's `ole32.dll`.
- `rapidr.exe __win7_check` prints where each of the six was found (and calls
  the stand-in of one that is not there). On Windows 11: all six native. A
  throwaway build whose lookups were pointed at a DLL that does not exist: all
  six stand-ins, each called once, no crash.
