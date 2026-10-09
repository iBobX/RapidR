' A bare INPUT$ (no count, no parentheses) reads a line, as INPUT does —
' RapidQ's examples end with `a = INPUT$` to wait before they close. The
' .expected is RC.EXE's output with this .input.
$APPTYPE CONSOLE
DIM a AS STRING
PRINT "before"
a = INPUT$
PRINT "got ["; a; "]"; LEN(a)
