' A program run with no arguments sees none (not the runner's `run-bc
' <file>`): CommandCount 0, COMMAND$(1) "", the bare COMMAND$ "".
$APPTYPE CONSOLE
PRINT "count="; CommandCount
PRINT "arg1=["; COMMAND$(1); "]"
PRINT "bare=["; COMMAND$; "]"
IF LEN(COMMAND$(0)) > 0 THEN PRINT "self ok"
