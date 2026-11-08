' RC.EXE (the Windows VM, 2026-11-07): ERROR: CONST variable A already defined — an include file
' without the option can't write a name another has (Public Const too).
$INCLUDE "include_files/weaktype_a.inc"
$INCLUDE "include_files/weaktype_b.inc"
PRINT A
