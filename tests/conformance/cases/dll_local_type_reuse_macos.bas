' skip-on: win32, linux, web — the C library on macOS
' A SUB whose local TYPE is handed to a DLL, called 100,000 times: the
' memory of the TYPEs gone goes to the new ones (after a quarantine of 32),
' so the program's addresses stop growing (docs/windows-dll-calls.md §7;
' before, each call took 8 KB of addresses for good, and after about
' 260,000 calls the program stopped: "out of memory for addresses").
TYPE TPoint
    x AS LONG
    y AS LONG
END TYPE
DECLARE FUNCTION memset LIB "libSystem.B.dylib" ALIAS "memset" (p AS TPoint, BYVAL c AS LONG, BYVAL n AS LONG) AS LONG
DECLARE SUB Once
DIM lowest AS LONG, highest AS LONG, i AS LONG, filled AS LONG

SUB Once
    DIM pt AS TPoint
    pt.y = 7
    memset(pt, 1, 4)
    IF pt.x = &H01010101 AND pt.y = 7 THEN filled = filled + 1
    IF lowest = 0 OR VARPTR(pt) < lowest THEN lowest = VARPTR(pt)
    IF VARPTR(pt) > highest THEN highest = VARPTR(pt)
END SUB

FOR i = 1 TO 100000
    Once
NEXT i
PRINT "calls that filled their TYPE:"; filled
PRINT "addresses within 1 MB:"; highest - lowest < 1048576
