' RC.EXE (the Windows VM, 2026-11-07): with $OPTION WEAKTYPE ON a CONST can be written again,
' no error, and the last value wins.
$OPTION WEAKTYPE ON
CONST A = 1
CONST A = 2
PRINT A
