' `FOR N = TO 3` with no start value counts from 0, as RC.EXE builds it
' (RapidQ's music/StutterBox.rqb). The .expected is RC.EXE's output.
$APPTYPE CONSOLE
DIM N AS INTEGER
N = 7
For N = To 3
  PRINT N;
NEXT N
PRINT
PRINT "end"; N
