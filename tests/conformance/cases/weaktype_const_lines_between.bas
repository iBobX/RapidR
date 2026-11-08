' RC.EXE (the Windows VM, 2026-11-07): a line between two CONSTs reads the value of the one before it.
$OPTION WEAKTYPE ON
CONST A = 1
PRINT A
CONST A = 2
PRINT A
CONST A = 3
PRINT A
