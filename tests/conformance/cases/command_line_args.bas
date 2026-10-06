' The program's command line (command_line_args.args: a, "b c", x=1 /opt):
' its own arguments only, however it runs (`rapidr run-bc file` here, the
' built executable on the codegen backend — never the runner's own).
' RapidQ (RC.EXE): CommandCount is how many; COMMAND$(0) the program's path,
' COMMAND$(1..CommandCount) the arguments ("b c" is one), any other index
' "". A bare COMMAND$ is RapidR's: the arguments joined with spaces.
$APPTYPE CONSOLE
DIM i AS INTEGER
PRINT "count="; CommandCount
FOR i = 1 TO CommandCount + 1
  PRINT "arg"; i; "=["; COMMAND$(i); "]"
NEXT
PRINT "neg=["; COMMAND$(-1); "]"
' (the path depends on the backend: the program's own file, by name)
IF INSTR(COMMAND$(0), "command_line_args") > 0 THEN PRINT "self ok"
IF RIGHT$(COMMAND$(0), LEN(Application.ExeName)) = Application.ExeName THEN PRINT "exename ok"
PRINT "bare=["; COMMAND$; "]"
DIM s AS STRING
s = COMMAND$(2)
PRINT "s=["; s; "]"; LEN(s)
