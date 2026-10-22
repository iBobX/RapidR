' A RapidQ program (.bas) without $INCLUDE "RAPIDQ.INC" reads mbYes, clRed
' and crHandPoint as RapidQ does (RC.EXE, confirmed 2026-10-22): undeclared
' variables, 0 — assignable, and with a $ an empty string — the constants
' come only with the include (a RapidR program, .rr, has them built in:
' rr_builtin_constants). The .expected is RC.EXE's output.
PRINT mbYes; " "; clRed; " "; crHandPoint
IF mbYes = 0 THEN PRINT "mbYes is 0 here"
PRINT False; " "; clBtnFace
clRed = 5
PRINT clRed
PRINT "str ["; mbNo$; "]"
