' A RapidQ program (.bas) without $INCLUDE "RAPIDQ.INC" reads mbYes, clRed
' and crHandPoint as RapidQ does (RC.EXE): undeclared variables, 0 — the
' constants come only with the include (a RapidR program, .rr, has them
' built in: rr_builtin_constants).
PRINT mbYes; " "; clRed; " "; crHandPoint
IF mbYes = 0 THEN PRINT "mbYes is 0 here"
