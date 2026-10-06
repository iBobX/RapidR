' counter.bas: a SUB's own variables, as the debugger shows them
$APPTYPE CONSOLE

DIM Total AS INTEGER

SUB Tick(n AS INTEGER)
    STATIC calls AS INTEGER
    calls = calls + 1
    last = last + n             ' its own: RapidQ keeps it between calls
    Total = Total + last
    PRINT "call"; calls; ": total"; Total
END SUB

Tick 1
Tick 2
Tick 3
PRINT "Total: "; Total
