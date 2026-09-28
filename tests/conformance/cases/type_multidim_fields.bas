TYPE Grid
    cells(2, 3) AS INTEGER
    tag(1 TO 2, 0 TO 1) AS STRING
    n AS INTEGER
    SUB Fill
        FOR i = 0 TO 2
            FOR j = 0 TO 3
                This.cells(i, j) = i * 10 + j
            NEXT j
        NEXT i
        This.tag(2, 1) = "x"
    END SUB
    FUNCTION Sum AS INTEGER
        s = 0
        FOR i = 0 TO 2
            FOR j = 0 TO 3
                s = s + cells(i, j)
            NEXT j
        NEXT i
        Sum = s
    END FUNCTION
END TYPE
DIM g AS Grid
g.Fill
PRINT g.cells(1, 2); " "; g.cells(2, 3); " "; g.Sum; " "; g.tag(2, 1)
g.cells(0, 0) = 5
PRINT g.cells(0, 0); UBOUND(g.cells, 1); UBOUND(g.cells, 2)
