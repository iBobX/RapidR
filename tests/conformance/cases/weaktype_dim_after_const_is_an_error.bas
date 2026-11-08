' RC.EXE (the Windows VM, 2026-11-07): ERROR: Identifier A already used, try another name — a DIM
' can't take a CONST's name.
$OPTION WEAKTYPE ON
CONST A = 2
DIM A AS INTEGER
PRINT A
