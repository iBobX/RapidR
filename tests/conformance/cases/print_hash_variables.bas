' PRINT # with variables, then the same variables used again (2.122.1: the
' native build moved them into the call and failed to compile).
SUB Test
    DIM n AS INTEGER = 3, s AS STRING = "x"
    DIM line$ AS STRING
    OPEN "tests/conformance/.work/print_hash_vars.txt" FOR OUTPUT AS #9
    PRINT #9, "n="; n; " s="; s
    PRINT #9, n, s
    CLOSE #9
    IF s = "x" AND n = 3 THEN PRINT "kept"
    OPEN "tests/conformance/.work/print_hash_vars.txt" FOR INPUT AS #9
    WHILE NOT EOF(9)
        LINE INPUT #9, line$
        PRINT "[" + line$ + "]"
    WEND
    CLOSE #9
END SUB
Test
