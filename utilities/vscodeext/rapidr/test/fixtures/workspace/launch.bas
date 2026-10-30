' launch.bas: what a launch configuration passes (arguments, environment, folder).
$APPTYPE CONSOLE
PRINT "count=" + STR$(COMMANDCOUNT)
PRINT "arg1=" + COMMAND$(1)
PRINT "arg2=" + COMMAND$(2)
PRINT "env=" + ENVIRON("RVSC_ENV")
PRINT "cwd=" + CURDIR$
