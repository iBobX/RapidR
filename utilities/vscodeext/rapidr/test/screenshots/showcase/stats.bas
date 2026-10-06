' stats.bas: a console program to debug
$APPTYPE CONSOLE

DIM values(1 TO 5) AS DOUBLE
DIM i AS INTEGER

FUNCTION Mean(n AS INTEGER) AS DOUBLE
    DIM sum AS DOUBLE
    DIM k AS INTEGER
    FOR k = 1 TO n
        sum = sum + values(k)
    NEXT
    Mean = sum / n
END FUNCTION

FOR i = 1 TO 5
    values(i) = i * i
NEXT
PRINT "Mean of the squares: "; Mean(5)
