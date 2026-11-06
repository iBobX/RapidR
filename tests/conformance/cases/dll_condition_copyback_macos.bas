' skip-on: win32, linux, web — the C library on macOS
' A DLL call in an IF's, a WHILE's or a SELECT's own expression that writes
' a variable handed over (VARPTR, a STRING parameter): the statements
' inside and after see what it wrote, as with RapidQ's one memory
' (docs/windows-dll-calls.md §7; before, they read the old value).
DECLARE FUNCTION strcpy LIB "libSystem.B.dylib" ALIAS "strcpy" (BYVAL dest AS LONG, BYVAL src AS STRING) AS LONG
DECLARE FUNCTION strcat LIB "libSystem.B.dylib" ALIAS "strcat" (dest AS STRING, BYVAL src AS STRING) AS LONG
DIM b AS STRING, c AS STRING
DIM k AS LONG

b = STRING$(10, 0)
IF strcpy(VARPTR(b), "abc") THEN PRINT "in the IF: ["; LEFT$(b, 3); "]"
PRINT "after the IF: ["; LEFT$(b, 3); "]"

c = STRING$(10, 0)
WHILE strcat(c, "x") AND k < 3
    k = k + 1
    PRINT "in the WHILE: ["; LEFT$(c, INSTR(c, CHR$(0)) - 1); "]"
WEND
PRINT "after the WHILE: ["; LEFT$(c, INSTR(c, CHR$(0)) - 1); "]"

SELECT CASE strcpy(VARPTR(b), "xyz") <> 0
CASE -1
    PRINT "in the CASE: ["; LEFT$(b, 3); "]"
END SELECT
