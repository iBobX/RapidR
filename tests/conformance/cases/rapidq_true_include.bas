' True and False with RAPIDQ.INC (RC.EXE, 2026-10-22): the include's
' `CONST True = 1` and `CONST False = 0` — so a comparison's own -1 isn't
' equal to True. (Without the include they are undeclared variables, 0:
' bas_implicit_constant; a RapidR program, .rr, has True = -1 built in:
' rr_builtin_constants.) The .expected is RC.EXE's output.
$INCLUDE "RAPIDQ.INC"
DIM c AS INTEGER
PRINT True; " "; False
c = (1 = 1)
PRINT "c "; c
IF c = True THEN
  PRINT "c = True"
ELSE
  PRINT "c <> True"
END IF
IF True THEN PRINT "IF True runs"
c = True
PRINT "c "; c
