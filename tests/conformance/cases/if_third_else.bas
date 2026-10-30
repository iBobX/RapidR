' RapidQ's compiler takes a second ELSE on a single-line IF (the statement
' after it never runs: if_else_forms) and refuses a third — "ELSE
' without IF statement", as for a nested IF's ELSEs (the innermost IF, then
' its one spare) and for a chain of ELSE IFs. RapidR refuses each at its
' ELSE, in its own words.
IF 0 THEN PRINT "a" ELSE PRINT "b" ELSE PRINT "c" ELSE PRINT "d"
IF 0 THEN IF 1 THEN IF 1 THEN PRINT "a" ELSE PRINT "b" ELSE PRINT "c" ELSE PRINT "d"
IF 0 THEN PRINT "a" ELSE IF 0 THEN PRINT "b" ELSE PRINT "c" ELSE PRINT "d" ELSE PRINT "e"
