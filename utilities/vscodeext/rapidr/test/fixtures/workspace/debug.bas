' debug.bas: a console program for the debugger's test.
$APPTYPE CONSOLE
DIM total AS INTEGER

SUB AddUp(n AS INTEGER)
    DIM i AS INTEGER
    FOR i = 1 TO n
        total = total + i
    NEXT
END SUB

AddUp 3
PRINT "total " + STR$(total)
