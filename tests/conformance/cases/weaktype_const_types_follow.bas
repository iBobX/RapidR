' RC.EXE (the Windows VM, 2026-11-07): a name can go from a number to text to a fraction; each line
' reads the value its CONST gave it.
$OPTION WEAKTYPE ON
CONST A = 1
CONST A = "x"
PRINT A
CONST A = 2.5
PRINT A
PRINT A + 1
