' RC.EXE (the Windows VM, 2026-11-07): ERROR: CONST variable A already defined — the option was on
' for the first CONST, not for the second.
$OPTION WEAKTYPE ON
CONST A = 1
$OPTION WEAKTYPE OFF
CONST A = 1
PRINT A
