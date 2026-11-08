' RC.EXE (the Windows VM, 2026-11-07): Public Const and Private Const define it again too, in any
' capitals (a and A are one name).
$option weaktype on
CONST A = 1
PUBLIC CONST A = 3
PRIVATE CONST a = 4
PRINT A
