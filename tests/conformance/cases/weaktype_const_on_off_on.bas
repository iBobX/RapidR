' RC.EXE (the Windows VM, 2026-11-07): off for a while: only the CONSTs written while it is on.
$OPTION WEAKTYPE ON
CONST A = 1
$OPTION WEAKTYPE OFF
CONST B = 1
$OPTION WEAKTYPE ON
CONST A = 2
PRINT A
PRINT B
