' Variables that are never DIMmed, as RapidQ's one-pass compiler places
' them (RC.EXE prints this): a name the main program used above a SUB is that
' global inside it; a name a SUB uses first is the SUB's own, kept between
' calls (S3's w), and the main program's same name further down is another
' variable (q, z, k, calls). rapidr_ast::implicit_scope.
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
