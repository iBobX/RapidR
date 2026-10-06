' ownvars.bas: a SUB's own variables (a STATIC, and one it uses first,
' which RapidQ keeps between calls) are its frame's, never globals.
$APPTYPE CONSOLE
DIM Total AS INTEGER

SUB Tick(n AS INTEGER)
    STATIC hits AS INTEGER
    hits = hits + 1
    p = p + n
    Total = Total + p
END SUB

Tick 1
Tick 2
PRINT "total " + STR$(Total)
