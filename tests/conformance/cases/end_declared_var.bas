' END with a bare name is an exit code only when the name was declared: a
' DIM here (a CONST, a parameter, a local … too). The typo of a block's
' closer (END SUBB) and an undeclared word are RapidQ's syntax error
' (end_typo_closer).
DIM code AS INTEGER = 5
PRINT "ending"
END code
PRINT "not reached"
