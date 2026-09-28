' Variables that are never DIMmed are global: shared by the program and
' every SUB / FUNCTION, and kept between calls.
SUB S1
    q = 5
END SUB
SUB S2
    PRINT "in S2: "; z
    z = 9
END SUB
SUB S3
    w = w + 1
    PRINT "w "; w
END SUB
SUB S4
    PRINT "k "; k
END SUB
FUNCTION Twice (n AS INTEGER) AS INTEGER
    calls = calls + 1
    Twice = n * 2
END FUNCTION
S1
PRINT "q "; q
z = 1
S2
PRINT "z "; z
S3
S3
k = 4
S4
PRINT Twice(2) + Twice(3); " calls "; calls
