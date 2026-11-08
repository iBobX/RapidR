' RC.EXE (the Windows VM, 2026-11-07): a CONST in an IF block at the top of the program counts too.
$OPTION WEAKTYPE ON
CONST A = 1
IF A = 1 THEN
  CONST A = 2
END IF
PRINT A
