' RC.EXE (the Windows VM, 2026-11-07): the option counts where the second CONST is written, not
' where the first was.
CONST A = 1
$OPTION WEAKTYPE ON
CONST A = 2
PRINT A
