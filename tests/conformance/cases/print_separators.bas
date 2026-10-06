' A trailing ';' or ',' keeps the cursor on the line; ';' and ',' both join
' items the same way (RapidQ's manual says so, and RC.EXE prints this), no
' QBasic print zones.
PRINT "a"; "b"
PRINT "c";
PRINT "d"
PRINT "e"
PRINT "ab", "c"
PRINT "x",
PRINT "y"
