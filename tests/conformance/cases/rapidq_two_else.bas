' A second ELSE in a block IF (RapidQ's forms/Window Sizer.BAS): RC.EXE
' compiles it, and what follows it never runs. The .expected is RC.EXE's.
$APPTYPE CONSOLE
DIM i AS INTEGER
FOR i = 0 TO 1
IF i THEN
  PRINT "a"; i
ELSE
  PRINT "b"; i
ELSE
  PRINT "c"; i
END IF
NEXT
PRINT "end"
