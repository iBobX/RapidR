' RapidQ doesn't check an array's bounds (its manual: "There is no checking
' for limits on arrays"): a program writing past the end — `DEFSTR D(16)`
' then `READ D(N)` for N up to 28, RapidQ's GMDrum example — reads back what
' it wrote (RC.EXE). RapidR keeps those elements beside the array
' (memory-safely). The .expected is RC.EXE's output.
$APPTYPE CONSOLE
DEFSTR D(2), T
DEFINT N
FOR N = 0 TO 5
  READ D(N)
  PRINT N; " "; D(N)
NEXT N
PRINT "after"
DATA "a", "b", "c", "d", "e", "f"
