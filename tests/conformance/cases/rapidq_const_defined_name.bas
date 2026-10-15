' A CONST whose name a $DEFINE replaced by a number — windows.inc's
' `$DEFINE TRUE 1` before rapidq2.inc's `CONST True = 1` — RC.EXE takes it
' and nothing changes: the number is still the number. The .expected is
' RC.EXE's output.
$APPTYPE CONSOLE
$DEFINE TRUE 1
CONST True = 1
PRINT True
CONST 2 = 7
PRINT 2 + 2
$DEFINE X Y
CONST X = 5
PRINT Y
$DEFINE FOO 7
PRINT foo; FOO
