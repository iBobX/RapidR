' RC.EXE (the Windows VM, 2026-11-07): the last CONST wins even when it is another type.
CONST A = 1
$OPTION WEAKTYPE ON
CONST A = "x"
PRINT A
