' Name() passes a whole array to a SUB / FUNCTION with an array parameter.
DECLARE SUB Fill (A() AS INTEGER, n AS INTEGER)
DECLARE FUNCTION Total (A() AS INTEGER, n AS INTEGER) AS INTEGER
DECLARE SUB Inner (B() AS INTEGER)
DIM M(5) AS INTEGER
Fill M(), 5
PRINT M(2); " "; M(5)
CALL Fill(M(), 3)
Fill (M(), 2)
PRINT M(1); " "; Total(M(), 5)
PRINT UBOUND(M())
SUB Fill (A() AS INTEGER, n AS INTEGER)
    DIM L(3) AS INTEGER
    FOR i = 0 TO n: A(i) = i * 10: NEXT i
    Inner L()
    PRINT "inner "; L(0); " "; UBOUND(L())
END SUB
SUB Inner (B() AS INTEGER)
    B(0) = 99
END SUB
FUNCTION Total (A() AS INTEGER, n AS INTEGER) AS INTEGER
    DIM s AS INTEGER
    FOR i = 0 TO n: s = s + A(i): NEXT i
    Total = s
END FUNCTION
