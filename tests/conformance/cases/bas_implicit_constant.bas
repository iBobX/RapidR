' A RapidQ program (.bas) without $INCLUDE "RAPIDQ.INC" reads mbYes, clRed
' and crHandPoint as RapidQ does (RC.EXE, confirmed 2026-10-22): undeclared
' variables, 0 — assignable, and with a $ an empty string — the constants
' come only with the include (a RapidR program, .rr, has them built in:
' rr_builtin_constants). True and False are RAPIDQ.INC's names too: 0 here,
' so `IF True` doesn't run its THEN (rapidq_true_include: 1 and 0 with the
' include). The .expected is RC.EXE's output.
PRINT mbYes; " "; clRed; " "; crHandPoint
IF mbYes = 0 THEN PRINT "mbYes is 0 here"
PRINT True; " "; False; " "; clBtnFace
IF True THEN
  PRINT "True is true"
ELSE
  PRINT "True is 0 here"
END IF
True = 3
PRINT "True now "; True
clRed = 5
PRINT clRed
PRINT "str ["; mbNo$; "]"
SUB S
  PRINT "in S "; False
END SUB
S
