' RC.EXE (the Windows VM, 2026-11-07): a CONST can take the name of a DIM'd variable: from there it is
' the CONST.
$OPTION WEAKTYPE ON
DIM A AS INTEGER
A = 5
PRINT A
CONST A = 2
PRINT A
